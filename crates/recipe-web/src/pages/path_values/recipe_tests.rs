//! Tests of the parsers of the path values of the recipe pages (task 8.1, design D6).
//!
//! The values are `{ref}`, `{n}`, `{key}`, and `{lang}` (`api.md` section 1). A bad `{ref}`, `{n}`,
//! or `{key}` gives `404 E-WEB-005` and a variant of `Missing` that holds no text. A bad `{lang}`
//! gives `422 E-RCP-027`. No bad value gives a `500`.

use recipe_app::error::{AppError, Missing};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::RecipeError;

use super::{lang, parameter_key, reference, version_number};
use crate::error::WebError;

/// A parameter key in the hyphenated form.
const KEY: &str = "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10";

fn is_missing(error: &WebError, expected: &Missing) -> bool {
    matches!(error, WebError::App(AppError::NotFound { entity }) if entity == expected)
}

#[test]
fn recipe_path_values_reference_accepts_a_reference_number_and_folds_the_case() {
    assert_eq!(reference("R-100").unwrap().to_string(), "R-100");
    assert_eq!(reference("r-100").unwrap().to_string(), "R-100");
    assert_eq!(reference("7").unwrap().to_string(), "7");
    let longest = "A".repeat(50);
    assert_eq!(reference(&longest).unwrap().to_string(), longest);
}

#[test]
fn recipe_path_values_reference_refuses_anything_else_with_the_malformed_recipe() {
    let too_long = "A".repeat(51);
    let bad = [
        "",
        " R-100",
        "R-100 ",
        "R 100",
        "-R100",
        "_R100",
        "R.100",
        "R/100",
        "R-100\n",
        "\u{141}-1",
        too_long.as_str(),
    ];
    for text in bad {
        let error = reference(text).unwrap_err();
        assert!(is_missing(&error, &Missing::MalformedRecipe), "{text:?}");
        assert_eq!(error.code(), ErrorCode::WEB_005, "{text:?}");
    }
}

#[test]
fn recipe_path_values_version_number_is_a_whole_number_from_1_without_a_sign() {
    assert_eq!(version_number("1").unwrap(), 1);
    assert_eq!(version_number("3").unwrap(), 3);
    assert_eq!(version_number("4294967295").unwrap(), u32::MAX);
}

#[test]
fn recipe_path_values_version_number_refuses_anything_else_with_the_malformed_version() {
    let bad = [
        "",
        "0",
        "01",
        "+1",
        "-1",
        " 1",
        "1 ",
        "1.0",
        "x",
        "4294967296",
        "\u{661}",
    ];
    for text in bad {
        let error = version_number(text).unwrap_err();
        assert!(is_missing(&error, &Missing::MalformedVersion), "{text:?}");
        assert_eq!(error.code(), ErrorCode::WEB_005, "{text:?}");
    }
}

#[test]
fn recipe_path_values_parameter_key_accepts_only_the_hyphenated_uuid() {
    assert_eq!(parameter_key(KEY).unwrap().to_string(), KEY);
    assert_eq!(parameter_key(&KEY.to_uppercase()).unwrap().to_string(), KEY);
}

#[test]
fn recipe_path_values_parameter_key_non_ascii_cases_have_the_length_of_a_uuid() {
    for text in [
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e\u{e9}",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e\u{142}",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a\u{142}\u{142}",
    ] {
        assert_eq!(text.len(), 36, "{text:?}");
        assert!(parameter_key(text).is_err(), "{text:?}");
    }
}

#[test]
fn recipe_path_values_parameter_key_refuses_anything_else_with_the_malformed_parameter() {
    let bad = [
        "",
        "LINE",
        "0b9d8a4e5f1c4c529c1e2f6f3c1a7e10",
        "{0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10}",
        "urn:uuid:0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e1",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e100",
        " 0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e10 ",
        // 36 bytes with a non-ASCII letter: the text has the length of a UUID and must still fail.
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e\u{e9}",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a7e\u{142}",
        "0b9d8a4e-5f1c-4c52-9c1e-2f6f3c1a\u{142}\u{142}",
    ];
    for text in bad {
        let error = parameter_key(text).unwrap_err();
        assert!(is_missing(&error, &Missing::MalformedParameter), "{text:?}");
        assert_eq!(error.code(), ErrorCode::WEB_005, "{text:?}");
    }
}

#[test]
fn recipe_path_values_lang_accepts_a_language_code() {
    for text in ["pl", "de", "deu", "de-AT"] {
        assert_eq!(lang(text).unwrap().to_string(), text);
    }
}

#[test]
fn recipe_path_values_lang_refuses_anything_else_with_e_rcp_027() {
    let bad = [
        "", "en", "p", "PL", "pl-pl", "pl_PL", "pl ", " pl", "polish", "\u{141}l", "pl%00",
    ];
    for text in bad {
        let error = lang(text).unwrap_err();
        assert!(
            matches!(
                error,
                WebError::App(AppError::Recipe(RecipeError::InvalidLangCode { .. }))
            ),
            "{text:?}"
        );
        assert_eq!(error.code(), ErrorCode::RCP_027, "{text:?}");
    }
}

#[test]
fn recipe_path_values_no_bad_value_gives_a_server_error_code() {
    let codes = [
        reference("").unwrap_err().code(),
        version_number("0").unwrap_err().code(),
        parameter_key("x").unwrap_err().code(),
        lang("EN").unwrap_err().code(),
    ];
    for code in codes {
        assert_ne!(code, ErrorCode::WEB_010);
        assert_ne!(code, ErrorCode::WEB_011);
    }
}
