//! The UI language and the `fluent` bundles (`api.md` section 8, design D13).
//!
//! The language comes from the cookie `recipe_lang` only. The server ignores `Accept-Language`,
//! because a plant workstation often has an English operating system and a Polish operator.

use std::fmt;

use axum::http::HeaderMap;
use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentResource};

use crate::cookies::request_cookie;

/// The name of the cookie that stores the UI language.
pub(crate) const LANG_COOKIE: &str = "recipe_lang";
/// The texts of the Polish bundle.
const PL_SOURCE: &str = include_str!("../i18n/pl.ftl");
/// The texts of the English bundle.
const EN_SOURCE: &str = include_str!("../i18n/en.ftl");
/// The texts of the Polish bundle that the approval and audit pages add.
const PL_APPROVAL_SOURCE: &str = include_str!("../i18n/pl-approval.ftl");
/// The texts of the English bundle that the approval and audit pages add.
const EN_APPROVAL_SOURCE: &str = include_str!("../i18n/en-approval.ftl");

/// The reason that a text is not a supported language.
///
/// The type is a reason of `Lang::parse`, not an error type of its own (`reliability.md` 1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LangReason {
    /// The text is neither `pl` nor `en`.
    Unsupported,
}

impl fmt::Display for LangReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the language must be pl or en")
    }
}

/// A UI language. Polish is the default (R10), English is the second language (A17).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang {
    /// Polish.
    #[default]
    Pl,
    /// English.
    En,
}

impl Lang {
    /// Parse the value of the cookie `recipe_lang` or of the form field `lang`.
    ///
    /// The match is exact and case-sensitive.
    ///
    /// # Errors
    ///
    /// Returns [`LangReason::Unsupported`] for any text except `pl` and `en`.
    pub fn parse(text: &str) -> Result<Self, LangReason> {
        match text {
            "pl" => Ok(Self::Pl),
            "en" => Ok(Self::En),
            _ => Err(LangReason::Unsupported),
        }
    }

    /// Return the code of the language, as the cookie stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }

    /// Read the language from the `Cookie` headers of a request.
    ///
    /// The first cookie of the name `recipe_lang` decides. A missing cookie and any other value
    /// give the default language. The function never reads `Accept-Language`.
    #[must_use]
    pub fn from_headers(headers: &HeaderMap) -> Self {
        request_cookie(headers, LANG_COOKIE)
            .and_then(|value| Self::parse(&value).ok())
            .unwrap_or_default()
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The reason that the text bundle of a language cannot load.
///
/// Only the startup of the server sees it. It is a reason, not an error type of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BundleReason {
    /// The language whose `.ftl` file is not valid.
    pub lang: Lang,
}

impl fmt::Display for BundleReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the text bundle {} is not valid", self.lang)
    }
}

/// The text bundles of both languages.
///
/// The type solves one problem: the router state must be `Send + Sync`, so the bundles use the
/// concurrent memoizer of `fluent`. A bundle formats without isolation marks, because the text
/// goes into HTML and a test compares it with the registry of `reliability.md` 1.2.
pub struct Bundles {
    pl: FluentBundle<FluentResource>,
    en: FluentBundle<FluentResource>,
}

/// Build the bundle of one language from its `.ftl` sources.
///
/// The function solves one problem: a file must stay below 800 lines, so a language has more than
/// one file. A key that two files of one language define fails the load. A later file therefore
/// cannot shadow an earlier text without notice.
fn load_bundle(lang: Lang, sources: &[&str]) -> Result<FluentBundle<FluentResource>, BundleReason> {
    let reason = BundleReason { lang };
    let locale = lang.as_str().parse().map_err(|_failed| reason)?;
    let mut bundle = FluentBundle::new_concurrent(vec![locale]);
    bundle.set_use_isolating(false);
    for source in sources {
        let resource = FluentResource::try_new((*source).to_owned()).map_err(|_failed| reason)?;
        bundle.add_resource(resource).map_err(|_failed| reason)?;
    }
    Ok(bundle)
}

impl Bundles {
    /// Load the bundles `pl` and `en` from the files in `i18n/`.
    ///
    /// Each language has two files: the main file and the approval file.
    ///
    /// # Errors
    ///
    /// Returns the [`BundleReason`] of the first language whose files do not parse or hold a key
    /// twice, also across the two files of the language.
    pub fn load() -> Result<Self, BundleReason> {
        Ok(Self {
            pl: load_bundle(Lang::Pl, &[PL_SOURCE, PL_APPROVAL_SOURCE])?,
            en: load_bundle(Lang::En, &[EN_SOURCE, EN_APPROVAL_SOURCE])?,
        })
    }

    const fn bundle(&self, lang: Lang) -> &FluentBundle<FluentResource> {
        match lang {
            Lang::Pl => &self.pl,
            Lang::En => &self.en,
        }
    }

    /// Format the message `key` in `lang` with the arguments `args`.
    ///
    /// Returns `None` when the bundle has no such message or when formatting fails, for example
    /// because an argument is missing. The caller then falls back to the English text of the error.
    #[must_use]
    pub fn message(&self, lang: Lang, key: &str, args: Option<&FluentArgs<'_>>) -> Option<String> {
        let bundle = self.bundle(lang);
        let pattern = bundle.get_message(key)?.value()?;
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, args, &mut errors);
        errors.is_empty().then(|| text.into_owned())
    }

    /// Return true when the bundle of `lang` holds the message `key`.
    #[cfg(test)]
    pub(crate) fn has_message(&self, lang: Lang, key: &str) -> bool {
        self.bundle(lang).has_message(key)
    }

    /// Return the message ids of the source files of `lang`.
    #[cfg(test)]
    pub(crate) fn keys(&self, lang: Lang) -> Vec<&'static str> {
        let sources = match lang {
            Lang::Pl => [PL_SOURCE, PL_APPROVAL_SOURCE],
            Lang::En => [EN_SOURCE, EN_APPROVAL_SOURCE],
        };
        sources
            .into_iter()
            .flat_map(str::lines)
            .filter_map(|line| line.split_once(" =").map(|(id, _text)| id))
            .filter(|id| {
                !id.starts_with('-')
                    && !id.is_empty()
                    && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod plant_error_tests;

#[cfg(test)]
mod plant_texts_tests;

#[cfg(test)]
mod review_texts_tests;

#[cfg(test)]
mod recipe_texts_tests;

#[cfg(test)]
mod resource_tests;

#[cfg(test)]
mod approval_texts_tests;

#[cfg(test)]
mod approval_usage_tests;
