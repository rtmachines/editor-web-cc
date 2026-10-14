//! Shapes of `plc_type` images (design D5 and D13, `data-model.md` section 3.3).

use super::images::Kind;

/// The kind of a `plc_type` row for `type_shape`.
#[derive(Clone, Copy, Debug)]
pub enum TypeKind {
    /// A `STRING(n)` or `WSTRING(n)` row of a machine.
    String,
    /// An array row.
    Array,
    /// A struct row. The flag tells whether the row supersedes an older revision.
    Struct { supersedes: bool },
}

/// The keys that only some kinds of row fill.
const OPTIONAL_KEYS: [&str; 8] = [
    "name",
    "size_bytes",
    "string_length",
    "leaf_class",
    "element_type_id",
    "dim_lower",
    "dim_upper",
    "supersedes_type_id",
];

/// The optional keys that a row of `kind` fills, with the JSON type of each.
fn filled(kind: TypeKind) -> Vec<(&'static str, Kind)> {
    match kind {
        TypeKind::String => vec![
            ("name", Kind::String),
            ("size_bytes", Kind::Number),
            ("string_length", Kind::Number),
            ("leaf_class", Kind::String),
        ],
        TypeKind::Array => vec![
            ("element_type_id", Kind::String),
            ("dim_lower", Kind::Array),
            ("dim_upper", Kind::Array),
        ],
        TypeKind::Struct { supersedes } => {
            let mut keys = vec![("name", Kind::String)];
            if supersedes {
                keys.push(("supersedes_type_id", Kind::String));
            }
            keys
        }
    }
}

/// The shape of a `plc_type` image. A struct image adds the key `members`.
pub fn type_shape(kind: TypeKind) -> Vec<(&'static str, Kind)> {
    let filled = filled(kind);
    let mut shape = vec![
        ("id", Kind::String),
        ("kind", Kind::String),
        ("platforms", Kind::Array),
        ("machine_id", Kind::String),
        ("revision", Kind::Number),
        ("created_at", Kind::String),
    ];
    for key in OPTIONAL_KEYS {
        let kind = filled
            .iter()
            .find(|(name, _)| *name == key)
            .map_or(Kind::Null, |(_, kind)| *kind);
        shape.push((key, kind));
    }
    if matches!(kind, TypeKind::Struct { .. }) {
        shape.push(("members", Kind::Array));
    }
    shape
}

/// The keys of one member row of a struct image.
pub const MEMBER_KEYS: [&str; 5] = [
    "created_at",
    "member_name",
    "member_type_id",
    "ordinal",
    "struct_type_id",
];
