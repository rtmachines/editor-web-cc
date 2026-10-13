//! Class-2 advisory lock keys of derived types (`data-model.md` section 1.4, design D11).
//!
//! A command that can insert a type row collects its keys before its first insert.
//! The keys come in the order of `DerivedTypeLock`. Two commands that take two keys
//! in opposite orders could deadlock, so every command uses this one collection.

use recipe_domain::plant::{BaseExpr, StringKind, TypeExpr, TypeName};
use recipe_domain::recipe::ArrayShape;

use crate::ports::DerivedTypeLock;

/// Collect the class-2 lock keys that a command needs before its first type insert.
///
/// The function solves one problem: two commands must take the same keys in one order,
/// or they can deadlock. It adds the struct name when `DefineUdt` defines a struct.
/// A `STRING` or `WSTRING` without a length and a primitive need no key, because
/// `EnsureDerivedType` never inserts them. Any array level needs the `ARRAY` key.
/// The result is sorted and has no duplicates.
#[must_use]
pub(crate) fn derived_type_locks<'a>(
    struct_name: Option<&TypeName>,
    exprs: impl IntoIterator<Item = &'a TypeExpr>,
) -> Vec<DerivedTypeLock> {
    let mut locks: Vec<DerivedTypeLock> = struct_name
        .map(|name| DerivedTypeLock::Struct(name.clone()))
        .into_iter()
        .collect();
    for expr in exprs {
        push_expr_locks(expr, &mut locks);
    }
    locks.sort();
    locks.dedup();
    locks
}

/// Collect the `ARRAY` key of a rebind, because `RebindTypeRevision` inserts array rows.
///
/// The function solves one problem: a rebind that creates array rows must take the
/// `ARRAY` key in the same order as every other command (`data-model.md` section 1.4).
/// It returns no key when the plan has no array shape.
#[must_use]
pub(crate) fn rebind_locks(shapes: &[ArrayShape]) -> Vec<DerivedTypeLock> {
    if shapes.is_empty() {
        Vec::new()
    } else {
        vec![DerivedTypeLock::Array]
    }
}

/// Push the keys of one type expression: `ARRAY` for an array, and the string key for a sized string.
fn push_expr_locks(expr: &TypeExpr, locks: &mut Vec<DerivedTypeLock>) {
    if !expr.arrays().is_empty() {
        locks.push(DerivedTypeLock::Array);
    }
    if let BaseExpr::String {
        kind,
        length: Some(_),
    } = expr.base()
    {
        locks.push(match kind {
            StringKind::String => DerivedTypeLock::String,
            StringKind::Wstring => DerivedTypeLock::Wstring,
        });
    }
}

#[cfg(test)]
mod tests {
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
}
