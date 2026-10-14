//! Contract cases of `TypeCatalog::insert_struct`.

use std::collections::BTreeSet;

use recipe_app::error::{PLC_TYPE_NAMED_UNIQUE, StorageError};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcType, Primitive, StringKind};
use serde_json::json;

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::images::{assert_shape, field, text};
use crate::contract::seed::{
    array_record, insert, machine, seed_type, string_record, struct_record, type_id,
};
use crate::contract::type_images::{MEMBER_KEYS, TypeKind, type_shape};

/// The name of the foreign key from a member row to its type row.
const MEMBER_TYPE_FKEY: &str = "plc_type_member_member_type_id_fkey";

async fn insert_struct_gives_an_image_with_the_content_of_its_members(h: &dyn Harness) {
    let built = machine(140, "M140");
    insert(h, &built).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let boolean = seed_type(h, built.id(), Primitive::Bool).await;
    let record = struct_record(
        &built,
        type_id(40),
        "Pump",
        1,
        None,
        &[("speed", dint), ("running", boolean)],
    );
    let row = PlcType::try_from(record).unwrap();
    let mut tx = begin(h).await;

    let image = tx.types().insert_struct(&row).await.unwrap();
    commit(h, tx, built.id()).await;

    assert_shape(&image, &type_shape(TypeKind::Struct { supersedes: false }));
    assert_eq!(text(&image, "id"), type_id(40).to_string());
    assert_eq!(text(&image, "kind"), "struct");
    assert_eq!(text(&image, "name"), "Pump");
    assert_eq!(text(&image, "machine_id"), built.id().to_string());
    assert_eq!(field(&image, "revision"), &json!(1));
    assert_eq!(field(&image, "platforms"), &json!(["TwinCat3"]));
    let members = field(&image, "members").as_array().unwrap();
    assert_eq!(members.len(), 2);
    for member in members {
        let keys: BTreeSet<&str> = member
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, BTreeSet::from(MEMBER_KEYS));
        assert_eq!(member["struct_type_id"], json!(type_id(40).to_string()));
    }
    assert_eq!(members[0]["ordinal"], json!(1));
    assert_eq!(members[0]["member_name"], json!("speed"));
    assert_eq!(members[0]["member_type_id"], json!(dint.to_string()));
    assert_eq!(members[1]["ordinal"], json!(2));
    assert_eq!(members[1]["member_name"], json!("running"));
    assert_eq!(members[1]["member_type_id"], json!(boolean.to_string()));
    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    assert!(rows.contains(&row), "the struct and its members are stored");
}

async fn a_new_revision_keeps_the_link_to_the_old_one(h: &dyn Harness) {
    let built = machine(141, "M141");
    insert(h, &built).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let first = PlcType::try_from(struct_record(
        &built,
        type_id(41),
        "Pump",
        1,
        None,
        &[("speed", dint)],
    ))
    .unwrap();
    let second = PlcType::try_from(struct_record(
        &built,
        type_id(42),
        "Pump",
        2,
        Some(type_id(41)),
        &[("speed", dint)],
    ))
    .unwrap();
    let mut tx = begin(h).await;
    tx.types().insert_struct(&first).await.unwrap();

    let image = tx.types().insert_struct(&second).await.unwrap();
    commit(h, tx, built.id()).await;

    assert_shape(&image, &type_shape(TypeKind::Struct { supersedes: true }));
    assert_eq!(field(&image, "revision"), &json!(2));
    assert_eq!(text(&image, "supersedes_type_id"), type_id(41).to_string());
}

async fn a_duplicate_revision_of_one_name_gives_duplicate(h: &dyn Harness) {
    let built = machine(142, "M142");
    insert(h, &built).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let first = PlcType::try_from(struct_record(
        &built,
        type_id(43),
        "Pump",
        1,
        None,
        &[("speed", dint)],
    ))
    .unwrap();
    let same_revision = PlcType::try_from(struct_record(
        &built,
        type_id(44),
        "Pump",
        1,
        None,
        &[("speed", dint)],
    ))
    .unwrap();
    let mut tx = begin(h).await;
    tx.types().insert_struct(&first).await.unwrap();

    let error = tx.types().insert_struct(&same_revision).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == PLC_TYPE_NAMED_UNIQUE),
        "{error:?}"
    );
}

async fn the_same_struct_name_on_another_machine_is_allowed(h: &dyn Harness) {
    let first = machine(143, "M143");
    let second = machine(144, "M144");
    insert(h, &first).await;
    insert(h, &second).await;
    let dint = seed_type(h, first.id(), Primitive::Dint).await;
    for (owner, id) in [(&first, 45), (&second, 46)] {
        let row = PlcType::try_from(struct_record(
            owner,
            type_id(id),
            "Pump",
            1,
            None,
            &[("speed", dint)],
        ))
        .unwrap();
        let mut tx = begin(h).await;
        tx.types().insert_struct(&row).await.unwrap();
        commit(h, tx, owner.id()).await;
    }
}

async fn a_member_of_a_derived_type_keeps_its_row(h: &dyn Harness) {
    let built = machine(145, "M145");
    insert(h, &built).await;
    let text_type = crate::contract::seed::ensure(
        h,
        built.id(),
        string_record(&built, type_id(47), StringKind::String, 20),
    )
    .await;
    let int = seed_type(h, built.id(), Primitive::Int).await;
    let array = crate::contract::seed::ensure(
        h,
        built.id(),
        array_record(&built, type_id(48), int, &[(1, 3)]),
    )
    .await;
    let row = PlcType::try_from(struct_record(
        &built,
        type_id(49),
        "Mixed",
        1,
        None,
        &[("label", text_type.id), ("samples", array.id)],
    ))
    .unwrap();
    let mut tx = begin(h).await;

    tx.types().insert_struct(&row).await.unwrap();
    commit(h, tx, built.id()).await;

    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    assert!(rows.contains(&row));
}

async fn an_unknown_member_type_gives_a_missing_reference(h: &dyn Harness) {
    let built = machine(146, "M146");
    insert(h, &built).await;
    let row = PlcType::try_from(struct_record(
        &built,
        type_id(50),
        "Pump",
        1,
        None,
        &[("speed", type_id(9999))],
    ))
    .unwrap();
    let mut tx = begin(h).await;

    let error = tx.types().insert_struct(&row).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == MEMBER_TYPE_FKEY),
        "{error:?}"
    );
}

async fn a_row_that_is_not_a_struct_gives_the_database_rule_error(h: &dyn Harness) {
    let built = machine(147, "M147");
    insert(h, &built).await;
    let row =
        PlcType::try_from(string_record(&built, type_id(51), StringKind::String, 20)).unwrap();
    let mut tx = begin(h).await;

    let error = tx.types().insert_struct(&row).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Constraint { code, .. } if *code == ErrorCode::DB_003),
        "{error:?}"
    );
}

/// The largest number of members of one struct (`type.max_members`).
const MAX_MEMBERS: usize = 1024;

async fn a_struct_with_the_largest_number_of_members_keeps_every_member_in_order(h: &dyn Harness) {
    let built = machine(148, "M148");
    insert(h, &built).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let names: Vec<String> = (0..MAX_MEMBERS).map(|index| format!("m{index}")).collect();
    let members: Vec<(&str, _)> = names.iter().map(|name| (name.as_str(), dint)).collect();
    let row = PlcType::try_from(struct_record(
        &built,
        type_id(52),
        "Wide",
        1,
        None,
        &members,
    ))
    .unwrap();
    let mut tx = begin(h).await;

    let image = tx.types().insert_struct(&row).await.unwrap();
    commit(h, tx, built.id()).await;

    let stored = field(&image, "members").as_array().unwrap();
    assert_eq!(stored.len(), MAX_MEMBERS);
    for (index, member) in stored.iter().enumerate() {
        assert_eq!(member["ordinal"], json!(index.checked_add(1).unwrap()));
        assert_eq!(member["member_name"], json!(format!("m{index}")));
    }
    let mut tx = begin(h).await;
    let rows = tx.types().type_rows(built.id()).await.unwrap();
    assert!(rows.contains(&row), "the struct and its members are stored");
}

contract_case!(
    a_struct_with_the_largest_number_of_members_keeps_every_member_in_order,
    a_struct_with_the_largest_number_of_members_keeps_every_member_in_order_memory,
    a_struct_with_the_largest_number_of_members_keeps_every_member_in_order_postgres
);
contract_case!(
    insert_struct_gives_an_image_with_the_content_of_its_members,
    insert_struct_gives_an_image_with_the_content_of_its_members_memory,
    insert_struct_gives_an_image_with_the_content_of_its_members_postgres
);
contract_case!(
    a_new_revision_keeps_the_link_to_the_old_one,
    a_new_revision_keeps_the_link_to_the_old_one_memory,
    a_new_revision_keeps_the_link_to_the_old_one_postgres
);
contract_case!(
    a_duplicate_revision_of_one_name_gives_duplicate,
    a_duplicate_revision_of_one_name_gives_duplicate_memory,
    a_duplicate_revision_of_one_name_gives_duplicate_postgres
);
contract_case!(
    the_same_struct_name_on_another_machine_is_allowed,
    the_same_struct_name_on_another_machine_is_allowed_memory,
    the_same_struct_name_on_another_machine_is_allowed_postgres
);
contract_case!(
    a_member_of_a_derived_type_keeps_its_row,
    a_member_of_a_derived_type_keeps_its_row_memory,
    a_member_of_a_derived_type_keeps_its_row_postgres
);
contract_case!(
    an_unknown_member_type_gives_a_missing_reference,
    an_unknown_member_type_gives_a_missing_reference_memory,
    an_unknown_member_type_gives_a_missing_reference_postgres
);
contract_case!(
    a_row_that_is_not_a_struct_gives_the_database_rule_error,
    a_row_that_is_not_a_struct_gives_the_database_rule_error_memory,
    a_row_that_is_not_a_struct_gives_the_database_rule_error_postgres
);
