//! Contract cases of `TypeCatalog::ensure_derived`.

use recipe_app::error::StorageError;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcType, PlcTypeRecord, Primitive, StringKind};
use serde_json::json;

use crate::contract::harness::{Harness, begin};
use crate::contract::images::{assert_shape, field, text};
use crate::contract::seed::{
    array_record, ensure, insert, machine, seed_type, string_record, struct_record, type_id,
};
use crate::contract::type_images::{TypeKind, type_shape};

/// The name of the foreign key from a type row to its machine.
const TYPE_MACHINE_FKEY: &str = "plc_type_machine_id_fkey";
/// The name of the foreign key from an array row to its element row.
const TYPE_ELEMENT_FKEY: &str = "plc_type_element_type_id_fkey";

async fn ensure_derived_inserts_a_string_and_gives_its_image(h: &dyn Harness) {
    let built = machine(120, "M120");
    insert(h, &built).await;
    let record = string_record(&built, type_id(10), StringKind::String, 20);
    let mut tx = begin(h).await;

    let ensured = tx.types().ensure_derived(record.clone()).await.unwrap();
    crate::contract::harness::commit(h, tx, built.id()).await;

    assert_eq!(ensured.id, type_id(10));
    let image = ensured.created.expect("the call inserted the row");
    assert_shape(&image, &type_shape(TypeKind::String));
    assert_eq!(text(&image, "id"), type_id(10).to_string());
    assert_eq!(text(&image, "kind"), "primitive");
    assert_eq!(text(&image, "name"), "STRING");
    assert_eq!(text(&image, "leaf_class"), "string");
    assert_eq!(text(&image, "machine_id"), built.id().to_string());
    assert_eq!(field(&image, "string_length"), &json!(20));
    assert_eq!(field(&image, "size_bytes"), &json!(22));
    assert_eq!(field(&image, "platforms"), &json!(["TwinCat3"]));
    assert_eq!(field(&image, "revision"), &json!(1));
    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    let stored = rows.iter().find(|row| row.id() == type_id(10)).unwrap();
    assert_eq!(stored, &PlcType::try_from(record).unwrap());
}

async fn ensure_derived_inserts_a_wstring(h: &dyn Harness) {
    let built = machine(121, "M121");
    insert(h, &built).await;
    let record = string_record(&built, type_id(11), StringKind::Wstring, 40);

    let ensured = ensure(h, built.id(), record).await;

    let image = ensured.created.expect("the call inserted the row");
    assert_shape(&image, &type_shape(TypeKind::String));
    assert_eq!(text(&image, "name"), "WSTRING");
    assert_eq!(field(&image, "string_length"), &json!(40));
}

async fn a_second_string_of_one_length_gives_the_first_id_and_no_image(h: &dyn Harness) {
    let built = machine(122, "M122");
    insert(h, &built).await;
    let first = ensure(
        h,
        built.id(),
        string_record(&built, type_id(12), StringKind::String, 20),
    )
    .await;
    let mut tx = begin(h).await;

    let second = tx
        .types()
        .ensure_derived(string_record(&built, type_id(13), StringKind::String, 20))
        .await
        .unwrap();
    let longer = tx
        .types()
        .ensure_derived(string_record(&built, type_id(14), StringKind::String, 21))
        .await
        .unwrap();
    let wide = tx
        .types()
        .ensure_derived(string_record(&built, type_id(15), StringKind::Wstring, 20))
        .await
        .unwrap();

    assert!(first.created.is_some());
    assert_eq!(second.id, type_id(12));
    assert!(second.created.is_none());
    assert_eq!(longer.id, type_id(14));
    assert!(longer.created.is_some());
    assert_eq!(wide.id, type_id(15));
    assert!(wide.created.is_some());
}

async fn the_same_string_on_another_machine_is_a_new_row(h: &dyn Harness) {
    let first = machine(123, "M123");
    let second = machine(124, "M124");
    insert(h, &first).await;
    insert(h, &second).await;
    ensure(
        h,
        first.id(),
        string_record(&first, type_id(16), StringKind::String, 20),
    )
    .await;

    let other = ensure(
        h,
        second.id(),
        string_record(&second, type_id(17), StringKind::String, 20),
    )
    .await;

    assert_eq!(other.id, type_id(17));
    assert!(other.created.is_some());
}

async fn an_array_of_an_array_creates_one_row_for_each_level(h: &dyn Harness) {
    let built = machine(125, "M125");
    insert(h, &built).await;
    let int = seed_type(h, built.id(), Primitive::Int).await;
    let inner_record = array_record(&built, type_id(18), int, &[(1, 3)]);
    let inner = ensure(h, built.id(), inner_record.clone()).await;
    let outer_record = array_record(&built, type_id(19), inner.id, &[(1, 2), (0, 4)]);

    let outer = ensure(h, built.id(), outer_record.clone()).await;

    let inner_image = inner.created.expect("the inner array is new");
    assert_shape(&inner_image, &type_shape(TypeKind::Array));
    assert_eq!(text(&inner_image, "kind"), "array");
    assert_eq!(text(&inner_image, "element_type_id"), int.to_string());
    assert_eq!(field(&inner_image, "dim_lower"), &json!([1]));
    assert_eq!(field(&inner_image, "dim_upper"), &json!([3]));
    assert_eq!(field(&inner_image, "platforms"), &json!(["TwinCat3"]));
    let outer_image = outer.created.expect("the outer array is new");
    assert_shape(&outer_image, &type_shape(TypeKind::Array));
    assert_eq!(text(&outer_image, "element_type_id"), inner.id.to_string());
    assert_eq!(field(&outer_image, "dim_lower"), &json!([1, 0]));
    assert_eq!(field(&outer_image, "dim_upper"), &json!([2, 4]));
    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    for record in [inner_record, outer_record] {
        let wanted = PlcType::try_from(record).unwrap();
        assert!(rows.contains(&wanted), "{wanted:?}");
    }
}

async fn the_same_array_gives_the_first_id_and_no_image(h: &dyn Harness) {
    let built = machine(126, "M126");
    insert(h, &built).await;
    let int = seed_type(h, built.id(), Primitive::Int).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let first = ensure(
        h,
        built.id(),
        array_record(&built, type_id(20), int, &[(1, 3)]),
    )
    .await;
    let mut tx = begin(h).await;

    let again = tx
        .types()
        .ensure_derived(array_record(&built, type_id(21), int, &[(1, 3)]))
        .await
        .unwrap();
    let other_bounds = tx
        .types()
        .ensure_derived(array_record(&built, type_id(22), int, &[(1, 4)]))
        .await
        .unwrap();
    let other_element = tx
        .types()
        .ensure_derived(array_record(&built, type_id(23), dint, &[(1, 3)]))
        .await
        .unwrap();

    assert!(first.created.is_some());
    assert_eq!(again.id, type_id(20));
    assert!(again.created.is_none());
    assert!(other_bounds.created.is_some());
    assert!(other_element.created.is_some());
}

fn assert_database_rule(error: &StorageError) {
    assert!(
        matches!(error, StorageError::Constraint { code, message }
            if *code == ErrorCode::DB_003
                && message.as_str() == "The change violates a database rule."),
        "{error:?}"
    );
}

async fn a_struct_record_gives_the_database_rule_error(h: &dyn Harness) {
    let built = machine(127, "M127");
    insert(h, &built).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let record = struct_record(&built, type_id(24), "Pump", 1, None, &[("speed", dint)]);
    let mut tx = begin(h).await;

    let error = tx.types().ensure_derived(record).await.unwrap_err();

    assert_database_rule(&error);
}

async fn a_global_primitive_record_gives_the_database_rule_error(h: &dyn Harness) {
    let built = machine(128, "M128");
    insert(h, &built).await;
    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    let int: PlcTypeRecord = {
        let seed = rows
            .iter()
            .find(|row| {
                row.machine().is_none()
                    && matches!(row.shape(),
                        recipe_domain::plant::RecordShape::Primitive { primitive, .. }
                            if *primitive == Primitive::Int)
            })
            .unwrap();
        PlcTypeRecord {
            id: type_id(25),
            machine: None,
            platforms: seed.platforms().clone(),
            shape: seed.shape().clone(),
        }
    };

    let error = tx.types().ensure_derived(int).await.unwrap_err();

    assert_database_rule(&error);
}

async fn an_unknown_machine_gives_a_missing_reference(h: &dyn Harness) {
    let never_inserted = machine(129, "M129");
    let mut tx = begin(h).await;

    let error = tx
        .types()
        .ensure_derived(string_record(
            &never_inserted,
            type_id(26),
            StringKind::String,
            20,
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == TYPE_MACHINE_FKEY),
        "{error:?}"
    );
}

async fn an_unknown_element_type_gives_a_missing_reference(h: &dyn Harness) {
    let built = machine(130, "M130");
    insert(h, &built).await;
    let mut tx = begin(h).await;

    let error = tx
        .types()
        .ensure_derived(array_record(&built, type_id(27), type_id(9999), &[(1, 3)]))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == TYPE_ELEMENT_FKEY),
        "{error:?}"
    );
}

async fn a_dropped_transaction_keeps_no_derived_row(h: &dyn Harness) {
    let built = machine(131, "M131");
    insert(h, &built).await;
    let mut tx = begin(h).await;
    tx.types()
        .ensure_derived(string_record(&built, type_id(28), StringKind::String, 20))
        .await
        .unwrap();

    drop(tx);

    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    assert!(rows.iter().all(|row| row.machine().is_none()));
}

contract_case!(
    ensure_derived_inserts_a_string_and_gives_its_image,
    ensure_derived_inserts_a_string_and_gives_its_image_memory,
    ensure_derived_inserts_a_string_and_gives_its_image_postgres
);
contract_case!(
    ensure_derived_inserts_a_wstring,
    ensure_derived_inserts_a_wstring_memory,
    ensure_derived_inserts_a_wstring_postgres
);
contract_case!(
    a_second_string_of_one_length_gives_the_first_id_and_no_image,
    a_second_string_of_one_length_gives_the_first_id_and_no_image_memory,
    a_second_string_of_one_length_gives_the_first_id_and_no_image_postgres
);
contract_case!(
    the_same_string_on_another_machine_is_a_new_row,
    the_same_string_on_another_machine_is_a_new_row_memory,
    the_same_string_on_another_machine_is_a_new_row_postgres
);
contract_case!(
    an_array_of_an_array_creates_one_row_for_each_level,
    an_array_of_an_array_creates_one_row_for_each_level_memory,
    an_array_of_an_array_creates_one_row_for_each_level_postgres
);
contract_case!(
    the_same_array_gives_the_first_id_and_no_image,
    the_same_array_gives_the_first_id_and_no_image_memory,
    the_same_array_gives_the_first_id_and_no_image_postgres
);
contract_case!(
    a_struct_record_gives_the_database_rule_error,
    a_struct_record_gives_the_database_rule_error_memory,
    a_struct_record_gives_the_database_rule_error_postgres
);
contract_case!(
    a_global_primitive_record_gives_the_database_rule_error,
    a_global_primitive_record_gives_the_database_rule_error_memory,
    a_global_primitive_record_gives_the_database_rule_error_postgres
);
contract_case!(
    an_unknown_machine_gives_a_missing_reference,
    an_unknown_machine_gives_a_missing_reference_memory,
    an_unknown_machine_gives_a_missing_reference_postgres
);
contract_case!(
    an_unknown_element_type_gives_a_missing_reference,
    an_unknown_element_type_gives_a_missing_reference_memory,
    an_unknown_element_type_gives_a_missing_reference_postgres
);
contract_case!(
    a_dropped_transaction_keeps_no_derived_row,
    a_dropped_transaction_keeps_no_derived_row_memory,
    a_dropped_transaction_keeps_no_derived_row_postgres
);
