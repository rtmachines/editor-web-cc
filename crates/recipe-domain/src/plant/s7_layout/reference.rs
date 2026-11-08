//! Reference table of `S7Layout` (`domain-model.md` section 2.1, ADR-0017).
//!
//! Each row holds an expected size, an expected start class, and the expected
//! rows of `s7_members`. The expected values are literals of the table. No
//! value comes from production code such as `StringLength::size_bytes`.

use super::super::type_tree::fixtures::{
    array_row, id, machine, on_platform, primitive_row, string_row, struct_row, types_for,
    wstring_row,
};
use super::super::{PlatformTypes, PlcPlatform, PlcType, Primitive, TypeTree};
use super::*;

/// Where an expected value of a row comes from.
#[derive(Clone, Copy, Debug)]
pub(super) enum Source {
    /// A table of a Siemens manual. The text names the manual and the page.
    Siemens(&'static str),
    /// A rule of this repository. The text names the document and the section.
    Rule(&'static str),
    /// A derivation from cited Siemens rules. The text names the rule ids of the research record
    /// `docs/plan/review/2026-11-08-wp7-r1-layout-research.md` (ADR-0147 (h)). The owner accepts
    /// it in place of a TIA Portal measurement. A rule that rests on an example or on a literal
    /// reading ends the text with "medium confidence" (ADR-0147 (i)).
    Documented(&'static str),
    /// No source in reach. The text names the open question of the review.
    Unverified(&'static str),
}

impl Source {
    fn describe(self) -> String {
        match self {
            Self::Siemens(text) => format!("Siemens {text}"),
            Self::Documented(text) => format!("documented {text}"),
            Self::Rule(text) => format!("rule {text}"),
            Self::Unverified(text) => format!("unverified {text}"),
        }
    }
}

/// One expected member row: the path text, the byte, and the bit.
pub(super) type MemberRow = (&'static str, u32, u8);

/// One row of the reference table.
pub(super) struct Row {
    pub(super) name: &'static str,
    /// The number of the root type in the shared catalog.
    pub(super) root: u128,
    pub(super) size_bytes: u32,
    pub(super) start: S7Start,
    pub(super) members: &'static [MemberRow],
    pub(super) size_source: Source,
    pub(super) layout_source: Source,
}

impl Row {
    fn describe(&self) -> String {
        format!(
            "{} (size: {}, layout: {})",
            self.name,
            self.size_source.describe(),
            self.layout_source.describe()
        )
    }
}

/// Source of the start class of a primitive. No manual states it.
const LAYOUT_UNVERIFIED: Source = Source::Unverified("Q10");
/// Source of the size of a primitive: the comparison list of the review, S1.
const S1: Source = Source::Siemens("S1 Comparison list for programming languages 02/2025");

/// A row of a primitive root. A leaf has no struct member.
fn leaf(name: &'static str, root: u128, size_bytes: u32, start: S7Start) -> Row {
    Row {
        name,
        root,
        size_bytes,
        start,
        members: &[],
        size_source: S1,
        layout_source: LAYOUT_UNVERIFIED,
    }
}

/// The primitive rows of the catalog, in the order of the rows P01 to P30.
///
/// The number of the first row is 101. `STRING(3)` is 118 and `WSTRING(3)` is 120.
const FIXED_PRIMITIVES: [(u128, Primitive); 26] = [
    (101, Primitive::Bool),
    (102, Primitive::Byte),
    (103, Primitive::Word),
    (104, Primitive::Dword),
    (105, Primitive::Lword),
    (106, Primitive::Sint),
    (107, Primitive::Usint),
    (108, Primitive::Int),
    (109, Primitive::Uint),
    (110, Primitive::Dint),
    (111, Primitive::Udint),
    (112, Primitive::Lint),
    (113, Primitive::Ulint),
    (114, Primitive::Real),
    (115, Primitive::Lreal),
    (116, Primitive::Char),
    (117, Primitive::Wchar),
    (122, Primitive::S5Time),
    (123, Primitive::Time),
    (124, Primitive::Ltime),
    (125, Primitive::Date),
    (126, Primitive::TimeOfDay),
    (127, Primitive::Ltod),
    (128, Primitive::DateAndTime),
    (129, Primitive::Ldt),
    (130, Primitive::Dtl),
];

/// The shared catalog on `S7_1500`. It holds every type of the table once.
///
/// The type solves one problem. Every row of the table and the property test
/// of `prop.rs` resolve their roots over the same fixed ids.
pub(super) fn catalog() -> PlatformTypes {
    let mut rows: Vec<PlcType> = FIXED_PRIMITIVES
        .iter()
        .map(|(n, primitive)| primitive_row(*n, *primitive))
        .collect();
    rows.push(string_row(118, 3));
    rows.push(string_row(119, 254));
    rows.push(wstring_row(120, 3));
    rows.push(wstring_row(121, 254));
    rows.push(struct_row(
        201,
        "ST_Flags",
        &[("a", 101), ("b", 101), ("c", 101)],
    ));
    rows.push(wstring_row(131, 1));
    rows.push(array_row(211, 101, &[(1, 2)]));
    for (root, name, members) in COMPOSITE_STRUCTS {
        rows.push(struct_row(root, name, members));
    }
    for (root, element, bounds) in COMPOSITE_ARRAYS {
        rows.push(array_row(root, element, bounds));
    }
    types_for(
        machine(),
        PlcPlatform::S7_1500,
        on_platform(PlcPlatform::S7_1500, rows),
    )
}

/// The rows P01 to P30: one row per primitive root.
pub(super) fn primitive_rows() -> Vec<Row> {
    use S7Start::{Bit, Byte, Even};
    let mut bool_row = leaf("P01 BOOL", 101, 1, Bit);
    bool_row.size_source = Source::Rule("domain-model.md §2.1 storage unit, S1 gives 1 bit");
    vec![
        bool_row,
        leaf("P02 BYTE", 102, 1, Byte),
        leaf("P03 WORD", 103, 2, Even),
        leaf("P04 DWORD", 104, 4, Even),
        leaf("P05 LWORD", 105, 8, Even),
        leaf("P06 SINT", 106, 1, Byte),
        leaf("P07 USINT", 107, 1, Byte),
        leaf("P08 INT", 108, 2, Even),
        leaf("P09 UINT", 109, 2, Even),
        leaf("P10 DINT", 110, 4, Even),
        leaf("P11 UDINT", 111, 4, Even),
        leaf("P12 LINT", 112, 8, Even),
        leaf("P13 ULINT", 113, 8, Even),
        leaf("P14 REAL", 114, 4, Even),
        leaf("P15 LREAL", 115, 8, Even),
        leaf("P16 CHAR", 116, 1, Byte),
        leaf("P17 WCHAR", 117, 2, Even),
        leaf("P18 STRING(3)", 118, 5, Even),
        leaf("P19 STRING(254)", 119, 256, Even),
        leaf("P20 WSTRING(3)", 120, 10, Even),
        leaf("P21 WSTRING(254)", 121, 512, Even),
        leaf("P22 S5TIME", 122, 2, Even),
        leaf("P23 TIME", 123, 4, Even),
        leaf("P24 LTIME", 124, 8, Even),
        leaf("P25 DATE", 125, 2, Even),
        leaf("P26 TIME_OF_DAY", 126, 4, Even),
        leaf("P27 LTOD", 127, 8, Even),
        leaf("P28 DATE_AND_TIME", 128, 8, Even),
        leaf("P29 LDT", 129, 8, Even),
        leaf("P30 DTL", 130, 12, Even),
    ]
}

/// A member list of a struct root: the member name and the number of its type.
type MemberTypes = &'static [(&'static str, u128)];

/// The struct roots of the composite rows. Each entry is the root number, the
/// struct name, and the members. The struct of the row C`nn` is named `ST_Cnn`.
const COMPOSITE_STRUCTS: [(u128, &str, MemberTypes); 13] = [
    (
        301,
        "ST_C01",
        &[
            ("a", 101),
            ("b", 101),
            ("c", 101),
            ("d", 101),
            ("e", 101),
            ("f", 101),
            ("g", 101),
            ("h", 101),
            ("i", 101),
        ],
    ),
    (302, "ST_C02", &[("a", 101), ("b", 102)]),
    (303, "ST_C03", &[("a", 101), ("b", 108)]),
    (
        304,
        "ST_C04",
        &[("a", 102), ("b", 116), ("c", 106), ("d", 107)],
    ),
    (305, "ST_C05", &[("a", 102), ("b", 118)]),
    (306, "ST_C06", &[("a", 102), ("b", 120)]),
    (307, "ST_C07", &[("a", 131), ("b", 102)]),
    (308, "ST_C08", &[("a", 102), ("b", 201)]),
    (309, "ST_C09", &[("a", 101), ("b", 211)]),
    (310, "ST_C10", &[("a", 102), ("b", 102), ("c", 102)]),
    (311, "ST_C11", &[("a", 118), ("b", 102)]),
    (312, "ST_C12", &[("a", 118), ("b", 108)]),
    (313, "ST_C13", &[("a", 102), ("b", 130)]),
];

/// The array roots of the composite rows: the root number, the element number,
/// and the bounds of each dimension.
type ArrayBounds = &'static [(i32, i32)];
const COMPOSITE_ARRAYS: [(u128, u128, ArrayBounds); 9] = [
    (401, 101, &[(1, 17)]),
    (402, 101, &[(1, 3), (1, 3)]),
    (403, 101, &[(1, 3), (1, 3), (1, 3)]),
    (404, 102, &[(1, 3)]),
    (405, 116, &[(1, 3)]),
    (406, 118, &[(1, 4)]),
    (407, 120, &[(1, 2)]),
    (408, 108, &[(0, 1), (1, 2)]),
    (411, 201, &[(1, 2)]),
];

/// A row of a composite root. The size and the layout have the same mark.
fn composite(
    name: &'static str,
    root: u128,
    size_bytes: u32,
    members: &'static [MemberRow],
    mark: Source,
) -> Row {
    Row {
        name,
        root,
        size_bytes,
        start: S7Start::Even,
        members,
        size_source: mark,
        layout_source: mark,
    }
}

/// The composite rows C01 to C13, A01 to A08, and A11, in the order of the table.
///
/// Every row applies a rule of `domain-model.md` section 2.1. Each row but A03 has the mark
/// `Documented` with the rule ids of the research record (ADR-0147 (h), (i)). The row A03 keeps
/// `Unverified("needs a TIA measurement")`. The rows A09 and A10 are not in the table
/// (ADR-0148 (j)).
pub(super) fn composite_rows() -> Vec<Row> {
    use Source::{Documented, Unverified};
    vec![
        // `BOOL` members pack 8 per byte.
        composite(
            "C01 struct of 9 BOOL",
            301,
            2,
            &[
                (".a", 0, 0),
                (".b", 0, 1),
                (".c", 0, 2),
                (".d", 0, 3),
                (".e", 0, 4),
                (".f", 0, 5),
                (".g", 0, 6),
                (".h", 0, 7),
                (".i", 1, 0),
            ],
            Documented("WP7-R1 S-ST1 S-ST2 S-ST2a S-BOOL, medium confidence"),
        ),
        // A 1-byte member starts at the next byte.
        composite(
            "C02 BOOL, BYTE",
            302,
            2,
            &[(".a", 0, 0), (".b", 1, 0)],
            Documented("WP7-R1 S-ST1 S-ST2"),
        ),
        // A wider member starts at the next even byte.
        composite(
            "C03 BOOL, INT",
            303,
            4,
            &[(".a", 0, 0), (".b", 2, 0)],
            Documented("WP7-R1 S-ST1 S-ST2"),
        ),
        // A `BYTE` and a `CHAR` member start at any byte. A `SINT` and a `USINT` member start
        // at an even byte (ADR-0147 (a)).
        composite(
            "C04 BYTE, CHAR, SINT, USINT",
            304,
            6,
            &[(".a", 0, 0), (".b", 1, 0), (".c", 2, 0), (".d", 4, 0)],
            Documented("WP7-R1 S-ST1 S-ST2 S-ST2b, medium confidence"),
        ),
        // A `STRING` starts on an even byte.
        composite(
            "C05 BYTE, STRING(3)",
            305,
            8,
            &[(".a", 0, 0), (".b", 2, 0)],
            Documented("WP7-R1 S-ST2 S-STR1 S-STR2 S-ST1"),
        ),
        // A `WSTRING` starts on an even byte.
        composite(
            "C06 BYTE, WSTRING(3)",
            306,
            12,
            &[(".a", 0, 0), (".b", 2, 0)],
            Documented("WP7-R1 S-ST2 S-WSTR S-ST1"),
        ),
        // A 1-byte member after a `WSTRING` starts at the next byte.
        composite(
            "C07 WSTRING(1), BYTE",
            307,
            8,
            &[(".a", 0, 0), (".b", 6, 0)],
            Documented("WP7-R1 S-WSTR S-ST2 S-ST1"),
        ),
        // A struct starts on an even byte. A nested member counts from the row start.
        composite(
            "C08 BYTE, ST_Flags",
            308,
            4,
            &[
                (".a", 0, 0),
                (".b", 2, 0),
                (".b.a", 2, 0),
                (".b.b", 2, 1),
                (".b.c", 2, 2),
            ],
            Documented("WP7-R1 S-ST1 S-ST2 S-ST2a S-PAD, medium confidence"),
        ),
        // An array starts on an even byte.
        composite(
            "C09 BOOL, ARRAY[1..2] OF BOOL",
            309,
            4,
            &[(".a", 0, 0), (".b", 2, 0)],
            Documented("WP7-R1 S-AR1 S-PAD S-AR3"),
        ),
        // A struct size is even.
        composite(
            "C10 BYTE, BYTE, BYTE",
            310,
            4,
            &[(".a", 0, 0), (".b", 1, 0), (".c", 2, 0)],
            Documented("WP7-R1 S-ST2 S-ST1"),
        ),
        // A 1-byte member after an odd `STRING(n)` starts at the next byte.
        composite(
            "C11 STRING(3), BYTE",
            311,
            6,
            &[(".a", 0, 0), (".b", 5, 0)],
            Documented("WP7-R1 S-STR1 S-ST2 S-ST1"),
        ),
        // A wider member after an odd `STRING(n)` starts on an even byte.
        composite(
            "C12 STRING(3), INT",
            312,
            8,
            &[(".a", 0, 0), (".b", 6, 0)],
            Documented("WP7-R1 S-STR1 S-ST2 S-ST1"),
        ),
        // A wider member starts on an even byte.
        composite(
            "C13 BYTE, DTL",
            313,
            14,
            &[(".a", 0, 0), (".b", 2, 0)],
            Documented("WP7-R1 S-DTL S-ST2 S-ST1"),
        ),
        // `BOOL` elements pack 8 per byte.
        composite(
            "A01 ARRAY[1..17] OF BOOL",
            401,
            4,
            &[],
            Documented("WP7-R1 S-AR1 S-AR2 S-AR3"),
        ),
        // Each line of the last index of a `BOOL` array starts at the next byte
        // (ADR-0147 (d), (g)).
        composite(
            "A02 ARRAY[1..3,1..3] OF BOOL",
            402,
            4,
            &[],
            Documented("WP7-R1 S-AR4 S-AR1"),
        ),
        // Three dimensions: the reading of "a new dimension" is open (ADR-0147 (g), (i)).
        composite(
            "A03 ARRAY[1..3,1..3,1..3] OF BOOL",
            403,
            10,
            &[],
            Unverified("needs a TIA measurement"),
        ),
        // 1-byte elements pack one per byte.
        composite(
            "A04 ARRAY[1..3] OF BYTE",
            404,
            4,
            &[],
            Documented("WP7-R1 S-AR1 S-AR2 S-PAD"),
        ),
        composite(
            "A05 ARRAY[1..3] OF CHAR",
            405,
            4,
            &[],
            Documented("WP7-R1 S-AR1 S-AR2"),
        ),
        // Wider elements start on even bytes.
        composite(
            "A06 ARRAY[1..4] OF STRING(3)",
            406,
            24,
            &[],
            Documented("WP7-R1 S-STR1 S-STR2 S-AR1"),
        ),
        composite(
            "A07 ARRAY[1..2] OF WSTRING(3)",
            407,
            20,
            &[],
            Documented("WP7-R1 S-WSTR S-AR1"),
        ),
        composite(
            "A08 ARRAY[0..1,1..2] OF INT",
            408,
            8,
            &[],
            Documented("WP7-R1 S-AR4 S-AR1"),
        ),
        // A struct element starts on an even byte.
        composite(
            "A11 ARRAY[1..2] OF ST_Flags",
            411,
            4,
            &[],
            Documented("WP7-R1 S-ST1 S-PAD S-ST2a, medium confidence"),
        ),
    ]
}

/// Every row of the reference table.
pub(super) fn rows() -> Vec<Row> {
    let mut all = primitive_rows();
    all.extend(composite_rows());
    all
}

#[test]
fn s7_layout_matches_the_reference_table() {
    let catalog = catalog();
    for row in rows() {
        let label = row.describe();
        let tree = TypeTree::resolve(id(row.root), &catalog).expect(&label);
        let layout = S7Layout::of(&tree).expect(&label);
        assert_eq!(layout.size_bytes(), row.size_bytes, "size of {label}");
        assert_eq!(layout.start(), row.start, "start of {label}");
        let found: Vec<(String, u32, u8)> = s7_members(&tree)
            .expect(&label)
            .iter()
            .map(|member| (member.path_text(), member.byte(), member.bit()))
            .collect();
        let expected: Vec<(String, u32, u8)> = row
            .members
            .iter()
            .map(|(path, byte, bit)| ((*path).to_owned(), *byte, *bit))
            .collect();
        assert_eq!(found, expected, "members of {label}");
    }
}

/// The mark of each documented composite row, by its id (ADR-0147 (h), (i)).
///
/// The texts are literals of the column "Cited rules" in the section "Rows" of the research
/// record `docs/plan/review/2026-11-08-wp7-r1-layout-research.md`. A rule that rests on an
/// example or on a literal reading ends with "medium confidence".
const DOCUMENTED_ROWS: [(&str, &str); 21] = [
    ("C01", "WP7-R1 S-ST1 S-ST2 S-ST2a S-BOOL, medium confidence"),
    ("C02", "WP7-R1 S-ST1 S-ST2"),
    ("C03", "WP7-R1 S-ST1 S-ST2"),
    ("C04", "WP7-R1 S-ST1 S-ST2 S-ST2b, medium confidence"),
    ("C05", "WP7-R1 S-ST2 S-STR1 S-STR2 S-ST1"),
    ("C06", "WP7-R1 S-ST2 S-WSTR S-ST1"),
    ("C07", "WP7-R1 S-WSTR S-ST2 S-ST1"),
    ("C08", "WP7-R1 S-ST1 S-ST2 S-ST2a S-PAD, medium confidence"),
    ("C09", "WP7-R1 S-AR1 S-PAD S-AR3"),
    ("C10", "WP7-R1 S-ST2 S-ST1"),
    ("C11", "WP7-R1 S-STR1 S-ST2 S-ST1"),
    ("C12", "WP7-R1 S-STR1 S-ST2 S-ST1"),
    ("C13", "WP7-R1 S-DTL S-ST2 S-ST1"),
    ("A01", "WP7-R1 S-AR1 S-AR2 S-AR3"),
    ("A02", "WP7-R1 S-AR4 S-AR1"),
    ("A04", "WP7-R1 S-AR1 S-AR2 S-PAD"),
    ("A05", "WP7-R1 S-AR1 S-AR2"),
    ("A06", "WP7-R1 S-STR1 S-STR2 S-AR1"),
    ("A07", "WP7-R1 S-WSTR S-AR1"),
    ("A08", "WP7-R1 S-AR4 S-AR1"),
    ("A11", "WP7-R1 S-ST1 S-PAD S-ST2a, medium confidence"),
];

/// Return the id of a row, the first word of its name.
fn row_id(row: &Row) -> &'static str {
    row.name.split_whitespace().next().unwrap_or_default()
}

#[test]
fn reference_table_marks_each_composite_row_by_its_id() {
    let composite = composite_rows();
    for (id, rules) in DOCUMENTED_ROWS {
        let row = composite.iter().find(|row| row_id(row) == id).expect(id);
        let label = row.describe();
        assert!(
            matches!(row.size_source, Source::Documented(text) if text == rules),
            "size mark of {label}"
        );
        assert!(
            matches!(row.layout_source, Source::Documented(text) if text == rules),
            "layout mark of {label}"
        );
    }
}

#[test]
fn reference_table_keeps_a03_unverified() {
    let composite = composite_rows();
    let row = composite
        .iter()
        .find(|row| row_id(row) == "A03")
        .expect("the table has the row A03");
    let label = row.describe();
    assert!(
        matches!(
            row.size_source,
            Source::Unverified("needs a TIA measurement")
        ),
        "size mark of {label}"
    );
    assert!(
        matches!(
            row.layout_source,
            Source::Unverified("needs a TIA measurement")
        ),
        "layout mark of {label}"
    );
}

#[test]
fn reference_table_has_no_open_question_mark_and_no_nested_array_row() {
    let composite = composite_rows();
    assert_eq!(composite.len(), 22, "13 C rows and 9 A rows");
    for row in &composite {
        let label = row.describe();
        assert!(
            !matches!(row.size_source, Source::Unverified("Q10")),
            "size mark of {label}"
        );
        assert!(
            !matches!(row.layout_source, Source::Unverified("Q10")),
            "layout mark of {label}"
        );
    }
    let ids: Vec<&str> = composite.iter().map(row_id).collect();
    assert!(!ids.contains(&"A09"), "A09 left the table (ADR-0148 (j))");
    assert!(!ids.contains(&"A10"), "A10 left the table (ADR-0148 (j))");
}
