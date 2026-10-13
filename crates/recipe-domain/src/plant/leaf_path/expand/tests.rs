use crate::plant::fixtures::{array_row, id, primitive_row, struct_row, types};
use crate::plant::{LeafPath, PlcType, Primitive, TypeTree};

const INT: u128 = 1;

fn tree_of(mut rows: Vec<PlcType>, root: u128) -> TypeTree {
    let mut all = vec![primitive_row(INT, Primitive::Int)];
    all.append(&mut rows);
    TypeTree::resolve(id(root), &types(all)).unwrap()
}

fn expanded(tree: &TypeTree, text: &str) -> Option<Vec<String>> {
    let path = LeafPath::parse(text).unwrap();
    path.concrete_paths(tree)
        .map(|paths| paths.iter().map(ToString::to_string).collect())
}

#[test]
fn path_without_all_elements_stays_one_path() {
    let tree = tree_of(vec![array_row(10, INT, &[(1, 3)])], 10);
    assert_eq!(expanded(&tree, "[2]"), Some(vec!["[2]".to_owned()]));
}

#[test]
fn all_elements_runs_over_every_index_in_row_major_order() {
    let tree = tree_of(vec![array_row(10, INT, &[(0, 1), (1, 2)])], 10);
    assert_eq!(
        expanded(&tree, "[*]"),
        Some(
            vec!["[0,1]", "[0,2]", "[1,1]", "[1,2]"]
                .into_iter()
                .map(String::from)
                .collect()
        )
    );
}

#[test]
fn two_all_elements_segments_multiply() {
    let rows = vec![array_row(10, INT, &[(1, 2)]), array_row(11, 10, &[(5, 6)])];
    let tree = tree_of(rows, 11);
    assert_eq!(expanded(&tree, "[*][*]").map(|paths| paths.len()), Some(4));
    assert_eq!(
        expanded(&tree, "[*][2]"),
        Some(vec!["[5][2]".to_owned(), "[6][2]".to_owned()])
    );
}

#[test]
fn member_path_with_all_elements_below_keeps_its_prefix() {
    let rows = vec![
        array_row(10, INT, &[(1, 2)]),
        struct_row(20, "ST_A", &[("ramps", 10)]),
    ];
    let tree = tree_of(rows, 20);
    assert_eq!(
        expanded(&tree, ".ramps[*]"),
        Some(vec![".ramps[1]".to_owned(), ".ramps[2]".to_owned()])
    );
}

#[test]
fn path_without_a_leaf_gives_none() {
    let tree = tree_of(vec![array_row(10, INT, &[(1, 2)])], 10);
    assert_eq!(expanded(&tree, "[3]"), None);
    assert_eq!(expanded(&tree, ""), None);
    assert_eq!(expanded(&tree, ".x"), None);
}
