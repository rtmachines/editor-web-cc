//! The view of the parameter form: the form of a new parameter and the edit form (design D6 to D8).
//!
//! The module solves one problem: the fragment, the page, the answer of `fields=1`, and the
//! out-of-band form of an error answer show the same form. One view gives them the same fields,
//! the same grid, and the same `row_version`, so they cannot disagree (ADR-0039 (h), ADR-0041 (e)).

use askama::Template;
use recipe_app::recipe::{ParameterForm, ParameterView};
use recipe_domain::plant::{LeafPath, PlcPlatform, TypeTree};
use recipe_domain::recipe::{ParameterValue, ValueFault};

use super::fields::SentFields;
use super::grid::{GridSource, GridView, LimitTexts, ValueTexts};
use crate::error::FormErrors;
use crate::pages::recipes::Choice;
use crate::pages::texts::Texts;

/// The texts of the definition fields as the form shows them.
///
/// The type solves one problem: an answer without a write shows the sent texts, a new form shows
/// the defaults, and an edit form shows the stored definition. All three fill one struct.
#[derive(Clone, Debug, Default)]
pub(crate) struct DefinitionTexts {
    /// The text of the hidden field `row_version`.
    pub(crate) row_version: String,
    /// The unit path of the select `unit_path`.
    pub(crate) unit: String,
    /// The text of the input `tag`.
    pub(crate) tag: String,
    /// The text of the input `name_en`.
    pub(crate) name_en: String,
    /// The text of the input `description_en`.
    pub(crate) description_en: String,
    /// The text of the input `type`.
    pub(crate) type_text: String,
    /// The text of the input `s7_address`.
    pub(crate) s7_address: String,
    /// The text of the input `ordinal`.
    pub(crate) ordinal: String,
}

impl DefinitionTexts {
    /// Take the texts of the definition fields as the form sent them (ADR-0039 (h)).
    pub(crate) fn from_sent(sent: &SentFields) -> Self {
        let text = |name: &str| sent.text(name).unwrap_or_default().to_owned();
        Self {
            row_version: text("row_version"),
            unit: text("unit_path"),
            tag: text("tag"),
            name_en: text("name_en"),
            description_en: text("description_en"),
            type_text: text("type"),
            s7_address: text("s7_address"),
            ordinal: text("ordinal"),
        }
    }

    /// Take the texts of a stored parameter and the `row_version` of the read (ADR-0042).
    pub(crate) fn stored(view: &ParameterView) -> Self {
        let parameter = &view.parameter;
        Self {
            row_version: view.detail.row_version.to_string(),
            unit: view.unit_path.to_string(),
            tag: parameter.tag.to_string(),
            name_en: parameter.name_en.to_string(),
            description_en: parameter.description_en.to_string(),
            type_text: view.type_text.clone(),
            s7_address: parameter
                .s7_address
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            ordinal: parameter.ordinal.value().to_string(),
        }
    }

    /// Build the texts of an empty form of a new parameter: the unit of the query and the order 0.
    pub(crate) fn empty(form: &ParameterForm) -> Self {
        let unit = form
            .unit
            .as_ref()
            .or_else(|| form.units.first().map(|unit| unit.path()))
            .map(ToString::to_string)
            .unwrap_or_default();
        Self {
            row_version: form.detail.row_version.to_string(),
            unit,
            ordinal: "0".to_owned(),
            ..Self::default()
        }
    }
}

/// What a grid of a form shows: the tree, the texts of the inputs, and the pressed mode button.
pub(crate) struct GridTexts<'a> {
    /// The tree of the type of the grid.
    pub(crate) tree: &'a TypeTree,
    /// The text of each value input.
    pub(crate) values: ValueTexts,
    /// The limit rows.
    pub(crate) limits: Vec<LimitTexts>,
    /// The array level of the pressed mode button, if any.
    pub(crate) toggle: Option<&'a LeafPath>,
}

impl<'a> GridTexts<'a> {
    /// Take the stored value and the stored limits of a parameter (A43).
    ///
    /// # Errors
    ///
    /// Returns the fault of a stored value that does not match its tree, a defect of the storage.
    pub(crate) fn stored(view: &'a ParameterView) -> Result<Self, ValueFault> {
        let value = ParameterValue::try_from((&view.parameter.value, &view.tree))?;
        Ok(Self {
            tree: &view.tree,
            values: ValueTexts::from_stored(&value.texts(&view.tree)?),
            limits: LimitTexts::from_stored(&view.tree, &view.parameter.limits),
            toggle: None,
        })
    }

    /// Take the value fields and the limit rows of a form as the form sent them.
    pub(crate) fn sent(tree: &'a TypeTree, sent: &'a SentFields) -> Self {
        Self {
            tree,
            values: ValueTexts::from_sent(sent.values()),
            limits: LimitTexts::from_sent(sent.limits()),
            toggle: sent.limit_mode(),
        }
    }
}

/// Build the grid of a form in the language of `t` (design D8).
///
/// `root_label` names the row of a primitive parameter: the tag, or the title of a new parameter.
pub(crate) fn grid_of(t: Texts<'_>, texts: GridTexts<'_>, chunk: usize, tag: &str) -> GridView {
    let root_label = if tag.is_empty() {
        t.get("parameter-new-title")
    } else {
        tag.to_owned()
    };
    let source = GridSource {
        tree: texts.tree,
        values: texts.values,
        limits: texts.limits,
        toggle: texts.toggle,
        chunk,
        root_label,
    };
    GridView::build(source, t)
}

/// Where a form stands (design D7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// The region `#parameter-form` or the answer of `fields=1`: a boosted form.
    Fragment,
    /// The page of the form. It has no `#parameter-rows`, so the form of a new parameter targets
    /// `#content` like the other stand-alone forms. The edit form of the page is plain.
    Page,
    /// The out-of-band form of an error answer or of a success (ADR-0041 (e) and (h)).
    OutOfBand,
    /// The out-of-band form of an error answer of the page form. It keeps the attributes of `Page`.
    PageOutOfBand,
}

impl Place {
    /// Return true for the form of the page and its out-of-band form.
    const fn is_page(self) -> bool {
        matches!(self, Self::Page | Self::PageOutOfBand)
    }
}

/// What a form shows besides the data of the query: the texts, the grid, and the notice.
pub(crate) struct Shown {
    /// The texts of the definition fields.
    pub(crate) texts: DefinitionTexts,
    /// The leaf grid, or `None` before the user names a type.
    pub(crate) grid: Option<GridView>,
    /// The type of the grid for the hidden field `value_type`. Empty without a grid.
    pub(crate) value_type: String,
    /// The notice above the grid, for example `value-type-notice`. Empty for none.
    pub(crate) notice: String,
}

/// The data of `parameter-form.html` (design D6).
pub(crate) struct ParameterFormView {
    /// True for the edit form of a stored parameter. It sends to `/definition`.
    pub(crate) edit: bool,
    /// True when the form has the literal `hx-*` attributes of design D7.
    pub(crate) boosted: bool,
    /// True when the boosted form stands alone on its page and targets `#content` (design D7).
    pub(crate) standalone: bool,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(crate) oob: bool,
    /// The address of the command.
    pub(crate) action: String,
    /// The text of the `h2` of the form. Empty for no heading.
    pub(crate) heading: String,
    /// The fixed `id` of the save button.
    pub(crate) save_id: &'static str,
    /// The label of the save button.
    pub(crate) save_label: String,
    /// The CSRF token of the session.
    pub(crate) csrf_token: String,
    /// The live units of the machine as choices.
    pub(crate) units: Vec<Choice>,
    /// True when the platform has an S7 address field (A31).
    pub(crate) has_s7: bool,
    /// True when the S7 address is required (`S7_300`).
    pub(crate) s7_required: bool,
    /// The proposals of the type input.
    pub(crate) choices: Vec<String>,
    /// The texts, the grid, and the notice.
    pub(crate) shown: Shown,
    /// The error lines under the inputs.
    pub(crate) errors: FormErrors,
}

/// Return whether a platform has an S7 address field, and whether the field is required (A31).
const fn s7_field(platform: PlcPlatform) -> (bool, bool) {
    match platform {
        PlcPlatform::S7_300 => (true, true),
        PlcPlatform::S7_1200 | PlcPlatform::S7_1500 => (true, false),
        PlcPlatform::CodesysIec | PlcPlatform::TwinCat3 => (false, false),
    }
}

/// Return one choice per live unit of the machine: the unit path, then its name.
fn unit_choices(form: &ParameterForm) -> Vec<Choice> {
    form.units
        .iter()
        .map(|unit| {
            let path = unit.path().to_string();
            let text = format!("{path} {}", unit.name());
            Choice::new(&path, text)
        })
        .collect()
}

impl ParameterFormView {
    /// Build the view from the result of `GetParameterForm` and what the form shows.
    ///
    /// A result with a parameter gives the edit form. Any other result gives the form of a new
    /// parameter.
    pub(crate) fn new(
        t: Texts<'_>,
        form: &ParameterForm,
        (shown, place): (Shown, Place),
        csrf_token: String,
        errors: FormErrors,
    ) -> Self {
        let base = format!(
            "/recipes/{}/versions/{}/parameters",
            form.recipe.reference(),
            form.detail.number
        );
        let key = form.parameter.as_ref().map(|parameter| parameter.key);
        let (action, save_id, save_label) = match key {
            Some(key) => (
                format!("{base}/{key}/definition"),
                "parameter-save-submit",
                t.get("parameter-save-submit"),
            ),
            None => (base, "parameter-add-submit", t.get("parameter-add-submit")),
        };
        let heading = match (key, place) {
            (None, Place::Fragment | Place::OutOfBand) => t.get("parameter-new-title"),
            _other => String::new(),
        };
        let (has_s7, s7_required) = s7_field(form.machine.platform());
        let standalone = place.is_page() && key.is_none();
        Self {
            edit: key.is_some(),
            boosted: standalone || !place.is_page(),
            standalone,
            oob: matches!(place, Place::OutOfBand | Place::PageOutOfBand),
            action,
            heading,
            save_id,
            save_label,
            csrf_token,
            units: unit_choices(form),
            has_s7,
            s7_required,
            choices: form.choices.clone(),
            shown,
            errors,
        }
    }
}

/// The fragment `parameter-form.html`: the form and nothing around it.
#[derive(Template)]
#[template(path = "parameter-form.html")]
pub(crate) struct ParameterFormFragment<'a> {
    /// The texts of the UI language.
    pub(crate) t: Texts<'a>,
    /// The form.
    pub(crate) form: ParameterFormView,
}

#[cfg(test)]
mod swap_tests;
