use axum::http::header::{ACCEPT_LANGUAGE, COOKIE};
use axum::http::{HeaderMap, HeaderValue};

use super::*;

fn headers_with_cookie(cookie: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_str(cookie).unwrap());
    headers
}

#[test]
fn i18n_lang_parse_accepts_only_pl_and_en() {
    assert_eq!(Lang::parse("pl"), Ok(Lang::Pl));
    assert_eq!(Lang::parse("en"), Ok(Lang::En));
    for text in ["", "de", "PL", "En", " pl", "pl ", "pl-PL", "en-US"] {
        assert_eq!(Lang::parse(text), Err(LangReason::Unsupported), "{text:?}");
    }
}

#[test]
fn i18n_lang_shows_its_code() {
    assert_eq!(Lang::Pl.as_str(), "pl");
    assert_eq!(Lang::En.to_string(), "en");
    assert_eq!(Lang::default(), Lang::Pl);
}

#[test]
fn i18n_missing_cookie_gives_polish() {
    assert_eq!(Lang::from_headers(&HeaderMap::new()), Lang::Pl);
}

#[test]
fn i18n_cookie_gives_its_language() {
    assert_eq!(
        Lang::from_headers(&headers_with_cookie("recipe_lang=en")),
        Lang::En
    );
    assert_eq!(
        Lang::from_headers(&headers_with_cookie("recipe_lang=pl")),
        Lang::Pl
    );
}

#[test]
fn i18n_cookie_among_other_cookies_is_found() {
    let headers = headers_with_cookie("a=1; recipe_lang=en;  b=2");
    assert_eq!(Lang::from_headers(&headers), Lang::En);
}

#[test]
fn i18n_cookie_in_a_second_cookie_header_is_found() {
    let mut headers = headers_with_cookie("a=1");
    headers.append(COOKIE, HeaderValue::from_static("recipe_lang=en"));
    assert_eq!(Lang::from_headers(&headers), Lang::En);
}

#[test]
fn i18n_unknown_cookie_value_gives_polish_and_ignores_accept_language() {
    let mut headers = headers_with_cookie("recipe_lang=de");
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US"));
    assert_eq!(Lang::from_headers(&headers), Lang::Pl);
}

#[test]
fn i18n_accept_language_alone_gives_polish() {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    assert_eq!(Lang::from_headers(&headers), Lang::Pl);
}

#[test]
fn i18n_cookie_name_and_value_must_match_exactly() {
    for cookie in [
        "xrecipe_lang=en",
        "recipe_lang_=en",
        "recipe_lang=EN",
        "recipe_lang=en-US",
        "recipe_lang=",
        "recipe_lang",
        "Recipe_Lang=en",
    ] {
        assert_eq!(
            Lang::from_headers(&headers_with_cookie(cookie)),
            Lang::Pl,
            "{cookie}"
        );
    }
}

#[test]
fn i18n_first_cookie_of_the_name_wins() {
    let headers = headers_with_cookie("recipe_lang=de; recipe_lang=en");
    assert_eq!(Lang::from_headers(&headers), Lang::Pl);
}

#[test]
fn i18n_bundles_load_both_languages_without_error() {
    assert!(Bundles::load().is_ok());
}

#[test]
fn i18n_message_comes_from_the_bundle_of_the_language() {
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        bundles.message(Lang::En, "language-name", None).unwrap(),
        "English"
    );
    assert_eq!(
        bundles.message(Lang::Pl, "language-name", None).unwrap(),
        "Polski"
    );
}

#[test]
fn i18n_message_inserts_arguments_without_isolation_marks() {
    let bundles = Bundles::load().unwrap();
    let mut args = FluentArgs::new();
    args.set("limit", "256 KiB");
    let text = bundles
        .message(Lang::En, "error-E-WEB-003", Some(&args))
        .unwrap();
    assert_eq!(text, "The request is larger than 256 KiB.");
}

#[test]
fn i18n_unknown_key_gives_none() {
    let bundles = Bundles::load().unwrap();
    assert_eq!(bundles.message(Lang::En, "no-such-key", None), None);
}

#[test]
fn i18n_messages_every_english_key_exists_in_polish_and_back() {
    let bundles = Bundles::load().unwrap();
    for key in bundles.keys(Lang::En) {
        assert!(bundles.has_message(Lang::Pl, key), "{key}");
    }
    for key in bundles.keys(Lang::Pl) {
        assert!(bundles.has_message(Lang::En, key), "{key}");
    }
    assert!(!bundles.keys(Lang::En).is_empty());
}

/// The registry table of `reliability.md` section 1.2 (task 6.4, design D13).
const REGISTRY: &str = include_str!("../../../../docs/architecture/reliability.md");

/// The codes whose message a route of this change can return (`api.md` section 5).
///
/// `E-IDN-007` is left out, because it gives a redirect and no message.
fn codes_with_a_message() -> Vec<String> {
    let idn = (1..=10)
        .filter(|n| *n != 7)
        .map(|n| format!("E-IDN-{n:03}"));
    let web = (1..=11).map(|n| format!("E-WEB-{n:03}"));
    let db = (1..=8).map(|n| format!("E-DB-{n:03}"));
    idn.chain(web)
        .chain(db)
        .chain(std::iter::once("E-AUD-001".to_owned()))
        .collect()
}

/// Return the user message of a code from its row of the registry, without the reference.
///
/// The parenthetical note of a row (for example the note of `E-IDN-003`) is not part of the text.
fn registry_message(code: &str) -> Option<&'static str> {
    REGISTRY
        .lines()
        .filter(|line| line.starts_with("| `E-"))
        .find_map(|line| {
            let cells: Vec<&'static str> = line.split('|').map(str::trim).collect();
            let [_, row_code, _cause, _status, message, ..] = cells.as_slice() else {
                return None;
            };
            let text = message.split(" (").next()?;
            (row_code.trim_matches('`') == code).then_some(text)
        })
}

/// Turn a registry text into the `fluent` text: each `{name}` becomes `{ $name }`.
///
/// The sentence "Reference: {request_id}." is left out, because the page adds it.
fn fluent_text(registry: &str) -> String {
    let text = registry
        .strip_suffix(" Reference: {request_id}.")
        .unwrap_or(registry);
    let mut out = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once('{') {
        let Some((name, tail)) = after.split_once('}') else {
            break;
        };
        out.push_str(before);
        out.push_str("{ $");
        out.push_str(name);
        out.push_str(" }");
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Return the text of the message `key` in a `.ftl` source.
fn ftl_message<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    source
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(" = "))
}

#[test]
fn i18n_messages_english_text_equals_its_registry_row() {
    let codes = codes_with_a_message()
        .into_iter()
        .filter(|code| code.as_str() != "E-IDN-004");
    for code in codes {
        let expected = fluent_text(registry_message(&code).unwrap_or("<no registry row>"));
        let actual = ftl_message(EN_SOURCE, &format!("error-{code}"));
        assert_eq!(actual, Some(expected.as_str()), "{code}");
    }
}

#[test]
fn i18n_messages_polish_has_a_text_for_each_code() {
    for code in codes_with_a_message() {
        let key = format!("error-{code}");
        assert!(ftl_message(PL_SOURCE, &key).is_some(), "{key} in pl.ftl");
        assert!(ftl_message(EN_SOURCE, &key).is_some(), "{key} in en.ftl");
    }
}

#[test]
fn i18n_messages_e_idn_004_shares_the_text_of_e_idn_003() {
    for source in [EN_SOURCE, PL_SOURCE] {
        assert_eq!(
            ftl_message(source, "error-E-IDN-004"),
            ftl_message(source, "error-E-IDN-003")
        );
    }
}
