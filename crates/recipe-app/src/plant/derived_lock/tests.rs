use super::*;

fn expr(text: &str) -> TypeExpr {
    TypeExpr::parse(text).unwrap()
}

fn name(text: &str) -> TypeName {
    TypeName::parse(text).unwrap()
}

#[test]
fn array_of_sized_strings_needs_string_and_array_keys() {
    let exprs = [expr("ARRAY[1..4] OF STRING(16)")];
    assert_eq!(
        derived_type_locks(None, &exprs),
        vec![DerivedTypeLock::String, DerivedTypeLock::Array]
    );
}

#[test]
fn sized_wstring_needs_wstring_key() {
    let exprs = [expr("WSTRING(10)")];
    assert_eq!(
        derived_type_locks(None, &exprs),
        vec![DerivedTypeLock::Wstring]
    );
}

#[test]
fn seed_string_primitive_and_struct_reference_need_no_key() {
    let exprs = [expr("STRING"), expr("INT"), expr("ST_Zone@3")];
    assert!(derived_type_locks(None, &exprs).is_empty());
}

#[test]
fn nested_array_needs_array_key_once() {
    let exprs = [expr("ARRAY[1..2] OF ARRAY[1..3] OF INT")];
    assert_eq!(
        derived_type_locks(None, &exprs),
        vec![DerivedTypeLock::Array]
    );
}

#[test]
fn struct_definition_adds_struct_key_before_string_and_array() {
    let members = [expr("STRING(8)"), expr("ARRAY[1..2] OF REAL")];
    let locks = derived_type_locks(Some(&name("ST_Zone")), &members);
    assert_eq!(
        locks,
        vec![
            DerivedTypeLock::Struct(name("ST_Zone")),
            DerivedTypeLock::String,
            DerivedTypeLock::Array,
        ]
    );
}

#[test]
fn keys_are_sorted_not_in_input_order() {
    let exprs = [expr("WSTRING(5)"), expr("STRING(5)")];
    assert_eq!(
        derived_type_locks(None, &exprs),
        vec![DerivedTypeLock::String, DerivedTypeLock::Wstring]
    );
}

#[test]
fn rebind_without_shapes_needs_no_key() {
    assert!(rebind_locks(&[]).is_empty());
}
