use super::OPEN_TRANSACTION_SQL;

const PROBE: &str = "transaction_timestamp() <> statement_timestamp()";

#[test]
fn the_release_probe_compares_the_two_timestamps_with_not_equal() {
    assert!(
        OPEN_TRANSACTION_SQL.contains(PROBE),
        "probe text: {OPEN_TRANSACTION_SQL}"
    );
}

#[cfg(feature = "test-support")]
#[test]
fn the_session_facts_probe_compares_the_two_timestamps_with_not_equal() {
    assert!(
        super::SESSION_FACTS_SQL.contains(PROBE),
        "probe text: {}",
        super::SESSION_FACTS_SQL
    );
}
