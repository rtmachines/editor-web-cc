//! The view model of the unit tree (design D7, ADR-0037 b to d).
//!
//! The module solves one problem: the machine page, the page of the tree, and the answer of
//! `AddUnit` show the same tree. One function builds the nodes from a machine. So these places
//! agree on the expanded nodes, the links, and the nodes that can get a child.
//!
//! The template `unit-tree.html` renders a [`UnitTree`]. A template struct that includes it needs
//! the fields `t: Texts` and `tree: UnitTree`.

use std::collections::BTreeMap;

use recipe_app::auth::Operation;
use recipe_domain::identity::Role;
use recipe_domain::plant::{Machine, MachineCode, Unit, UnitId, UnitPath};

/// The deepest level of a unit path (A30). A node of this depth has no child.
const MAX_DEPTH: u8 = 5;
/// The nodes down to this depth are expanded when a page opens (ADR-0037 b).
const EXPANDED_DEPTH: u8 = 2;

/// Which units of the machine the tree shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The live units only. The tree of a live machine uses it (ADR-0037 d).
    Live,
    /// Every unit, live or archived. The page of an archived machine uses it.
    All,
}

impl Scope {
    /// Return true when the tree of this scope shows `unit`.
    fn shows(self, unit: &Unit) -> bool {
        matches!(self, Self::All) || unit.is_live()
    }
}

/// One unit in the tree with its children.
///
/// The template writes a node with children as a button and a group, and a node without children
/// as a leaf. An empty address omits the link, so one flag set serves a live and an archived tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnitNode {
    /// The identifier of the unit. The `id` of the group of its children uses it.
    pub(crate) id: UnitId,
    /// The path of the unit, for example `U1.2`.
    pub(crate) path: String,
    /// The name of the unit.
    pub(crate) name: String,
    /// The number of segments of the path, from 1 to 5.
    pub(crate) depth: u8,
    /// True for an archived unit. The node then shows the state "Archived".
    pub(crate) archived: bool,
    /// True when the group of the children shows when the page opens.
    pub(crate) expanded: bool,
    /// True for the unit that gets the focus after the swap.
    pub(crate) focus: bool,
    /// The address of the unit page, or an empty text when the tree has no links.
    pub(crate) href: String,
    /// The address of the form for a sub-unit, or an empty text when the node has no add link.
    pub(crate) add_href: String,
    /// The children in the order of the path.
    pub(crate) children: Vec<UnitNode>,
}

/// The unit tree of one machine: the main units with their children, in the order of the path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnitTree {
    /// The nodes of depth 1.
    pub(crate) roots: Vec<UnitNode>,
}

/// What every node needs besides its unit.
struct Context<'a> {
    code: MachineCode,
    links: bool,
    can_add: bool,
    focus: Option<&'a UnitPath>,
}

/// The units that are shown, grouped by the path of their parent. A main unit has no parent.
type ByParent<'a> = BTreeMap<Option<UnitPath>, Vec<&'a Unit>>;

impl UnitTree {
    /// Build the tree of `machine`.
    ///
    /// - `scope` chooses the live units or every unit.
    /// - `role` decides the add links: a node of depth 1 to 4 gets one when the role holds
    ///   `AddUnit`. An archived node never gets one.
    /// - `focus` is the unit that an `AddUnit` request made. Its link gets the focus and its
    ///   ancestors are expanded. A full page passes `None`.
    /// - `links` is false for the page of an archived machine. The tree then has no address, no
    ///   add link, and no focus, because every route below `/machines/{code}` resolves the live
    ///   machine (ADR-0037 d).
    ///
    /// The nodes of depth 1 and 2 are expanded.
    #[must_use]
    pub(crate) fn build(
        machine: &Machine,
        scope: Scope,
        role: Role,
        focus: Option<&UnitPath>,
        links: bool,
    ) -> Self {
        let context = Context {
            code: machine.code(),
            links,
            can_add: links && Operation::AddUnit.requirement().allows(role),
            focus,
        };
        let mut by_parent: ByParent<'_> = BTreeMap::new();
        for unit in machine.units().filter(|unit| scope.shows(unit)) {
            by_parent
                .entry(unit.path().parent())
                .or_default()
                .push(unit);
        }
        Self {
            roots: nodes_under(&by_parent, None, &context),
        }
    }
}

/// Build the nodes of the units whose parent is `parent`, in the order of the path.
fn nodes_under(
    by_parent: &ByParent<'_>,
    parent: Option<UnitPath>,
    context: &Context<'_>,
) -> Vec<UnitNode> {
    by_parent
        .get(&parent)
        .map(|units| {
            units
                .iter()
                .map(|unit| node_of(unit, by_parent, context))
                .collect()
        })
        .unwrap_or_default()
}

/// Build the node of `unit` with its children. The depth of a path is at most 5, so the
/// recursion ends.
fn node_of(unit: &Unit, by_parent: &ByParent<'_>, context: &Context<'_>) -> UnitNode {
    let path = unit.path();
    let depth = path.depth();
    let archived = !unit.is_live();
    let add_link = context.can_add && !archived && depth < MAX_DEPTH;
    UnitNode {
        id: unit.id(),
        path: path.to_string(),
        name: unit.name().to_string(),
        depth,
        archived,
        expanded: depth <= EXPANDED_DEPTH
            || context
                .focus
                .is_some_and(|focus| path.is_ancestor_of(focus)),
        focus: context.links && context.focus == Some(path),
        href: if context.links {
            format!("/machines/{}/units/{path}", context.code)
        } else {
            String::new()
        },
        add_href: if add_link {
            format!("/machines/{}/units/new?parent_path={path}", context.code)
        } else {
            String::new()
        },
        children: nodes_under(by_parent, Some(path.clone()), context),
    }
}
