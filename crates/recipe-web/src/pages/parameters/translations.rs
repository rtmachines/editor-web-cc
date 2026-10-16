//! The translation rows of a parameter and their routes (`api.md` section 2.6, ADR-0040 (a), (b),
//! (f), ADR-0043 (f)).
//!
//! The page shows one row per tag of `i18n.translation_langs` and one row per other tag of the
//! parameter. Each listed row of a draft is one form whose `action` holds the tag. A save answers
//! the new row. A removal answers every row. A tag outside the list gives `422 E-WEB-008` on the
//! route that writes, so its row shows the texts and only the removal. The removal accepts every
//! tag that the parameter holds.

use askama::Template;
use recipe_domain::recipe::{LangCode, Translation};

use crate::error::FormErrors;
use crate::pages::texts::Texts;

/// One translation row: the texts, the form, and the removal of one tag.
pub(crate) struct TranslationRow {
    /// The language tag, for example `pl`.
    pub(crate) lang: String,
    /// The `id` of the name input, `translation-<lang>-name`.
    pub(crate) name_id: String,
    /// The `id` of the description input, `translation-<lang>-description`.
    pub(crate) description_id: String,
    /// The translated name, or the sent text of an error answer.
    pub(crate) name: String,
    /// The translated description, or the sent text of an error answer.
    pub(crate) description: String,
    /// True when the parameter holds a translation of this tag. Only then a removal exists.
    pub(crate) stored: bool,
    /// True when the row may hold a form: a live draft and a role with `ParameterEdit`.
    pub(crate) editable: bool,
    /// True when `i18n.translation_langs` holds the tag. Only then the row is a form to save.
    /// A row of another tag shows its texts and only the removal (ADR-0043 (f)).
    pub(crate) listed: bool,
    /// The address of `UpsertTranslation`.
    pub(crate) action: String,
    /// The address of `RemoveTranslation`.
    pub(crate) delete_action: String,
    /// The CSRF token of the session.
    pub(crate) csrf_token: String,
    /// The text of the hidden field `row_version`.
    pub(crate) row_version: String,
    /// True when an error answer swaps the form out of band (ADR-0041 (e)).
    pub(crate) oob: bool,
    /// The success sentence of a save for `#toast`. Empty in a list.
    pub(crate) toast: String,
    /// The error lines under the inputs.
    pub(crate) errors: FormErrors,
}

/// What every row of one parameter shares: the address, the token, the version, and the mode.
pub(crate) struct RowSource {
    /// The address of the parameter page, for example `/recipes/R-100/versions/3/parameters/<key>`.
    pub(crate) parameter_href: String,
    /// The CSRF token of the session.
    pub(crate) csrf_token: String,
    /// The `row_version` of the read.
    pub(crate) row_version: String,
    /// True when the rows are forms.
    pub(crate) editable: bool,
}

impl TranslationRow {
    /// Build the row of a listed tag `lang` with the stored translation, if any.
    pub(crate) fn new(source: &RowSource, lang: &LangCode, stored: Option<&Translation>) -> Self {
        let lang = lang.to_string();
        Self {
            name_id: format!("translation-{lang}-name"),
            description_id: format!("translation-{lang}-description"),
            name: stored
                .map(|found| found.name.to_string())
                .unwrap_or_default(),
            description: stored
                .and_then(|found| found.description.as_ref())
                .map(ToString::to_string)
                .unwrap_or_default(),
            stored: stored.is_some(),
            editable: source.editable,
            listed: true,
            action: format!("{}/translations/{lang}", source.parameter_href),
            delete_action: format!("{}/translations/{lang}/delete", source.parameter_href),
            csrf_token: source.csrf_token.clone(),
            row_version: source.row_version.clone(),
            oob: false,
            toast: String::new(),
            errors: FormErrors::default(),
            lang,
        }
    }
}

/// Build one row per tag of `langs`, then one row per other tag of `stored`, in their order.
pub(crate) fn translation_rows(
    source: &RowSource,
    langs: &[LangCode],
    stored: &[Translation],
) -> Vec<TranslationRow> {
    let listed = langs.iter().map(|lang| {
        let found = stored.iter().find(|translation| translation.lang == *lang);
        TranslationRow::new(source, lang, found)
    });
    let others = stored
        .iter()
        .filter(|translation| !langs.contains(&translation.lang))
        .map(|translation| TranslationRow {
            listed: false,
            ..TranslationRow::new(source, &translation.lang, Some(translation))
        });
    listed.chain(others).collect()
}

/// The translation rows of a parameter with the English base text.
pub(crate) struct TranslationsBlock {
    /// The English name, the base text of each row.
    pub(crate) name_en: String,
    /// The English description, the base text of each row.
    pub(crate) description_en: String,
    /// One row per tag.
    pub(crate) rows: Vec<TranslationRow>,
    /// The success sentence of a removal for `#toast`. Empty on the page.
    pub(crate) toast: String,
}

/// The fragment `translations.html`: the rows of `GET .../translations` and of a removal.
#[derive(Template)]
#[template(path = "translations.html")]
pub(crate) struct TranslationsFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The rows.
    pub(crate) block: TranslationsBlock,
}

/// The fragment `translation-row.html`: one row with its toast, or its form alone.
#[derive(Template)]
#[template(path = "translation-row.html")]
pub(crate) struct TranslationRowFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The row.
    pub(crate) row: TranslationRow,
    /// True for the form alone, the out-of-band form of an error answer.
    pub(crate) form_only: bool,
}
