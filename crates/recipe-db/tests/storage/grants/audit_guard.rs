//! The static test of design D11: no migration file drops, alters, or truncates an audit object
//! outside the five steps of `data-model.md` section 5.1.
//!
//! `recipe_migrator` owns the schema `public`, so the database cannot stop such a statement.
//! The guard reads the text of a migration file. Two cases are allowed. One is the transfer
//! `ALTER ... OWNER TO audit_owner` of section 5.2. The other is every statement between
//! `SET ROLE audit_owner` and `RESET ROLE`, which is step 3 of the five steps.

use super::sql_tokens::{Token, split_statements};
use crate::support::migration_texts;

const AUDIT_OBJECTS: [&str; 8] = [
    "audit_log",
    "audit_chain_head",
    "security_event",
    "audit_canonical",
    "audit_hash_chain",
    "security_event_purge",
    "security_event_guard",
    "replication_state",
];
const OWNER: &str = "audit_owner";

/// True for `ALTER ... OWNER TO audit_owner`.
fn is_transfer_to_owner(words: &[String]) -> bool {
    let tail: Vec<&str> = words.iter().rev().take(3).map(String::as_str).collect();
    tail == [OWNER, "to", "owner"]
}

/// The target of `SET [SESSION|LOCAL] ROLE <target>`, or `None` for another statement.
fn set_role_target(words: &[String]) -> Option<&str> {
    let mut rest = words.iter().map(String::as_str).skip(1).peekable();
    rest.next_if(|word| matches!(*word, "session" | "local"));
    (rest.next()? == "role").then(|| rest.next().unwrap_or_default())
}

/// What a statement does to the open `SET ROLE audit_owner` span, and whether it is a violation.
fn check_statement(statement: &[Token], in_span: &mut bool) -> bool {
    let words: Vec<String> = statement.iter().map(Token::lowercase).collect();
    let word_refs: Vec<&str> = words.iter().map(String::as_str).collect();
    match word_refs.as_slice() {
        ["set", ..] if set_role_target(&words).is_some() => {
            let to_owner = set_role_target(&words) == Some(OWNER);
            *in_span = to_owner;
            return !to_owner;
        }
        ["set" | "reset", "session", "authorization", ..] => return true,
        ["reset", "role"] => return !std::mem::replace(in_span, false),
        _ => {}
    }
    if *in_span {
        return false;
    }
    let first = word_refs.first().copied().unwrap_or_default();
    let touches_audit = statement.iter().any(|token| {
        matches!(token, Token::Word(word) if AUDIT_OBJECTS.contains(&word.to_ascii_lowercase().as_str()))
    });
    match first {
        "do" => true,
        "drop" => {
            word_refs.get(1) == Some(&"schema") || word_refs.contains(&"cascade") || touches_audit
        }
        "alter" | "truncate" => touches_audit && !is_transfer_to_owner(&words),
        _ => false,
    }
}

/// The statements of a migration text that the guard rejects. Each entry is the statement with
/// single spaces. A literal shows as `<literal>`. An unterminated literal or comment is one entry.
pub fn audit_violations(text: &str) -> Vec<String> {
    let (statements, terminated) = split_statements(text);
    let mut in_span = false;
    let mut violations = Vec::new();
    for statement in &statements {
        if check_statement(statement, &mut in_span) {
            let shown: Vec<String> = statement.iter().map(Token::shown).collect();
            violations.push(shown.join(" "));
        }
    }
    if !terminated {
        violations.push("unterminated literal or comment".to_owned());
    }
    violations
}

#[test]
fn every_migration_file_obeys_the_audit_guard() {
    let files = migration_texts();

    assert!(files.len() >= 5, "migration files: {}", files.len());
    for (name, text) in &files {
        assert_eq!(audit_violations(text), Vec::<String>::new(), "{name}");
    }
}

/// Assert that the guard accepts one migration file and that the file holds the five steps.
fn assert_guard_accepts_the_five_steps_of(file: &str) {
    let files = migration_texts();
    let found = files.iter().find(|(name, _)| name == file);
    assert!(found.is_some(), "migration {file} is missing");
    let (_, text) = found.unwrap();

    assert_eq!(audit_violations(text), Vec::<String>::new());
    let steps = [
        "GRANT CREATE ON SCHEMA public TO audit_owner;",
        "SET ROLE audit_owner;",
        "RESET ROLE;",
        "REVOKE CREATE ON SCHEMA public FROM audit_owner;",
    ];
    let positions: Vec<Option<usize>> = steps.iter().map(|step| text.find(step)).collect();
    assert!(positions.iter().all(Option::is_some), "{positions:?}");
    assert!(positions.is_sorted(), "{positions:?}");
}

#[test]
fn the_guard_accepts_migration_0005_and_sees_its_five_steps() {
    assert_guard_accepts_the_five_steps_of("0005_security_event_guard.sql");
}

#[test]
fn the_guard_accepts_migration_0006_and_sees_its_five_steps() {
    assert_guard_accepts_the_five_steps_of("0006_operations.sql");
}

#[test]
fn the_guard_rejects_a_dropped_replication_state_function() {
    let text = "DROP FUNCTION replication_state();";

    assert_eq!(audit_violations(text).len(), 1);
}

#[test]
fn the_guard_rejects_a_dropped_audit_table() {
    let text = "CREATE TABLE x (id INT);\nDROP TABLE audit_chain_head;\n";

    assert_eq!(
        audit_violations(text),
        ["DROP TABLE audit_chain_head".to_owned()]
    );
}

#[test]
fn the_guard_rejects_an_alter_and_a_truncate_outside_the_five_steps() {
    let text = "ALTER TABLE audit_log ADD COLUMN extra TEXT;\n\
                TRUNCATE security_event;\n\
                DROP FUNCTION audit_hash_chain();\n\
                ALTER TABLE audit_log OWNER TO recipe_migrator;";

    assert_eq!(audit_violations(text).len(), 4);
}

#[test]
fn the_guard_rejects_a_change_after_reset_role() {
    let text = "SET ROLE audit_owner;\nALTER TABLE audit_log ADD COLUMN a TEXT;\nRESET ROLE;\n\
                ALTER TABLE audit_log ADD COLUMN b TEXT;";

    assert_eq!(
        audit_violations(text),
        ["ALTER TABLE audit_log ADD COLUMN b TEXT".to_owned()]
    );
}

#[test]
fn the_guard_accepts_the_transfer_of_ownership_and_the_role_block() {
    let text = "ALTER TABLE audit_log OWNER TO audit_owner;\n\
                ALTER FUNCTION audit_canonical(audit_log) OWNER TO audit_owner;\n\
                SET ROLE audit_owner;\nDROP INDEX audit_log_action_idx;\nRESET ROLE;";

    assert_eq!(audit_violations(text), Vec::<String>::new());
}

#[test]
fn the_guard_rejects_drop_schema_with_cascade() {
    assert_eq!(
        audit_violations("DROP SCHEMA public CASCADE;"),
        ["DROP SCHEMA public CASCADE".to_owned()]
    );
}

#[test]
fn the_guard_rejects_any_drop_schema_and_any_drop_cascade() {
    let text = "DROP SCHEMA scratch;\nDROP TABLE IF EXISTS scratch_table CASCADE;";

    assert_eq!(audit_violations(text).len(), 2);
}

#[test]
fn the_guard_rejects_a_do_block() {
    let text = "DO $$ BEGIN EXECUTE 'DROP TABLE audit_log'; END $$;";

    assert_eq!(audit_violations(text), ["DO <literal>".to_owned()]);
}

#[test]
fn the_guard_rejects_set_role_none_after_the_role_block_opened() {
    let text = "SET ROLE audit_owner;\nSET ROLE NONE;\nDROP TABLE audit_log;";

    assert_eq!(
        audit_violations(text),
        [
            "SET ROLE NONE".to_owned(),
            "DROP TABLE audit_log".to_owned()
        ]
    );
}

#[test]
fn the_guard_rejects_a_set_role_to_another_role_and_a_lone_reset_role() {
    let text = "SET ROLE recipe_app;\nSET LOCAL ROLE 'recipe_app';\nRESET ROLE;\n\
                SET SESSION AUTHORIZATION postgres;";

    assert_eq!(audit_violations(text).len(), 4);
}

#[test]
fn the_guard_reads_a_block_comment_before_a_statement() {
    let text = "/* x */ DROP TABLE audit_log;";

    assert_eq!(audit_violations(text), ["DROP TABLE audit_log".to_owned()]);
}

#[test]
fn the_guard_reads_a_nested_block_comment() {
    let text = "/* a /* b */ DROP TABLE audit_log; */ SELECT 1;\nDROP TABLE audit_chain_head;";

    assert_eq!(
        audit_violations(text),
        ["DROP TABLE audit_chain_head".to_owned()]
    );
}

#[test]
fn the_guard_reads_a_literal_that_looks_like_a_comment() {
    let text = "SELECT '--'; DROP TABLE audit_log;";

    assert_eq!(audit_violations(text), ["DROP TABLE audit_log".to_owned()]);
}

#[test]
fn the_guard_reads_an_escaped_quote_inside_a_literal() {
    let text = "SELECT 'it''s; DROP TABLE audit_log; fine'; DROP TABLE security_event;";

    assert_eq!(
        audit_violations(text),
        ["DROP TABLE security_event".to_owned()]
    );
}

#[test]
fn the_guard_reads_a_backslash_escape_in_an_escape_string() {
    let text = "SELECT E'a\\'; DROP TABLE audit_log; b'; DROP TABLE security_event;";

    assert_eq!(
        audit_violations(text),
        ["DROP TABLE security_event".to_owned()]
    );
}

#[test]
fn the_guard_reads_a_quoted_identifier() {
    let text = "DROP TABLE \"audit_log\";";

    assert_eq!(audit_violations(text), ["DROP TABLE audit_log".to_owned()]);
}

#[test]
fn the_guard_rejects_an_unterminated_literal_or_comment() {
    for text in ["SELECT 'open; DROP TABLE x;", "/* open", "SELECT $a$ open"] {
        assert_eq!(audit_violations(text).len(), 1, "{text}");
    }
}

#[test]
fn the_guard_allows_the_transfer_inside_no_role_block_and_changes_inside_one() {
    let text = "ALTER TABLE security_event OWNER TO audit_owner;\n\
                SET ROLE audit_owner;\nDO $$ BEGIN NULL; END $$;\n\
                DROP SCHEMA IF EXISTS scratch CASCADE;\nRESET ROLE;";

    assert_eq!(audit_violations(text), Vec::<String>::new());
}

#[test]
fn the_guard_ignores_comments_and_function_bodies() {
    let text = "-- DROP TABLE audit_log;\n\
                CREATE FUNCTION f() RETURNS void LANGUAGE sql AS $body$ DROP TABLE audit_log; $body$;\n\
                CREATE FUNCTION g() RETURNS void LANGUAGE sql AS $$ TRUNCATE audit_log; $$;\n\
                SELECT $1;";

    assert_eq!(audit_violations(text), Vec::<String>::new());
}
