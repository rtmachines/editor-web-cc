//! The page texts of one request (`api.md` section 8, design D13).

use fluent::FluentArgs;
use recipe_domain::identity::Role;

use crate::i18n::{Bundles, Lang};

/// The texts of one language.
///
/// The type solves one problem: a template must call `t.get("key")` and get the text of the UI
/// language, and no template may build a text from user data. A key that a bundle lacks shows the
/// key itself, and a test finds every key of every template in both bundles.
#[derive(Clone, Copy)]
pub(crate) struct Texts<'a> {
    bundles: &'a Bundles,
    lang: Lang,
}

impl<'a> Texts<'a> {
    /// Build the texts of `lang` over the bundles.
    pub(crate) const fn new(bundles: &'a Bundles, lang: Lang) -> Self {
        Self { bundles, lang }
    }

    /// Return the text of the message `key`.
    pub(crate) fn get(&self, key: &str) -> String {
        self.bundles
            .message(self.lang, key, None)
            .unwrap_or_else(|| key.to_owned())
    }

    /// Return the text of the message `key` with arguments.
    pub(crate) fn with(&self, key: &str, args: &FluentArgs<'_>) -> String {
        self.bundles
            .message(self.lang, key, Some(args))
            .unwrap_or_else(|| key.to_owned())
    }

    /// Return the name of a role in the language of the page.
    pub(crate) fn role(&self, role: Role) -> String {
        self.get(&format!("role-{}", role.as_sql().replace('_', "-")))
    }
}

#[cfg(test)]
mod tests;
