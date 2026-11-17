//! The type table and the hand-written tag and address lists of the S7 platforms.
// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.

/// The limit of one type in the document.
pub enum Limit {
    /// The type has no limit (A36).
    None,
    /// The lower and the upper bound as canonical decimal text.
    Range(&'static str, &'static str),
    /// The maximum length of a string.
    Length(u16),
}

/// One row of the type table.
pub struct TypeCase {
    pub tag: &'static str,
    /// The value as JSON text, as the request sends it.
    pub request: &'static str,
    /// The value in the document as JSON text.
    pub value: &'static str,
    pub limit: Limit,
}

pub const fn case(
    tag: &'static str,
    request: &'static str,
    value: &'static str,
    limit: Limit,
) -> TypeCase {
    TypeCase {
        tag,
        request,
        value,
        limit,
    }
}

/// The 28 primitives of the S7 matrix without `LDATE`. The time counts follow the base units of
/// `domain-model.md` section 2.1. They were computed by hand from 1970-01-01 = day 0 and
/// 2026-10-12 = day 20738.
pub fn type_table() -> [TypeCase; 28] {
    use Limit::{Length, None, Range};
    [
        case("BOOL", "true", "true", None),
        case("BYTE", "255", r#""255""#, Range("0", "255")),
        case("WORD", "65535", r#""65535""#, Range("0", "65535")),
        case(
            "DWORD",
            "4294967295",
            r#""4294967295""#,
            Range("0", "4294967295"),
        ),
        case(
            "LWORD",
            "18446744073709551615",
            r#""18446744073709551615""#,
            Range("0", "18446744073709551615"),
        ),
        case("SINT", "-128", r#""-128""#, Range("-128", "127")),
        case("USINT", "255", r#""255""#, Range("0", "255")),
        case("INT", "-32768", r#""-32768""#, Range("-32768", "32767")),
        case("UINT", "65535", r#""65535""#, Range("0", "65535")),
        case(
            "DINT",
            "-2147483648",
            r#""-2147483648""#,
            Range("-2147483648", "2147483647"),
        ),
        case(
            "UDINT",
            "4294967295",
            r#""4294967295""#,
            Range("0", "4294967295"),
        ),
        case(
            "LINT",
            "-9223372036854775808",
            r#""-9223372036854775808""#,
            Range("-9223372036854775808", "9223372036854775807"),
        ),
        case(
            "ULINT",
            "18446744073709551615",
            r#""18446744073709551615""#,
            Range("0", "18446744073709551615"),
        ),
        // 3 significant digits.
        case("REAL", "1.5e2", r#""150""#, Range("0", "200")),
        case("LREAL", "-0.1250", r#""-0.125""#, Range("-1", "1")),
        // The `CHAR` value is U+00E9, a character of two UTF-8 bytes. The test pins review Q06.
        case("CHAR", "\"\u{e9}\"", "\"\u{e9}\"", None),
        case("WCHAR", "\"\u{3a9}\"", "\"\u{3a9}\"", None),
        case(
            "STRING",
            "\"Zone \u{e9}\"",
            "\"Zone \u{e9}\"",
            Length(super::STRING_LENGTH),
        ),
        // The `WSTRING` value holds U+03A9. A `WSTRING` on S7 holds no scalar above U+CFFF
        // (ADR-0180 (b)).
        case(
            "WSTRING",
            "\"Z\u{3a9}\"",
            "\"Z\u{3a9}\"",
            Length(super::STRING_LENGTH),
        ),
        case("S5TIME", r#""S5T#1s""#, r#""1000""#, Range("0", "9990000")),
        // S7 `TIME` is signed.
        case(
            "TIME",
            r#""T#-5s""#,
            r#""-5000""#,
            Range("-2147483648", "2147483647"),
        ),
        case(
            "LTIME",
            r#""LT#1m30s""#,
            r#""90000000000""#,
            Range("-9223372036854775808", "9223372036854775807"),
        ),
        // The limit is 2026-01-01 to 2026-12-31.
        case(
            "DATE",
            r#""D#2026-10-12""#,
            r#""20738""#,
            Range("20454", "20818"),
        ),
        case(
            "TIME_OF_DAY",
            r#""TOD#12:30:00.5""#,
            r#""45000500""#,
            Range("0", "86399999"),
        ),
        case(
            "LTOD",
            r#""LTOD#12:30:00.000000001""#,
            r#""45000000000001""#,
            Range("0", "86399999999999"),
        ),
        // The limit is 2026-01-01 to 2026-12-31-23:59:59.
        case(
            "DATE_AND_TIME",
            r#""DT#2026-10-12-12:30:00""#,
            r#""1791808200""#,
            Range("1767225600", "1798761599"),
        ),
        case(
            "LDT",
            r#""LDT#2026-10-12-12:30:00.000000001""#,
            r#""1791808200000000001""#,
            Range("0", "9223372036854775807"),
        ),
        case(
            "DTL",
            r#""DTL#2026-10-12-12:30:00.5""#,
            r#""1791808200500000000""#,
            Range("0", "9223372036854775807"),
        ),
    ]
}

/// The 14 primitives of the `S7_300` platform (`data-model.md` section 7), written by hand.
pub const S7_300_TAGS: [&str; 14] = [
    "BOOL",
    "BYTE",
    "WORD",
    "DWORD",
    "INT",
    "DINT",
    "REAL",
    "CHAR",
    "STRING",
    "S5TIME",
    "TIME",
    "DATE",
    "TIME_OF_DAY",
    "DATE_AND_TIME",
];

/// The 20 primitives of the `S7_1200` platform (`data-model.md` section 7), written by hand.
pub const S7_1200_TAGS: [&str; 20] = [
    "BOOL",
    "BYTE",
    "WORD",
    "DWORD",
    "SINT",
    "USINT",
    "INT",
    "UINT",
    "DINT",
    "UDINT",
    "REAL",
    "LREAL",
    "CHAR",
    "WCHAR",
    "STRING",
    "WSTRING",
    "TIME",
    "DATE",
    "TIME_OF_DAY",
    "DTL",
];

/// The 28 primitives of the `S7_1500` platform without `LDATE` (`data-model.md` section 7),
/// written by hand.
pub const S7_1500_TAGS: [&str; 28] = [
    "BOOL",
    "BYTE",
    "WORD",
    "DWORD",
    "LWORD",
    "SINT",
    "USINT",
    "INT",
    "UINT",
    "DINT",
    "UDINT",
    "LINT",
    "ULINT",
    "REAL",
    "LREAL",
    "CHAR",
    "WCHAR",
    "STRING",
    "WSTRING",
    "S5TIME",
    "TIME",
    "LTIME",
    "DATE",
    "TIME_OF_DAY",
    "LTOD",
    "DATE_AND_TIME",
    "LDT",
    "DTL",
];

/// The addresses of the `S7_300` parameters in `DB10`. Each address is aligned, no two overlap,
/// and the last byte is 57.
pub const S7_300_ADDRESSES: [(&str, &str); 14] = [
    ("BOOL", "DB10.DBX0.0"),
    ("BYTE", "DB10.DBB1"),
    ("WORD", "DB10.DBW2"),
    ("DWORD", "DB10.DBD4"),
    ("INT", "DB10.DBW8"),
    ("DINT", "DB10.DBD10"),
    ("REAL", "DB10.DBD14"),
    ("CHAR", "DB10.DBB18"),
    ("STRING", "DB10.DBB20"),
    ("S5TIME", "DB10.DBW38"),
    ("TIME", "DB10.DBD40"),
    ("DATE", "DB10.DBW44"),
    ("TIME_OF_DAY", "DB10.DBD46"),
    ("DATE_AND_TIME", "DB10.DBB50"),
];
