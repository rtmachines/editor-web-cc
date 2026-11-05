use recipe_domain::audit::SecurityEventKind;

use super::statement_for;

#[test]
fn exactly_the_three_aggregated_kinds_get_the_conflict_clause() {
    for kind in SecurityEventKind::ALL {
        let aggregated = matches!(
            kind,
            SecurityEventKind::LoginFailed
                | SecurityEventKind::LoginThrottled
                | SecurityEventKind::CsrfRejected
        );
        let statement = statement_for(kind);
        assert_eq!(
            statement.contains("ON CONFLICT"),
            aggregated,
            "{}",
            kind.as_sql()
        );
    }
}
