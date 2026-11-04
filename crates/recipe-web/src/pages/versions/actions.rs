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
use recipe_domain::recipe::{VersionHeader, VersionState};
use serde::Deserialize;

use super::page::version_answer;
use super::path_version;
use crate::error::WebError;
use crate::pages::draft_reply::{conflict, with_push_url, with_row_version};
use crate::pages::path_values::page as read_page;
use crate::pages::recipes::Choice;
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

/// The compare form of the version page (ADR-0108 (d), design D3).
///
/// The type solves one problem: the user must pick any two versions of the recipe. The two links
/// of ADR-0050 (l) cover only the Approved version and the base version.
pub(crate) struct CompareForm {
    /// The address of the compare route, for example `/recipes/R-100/compare`.
    pub(crate) action: String,
    /// Each version number of the recipe as an option, the highest first. Both lists use it.
    pub(crate) numbers: Vec<Choice>,
    /// The number that the list `n` selects: the shown version.
    pub(crate) shown: String,
    /// The number that the list `m` selects: the base version or the highest other version.
    pub(crate) base: String,
}

/// The export link of an Approved or a Retired version (ADR-0052 (b), (c), and (e)).
///
/// The link is a plain download link and not a boosted one. The page shows the notice, when the
/// link has one, before it.
pub(crate) struct ExportLink {
    /// The address of the export route, for example `/export/v1/recipes/R-100/approved`.
    pub(crate) href: String,
    /// The text of the link, which names the version and the file type.
    pub(crate) text: String,
    /// The notice before the link, or `None`. The link then names the notice in `aria-describedby`.
    pub(crate) notice: Option<ExportNotice>,
}

/// The notice in the export area of a version that the file marks as not loadable (ADR-0119 (l)).
///
/// The type solves one problem: a Retired version and an Approved version with a rule fault show
/// the same markup with another severity and another sentence. The severity comes from code.
pub(crate) struct ExportNotice {
    /// The severity of the alert: `info` for a Retired version, `warning` for a rule fault.
    pub(crate) severity: &'static str,
    /// The sentence of the notice, in the language of the page.
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
    /// The compare form, or `None` for a recipe with one version.
    pub(crate) compare_form: Option<CompareForm>,
    /// The export link of an Approved or a Retired version of a live recipe, for a role that the
    /// matrix row of the export allows. It stays in read-only mode (A24).
    pub(crate) export: Option<ExportLink>,
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

/// Build the compare form of `view`, or `None` when the recipe has fewer than two versions.
///
/// The function solves one problem: the lists must offer each version and must start with a useful
/// pair. The list `m` selects the base version when it exists and differs from the shown version,
/// else the highest other version. A deleted recipe keeps the form, because it only reads.
fn compare_form(view: &VersionView) -> Option<CompareForm> {
    let number = view.detail.number;
    let mut numbers: Vec<u32> = view
        .recipe
        .versions()
        .iter()
        .map(VersionHeader::number)
        .collect();
    numbers.sort_unstable_by(|a, b| b.cmp(a));
    let highest_other = numbers.iter().copied().find(|other| *other != number)?;
    let base = view
        .detail
        .based_on
        .filter(|base| *base != number && numbers.contains(base))
        .unwrap_or(highest_other);
    Some(CompareForm {
        action: format!("/recipes/{}/compare", view.recipe.reference()),
        numbers: numbers
            .iter()
            .map(|n| Choice::new(&n.to_string(), n.to_string()))
            .collect(),
        shown: number.to_string(),
        base: base.to_string(),
    })
}

/// Build the export link of `view`, or `None` when the page shows none.
///
/// The function solves one problem: the page must not show an export that the route refuses. A
/// Draft, a discarded draft, and each version of a deleted recipe get no link. The request needs
/// the operation of the route that the link opens, as the matrix of `api.md` section 3.3 says.
/// An export stays open in read-only mode (A24, ADR-0113).
fn export_link(
    t: Texts<'_>,
    request: &PageRequest,
    view: &VersionView,
) -> Result<Option<ExportLink>, WebError> {
    let number = view.detail.number;
    let reference = view.recipe.reference();
    let (operation, href, notice) = match view.detail.state {
        VersionState::Approved => (
            Operation::ExportApprovedVersion,
            format!("/export/v1/recipes/{reference}/approved"),
            rule_notice(t, view),
        ),
        VersionState::Retired => (
            Operation::ExportVersionByNumber,
            format!("/export/v1/recipes/{reference}/versions/{number}"),
            Some(ExportNotice {
                severity: "info",
                text: t.get("version-export-retired-notice"),
            }),
        ),
        VersionState::LiveDraft | VersionState::Discarded => return Ok(None),
    };
    let shown = !view.recipe.is_deleted() && request.may_run(operation)?;
    Ok(shown.then(|| ExportLink {
        href,
        text: t.with_arg("version-export-link", "number", number),
        notice,
    }))
}

/// Build the notice of the rule faults of `view`, or `None` when it has none (ADR-0119 (l)).
///
/// The function solves one problem: the export of an affected Approved version sets `loadable` to
/// `false`, so the page must tell the reader why before the link.
fn rule_notice(t: Texts<'_>, view: &VersionView) -> Option<ExportNotice> {
    if view.rule_codes.is_empty() {
        return None;
    }
    let codes: Vec<&str> = view.rule_codes.iter().map(|code| code.as_str()).collect();
    Some(ExportNotice {
        severity: "warning",
        text: t.with_arg("version-export-rule-notice", "codes", codes.join(", ")),
    })
}

/// Build the action area of `view` for the session of `request`.
///
/// A deleted recipe gets no action, only the compare links and the history link (ADR-0043 (e)).
/// An action shows only when `PageRequest::may_run` permits its command, so read-only mode hides
/// the approve link and the danger forms (ADR-0113).
///
/// # Errors
///
/// Returns `E-WEB-004` when the request has no session.
pub(crate) fn version_actions(
    t: Texts<'_>,
    request: &PageRequest,
    view: &VersionView,
) -> Result<VersionActions, WebError> {
    let detail = &view.detail;
    let reference = view.recipe.reference();
    let base = format!("/recipes/{reference}/versions/{}", detail.number);
    let live = !view.recipe.is_deleted();
    let allows = |operation: Operation| Ok::<bool, WebError>(live && request.may_run(operation)?);
    let draft = detail.state == VersionState::LiveDraft;
    let approve = (draft && allows(Operation::ApproveVersion)?).then(|| ActionLink {
        href: format!("{base}/approve"),
        text: t.with_arg("version-approve-link", "number", detail.number),
    });
    let number = detail.number;
    let danger = match detail.state {
        VersionState::LiveDraft if allows(Operation::DiscardDraft)? => Some(danger_form(
            t,
            DISCARD_KEYS,
            (format!("{base}/discard"), number),
        )),
        VersionState::Approved if allows(Operation::RetireVersion)? => Some(danger_form(
            t,
            RETIRE_KEYS,
            (format!("{base}/retire"), number),
        )),
        _other => None,
    };
    Ok(VersionActions {
        approve,
        compare: compare_links(t, view),
        compare_form: compare_form(view),
        export: export_link(t, request, view)?,
        danger,
        history_href: format!("/recipes/{reference}/history"),
        row_version: detail.row_version,
    })
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
