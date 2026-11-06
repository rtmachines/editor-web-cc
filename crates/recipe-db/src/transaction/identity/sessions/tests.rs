use super::DELETE_ALL_OF_USER;

/// The lock step of D4 of `low-round-c`: the delete locks its rows in `id` order.
#[test]
fn delete_all_of_user_locks_its_rows_in_id_order() {
    assert!(DELETE_ALL_OF_USER.contains("ORDER BY id FOR UPDATE"));
    assert!(DELETE_ALL_OF_USER.contains("WHERE id IN (SELECT id FROM locked)"));
}
