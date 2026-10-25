use recipe_app::ports::TimeStateKey;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::{SnapshotRow, into_snapshot, parse_state, unsigned, user_management_operations};

fn row() -> SnapshotRow {
    (
        OffsetDateTime::UNIX_EPOCH,
        true,
        Some(7),
        3,
        4,
        None,
        None,
        None,
    )
}

#[test]
fn user_management_operations_name_the_user_use_cases_only() {
    let names = user_management_operations();

    for expected in ["CreateUser", "ListUsers", "SetUserRole", "ResetPassword"] {
        assert!(names.contains(&expected), "{expected}");
    }
    for other in ["RenameMachine", "Login", "UnlockLogin", "ChangeOwnPassword"] {
        assert!(!names.contains(&other), "{other}");
    }
}

#[test]
fn a_row_becomes_a_snapshot() {
    let snapshot = into_snapshot(row()).unwrap();

    assert_eq!(snapshot.failed_logins, 3);
    assert_eq!(snapshot.security_alerts, 4);
    assert!(snapshot.replication.connected);
    assert_eq!(snapshot.replication.lag_bytes, Some(7));
    assert_eq!(
        snapshot.database_now,
        Timestamp::from(OffsetDateTime::UNIX_EPOCH)
    );
}

#[test]
fn a_negative_count_or_lag_is_a_defect() {
    assert!(unsigned(-1).is_err());
    let mut negative_count = row();
    negative_count.3 = -1;
    assert!(into_snapshot(negative_count).is_err());
    let mut negative_lag = row();
    negative_lag.2 = Some(-5);
    assert!(into_snapshot(negative_lag).is_err());
}

#[test]
fn a_bad_state_text_is_a_defect_that_hides_the_text() {
    let error = parse_state(TimeStateKey::HeartbeatLastAt, "secret-looking-text").unwrap_err();

    let shown = format!("{error:?}");
    assert!(shown.contains("heartbeat.last_at"), "{shown}");
    assert!(!shown.contains("secret-looking-text"), "{shown}");
}

#[test]
fn a_state_text_in_the_form_of_adr_0073_parses() {
    let parsed = parse_state(
        TimeStateKey::AuditLastCheckAt,
        "1970-01-01T00:00:00.000000Z",
    );

    assert_eq!(parsed.unwrap(), Timestamp::from(OffsetDateTime::UNIX_EPOCH));
}
