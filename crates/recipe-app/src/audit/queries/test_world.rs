//! The test world of the audit queries: users, a machine, a recipe, and eight entries.
//!
//! | id | time (UTC)        | action              | entity                   | actor     |
//! |----|-------------------|---------------------|--------------------------|-----------|
//! | 1  | 2026-10-15 23:59  | `MachineRenamed`    | machine `M12`            | admin     |
//! | 2  | 2026-10-16 00:00  | `RecipeRenamed`     | recipe `R-100`           | jkowalski |
//! | 3  | 2026-10-16 10:00  | `ParameterValueSet` | parameter `SPEED_TABLE`  | jkowalski |
//! | 4  | 2026-10-16 10:01  | `LimitsReplaced`    | parameter `SPEED_TABLE`  | jkowalski |
//! | 5  | 2026-10-16 10:02  | `ParameterValueSet` | parameter `HOLD`         | jkowalski |
//! | 6  | 2026-10-16 10:03  | `RecipeDeleted`     | recipe `R-300`           | admin     |
//! | 7  | 2026-10-16 10:04  | `UserCreated`       | user `jsmith`            | admin     |
//! | 8  | 2026-10-17 00:00  | `RecipeRenamed`     | recipe `R-100`           | jkowalski |

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, DiffAction, RequestId, RowAction, RowChange, SessionRef,
};
use recipe_domain::identity::{Login, PasswordHash, Role};
use recipe_domain::plant::PlcPlatform;
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::{Date, Month, OffsetDateTime, Time};

use crate::blocking::{AppSettings, PageSizes};
use crate::context::AppContext;
use crate::memory::fixtures::{Fixture, mid, pid, rid, user, uuid};
use crate::ports::{AuditEntryId, Isolation, NewUser, NonEmpty, UnitOfWork};

/// The number of the recipe `R-100`, which exists in the store.
pub(super) const R100: u128 = 100;
/// The number of the deleted recipe `R-300`, which has only its entry.
pub(super) const R300: u128 = 300;
/// The number of the machine `M12`.
pub(super) const M12: u128 = 12;
/// The number of the parameter `SPEED_TABLE`.
pub(super) const SPEED_TABLE: u128 = 1;
/// The number of the parameter `HOLD`.
pub(super) const HOLD: u128 = 2;
/// The number of the user `jsmith`.
pub(super) const JSMITH: u128 = 5;

/// Return the instant of October `day`, 2026, at `hour`:`minute` UTC.
pub(super) fn october(day: u8, hour: u8, minute: u8) -> Timestamp {
    let date = Date::from_calendar_date(2026, Month::October, day).unwrap();
    let time = Time::from_hms(hour, minute, 0).unwrap();
    Timestamp::from(OffsetDateTime::new_utc(date, time))
}

/// Return the entry id `n`.
pub(super) fn entry_id(n: u64) -> AuditEntryId {
    AuditEntryId::from(std::num::NonZeroU64::new(n).unwrap())
}

/// Return the ids of `rows`, in their order.
pub(super) fn ids<'a>(rows: impl IntoIterator<Item = &'a crate::ports::AuditRow>) -> Vec<u64> {
    rows.into_iter().map(|row| row.id.get()).collect()
}

/// Return the login `text`.
pub(super) fn login(text: &str) -> Login {
    Login::parse(text).unwrap()
}

/// Store the user `n` with `text` as login and `role`.
pub(super) fn seed_user(fixture: &Fixture, n: u128, text: &str, role: Role) {
    fixture.store.seed_user(&NewUser {
        id: user(n),
        login: login(text),
        password_hash: PasswordHash::parse("$argon2id$fake$secret").unwrap(),
        role,
        must_change_password: false,
    });
}

/// Return the audit context of a request of user `n` from `192.0.2.10` with a session.
fn context(n: u128) -> AuditContext {
    AuditContext {
        actor: Some(user(n)),
        request_id: RequestId::from(uuid(0x0100_u128.checked_add(n).unwrap())),
        client_ip: Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))),
        session_ref: Some(SessionRef::parse("0123456789abcdef").unwrap()),
    }
}

/// Return an update entry of `action` on `target` by user `actor`.
fn updated(action: RowAction, target: AuditTarget, actor: u128) -> AuditEntry {
    let change = RowChange::Update {
        before: json!({"name": "old"}),
        after: json!({"name": "new"}),
    };
    AuditEntry::row(action, target, change, context(actor))
}

/// Return a diff entry of `action` on the parameter `parameter` of `R-100`.
fn parameter_diff(action: DiffAction, parameter: u128) -> AuditEntry {
    let target = AuditTarget::parameter(rid(R100), pid(parameter));
    AuditEntry::diff(
        action,
        target,
        json!({"value": "1"}),
        json!({"value": "2"}),
        context(2),
    )
}

/// Return the entry `UserCreated` of `jsmith`. Its state holds no `password_hash`.
fn user_created() -> AuditEntry {
    let after =
        json!({"id": user(JSMITH).as_uuid().to_string(), "login": "jsmith", "role": "Operator"});
    let target = AuditTarget::app_user(user(JSMITH));
    AuditEntry::row(
        RowAction::UserCreated,
        target,
        RowChange::Create { after },
        context(1),
    )
}

/// Return the eight entries of the module table with their commit times.
fn entries() -> Vec<(Timestamp, AuditEntry)> {
    let recipe = AuditTarget::recipe(rid(R100));
    vec![
        (
            october(15, 23, 59),
            updated(RowAction::MachineRenamed, AuditTarget::machine(mid(M12)), 1),
        ),
        (
            october(16, 0, 0),
            updated(RowAction::RecipeRenamed, recipe.clone(), 2),
        ),
        (
            october(16, 10, 0),
            parameter_diff(DiffAction::ParameterValueSet, SPEED_TABLE),
        ),
        (
            october(16, 10, 1),
            parameter_diff(DiffAction::LimitsReplaced, SPEED_TABLE),
        ),
        (
            october(16, 10, 2),
            parameter_diff(DiffAction::ParameterValueSet, HOLD),
        ),
        (
            october(16, 10, 3),
            updated(RowAction::RecipeDeleted, AuditTarget::recipe(rid(R300)), 1),
        ),
        (october(16, 10, 4), user_created()),
        (
            october(17, 0, 0),
            updated(RowAction::RecipeRenamed, recipe, 2),
        ),
    ]
}

/// Commit `entry` in one transaction that begins at `instant`.
pub(super) async fn commit_at(fixture: &Fixture, instant: Timestamp, entry: AuditEntry) {
    fixture.clock.set(instant);
    let uow: &dyn UnitOfWork = &fixture.store;
    let tx = uow.begin(Isolation::ReadCommitted).await.unwrap();
    uow.commit(tx, NonEmpty::new(entry)).await.unwrap();
}

/// Return the fixture with the users, `M12`, `R-100`, and the eight entries.
pub(super) async fn world() -> Fixture {
    let fixture = Fixture::new();
    seed_user(&fixture, 1, "admin", Role::Administrator);
    seed_user(&fixture, 2, "jkowalski", Role::ProcessEngineer);
    seed_user(&fixture, 4, "mhoffman", Role::Operator);
    seed_user(&fixture, JSMITH, "jsmith", Role::Operator);
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, M12, "M12");
    fixture.seed_draft(&machine, R100, "R-100");
    for (instant, entry) in entries() {
        commit_at(&fixture, instant, entry).await;
    }
    fixture
}

/// Return a context over `fixture` with the page sizes `pages`.
///
/// A test sets only the size of the query under test. A query that reads another size then
/// returns a page of the default size, and the test fails.
pub(super) fn context_with_pages(fixture: &Fixture, pages: PageSizes) -> AppContext {
    fixture.context_with(AppSettings {
        pages,
        ..AppSettings::default()
    })
}
