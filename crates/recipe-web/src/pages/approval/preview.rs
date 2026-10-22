//! `GET /recipes/{ref}/versions/{n}/approve`: the approval preview (ADR-0049 (a) to (d),
//! ADR-0050 (h), design D3).
//!
//! The page shows the header, the blockers, the notice of `E-RCP-015`, the notice of a changed
//! draft, the summary and the sections of the diff, and the approve form when no blocker exists.
//! The result of the query holds the permit of the diff gate. The handler drops it after the page
//! renders (ADR-0050 (g)).

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::error::AppError;
use recipe_app::recipe::{ApprovalPreview, ApprovalPreviewInput, DiffChoice, GetApprovalPreview};
use recipe_domain::recipe::DiffKind;
use serde::Deserialize;

use super::blockers::{BlockerLines, BlockerList, BlockerView, approver_message, blocker_lines};
use crate::error::{FieldReason, FormErrors, WebError};
use crate::i18n::Bundles;
use crate::layout::{Layout, NavTarget};
use crate::pages::diff_view::{DiffLinks, DiffView, kind as read_kind};
use crate::pages::path_values::{Page, page as read_page, positive_number, query_values};
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::pages::versions::path_version;
use crate::state::WebState;

/// The query string of the preview. A key other than `kind`, `page`, and `rv` gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviewQuery {
    kind: Option<String>,
    page: Option<String>,
    rv: Option<String>,
}

/// The checked query of the preview.
struct Review {
    /// The section of the diff, or `None` for the page that fills the sections in their order.
    kind: Option<DiffKind>,
    /// The page of the diff.
    page: Page,
    /// The row version that the review started with, or `None` on the first page.
    rv: Option<i64>,
}

impl PreviewQuery {
    /// Read the section, the page, and the review state `rv`.
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` for a bad `kind`, `page`, or `rv`.
    fn check(self) -> Result<Review, WebError> {
        Ok(Review {
            kind: read_kind(self.kind.as_deref())?,
            page: read_page(self.page.as_deref())?,
            rv: review_state(self.rv.as_deref())?,
        })
    }
}

/// Read the query value `rv`: a positive whole number with no sign and no zero in front.
///
/// # Errors
///
/// Returns `422 E-WEB-008` with the field `rv` and `FieldReason::Query` for any other text.
fn review_state(text: Option<&str>) -> Result<Option<i64>, WebError> {
    let Some(text) = text else {
        return Ok(None);
    };
    positive_number(text)
        .and_then(|number| i64::try_from(number.get()).ok())
        .map(Some)
        .ok_or(WebError::BadRequest {
            field: "rv",
            reason: FieldReason::Query,
        })
}

/// The notice of the blocker `E-RCP-015` (ADR-0049 (c)).
struct ApproverNotice {
    /// The code `E-RCP-015`.
    code: &'static str,
    /// The sentence of the code in the UI language.
    message: String,
}

/// The approve form (ADR-0049 (b), design D3).
///
/// The form holds the notice of the retired version and the outdated parameters, so a page without
/// a form shows neither of them.
pub(super) struct ApproveForm {
    /// The address of the form: the address of the preview.
    action: String,
    /// The session token of the request.
    csrf_token: String,
    /// The row version of the review (ADR-0050 (h)), or the sent row version of an error answer.
    row_version: i64,
    /// The text "Approve version {n}".
    button: String,
    /// The notice about the version that the approval retires.
    retire_notice: String,
    /// The legend of the fieldset of the outdated parameters.
    outdated_legend: String,
    /// The outdated parameters with the links to their pages (A29).
    outdated: Vec<OutdatedLink>,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    /// The error lines of the inputs of the form.
    errors: FormErrors,
}

impl ApproveForm {
    /// Build the empty form of `preview` with `row_version` in its hidden field.
    fn new(t: Texts<'_>, preview: &ApprovalPreview, row_version: i64, csrf_token: String) -> Self {
        let addresses = Addresses::of(preview);
        Self {
            action: addresses.path,
            csrf_token,
            row_version,
            button: t.with_arg("approve-button", "number", preview.number),
            retire_notice: retire_notice(t, preview),
            outdated_legend: t.with_arg("approve-outdated-legend", "count", preview.outdated.len()),
            outdated: outdated_links(preview, &addresses.base),
            oob: false,
            errors: FormErrors::default(),
        }
    }

    /// Build the form only when the preview shows it (design D3).
    ///
    /// The preview shows the form when `may_approve` is true and `row_version` equals the row
    /// version of the preview. Otherwise the function returns `None`.
    pub(super) fn offered(
        t: Texts<'_>,
        preview: &ApprovalPreview,
        row_version: i64,
        csrf_token: String,
    ) -> Option<Self> {
        (preview.may_approve && row_version == preview.row_version)
            .then(|| Self::new(t, preview, row_version, csrf_token))
    }

    /// Return the form with the error lines, swapped out of band.
    pub(super) fn refused(self, errors: FormErrors) -> Self {
        Self {
            oob: true,
            errors,
            ..self
        }
    }
}

/// The block `form` of `approve.html` alone: the form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "approve.html", block = "form")]
pub(super) struct ApproveFormBlock<'a> {
    /// The texts in the language of the request.
    pub(super) t: Texts<'a>,
    /// The form with the error lines. The block shows nothing for `None`.
    pub(super) form: Option<ApproveForm>,
}

/// One outdated parameter with the link to its page (A29).
struct OutdatedLink {
    /// The address of the parameter page.
    href: String,
    /// The tag of the parameter.
    tag: String,
    /// The path of the unit of the parameter.
    unit: String,
}

/// The page `approve.html`.
#[derive(Template)]
#[template(path = "approve.html")]
struct ApprovePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    blockers: Vec<BlockerView>,
    blockers_more: String,
    approver: Option<ApproverNotice>,
    changed: Option<String>,
    diff: Option<DiffView>,
    jump: &'static str,
    form: Option<ApproveForm>,
    back_href: String,
    back_text: String,
}

/// The addresses of the preview of one draft.
struct Addresses {
    /// The address of the version, for example `/recipes/R-500/versions/2`.
    base: String,
    /// The address of the versions of the recipe.
    versions: String,
    /// The address of the preview without a query.
    path: String,
}

impl Addresses {
    /// Build the addresses of the draft of `preview`.
    fn of(preview: &ApprovalPreview) -> Self {
        let versions = format!("/recipes/{}/versions", preview.recipe.reference());
        let base = format!("{versions}/{}", preview.number);
        Self {
            path: format!("{base}/approve"),
            base,
            versions,
        }
    }
}

/// Return the notice text about the version that the approval retires (ADR-0049 (a)).
fn retire_notice(t: Texts<'_>, preview: &ApprovalPreview) -> String {
    let mut args = fluent::FluentArgs::new();
    args.set("number", preview.number.to_string());
    args.set("reference", preview.recipe.reference().to_string());
    match preview.retired {
        Some(retired) => {
            args.set("retired", retired.to_string());
            t.with("approve-retires-notice", &args)
        }
        None => t.with("approve-first-notice", &args),
    }
}

/// Return the outdated parameters with the links to their pages.
fn outdated_links(preview: &ApprovalPreview, base: &str) -> Vec<OutdatedLink> {
    preview
        .outdated
        .iter()
        .map(|item| OutdatedLink {
            href: format!("{base}/parameters/{}", item.key),
            tag: item.tag.to_string(),
            unit: item.unit_path.to_string(),
        })
        .collect()
}

/// The blockers of the preview as the page shows them.
struct Blockers {
    /// The first lines of the draft blocker with the sentence that counts the other lines.
    list: BlockerList,
    approver: Option<ApproverNotice>,
}

/// Return the blocker lines and the notice of `E-RCP-015` in the language of `request`.
///
/// # Errors
///
/// Returns the draft blocker as `WebError::App` when it is not an error of a draft check.
fn blockers_of(
    preview: &ApprovalPreview,
    draft_blocker: Option<AppError>,
    (bundles, request, base): (&Bundles, &PageRequest, &str),
) -> Result<Blockers, WebError> {
    let lang = request.lang();
    let lines = match draft_blocker {
        Some(error) => blocker_lines(error, &preview.blocker_keys, base)?,
        None => BlockerLines {
            shown: Vec::new(),
            total: 0,
        },
    };
    let approver = preview
        .approver_blocker
        .as_ref()
        .map(|error| ApproverNotice {
            code: error.code().as_str(),
            message: approver_message(error, bundles, lang),
        });
    Ok(Blockers {
        list: lines.into_list(bundles, lang),
        approver,
    })
}

impl<'a> ApprovePage<'a> {
    /// Build the page from the preview and the review state.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        preview: &ApprovalPreview,
        blockers: Blockers,
        review: &Review,
    ) -> Result<Self, WebError> {
        let addresses = Addresses::of(preview);
        let rv = review.rv.unwrap_or(preview.row_version);
        let changed = rv != preview.row_version;
        let rv_text = rv.to_string();
        let keep = [("rv", rv_text.as_str())];
        let links = DiffLinks {
            path: &addresses.path,
            keep: &keep,
            versions: &addresses.versions,
            old: preview.retired,
            new: preview.number,
        };
        let form = ApproveForm::offered(t, preview, rv, request.csrf_token());
        let mut args = fluent::FluentArgs::new();
        args.set("number", preview.number.to_string());
        args.set("reference", preview.recipe.reference().to_string());
        let title = t.with("approve-title", &args);
        Ok(Self {
            layout: Layout::for_session(t, request, title, Some(NavTarget::Recipes))?,
            t,
            blockers: blockers.list.views,
            blockers_more: blockers.list.more,
            approver: blockers.approver,
            changed: changed.then(|| addresses.path.clone()),
            diff: preview
                .diff
                .as_ref()
                .map(|page| DiffView::new(t, page, &links)),
            jump: if form.is_some() { "#approve-form" } else { "" },
            form,
            back_text: t.with_arg("approve-back", "number", preview.number),
            back_href: addresses.base,
        })
    }
}

/// `GET /recipes/{ref}/versions/{n}/approve`: show the review step of the approval.
pub(crate) async fn preview(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<PreviewQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let (reference, number) = path_version(path)?;
    let review = query_values(query)?.check()?;
    let user = request.auth()?.user();
    let input = ApprovalPreviewInput {
        reference,
        number,
        choice: DiffChoice {
            kind: review.kind,
            page: review.page.index,
        },
    };
    let mut preview = GetApprovalPreview::new(state.identity().app().clone())
        .execute(user, &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let base = Addresses::of(&preview).base;
    let draft_blocker = preview.draft_blocker.take();
    let blockers = blockers_of(&preview, draft_blocker, (state.bundles(), &request, &base))?;
    let page = ApprovePage::new(t, &request, &preview, blockers, &review)?;
    let html = render(&page, request.request_id())?;
    // The permit of the diff gate lives until the page is text (ADR-0050 (g)).
    drop(page);
    drop(preview);
    Ok(html.into_response())
}

#[cfg(test)]
mod review_state_tests;
