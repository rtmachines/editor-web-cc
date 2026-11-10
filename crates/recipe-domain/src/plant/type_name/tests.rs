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
fn reserved_words_are_refused_in_any_case() {
    for text in [
        "Int",
        "int",
        "real",
        "ARRAY",
        "OF",
        "STRUCT",
        "END_STRUCT",
        "end_method",
    ] {
        assert_eq!(
            TypeName::parse(text),
            Err(TypeNameReason::Reserved),
            "{text}"
        );
    }
}

#[test]
fn a_name_that_is_not_reserved_is_accepted() {
    assert!(TypeName::parse("ST_Zone").is_ok());
}

#[test]
fn stored_name_under_the_earlier_rules_may_be_reserved_in_another_case() {
    let stored = TypeName::from_stored("Int", RuleSet::Earlier);
    assert_eq!(stored.map(|n| n.to_string()), Ok("Int".to_owned()));
    assert_eq!(
        TypeName::from_stored("Int", RuleSet::Current),
        Err(TypeNameReason::Reserved)
    );
}

#[test]
fn earlier_rules_refuse_a_primitive_name_in_exact_case_and_a_bad_format() {
    assert_eq!(
        TypeName::from_stored("INT", RuleSet::Earlier),
        Err(TypeNameReason::Reserved)
    );
    assert_eq!(
        TypeName::from_stored("1abc", RuleSet::Earlier),
        Err(TypeNameReason::Format)
    );
}

#[test]
fn current_rules_of_from_stored_equal_parse() {
    for text in ["ST_Zone", "INT", "end_method", "1abc"] {
        assert_eq!(
            TypeName::from_stored(text, RuleSet::Current),
            TypeName::parse(text),
            "{text}"
        );
    }
}

#[test]
fn reserved_keywords_are_182_upper_case_words_without_a_primitive_name() {
    assert_eq!(RESERVED_KEYWORDS.len(), 182);
    let mut seen = std::collections::HashSet::new();
    for word in RESERVED_KEYWORDS {
        assert!(seen.insert(word), "duplicate {word}");
        assert_eq!(word, word.to_ascii_uppercase(), "{word}");
        assert!(is_name(word), "{word}");
        assert!(Primitive::parse(word).is_err(), "primitive {word}");
    }
}

#[test]
fn is_reserved_name_covers_keywords_and_primitives_in_any_case() {
    for word in RESERVED_KEYWORDS {
        assert!(is_reserved_name(word), "{word}");
        assert!(is_reserved_name(&word.to_ascii_lowercase()), "{word}");
    }
    for primitive in Primitive::ALL {
        assert!(is_reserved_name(primitive.name()), "{}", primitive.name());
        assert!(is_reserved_name(&primitive.name().to_ascii_lowercase()));
    }
    assert!(!is_reserved_name("ST_Zone"));
}

#[test]
fn member_names_may_be_keywords() {
    for text in ["END_STRUCT", "Int", "array"] {
        assert!(MemberName::parse(text).is_ok(), "{text}");
    }
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

// domain-model.md section 2.1: the name grammar is `[A-Za-z_][A-Za-z0-9_]{0,63}`.
#[test]
fn format_texts_name_the_rule_for_the_first_character() {
    assert_eq!(TypeName::parse("1abc"), Err(TypeNameReason::Format));
    assert_eq!(MemberName::parse("1abc"), Err(MemberNameReason::Format));
    for text in [
        TypeNameReason::Format.to_string(),
        MemberNameReason::Format.to_string(),
    ] {
        assert!(text.contains("first character"), "{text}");
        assert!(text.contains("letter or an underscore"), "{text}");
    }
}

fn assert_fragment(text: &str) {
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
}

// domain-model.md section 2.1 and ADR-0143 (c): 1 to 64 characters, no reserved word.
#[test]
fn type_name_reason_texts_name_the_length_limit_and_the_reserved_words() {
    assert_eq!(
        TypeName::parse(&long_name(64)).map(|n| n.as_str().len()),
        Ok(64)
    );
    assert_eq!(TypeName::parse(&long_name(65)), Err(TypeNameReason::Format));
    assert_eq!(TypeName::parse("INT"), Err(TypeNameReason::Reserved));
    let format = TypeNameReason::Format.to_string();
    let reserved = TypeNameReason::Reserved.to_string();
    assert_fragment(&format);
    assert_fragment(&reserved);
    assert!(format.contains("1 to 64"), "{format}");
    assert!(reserved.contains("keyword"), "{reserved}");
    assert_ne!(format, reserved);
}

// domain-model.md section 2.1 (LeafPath member): 1 to 64 characters.
#[test]
fn member_name_reason_text_names_the_length_limit() {
    assert_eq!(
        MemberName::parse(&long_name(64)).map(|n| n.as_str().len()),
        Ok(64)
    );
    assert_eq!(
        MemberName::parse(&long_name(65)),
        Err(MemberNameReason::Format)
    );
    let member = MemberNameReason::Format.to_string();
    assert_fragment(&member);
    assert!(member.contains("1 to 64"), "{member}");
    assert!(member.contains("member"), "{member}");
    assert_ne!(member, TypeNameReason::Format.to_string());
}
