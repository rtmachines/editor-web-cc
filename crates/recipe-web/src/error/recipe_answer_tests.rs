//! Tests of the answers of the recipe codes and of `E-PLT-019`, `E-PLT-020`, and `E-PLT-030..038`
//! in `pl` and `en` (task 7.3, design D13), and of the English texts against the registry.

use std::collections::BTreeSet;

use axum::Router;
use axum::extract::Path;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::Method;
use axum::routing::get;
use recipe_app::error::{AppError, StorageError};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineCode, PlantError, UnitPath};
use recipe_domain::recipe::{RecipeError, ReferenceNumber, VersionStatus};

use super::WebError;
use crate::i18n::Lang;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{PEER, Reply, get_with, state};

/// The registry of `reliability.md` section 1.2.
const REGISTRY: &str = include_str!("../../../../docs/architecture/reliability.md");
/// The English bundle.
const EN: &str = include_str!("../../i18n/en.ftl");
/// The Polish bundle.
const PL: &str = include_str!("../../i18n/pl.ftl");

/// The trigger codes of design D13 that reach the user as `StorageError::Constraint`.
fn trigger_codes() -> Vec<String> {
    let plant = (30..=38).map(|n| format!("E-PLT-{n:03}"));
    let recipe = (30..=36).map(|n| format!("E-RCP-{n:03}"));
    plant.chain(recipe).collect()
}

/// Return the codes of the rows of the registry that start with `prefix`, such as `E-RCP-`.
fn registry_codes(prefix: &str) -> Vec<String> {
    let start = format!("| `{prefix}");
    REGISTRY
        .lines()
        .filter(|line| line.starts_with(&start))
        .filter_map(|line| line.split('`').nth(1).map(str::to_owned))
        .collect()
}

/// Return the codes of the keys `error-<prefix>...` of a `.ftl` source.
fn bundle_codes(source: &str, prefix: &str) -> Vec<String> {
    let start = format!("error-{prefix}");
    source
        .lines()
        .filter_map(|line| line.strip_prefix("error-")?.split_once(" = "))
        .map(|(code, _text)| code.to_owned())
        .filter(|code| format!("error-{code}").starts_with(&start))
        .collect()
}

/// Every code of design D13 with a message: each recipe row of the registry, `E-PLT-019`,
/// `E-PLT-020`, and the plant trigger codes.
fn d13_codes() -> Vec<String> {
    let plant = ["E-PLT-019".to_owned(), "E-PLT-020".to_owned()];
    let plant_triggers = (30..=38).map(|n| format!("E-PLT-{n:03}"));
    registry_codes("E-RCP-")
        .into_iter()
        .chain(plant)
        .chain(plant_triggers)
        .collect()
}

#[test]
fn recipe_answer_the_recipe_codes_of_the_registry_are_the_recipe_keys_of_both_bundles() {
    let registry: BTreeSet<String> = registry_codes("E-RCP-").into_iter().collect();
    assert!(registry.len() >= 41, "{registry:?}");
    for source in [EN, PL] {
        let bundle: BTreeSet<String> = bundle_codes(source, "E-RCP-").into_iter().collect();
        assert_eq!(bundle, registry);
    }
}

/// Return the user message of `code` in its registry row, without a note in parentheses.
fn registry_message(code: &str) -> Option<&'static str> {
    REGISTRY
        .lines()
        .filter(|line| line.starts_with("| `E-"))
        .find_map(|line| {
            let cells: Vec<&'static str> = line.split('|').map(str::trim).collect();
            let row_code = cells.get(1)?.trim_matches('`');
            let message = cells.get(4)?.split(" (").next()?;
            (row_code == code).then_some(message)
        })
}

/// Turn a registry text into its `fluent` form without the sentence of the reference.
fn fluent_text(registry: &str) -> String {
    let text = registry
        .strip_suffix(" Reference: {request_id}.")
        .unwrap_or(registry);
    text.replace('{', "{ $").replace('}', " }")
}

/// Return the text of the message `key` in a `.ftl` source.
fn ftl_message<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    source
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(" = "))
}

#[test]
fn recipe_answer_each_english_message_of_design_d13_equals_its_registry_row() {
    for code in d13_codes() {
        let expected = fluent_text(registry_message(&code).unwrap_or("<no registry row>"));
        let actual = ftl_message(EN, &format!("error-{code}"));
        assert_eq!(actual, Some(expected.as_str()), "{code}");
    }
}

#[test]
fn recipe_answer_each_code_of_design_d13_has_a_polish_message() {
    for code in d13_codes() {
        let key = format!("error-{code}");
        let polish = ftl_message(PL, &key).unwrap_or_default();
        assert!(!polish.is_empty(), "{key}");
        assert_ne!(Some(polish), ftl_message(EN, &key), "{key}");
    }
}

/// Return the error that the test route raises for `case`.
fn error_for(case: &str) -> WebError {
    let reference = ReferenceNumber::parse("R-100").unwrap();
    match case {
        "editable" => WebError::App(AppError::Recipe(RecipeError::VersionNotEditable {
            reference,
            number: 3,
            status: VersionStatus::Retired,
        })),
        "machine" => WebError::App(AppError::Plant(PlantError::MachineNotFound {
            code: MachineCode::parse("M99").unwrap(),
        })),
        "unit" => WebError::App(AppError::Plant(PlantError::UnitNotFound {
            path: UnitPath::parse("U9").unwrap(),
        })),
        trigger => WebError::App(AppError::Storage(StorageError::Constraint {
            code: ErrorCode::find(trigger).unwrap(),
            message: "The rule of the database stopped the change.".to_owned(),
        })),
    }
}

async fn raise(Path(case): Path<String>) -> Result<&'static str, WebError> {
    Err(error_for(&case))
}

fn router() -> Router {
    let rows = vec![RouteEntry {
        method: Method::GET,
        path: "/test/{case}",
        access: Access::Static,
        handler: get(raise),
    }];
    assemble(state(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

async fn ask(case: &str, lang: Lang) -> Reply {
    let cookie = format!("recipe_lang={lang}");
    let headers = [("cookie", cookie.as_str()), ("hx-request", "true")];
    get_with(&router(), &format!("/test/{case}"), &headers).await
}

#[tokio::test]
async fn recipe_answer_a_single_recipe_error_shows_its_polish_sentence_with_the_state_word() {
    let reply = ask("editable", Lang::Pl).await;
    assert!(reply.body.contains("E-RCP-002"), "{}", reply.body);
    let sentence = "Wersja 3 receptury R-100 ma stan Wycofana. Utw\u{f3}rz now\u{105} wersj\u{119} robocz\u{105}, aby wprowadzi\u{107} zmiany.";
    assert!(reply.body.contains(sentence), "{}", reply.body);
    let english = ask("editable", Lang::En).await;
    let sentence = "Version 3 of recipe R-100 is Retired. Create a new draft to make changes.";
    assert!(english.body.contains(sentence), "{}", english.body);
}

#[tokio::test]
async fn recipe_answer_e_plt_019_and_e_plt_020_show_their_sentence_in_both_languages() {
    let cases = [
        (
            "machine",
            Lang::Pl,
            "Maszyna M99 nie istnieje. Wybierz istniej\u{105}c\u{105} maszyn\u{119}.",
        ),
        (
            "machine",
            Lang::En,
            "Machine M99 does not exist. Select an existing machine.",
        ),
        (
            "unit",
            Lang::Pl,
            "Jednostka U9 nie istnieje na tej maszynie. Wybierz istniej\u{105}c\u{105} jednostk\u{119}.",
        ),
        (
            "unit",
            Lang::En,
            "Unit U9 does not exist on this machine. Select an existing unit.",
        ),
    ];
    for (case, lang, sentence) in cases {
        let reply = ask(case, lang).await;
        assert!(
            reply.body.contains(sentence),
            "{case} {lang}: {}",
            reply.body
        );
    }
}

#[tokio::test]
async fn recipe_answer_each_trigger_code_shows_the_sentence_of_its_bundle() {
    for code in trigger_codes() {
        let key = format!("error-{code}");
        for (lang, source) in [(Lang::Pl, PL), (Lang::En, EN)] {
            let reply = ask(&code, lang).await;
            let sentence = ftl_message(source, &key).unwrap();
            assert!(
                reply.body.contains(sentence),
                "{code} {lang}: {}",
                reply.body
            );
            assert!(!reply.body.contains("The rule of the database"), "{code}");
        }
    }
}
