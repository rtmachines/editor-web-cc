//! SQL of the security event list of `AuditReads` (ADR-0051 (b), (h), `data-model.md` section 3.8).
//!
//! The list is one statement on the connection of the open transaction. It takes no lock. The
//! column `login_hmac` appears only in a predicate, so no HMAC leaves the adapter (R35, A33).

use std::net::IpAddr;

use recipe_app::error::StorageError;
use recipe_app::ports::{
    CursorPage, LoginMatch, SecurityEventFilter, SecurityEventId, SecurityEventRow,
};
use recipe_domain::audit::{RequestId, SecurityEventKind, UserAgent};
use recipe_domain::identity::Login;
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use sqlx::{PgConnection, Postgres, QueryBuilder};
use time::OffsetDateTime;
use uuid::Uuid;

use super::audit_reads::{positive_id, stored_id, window};
use crate::address::canonical_text;
use crate::error::{corrupt, map_error};

/// The table name of the defects.
const TABLE: &str = "security_event";

/// The columns of a row in the inner select. `host` gives the address without a mask.
const EVENT_COLUMNS: &str = "\
id, occurred_at, kind, count, host(client_ip) AS client_ip, user_agent, request_id, user_id, detail";

/// The outer select of a row with the login of its user.
const OUTER_EVENT: &str = "\
SELECT e.id, e.occurred_at, e.kind, e.count, e.client_ip, e.user_agent, e.request_id, \
    u.login, e.detail";

/// One row as the statement returns it. It holds no `login_hmac`.
#[derive(sqlx::FromRow)]
struct EventRecord {
    id: i64,
    occurred_at: OffsetDateTime,
    kind: String,
    count: i32,
    client_ip: Option<String>,
    user_agent: Option<String>,
    request_id: Uuid,
    login: Option<String>,
    detail: Option<Value>,
}

/// Return the kind of a stored kind text.
fn kind(text: &str) -> Result<SecurityEventKind, StorageError> {
    SecurityEventKind::ALL
        .into_iter()
        .find(|kind| kind.as_sql() == text)
        .ok_or_else(|| corrupt(TABLE, format!("the kind {text} is unknown")))
}

/// Build the list row of a stored row.
fn event_row(record: EventRecord) -> Result<SecurityEventRow, StorageError> {
    let client_ip = record
        .client_ip
        .as_deref()
        .map(str::parse::<IpAddr>)
        .transpose()
        .map_err(|error| corrupt(TABLE, error))?;
    let login = record
        .login
        .as_deref()
        .map(Login::parse)
        .transpose()
        .map_err(|error| corrupt("app_user", error))?;
    Ok(SecurityEventRow {
        id: SecurityEventId::from(positive_id(TABLE, record.id)?),
        occurred_at: Timestamp::from(record.occurred_at),
        kind: kind(&record.kind)?,
        count: u32::try_from(record.count).map_err(|error| corrupt(TABLE, error))?,
        client_ip,
        user_agent: record.user_agent.as_deref().map(UserAgent::from),
        request_id: RequestId::from(record.request_id),
        login,
        detail: record.detail,
    })
}

/// Add the predicate of the filter `login` (ADR-0051 (h)).
///
/// A row matches when its user has the login, or when its `login_hmac` equals the HMAC under the
/// current key. A login outside the grammar of `Login` names no account, so only the HMAC counts.
fn push_login<'args>(builder: &mut QueryBuilder<'args, Postgres>, subject: &'args LoginMatch) {
    let hmac = subject.hmac.as_bytes().as_slice();
    match &subject.login {
        Some(login) => {
            builder
                .push(" AND (user_id = (SELECT a.id FROM app_user a WHERE a.login = ")
                .push_bind(login.as_str())
                .push(") OR login_hmac = ")
                .push_bind(hmac)
                .push(")");
        }
        None => {
            builder.push(" AND login_hmac = ").push_bind(hmac);
        }
    }
}

/// Add the predicate of each sent filter to the inner select.
fn push_filters<'args>(
    builder: &mut QueryBuilder<'args, Postgres>,
    filter: &'args SecurityEventFilter,
) {
    if let Some(kind) = filter.kind {
        builder.push(" AND kind = ").push_bind(kind.as_sql());
    }
    if let Some(subject) = &filter.login {
        push_login(builder, subject);
    }
    if let Some(client) = filter.client_ip {
        builder
            .push(" AND client_ip = ")
            .push_bind(canonical_text(client))
            .push("::inet");
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
}

/// Read one page of the security event rows that match `filter`, newest first.
pub(super) async fn security_events(
    connection: &mut PgConnection,
    filter: &SecurityEventFilter,
    before: Option<SecurityEventId>,
    limit: u32,
) -> Result<CursorPage<SecurityEventRow>, StorageError> {
    let mut builder = QueryBuilder::<Postgres>::new(OUTER_EVENT);
    builder
        .push(" FROM (SELECT ")
        .push(EVENT_COLUMNS)
        .push(" FROM security_event WHERE true");
    if let Some(before) = before.map(SecurityEventId::get).and_then(stored_id) {
        builder.push(" AND id < ").push_bind(before);
    }
    push_filters(&mut builder, filter);
    builder
        .push(" ORDER BY id DESC LIMIT ")
        .push_bind(window(limit)?);
    builder
        .push(") e LEFT JOIN app_user u ON u.id = e.user_id")
        .push(" ORDER BY e.id DESC");
    let records = builder
        .build_query_as::<EventRecord>()
        .fetch_all(connection)
        .await
        .map_err(map_error)?;
    let rows = records
        .into_iter()
        .map(event_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CursorPage::cut(rows, limit))
}
