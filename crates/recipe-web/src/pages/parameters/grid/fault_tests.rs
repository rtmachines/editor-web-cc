//! Tests of the input of a fault in the leaf grid (ADR-0039 (n), ADR-0041 (e)).
//!
//! A fault of a value stands under the value input of its leaf. A fault of a limit stands under the
//! first input of the limit row of its leaf. The line, `aria-invalid`, and the summary link all use
//! this input.

use recipe_domain::plant::{LeafPath, Primitive, TypeTree};
use recipe_domain::recipe::{ParameterTag, RecipeError};

use super::limits::LimitTexts;
use super::{GridSource, GridView, LeafInput, ValueTexts};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::parameters::tree_kit::{array_tree, primitive_tree, string_tree};

/// Build the English grid of `tree` with the limit rows `limits`.
fn grid(tree: &TypeTree, limits: &[(&str, &str, &str)]) -> GridView {
    let bundles = Bundles::load().unwrap();
    let limits = limits
        .iter()
        .map(|(path, min, max)| LimitTexts {
            path: LeafPath::parse(path).unwrap(),
            min: (*min).to_owned(),
            max: (*max).to_owned(),
            max_length: String::new(),
        })
        .collect();
    let source = GridSource {
        tree,
        values: ValueTexts::default(),
        limits,
        toggle: None,
        chunk: 100,
        root_label: "SPEED".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}

/// Return the tag `SPEED` and the leaf path `text`.
fn spot(text: &str) -> (ParameterTag, LeafPath) {
    (
        ParameterTag::parse("SPEED").unwrap(),
        LeafPath::parse(text).unwrap(),
    )
}

/// Return one fault of each kind on the leaf `text`, with the kind of input that it names.
fn faults(text: &str) -> Vec<(RecipeError, LeafInput)> {
    let (tag, leaf) = spot(text);
    let bounds = || ("10".to_owned(), "5".to_owned());
    vec![
        (
            RecipeError::ValueOutOfLimit {
                tag: tag.clone(),
                leaf: leaf.clone(),
                value: "42".to_owned(),
                min: bounds().1,
                max: bounds().0,
            },
            LeafInput::Value,
        ),
        (
            RecipeError::LimitOutsideTypeRange {
                tag: tag.clone(),
                leaf: leaf.clone(),
            },
            LeafInput::Limit,
        ),
        (
            RecipeError::MinGreaterThanMax {
                tag: tag.clone(),
                leaf: leaf.clone(),
                min: bounds().0,
                max: bounds().1,
            },
            LeafInput::Limit,
        ),
        (RecipeError::LimitMissing { tag, leaf }, LeafInput::Limit),
    ]
}

#[test]
fn grid_fault_of_a_value_names_the_value_input_and_of_a_limit_its_limit_input() {
    let view = grid(&primitive_tree(Primitive::Int), &[("", "0", "10")]);
    for (fault, kind) in faults("") {
        let want = match kind {
            LeafInput::Value => "value-0",
            LeafInput::Limit => "limits-0-min",
        };
        assert_eq!(view.input_of(&fault), Some(want.to_owned()), "{fault}");
    }
}

#[test]
fn grid_fault_of_a_limit_of_an_element_names_the_input_of_the_row_all() {
    let view = grid(&array_tree(Primitive::Int, &[(1, 8)]), &[("[*]", "0", "1")]);
    let found: Vec<Option<String>> = faults("[3]")
        .iter()
        .map(|(fault, _kind)| view.input_of(fault))
        .collect();
    let all = Some("limits-0-min".to_owned());
    assert_eq!(
        found,
        [Some("value-2".to_owned()), all.clone(), all.clone(), all]
    );
}

#[test]
fn grid_fault_of_a_string_limit_names_the_maximum_length() {
    let view = grid(&string_tree(Primitive::String, 4), &[]);
    let (tag, leaf) = spot("");
    let fault = RecipeError::LimitOutsideTypeRange { tag, leaf };
    assert_eq!(
        view.input_of(&fault),
        Some("limits-0-max_length".to_owned())
    );
}

#[test]
fn grid_fault_of_a_limit_on_a_bool_leaf_names_no_input() {
    let view = grid(&primitive_tree(Primitive::Bool), &[]);
    let (tag, leaf) = spot("");
    let fault = RecipeError::LimitNotApplicable { tag, leaf };
    assert_eq!(view.input_of(&fault), None);
    let fault = RecipeError::ValueShapeMismatch {
        tag: ParameterTag::parse("SPEED").unwrap(),
        leaf: LeafPath::parse("").unwrap(),
        expected: recipe_domain::recipe::Expected::Scalar,
    };
    assert_eq!(view.input_of(&fault), Some("value-0".to_owned()));
}
