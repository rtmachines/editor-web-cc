//! Tests that keep `static/app.css` equal to `docs/architecture/design-system.md` (design D4).
//!
//! The token test compares each token table with the one `:root` block. The contrast test computes
//! the WCAG 2.2 ratio of each pair of section 4. The rule tests live in `rules`.

mod css;
mod review_rules;
mod rules;

use css::{Rule, collapse, contrast_ratio, parse_sheet, parse_tables};

/// The design system document.
const DESIGN: &str = include_str!("../../../docs/architecture/design-system.md");
/// The style sheet.
const APP_CSS: &str = include_str!("../static/app.css");

/// Read the style sheet.
fn sheet() -> Vec<Rule> {
    parse_sheet(APP_CSS).unwrap()
}

/// Return the custom properties of the one `:root` block, in source order.
fn root_tokens(sheet: &[Rule]) -> Vec<(String, String)> {
    let roots: Vec<&Rule> = sheet
        .iter()
        .filter(|rule| rule.selectors == [":root"])
        .collect();
    assert_eq!(roots.len(), 1, "app.css needs exactly one :root block");
    let root = roots.first().unwrap();
    assert!(root.media.is_none());
    root.decls
        .iter()
        .inspect(|(name, _value)| assert!(name.starts_with("--"), "{name} in :root"))
        .cloned()
        .collect()
}

/// Return the rows of every token table of the document: name and value.
fn documented_tokens() -> Vec<(String, String)> {
    parse_tables(DESIGN, &["Token", "Value", "Use"])
        .unwrap()
        .into_iter()
        .map(|row| (row[0].clone(), collapse(&row[1])))
        .collect()
}

#[test]
fn tokens_every_documented_token_equals_the_root_value_in_document_order() {
    let documented = documented_tokens();
    assert!(documented.len() > 60, "found {} rows", documented.len());
    let actual = root_tokens(&sheet());
    for (name, value) in &documented {
        let found = actual.iter().find(|(other, _value)| other == name);
        assert_eq!(found.map(|(_name, v)| v), Some(value), "{name}");
    }
    assert_eq!(actual, documented);
}

#[test]
fn tokens_every_root_property_has_a_row_in_a_token_table() {
    let documented = documented_tokens();
    for (name, _value) in root_tokens(&sheet()) {
        assert!(
            documented.iter().any(|(other, _value)| *other == name),
            "{name} has no row"
        );
    }
}

#[test]
fn tokens_the_document_has_no_duplicate_token_row() {
    let mut names: Vec<String> = documented_tokens().into_iter().map(|(n, _v)| n).collect();
    let count = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), count);
}

#[test]
fn contrast_every_pair_of_section_4_reaches_its_minimum() {
    let root = root_tokens(&sheet());
    let color = |name: &str| -> String {
        root.iter()
            .find(|(other, _value)| other == name)
            .map(|(_name, value)| value.clone())
            .unwrap_or_else(|| format!("missing token {name}"))
    };
    let pairs = parse_tables(DESIGN, &["Foreground", "Background", "Minimum"]).unwrap();
    assert!(pairs.len() > 50, "found {} pairs", pairs.len());
    for pair in pairs {
        let minimum: f64 = pair[2].parse().unwrap();
        let ratio = contrast_ratio(&color(&pair[0]), &color(&pair[1])).unwrap();
        assert!(
            ratio >= minimum,
            "{} on {}: {ratio:.2} < {minimum}",
            pair[0],
            pair[1]
        );
    }
}
