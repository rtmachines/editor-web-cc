//! The shared parts of the tests of the parameter routes (design D12). This module is test code.
//!
//! The module solves one problem: the tests of the form, the commands, and the pages of a parameter
//! send the same fields to the same draft. One list of fields and one sender keep them equal.

use axum::http::StatusCode;
use recipe_app::auth::RequestMeta;
use recipe_app::identity::authenticate;
use recipe_app::recipe::{UpsertTranslation, UpsertTranslationInput};
use recipe_domain::audit::RequestId;
use recipe_domain::identity::SessionToken;
use recipe_domain::recipe::{LangCode, ParameterKey, ReferenceNumber, Translation};
use recipe_domain::text::Text;

use crate::pages::test_helpers::{assert_htmx_error, encode};
use crate::test_support::Reply;
use crate::test_support::plant::{ENGLISH, HTMX, World};

/// Store the translation `lang` of the parameter `key` of the draft 3 of `R-100` as the
/// ProcessEngineer, through the use case and not through the web layer.
///
/// A test uses it for a tag outside `i18n.translation_langs`, which the web route refuses.
pub(crate) async fn seed_translation(
    world: &World,
    key: uuid::Uuid,
    (lang, row_version): (&str, i64),
) {
    let token = SessionToken::from_bytes([2; 16]);
    let auth = authenticate(world.fixture.state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    let meta = RequestMeta::new(RequestId::from(uuid::Uuid::nil()), None, None, None);
    let input = UpsertTranslationInput {
        reference: ReferenceNumber::parse("R-100").unwrap(),
        number: 3,
        expected_row_version: row_version,
        key: ParameterKey::from(key),
        translation: Translation {
            lang: LangCode::parse(lang).unwrap(),
            name: Text::parse("Linie").unwrap(),
            description: None,
        },
    };
    UpsertTranslation::new(world.fixture.state.identity().app().clone())
        .execute(auth.user(), &meta, input)
        .await
        .unwrap();
}

/// The path of the form of a new parameter of the draft 3 of `R-100`.
pub(crate) const NEW_FORM: &str = "/recipes/R-100/versions/3/parameters/new";
/// The path of `AddParameter` on the draft 3 of `R-100`.
pub(crate) const ADD: &str = "/recipes/R-100/versions/3/parameters";
/// The header that asks for the form fragment of `#parameter-form`.
pub(crate) const FORM_TARGET: (&str, &str) = ("hx-target", "parameter-form");

/// The definition fields of the stored parameter `SPEED` (`INT`, limit 0 to 3000) of `R-100`.
pub(crate) const SPEED_FIELDS: [(&str, &str); 13] = [
    ("row_version", "1"),
    ("unit_path", "U1"),
    ("tag", "SPEED"),
    ("name_en", "SPEED"),
    ("description_en", "A parameter of the press line"),
    ("type", "INT"),
    ("value_type", "INT"),
    ("s7_address", "DB2.DBW0"),
    ("ordinal", "1"),
    ("value", "1200"),
    ("limits[0].leaf_path", ""),
    ("limits[0].min", "0"),
    ("limits[0].max", "3000"),
];

/// Return the path of the parameter `key` of the draft 3 of `R-100` with the suffix `rest`.
///
/// An example is `parameter_path(SPEED_KEY, "/definition")`.
pub(crate) fn parameter_path(key: uuid::Uuid, rest: &str) -> String {
    format!("/recipes/R-100/versions/3/parameters/{key}{rest}")
}

/// The fields of a valid new `INT` parameter `NEW_P` of the draft 3 of `R-100` with the value 42.
///
/// The draft has the `row_version` 1, and its machine `M7` has the platform `S7_300`.
pub(crate) const INT_FIELDS: [(&str, &str); 10] = [
    ("row_version", "1"),
    ("unit_path", "U1"),
    ("tag", "NEW_P"),
    ("name_en", "New parameter"),
    ("description_en", "A new parameter"),
    ("type", "INT"),
    ("value_type", "INT"),
    ("s7_address", "DB9.DBW0"),
    ("ordinal", "5"),
    ("value", "42"),
];

/// Build a form body from pairs. Each key and each value is percent-encoded.
pub(crate) fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<String>>()
        .join("&")
}

/// Return `base` with the value of each key of `changes` replaced. A new key goes to the end.
///
/// A change with the value `None` removes the key.
pub(crate) fn changed<'a>(
    base: &[(&'a str, &'a str)],
    changes: &[(&'a str, Option<&'a str>)],
) -> Vec<(&'a str, &'a str)> {
    let mut pairs: Vec<(&str, &str)> = base
        .iter()
        .filter(|(key, _value)| !changes.iter().any(|(changed, _new)| changed == key))
        .copied()
        .collect();
    pairs.extend(
        changes
            .iter()
            .filter_map(|(key, value)| value.map(|value| (*key, value))),
    );
    pairs
}

/// Send the form `pairs` to `path` as the ProcessEngineer, as an HTMX request in English.
pub(crate) async fn post_htmx(world: &World, path: &str, pairs: &[(&str, &str)]) -> Reply {
    world
        .post_with(&world.engineer, path, &form(pairs), &[HTMX])
        .await
}

/// Send `GET path` as the ProcessEngineer as an HTMX request for `#parameter-form`, in English.
pub(crate) async fn get_form_fragment(world: &World, path: &str) -> Reply {
    world
        .get(&world.engineer, path, &[ENGLISH, HTMX, FORM_TARGET])
        .await
}

/// Assert an HTMX error answer in `#error-panel` that keeps the URL (`api.md` section 5.3).
pub(crate) fn assert_panel_error(reply: &Reply, status: StatusCode, code: &str) {
    assert_htmx_error(reply, status, code);
    assert_eq!(reply.header("hx-push-url"), "false", "{code}");
    assert_eq!(
        reply.header("hx-reswap"),
        "innerHTML show:#error-panel:top",
        "{code}"
    );
}

/// Return the position of `needle` in `body`. The test fails when `body` lacks it.
pub(crate) fn position(body: &str, needle: &str) -> usize {
    assert!(body.contains(needle), "missing {needle:?} in {body}");
    body.find(needle).unwrap()
}
