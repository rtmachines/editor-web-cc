//! Tests that keep `static/app.css` and `static/components.css` equal to
//! `docs/architecture/design-system.md` (design D4, ADR-0037 r).
//!
//! The token test compares each token table with the one `:root` block of `app.css`. The contrast
//! test computes the WCAG 2.2 ratio of each pair of section 4. The rule tests of the base rules live
//! in `rules` and `review_rules`. The rule tests of the components live in `component_rules`.

mod component_rules;
mod css;
mod fix_rules;
mod review_rules;
mod rules;

use css::{Rule, collapse, contrast_ratio, parse_sheet, parse_tables};

/// The design system document.
const DESIGN: &str = include_str!("../../../docs/architecture/design-system.md");
/// The base style sheet. A page links it first.
const APP_CSS: &str = include_str!("../static/app.css");
/// The component style sheet. A page links it second.
const COMPONENTS_CSS: &str = include_str!("../static/components.css");

/// Read the base style sheet.
fn app_sheet() -> Vec<Rule> {
    parse_sheet(APP_CSS).unwrap()
}

/// Read the component style sheet.
fn components_sheet() -> Vec<Rule> {
    parse_sheet(COMPONENTS_CSS).unwrap()
}

/// Read both style sheets as one list of rules, in the order that a page links them.
///
/// A rule of the components comes after a rule of the base, as in the browser.
fn sheet() -> Vec<Rule> {
    let mut rules = app_sheet();
    rules.extend(components_sheet());
    rules
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
    let actual = root_tokens(&app_sheet());
    for (name, value) in &documented {
        let found = actual.iter().find(|(other, _value)| other == name);
        assert_eq!(found.map(|(_name, v)| v), Some(value), "{name}");
    }
    assert_eq!(actual, documented);
}

#[test]
fn tokens_every_root_property_has_a_row_in_a_token_table() {
    let documented = documented_tokens();
    for (name, _value) in root_tokens(&app_sheet()) {
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
    let root = root_tokens(&app_sheet());
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
