use recipe_app::ports::Isolation;

use super::begin_statement;

#[test]
fn each_isolation_begins_with_its_level_and_its_access_mode() {
    let expected = [
        (
            Isolation::ReadCommitted,
            "BEGIN ISOLATION LEVEL READ COMMITTED",
        ),
        (
            Isolation::ReadOnly,
            "BEGIN ISOLATION LEVEL READ COMMITTED READ ONLY",
        ),
        (
            Isolation::Serializable,
            "BEGIN ISOLATION LEVEL SERIALIZABLE",
        ),
        (
            Isolation::RepeatableRead,
            "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
        ),
    ];
    for (isolation, statement) in expected {
        assert_eq!(begin_statement(isolation), statement, "{isolation:?}");
    }
}
