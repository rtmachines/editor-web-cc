use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;

use recipe_app::error::StorageError;
use recipe_domain::error_codes::ErrorCode;
use sqlx::error::{DatabaseError, ErrorKind};

use recipe_domain::recipe::RecipeVersionId;
use uuid::Uuid;

use super::{corrupt, is_lock_timeout, map_error, missing_row, without_lock};

/// A database error with a chosen SQLSTATE, message, and constraint name.
#[derive(Debug)]
struct FakeDatabaseError {
    sqlstate: &'static str,
    message: &'static str,
    constraint: Option<&'static str>,
}

impl fmt::Display for FakeDatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("fake database error")
    }
}

impl StdError for FakeDatabaseError {}

impl DatabaseError for FakeDatabaseError {
    fn message(&self) -> &str {
        self.message
    }

    fn code(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.sqlstate))
    }

    fn as_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn StdError + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn StdError + Send + Sync + 'static> {
        self
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }

    fn constraint(&self) -> Option<&str> {
        self.constraint
    }
}

fn database_error(sqlstate: &'static str) -> sqlx::Error {
    database_error_with_message(sqlstate, "fake database error")
}

fn database_error_with_message(sqlstate: &'static str, message: &'static str) -> sqlx::Error {
    sqlx::Error::Database(Box::new(FakeDatabaseError {
        sqlstate,
        message,
        constraint: None,
    }))
}

fn database_error_with_constraint(sqlstate: &'static str, constraint: &'static str) -> sqlx::Error {
    sqlx::Error::Database(Box::new(FakeDatabaseError {
        sqlstate,
        message: "fake database error",
        constraint: Some(constraint),
    }))
}

#[test]
fn check_violation_with_a_trigger_code_gives_that_code_and_its_message() {
    let error = database_error_with_message("23514", "E-RCP-031 version 7 is not a live draft");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::Constraint { code, message }
            if *code == ErrorCode::RCP_031 && message.as_str() == "The version is not a live draft."),
        "{mapped:?}"
    );
}

#[test]
fn check_violation_without_a_trigger_code_gives_e_db_003() {
    let error = database_error_with_message("23514", "new row violates check constraint");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::Constraint { code, message }
            if *code == ErrorCode::DB_003 && message.as_str() == "The change violates a database rule."),
        "{mapped:?}"
    );
}

#[test]
fn a_code_outside_the_trigger_ranges_gives_e_db_003() {
    let error = database_error_with_message("23514", "E-PLT-001 duplicate machine code");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::Constraint { code, .. } if *code == ErrorCode::DB_003),
        "{mapped:?}"
    );
}

#[test]
fn lock_not_available_maps_to_deadlock() {
    let error = database_error("55P03");

    assert!(is_lock_timeout(&error));
    assert!(matches!(map_error(error), StorageError::Deadlock));
}

#[test]
fn another_sqlstate_is_not_a_lock_timeout_and_keeps_its_code() {
    let error = database_error("42P01");

    assert!(!is_lock_timeout(&error));
    let mapped = map_error(error);
    assert!(
        matches!(&mapped, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("42P01")),
        "{mapped:?}"
    );
}

#[test]
fn connection_failures_map_to_unavailable() {
    let io = sqlx::Error::Io(std::io::Error::other("connection refused"));

    assert!(matches!(map_error(io), StorageError::Unavailable));
    assert!(matches!(
        map_error(sqlx::Error::PoolClosed),
        StorageError::Unavailable
    ));
}

#[test]
fn an_error_without_a_sqlstate_maps_to_other_without_a_code() {
    let mapped = map_error(sqlx::Error::RowNotFound);

    assert!(
        matches!(&mapped, StorageError::Other { sqlstate: None, .. }),
        "{mapped:?}"
    );
}

#[test]
fn a_trigger_code_decides_whatever_the_sqlstate() {
    // `RAISE EXCEPTION` without a USING clause gives P0001. The trigger code still decides.
    for sqlstate in ["P0001", "23505", "23503", "40001", "XX000"] {
        let error = database_error_with_message(sqlstate, "E-PLT-031 the machine has a recipe");

        let mapped = map_error(error);

        assert!(
            matches!(&mapped, StorageError::Constraint { code, .. } if *code == ErrorCode::PLT_031),
            "{sqlstate}: {mapped:?}"
        );
    }
}

#[test]
fn a_code_outside_the_trigger_ranges_with_another_sqlstate_gives_other() {
    let error = database_error_with_message("P0001", "E-PLT-001 duplicate machine code");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("P0001")),
        "{mapped:?}"
    );
}

#[test]
fn a_unique_violation_gives_duplicate_with_the_constraint_name() {
    let error = database_error_with_constraint("23505", "recipe_reference_unique");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::Duplicate { constraint } if constraint == "recipe_reference_unique"),
        "{mapped:?}"
    );
}

#[test]
fn a_foreign_key_violation_gives_missing_reference_with_the_constraint_name() {
    let error = database_error_with_constraint("23503", "unit_parent_exists");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::MissingReference { constraint } if constraint == "unit_parent_exists"),
        "{mapped:?}"
    );
}

#[test]
fn a_violation_without_a_constraint_name_gives_other_with_its_sqlstate() {
    for sqlstate in ["23505", "23503"] {
        let mapped = map_error(database_error(sqlstate));

        assert!(
            matches!(&mapped, StorageError::Other { sqlstate: Some(state), .. } if state == sqlstate),
            "{sqlstate}: {mapped:?}"
        );
    }
}

#[test]
fn a_deadlock_and_a_serialization_failure_give_deadlock() {
    for sqlstate in ["40P01", "40001"] {
        let mapped = map_error(database_error(sqlstate));

        assert!(matches!(mapped, StorageError::Deadlock), "{sqlstate}");
    }
}

#[test]
fn a_canceled_statement_gives_statement_timeout() {
    let mapped = map_error(database_error("57014"));

    assert!(
        matches!(mapped, StorageError::StatementTimeout),
        "{mapped:?}"
    );
}

#[test]
fn a_pool_acquire_timeout_gives_pool_timeout() {
    let mapped = map_error(sqlx::Error::PoolTimedOut);

    assert!(matches!(mapped, StorageError::PoolTimeout), "{mapped:?}");
}

#[test]
fn every_connection_failure_gives_unavailable() {
    let failures = [
        sqlx::Error::Io(std::io::Error::other("reset by peer")),
        sqlx::Error::Tls(Box::new(std::io::Error::other("handshake failed"))),
        sqlx::Error::PoolClosed,
        sqlx::Error::WorkerCrashed,
    ];
    for failure in failures {
        let text = format!("{failure:?}");

        let mapped = map_error(failure);

        assert!(matches!(mapped, StorageError::Unavailable), "{text}");
    }
}

#[test]
fn every_sqlstate_of_a_lost_connection_gives_unavailable() {
    // Class 08 (all codes), admin shutdown, crash shutdown, cannot connect now, too many connections.
    let sqlstates = [
        "08000", "08001", "08003", "08004", "08006", "08007", "08P01", "57P01", "57P02", "57P03",
        "53300",
    ];
    for sqlstate in sqlstates {
        let mapped = map_error(database_error(sqlstate));

        assert!(
            matches!(mapped, StorageError::Unavailable),
            "{sqlstate}: {mapped:?}"
        );
    }
}

#[test]
fn a_sqlstate_next_to_the_connection_classes_stays_other() {
    for sqlstate in ["09000", "0A000", "57P04", "53200", "53400"] {
        let mapped = map_error(database_error(sqlstate));

        assert!(
            matches!(&mapped, StorageError::Other { sqlstate: Some(state), .. } if state == sqlstate),
            "{sqlstate}: {mapped:?}"
        );
    }
}

#[test]
fn a_trigger_code_still_wins_over_a_connection_sqlstate() {
    let error = database_error_with_message("08006", "E-PLT-031 the machine has a recipe");

    let mapped = map_error(error);

    assert!(
        matches!(&mapped, StorageError::Constraint { code, .. } if *code == ErrorCode::PLT_031),
        "{mapped:?}"
    );
}

#[test]
fn a_corrupt_row_names_its_table_and_gives_e_db_008() {
    let mapped = corrupt("machine", "the code is empty");

    assert_eq!(mapped.code(), ErrorCode::DB_008);
    assert!(
        matches!(&mapped, StorageError::Other { sqlstate: None, .. }),
        "{mapped:?}"
    );
    let cause = mapped.source().unwrap().to_string();
    assert!(cause.contains("machine"), "{cause}");
    assert!(cause.contains("the code is empty"), "{cause}");
}

#[test]
fn a_missing_row_names_its_table_and_gives_e_db_008() {
    let mapped = missing_row("parameter");

    assert_eq!(mapped.code(), ErrorCode::DB_008);
    let cause = mapped.source().unwrap().to_string();
    assert!(cause.contains("parameter"), "{cause}");
}

#[test]
fn a_status_write_without_a_lock_names_the_version_and_gives_e_db_008() {
    let id = Uuid::from_u128(0x1234);
    let mapped = without_lock(RecipeVersionId::from(id));

    assert_eq!(mapped.code(), ErrorCode::DB_008);
    let cause = mapped.source().unwrap().to_string();
    assert!(cause.contains(&id.to_string()), "{cause}");
}

#[test]
fn another_database_error_keeps_the_original_error_as_its_source() {
    let mapped = map_error(database_error("42P01"));

    let original = mapped
        .source()
        .unwrap()
        .downcast_ref::<sqlx::Error>()
        .unwrap();
    assert!(matches!(original, sqlx::Error::Database(_)), "{original:?}");
}

#[test]
fn a_driver_error_without_a_database_part_keeps_the_original_error_as_its_source() {
    let mapped = map_error(sqlx::Error::RowNotFound);

    let original = mapped
        .source()
        .unwrap()
        .downcast_ref::<sqlx::Error>()
        .unwrap();
    assert!(matches!(original, sqlx::Error::RowNotFound), "{original:?}");
}
