//! The type expression of a type row (design D3).

use std::collections::BTreeSet;

use recipe_domain::plant::{PlatformTypes, PlcTypeId, RecordShape, TypeExpr};

/// Write the type expression of the row `id`, for example `ARRAY[1..4] OF STRING(16)` or `ST_Zone@2`.
///
/// The function solves one problem: a page shows a type as text, and a form sends that text back.
/// `TypeExpr::parse` reads the text, and the resolution of `EnsureDerivedType` finds the same row
/// again. An array row writes one `ARRAY[...] OF ` prefix
/// for each level, the outermost first. A struct row writes `Name@revision`.
/// A global `STRING` or `WSTRING` row has no length in its text, because a text without a length
/// names the seed row of the platform. A `STRING(n)` row of the machine keeps its length.
/// The result is `None` when `types` lacks the row or an element row, or when array rows form a cycle.
pub(crate) fn type_text(types: &PlatformTypes, id: PlcTypeId) -> Option<String> {
    let mut prefix = String::new();
    let mut visited = BTreeSet::new();
    let mut current = id;
    loop {
        if !visited.insert(current) {
            return None;
        }
        let row = types.get(current)?;
        match row.shape() {
            RecordShape::Array {
                element,
                dimensions,
            } => {
                prefix.push_str(&format!("ARRAY[{dimensions}] OF "));
                current = *element;
            }
            RecordShape::Struct { name, revision, .. } => {
                return Some(format!("{prefix}{name}@{revision}"));
            }
            RecordShape::Primitive {
                primitive,
                string_length,
                ..
            } => {
                let base = match (string_length, row.machine()) {
                    (Some(length), Some(_)) => format!("{}({length})", primitive.name()),
                    _ => primitive.name().to_owned(),
                };
                return Some(format!("{prefix}{base}"));
            }
        }
    }
}

/// Tell whether the type expression `expr` names the type row `id`, without a lookup or an insert.
///
/// The function solves one problem: `UpdateParameterDefinition` must know before the unit lock
/// whether a request changes the stored type of a parameter. `DerivedTypes::ensure_expr` answers
/// that question, but it can lock and insert a row. The function compares the text of the row with
/// the canonical text of `expr` instead. The result is `true` exactly when `ensure_expr` resolves
/// `expr` to `id`. Each row has one identity, and `type_text` writes the text that `TypeExpr`
/// writes (design D2 of the change `type-check-before-insert`). The result is `false`
/// when `types` lacks the row or an element row. The caller then treats the type as changed.
pub(crate) fn expr_names_row(types: &PlatformTypes, id: PlcTypeId, expr: &TypeExpr) -> bool {
    type_text(types, id).is_some_and(|text| text == expr.to_string())
}
