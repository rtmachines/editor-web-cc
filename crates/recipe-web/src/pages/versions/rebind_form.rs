//! The rebind forms of a draft (ADR-0043 (l), ADR-0039 (f), design D4, D6, and D7).
//!
//! The draft page shows one form per outdated struct revision that the rebind can move. The form
//! holds the two type identifiers and one input per new member leaf of each moved parameter.
//! `GetVersion` gives the leaves, so the forms run no query of their own (design D4).
//!
//! The form `id` is `rebind-<from>`, so it stays when another group sorts first. The input `id` is
//! `new-<k>-<n>`. `k` counts the moved parameters of the page in the order of the list, so two
//! forms never share an `id`. `n` counts the new leaves of one parameter.

use std::collections::BTreeMap;

use fluent::FluentArgs;
use recipe_app::auth::Operation;
use recipe_app::recipe::{OutdatedParameter, VersionView};
use recipe_domain::plant::{LeafClass, LeafPath, PlcTypeId, TypeTree};
use recipe_domain::recipe::{ParameterKey, RebindLeaves, RecipeError, VersionState};

use crate::error::{FormErrors, WebError};
use crate::pages::recipes::Choice;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;

/// One input of a new member leaf (design D6).
pub(crate) struct MemberInput {
    /// The `id` of the input: `new-<k>-<n>`.
    pub(crate) id: String,
    /// The field name: `new_member_values[<key>]<leaf path>` (ADR-0039 (f)).
    pub(crate) name: String,
    /// The label `field-new-member` with the leaf path and the tag.
    pub(crate) label: String,
    /// The text in the input: empty, or the sent text of an error answer.
    pub(crate) text: String,
    /// True for a `BOOL` leaf, which gets a select.
    pub(crate) choice: bool,
    /// True when the input needs a text. A `String` or `Char` leaf takes the empty text too.
    pub(crate) required: bool,
    /// The length of a string leaf, or 0.
    pub(crate) maxlength: u16,
    /// The path of the leaf in the parameter after the rebind.
    path: LeafPath,
}

/// One moved parameter of a form with its inputs.
pub(crate) struct RebindParameter {
    /// The `id` of the `fieldset`: `rebind-parameter-<k>`. A line without an input names it.
    pub(crate) id: String,
    /// The tag of the parameter.
    pub(crate) tag: String,
    /// The key of the parameter.
    key: ParameterKey,
    /// The planned tree of the parameter. The class of a leaf in it converts the sent text.
    tree: TypeTree,
    /// One input per new member leaf. A parameter can have none.
    pub(crate) inputs: Vec<MemberInput>,
}

/// The data of one rebind form (ADR-0043 (l)).
///
/// The type solves one problem: the draft page and the error answer of a rebind show one form. The
/// page shows it empty, and the answer shows the sent texts, the sent `row_version`, and the lines.
pub(crate) struct RebindForm {
    /// The fixed `id` of the form: `rebind-<from>`. The answer swaps the form by it.
    pub(crate) id: String,
    /// The address of the route, for example `/recipes/R-100/versions/3/rebind`.
    pub(crate) action: String,
    /// The heading `rebind-title`, for example "Rebind ST_Zone@2 to ST_Zone@3".
    pub(crate) title: String,
    /// The CSRF token of the session.
    pub(crate) csrf_token: String,
    /// The `row_version` of the page, or the sent text of an error answer.
    pub(crate) row_version: String,
    /// The struct revision that the parameters use now.
    pub(crate) from: PlcTypeId,
    /// The newest revision of the struct.
    pub(crate) to: PlcTypeId,
    /// The newest revision as text, for example `ST_Zone@3`.
    pub(crate) to_text: String,
    /// The moved parameters, in the order of the list.
    pub(crate) parameters: Vec<RebindParameter>,
    /// The options of a `BOOL` leaf: none, `true`, and `false`.
    pub(crate) bool_options: Vec<Choice>,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    pub(crate) oob: bool,
    /// The error lines under the inputs.
    pub(crate) errors: FormErrors,
    /// The notice `rebind-too-large` when the page passes the bound `form.rebind_leaves`.
    ///
    /// A form with a notice holds no parameter, no field, and no submit button (ADR-0044 (c)).
    pub(crate) notice: Option<String>,
}

impl RebindForm {
    /// Return true when the form has no input, so the new revision adds no member.
    pub(crate) fn has_no_input(&self) -> bool {
        self.parameters
            .iter()
            .all(|parameter| parameter.inputs.is_empty())
    }

    /// Return the form with the sent texts, the sent `row_version`, and the lines, out of band.
    pub(crate) fn sent(
        mut self,
        texts: &BTreeMap<(ParameterKey, LeafPath), String>,
        row_version: String,
        errors: FormErrors,
    ) -> Self {
        for parameter in &mut self.parameters {
            for input in &mut parameter.inputs {
                let text = texts.get(&(parameter.key, input.path.clone()));
                input.text = text.cloned().unwrap_or_default();
            }
        }
        Self {
            row_version,
            oob: true,
            errors,
            ..self
        }
    }

    /// Return the `id` of the place of the line of `error`, or `None` for an error of no parameter.
    ///
    /// A line of a new leaf stands under its input. A line of a removed member (`E-RCP-029`) or of
    /// the whole value stands under the first input of its parameter. A parameter without an input
    /// takes the line under its `fieldset` (`design-system.md` section 10.5).
    pub(crate) fn target_of(&self, error: &RecipeError) -> Option<String> {
        let (tag, leaf) = match error {
            RecipeError::RebindNeedsValue { tag, leaf }
            | RecipeError::RebindDropsLimit { tag, leaf }
            | RecipeError::ValueShapeMismatch { tag, leaf, .. } => (tag, leaf),
            _other => return None,
        };
        let parameter = self
            .parameters
            .iter()
            .find(|parameter| parameter.tag == tag.as_str())?;
        let input = parameter
            .inputs
            .iter()
            .find(|input| &input.path == leaf)
            .or_else(|| parameter.inputs.first());
        Some(input.map_or_else(|| parameter.id.clone(), |input| input.id.clone()))
    }

    /// Return the planned tree of the moved parameter `key`, or `None` for another key.
    pub(crate) fn tree_of(&self, key: ParameterKey) -> Option<&TypeTree> {
        self.parameters
            .iter()
            .find(|parameter| parameter.key == key)
            .map(|parameter| &parameter.tree)
    }

    /// Return true when the form holds an input of the leaf `path` of the parameter `key`.
    pub(crate) fn holds(&self, key: ParameterKey, path: &LeafPath) -> bool {
        self.parameters
            .iter()
            .filter(|parameter| parameter.key == key)
            .flat_map(|parameter| &parameter.inputs)
            .any(|input| &input.path == path)
    }
}

/// Return true when the page shows the rebind forms.
///
/// The version must be a live draft of a live recipe, and the role must hold `ParameterEdit`.
pub(crate) fn shows_forms(request: &PageRequest, view: &VersionView) -> Result<bool, WebError> {
    let role = request.auth()?.user().role();
    Ok(view.detail.state == VersionState::LiveDraft
        && !view.recipe.is_deleted()
        && Operation::RebindTypeRevision.requirement().allows(role))
}

/// One moved parameter of the list with its number `k` and its new leaves.
type Moved<'v> = (usize, &'v OutdatedParameter, &'v RebindLeaves);

/// Return the moved parameters of the list with their numbers, in the order of the list.
fn moved(view: &VersionView) -> Vec<Moved<'_>> {
    view.outdated
        .iter()
        .filter_map(|item| item.rebind_leaves.as_ref().map(|leaves| (item, leaves)))
        .enumerate()
        .map(|(k, (item, leaves))| (k, item, leaves))
        .collect()
}

/// Build the rebind forms of a draft: one form per pair of revisions with a moved parameter.
///
/// The caller checks [`shows_forms`] first. The forms come in the order of the first parameter of
/// each pair in the list. Over the bound `form.rebind_leaves`, each form holds only the notice.
pub(crate) fn rebind_forms(
    t: Texts<'_>,
    request: &PageRequest,
    view: &VersionView,
) -> Vec<RebindForm> {
    if view.rebind_over_bound {
        return notice_forms(t, request, view);
    }
    let all = moved(view);
    let mut pairs: Vec<(PlcTypeId, PlcTypeId)> = Vec::new();
    for (_k, item, _leaves) in &all {
        if !pairs.contains(&(item.from, item.to)) {
            pairs.push((item.from, item.to));
        }
    }
    pairs
        .into_iter()
        .filter_map(|pair| build_form(t, request, view, (&all, pair)))
        .collect()
}

/// Build the rebind form of the pair `(from, to)`, or `None` when no parameter of the pair moves.
pub(crate) fn posted_form(
    t: Texts<'_>,
    request: &PageRequest,
    view: &VersionView,
    pair: (PlcTypeId, PlcTypeId),
) -> Option<RebindForm> {
    build_form(t, request, view, (&moved(view), pair))
}

/// Build the form of one pair from the moved parameters of the list.
fn build_form(
    t: Texts<'_>,
    request: &PageRequest,
    view: &VersionView,
    (all, (from, to)): (&[Moved<'_>], (PlcTypeId, PlcTypeId)),
) -> Option<RebindForm> {
    let group: Vec<&Moved<'_>> = all
        .iter()
        .filter(|(_k, item, _leaves)| (item.from, item.to) == (from, to))
        .collect();
    let (_k, first, _leaves) = group.first()?;
    let parameters = group
        .iter()
        .map(|(k, item, leaves)| rebind_parameter(t, *k, item, leaves))
        .collect();
    Some(RebindForm {
        parameters,
        ..form_shell(t, request, view, first)
    })
}

/// Build one form per pair of revisions of the list, each with the notice `rebind-too-large`.
///
/// Over the bound, no parameter holds its new leaves (ADR-0044 (c)). So each pair of the list
/// gets a form, also a pair whose parameters the rebind does not move.
fn notice_forms(t: Texts<'_>, request: &PageRequest, view: &VersionView) -> Vec<RebindForm> {
    let mut args = FluentArgs::new();
    args.set("count", view.rebind_sum);
    args.set("max", view.rebind_bound);
    let notice = t.with("rebind-too-large", &args);
    let mut firsts: Vec<&OutdatedParameter> = Vec::new();
    for item in &view.outdated {
        if !firsts
            .iter()
            .any(|first| (first.from, first.to) == (item.from, item.to))
        {
            firsts.push(item);
        }
    }
    firsts
        .into_iter()
        .map(|first| RebindForm {
            notice: Some(notice.clone()),
            ..form_shell(t, request, view, first)
        })
        .collect()
}

/// Build the form of the pair of `first` without a parameter and without a notice.
fn form_shell(
    t: Texts<'_>,
    request: &PageRequest,
    view: &VersionView,
    first: &OutdatedParameter,
) -> RebindForm {
    let to_text = format!("{}@{}", first.type_name, first.newest);
    let mut args = FluentArgs::new();
    args.set("from", format!("{}@{}", first.type_name, first.current));
    args.set("to", to_text.clone());
    RebindForm {
        id: format!("rebind-{}", first.from),
        action: format!(
            "/recipes/{}/versions/{}/rebind",
            view.recipe.reference(),
            view.detail.number
        ),
        title: t.with("rebind-title", &args),
        csrf_token: request.csrf_token(),
        row_version: view.detail.row_version.to_string(),
        from: first.from,
        to: first.to,
        to_text,
        parameters: Vec::new(),
        bool_options: vec![
            Choice::new("", String::new()),
            Choice::new("true", t.get("grid-true")),
            Choice::new("false", t.get("grid-false")),
        ],
        oob: false,
        errors: FormErrors::default(),
        notice: None,
    }
}

/// Build the moved parameter number `k` with one input per new leaf.
fn rebind_parameter(
    t: Texts<'_>,
    k: usize,
    item: &OutdatedParameter,
    leaves: &RebindLeaves,
) -> RebindParameter {
    let tag = item.tag.to_string();
    let inputs = leaves
        .leaves()
        .iter()
        .enumerate()
        .map(|(n, leaf)| {
            let target = leaf.target();
            let class = target.leaf_class();
            let mut args = FluentArgs::new();
            args.set("leaf", leaf.path().to_string());
            args.set("tag", tag.clone());
            MemberInput {
                id: format!("new-{k}-{n}"),
                name: format!("new_member_values[{}]{}", item.key, leaf.path()),
                label: t.with("field-new-member", &args),
                text: String::new(),
                choice: class == LeafClass::Bool,
                required: !matches!(class, LeafClass::String | LeafClass::Char),
                maxlength: target.string_length().unwrap_or_default(),
                path: leaf.path().clone(),
            }
        })
        .collect();
    RebindParameter {
        id: format!("rebind-parameter-{k}"),
        tag,
        key: item.key,
        tree: leaves.tree().clone(),
        inputs,
    }
}
