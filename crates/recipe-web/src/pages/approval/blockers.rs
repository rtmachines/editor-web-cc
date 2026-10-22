//! The blocker lines of the approval preview (ADR-0049 (c), design D3).
//!
//! A blocker is data of the preview, not an error answer. Each line shows its code and its
//! sentence. A line of a parameter links to the parameter page with the key of `blocker_keys`.

use std::collections::BTreeMap;

use recipe_app::error::AppError;
use recipe_domain::recipe::{ParameterKey, ParameterTag, RecipeError, SnapshotItem};

use crate::error::{FieldError, SHOWN_LINES, WebError, item_lines, line_view, more_text};
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

/// Return one line for each of the first `SHOWN_LINES` items of a failed snapshot check
/// (`E-RCP-020`), in their order.
///
/// A line of the version names no parameter, so it has no link.
fn snapshot_lines(
    items: &[SnapshotItem],
    href: &dyn Fn(Option<&ParameterTag>) -> Option<String>,
) -> Vec<BlockerLine> {
    let first = items.get(..SHOWN_LINES).unwrap_or(items);
    first
        .iter()
        .zip(item_lines(first, &|_tag| None))
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

/// The first lines of the draft blocker and the number of all its lines.
///
/// A draft can cause thousands of lines. This type solves the cost of building them: the page
/// shows `SHOWN_LINES` lines, so only these lines exist (ADR-0049 (c), ADR-0041 (d)).
#[derive(Debug)]
pub(super) struct BlockerLines {
    /// The first `SHOWN_LINES` lines, in the order of the blocker.
    pub(super) shown: Vec<BlockerLine>,
    /// The number of all lines of the blocker, also of the lines that were not built.
    pub(super) total: usize,
}

/// The blocker lines as the template shows them.
pub(super) struct BlockerList {
    /// The views of the shown lines.
    pub(super) views: Vec<BlockerView>,
    /// The sentence `errors-more` that counts the other lines. Empty when the views show all.
    pub(super) more: String,
}

impl BlockerLines {
    /// Return the views of the shown lines in `lang` with the sentence about the other lines.
    ///
    /// One call gives both parts, so the sentence always counts the lines that the views leave out.
    pub(super) fn into_list(self, bundles: &Bundles, lang: Lang) -> BlockerList {
        let views = self
            .shown
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
            .collect();
        BlockerList {
            views,
            more: more_text(self.total, bundles, lang),
        }
    }
}

/// Return the first lines of the draft blocker of the preview (ADR-0049 (c), design D3).
///
/// `base` is the address of the version, for example `/recipes/R-500/versions/2`. A line of a
/// parameter links to `{base}/parameters/{key}` when `keys` holds its tag. Another line has no link.
/// The function builds at most `SHOWN_LINES` lines and takes the total from the source list.
///
/// # Errors
///
/// Returns the error as `WebError::App` when it is not an error of Recipe Management or of the
/// Plant Model.
pub(super) fn blocker_lines(
    blocker: AppError,
    keys: &BTreeMap<ParameterTag, ParameterKey>,
    base: &str,
) -> Result<BlockerLines, WebError> {
    let href = |tag: Option<&ParameterTag>| {
        tag.and_then(|tag| keys.get(tag))
            .map(|key| format!("{base}/parameters/{key}"))
    };
    let recipe = |error: &RecipeError| BlockerLine {
        error: FieldError::recipe(error, None),
        href: href(tag_of(error)),
    };
    match blocker {
        AppError::Recipe(RecipeError::ValidationFailed { items }) => Ok(BlockerLines {
            shown: snapshot_lines(&items, &href),
            total: items.len(),
        }),
        AppError::Recipe(error) => Ok(BlockerLines {
            shown: vec![recipe(&error)],
            total: 1,
        }),
        AppError::Recipes(errors) => Ok(BlockerLines {
            shown: errors.iter().take(SHOWN_LINES).map(recipe).collect(),
            total: errors.len(),
        }),
        AppError::Plant(error) => Ok(BlockerLines {
            shown: vec![BlockerLine {
                error: FieldError::plant(&error, None),
                href: None,
            }],
            total: 1,
        }),
        other => Err(WebError::App(other)),
    }
}

/// Return the sentence of the blocker `E-RCP-015` in `lang` (ADR-0049 (c)).
pub(super) fn approver_message(error: &RecipeError, bundles: &Bundles, lang: Lang) -> String {
    line_view(&FieldError::recipe(error, None), bundles, lang).message
}
