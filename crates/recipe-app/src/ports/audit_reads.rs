//! Read port of the audit views (ADR-0051 (a), design D5).

use std::fmt;
use std::net::IpAddr;
use std::num::NonZeroU64;

use async_trait::async_trait;
use recipe_domain::audit::{
    AggregateType, AuditAction, LoginHmac, RequestId, SecurityEventKind, SessionRef, UserAgent,
};
use recipe_domain::identity::Login;
use recipe_domain::plant::{MachineCode, TypeName, UnitPath};
use recipe_domain::recipe::{ParameterTag, RecipeId, ReferenceNumber};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use uuid::Uuid;

use crate::error::StorageError;

/// The number of characters of a short id (ADR-0051 (e)).
const SHORT_ID_CHARS: usize = 8;

/// Return the first eight characters of the text of `id`.
fn short_text(id: &Uuid) -> String {
    id.simple()
        .to_string()
        .chars()
        .take(SHORT_ID_CHARS)
        .collect()
}

/// The identifier of one audit entry: the value of `audit_log.id`, a positive integer.
///
/// The type solves one problem: the cursor `before` and the path value `{id}` name an entry, and a
/// bare integer could name a security event instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuditEntryId(NonZeroU64);

impl From<NonZeroU64> for AuditEntryId {
    fn from(value: NonZeroU64) -> Self {
        Self(value)
    }
}

impl AuditEntryId {
    /// Return the integer of the identifier.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for AuditEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The identifier of one row of `security_event`, a positive integer.
///
/// The type solves one problem: the cursor of the security event list must not take the id of an
/// audit entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecurityEventId(NonZeroU64);

impl From<NonZeroU64> for SecurityEventId {
    fn from(value: NonZeroU64) -> Self {
        Self(value)
    }
}

impl SecurityEventId {
    /// Return the integer of the identifier.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// The id of the root of an aggregate (`audit_log.aggregate_id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AggregateId(Uuid);

impl From<Uuid> for AggregateId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl AggregateId {
    /// Return the UUID of the aggregate.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Return the first eight characters of the id (ADR-0051 (e)).
    #[must_use]
    pub fn short(&self) -> String {
        short_text(&self.0)
    }
}

/// The id of the changed row of an audit entry (`audit_log.entity_id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(Uuid);

impl From<Uuid> for EntityId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl EntityId {
    /// Return the UUID of the row.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Return the first eight characters of the id (ADR-0051 (e)).
    #[must_use]
    pub fn short(&self) -> String {
        short_text(&self.0)
    }
}

/// The entity type of an audit entry: one of the eight values of `data-model.md` section 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditEntityType {
    /// A machine row.
    Machine,
    /// A unit row.
    Unit,
    /// A type row.
    PlcType,
    /// A recipe row.
    Recipe,
    /// A version row.
    RecipeVersion,
    /// A parameter row, with its limit rows and translation rows.
    Parameter,
    /// A user row.
    AppUser,
    /// A setting row.
    AppSetting,
}

impl AuditEntityType {
    /// Every entity type, in the order of `data-model.md` section 10.
    pub const ALL: [Self; 8] = [
        Self::Machine,
        Self::Unit,
        Self::PlcType,
        Self::Recipe,
        Self::RecipeVersion,
        Self::Parameter,
        Self::AppUser,
        Self::AppSetting,
    ];

    /// Return the text that the column `entity_type` stores.
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Unit => "unit",
            Self::PlcType => "plc_type",
            Self::Recipe => "recipe",
            Self::RecipeVersion => "recipe_version",
            Self::Parameter => "parameter",
            Self::AppUser => "app_user",
            Self::AppSetting => "app_setting",
        }
    }

    /// Return the entity type of the SQL text `text`, or `None` for another text.
    #[must_use]
    pub fn from_sql(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_sql() == text)
    }

    /// Return whether the entries of this type have `aggregate_type = 'recipe'` (`data-model.md` 10).
    #[must_use]
    pub const fn is_recipe_scope(self) -> bool {
        matches!(self, Self::Recipe | Self::RecipeVersion | Self::Parameter)
    }
}

/// The root of each aggregate type is a row of the table with the same name (`data-model.md` 10).
impl From<AggregateType> for AuditEntityType {
    fn from(value: AggregateType) -> Self {
        match value {
            AggregateType::Machine => Self::Machine,
            AggregateType::Recipe => Self::Recipe,
            AggregateType::AppUser => Self::AppUser,
            AggregateType::AppSetting => Self::AppSetting,
            AggregateType::PlcType => Self::PlcType,
        }
    }
}

/// The part of the audit log that a role may read (A20, ADR-0051 (c)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditScope {
    /// Every entry. A role with `AuditView` reads this scope.
    All,
    /// Only the entries with `aggregate_type = 'recipe'`. A role with only `AuditViewRecipes` reads this scope.
    Recipes,
}

impl AuditScope {
    /// Return whether an entry with `aggregate` is inside the scope.
    #[must_use]
    pub fn admits(self, aggregate: AggregateType) -> bool {
        match self {
            Self::All => true,
            Self::Recipes => aggregate == AggregateType::Recipe,
        }
    }
}

/// A range of instants: `from` is inside, `until` is outside.
///
/// The type solves one problem: the filters `from` and `to` take two dates in UTC, and both days are
/// inside. `recipe-web` sets `from` to the start of the first day and `until` to the start of the
/// day after the last day (design D6). `None` gives no bound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeRange {
    /// The first instant inside the range.
    pub from: Option<Timestamp>,
    /// The first instant after the range.
    pub until: Option<Timestamp>,
}

impl TimeRange {
    /// Return whether `instant` is inside the range.
    #[must_use]
    pub fn contains(&self, instant: Timestamp) -> bool {
        self.from.is_none_or(|from| instant >= from)
            && self.until.is_none_or(|until| instant < until)
    }
}

/// The filter `entity_type` with the optional filter `entity_id`.
///
/// The type solves one problem: `entity_id` needs `entity_type` (ADR-0051 (d)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityFilter {
    /// The entity type of the entries.
    pub entity_type: AuditEntityType,
    /// The id of the changed row. `None` keeps every row of the type.
    pub entity_id: Option<EntityId>,
}

/// The filter of the audit list (ADR-0051 (d)). Each `None` keeps every entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditFilter {
    /// The id of the aggregate of the entries.
    pub aggregate: Option<AggregateId>,
    /// The entity type and the entity id of the entries.
    pub entity: Option<EntityFilter>,
    /// The login of the actor. A login that names no user gives the empty list (A42).
    pub actor: Option<Login>,
    /// The action of the entries.
    pub action: Option<AuditAction>,
    /// The range of the commit times.
    pub range: TimeRange,
}

/// The label of the entity of an entry. It comes from the current row (ADR-0051 (e)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityLabel {
    /// A recipe: its reference number.
    Recipe(ReferenceNumber),
    /// A version: its number.
    Version(u32),
    /// A parameter: its tag and the number of its version.
    Parameter {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The number of the version of the parameter.
        version: u32,
    },
    /// A machine: its code.
    Machine(MachineCode),
    /// A unit: its path.
    Unit(UnitPath),
    /// A struct type: its name and its revision.
    ///
    /// Only a `struct` row gives this label. A derived type (`ARRAY`, `STRING(n)`, `WSTRING(n)`)
    /// and a primitive give `Missing`, also when the row exists (design D5).
    Type {
        /// The name of the type.
        name: TypeName,
        /// The revision of the type.
        revision: u32,
    },
    /// A user: the login.
    User(Login),
    /// A setting: its key.
    Setting(Text),
    /// No current row gives a label: the row does not exist, or the row is not a `struct` type.
    /// The page shows the word of the entity type and the short id.
    Missing,
}

/// One row of the audit list or of the recipe history. It holds no state (R30).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRow {
    /// The id of the entry.
    pub id: AuditEntryId,
    /// The commit time of the entry (`occurred_at`).
    pub occurred_at: Timestamp,
    /// The login of the actor, or `None` for an entry without an actor.
    pub actor: Option<Login>,
    /// The action of the entry.
    pub action: AuditAction,
    /// The entity type of the entry.
    pub entity_type: AuditEntityType,
    /// The id of the changed row.
    pub entity_id: EntityId,
    /// The root of the aggregate.
    pub aggregate_type: AggregateType,
    /// The id of the root of the aggregate.
    pub aggregate_id: AggregateId,
    /// The label of the entity from its current row.
    pub label: EntityLabel,
}

/// One audit entry with its states and its trace fields (ADR-0051 (f)).
///
/// The record holds no `entry_hash` and no `prev_hash`. A state never holds `password_hash`.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditRecord {
    /// The fields of the list row.
    pub row: AuditRow,
    /// The request that wrote the entry.
    pub request_id: RequestId,
    /// The state before the change.
    pub before: Option<Value>,
    /// The state after the change.
    pub after: Option<Value>,
    /// The position of the entry in the hash chain.
    pub chain_seq: i64,
    /// The client address of the request.
    pub client_ip: Option<IpAddr>,
    /// The session reference of the request.
    pub session_ref: Option<SessionRef>,
}

/// One page of a cursor list, newest row first.
///
/// The type solves one problem: a cursor list reads one row more than the page, and every caller
/// must cut it in the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorPage<T> {
    /// The rows of the page, newest first.
    pub rows: Vec<T>,
    /// Whether an older row exists after the last row of the page.
    pub has_older: bool,
}

impl<T> CursorPage<T> {
    /// Build a page from a window of up to `limit + 1` rows, newest first.
    #[must_use]
    pub fn cut(mut rows: Vec<T>, limit: u32) -> Self {
        let size = usize::try_from(limit).unwrap_or(usize::MAX);
        let has_older = rows.len() > size;
        rows.truncate(size);
        Self { rows, has_older }
    }

    /// Return the page without a row.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            rows: Vec::new(),
            has_older: false,
        }
    }
}

/// The subject of the filter `login` of the security events (ADR-0051 (h)).
///
/// A row matches when its user has `login`, or when its `login_hmac` equals `hmac`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginMatch {
    /// The login of an account. `None` when the typed login breaks the grammar of `Login`.
    pub login: Option<Login>,
    /// The HMAC of the typed login under the current key of the key ring.
    pub hmac: LoginHmac,
}

/// The filter of the security event list (ADR-0051 (h)). Each `None` keeps every row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecurityEventFilter {
    /// The kind of the rows.
    pub kind: Option<SecurityEventKind>,
    /// The subject of the rows.
    pub login: Option<LoginMatch>,
    /// The client address of the rows.
    pub client_ip: Option<IpAddr>,
    /// The range of the times of the rows.
    pub range: TimeRange,
}

/// One row of the security event list. It holds no `login_hmac` (R35, A33).
#[derive(Clone, Debug, PartialEq)]
pub struct SecurityEventRow {
    /// The id of the row.
    pub id: SecurityEventId,
    /// The time of the first event of the row.
    pub occurred_at: Timestamp,
    /// The kind of the events.
    pub kind: SecurityEventKind,
    /// The number of events of the row (`data-model.md` section 3.8).
    pub count: u32,
    /// The client address of the events.
    pub client_ip: Option<IpAddr>,
    /// The kept User-Agent header of the first event.
    pub user_agent: Option<UserAgent>,
    /// The request of the first event.
    pub request_id: RequestId,
    /// The login of the user of the row, or `None` for an unknown login.
    pub login: Option<Login>,
    /// The detail object of the row.
    pub detail: Option<Value>,
}

/// Read port of the audit views.
///
/// The port solves one problem: the audit pages read pages of a log that grows without an end.
/// Each list reads the rows with an id below the cursor, newest first, and at most `limit + 1`
/// rows. It returns at most `limit` rows and whether an older row exists. No list reads a state.
#[async_trait]
pub trait AuditReads: Send {
    /// Read one page of the audit entries in `scope` that match `filter`.
    ///
    /// `before` keeps the entries with a smaller id. A `filter.actor` that names no user gives the
    /// empty page.
    async fn entries(
        &mut self,
        filter: &AuditFilter,
        scope: AuditScope,
        before: Option<AuditEntryId>,
        limit: u32,
    ) -> Result<CursorPage<AuditRow>, StorageError>;

    /// Read the entry `id` with its states. An absent entry, or an entry outside `scope`, gives `None`.
    async fn entry(
        &mut self,
        id: AuditEntryId,
        scope: AuditScope,
    ) -> Result<Option<AuditRecord>, StorageError>;

    /// Read one page of the entries with the aggregate id of `recipe`, newest first.
    async fn history(
        &mut self,
        recipe: RecipeId,
        before: Option<AuditEntryId>,
        limit: u32,
    ) -> Result<CursorPage<AuditRow>, StorageError>;

    /// Read one page of the security event rows that match `filter`.
    ///
    /// The rows of `login_failed`, `login_throttled`, and `csrf_rejected` aggregate per client
    /// address, kind, and minute (`data-model.md` section 3.8).
    async fn security_events(
        &mut self,
        filter: &SecurityEventFilter,
        before: Option<SecurityEventId>,
        limit: u32,
    ) -> Result<CursorPage<SecurityEventRow>, StorageError>;

    /// Read the label of the root `id` of an aggregate of `aggregate_type` from its current row.
    ///
    /// The label has the rules of `EntityLabel`. A root that does not exist gives `Missing`.
    async fn aggregate_label(
        &mut self,
        aggregate_type: AggregateType,
        id: AggregateId,
    ) -> Result<EntityLabel, StorageError>;
}
