use recipe_app::error::StorageError;

use super::{bind_offset, bind_window};

#[test]
fn an_offset_above_i64_max_is_a_storage_error_with_the_offset() {
    let beyond = u64::try_from(i64::MAX).unwrap().checked_add(1).unwrap();

    let error = bind_offset(beyond).unwrap_err();

    assert!(matches!(
        &error,
        StorageError::Other { sqlstate: None, source } if source.to_string().contains(&beyond.to_string())
    ));
}

#[test]
fn the_largest_offset_that_bigint_holds_binds_unchanged() {
    let largest = u64::try_from(i64::MAX).unwrap();

    assert_eq!(bind_offset(largest).unwrap(), i64::MAX);
    assert_eq!(bind_offset(0).unwrap(), 0);
}

#[test]
fn the_offset_of_a_page_is_the_product_of_the_page_and_the_size() {
    assert_eq!(super::page_offset(3, 100).unwrap(), 300);
    assert_eq!(
        super::page_offset(u32::MAX, u32::MAX).unwrap(),
        u64::from(u32::MAX)
            .checked_mul(u64::from(u32::MAX))
            .unwrap()
    );
}

#[test]
fn the_window_binds_the_offset_and_the_limit() {
    assert_eq!(bind_window(200, 101).unwrap(), (200, 101));
    assert!(bind_window(u64::MAX, 1).is_err());
}
