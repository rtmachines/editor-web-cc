//! SQL of the audit list, one entry, and the recipe history of `AuditReads` (ADR-0051, design D5).
//!
//! `data-model.md` sections 3.15 and 10 give the table and the entity types. Each list is one
//! statement, after the lookup of the filter `actor`. Each statement runs on the connection of the
//! open transaction. No statement takes a lock.

use std::fmt;
use std::net::IpAddr;
use std::num::NonZeroU64;

use recipe_app::error::StorageError;
use recipe_app::ports::{
    AggregateId, AuditEntityType, AuditEntryId, AuditFilter, AuditRecord, AuditRow, AuditScope,
    CursorPage, EntityId, EntityLabel,
};
use recipe_domain::audit::{AggregateType, AuditAction, RequestId, SessionRef};
use recipe_domain::identity::{Login, UserId};
use recipe_domain::plant::{MachineCode, TypeName, UnitPath};
use recipe_domain::recipe::{ParameterTag, RecipeId, ReferenceNumber};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use sqlx::{PgConnection, Postgres, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{corrupt, map_error};

/// The table name of the defects.
const TABLE: &str = "audit_log";

/// The user id of a login. The audit list resolves the filter `actor` with it first.
const SELECT_ACTOR: &str = "SELECT id FROM app_user WHERE login = $1";

/// The columns of a list row in the inner select. No state column is part of them (R30).
const ROW_COLUMNS: &str = "\
id, occurred_at, actor_user_id, action, entity_type, entity_id, aggregate_type, aggregate_id";

/// The outer select of a list row with the actor login and the label columns.
const OUTER_ROW: &str = "\
SELECT l.id, l.occurred_at, actor.login AS actor, l.action, l.entity_type, l.entity_id, \
    l.aggregate_type, l.aggregate_id, m.code AS machine_code, u.path AS unit_path, \
    t.name AS type_name, t.revision AS type_revision, r.reference_number, \
    v.number AS version_number, p.tag AS parameter_tag, pv.number AS parameter_version, \
    au.login AS user_login, s.key AS setting_key";

/// The joins of the actor and of the current row of each entity type (ADR-0051 (e)).
///
/// Each join matches only its entity type. A row that no longer exists gives null columns.
/// Only a struct type has a name and a revision for a label, as in the memory adapter.
const LABEL_JOINS: &str = " \
LEFT JOIN app_user actor ON actor.id = l.actor_user_id \
LEFT JOIN machine m ON l.entity_type = 'machine' AND m.id = l.entity_id \
LEFT JOIN unit u ON l.entity_type = 'unit' AND u.id = l.entity_id \
LEFT JOIN plc_type t ON l.entity_type = 'plc_type' AND t.id = l.entity_id AND t.kind = 'struct' \
LEFT JOIN recipe r ON l.entity_type = 'recipe' AND r.id = l.entity_id \
LEFT JOIN recipe_version v ON l.entity_type = 'recipe_version' AND v.id = l.entity_id \
LEFT JOIN parameter p ON l.entity_type = 'parameter' AND p.id = l.entity_id \
LEFT JOIN recipe_version pv ON pv.id = p.recipe_version_id \
LEFT JOIN app_user au ON l.entity_type = 'app_user' AND au.id = l.entity_id \
LEFT JOIN app_setting s ON l.entity_type = 'app_setting' AND s.id = l.entity_id";

/// The columns of one entry in the inner select besides the list columns (ADR-0051 (f)).
///
/// The select reads no `entry_hash` and no `prev_hash`. `host` gives the address without a mask.
const RECORD_COLUMNS: &str = ", request_id, before_state, after_state, chain_seq, \
host(client_ip) AS client_ip, session_ref";

/// The columns of one entry in the outer select besides the list columns.
const OUTER_RECORD: &str = ", l.request_id, l.before_state, l.after_state, l.chain_seq, \
l.client_ip, l.session_ref";

/// The predicate of the scope `Recipes` (A20, ADR-0051 (c)).
const RECIPE_SCOPE: &str = " AND aggregate_type = 'recipe'";

/// The cause of the storage error for a page size whose window has no `bigint` value.
#[derive(Debug, thiserror::Error)]
#[error("the page size {limit} has no window")]
struct WindowOverflow {
    limit: u32,
}

/// The code of the root of a machine aggregate.
const MACHINE_LABEL: &str = "SELECT code FROM machine WHERE id = $1";

/// The reference number of the root of a recipe aggregate. A deleted recipe keeps its row.
const RECIPE_LABEL: &str = "SELECT reference_number FROM recipe WHERE id = $1";

/// The login of the root of a user aggregate.
const USER_LABEL: &str = "SELECT login FROM app_user WHERE id = $1";

/// The key of the root of a setting aggregate.
const SETTING_LABEL: &str = "SELECT key FROM app_setting WHERE id = $1";

/// The name and the revision of the root of a type aggregate. Only a struct row gives a label.
const TYPE_LABEL: &str = "SELECT name, revision FROM plc_type WHERE id = $1 AND kind = 'struct'";

/// The label columns of one row. Each column is null when its join finds no row.
#[derive(Default, sqlx::FromRow)]
struct LabelRecord {
    machine_code: Option<String>,
    unit_path: Option<String>,
    type_name: Option<String>,
    type_revision: Option<i32>,
    reference_number: Option<String>,
    version_number: Option<i32>,
    parameter_tag: Option<String>,
    parameter_version: Option<i32>,
    user_login: Option<String>,
    setting_key: Option<String>,
}

/// One list row as the statement returns it.
#[derive(sqlx::FromRow)]
struct RowRecord {
    id: i64,
    occurred_at: OffsetDateTime,
    actor: Option<String>,
    action: String,
    entity_type: String,
    entity_id: Uuid,
    aggregate_type: String,
    aggregate_id: Uuid,
    #[sqlx(flatten)]
    label: LabelRecord,
}

/// One entry with its states and its trace fields as the statement returns it.
#[derive(sqlx::FromRow)]
struct EntryRecord {
    #[sqlx(flatten)]
    row: RowRecord,
    request_id: Uuid,
    before_state: Option<Value>,
    after_state: Option<Value>,
    chain_seq: i64,
    client_ip: Option<String>,
    session_ref: Option<String>,
}

/// Parse an optional column with `parse`. A value that fails the grammar is corrupt.
fn parsed<T, E: fmt::Display>(
    table: &'static str,
    value: Option<&str>,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<Option<T>, StorageError> {
    value
        .map(parse)
        .transpose()
        .map_err(|error| corrupt(table, error))
}

/// Convert an optional `INTEGER` number of a version or a revision.
fn number(table: &'static str, value: Option<i32>) -> Result<Option<u32>, StorageError> {
    value
        .map(u32::try_from)
        .transpose()
        .map_err(|error| corrupt(table, error))
}

/// Build the label of an entity of `kind` from the label columns of its row.
fn label(kind: AuditEntityType, columns: LabelRecord) -> Result<EntityLabel, StorageError> {
    let found = match kind {
        AuditEntityType::Machine => parsed(
            "machine",
            columns.machine_code.as_deref(),
            MachineCode::parse,
        )?
        .map(EntityLabel::Machine),
        AuditEntityType::Unit => {
            parsed("unit", columns.unit_path.as_deref(), UnitPath::parse)?.map(EntityLabel::Unit)
        }
        AuditEntityType::PlcType => {
            let name = parsed("plc_type", columns.type_name.as_deref(), TypeName::parse)?;
            let revision = number("plc_type", columns.type_revision)?;
            name.zip(revision)
                .map(|(name, revision)| EntityLabel::Type { name, revision })
        }
        AuditEntityType::Recipe => parsed(
            "recipe",
            columns.reference_number.as_deref(),
            ReferenceNumber::parse,
        )?
        .map(EntityLabel::Recipe),
        AuditEntityType::RecipeVersion => {
            number("recipe_version", columns.version_number)?.map(EntityLabel::Version)
        }
        AuditEntityType::Parameter => {
            let tag = parsed(
                "parameter",
                columns.parameter_tag.as_deref(),
                ParameterTag::parse,
            )?;
            let version = number("recipe_version", columns.parameter_version)?;
            tag.zip(version)
                .map(|(tag, version)| EntityLabel::Parameter { tag, version })
        }
        AuditEntityType::AppUser => {
            parsed("app_user", columns.user_login.as_deref(), Login::parse)?.map(EntityLabel::User)
        }
        AuditEntityType::AppSetting => {
            parsed("app_setting", columns.setting_key.as_deref(), Text::parse)?
                .map(EntityLabel::Setting)
        }
    };
    Ok(found.unwrap_or(EntityLabel::Missing))
}

/// Convert a `BIGINT` id of the table into a positive id.
pub(super) fn positive_id(table: &'static str, id: i64) -> Result<NonZeroU64, StorageError> {
    u64::try_from(id)
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or_else(|| corrupt(table, format!("the id {id} is not positive")))
}

/// Return the action of a stored action name.
fn action(text: &str) -> Result<AuditAction, StorageError> {
    AuditAction::ALL
        .into_iter()
        .find(|action| action.as_str() == text)
        .ok_or_else(|| corrupt(TABLE, format!("the action {text} is unknown")))
}

/// Return the aggregate type of a stored aggregate type text.
fn aggregate_type(text: &str) -> Result<AggregateType, StorageError> {
    AggregateType::ALL
        .into_iter()
        .find(|aggregate| aggregate.as_sql() == text)
        .ok_or_else(|| corrupt(TABLE, format!("the aggregate type {text} is unknown")))
}

/// Build the list row of a stored row.
fn row(record: RowRecord) -> Result<AuditRow, StorageError> {
    let entity_type = AuditEntityType::from_sql(&record.entity_type).ok_or_else(|| {
        corrupt(
            TABLE,
            format!("the entity type {} is unknown", record.entity_type),
        )
    })?;
    Ok(AuditRow {
        id: AuditEntryId::from(positive_id(TABLE, record.id)?),
        occurred_at: Timestamp::from(record.occurred_at),
        actor: parsed("app_user", record.actor.as_deref(), Login::parse)?,
        action: action(&record.action)?,
        entity_type,
        entity_id: EntityId::from(record.entity_id),
        aggregate_type: aggregate_type(&record.aggregate_type)?,
        aggregate_id: AggregateId::from(record.aggregate_id),
        label: label(entity_type, record.label)?,
    })
}

/// Return the `LIMIT` value of a window: one row more than the page.
pub(super) fn window(limit: u32) -> Result<i64, StorageError> {
    i64::from(limit)
        .checked_add(1)
        .ok_or_else(|| StorageError::Other {
            sqlstate: None,
            source: Box::new(WindowOverflow { limit }),
        })
}

/// Return the `bigint` value of an id, or `None` for an id above every stored id.
///
/// No stored id exceeds `i64::MAX`. Such a cursor therefore keeps every row and needs no
/// predicate, and such an entry id names no row.
pub(super) fn stored_id(id: u64) -> Option<i64> {
    i64::try_from(id).ok()
}

/// Add the predicate of each sent filter to the inner select. `actor` is the resolved user id.
fn push_filters(
    builder: &mut QueryBuilder<'_, Postgres>,
    filter: &AuditFilter,
    actor: Option<UserId>,
    scope: AuditScope,
) {
    if let Some(aggregate) = filter.aggregate {
        builder
            .push(" AND aggregate_id = ")
            .push_bind(*aggregate.as_uuid());
    }
    if let Some(entity) = filter.entity {
        builder
            .push(" AND entity_type = ")
            .push_bind(entity.entity_type.as_sql());
        if let Some(id) = entity.entity_id {
            builder.push(" AND entity_id = ").push_bind(*id.as_uuid());
        }
    }
    if let Some(actor) = actor {
        builder
            .push(" AND actor_user_id = ")
            .push_bind(*actor.as_uuid());
    }
    if let Some(action) = filter.action {
        builder.push(" AND action = ").push_bind(action.as_str());
    }
    if let Some(from) = filter.range.from {
        builder
            .push(" AND occurred_at >= ")
            .push_bind(*from.as_offset_date_time());
    }
    if let Some(until) = filter.range.until {
        builder
            .push(" AND occurred_at < ")
            .push_bind(*until.as_offset_date_time());
    }
    if scope == AuditScope::Recipes {
        builder.push(RECIPE_SCOPE);
    }
}

/// Return the user id of `login`, or `None` when no user has it.
async fn actor_id(
    connection: &mut PgConnection,
    login: &Login,
) -> Result<Option<UserId>, StorageError> {
    let found: Option<Uuid> = sqlx::query_scalar(SELECT_ACTOR)
        .bind(login.as_str())
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    Ok(found.map(UserId::from))
}

/// Read one page of the entries in `scope` that match `filter`, newest first.
///
/// A filter `actor` without a user gives the empty page without a further read (A42).
pub(super) async fn entries(
    connection: &mut PgConnection,
    filter: &AuditFilter,
    scope: AuditScope,
    before: Option<AuditEntryId>,
    limit: u32,
) -> Result<CursorPage<AuditRow>, StorageError> {
    let actor = match &filter.actor {
        None => None,
        Some(login) => match actor_id(&mut *connection, login).await? {
            None => return Ok(CursorPage::empty()),
            found => found,
        },
    };
    let mut builder = QueryBuilder::<Postgres>::new(OUTER_ROW);
    builder
        .push(" FROM (SELECT ")
        .push(ROW_COLUMNS)
        .push(" FROM audit_log WHERE true");
    if let Some(before) = before.map(AuditEntryId::get).and_then(stored_id) {
        builder.push(" AND id < ").push_bind(before);
    }
    push_filters(&mut builder, filter, actor, scope);
    builder
        .push(" ORDER BY id DESC LIMIT ")
        .push_bind(window(limit)?);
    builder
        .push(") l")
        .push(LABEL_JOINS)
        .push(" ORDER BY l.id DESC");
    let records = builder
        .build_query_as::<RowRecord>()
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let rows = records
        .into_iter()
        .map(row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CursorPage::cut(rows, limit))
}

/// Build the record of a stored entry.
fn record(stored: EntryRecord) -> Result<AuditRecord, StorageError> {
    let client_ip = stored
        .client_ip
        .as_deref()
        .map(str::parse::<IpAddr>)
        .transpose()
        .map_err(|error| corrupt(TABLE, error))?;
    Ok(AuditRecord {
        row: row(stored.row)?,
        request_id: RequestId::from(stored.request_id),
        before: stored.before_state,
        after: stored.after_state,
        chain_seq: stored.chain_seq,
        client_ip,
        session_ref: parsed(TABLE, stored.session_ref.as_deref(), SessionRef::parse)?,
    })
}

/// Read the entry `id` in `scope` with its states.
///
/// The scope `Recipes` adds its predicate to the `WHERE` clause. An entry outside the scope and
/// an absent entry therefore give `None` from the same statement.
pub(super) async fn entry(
    connection: &mut PgConnection,
    id: AuditEntryId,
    scope: AuditScope,
) -> Result<Option<AuditRecord>, StorageError> {
    let Some(id) = stored_id(id.get()) else {
        return Ok(None);
    };
    let mut builder = QueryBuilder::<Postgres>::new(OUTER_ROW);
    builder
        .push(OUTER_RECORD)
        .push(" FROM (SELECT ")
        .push(ROW_COLUMNS)
        .push(RECORD_COLUMNS);
    builder.push(" FROM audit_log WHERE id = ").push_bind(id);
    if scope == AuditScope::Recipes {
        builder.push(RECIPE_SCOPE);
    }
    builder.push(") l").push(LABEL_JOINS);
    let stored = builder
        .build_query_as::<EntryRecord>()
        .fetch_optional(connection)
        .await
        .map_err(map_error)?;
    stored.map(record).transpose()
}

/// Read the one text column of the row `id` with the statement `sql`.
async fn label_text(
    connection: &mut PgConnection,
    sql: &'static str,
    id: Uuid,
) -> Result<Option<String>, StorageError> {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_optional(connection)
        .await
        .map_err(map_error)
}

/// Read the label of the root `id` of an aggregate of `aggregate_type` (ADR-0051 (d)).
///
/// One bound statement reads the current row of the root. No row gives `EntityLabel::Missing`.
pub(super) async fn aggregate_label(
    connection: &mut PgConnection,
    aggregate_type: AggregateType,
    id: AggregateId,
) -> Result<EntityLabel, StorageError> {
    let id = *id.as_uuid();
    let columns = match aggregate_type {
        AggregateType::Machine => LabelRecord {
            machine_code: label_text(connection, MACHINE_LABEL, id).await?,
            ..LabelRecord::default()
        },
        AggregateType::Recipe => LabelRecord {
            reference_number: label_text(connection, RECIPE_LABEL, id).await?,
            ..LabelRecord::default()
        },
        AggregateType::AppUser => LabelRecord {
            user_login: label_text(connection, USER_LABEL, id).await?,
            ..LabelRecord::default()
        },
        AggregateType::AppSetting => LabelRecord {
            setting_key: label_text(connection, SETTING_LABEL, id).await?,
            ..LabelRecord::default()
        },
        AggregateType::PlcType => {
            let found: Option<(Option<String>, Option<i32>)> = sqlx::query_as(TYPE_LABEL)
                .bind(id)
                .fetch_optional(connection)
                .await
                .map_err(map_error)?;
            let (type_name, type_revision) = found.unwrap_or_default();
            LabelRecord {
                type_name,
                type_revision,
                ..LabelRecord::default()
            }
        }
    };
    label(AuditEntityType::from(aggregate_type), columns)
}

/// Read one page of the entries with the aggregate id of `recipe`, newest first.
///
/// The history reads in the scope `Recipes`, as the memory adapter does. It uses the index
/// `audit_log_aggregate_idx`.
pub(super) async fn history(
    connection: &mut PgConnection,
    recipe: RecipeId,
    before: Option<AuditEntryId>,
    limit: u32,
) -> Result<CursorPage<AuditRow>, StorageError> {
    let filter = AuditFilter {
        aggregate: Some(AggregateId::from(*recipe.as_uuid())),
        ..AuditFilter::default()
    };
    entries(connection, &filter, AuditScope::Recipes, before, limit).await
}
