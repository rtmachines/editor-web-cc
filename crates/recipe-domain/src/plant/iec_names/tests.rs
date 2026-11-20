use std::collections::BTreeSet;

use super::*;
use crate::error_codes::ErrorCode;
use crate::plant::{
    DerivedTypeReason, MemberName, PlantError, PlcPlatform, Primitive, RESERVED_KEYWORDS, TypeName,
    is_siemens_reserved_name,
};

const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];
const IEC_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

fn name(text: &str) -> TypeName {
    TypeName::parse(text).unwrap()
}

fn members(texts: &[&str]) -> Vec<MemberName> {
    texts
        .iter()
        .map(|text| MemberName::parse(text).unwrap())
        .collect()
}

fn reason_of(outcome: &Result<(), PlantError>) -> Option<DerivedTypeReason> {
    match outcome {
        Err(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

#[test]
fn adr_0203_the_list_has_15_struct_names_without_a_duplicate_in_any_case() {
    assert_eq!(IEC_RESERVED_STRUCT_NAMES.len(), 15);
    let mut seen = BTreeSet::new();
    for text in IEC_RESERVED_STRUCT_NAMES {
        assert!(seen.insert(text.to_ascii_uppercase()), "duplicate {text}");
        assert!(TypeName::parse(text).is_ok(), "{text} is not a struct name");
    }
}

#[test]
fn adr_0203_struct_names_are_new() {
    for text in IEC_RESERVED_STRUCT_NAMES {
        assert!(
            !RESERVED_KEYWORDS
                .iter()
                .any(|word| word.eq_ignore_ascii_case(text)),
            "{text} is a keyword of ADR-0143"
        );
        assert!(
            !Primitive::ALL
                .iter()
                .any(|primitive| primitive.name().eq_ignore_ascii_case(text)),
            "{text} is a primitive name of ADR-0143"
        );
        assert!(
            !is_siemens_reserved_name(text),
            "{text} is a Siemens name of ADR-0186"
        );
    }
}

#[test]
fn adr_0203_member_keywords_are_the_keywords_of_adr_0143_and_two_more() {
    assert_eq!(IEC_EXTRA_KEYWORDS, ["UNION", "END_UNION"]);
    let mut keywords: BTreeSet<String> = RESERVED_KEYWORDS
        .iter()
        .map(|word| word.to_ascii_uppercase())
        .collect();
    assert_eq!(keywords.len(), 182);
    for word in IEC_EXTRA_KEYWORDS {
        assert!(keywords.insert(word.to_owned()), "{word} is in ADR-0143");
    }
    assert_eq!(keywords.len(), 184);
    for word in &keywords {
        assert!(is_iec_reserved_member_name(word), "{word}");
        assert!(
            is_iec_reserved_member_name(&word.to_ascii_lowercase()),
            "{word}"
        );
    }
}

#[test]
fn adr_0203_the_extra_keywords_are_struct_names_too() {
    for word in IEC_EXTRA_KEYWORDS {
        assert!(is_iec_reserved_struct_name(word), "{word}");
    }
}

#[test]
fn adr_0203_every_listed_struct_name_is_reserved_in_any_case() {
    for text in IEC_RESERVED_STRUCT_NAMES {
        assert!(is_iec_reserved_struct_name(text), "{text}");
        assert!(
            is_iec_reserved_struct_name(&text.to_ascii_uppercase()),
            "{text}"
        );
        assert!(
            is_iec_reserved_struct_name(&text.to_ascii_lowercase()),
            "{text}"
        );
    }
    for text in [
        "ST_Zone",
        "VERSION2",
        "XINTS",
        "TIMESTRUCT_1",
        "ST_AmsAddr2",
    ] {
        assert!(!is_iec_reserved_struct_name(text), "{text}");
    }
}

#[test]
fn adr_0203_a_member_can_be_a_primitive_name_or_a_type_name_of_the_list() {
    for text in ["INT", "String", "BIT", "VERSION", "PVOID", "T_FILETIME"] {
        assert!(!is_iec_reserved_member_name(text), "{text}");
    }
}

#[test]
fn adr_0203_a_struct_name_is_not_a_member_keyword_by_the_member_rule() {
    assert!(!is_iec_reserved_struct_name("END_IF"));
    assert!(is_iec_reserved_member_name("END_IF"));
}

#[test]
fn adr_0203_the_check_refuses_a_listed_struct_name_on_codesys_and_twincat() {
    for platform in IEC_PLATFORMS {
        for text in [
            "VERSION",
            "version",
            "BIT",
            "UNION",
            "End_Union",
            "ST_AmsAddr",
        ] {
            let outcome = check_iec_names(platform, &name(text), &members(&["A", "B"]));
            assert_eq!(
                reason_of(&outcome),
                Some(DerivedTypeReason::ReservedName),
                "{platform:?} {text}: {outcome:?}"
            );
        }
    }
}

#[test]
fn adr_0203_the_check_refuses_a_keyword_member_on_codesys_and_twincat() {
    for platform in IEC_PLATFORMS {
        for text in ["R", "end_struct", "UNION", "END_UNION", "AT"] {
            let outcome = check_iec_names(platform, &name("ST_Coil"), &members(&["Open", text]));
            assert_eq!(
                reason_of(&outcome),
                Some(DerivedTypeReason::ReservedMemberName),
                "{platform:?} {text}: {outcome:?}"
            );
        }
    }
}

#[test]
fn adr_0203_the_struct_name_comes_before_the_members() {
    let outcome = check_iec_names(
        PlcPlatform::TwinCat3,
        &name("VERSION"),
        &members(&["R", "S"]),
    );
    assert_eq!(reason_of(&outcome), Some(DerivedTypeReason::ReservedName));
}

#[test]
fn adr_0203_the_check_names_the_struct_and_gives_the_code_e_plt_013() {
    let error =
        check_iec_names(PlcPlatform::TwinCat3, &name("ST_Coil"), &members(&["R"])).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(
        matches!(&error, PlantError::DerivedTypeInvalid { type_expr, .. } if type_expr == "ST_Coil"),
        "{error:?}"
    );
}

#[test]
fn adr_0203_the_check_accepts_valid_names_on_codesys_and_twincat() {
    for platform in IEC_PLATFORMS {
        let outcome = check_iec_names(
            platform,
            &name("ST_Coil"),
            &members(&["Open", "Int", "VERSION", "Bit"]),
        );
        assert!(outcome.is_ok(), "{platform:?}: {outcome:?}");
        assert!(check_iec_names(platform, &name("ST_Coil"), &[]).is_ok());
    }
}

#[test]
fn adr_0203_the_check_accepts_every_name_on_the_s7_platforms() {
    for platform in S7_PLATFORMS {
        for text in ["VERSION", "UNION", "BIT"] {
            let outcome =
                check_iec_names(platform, &name(text), &members(&["R", "END_IF", "UNION"]));
            assert!(outcome.is_ok(), "{platform:?} {text}: {outcome:?}");
        }
    }
}
