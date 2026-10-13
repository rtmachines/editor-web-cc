//! Language code and translation (`domain-model.md` section 2.2, ADR-0012).

use super::RecipeError;
use crate::text::{LongText, Text};

use std::fmt;

/// Code of a translation language, such as `pl`, `deu`, or `de-AT`.
///
/// The grammar is `^[a-z]{2,3}(-[A-Z]{2})?$`. The value `en` is not a
/// translation language, because the base text is English (A17). The parser
/// does not fold case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LangCode(String);

/// Return true when `text` matches `^[a-z]{2,3}(-[A-Z]{2})?$`.
fn matches_grammar(text: &str) -> bool {
    let (language, region) = match text.split_once('-') {
        Some((language, region)) => (language, Some(region)),
        None => (text, None),
    };
    let language_ok =
        (2..=3).contains(&language.len()) && language.bytes().all(|b| b.is_ascii_lowercase());
    let region_ok =
        region.is_none_or(|r| r.len() == 2 && r.bytes().all(|b| b.is_ascii_uppercase()));
    language_ok && region_ok
}

impl LangCode {
    /// Parse a language code. The check does not fold case.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::InvalidLangCode`] when the text does not match
    /// the grammar or is `en`.
    pub fn parse(text: &str) -> Result<Self, RecipeError> {
        if text == "en" || !matches_grammar(text) {
            return Err(RecipeError::InvalidLangCode {
                lang: text.to_owned(),
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// Return the code as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LangCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Name and description of a recipe in one language (`domain-model.md` section 2.2, ADR-0012).
///
/// The type groups the three fields of one translation. A description of
/// `None` means an empty description (design D15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Translation {
    /// The language of the translation.
    pub lang: LangCode,
    /// The translated name.
    pub name: Text,
    /// The translated description. `None` means empty.
    pub description: Option<LongText>,
}

#[cfg(test)]
mod tests {
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
}
