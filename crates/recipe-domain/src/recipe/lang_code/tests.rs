use super::*;
use crate::error_codes::{ErrorCode, REGISTRY};
use proptest::prelude::*;

#[test]
fn valid_codes_pass() {
    for text in ["pl", "de", "deu", "de-AT"] {
        let code = LangCode::parse(text).unwrap();
        assert_eq!(code.as_str(), text);
        assert_eq!(code.to_string(), text);
    }
}

#[test]
fn invalid_codes_fail_with_rcp_027_and_the_input() {
    for text in [
        "en", "EN", "de-at", "d", "deut", "de-A", "de-ATX", "", "pl ",
    ] {
        let error = LangCode::parse(text).unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_027, "{text:?}");
        assert!(
            matches!(&error, RecipeError::InvalidLangCode { lang } if lang == text),
            "{text:?}"
        );
    }
}

#[test]
fn invalid_lang_code_has_code_registry_row_and_text() {
    let error = RecipeError::InvalidLangCode {
        lang: "xx1".to_owned(),
    };
    assert_eq!(error.code(), ErrorCode::RCP_027);
    assert_eq!(error.code().as_str(), "E-RCP-027");
    assert!(REGISTRY.contains(&ErrorCode::RCP_027));
    assert_eq!(
        error.to_string(),
        "xx1 is not a language code. Use a code such as pl or de-AT."
    );
}

proptest! {
    #[test]
    fn parse_and_display_round_trip(text in "[a-z]{2,3}(-[A-Z]{2})?") {
        prop_assume!(text != "en");
        let code = LangCode::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(code.as_str(), text.as_str());
        prop_assert_eq!(code.to_string(), text);
    }
}
