use std::io;

use recipe_app::error::StorageError;

use super::coded;

const STORAGE_OTHER_MESSAGE: &str =
    "An unexpected database error occurred. Contact the administrator.";

fn storage_other(cause: &'static str) -> StorageError {
    StorageError::Other {
        sqlstate: None,
        source: Box::new(io::Error::other(cause)),
    }
}

#[test]
fn the_alternate_text_starts_with_the_code_and_holds_the_source() {
    let error = coded(storage_other("connection reset by peer"));

    let text = format!("{error:#}");

    assert_eq!(
        text,
        format!("E-DB-008: {STORAGE_OTHER_MESSAGE}: connection reset by peer")
    );
}

#[test]
fn the_plain_text_is_the_code_only() {
    let error = coded(storage_other("connection reset by peer"));

    assert_eq!(error.to_string(), "E-DB-008");
}

#[test]
fn an_error_without_a_source_gives_the_code_and_the_message_only() {
    let error = coded(StorageError::Unavailable);

    let text = format!("{error:#}");

    assert_eq!(text, "E-DB-005: The database does not answer. Try again.");
}

#[test]
fn the_debug_chain_keeps_the_source() {
    let error = coded(storage_other("connection reset by peer"));

    let causes: Vec<String> = error.chain().map(ToString::to_string).collect();

    assert_eq!(causes.len(), 3, "{causes:?}");
    assert_eq!(
        causes.last().map(String::as_str),
        Some("connection reset by peer")
    );
}
