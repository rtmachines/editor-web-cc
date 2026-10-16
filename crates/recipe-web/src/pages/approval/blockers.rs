//! The blocker lines of the approval preview (ADR-0049 (c), design D3).
//!
//! A blocker is data of the preview, not an error answer. Each line shows its code and its
//! sentence. A line of a parameter links to the parameter page with the key of `blocker_keys`.

use std::collections::BTreeMap;

use recipe_app::error::AppError;
use recipe_domain::recipe::{ParameterKey, ParameterTag, RecipeError, SnapshotItem};

use crate::error::{FieldError, WebError, item_lines, line_view};
use crate::i18n::{Bundles, Lang};

/// One line of a blocker with the address of its parameter page.
#[derive(Debug)]
pub(super) struct BlockerLine {
    /// The line with its code and its arguments.
    pub(super) error: FieldError,
    /// The address of the parameter page, or `None` for a line without a parameter.
    pub(super) href: Option<String>,
}

/// One blocker line as the template shows it. The macro `line_text` reads the first three fields.
pub(super) struct BlockerView {
    /// The code `E-<CTX>-<nnn>` of the line.
    pub(super) code: &'static str,
    /// The data that the line is about, such as a tag. Empty when the message names it.
    pub(super) subject: String,
    /// One sentence in the UI language.
    pub(super) message: String,
    /// The address of the parameter page. Empty when the line has no link.
    pub(super) href: String,
}

/// Return the tag that an error of `check_for_approval` names first, as `GetApprovalPreview` does.
const fn tag_of(error: &RecipeError) -> Option<&ParameterTag> {
    match error {
        RecipeError::S7AddressOverlap { tag, .. }
        | RecipeError::LimitMissing { tag, .. }
        | RecipeError::S7AddressMisaligned { tag, .. }
        | RecipeError::S7AddressRequired { tag, .. }
        | RecipeError::S7AddressNotAllowed { tag, .. }
        | RecipeError::S7AddressWidthMismatch { tag, .. }
        | RecipeError::TypeRevisionOutdated { tag, .. } => Some(tag),
        _ => None,
    }
}

/// Return one line for each item of a failed snapshot check (`E-RCP-020`), in their order.
///
/// A line of the version names no parameter, so it has no link.
fn snapshot_lines(
    items: &[SnapshotItem],
    href: &dyn Fn(Option<&ParameterTag>) -> Option<String>,
) -> Vec<BlockerLine> {
    items
        .iter()
        .zip(item_lines(items, &|_tag| None))
        .map(|(item, error)| {
            let tag = match item {
                SnapshotItem::Parameter { tag, .. } | SnapshotItem::UnknownUnit { tag, .. } => {
                    Some(tag)
                }
                SnapshotItem::Version { .. } => None,
            };
            BlockerLine {
                error,
                href: href(tag),
            }
        })
        .collect()
}

/// Return the lines of the draft blocker of the preview (ADR-0049 (c), design D3).
///
/// `base` is the address of the version, for example `/recipes/R-500/versions/2`. A line of a
/// parameter links to `{base}/parameters/{key}` when `keys` holds its tag. Another line has no link.
///
/// # Errors
///
/// Returns the error as `WebError::App` when it is not an error of Recipe Management or of the
/// Plant Model.
pub(super) fn blocker_lines(
    blocker: AppError,
    keys: &BTreeMap<ParameterTag, ParameterKey>,
    base: &str,
) -> Result<Vec<BlockerLine>, WebError> {
    if !matches!(
        blocker,
        AppError::Recipe(_) | AppError::Recipes(_) | AppError::Plant(_)
    ) {
        return Err(WebError::App(blocker));
    }
    let href = |tag: Option<&ParameterTag>| {
        tag.and_then(|tag| keys.get(tag))
            .map(|key| format!("{base}/parameters/{key}"))
    };
    let recipe = |error: &RecipeError| BlockerLine {
        error: FieldError::recipe(error, None),
        href: href(tag_of(error)),
    };
    Ok(match &blocker {
        AppError::Recipe(RecipeError::ValidationFailed { items }) => snapshot_lines(items, &href),
        AppError::Recipe(error) => vec![recipe(error)],
        AppError::Recipes(errors) => errors.iter().map(recipe).collect(),
        AppError::Plant(error) => vec![BlockerLine {
            error: FieldError::plant(error, None),
            href: None,
        }],
        _other => Vec::new(),
    })
}

/// Return the view of each line in `lang`, in the order of the lines.
pub(super) fn blocker_views(
    lines: &[BlockerLine],
    bundles: &Bundles,
    lang: Lang,
) -> Vec<BlockerView> {
    lines
        .iter()
        .map(|line| {
            let view = line_view(&line.error, bundles, lang);
            BlockerView {
                code: view.code,
                subject: view.subject,
                message: view.message,
                href: line.href.clone().unwrap_or_default(),
            }
        })
        .collect()
}

/// Return the sentence of the blocker `E-RCP-015` in `lang` (ADR-0049 (c)).
pub(super) fn approver_message(error: &RecipeError, bundles: &Bundles, lang: Lang) -> String {
    line_view(&FieldError::recipe(error, None), bundles, lang).message
}
