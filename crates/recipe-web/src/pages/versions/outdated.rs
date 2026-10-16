//! The list of the outdated parameters of a draft (ADR-0043 (j) and (l), design D7).
//!
//! The version page shows the list in `#outdated-list`. The answer of a rebind swaps the list out
//! of band, because the rebind changes it.

use fluent::FluentArgs;
use recipe_app::recipe::{OutdatedParameter, VersionView};

use super::rebind_form::{RebindForm, rebind_forms, shows_forms};
use crate::error::WebError;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;

/// Build the list of a version page, with the rebind forms when the page shows them.
///
/// The forms come from `view` alone, so the function runs no query (design D4).
///
/// # Errors
///
/// Returns `E-WEB-004` without a session.
pub(crate) fn version_list(
    request: &PageRequest,
    view: &VersionView,
    t: Texts<'_>,
) -> Result<OutdatedList, WebError> {
    let base = format!(
        "/recipes/{}/versions/{}",
        view.recipe.reference(),
        view.detail.number
    );
    let list = OutdatedList::new(&base, &view.outdated).with_hints(t, &view.outdated);
    if !shows_forms(request, view)? {
        return Ok(list);
    }
    Ok(list.with_forms(rebind_forms(t, request, view)))
}

/// One row of the list: one parameter whose type uses an older struct revision.
pub(crate) struct OutdatedRow {
    /// The address of the parameter page.
    pub(crate) href: String,
    /// The tag of the parameter.
    pub(crate) tag: String,
    /// The path of the unit that holds the parameter.
    pub(crate) unit: String,
    /// The outdated struct revision, for example `ST_Zone@2`.
    pub(crate) type_text: String,
    /// The text `outdated-nested-hint` when another struct holds the outdated struct.
    ///
    /// A rebind does not move such a parameter, so the hint names the struct that needs a new
    /// revision first.
    pub(crate) hint: Option<String>,
}

/// Return the text `outdated-nested-hint` of `item`, or `None` when no struct holds its type.
fn nested_hint(t: Texts<'_>, item: &OutdatedParameter) -> Option<String> {
    let holder = item.holder.as_ref()?;
    let mut args = FluentArgs::new();
    args.set("tag", item.tag.to_string());
    args.set("type", item.type_name.to_string());
    args.set("holder", holder.to_string());
    args.set("newest", item.newest.to_string());
    Some(t.with("outdated-nested-hint", &args))
}

/// The data of `#outdated-list`: the rows of the outdated parameters and the rebind forms.
///
/// The type solves one problem: the version page and the answer of a rebind show one list. The
/// page renders it in place, and the answer swaps it out of band.
pub(crate) struct OutdatedList {
    /// True when the answer swaps the list out of band.
    pub(crate) oob: bool,
    /// One row per outdated parameter, in the order unit path, ordinal, and tag.
    pub(crate) rows: Vec<OutdatedRow>,
    /// The rebind forms. Only a live draft and a role with `ParameterEdit` get them.
    pub(crate) forms: Vec<RebindForm>,
}

impl OutdatedList {
    /// Return the list with the rebind forms `forms`.
    pub(crate) fn with_forms(self, forms: Vec<RebindForm>) -> Self {
        Self { forms, ..self }
    }

    /// Return the list with the text `outdated-nested-hint` on each row of a nested struct.
    ///
    /// `outdated` is the slice that built the rows, so row `n` belongs to item `n`.
    pub(crate) fn with_hints(self, t: Texts<'_>, outdated: &[OutdatedParameter]) -> Self {
        let rows = self
            .rows
            .into_iter()
            .zip(outdated)
            .map(|(row, item)| OutdatedRow {
                hint: nested_hint(t, item),
                ..row
            })
            .collect();
        Self { rows, ..self }
    }

    /// Return the list that an answer swaps out of band.
    pub(crate) fn out_of_band(self) -> Self {
        Self { oob: true, ..self }
    }

    /// Build the list of the page without a form. `base` is the address of the version page.
    pub(crate) fn new(base: &str, outdated: &[OutdatedParameter]) -> Self {
        Self {
            oob: false,
            forms: Vec::new(),
            rows: outdated
                .iter()
                .map(|item| OutdatedRow {
                    href: format!("{base}/parameters/{}", item.key),
                    tag: item.tag.to_string(),
                    unit: item.unit_path.to_string(),
                    type_text: format!("{}@{}", item.type_name, item.current),
                    hint: None,
                })
                .collect(),
        }
    }
}
