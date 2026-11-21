use super::*;

/// The faults of one production source for a table of the given entries.
fn faults_of(text: &str, table: &[Entry]) -> Vec<String> {
    coverage_faults(&[source("pages/form.rs", text)], table)
}

/// A request struct with a `String` field and a `bool` field. The module reads them four
/// lines apart.
const PARSED_REQUEST: &str = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    role: String,\n    flag: bool,\n}\nfn handle(form: Form) {\n    let role = Role::parse(&form.role);\n    // 1\n    // 2\n    // 3\n    let flag = form.flag;\n}\n";

/// A request struct that a module reads through a parse function of another field only.
const OTHER_FIELD_PARSED: &str = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    count: u8,\n    role: String,\n}\nfn handle(form: Form) {\n    let count = Role::parse(&form.count);\n    let role = Other::parse(&form.role);\n}\n";

#[test]
fn xss_coverage_accepts_the_parsed_kind_on_the_list_query_page() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pages/users/query.rs");
    let text = std::fs::read_to_string(path).unwrap();
    let sources = [
        source("pages/users/query.rs", &text),
        source(
            "pages/users/query_tests.rs",
            "#[test]\nfn xss_covers_search() {}",
        ),
    ];
    let table: &[Entry] = &[
        (
            "pages/users/query.rs",
            "ListQuery",
            "q",
            Cover::Corpus("xss_covers_search"),
        ),
        (
            "pages/users/query.rs",
            "ListQuery",
            "page",
            Cover::NoText(Kind::Parsed("path_values::page")),
        ),
    ];

    assert_eq!(coverage_faults(&sources, table), Vec::<String>::new());
}

#[test]
fn xss_coverage_refuses_the_typed_kind_on_a_string_field() {
    let table: &[Entry] = &[("pages/form.rs", "Form", "text", Cover::NoText(Kind::Typed))];

    let faults = faults_of(REQUEST, table);

    assert_eq!(
        faults,
        [
            "the entry pages/form.rs Form.text has the kind Typed, but its type `String` is no integer or bool"
        ]
    );
}

#[test]
fn xss_coverage_refuses_a_parsed_kind_that_the_module_does_not_call_for_the_field() {
    let table: &[Entry] = &[
        ("pages/form.rs", "Form", "count", Cover::NoText(Kind::Typed)),
        (
            "pages/form.rs",
            "Form",
            "role",
            Cover::NoText(Kind::Parsed("Role::parse")),
        ),
    ];

    let faults = faults_of(OTHER_FIELD_PARSED, table);

    assert_eq!(
        faults,
        [
            "the entry pages/form.rs Form.role has the kind Parsed, but no read of `.role` has `Role::parse` within three lines and no other field"
        ]
    );
}

#[test]
fn xss_coverage_refuses_the_unread_kind_on_a_named_field() {
    let table: &[Entry] = &[("pages/form.rs", "Form", "text", Cover::NoText(Kind::Unread))];

    let faults = faults_of(REQUEST, table);

    assert_eq!(
        faults,
        [
            "the entry pages/form.rs Form.text has the kind Unread, but its Rust name `text` does not start with `_`"
        ]
    );
}

#[test]
fn xss_coverage_accepts_the_unread_secret_and_typed_kinds_on_a_matching_field() {
    let text = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    #[serde(rename = \"csrf_token\")]\n    _csrf_token: Option<String>,\n    pub(super) password: Option<SecretField>,\n    flag: Option<bool>,\n}\n";
    let table: &[Entry] = &[
        (
            "pages/form.rs",
            "Form",
            "csrf_token",
            Cover::NoText(Kind::Unread),
        ),
        (
            "pages/form.rs",
            "Form",
            "password",
            Cover::NoText(Kind::Secret),
        ),
        ("pages/form.rs", "Form", "flag", Cover::NoText(Kind::Typed)),
    ];

    assert_eq!(faults_of(text, table), Vec::<String>::new());
}

#[test]
fn xss_coverage_refuses_the_secret_kind_on_a_string_field() {
    let table: &[Entry] = &[("pages/form.rs", "Form", "text", Cover::NoText(Kind::Secret))];

    let faults = faults_of(REQUEST, table);

    assert_eq!(
        faults,
        [
            "the entry pages/form.rs Form.text has the kind Secret, but its type `String` is no SecretField"
        ]
    );
}

#[test]
fn xss_coverage_accepts_a_parsed_kind_that_the_module_calls_next_to_a_read_of_the_field() {
    let table: &[Entry] = &[
        (
            "pages/form.rs",
            "Form",
            "role",
            Cover::NoText(Kind::Parsed("Role::parse")),
        ),
        ("pages/form.rs", "Form", "flag", Cover::NoText(Kind::Typed)),
    ];

    assert_eq!(faults_of(PARSED_REQUEST, table), Vec::<String>::new());
}

#[test]
fn xss_coverage_refuses_a_parsed_kind_when_another_field_is_read_in_the_window() {
    let table: &[Entry] = &[
        (
            "pages/form.rs",
            "Form",
            "role",
            Cover::NoText(Kind::Parsed("Role::parse")),
        ),
        ("pages/form.rs", "Form", "flag", Cover::NoText(Kind::Typed)),
    ];
    let text = PARSED_REQUEST.replace(
        "Role::parse(&form.role);",
        "Role::parse(&form.role, form.flag);",
    );

    let faults = faults_of(&text, table);

    assert_eq!(faults.len(), 1, "{faults:?}");
}
