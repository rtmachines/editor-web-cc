//! The type tables and the hand-written tag and address lists of the platforms.
// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.

use recipe_domain::plant::PlcPlatform;

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

/// The 25 primitives of `CodesysIec` and `TwinCat3` (`data-model.md` section 7). The ends of the
/// 64-bit types and of the time types are written by hand from the base-unit table of
/// `domain-model.md` section 2.1:
///
/// - `LTIME`: 213503 d 23 h 34 min 33 s 709 ms 551 us 615 ns is 2^64 - 1 ns.
/// - `LDATE`: 106751 days is 9 223 286 400 000 000 000 ns. It is 2262-04-11, the last whole day
///   at or below 2^63 - 1 ns. The next day is 9 223 372 800 000 000 000 ns. A count of `LDATE`
///   is a whole number of days, so the limit ends at this day and not at 2^63 - 1.
/// - `LTOD`: 23:59:59.999999999 is 86 399 999 999 999 ns.
/// - `LDT`: 2262-04-11-23:47:16.854775807 is 106751 days and 85 636 s plus 854 775 807 ns, which
///   is 2^63 - 1 ns.
/// - `TIME`: 49 d 17 h 2 min 47 s 295 ms is 4 294 967 295 ms.
/// - `DATE`: 2106-02-07 is day 49710, and 49710 * 86400 = 4 294 944 000 s.
/// - `DATE_AND_TIME`: 2106-02-07-06:28:15 is 4 294 944 000 s + 23 295 s = 4 294 967 295 s.
///
/// `LREAL` holds 15 significant digits, the maximum of ADR-0117. `REAL` holds 3 of the 7 digits.
pub fn codesys_type_table() -> [TypeCase; 25] {
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
        case("REAL", "1.5e2", r#""150""#, Range("0", "200")),
        // 9 digits before the point and 6 after it: 15 significant digits.
        case(
            "LREAL",
            "123456789.012345",
            r#""123456789.012345""#,
            Range("-1000000000", "1000000000"),
        ),
        case(
            "STRING",
            "\"Zone \u{e9}\"",
            "\"Zone \u{e9}\"",
            Length(super::STRING_LENGTH),
        ),
        // The value holds U+03A9, a character of the BMP.
        case(
            "WSTRING",
            "\"Z\u{3a9}\"",
            "\"Z\u{3a9}\"",
            Length(super::STRING_LENGTH),
        ),
        // `TIME` is unsigned on CODESYS and TwinCAT 3.
        case(
            "TIME",
            r#""T#49d17h2m47s295ms""#,
            r#""4294967295""#,
            Range("0", "4294967295"),
        ),
        case(
            "LTIME",
            r#""LT#213503d23h34m33s709ms551us615ns""#,
            r#""18446744073709551615""#,
            Range("0", "18446744073709551615"),
        ),
        case(
            "DATE",
            r#""D#2106-02-07""#,
            r#""49710""#,
            Range("0", "49710"),
        ),
        case(
            "LDATE",
            r#""LD#2262-04-11""#,
            r#""9223286400000000000""#,
            Range("0", "9223286400000000000"),
        ),
        case(
            "TIME_OF_DAY",
            r#""TOD#12:30:00.5""#,
            r#""45000500""#,
            Range("0", "86399999"),
        ),
        case(
            "LTOD",
            r#""LTOD#23:59:59.999999999""#,
            r#""86399999999999""#,
            Range("0", "86399999999999"),
        ),
        case(
            "DATE_AND_TIME",
            r#""DT#2106-02-07-06:28:15""#,
            r#""4294967295""#,
            Range("0", "4294967295"),
        ),
        case(
            "LDT",
            r#""LDT#2262-04-11-23:47:16.854775807""#,
            r#""9223372036854775807""#,
            Range("0", "9223372036854775807"),
        ),
    ]
}

/// The type table of a platform.
pub fn platform_type_table(platform: PlcPlatform) -> Vec<TypeCase> {
    match platform {
        PlcPlatform::CodesysIec | PlcPlatform::TwinCat3 => codesys_type_table().into(),
        PlcPlatform::S7_300 | PlcPlatform::S7_1200 | PlcPlatform::S7_1500 => type_table().into(),
    }
}

/// The 25 primitives of `CodesysIec` and `TwinCat3` (`data-model.md` section 7), written by hand.
/// The matrix has no `CHAR`, `WCHAR`, `DTL`, or `S5TIME` on these platforms.
pub const CODESYS_TAGS: [&str; 25] = [
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
    "STRING",
    "WSTRING",
    "TIME",
    "LTIME",
    "DATE",
    "LDATE",
    "TIME_OF_DAY",
    "LTOD",
    "DATE_AND_TIME",
    "LDT",
];

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
