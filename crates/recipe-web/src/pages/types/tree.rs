//! The type tree page: `GET /machines/{code}/types/{name}` (`api.md` section 2.3, design D6 and
//! D10, ADR-0037 e).

use std::collections::BTreeMap;

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{GetTypeTree, TypeRef, TypeView};
use recipe_domain::plant::{MachineCode, MemberName, PlcTypeId, TypeName, TypeNode};
use serde::Deserialize;

use super::{can_define, current_nav, named_revision, query_values, revision_href, type_href};
use crate::error::WebError;
use crate::layout::Layout;
use crate::pages::path_values::{path_code_type, revision};
use crate::pages::render::render;
use crate::pages::{PageRequest, Texts};
use crate::state::WebState;

/// The query of `GET /machines/{code}/types/{name}`: only `rev`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TypeQuery {
    /// The revision that the tree shows. A missing value means the latest revision.
    rev: Option<String>,
}

/// A link to one revision of the struct.
struct RevisionLink {
    href: String,
    label: String,
    /// True for the revision that the page shows.
    current: bool,
}

/// One entry of the flat list of the tree, in preorder.
///
/// The template cannot call a macro for a tree of any depth, because askama refuses a macro that
/// calls itself. The list therefore holds every node once, and `closes` tells how many groups end
/// behind the node.
struct TreeItem {
    /// True for a node with members. False for a leaf.
    group: bool,
    /// The UUID of the type of a group. A leaf has an empty text.
    id: String,
    /// The index of the node in preorder. It makes the id of a group unique.
    index: usize,
    name: String,
    type_text: String,
    /// True for the root group only.
    expanded: bool,
    /// The number of groups that end behind this node.
    closes: usize,
}

/// The page `type.html`.
#[derive(Template)]
#[template(path = "type.html")]
struct TypePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    title: String,
    revisions: Vec<RevisionLink>,
    supersedes: Option<RevisionLink>,
    items: Vec<TreeItem>,
    usage_href: String,
    new_revision_href: Option<String>,
    list_href: String,
    list_text: String,
}

/// A type row that the result of `GetTypeTree` gives no text for. A resolved tree never has one.
#[derive(Debug)]
struct MissingText;

/// The type expressions of `GetTypeTree`, by type row.
type MemberTypes = BTreeMap<PlcTypeId, String>;

/// Return the struct behind the array levels of a node, or `None` for a leaf.
///
/// The loop follows the array levels without recursion. A tree has at most `json.max_depth`
/// levels (`domain-model.md` 2.1), so the loop ends.
fn innermost_struct(node: &TypeNode) -> Option<(PlcTypeId, &[(MemberName, TypeNode)])> {
    let mut current = node;
    loop {
        match current {
            TypeNode::Array { element, .. } => current = element.as_ref(),
            TypeNode::Struct {
                type_id, members, ..
            } => return Some((*type_id, members)),
            TypeNode::Leaf { .. } => return None,
        }
    }
}

/// Add the end of one group to the last item of the list.
fn close_group(items: &mut [TreeItem]) {
    if let Some(last) = items.last_mut() {
        last.closes = last.closes.saturating_add(1);
    }
}

/// Add a group node, its members, and the end of its group.
fn push_group(
    items: &mut Vec<TreeItem>,
    texts: &MemberTypes,
    head: (PlcTypeId, String, String),
    body: &[(MemberName, TypeNode)],
) -> Result<(), MissingText> {
    let (id, name, type_text) = head;
    let index = items.len();
    items.push(TreeItem {
        group: true,
        id: id.to_string(),
        index,
        name,
        type_text,
        expanded: index == 0,
        closes: 0,
    });
    for (member, node) in body {
        push_node(items, texts, member.to_string(), node)?;
    }
    close_group(items);
    Ok(())
}

/// Add one member: a leaf for a primitive, and a group for a struct.
///
/// An array shows its element type once. An array of a struct is the group of that struct, and an
/// array of a primitive is a leaf. The type text of the member is the text from `GetTypeTree`.
fn push_node(
    items: &mut Vec<TreeItem>,
    texts: &MemberTypes,
    name: String,
    node: &TypeNode,
) -> Result<(), MissingText> {
    let type_text = texts.get(&node.type_id()).ok_or(MissingText)?.clone();
    if let Some((id, members)) = innermost_struct(node) {
        return push_group(items, texts, (id, name, type_text), members);
    }
    let index = items.len();
    items.push(TreeItem {
        group: false,
        id: String::new(),
        index,
        name,
        type_text,
        expanded: false,
        closes: 0,
    });
    Ok(())
}

/// Build the flat list of the tree of a revision.
///
/// The root is the struct itself. Only its group is expanded (ADR-0037 e).
fn tree_items(view: &TypeView) -> Result<Vec<TreeItem>, MissingText> {
    let mut items = Vec::new();
    if let TypeNode::Struct { members, .. } = view.tree.root() {
        let label = format!("{}@{}", view.name, view.revision);
        let head = (view.id, label, String::new());
        push_group(&mut items, &view.member_types, head, members)?;
    }
    Ok(items)
}

/// Build the link to revision `number` of the struct.
fn revision_link(code: MachineCode, name: &TypeName, number: u32, current: u32) -> RevisionLink {
    RevisionLink {
        href: revision_href(code, name, number),
        label: format!("{name}@{number}"),
        current: number == current,
    }
}

/// Build the page of a revision from the result of `GetTypeTree` and the list of its tree.
///
/// `may_define` is true when the role may define a UDT type, so the page links to the form of a
/// new revision.
fn page_of<'a>(
    (t, layout): (Texts<'a>, Layout<'a>),
    view: &TypeView,
    items: Vec<TreeItem>,
    may_define: bool,
) -> TypePage<'a> {
    let code = view.machine.code();
    let name = &view.name;
    let shown = view.revision;
    TypePage {
        layout,
        t,
        title: named_revision(t, "type-title", name, shown),
        revisions: view
            .revisions
            .iter()
            .map(|number| revision_link(code, name, *number, shown))
            .collect(),
        supersedes: view
            .supersedes
            .map(|number| revision_link(code, name, number, shown)),
        items,
        usage_href: format!("{}/usage?rev={shown}", type_href(code, name)),
        new_revision_href: may_define.then(|| format!("/machines/{code}/types/new?from={name}")),
        list_href: format!("/machines/{code}/types"),
        list_text: t.with_arg("types-title", "code", code),
    }
}

/// `GET /machines/{code}/types/{name}`: show the type tree of the latest revision or of `rev`.
pub(super) async fn show(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
    query: Result<Query<TypeQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let (code, name) = path_code_type(path)?;
    let wanted = query_values(query)?
        .rev
        .as_deref()
        .map(revision)
        .transpose()?;
    let type_ref = TypeRef {
        code,
        name,
        revision: wanted,
    };
    let view = GetTypeTree::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), type_ref)
        .await?;
    let items = tree_items(&view).map_err(|_missing| WebError::Internal {
        request_id: request.request_id(),
    })?;
    let t = Texts::new(state.bundles(), request.lang());
    let title = named_revision(t, "type-title", &view.name, view.revision);
    let layout = Layout::for_session(t, &request, title, current_nav())?;
    let page = page_of((t, layout), &view, items, can_define(&request)?);
    Ok(render(&page, request.request_id())?.into_response())
}
