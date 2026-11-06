use super::{DELETE_SESSIONS, PURGE_ATTEMPTS};

/// The lock step of D4 of `low-round-c`: both deletes lock their rows in `id` order.
#[test]
fn the_cleanup_deletes_lock_their_rows_in_id_order() {
    for statement in [DELETE_SESSIONS, PURGE_ATTEMPTS] {
        assert!(statement.contains("ORDER BY id FOR UPDATE"), "{statement}");
        assert!(
            statement.contains("WHERE id IN (SELECT id FROM locked)"),
            "{statement}"
        );
    }
}
