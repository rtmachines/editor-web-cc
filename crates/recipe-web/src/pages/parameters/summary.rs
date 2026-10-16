//! The definition of a parameter as text, and the page of a parameter (design D7).
//!
//! The module solves one problem: the parameter page and the answer of a save show the same
//! definition. One view gives `#parameter-summary` the same lines in both places.

use askama::Template;
use recipe_app::recipe::ParameterView;
use recipe_domain::recipe::ParameterKey;

use super::definition::Definition;
use super::form_view::ParameterFormView;
use super::grid::GridView;
use super::translations::TranslationsBlock;
use crate::layout::Layout;
use crate::pages::texts::Texts;

/// The definition of a parameter in `#parameter-summary`.
pub(crate) struct SummaryView {
    /// The key of the parameter.
    pub(crate) key: String,
    /// The path of the unit of the parameter.
    pub(crate) unit: String,
    /// The tag.
    pub(crate) tag: String,
    /// The English name.
    pub(crate) name_en: String,
    /// The English description.
    pub(crate) description_en: String,
    /// The type text, for example `ARRAY[1..8] OF INT`.
    pub(crate) type_text: String,
    /// The S7 address, or `None` when the parameter has none.
    pub(crate) s7_address: Option<String>,
    /// The order of the parameter in its unit.
    pub(crate) ordinal: String,
    /// The success sentence of a save for `#toast`. Empty on the page.
    pub(crate) toast: String,
}

impl SummaryView {
    /// Build the summary of a stored parameter.
    pub(crate) fn stored(view: &ParameterView) -> Self {
        let parameter = &view.parameter;
        Self {
            key: parameter.key.to_string(),
            unit: view.unit_path.to_string(),
            tag: parameter.tag.to_string(),
            name_en: parameter.name_en.to_string(),
            description_en: parameter.description_en.to_string(),
            type_text: view.type_text.clone(),
            s7_address: parameter.s7_address.as_ref().map(ToString::to_string),
            ordinal: parameter.ordinal.value().to_string(),
            toast: String::new(),
        }
    }

    /// Build the summary of a saved definition with the success sentence `toast`.
    pub(crate) fn saved(key: ParameterKey, definition: &Definition, toast: String) -> Self {
        Self {
            key: key.to_string(),
            unit: definition.unit_path.to_string(),
            tag: definition.tag.to_string(),
            name_en: definition.name_en.to_string(),
            description_en: definition.description_en.to_string(),
            type_text: definition.type_expr.to_string(),
            s7_address: definition.s7_address.as_ref().map(ToString::to_string),
            ordinal: definition.ordinal.value().to_string(),
            toast,
        }
    }
}

/// The form of the removal of a parameter in its danger disclosure (ADR-0040 (f)).
pub(crate) struct RemoveView {
    /// The address of `RemoveParameter`.
    pub(crate) action: String,
    /// The tag that the danger button names.
    pub(crate) tag: String,
    /// The CSRF token of the session.
    pub(crate) csrf_token: String,
    /// The text of the hidden field `row_version`.
    pub(crate) row_version: String,
}

/// The answer `parameter-summary.html` of a save: the definition, the toast, and the empty panel.
#[derive(Template)]
#[template(path = "parameter-summary.html")]
pub(crate) struct SummaryFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The saved definition.
    pub(crate) summary: SummaryView,
}

/// The page `parameter.html`: a parameter, or the form of a new parameter.
///
/// The type solves one problem: one template shows a parameter with its edit form, a parameter with
/// the read-only grid, and the form of a new parameter alone.
#[derive(Template)]
#[template(path = "parameter.html")]
pub(crate) struct ParameterPage<'a> {
    /// The layout of the page.
    pub(crate) layout: Layout<'a>,
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The definition of a stored parameter, or `None` on the page of a new parameter.
    pub(crate) summary: Option<SummaryView>,
    /// The form, or `None` for a reader without `ParameterEdit` or a version that is not a draft.
    pub(crate) form: Option<ParameterFormView>,
    /// The read-only grid of the value and the limits when the page has no form.
    pub(crate) values: Option<GridView>,
    /// The translation rows of a stored parameter.
    pub(crate) translations: Option<TranslationsBlock>,
    /// The danger disclosure of the removal, for a reader with the edit form.
    pub(crate) remove: Option<RemoveView>,
    /// The address of the version page.
    pub(crate) version_href: String,
    /// The title of the version, the text of the link to it.
    pub(crate) version_text: String,
}
