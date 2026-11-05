use super::*;

fn long_name(length: usize) -> String {
    "a".repeat(length)
}

#[test]
fn type_name_accepts_valid_names() {
    for text in ["ST_Zone", "_x", "a", &long_name(64)] {
        assert!(TypeName::parse(text).is_ok(), "{text}");
    }
}

#[test]
fn member_name_accepts_valid_names() {
    for text in ["ST_Zone", "_x", "a", &long_name(64)] {
        assert!(MemberName::parse(text).is_ok(), "{text}");
    }
}

#[test]
fn type_name_of_64_characters_passes_and_65_fails() {
    assert_eq!(
        TypeName::parse(&long_name(64)).map(|n| n.as_str().len()),
        Ok(64)
    );
    assert_eq!(TypeName::parse(&long_name(65)), Err(TypeNameReason::Format));
}

#[test]
fn member_name_of_64_characters_passes_and_65_fails() {
    assert_eq!(
        MemberName::parse(&long_name(64)).map(|n| n.as_str().len()),
        Ok(64)
    );
    assert_eq!(
        MemberName::parse(&long_name(65)),
        Err(MemberNameReason::Format)
    );
}

#[test]
fn malformed_names_fail_for_both_kinds() {
    for text in ["", "1abc", "ST-Zone", "ST Zone", "Zoné"] {
        assert_eq!(TypeName::parse(text), Err(TypeNameReason::Format), "{text}");
        assert_eq!(
            MemberName::parse(text),
            Err(MemberNameReason::Format),
            "{text}"
        );
    }
}

#[test]
fn primitive_names_are_reserved_for_type_names() {
    for text in ["INT", "STRING", "TIME_OF_DAY"] {
        assert_eq!(
            TypeName::parse(text),
            Err(TypeNameReason::Reserved),
            "{text}"
        );
    }
}

#[test]
fn primitive_names_are_valid_member_names() {
    for text in ["INT", "STRING", "TIME_OF_DAY"] {
        assert!(MemberName::parse(text).is_ok(), "{text}");
    }
}

#[test]
fn reserved_check_is_case_sensitive() {
    assert!(TypeName::parse("Int").is_ok());
    assert!(TypeName::parse("int").is_ok());
}

#[test]
fn display_writes_the_name() {
    assert_eq!(
        TypeName::parse("ST_Zone").map(|n| n.to_string()),
        Ok("ST_Zone".to_owned())
    );
    assert_eq!(
        MemberName::parse("Limit").map(|n| n.to_string()),
        Ok("Limit".to_owned())
    );
}
