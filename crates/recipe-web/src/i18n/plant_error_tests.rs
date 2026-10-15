//! Tests of the messages of the plant errors (task 13.5, design D20 U1, spec "Error fragment
//! content").
//!
//! The registry rows come from `reliability.md` section 1.2. The page test sends a request that
//! gives `E-PLT-017` in Polish.

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::tests::{fluent_text, ftl_message, registry_message};
use super::{Bundles, EN_SOURCE, Lang, PL_SOURCE};
use crate::pages::Texts;
use crate::test_support::{Fixture, get_with};

/// The 18 codes of the plant errors that have a message in the bundles (design D13).
const PLANT_CODES: [&str; 18] = [
    "E-PLT-001",
    "E-PLT-002",
    "E-PLT-003",
    "E-PLT-004",
    "E-PLT-005",
    "E-PLT-006",
    "E-PLT-008",
    "E-PLT-009",
    "E-PLT-010",
    "E-PLT-011",
    "E-PLT-012",
    "E-PLT-013",
    "E-PLT-014",
    "E-PLT-015",
    "E-PLT-016",
    "E-PLT-017",
    "E-PLT-018",
    "E-PLT-021",
];

#[test]
fn plant_error_english_text_equals_its_registry_row() {
    for code in PLANT_CODES {
        let expected = fluent_text(registry_message(code).unwrap_or("<no registry row>"));
        let actual = ftl_message(EN_SOURCE, &format!("error-{code}"));
        assert_eq!(actual, Some(expected.as_str()), "{code}");
    }
}

#[test]
fn plant_error_polish_has_a_text_for_each_code() {
    for code in PLANT_CODES {
        let key = format!("error-{code}");
        assert!(ftl_message(PL_SOURCE, &key).is_some(), "{key} in pl.ftl");
        assert!(ftl_message(EN_SOURCE, &key).is_some(), "{key} in en.ftl");
    }
}

#[tokio::test]
async fn plant_error_in_polish_shows_the_polish_sentence_with_the_text() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, false);
    fixture.seed_plant();
    let headers = [
        ("cookie", session.cookie.as_str()),
        ("cookie", "recipe_lang=pl"),
        ("hx-request", "true"),
    ];
    let reply = get_with(&fixture.router(), "/machines/M7/units/X1", &headers).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-017"), "{}", reply.body);
    let bundles = Bundles::load().unwrap();
    let sentence = Texts::new(&bundles, Lang::Pl).with_arg("error-E-PLT-017", "text", "X1");
    assert!(sentence.starts_with("X1 nie jest ścieżką jednostki."));
    assert!(reply.body.contains(&sentence), "{}", reply.body);
    assert!(!reply.body.contains("is not a unit path"), "{}", reply.body);
}
