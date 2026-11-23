//! A parameter form with a grid of at least 1 MiB, for the tests of group 4 (design D3, ADR-0223).
//!
//! The module solves one problem: a bound on the memory of an answer proves nothing on a small
//! form. Every test of D3 builds its form here, so each one measures a form of at least 1 MiB.
//! The grid is `ARRAY[1..LEAVES] OF INT` with a value and a limit on each element. This module is
//! test code.

use recipe_domain::plant::{LeafPath, Primitive};

use super::form_view::{DefinitionTexts, ParameterFormFragment, ParameterFormView, Shown};
use super::grid::{GridSource, GridView, LimitTexts, ValueTexts};
use super::summary::ParameterPage;
use super::tree_kit::array_tree;
use crate::error::FormErrors;
use crate::i18n::Lang;
use crate::layout::{Layout, ReadOnlyFlag};
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The size that every form of this module reaches when it renders: 1 MiB.
pub(crate) const MIN_BYTES: usize = 1024 * 1024;
/// The count of elements of the grid. Each element renders to more than 1 KiB.
const LEAVES: i32 = 1_200;
/// The size of a group of the grid, as `form.array_chunk` of the tests.
const CHUNK: usize = 100;

/// Return the text of the type of the grid.
fn type_text() -> String {
    format!("ARRAY[1..{LEAVES}] OF INT")
}

/// Return the path of the element `index`, for example `[7]`.
fn element(index: i32) -> LeafPath {
    LeafPath::parse(&format!("[{index}]")).unwrap()
}

/// Build the grid `ARRAY[1..1200] OF INT` with a value and a limit on each element, in English.
pub(crate) fn big_grid(t: Texts<'_>) -> GridView {
    let tree = array_tree(Primitive::Int, &[(1, LEAVES)]);
    let values: Vec<(LeafPath, String)> = (1..=LEAVES)
        .map(|index| (element(index), index.to_string()))
        .collect();
    let limits: Vec<LimitTexts> = (1..=LEAVES)
        .map(|index| LimitTexts {
            path: element(index),
            min: "0".to_owned(),
            max: "32767".to_owned(),
            max_length: String::new(),
        })
        .collect();
    let source = GridSource {
        tree: &tree,
        values: ValueTexts::from_sent(&values),
        limits,
        toggle: None,
        chunk: CHUNK,
        root_label: "BIG".to_owned(),
    };
    GridView::build(source, t)
}

/// Build the view of the form of a new parameter with the grid of [`big_grid`].
///
/// With `oob`, the form swaps out of band, so it passes the check of `RenderedForm`.
pub(crate) fn big_form(t: Texts<'_>, oob: bool) -> ParameterFormView {
    let shown = Shown {
        texts: DefinitionTexts::default(),
        grid: Some(big_grid(t)),
        value_type: type_text(),
        notice: String::new(),
    };
    ParameterFormView {
        edit: false,
        boosted: true,
        standalone: false,
        oob,
        action: "/recipes/R-100/versions/3/parameters".to_owned(),
        heading: String::new(),
        save_id: "parameter-add-submit",
        save_label: t.get("parameter-add-submit"),
        csrf_token: "c0ffee00c0ffee00".to_owned(),
        units: Vec::new(),
        has_s7: false,
        s7_required: false,
        choices: Vec::new(),
        shown,
        errors: FormErrors::default(),
    }
}

/// Build the fragment `parameter-form.html` with a form of at least 1 MiB.
pub(crate) fn big_fragment(t: Texts<'_>, oob: bool) -> ParameterFormFragment<'_> {
    ParameterFormFragment {
        t,
        form: big_form(t, oob),
    }
}

/// Build the page of a new parameter with a form of at least 1 MiB, in English.
pub(crate) fn big_page(state: &WebState) -> ParameterPage<'_> {
    let t = Texts::new(state.bundles(), Lang::En);
    let layout = Layout::bare(t, Lang::En, "BIG".to_owned(), ReadOnlyFlag::of(state));
    ParameterPage {
        layout,
        t,
        summary: None,
        form: Some(big_form(t, false)),
        notice: None,
        values: None,
        faults: FormErrors::default(),
        translations: None,
        remove: None,
        version_href: "/recipes/R-100/versions/3".to_owned(),
        version_text: "R-100".to_owned(),
    }
}
