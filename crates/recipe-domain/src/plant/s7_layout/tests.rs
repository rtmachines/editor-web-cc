use super::super::type_tree::fixtures::{
    array_row, id, primitive_row, string_row, struct_row, types,
};
use super::super::{PlcType, Primitive, TypeTree};
use super::*;

const INT: u128 = 1;
const REAL: u128 = 2;
const BOOL: u128 = 3;
const BYTE: u128 = 4;
const CHAR: u128 = 5;
const DTL: u128 = 6;
const LREAL: u128 = 7;
const STRING_3: u128 = 8;
const ST_FLAGS: u128 = 9;
const ROOT: u128 = 100;

struct Case {
    name: &'static str,
    rows: Vec<PlcType>,
    size_bytes: u32,
    start: S7Start,
}

fn case(name: &'static str, rows: Vec<PlcType>, size_bytes: u32, start: S7Start) -> Case {
    Case {
        name,
        rows,
        size_bytes,
        start,
    }
}

fn layout_of(rows: Vec<PlcType>) -> Result<S7Layout, PlantError> {
    let mut all = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
        primitive_row(BYTE, Primitive::Byte),
        primitive_row(CHAR, Primitive::Char),
        primitive_row(DTL, Primitive::Dtl),
        primitive_row(LREAL, Primitive::Lreal),
        string_row(STRING_3, 3),
        struct_row(
            ST_FLAGS,
            "ST_Flags",
            &[("a", BOOL), ("b", BOOL), ("c", BOOL)],
        ),
    ];
    all.extend(rows);
    let tree = TypeTree::resolve(id(ROOT), &types(all)).expect("fixture tree resolves");
    S7Layout::of(&tree)
}

/// A struct root with the given members, one row per distinct member.
fn struct_case(name: &'static str, members: &[(&str, u128)], size_bytes: u32) -> Case {
    case(
        name,
        vec![struct_row(ROOT, "ST_Case", members)],
        size_bytes,
        S7Start::Even,
    )
}

fn array_case(name: &'static str, bounds: &[(i32, i32)], element: u128, size: u32) -> Case {
    case(
        name,
        vec![array_row(ROOT, element, bounds)],
        size,
        S7Start::Even,
    )
}

fn struct_cases() -> Vec<Case> {
    let zone = vec![
        array_row(10, INT, &[(1, 3)]),
        string_row(11, 16),
        struct_row(
            ROOT,
            "ST_Zone",
            &[
                ("temp_sp", REAL),
                ("active", BOOL),
                ("ramps", 10),
                ("label", 11),
            ],
        ),
    ];
    vec![
        struct_case("ST_Flags", &[("a", BOOL), ("b", BOOL), ("c", BOOL)], 2),
        struct_case(
            "ST_Mix",
            &[("flag", BOOL), ("speed", INT), ("mode", BYTE)],
            6,
        ),
        case("ST_Zone", zone, 30, S7Start::Even),
        struct_case("three bytes", &[("a", BYTE), ("b", BYTE), ("c", BYTE)], 4),
        struct_case("byte then int", &[("a", BYTE), ("b", INT)], 4),
        struct_case(
            "nine bools",
            &[
                ("a", BOOL),
                ("b", BOOL),
                ("c", BOOL),
                ("d", BOOL),
                ("e", BOOL),
                ("f", BOOL),
                ("g", BOOL),
                ("h", BOOL),
                ("i", BOOL),
            ],
            2,
        ),
        struct_case(
            "int, two bools, byte",
            &[("a", INT), ("b", BOOL), ("c", BOOL), ("d", BYTE)],
            4,
        ),
        struct_case("bool then byte", &[("a", BOOL), ("b", BYTE)], 2),
        struct_case("byte then ST_Flags", &[("a", BYTE), ("b", ST_FLAGS)], 4),
        case(
            "bool then bool array",
            vec![
                array_row(10, BOOL, &[(1, 2)]),
                struct_row(ROOT, "ST_Case", &[("a", BOOL), ("b", 10)]),
            ],
            4,
            S7Start::Even,
        ),
    ]
}

fn array_cases() -> Vec<Case> {
    vec![
        array_case("4 strings of 3", &[(1, 4)], STRING_3, 24),
        array_case("3 bytes", &[(1, 3)], BYTE, 4),
        array_case("8 bools", &[(1, 8)], BOOL, 2),
        array_case("10 bools", &[(1, 10)], BOOL, 2),
        array_case("17 bools", &[(1, 17)], BOOL, 4),
        array_case("2 by 2 ints", &[(0, 1), (1, 2)], INT, 8),
    ]
}

fn leaf_cases() -> Vec<Case> {
    vec![
        case("INT root", Vec::new(), 2, S7Start::Even),
        case("BOOL root", Vec::new(), 1, S7Start::Bit),
        case("BYTE root", Vec::new(), 1, S7Start::Byte),
        case("CHAR root", Vec::new(), 1, S7Start::Byte),
        case("DTL root", Vec::new(), 12, S7Start::Even),
        case("LREAL root", Vec::new(), 8, S7Start::Even),
        case("STRING(3) root", Vec::new(), 5, S7Start::Even),
    ]
}

#[test]
fn s7_layout_golden_struct_and_array_sizes() {
    for case in struct_cases().into_iter().chain(array_cases()) {
        let layout = layout_of(case.rows).expect(case.name);
        assert_eq!(layout.size_bytes(), case.size_bytes, "{}", case.name);
        assert_eq!(layout.start(), case.start, "{}", case.name);
    }
}

#[test]
fn s7_layout_golden_leaf_roots_are_not_rounded() {
    let roots = [INT, BOOL, BYTE, CHAR, DTL, LREAL, STRING_3];
    for (case, root) in leaf_cases().into_iter().zip(roots) {
        let mut all = vec![
            primitive_row(INT, Primitive::Int),
            primitive_row(BOOL, Primitive::Bool),
            primitive_row(BYTE, Primitive::Byte),
            primitive_row(CHAR, Primitive::Char),
            primitive_row(DTL, Primitive::Dtl),
            primitive_row(LREAL, Primitive::Lreal),
            string_row(STRING_3, 3),
        ];
        all.extend(case.rows);
        let tree = TypeTree::resolve(id(root), &types(all)).expect("leaf root resolves");
        let layout = S7Layout::of(&tree).expect(case.name);
        assert_eq!(layout.size_bytes(), case.size_bytes, "{}", case.name);
        assert_eq!(layout.start(), case.start, "{}", case.name);
    }
}

const ST_ZONE: u128 = 20;
const INT_RAMPS: u128 = 21;
const SIXTEEN_BOOLS: u128 = 22;

/// Resolve `root` over the primitives and the rows of the two spec structs.
fn members_of(root: u128, extra: Vec<PlcType>) -> Vec<S7Member> {
    let mut all = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(BOOL, Primitive::Bool),
        struct_row(
            ST_ZONE,
            "ST_Zone",
            &[
                ("active", BOOL),
                ("ready", BOOL),
                ("setpoint", REAL),
                ("count", INT),
            ],
        ),
        array_row(INT_RAMPS, INT, &[(1, 3)]),
    ];
    all.extend(extra);
    let tree = TypeTree::resolve(id(root), &types(all)).expect("fixture tree resolves");
    s7_members(&tree).expect("members of the fixture")
}

fn rows(members: &[S7Member]) -> Vec<(String, u32, u8)> {
    members
        .iter()
        .map(|member| (member.path_text(), member.byte(), member.bit()))
        .collect()
}

fn row(path: &str, byte: u32, bit: u8) -> (String, u32, u8) {
    (path.to_owned(), byte, bit)
}

#[test]
fn s7_members_golden_struct_with_two_bools() {
    let members = members_of(ST_ZONE, Vec::new());
    assert_eq!(
        rows(&members),
        vec![
            row(".active", 0, 0),
            row(".ready", 0, 1),
            row(".setpoint", 2, 0),
            row(".count", 6, 0),
        ]
    );
    let tree = TypeTree::resolve(
        id(ST_ZONE),
        &types(vec![
            primitive_row(INT, Primitive::Int),
            primitive_row(REAL, Primitive::Real),
            primitive_row(BOOL, Primitive::Bool),
            struct_row(
                ST_ZONE,
                "ST_Zone",
                &[
                    ("active", BOOL),
                    ("ready", BOOL),
                    ("setpoint", REAL),
                    ("count", INT),
                ],
            ),
        ]),
    )
    .unwrap();
    assert_eq!(S7Layout::of(&tree).unwrap().size_bytes(), 8);
}

#[test]
fn s7_members_golden_nested_struct_and_array() {
    let line = struct_row(ROOT, "ST_Line", &[("zone", ST_ZONE), ("ramps", INT_RAMPS)]);
    let members = members_of(ROOT, vec![line]);
    assert_eq!(
        rows(&members),
        vec![
            row(".zone", 0, 0),
            row(".zone.active", 0, 0),
            row(".zone.ready", 0, 1),
            row(".zone.setpoint", 2, 0),
            row(".zone.count", 6, 0),
            row(".ramps", 8, 0),
        ]
    );
    let names: Vec<&str> = members
        .iter()
        .filter_map(|member| member.names().last().map(MemberName::as_str))
        .collect();
    assert_eq!(
        names,
        ["zone", "active", "ready", "setpoint", "count", "ramps"]
    );
}

#[test]
fn s7_members_golden_nested_struct_after_a_bool_starts_at_the_even_byte() {
    // `flag` takes bit 0.0. `zone` is a struct, so it starts at the even byte 2.0.
    // The members of `ST_Zone` keep their offsets inside the struct:
    // `active` +0.0, `ready` +0.1, `setpoint` +2.0, `count` +6.0.
    // The child frame starts at the start of `zone`, so each row adds 2 bytes.
    let pad = struct_row(ROOT, "ST_Pad", &[("flag", BOOL), ("zone", ST_ZONE)]);
    let members = members_of(ROOT, vec![pad]);
    assert_eq!(
        rows(&members),
        vec![
            row(".flag", 0, 0),
            row(".zone", 2, 0),
            row(".zone.active", 2, 0),
            row(".zone.ready", 2, 1),
            row(".zone.setpoint", 4, 0),
            row(".zone.count", 8, 0),
        ]
    );
}

#[test]
fn s7_members_golden_nested_struct_after_an_int_keeps_its_children_inside_it() {
    // Case of a shrunk property test failure. `m0` is an INT at +0.0 with 2 bytes.
    // `m1` is a struct, so it starts at the even byte 2.0 and takes 2 bytes.
    // Its only member `m0` is an INT at the start of `m1`, so byte 2.
    // The child lies in the bytes 2 to 3 of its parent. The root has 4 bytes.
    const ST_INNER: u128 = 30;
    let inner = struct_row(ST_INNER, "ST_Inner", &[("m0", INT)]);
    let outer = struct_row(ROOT, "ST_Outer", &[("m0", INT), ("m1", ST_INNER)]);
    let members = members_of(ROOT, vec![inner, outer]);
    assert_eq!(
        rows(&members),
        vec![row(".m0", 0, 0), row(".m1", 2, 0), row(".m1.m0", 2, 0)]
    );
    let sizes: Vec<(u32, S7Start)> = members
        .iter()
        .map(|member| (member.layout().size_bytes(), member.layout().start()))
        .collect();
    assert_eq!(
        sizes,
        vec![(2, S7Start::Even), (2, S7Start::Even), (2, S7Start::Even)]
    );
}

#[test]
fn s7_members_golden_sixteen_bools_take_two_bytes() {
    let names: Vec<String> = (0..16).map(|n| format!("b{n}")).collect();
    let pairs: Vec<(&str, u128)> = names.iter().map(|name| (name.as_str(), BOOL)).collect();
    let sixteen = struct_row(SIXTEEN_BOOLS, "ST_Sixteen", &pairs);
    let members = members_of(SIXTEEN_BOOLS, vec![sixteen.clone()]);
    assert_eq!(members.len(), 16);
    assert_eq!(rows(&members).last(), Some(&row(".b15", 1, 7)));
    let tree = TypeTree::resolve(
        id(SIXTEEN_BOOLS),
        &types(vec![primitive_row(BOOL, Primitive::Bool), sixteen]),
    )
    .unwrap();
    assert_eq!(S7Layout::of(&tree).unwrap().size_bytes(), 2);
}

#[test]
fn s7_members_of_a_non_struct_root_is_empty() {
    let tree =
        TypeTree::resolve(id(INT), &types(vec![primitive_row(INT, Primitive::Int)])).unwrap();
    assert!(s7_members(&tree).unwrap().is_empty());
}
