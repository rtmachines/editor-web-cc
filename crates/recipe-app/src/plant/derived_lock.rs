//! Class-2 advisory lock keys of derived types (`data-model.md` section 1.4, design D11).
//!
//! A command that can insert a type row collects its keys before its first insert.
//! The keys come in the order of their texts, without duplicate texts (ADR-0144 (d)).
//! The order of the texts is not the order of the locks. The adapter of `lock_derived_types`
//! takes the locks in the order of the key values. Two commands that collect the same
//! texts in one order lock the same set, so every command uses this one collection.

use recipe_domain::plant::{BaseExpr, StringKind, TypeExpr, TypeName};
use recipe_domain::recipe::ArrayShape;

use crate::ports::DerivedTypeLock;

/// Collect the class-2 lock keys that a command needs before its first type insert.
///
/// The function solves one problem: two commands must collect the same key texts in one
/// order, whatever the kind of the key. A stored struct named `ARRAY` and the key `ARRAY`
/// have one text, so they give one key. It adds the struct name when `DefineUdt` defines a
/// struct. A `STRING` or `WSTRING` without a length and a primitive need no key, because
/// `EnsureDerivedType` never inserts them. Any array level needs the `ARRAY` key.
/// The result is sorted by key text and has no duplicate text.
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
    sort_by_text(&mut locks);
    locks
}

/// Collect the `ARRAY` key of a rebind, because `RebindTypeRevision` inserts array rows.
///
/// The function solves one problem: a rebind that creates array rows must take the
/// `ARRAY` key like every other command (`data-model.md` section 1.4, ADR-0144 (d)).
/// It returns no key when the plan has no array shape.
#[must_use]
pub(crate) fn rebind_locks(shapes: &[ArrayShape]) -> Vec<DerivedTypeLock> {
    if shapes.is_empty() {
        Vec::new()
    } else {
        vec![DerivedTypeLock::Array]
    }
}

/// Sort the keys by their text and remove the keys with a duplicate text (ADR-0144 (d)).
///
/// A struct key sorts before a keyword key of the same text, so the result does not depend
/// on the input order.
fn sort_by_text(locks: &mut Vec<DerivedTypeLock>) {
    locks.sort_by(|left, right| {
        left.key_text()
            .cmp(right.key_text())
            .then_with(|| left.cmp(right))
    });
    locks.dedup_by(|later, earlier| later.key_text() == earlier.key_text());
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
mod tests;
