use recipe_domain::plant::RuleSet;

use super::*;

fn expr(text: &str) -> TypeExpr {
    TypeExpr::parse(text).unwrap()
}

fn name(text: &str) -> TypeName {
    TypeName::parse(text).unwrap()
}

/// A stored struct can carry a reserved name such as `ARRAY` (ADR-0143 (i2)).
fn stored_name(text: &str) -> TypeName {
    TypeName::from_stored(text, RuleSet::Earlier).unwrap()
}

fn texts(locks: &[DerivedTypeLock]) -> Vec<&str> {
    locks.iter().map(DerivedTypeLock::key_text).collect()
}

#[test]
fn array_of_sized_strings_needs_string_and_array_keys() {
    let exprs = [expr("ARRAY[1..4] OF STRING(16)")];
    assert_eq!(
        derived_type_locks(None, &exprs),
        vec![DerivedTypeLock::Array, DerivedTypeLock::String]
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
fn struct_definition_orders_its_keys_by_text() {
    let members = [expr("STRING(8)"), expr("ARRAY[1..2] OF REAL")];
    let locks = derived_type_locks(Some(&name("ST_Zone")), &members);
    assert_eq!(texts(&locks), ["ARRAY", "STRING", "ST_Zone"]);
}

#[test]
fn struct_with_an_array_member_has_the_keys_array_and_the_struct_name() {
    let members = [expr("ARRAY[1..2] OF REAL")];
    let locks = derived_type_locks(Some(&name("ST_Zone")), &members);
    assert_eq!(
        locks,
        vec![
            DerivedTypeLock::Array,
            DerivedTypeLock::Struct(name("ST_Zone"))
        ]
    );
}

#[test]
fn keys_are_ordered_by_text_not_by_kind() {
    let exprs = [expr("WSTRING(5)"), expr("STRING(5)")];
    assert_eq!(
        derived_type_locks(None, &exprs),
        vec![DerivedTypeLock::String, DerivedTypeLock::Wstring]
    );
    let members = [expr("WSTRING(5)")];
    let locks = derived_type_locks(Some(&name("ST_Zone")), &members);
    assert_eq!(texts(&locks), ["ST_Zone", "WSTRING"]);
}

#[test]
fn define_udt_of_a_stored_struct_array_and_add_parameter_take_the_same_texts() {
    let define_members = [expr("STRING(5)")];
    let define = derived_type_locks(Some(&stored_name("ARRAY")), &define_members);
    let add_parameter = derived_type_locks(None, &[expr("ARRAY[1..2] OF STRING(5)")]);
    assert_eq!(texts(&define), ["ARRAY", "STRING"]);
    assert_eq!(texts(&add_parameter), ["ARRAY", "STRING"]);
}

#[test]
fn a_struct_named_array_and_the_array_key_give_one_text() {
    let members = [expr("ARRAY[1..2] OF INT")];
    let locks = derived_type_locks(Some(&stored_name("ARRAY")), &members);
    assert_eq!(texts(&locks), ["ARRAY"]);
}

#[test]
fn rebind_without_shapes_needs_no_key() {
    assert!(rebind_locks(&[]).is_empty());
}
