//! Tests of the view model of the unit tree (task 7.4, design D7).

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, UnitId, UnitPath, UnitUsage,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::tree::{Scope, UnitNode, UnitTree};

/// The units of the test machine `M5` besides the main unit `U1`: the path and the name.
///
/// A unit comes after its parent. `U2` and `U2.1` are archived at the end.
const UNITS: [(&str, &str); 9] = [
    ("U1.1", "Feeder"),
    ("U1.1.1", "Belt"),
    ("U1.1.1.1", "Roller"),
    ("U1.1.1.1.1", "Bearing"),
    ("U1.2", "Press"),
    ("U1.10", "Exit"),
    ("U2", "Spare"),
    ("U2.1", "Spare part"),
    ("U3", "Second main"),
];

/// Make the identifier of the unit with the number `number`.
fn unit_id(number: u128) -> UnitId {
    UnitId::from(Uuid::from_u128(number))
}

/// Parse a unit path.
fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Build the machine `M5` with the units of `UNITS`, and archive `U2` with `U2.1`.
fn machine() -> Machine {
    let name = |text: &str| Text::parse(text).unwrap();
    let mut machine = Machine::create(
        MachineId::from(Uuid::from_u128(5)),
        MachineCode::parse("M5").unwrap(),
        name("Line"),
        PlcPlatform::TwinCat3,
        (unit_id(1_001), name("Infeed")),
    );
    for (index, (text, unit_name)) in UNITS.iter().enumerate() {
        let unit_path = path(text);
        let parent = unit_path.parent();
        let segment = text.rsplit('.').next().unwrap().trim_start_matches('U');
        let number = 1_002_u128
            .checked_add(u128::try_from(index).unwrap())
            .unwrap();
        machine
            .add_unit(unit_id(number), parent.as_ref(), segment, name(unit_name))
            .unwrap();
    }
    let at = Timestamp::from(OffsetDateTime::from_unix_timestamp(10).unwrap());
    machine
        .archive_unit(&path("U2"), at, &UnitUsage::default())
        .unwrap();
    machine
}

/// Return the nodes in preorder.
fn flatten(nodes: &[UnitNode]) -> Vec<&UnitNode> {
    nodes
        .iter()
        .flat_map(|node| std::iter::once(node).chain(flatten(&node.children)))
        .collect()
}

/// Return the paths of the nodes in preorder.
fn paths(tree: &UnitTree) -> Vec<String> {
    flatten(&tree.roots)
        .into_iter()
        .map(|node| node.path.clone())
        .collect()
}

/// Return the node with this path.
fn node<'a>(tree: &'a UnitTree, text: &str) -> &'a UnitNode {
    flatten(&tree.roots)
        .into_iter()
        .find(|candidate| candidate.path == text)
        .unwrap()
}

/// Build the tree of `M5` for a ProcessEngineer with links and no focus unit.
fn tree_of(scope: Scope) -> UnitTree {
    UnitTree::build(&machine(), scope, Role::ProcessEngineer, None, true)
}

// --- Shape --------------------------------------------------------------------------------------

#[test]
fn machines_tree_model_nests_the_nodes_in_path_order_with_a_number_order() {
    let tree = tree_of(Scope::All);
    assert_eq!(
        paths(&tree),
        [
            "U1",
            "U1.1",
            "U1.1.1",
            "U1.1.1.1",
            "U1.1.1.1.1",
            "U1.2",
            "U1.10",
            "U2",
            "U2.1",
            "U3"
        ]
    );
    let roots: Vec<&str> = tree.roots.iter().map(|root| root.path.as_str()).collect();
    assert_eq!(roots, ["U1", "U2", "U3"]);
    let under_main: Vec<&str> = node(&tree, "U1")
        .children
        .iter()
        .map(|child| child.path.as_str())
        .collect();
    assert_eq!(
        under_main,
        ["U1.1", "U1.2", "U1.10"],
        "U1.2 comes before U1.10"
    );
}

#[test]
fn machines_tree_model_node_holds_identifier_path_name_depth_and_state() {
    let tree = tree_of(Scope::All);
    let main = node(&tree, "U1");
    assert_eq!(main.id, unit_id(1_001));
    assert_eq!(
        (main.path.as_str(), main.name.as_str(), main.depth),
        ("U1", "Infeed", 1)
    );
    assert!(!main.archived);
    let deep = node(&tree, "U1.1.1.1.1");
    assert_eq!((deep.name.as_str(), deep.depth), ("Bearing", 5));
    assert!(deep.children.is_empty());
}

#[test]
fn machines_tree_model_live_scope_drops_the_archived_units() {
    let tree = tree_of(Scope::Live);
    let shown = paths(&tree);
    assert!(
        !shown.iter().any(|text| text.starts_with("U2")),
        "{shown:?}"
    );
    assert_eq!(shown.len(), 8);
    assert!(flatten(&tree.roots).iter().all(|unit| !unit.archived));
}

#[test]
fn machines_tree_model_all_scope_marks_the_archived_units_and_their_children() {
    let tree = tree_of(Scope::All);
    let archived: Vec<&str> = flatten(&tree.roots)
        .into_iter()
        .filter(|unit| unit.archived)
        .map(|unit| unit.path.as_str())
        .collect();
    assert_eq!(archived, ["U2", "U2.1"]);
    assert_eq!(node(&tree, "U2").children.len(), 1);
}

#[test]
fn machines_tree_model_is_a_pure_function_of_its_input() {
    assert_eq!(tree_of(Scope::All), tree_of(Scope::All));
}

// --- Expansion and focus ------------------------------------------------------------------------

#[test]
fn machines_tree_model_expands_depth_1_and_2_and_collapses_the_deeper_nodes() {
    let tree = tree_of(Scope::Live);
    for unit in flatten(&tree.roots) {
        assert_eq!(unit.expanded, unit.depth <= 2, "{}", unit.path);
    }
}

#[test]
fn machines_tree_model_expands_the_ancestors_of_the_focus_unit_only() {
    let focus = path("U1.1.1.1");
    let tree = UnitTree::build(
        &machine(),
        Scope::Live,
        Role::ProcessEngineer,
        Some(&focus),
        true,
    );
    for text in ["U1", "U1.1", "U1.1.1"] {
        assert!(node(&tree, text).expanded, "{text}");
    }
    assert!(
        !node(&tree, "U1.1.1.1").expanded,
        "the focus unit is not its own ancestor"
    );
    assert!(!node(&tree, "U1.1.1.1.1").expanded);
    assert!(node(&tree, "U1.2").expanded, "depth 2 stays expanded");
}

#[test]
fn machines_tree_model_marks_only_the_focus_unit() {
    let focus = path("U1.1.1");
    let tree = UnitTree::build(&machine(), Scope::Live, Role::Operator, Some(&focus), true);
    let marked: Vec<&str> = flatten(&tree.roots)
        .into_iter()
        .filter(|unit| unit.focus)
        .map(|unit| unit.path.as_str())
        .collect();
    assert_eq!(marked, ["U1.1.1"]);
    assert!(
        flatten(&tree_of(Scope::Live).roots)
            .iter()
            .all(|unit| !unit.focus)
    );
}

// --- Links --------------------------------------------------------------------------------------

#[test]
fn machines_tree_model_links_each_unit_to_its_page() {
    let tree = tree_of(Scope::Live);
    for unit in flatten(&tree.roots) {
        assert_eq!(unit.href, format!("/machines/M5/units/{}", unit.path));
    }
}

#[test]
fn machines_tree_model_gives_an_add_link_to_live_nodes_of_depth_1_to_4() {
    for role in [Role::ProcessEngineer, Role::Administrator] {
        let tree = UnitTree::build(&machine(), Scope::All, role, None, true);
        for unit in flatten(&tree.roots) {
            let expected = !unit.archived && unit.depth < 5;
            assert_eq!(!unit.add_href.is_empty(), expected, "{role} {}", unit.path);
            if expected {
                let href = format!("/machines/M5/units/new?parent_path={}", unit.path);
                assert_eq!(unit.add_href, href);
            }
        }
    }
}

#[test]
fn machines_tree_model_gives_an_operator_no_add_link() {
    let tree = UnitTree::build(&machine(), Scope::Live, Role::Operator, None, true);
    assert!(
        flatten(&tree.roots)
            .iter()
            .all(|unit| unit.add_href.is_empty())
    );
    assert!(
        flatten(&tree.roots)
            .iter()
            .all(|unit| !unit.href.is_empty())
    );
}

#[test]
fn machines_tree_model_without_links_has_no_address_and_no_focus() {
    let focus = path("U1.2");
    let tree = UnitTree::build(
        &machine(),
        Scope::All,
        Role::Administrator,
        Some(&focus),
        false,
    );
    for unit in flatten(&tree.roots) {
        assert!(unit.href.is_empty(), "{}", unit.path);
        assert!(unit.add_href.is_empty(), "{}", unit.path);
        assert!(!unit.focus, "{}", unit.path);
    }
    assert!(
        node(&tree, "U1.2").expanded,
        "the focus still expands the ancestors"
    );
}
