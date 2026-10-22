//! The action area of the version page, `POST /recipes/{ref}/versions/{n}/retire`, and
//! `POST /recipes/{ref}/versions/{n}/discard` (`api.md` section 2.5, ADR-0049 (f), (i), (j), and
//! (k), ADR-0051 (g), design D7 to D9).
//!
//! The action area holds only the actions that the matrix row of each command allows for the role
//! and that the state of the version permits. Both forms send `csrf_token` and the `row_version` of
//! the version. A retire success answers `200` with the version page, its toast, and
//! `HX-Push-Url`. A discard success answers `303` to the recipe page. A stale `row_version` gives the
//! conflict of design D10.

use axum::extract::rejection::{FormRejection, PathRejection};
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::recipe::{
    DiscardDraft, DiscardDraftInput, RetireVersion, RetireVersionInput, VersionView,
};
use recipe_domain::identity::Role;
use recipe_domain::recipe::{VersionHeader, VersionState};
use serde::Deserialize;

use super::page::version_answer;
use super::path_version;
use crate::error::WebError;
use crate::pages::draft_reply::{conflict, with_push_url, with_row_version};
use crate::pages::path_values::page as read_page;
use crate::pages::render::see_other_path;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The keys of the summary, the notice, and the button of the retire action (design D8).
const RETIRE_KEYS: [&str; 3] = ["retire-summary", "retire-notice", "retire-button"];
/// The keys of the summary, the notice, and the button of the discard action (design D8).
const DISCARD_KEYS: [&str; 3] = ["discard-summary", "discard-notice", "discard-button"];

/// A link of the action area: the address and the text.
pub(crate) struct ActionLink {
    /// The address, for example `/recipes/R-100/versions/3/approve`.
    pub(crate) href: String,
    /// The text of the link from the bundles.
    pub(crate) text: String,
}

/// The danger action of a version: the retire form or the discard form (ADR-0049 (f)).
pub(crate) struct DangerForm {
    /// The text of the `summary`.
    pub(crate) summary: String,
    /// The notice sentence that states the effect.
    pub(crate) notice: String,
    /// The text of the button, which names the version.
    pub(crate) button: String,
    /// The address of the route, for example `/recipes/R-100/versions/2/retire`.
    pub(crate) action: String,
}

/// The action area of the version page (ADR-0049 (i), ADR-0051 (g)).
///
/// The type solves one problem: the template shows an action only when the role and the state
/// permit it, and it does not copy the rules of the matrix.
pub(crate) struct VersionActions {
    /// The link to the approval preview, for a live draft and a role with `VersionApprove`.
    pub(crate) approve: Option<ActionLink>,
    /// The links to the diff with the current Approved version and with the base version.
    ///
    /// The Approved link stands first. A link shows only when its version exists and has another
    /// number than the shown version. One version gets one link (design D7).
    pub(crate) compare: Vec<ActionLink>,
    /// The retire or the discard action, or `None`. A page holds at most one danger action.
    pub(crate) danger: Option<DangerForm>,
    /// The address of the history of the recipe.
    pub(crate) history_href: String,
    /// The `row_version` of the version, for the hidden field of the danger form.
    pub(crate) row_version: i64,
}

/// Build the danger form with the text keys `keys` for the version `number` and the route `action`.
fn danger_form(
    t: Texts<'_>,
    [summary, notice, button]: [&str; 3],
    (action, number): (String, u32),
) -> DangerForm {
    DangerForm {
        summary: t.get(summary),
        notice: t.get(notice),
        button: t.with_arg(button, "number", number),
        action,
    }
}

/// Build the compare links of `view`: the Approved link first, then the base link.
///
/// The function solves one problem: the page must not link a version to itself, and it must not
/// show two links to one version. The lower number is the old side of the diff (ADR-0050 (i)).
/// A deleted recipe keeps the links, because they are reads.
fn compare_links(t: Texts<'_>, view: &VersionView) -> Vec<ActionLink> {
    let number = view.detail.number;
    let reference = view.recipe.reference();
    let approved = view.recipe.approved_version().map(VersionHeader::number);
    let base = view.detail.based_on.filter(|base| Some(*base) != approved);
    [
        (approved, "version-compare-approved"),
        (base, "version-compare-base"),
    ]
    .into_iter()
    .filter_map(|(other, key)| other.map(|other| (other, key)))
    .filter(|(other, _key)| *other != number)
    .map(|(other, key)| ActionLink {
        href: format!(
            "/recipes/{reference}/versions/{}/diff/{}",
            number.max(other),
            number.min(other)
        ),
        text: t.with_arg(key, "number", other),
    })
    .collect()
}

/// Build the action area of `view` for `role`.
///
/// A deleted recipe gets no action, only the compare links and the history link (ADR-0043 (e)).
pub(crate) fn version_actions(t: Texts<'_>, role: Role, view: &VersionView) -> VersionActions {
    let detail = &view.detail;
    let reference = view.recipe.reference();
    let base = format!("/recipes/{reference}/versions/{}", detail.number);
    let live = !view.recipe.is_deleted();
    let allows = |operation: Operation| live && operation.requirement().allows(role);
    let draft = detail.state == VersionState::LiveDraft;
    let approve = (draft && allows(Operation::ApproveVersion)).then(|| ActionLink {
        href: format!("{base}/approve"),
        text: t.with_arg("version-approve-link", "number", detail.number),
    });
    let number = detail.number;
    let danger = match detail.state {
        VersionState::LiveDraft if allows(Operation::DiscardDraft) => Some(danger_form(
            t,
            DISCARD_KEYS,
            (format!("{base}/discard"), number),
        )),
        VersionState::Approved if allows(Operation::RetireVersion) => Some(danger_form(
            t,
            RETIRE_KEYS,
            (format!("{base}/retire"), number),
        )),
        _other => None,
    };
    VersionActions {
        approve,
        compare: compare_links(t, view),
        danger,
        history_href: format!("/recipes/{reference}/history"),
        row_version: detail.row_version,
    }
}

/// The fields of the retire form and the discard form. An unknown field gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionedFields {
    /// The `row_version` of the version that the page showed (ADR-0008).
    row_version: i64,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

/// `POST /recipes/{ref}/versions/{n}/retire`: retire the version and show its page with a toast.
///
/// The command commits, and then the read of the version page can fail (design D7).
/// The route then keeps the error answer of the read.
/// A repeat of the retire with the stale `row_version` gives `409 E-WEB-001`.
pub(crate) async fn retire(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<VersionedFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let Form(fields) = form?;
    let input = RetireVersionInput {
        reference: reference.clone(),
        number,
        expected_row_version: fields.row_version,
    };
    let result = RetireVersion::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = result {
        return Err(conflict(&state, &request, error).await);
    }
    let request_id = request.request_id();
    let t = Texts::new(state.bundles(), request.lang());
    let toast = t.with_arg("toast-version-retired", "number", number);
    let target = (reference.clone(), number);
    let page = (None, read_page(None)?);
    let answer = version_answer(&state, &request, target, page, Some(toast)).await?;
    let response = with_row_version(answer.html.into_response(), answer.row_version, request_id)?;
    let href = format!("/recipes/{reference}/versions/{number}");
    with_push_url(response, &href, request_id)
}

/// `POST /recipes/{ref}/versions/{n}/discard`: discard the draft and redirect to the recipe page.
pub(crate) async fn discard(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    form: Result<Form<VersionedFields>, FormRejection>,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let Form(fields) = form?;
    let input = DiscardDraftInput {
        reference: reference.clone(),
        number,
        expected_row_version: fields.row_version,
    };
    let result = DiscardDraft::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await;
    if let Err(error) = result {
        return Err(conflict(&state, &request, error).await);
    }
    see_other_path(&format!("/recipes/{reference}"), request.request_id(), [])
}
