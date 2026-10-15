//! Contract cases of `list` and `summary_by_login` of `UserRepository` (design D9).

use recipe_app::error::StorageError;
use recipe_app::ports::Transaction;
use recipe_domain::identity::Role;

use crate::contract::harness::{Harness, begin, new_user, seed_user};

/// The logins in byte order. The harness user `harness-admin` is part of the list.
const LOGINS: [&str; 6] = ["akowal", "jkowalski", "mhoffman", "a_b", "aaa", "a-c"];

async fn seeded(h: &dyn Harness) -> Box<dyn Transaction> {
    for login in LOGINS {
        seed_user(h.unit_of_work().as_ref(), &new_user(login, Role::Operator)).await;
    }
    begin(h).await
}

async fn logins(
    tx: &mut Box<dyn Transaction>,
    filter: Option<&str>,
    page: u32,
    size: u32,
) -> Vec<String> {
    tx.users()
        .list(filter, page, size)
        .await
        .unwrap()
        .iter()
        .map(|user| user.login.to_string())
        .collect()
}

async fn list_orders_by_login_in_byte_order(h: &dyn Harness) {
    let mut tx = seeded(h).await;

    let all = logins(&mut tx, None, 0, 100).await;

    // A locale collation would put `aaa` before `a_b` and `a-c`, because it skips punctuation.
    assert_eq!(
        all,
        [
            "a-c",
            "a_b",
            "aaa",
            "akowal",
            "harness-admin",
            "jkowalski",
            "mhoffman"
        ]
    );
}

async fn list_filters_with_a_plain_substring(h: &dyn Harness) {
    let mut tx = seeded(h).await;

    let kowal = logins(&mut tx, Some("kowal"), 0, 100).await;
    let underscore = logins(&mut tx, Some("_"), 0, 100).await;
    let percent = logins(&mut tx, Some("%"), 0, 100).await;
    let none = logins(&mut tx, Some("zzz"), 0, 100).await;

    assert_eq!(kowal, ["akowal", "jkowalski"]);
    assert_eq!(underscore, ["a_b"]);
    assert!(percent.is_empty());
    assert!(none.is_empty());
}

async fn list_pages_count_from_zero(h: &dyn Harness) {
    let mut tx = seeded(h).await;

    let first = logins(&mut tx, None, 0, 3).await;
    let second = logins(&mut tx, None, 1, 3).await;
    let third = logins(&mut tx, None, 2, 3).await;
    let past_the_end = logins(&mut tx, None, 3, 3).await;

    assert_eq!(first, ["a-c", "a_b", "aaa"]);
    assert_eq!(second, ["akowal", "harness-admin", "jkowalski"]);
    assert_eq!(third, ["mhoffman"]);
    assert!(past_the_end.is_empty());
}

async fn list_with_an_offset_above_i64_max_is_a_storage_error(h: &dyn Harness) {
    let mut tx = seeded(h).await;

    let beyond = tx.users().list(None, u32::MAX, u32::MAX).await;

    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

async fn summary_by_login_gives_the_columns_of_the_query(h: &dyn Harness) {
    let user = new_user("akowal", Role::ProcessEngineer);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let mut tx = begin(h).await;
    tx.users().deactivate(user.id).await.unwrap();

    let summary = tx
        .users()
        .summary_by_login(&user.login)
        .await
        .unwrap()
        .expect("the user exists");
    let unknown = new_user("unknown", Role::Operator);
    let missing = tx.users().summary_by_login(&unknown.login).await.unwrap();

    assert_eq!(summary.id, user.id);
    assert_eq!(summary.login, user.login);
    assert_eq!(summary.role, Role::ProcessEngineer);
    assert!(!summary.is_active);
    assert!(!summary.must_change_password);
    assert!(summary.deactivated_at.is_some());
    assert!(missing.is_none());
}

contract_case!(
    list_orders_by_login_in_byte_order,
    list_orders_by_login_in_byte_order_memory,
    list_orders_by_login_in_byte_order_postgres
);
contract_case!(
    list_filters_with_a_plain_substring,
    list_filters_with_a_plain_substring_memory,
    list_filters_with_a_plain_substring_postgres
);
contract_case!(
    list_pages_count_from_zero,
    list_pages_count_from_zero_memory,
    list_pages_count_from_zero_postgres
);
contract_case!(
    list_with_an_offset_above_i64_max_is_a_storage_error,
    list_with_an_offset_above_i64_max_is_a_storage_error_memory,
    list_with_an_offset_above_i64_max_is_a_storage_error_postgres
);
contract_case!(
    summary_by_login_gives_the_columns_of_the_query,
    summary_by_login_gives_the_columns_of_the_query_memory,
    summary_by_login_gives_the_columns_of_the_query_postgres
);
