use super::DELETE_FOR_LOGIN;

/// The lock step of D4 of `low-round-c`: the delete locks its rows in `id` order.
#[test]
fn delete_for_login_locks_its_rows_in_id_order() {
    assert!(DELETE_FOR_LOGIN.contains("ORDER BY id FOR UPDATE"));
    assert!(DELETE_FOR_LOGIN.contains("WHERE id IN (SELECT id FROM locked)"));
}
