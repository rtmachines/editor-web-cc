//! Type trees that the tests of the value module share.

use crate::plant::fixtures::{
    array_row, id, primitive_row, string_row, struct_row, types, wstring_row,
};
use crate::plant::{PlcType, Primitive, TypeTree};

pub(crate) const INT: u128 = 1;
pub(crate) const REAL: u128 = 2;
pub(crate) const BOOL: u128 = 3;
pub(crate) const USINT: u128 = 4;
pub(crate) const CHAR: u128 = 5;
pub(crate) const WCHAR: u128 = 6;
pub(crate) const TIME: u128 = 7;
pub(crate) const DATE: u128 = 8;
pub(crate) const LTIME: u128 = 9;
pub(crate) const STRING_16: u128 = 10;
pub(crate) const RAMPS: u128 = 11;
pub(crate) const ST_ZONE: u128 = 12;
pub(crate) const LREAL: u128 = 13;
/// The root id of the single-row tree of [`time_tree_on`].
const TIME_ROOT: u128 = 20;

/// Resolve the tree of `root` over the shared primitives and the given extra rows.
pub(crate) fn tree(extra: Vec<PlcType>, root: u128) -> TypeTree {
    let mut rows = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
        primitive_row(USINT, Primitive::Usint),
        primitive_row(CHAR, Primitive::Char),
        primitive_row(WCHAR, Primitive::Wchar),
        primitive_row(TIME, Primitive::Time),
        primitive_row(DATE, Primitive::Date),
        primitive_row(LTIME, Primitive::Ltime),
        primitive_row(LREAL, Primitive::Lreal),
    ];
    rows.extend(extra);
    TypeTree::resolve(id(root), &types(rows)).unwrap()
}

/// The tree of a primitive parameter.
pub(crate) fn primitive_tree(root: u128) -> TypeTree {
    tree(Vec::new(), root)
}

/// The tree of a `STRING(length)` parameter.
pub(crate) fn string_tree(length: u16) -> TypeTree {
    tree(vec![string_row(50, length)], 50)
}

/// The tree of a `WSTRING(length)` parameter.
pub(crate) fn wstring_tree(length: u16) -> TypeTree {
    tree(vec![wstring_row(50, length)], 50)
}

/// The tree of an array parameter of `INT` with the given bounds.
pub(crate) fn int_array_tree(bounds: &[(i32, i32)]) -> TypeTree {
    tree(vec![array_row(50, INT, bounds)], 50)
}

/// The tree of the UDT `ST_Zone` of `data-model.md` section 9.1.
pub(crate) fn zone_tree() -> TypeTree {
    tree(
        vec![
            string_row(STRING_16, 16),
            array_row(RAMPS, INT, &[(1, 3)]),
            struct_row(
                ST_ZONE,
                "ST_Zone",
                &[
                    ("temp_sp", REAL),
                    ("active", BOOL),
                    ("ramps", RAMPS),
                    ("label", STRING_16),
                ],
            ),
        ],
        ST_ZONE,
    )
}

/// The value of `ST_Zone` in `data-model.md` section 9.1.
pub(crate) const ZONE_JSON: &str =
    r#"{"temp_sp": 85.5, "active": true, "ramps": [10, 20, 30], "label": "Zone A"}"#;

/// The tree of a primitive parameter on `platform`.
pub(crate) fn primitive_tree_on(platform: crate::plant::PlcPlatform, root: u128) -> TypeTree {
    let rows = vec![primitive_row(root, Primitive::Int)];
    TypeTree::resolve(
        id(root),
        &crate::plant::fixtures::types_for(crate::plant::fixtures::machine(), platform, rows),
    )
    .unwrap()
}

/// The tree of a time or instant parameter of type `primitive` on `platform`.
pub(crate) fn time_tree_on(platform: crate::plant::PlcPlatform, primitive: Primitive) -> TypeTree {
    let rows = vec![primitive_row(TIME_ROOT, primitive)];
    TypeTree::resolve(
        id(TIME_ROOT),
        &crate::plant::fixtures::types_for(crate::plant::fixtures::machine(), platform, rows),
    )
    .unwrap()
}

/// The root id of the single-row tree in [`integer_tree`].
const BOUNDARY_ROOT: u128 = 60;

/// The tree of one parameter of the integer type `primitive`.
pub(crate) fn integer_tree(primitive: Primitive) -> TypeTree {
    tree(vec![primitive_row(BOUNDARY_ROOT, primitive)], BOUNDARY_ROOT)
}

/// One row of the boundary table: a type with its lowest and highest value as decimal text.
pub(crate) struct Boundary {
    pub(crate) primitive: Primitive,
    pub(crate) low: &'static str,
    pub(crate) high: &'static str,
    pub(crate) below_low: &'static str,
    pub(crate) above_high: &'static str,
}

/// The 12 integer types with min-1, min, max, and max+1 (finding #14).
pub(crate) const BOUNDARIES: [Boundary; 12] = [
    Boundary {
        primitive: Primitive::Sint,
        low: "-128",
        high: "127",
        below_low: "-129",
        above_high: "128",
    },
    Boundary {
        primitive: Primitive::Usint,
        low: "0",
        high: "255",
        below_low: "-1",
        above_high: "256",
    },
    Boundary {
        primitive: Primitive::Byte,
        low: "0",
        high: "255",
        below_low: "-1",
        above_high: "256",
    },
    Boundary {
        primitive: Primitive::Int,
        low: "-32768",
        high: "32767",
        below_low: "-32769",
        above_high: "32768",
    },
    Boundary {
        primitive: Primitive::Uint,
        low: "0",
        high: "65535",
        below_low: "-1",
        above_high: "65536",
    },
    Boundary {
        primitive: Primitive::Word,
        low: "0",
        high: "65535",
        below_low: "-1",
        above_high: "65536",
    },
    Boundary {
        primitive: Primitive::Dint,
        low: "-2147483648",
        high: "2147483647",
        below_low: "-2147483649",
        above_high: "2147483648",
    },
    Boundary {
        primitive: Primitive::Udint,
        low: "0",
        high: "4294967295",
        below_low: "-1",
        above_high: "4294967296",
    },
    Boundary {
        primitive: Primitive::Dword,
        low: "0",
        high: "4294967295",
        below_low: "-1",
        above_high: "4294967296",
    },
    Boundary {
        primitive: Primitive::Lint,
        low: "-9223372036854775808",
        high: "9223372036854775807",
        below_low: "-9223372036854775809",
        above_high: "9223372036854775808",
    },
    Boundary {
        primitive: Primitive::Ulint,
        low: "0",
        high: "18446744073709551615",
        below_low: "-1",
        above_high: "18446744073709551616",
    },
    Boundary {
        primitive: Primitive::Lword,
        low: "0",
        high: "18446744073709551615",
        below_low: "-1",
        above_high: "18446744073709551616",
    },
];
