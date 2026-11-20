//! Tests of the decoder of stored `plc_type` rows. They need no database.
//!
//! The decoder reads a stored type text under `RuleSet::Earlier`. ADR-0206 (f) keeps the catalog
//! name in storage, so a stored vendor alias stays a corrupt row (`E-DB-008`).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcPlatform, Primitive, RecordShape};
use uuid::Uuid;

use super::{TypeRecord, build_type, shape_of};

/// A stored row of kind `primitive` with the given type text, as the `TIME_OF_DAY` row of the seed.
fn primitive_row(name: &str) -> TypeRecord {
    TypeRecord {
        id: Uuid::from_u128(1),
        kind: "primitive".to_owned(),
        name: Some(name.to_owned()),
        platforms: vec![PlcPlatform::S7_1500.as_str().to_owned()],
        size_bytes: Some(4),
        string_length: None,
        leaf_class: Some(Primitive::TimeOfDay.leaf_class().as_sql().to_owned()),
        element_type_id: None,
        dim_lower: None,
        dim_upper: None,
        machine_id: None,
        revision: 1,
        supersedes_type_id: None,
    }
}

/// A stored row of kind `struct` with the given name and no member rows.
fn struct_row(name: &str) -> TypeRecord {
    TypeRecord {
        id: Uuid::from_u128(2),
        kind: "struct".to_owned(),
        name: Some(name.to_owned()),
        platforms: vec![PlcPlatform::S7_1500.as_str().to_owned()],
        size_bytes: None,
        string_length: None,
        leaf_class: None,
        element_type_id: None,
        dim_lower: None,
        dim_upper: None,
        machine_id: Some(Uuid::from_u128(3)),
        revision: 1,
        supersedes_type_id: None,
    }
}

#[test]
fn adr_0206_a_stored_tod_row_stays_a_corrupt_row() {
    let error = build_type(primitive_row("TOD"), Vec::new()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
}

#[test]
fn adr_0206_a_stored_tod_row_shape_is_refused_before_the_domain_check() {
    let error = shape_of(&primitive_row("TOD"), Vec::new()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
}

#[test]
fn adr_0206_a_stored_time_of_day_row_decodes() {
    let decoded = build_type(primitive_row("TIME_OF_DAY"), Vec::new()).unwrap();

    assert!(matches!(
        decoded.shape(),
        RecordShape::Primitive {
            primitive: Primitive::TimeOfDay,
            ..
        }
    ));
}

#[test]
fn adr_0206_a_stored_struct_name_reads_the_catalog_name_as_reserved() {
    let error = shape_of(&struct_row("TIME_OF_DAY"), Vec::new()).unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(shape_of(&struct_row("Cell"), Vec::new()).is_ok());
}
