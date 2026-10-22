//! `GET /recipes`: the recipe list with its filters (`api.md` section 2.4, ADR-0043 (a) to (d)).
//!
//! The URL counts pages from 1 and the query counts them from 0. The filters stay in the links to
//! the other pages. `show_deleted=1` runs `ListDeletedRecipes`, which needs `RecipeRestore`.

use askama::Template;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::plant::{ListMachines, ListMachinesInput};
use recipe_app::ports::{MachineFilter, RecipeSummary};
use recipe_app::recipe::{ListDeletedRecipes, ListRecipes, RecipeList, RecipeListInput};
use recipe_domain::plant::MachineCode;
use recipe_domain::recipe::VersionState;
use recipe_domain::text::Text;
use serde::Deserialize;

use super::fields::FieldLines;
use super::{Choice, pager_links};
use crate::error::{FieldReason, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::{Page, flag, machine_code, page as read_page, query_values};
use crate::pages::render::{page_url, render};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The path of the list. The pager and the search form use it.
const LIST_PATH: &str = "/recipes";
/// The first page. A bad page value falls back to it while the other fields are read.
const FIRST_PAGE: Page = Page {
    number: 1,
    index: 0,
};
/// The number of columns of the table.
const COLUMNS: u8 = 5;
/// The number of columns with the column Actions of the restore form.
const COLUMNS_WITH_ACTIONS: u8 = 6;
/// The values of the state filter and the key of the word of each (ADR-0043 (b)).
const STATES: [(&str, VersionState, &str); 3] = [
    ("draft", VersionState::LiveDraft, "status-draft"),
    ("approved", VersionState::Approved, "status-approved"),
    ("retired", VersionState::Retired, "status-retired"),
];

/// The raw query string. A key that is not listed here gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListQuery {
    q: Option<String>,
    machine: Option<String>,
    status: Option<String>,
    page: Option<String>,
    show_deleted: Option<String>,
}

/// The checked query: every value is a value object or a flag. The texts go back into the form.
struct Checked {
    q: String,
    machine: String,
    status: String,
    input: RecipeListInput,
    page: Page,
    show_deleted: bool,
}

/// Read the search text. A text of spaces only is no search text (D20 of `web-plant-model`).
fn search_text(q: &str) -> Result<Option<Text>, WebError> {
    if q.chars().all(|c| c == ' ') {
        return Ok(None);
    }
    Text::parse(q)
        .map(Some)
        .map_err(|_reason| WebError::BadRequest {
            field: "recipe_q",
            reason: FieldReason::SearchText,
        })
}

/// Read the state filter: empty, `draft`, `approved`, or `retired`.
fn state_filter(text: &str) -> Result<Option<VersionState>, WebError> {
    if text.is_empty() {
        return Ok(None);
    }
    STATES
        .iter()
        .find(|(value, _state, _key)| *value == text)
        .map(|(_value, state, _key)| Some(*state))
        .ok_or(WebError::BadRequest {
            field: "recipe_status",
            reason: FieldReason::Status,
        })
}

impl ListQuery {
    /// Check every value. Collect each bad field into one answer (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns `404 E-WEB-005` for a machine that is not a machine code. Returns `422 E-WEB-008`
    /// with one line for each bad search text, state, page, or flag.
    fn check(self) -> Result<Checked, WebError> {
        let mut lines = FieldLines::default();
        let q = self.q.unwrap_or_default();
        let text = lines.take_or(search_text(&q), None)?;
        let machine = self.machine.unwrap_or_default();
        let code: Option<MachineCode> = (!machine.is_empty())
            .then(|| machine_code(&machine))
            .transpose()?;
        let status = self.status.unwrap_or_default();
        let state = lines.take_or(state_filter(&status), None)?;
        let page = lines.take_or(read_page(self.page.as_deref()), FIRST_PAGE)?;
        let show_deleted =
            lines.take_or(flag("show_deleted", self.show_deleted.as_deref()), false)?;
        lines.check()?;
        Ok(Checked {
            input: RecipeListInput {
                text,
                machine: code,
                status: state,
                page: page.index,
            },
            q,
            machine,
            status,
            page,
            show_deleted,
        })
    }
}

/// A link to one version of a recipe in a row of the list.
struct VersionLink {
    href: String,
    number: u32,
}

/// One row of the table.
struct RecipeRow {
    href: String,
    reference: String,
    name: String,
    machine: String,
    deleted: bool,
    approved: Option<VersionLink>,
    draft: Option<VersionLink>,
}

impl From<&RecipeSummary> for RecipeRow {
    fn from(summary: &RecipeSummary) -> Self {
        let href = format!("/recipes/{}", summary.reference);
        let link = |number: u32| VersionLink {
            href: format!("{href}/versions/{number}"),
            number,
        };
        Self {
            reference: summary.reference.to_string(),
            name: summary.name.to_string(),
            machine: summary.machine_code.to_string(),
            deleted: summary.deleted,
            approved: summary.approved.map(link),
            draft: summary.draft.map(link),
            href,
        }
    }
}

/// The page `recipes.html`.
#[derive(Template)]
#[template(path = "recipes.html")]
struct RecipesPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    q: String,
    machine: String,
    status: String,
    show_deleted: bool,
    machines: Vec<Choice>,
    states: Vec<Choice>,
    can_show_deleted: bool,
    /// True when the list shows the deleted recipes to a role that restores them (ADR-0049 (h)).
    can_restore: bool,
    /// The number of table columns. The restore form adds the column Actions.
    columns: u8,
    can_create: bool,
    rows: Vec<RecipeRow>,
    previous_url: String,
    next_url: String,
    current: u32,
    /// True for a page after the last page: no row, and the page number is above 1.
    after_end: bool,
    /// The address of the first page. The page `after_end` links to it.
    first_url: String,
}

/// Return the pairs of the query that the links to the other pages keep. An empty value is left out.
fn kept_pairs(checked: &Checked) -> Vec<(&str, &str)> {
    [
        ("q", checked.q.as_str()),
        ("machine", checked.machine.as_str()),
        ("status", checked.status.as_str()),
        ("show_deleted", if checked.show_deleted { "1" } else { "" }),
    ]
    .into_iter()
    .filter(|(_key, value)| !value.is_empty())
    .collect()
}

/// Return the choices of the state filter: every state, then the three states of ADR-0043 (b).
fn state_choices(t: Texts<'_>) -> Vec<Choice> {
    std::iter::once(Choice::new("", t.get("recipes-any-status")))
        .chain(
            STATES
                .iter()
                .map(|(value, _state, key)| Choice::new(value, t.get(key))),
        )
        .collect()
}

impl<'a> RecipesPage<'a> {
    /// Build the page from the checked query, the machines, and one page of recipes.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        checked: Checked,
        (machines, list): (Vec<Choice>, RecipeList),
    ) -> Result<Self, WebError> {
        let role = request.auth()?.user().role();
        let layout =
            Layout::for_session(t, request, t.get("recipes-title"), Some(NavTarget::Recipes))?;
        let pairs = kept_pairs(&checked);
        let after_end = list.rows.is_empty() && checked.page.index > 0;
        let (previous_url, next_url) =
            pager_links(LIST_PATH, (checked.page, list.has_next, after_end), &pairs);
        let first_url = page_url(LIST_PATH, &pairs, 1);
        let can_show_deleted = Operation::ListDeletedRecipes.requirement().allows(role);
        let can_restore = checked.show_deleted
            && can_show_deleted
            && Operation::RestoreRecipe.requirement().allows(role);
        Ok(Self {
            layout,
            t,
            machines,
            states: state_choices(t),
            can_show_deleted,
            can_restore,
            columns: if can_restore {
                COLUMNS_WITH_ACTIONS
            } else {
                COLUMNS
            },
            can_create: Operation::CreateRecipe.requirement().allows(role),
            rows: list.rows.iter().map(RecipeRow::from).collect(),
            previous_url,
            next_url,
            current: checked.page.number,
            after_end,
            first_url,
            q: checked.q,
            machine: checked.machine,
            status: checked.status,
            show_deleted: checked.show_deleted,
        })
    }
}

/// Return the choices of the machine filter: every machine, then each live machine.
///
/// The query `ListMachines` reads the first page of the live machines. A line holds fewer
/// machines than one page (`page.machines`). A sent code always names a live machine, because
/// `ListRecipes` answers `404 E-WEB-005` for any other code (ADR-0043 (b)).
async fn machine_choices(
    state: &WebState,
    request: &PageRequest,
    t: Texts<'_>,
) -> Result<Vec<Choice>, WebError> {
    let input = ListMachinesInput {
        filter: MachineFilter::default(),
        page: 0,
        archived: None,
    };
    let machines = ListMachines::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let every = Choice::new("", t.get("recipes-any-machine"));
    let choices = std::iter::once(every)
        .chain(machines.rows.iter().map(|machine| {
            let code = machine.code.to_string();
            let text = format!("{code} {}", machine.name);
            Choice { value: code, text }
        }))
        .collect();
    Ok(choices)
}

/// `GET /recipes`: list the recipes, one page at a time. The query checks the permission first.
pub(crate) async fn list(
    State(state): State<WebState>,
    request: PageRequest,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let checked = query_values(query)?.check()?;
    let user = request.auth()?.user();
    let ctx = state.identity().app().clone();
    let input = checked.input.clone();
    let list = if checked.show_deleted {
        ListDeletedRecipes::new(ctx)
            .execute(user, &request.meta(), input)
            .await?
    } else {
        ListRecipes::new(ctx)
            .execute(user, &request.meta(), input)
            .await?
    };
    let t = Texts::new(state.bundles(), request.lang());
    let machines = machine_choices(&state, &request, t).await?;
    let page = RecipesPage::new(t, &request, checked, (machines, list))?;
    Ok(render(&page, request.request_id())?.into_response())
}
