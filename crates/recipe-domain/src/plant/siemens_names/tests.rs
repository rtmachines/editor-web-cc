use super::*;
use crate::plant::{
    DerivedTypeReason, PlantError, PlcPlatform, Primitive, RESERVED_KEYWORDS, TypeName,
};

const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];
const OTHER_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

fn name(text: &str) -> TypeName {
    TypeName::parse(text).unwrap()
}

#[test]
fn adr_0186_the_list_has_269_names_without_a_duplicate_in_any_case() {
    assert_eq!(SIEMENS_RESERVED_NAMES.len(), 269);
    let mut seen = std::collections::BTreeSet::new();
    for text in SIEMENS_RESERVED_NAMES {
        assert!(seen.insert(text.to_ascii_uppercase()), "duplicate {text}");
    }
}

#[test]
fn adr_0186_no_name_is_an_iec_keyword_a_primitive_or_a_telegram_name() {
    for text in SIEMENS_RESERVED_NAMES {
        assert!(
            !RESERVED_KEYWORDS
                .iter()
                .any(|word| word.eq_ignore_ascii_case(text)),
            "{text} is an IEC keyword"
        );
        assert!(
            !Primitive::ALL
                .iter()
                .any(|primitive| primitive.name().eq_ignore_ascii_case(text)),
            "{text} is a primitive name"
        );
        assert!(!is_telegram_name(text), "{text} matches the family");
        assert!(
            TypeName::parse(text).is_ok(),
            "{text} is not a valid struct name"
        );
    }
}

#[test]
fn adr_0186_every_listed_name_is_reserved_in_any_case() {
    for text in SIEMENS_RESERVED_NAMES {
        assert!(is_siemens_reserved_name(text), "{text}");
        assert!(
            is_siemens_reserved_name(&text.to_ascii_uppercase()),
            "{text}"
        );
        assert!(
            is_siemens_reserved_name(&text.to_ascii_lowercase()),
            "{text}"
        );
    }
}

#[test]
fn adr_0186_the_telegram_names_are_reserved_without_case() {
    assert!(is_siemens_reserved_name("PD_TEL3"));
    assert!(is_siemens_reserved_name("pd_tel3"));
    assert!(is_siemens_reserved_name("Pd_Tel12"));
}

#[test]
fn adr_0186_other_names_are_not_reserved() {
    for text in [
        "PD_TELx",
        "PD_TEL",
        "PD_TEL3_IN",
        "PD_TEL3x",
        "ST_Zone",
        "",
        "PID",
    ] {
        assert!(!is_siemens_reserved_name(text), "{text}");
    }
}

#[test]
fn adr_0186_the_telegram_family_holds_leading_zeros() {
    for text in ["PD_TEL0", "PD_TEL03", "PD_TEL003"] {
        assert!(is_siemens_reserved_name(text), "{text}");
    }
}

#[test]
fn adr_0186_the_check_refuses_a_siemens_name_on_the_three_s7_platforms() {
    for platform in S7_PLATFORMS {
        for text in ["PID_Compact", "pid_compact", "PD_TEL3", "BEGIN", "TIMER"] {
            let outcome = check_siemens_struct_name(platform, &name(text));
            assert!(
                matches!(
                    outcome,
                    Err(PlantError::DerivedTypeInvalid {
                        reason: DerivedTypeReason::ReservedName,
                        ..
                    })
                ),
                "{platform:?} {text}: {outcome:?}"
            );
        }
    }
}

#[test]
fn adr_0186_the_check_gives_the_code_e_plt_013() {
    let error = check_siemens_struct_name(PlcPlatform::S7_1500, &name("PID_Compact")).unwrap_err();
    assert_eq!(error.code(), crate::error_codes::ErrorCode::PLT_013);
}

#[test]
fn adr_0186_the_check_accepts_the_name_on_codesys_and_twincat() {
    for platform in OTHER_PLATFORMS {
        for text in ["PID_Compact", "PD_TEL3", "BEGIN"] {
            assert!(
                check_siemens_struct_name(platform, &name(text)).is_ok(),
                "{platform:?} {text}"
            );
        }
    }
}

#[test]
fn adr_0186_the_check_accepts_an_ordinary_name_on_every_platform() {
    for platform in S7_PLATFORMS.into_iter().chain(OTHER_PLATFORMS) {
        assert!(check_siemens_struct_name(platform, &name("ST_Zone")).is_ok());
    }
}
