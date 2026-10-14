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

/// The texts of the design table of this change: key, English text, Polish text.
const DESIGN_TEXTS: &[(&str, &str, &str)] = &[
    ("app-name", "Recipe editor", "Edytor receptur"),
    (
        "skip-to-content",
        "Skip to the content",
        "Przejdź do treści",
    ),
    ("nav-label", "Main", "Główna"),
    ("nav-recipes", "Recipes", "Receptury"),
    ("nav-users", "Users", "Użytkownicy"),
    ("nav-me", "My account", "Moje konto"),
    ("lang-switch-label", "Language", "Język"),
    ("lang-switch-pl", "Polski", "Polski"),
    ("lang-switch-en", "English", "English"),
    ("user-signed-in", "Signed in as", "Zalogowano jako"),
    ("recipes-title", "Recipes", "Receptury"),
    (
        "recipes-empty",
        "No recipe to show.",
        "Brak receptur do pokazania.",
    ),
    ("status-draft", "Draft", "Szkic"),
    ("status-approved", "Approved", "Zatwierdzona"),
    ("status-retired", "Retired", "Wycofana"),
    ("form-required", "required", "wymagane"),
    (
        "error-page-title",
        "The request failed",
        "Żądanie nie powiodło się",
    ),
    (
        "error-page-home",
        "Open the recipe list",
        "Otwórz listę receptur",
    ),
    ("field-form", "Form", "Formularz"),
    ("field-lang", "Language", "Język"),
    ("field-login", "Login", "Login"),
    ("field-password", "Password", "Hasło"),
    ("field-current-password", "Current password", "Obecne hasło"),
    ("field-role", "Role", "Rola"),
    ("field-query", "Search fields", "Pola wyszukiwania"),
    ("field-q", "Login contains", "Login zawiera"),
    ("field-page", "Page", "Strona"),
];

#[test]
fn i18n_texts_of_the_design_table_exist_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for &(key, en, pl) in DESIGN_TEXTS {
        assert_eq!(
            bundles.message(Lang::En, key, None).as_deref(),
            Some(en),
            "{key} in en"
        );
        assert_eq!(
            bundles.message(Lang::Pl, key, None).as_deref(),
            Some(pl),
            "{key} in pl"
        );
    }
}

/// The texts of the error value tables of design D13: key, English text, Polish text.
const ERROR_VALUE_TEXTS: [(&str, &str, &str); 92] = [
    (
        "reason-form",
        "the form has a missing field, an unknown field, or a wrong content type",
        "w formularzu brakuje pola, jest nieznane pole albo zły typ treści",
    ),
    (
        "reason-lang",
        "the language must be pl or en",
        "język musi być pl albo en",
    ),
    (
        "reason-login-length",
        "a login has 1 to 64 characters",
        "login ma od 1 do 64 znaków",
    ),
    (
        "reason-login-control",
        "a login holds no control character",
        "login nie zawiera znaku sterującego",
    ),
    (
        "reason-login-grammar",
        "a login is 3 to 64 characters from a to z, 0 to 9, period, underscore, and hyphen",
        "login ma od 3 do 64 znaków: litery a–z, cyfry 0–9, kropka, podkreślnik i łącznik",
    ),
    (
        "reason-password-length",
        "a password has 1 to 128 characters",
        "hasło ma od 1 do 128 znaków",
    ),
    (
        "reason-role",
        "choose a role from the list",
        "wybierz rolę z listy",
    ),
    (
        "reason-query",
        "the address has an unknown parameter or a parameter that is not text",
        "adres ma nieznany parametr albo parametr, który nie jest tekstem",
    ),
    (
        "reason-q",
        "the search text has at most 64 characters from a to z, 0 to 9, period, underscore, and hyphen",
        "tekst wyszukiwania ma najwyżej 64 znaki: litery a–z, cyfry 0–9, kropka, podkreślnik i łącznik",
    ),
    (
        "reason-page",
        "the page is a whole number from 1",
        "numer strony to liczba całkowita od 1",
    ),
    (
        "reason-forwarded",
        "every entry must be an IP address",
        "każdy wpis musi być adresem IP",
    ),
    ("missing-machine", "Machine { $code }", "Maszyna { $code }"),
    ("missing-machine-id", "Machine { $id }", "Maszyna { $id }"),
    (
        "missing-unit",
        "Unit { $path } of machine { $code }",
        "Jednostka { $path } maszyny { $code }",
    ),
    (
        "missing-type",
        "Type { $name } of machine { $code }",
        "Typ { $name } maszyny { $code }",
    ),
    (
        "missing-recipe",
        "Recipe { $reference }",
        "Receptura { $reference }",
    ),
    (
        "missing-version",
        "Version { $number } of recipe { $reference }",
        "Wersja { $number } receptury { $reference }",
    ),
    ("missing-user", "User { $login }", "Użytkownik { $login }"),
    ("missing-malformed-login", "User", "Użytkownik"),
    ("password-reason-empty", "it is empty", "jest puste"),
    (
        "password-reason-too-short",
        "it has fewer than 12 characters",
        "ma mniej niż 12 znaków",
    ),
    (
        "password-reason-too-long",
        "it has more than 128 characters",
        "ma więcej niż 128 znaków",
    ),
    (
        "password-reason-breached",
        "it is in the breached-password list",
        "jest na liście haseł, które wyciekły",
    ),
    (
        "task-stage-password-hash",
        "password hash",
        "obliczenie skrótu hasła",
    ),
    ("task-stage-validation", "validation", "walidacja"),
    ("task-stage-export", "export", "eksport"),
    ("task-stage-token", "token generation", "generowanie tokenu"),
    (
        "operation-create-machine",
        "Create machine",
        "Utworzenie maszyny",
    ),
    (
        "operation-rename-machine",
        "Rename machine",
        "Zmiana nazwy maszyny",
    ),
    (
        "operation-archive-machine",
        "Archive machine",
        "Archiwizacja maszyny",
    ),
    ("operation-add-unit", "Add unit", "Dodanie jednostki"),
    (
        "operation-rename-unit",
        "Rename unit",
        "Zmiana nazwy jednostki",
    ),
    (
        "operation-archive-unit",
        "Archive unit",
        "Archiwizacja jednostki",
    ),
    (
        "operation-define-udt",
        "Define UDT",
        "Zdefiniowanie typu UDT",
    ),
    (
        "operation-create-recipe",
        "Create recipe",
        "Utworzenie receptury",
    ),
    (
        "operation-rename-recipe",
        "Rename recipe",
        "Zmiana nazwy receptury",
    ),
    (
        "operation-delete-recipe",
        "Delete recipe",
        "Usunięcie receptury",
    ),
    (
        "operation-restore-recipe",
        "Restore recipe",
        "Przywrócenie receptury",
    ),
    (
        "operation-create-version",
        "Create version",
        "Utworzenie wersji",
    ),
    (
        "operation-discard-draft",
        "Discard draft",
        "Odrzucenie szkicu",
    ),
    (
        "operation-approve-version",
        "Approve version",
        "Zatwierdzenie wersji",
    ),
    (
        "operation-retire-version",
        "Retire version",
        "Wycofanie wersji",
    ),
    (
        "operation-add-parameter",
        "Add parameter",
        "Dodanie parametru",
    ),
    (
        "operation-update-parameter-definition",
        "Change parameter definition",
        "Zmiana definicji parametru",
    ),
    (
        "operation-set-parameter-value",
        "Set parameter value",
        "Ustawienie wartości parametru",
    ),
    (
        "operation-replace-limits",
        "Replace limits",
        "Zastąpienie limitów",
    ),
    (
        "operation-upsert-translation",
        "Save translation",
        "Zapisanie tłumaczenia",
    ),
    (
        "operation-remove-translation",
        "Delete translation",
        "Usunięcie tłumaczenia",
    ),
    (
        "operation-remove-parameter",
        "Delete parameter",
        "Usunięcie parametru",
    ),
    (
        "operation-rebind-type-revision",
        "Rebind type revision",
        "Przepięcie rewizji typu",
    ),
    ("operation-login", "Log in", "Logowanie"),
    ("operation-logout", "Log out", "Wylogowanie"),
    (
        "operation-create-user",
        "Create user",
        "Utworzenie użytkownika",
    ),
    (
        "operation-set-user-role",
        "Set user role",
        "Ustawienie roli użytkownika",
    ),
    (
        "operation-deactivate-user",
        "Deactivate user",
        "Dezaktywacja użytkownika",
    ),
    (
        "operation-activate-user",
        "Activate user",
        "Aktywacja użytkownika",
    ),
    ("operation-reset-password", "Reset password", "Reset hasła"),
    (
        "operation-change-own-password",
        "Change own password",
        "Zmiana własnego hasła",
    ),
    (
        "operation-unlock-login",
        "Unlock login",
        "Odblokowanie logowania",
    ),
    (
        "operation-export-version",
        "Export version",
        "Eksport wersji",
    ),
    ("operation-list-machines", "List machines", "Lista maszyn"),
    ("operation-get-machine", "Show machine", "Podgląd maszyny"),
    (
        "operation-get-unit-tree",
        "Show unit tree",
        "Podgląd drzewa jednostek",
    ),
    ("operation-get-unit", "Show unit", "Podgląd jednostki"),
    ("operation-list-types", "List types", "Lista typów"),
    (
        "operation-get-type-tree",
        "Show type tree",
        "Podgląd drzewa typu",
    ),
    (
        "operation-get-type-usage",
        "Show type usage",
        "Podgląd użycia typu",
    ),
    ("operation-list-recipes", "List recipes", "Lista receptur"),
    ("operation-get-recipe", "Show recipe", "Podgląd receptury"),
    ("operation-list-versions", "List versions", "Lista wersji"),
    ("operation-get-version", "Show version", "Podgląd wersji"),
    (
        "operation-diff-versions",
        "Compare versions",
        "Porównanie wersji",
    ),
    (
        "operation-get-recipe-history",
        "Show recipe history",
        "Podgląd historii receptury",
    ),
    (
        "operation-list-parameters",
        "List parameters",
        "Lista parametrów",
    ),
    (
        "operation-get-parameter",
        "Show parameter",
        "Podgląd parametru",
    ),
    ("operation-get-limits", "Show limits", "Podgląd limitów"),
    (
        "operation-get-translations",
        "Show translations",
        "Podgląd tłumaczeń",
    ),
    ("operation-list-users", "List users", "Lista użytkowników"),
    ("operation-get-user", "Show user", "Podgląd użytkownika"),
    (
        "operation-list-audit-entries",
        "List audit entries",
        "Lista wpisów audytu",
    ),
    (
        "operation-get-audit-entry",
        "Show audit entry",
        "Podgląd wpisu audytu",
    ),
    (
        "operation-list-security-events",
        "List security events",
        "Lista zdarzeń bezpieczeństwa",
    ),
    (
        "operation-get-own-profile",
        "Show own profile",
        "Podgląd własnego profilu",
    ),
    (
        "operation-get-machine-form",
        "Open machine form",
        "Formularz maszyny",
    ),
    (
        "operation-get-unit-form",
        "Open unit form",
        "Formularz jednostki",
    ),
    (
        "operation-get-udt-form",
        "Open UDT form",
        "Formularz typu UDT",
    ),
    (
        "operation-get-recipe-form",
        "Open recipe form",
        "Formularz receptury",
    ),
    (
        "operation-get-version-form",
        "Open version form",
        "Formularz wersji",
    ),
    (
        "operation-get-parameter-form",
        "Open parameter form",
        "Formularz parametru",
    ),
    (
        "operation-get-approval-preview",
        "Show approval preview",
        "Podgląd przed zatwierdzeniem",
    ),
    (
        "operation-get-user-form",
        "Open user form",
        "Formularz użytkownika",
    ),
    (
        "operation-get-login-form",
        "Open login form",
        "Formularz logowania",
    ),
];

#[test]
fn i18n_texts_of_the_error_value_tables_exist_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for &(key, en, pl) in ERROR_VALUE_TEXTS.iter() {
        if en.contains('{') || pl.contains('{') {
            assert!(bundles.has_message(Lang::En, key), "{key} in en");
            assert!(bundles.has_message(Lang::Pl, key), "{key} in pl");
            continue;
        }
        assert_eq!(
            bundles.message(Lang::En, key, None).as_deref(),
            Some(en),
            "{key} in en"
        );
        assert_eq!(
            bundles.message(Lang::Pl, key, None).as_deref(),
            Some(pl),
            "{key} in pl"
        );
    }
}
