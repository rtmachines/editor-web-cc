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

/// Two request structs with a `role` field each. Two functions read `role` of `A` only.
const TWO_ROLE_STRUCTS: &str = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct A {\n    role: String,\n}\n#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct B {\n    role: String,\n}\nfn first(form: A) {\n    let role = Role::parse(&form.role);\n}\nfn second(form: A) {\n    let role = Role::parse(&form.role);\n}\n";

/// The entries of `TWO_ROLE_STRUCTS`: both `role` fields use `Role::parse`.
const TWO_ROLE_TABLE: &[Entry] = &[
    (
        "pages/form.rs",
        "A",
        "role",
        Cover::NoText(Kind::Parsed("Role::parse")),
    ),
    (
        "pages/form.rs",
        "B",
        "role",
        Cover::NoText(Kind::Parsed("Role::parse")),
    ),
];

#[test]
fn parsed_entry_needs_its_own_parse_window() {
    let faults = faults_of(TWO_ROLE_STRUCTS, TWO_ROLE_TABLE);

    assert_eq!(
        faults,
        [
            "the entry pages/form.rs B.role has the kind Parsed, but no read of `.role` has `Role::parse` within three lines and no other field"
        ]
    );
}

#[test]
fn parsed_entries_of_two_structs_pass_with_one_window_each() {
    let text = TWO_ROLE_STRUCTS.replace("fn second(form: A)", "fn second(form: B)");

    assert_eq!(faults_of(&text, TWO_ROLE_TABLE), Vec::<String>::new());
}

#[test]
fn parsed_entry_does_not_count_a_window_of_a_function_that_names_both_structs() {
    let text = TWO_ROLE_STRUCTS.replace("fn second(form: A)", "fn second(form: A, other: B)");

    let faults = faults_of(&text, TWO_ROLE_TABLE);

    assert_eq!(faults.len(), 1, "{faults:?}");
}

#[test]
fn parsed_entry_counts_a_window_in_a_method_of_its_impl_block() {
    let text = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct A {\n    role: String,\n}\n#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct B {\n    role: String,\n}\nimpl A {\n    fn check(&self) {\n        let role = Role::parse(&self.role);\n    }\n}\nimpl B {\n    fn check(&self) {\n        let role = Role::parse(&self.role);\n    }\n}\n";

    assert_eq!(faults_of(text, TWO_ROLE_TABLE), Vec::<String>::new());
}

#[test]
fn parsed_entry_refuses_a_parse_call_in_the_next_function() {
    let text = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    role: String,\n}\nfn read(form: Form) {\n    let role = &form.role;\n}\nfn convert(text: &str) {\n    let role = Role::parse(text);\n}\n";
    let table: &[Entry] = &[(
        "pages/form.rs",
        "Form",
        "role",
        Cover::NoText(Kind::Parsed("Role::parse")),
    )];

    let faults = faults_of(text, table);

    assert_eq!(faults.len(), 1, "{faults:?}");
}

#[test]
fn xss_coverage_accepts_the_parsed_kind_on_the_list_query_page() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pages/users/query.rs");
    let text = std::fs::read_to_string(path).unwrap();
    let sources = [
        source("pages/users/query.rs", &text),
        source(
            "pages/users/query_tests.rs",
            "#[test]\nfn xss_covers_search() {\n    get(&format!(\"/users?q={payload}\"));\n}",
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

/// A request struct with the unread field `_note`, and a handler text that follows the struct.
fn unread_note_module(handler: &str) -> String {
    format!(
        "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {{\n    _note: String,\n}}\n{handler}"
    )
}

/// The entry that gives the field `_note` of `Form` the kind `Unread`.
const UNREAD_NOTE_TABLE: &[Entry] = &[(
    "pages/form.rs",
    "Form",
    "_note",
    Cover::NoText(Kind::Unread),
)];

/// The fault of the entry `UNREAD_NOTE_TABLE` for a module that reads `_note`.
const UNREAD_NOTE_FAULT: &str =
    "the entry pages/form.rs Form._note has the kind Unread, but the module reads `_note`";

#[test]
fn unread_field_with_a_read_of_its_name_fails() {
    let text = unread_note_module("fn handle(form: Form) {\n    let note = form._note;\n}\n");

    assert_eq!(faults_of(&text, UNREAD_NOTE_TABLE), [UNREAD_NOTE_FAULT]);
}

#[test]
fn unread_field_read_after_a_slash_slash_inside_a_string_fails() {
    let text = unread_note_module(
        "fn handle(form: Form) {\n    let u = \"http://h\"; let n = form._note;\n}\n",
    );

    assert_eq!(faults_of(&text, UNREAD_NOTE_TABLE), [UNREAD_NOTE_FAULT]);
}

#[test]
fn unread_field_read_after_a_slash_slash_inside_a_char_literal_fails() {
    let text = unread_note_module(
        "fn handle(form: Form) {\n    let c = '\"'; let u = \"//\"; let n = form._note;\n}\n",
    );

    assert_eq!(faults_of(&text, UNREAD_NOTE_TABLE), [UNREAD_NOTE_FAULT]);
}

#[test]
fn unread_field_in_a_real_comment_after_a_string_passes() {
    let handler = "fn handle(form: Form) {\n    let u = \"http://h\"; // form._note\n}\n";

    assert_eq!(
        faults_of(&unread_note_module(handler), UNREAD_NOTE_TABLE),
        Vec::<String>::new()
    );
}

#[test]
fn unread_field_with_a_read_through_self_fails() {
    let text = unread_note_module(
        "impl Form {\n    fn note(&self) -> &str {\n        &self._note\n    }\n}\n",
    );

    assert_eq!(faults_of(&text, UNREAD_NOTE_TABLE), [UNREAD_NOTE_FAULT]);
}

#[test]
fn unread_field_in_a_destructure_fails() {
    let text =
        unread_note_module("fn handle(form: Form) {\n    let Form { _note: n, .. } = form;\n}\n");

    assert_eq!(faults_of(&text, UNREAD_NOTE_TABLE), [UNREAD_NOTE_FAULT]);
}

#[test]
fn unread_field_in_a_shorthand_destructure_fails() {
    let text =
        unread_note_module("fn handle(form: Form) {\n    let Form { _note, .. } = form;\n}\n");

    assert_eq!(faults_of(&text, UNREAD_NOTE_TABLE), [UNREAD_NOTE_FAULT]);
}

#[test]
fn unread_field_passes_when_its_name_is_only_declared_or_quoted() {
    let handler =
        "fn handle(form: Form) {\n    // form._note is not read\n    let wire = \"_note\";\n}\n";

    assert_eq!(
        faults_of(&unread_note_module(handler), UNREAD_NOTE_TABLE),
        Vec::<String>::new()
    );
}

/// A request struct with the free text field `note`.
const NOTE_REQUEST: &str =
    "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    note: String,\n}\n";

/// The entry that covers the field `note` with the corpus test `xss_covers_text`.
const NOTE_CORPUS_TABLE: &[Entry] = &[(
    "pages/form.rs",
    "Form",
    "note",
    Cover::Corpus("xss_covers_text"),
)];

/// The fault of the entry `NOTE_CORPUS_TABLE` for a test that does not submit `note`.
const NOTE_NOT_SUBMITTED: &str =
    "the corpus test of pages/form.rs Form.note does not submit the field: xss_covers_text";

/// The faults of `NOTE_CORPUS_TABLE` for a test file with the given text.
fn faults_of_note_test(tests: &str) -> Vec<String> {
    let sources = [
        source("pages/form.rs", NOTE_REQUEST),
        source("pages/form_tests.rs", tests),
    ];
    coverage_faults(&sources, NOTE_CORPUS_TABLE)
}

#[test]
fn corpus_test_must_name_the_field() {
    let tests = "#[test]\nfn xss_covers_text() {\n    let reply = post(\"/form\", \"other=1\");\n    assert!(reply.ok, \"note\");\n}\n";

    assert_eq!(faults_of_note_test(tests), [NOTE_NOT_SUBMITTED]);
}

#[test]
fn corpus_test_refuses_a_name_in_an_assertion_macro_or_function() {
    let message =
        "#[test]\nfn xss_covers_text() {\n    assert_eq!(reply.status, 200, \"note\");\n}\n";
    let helper = "#[test]\nfn xss_covers_text() {\n    assert_escaped(&reply, \"note\");\n}\n";

    assert_eq!(faults_of_note_test(message), [NOTE_NOT_SUBMITTED]);
    assert_eq!(faults_of_note_test(helper), [NOTE_NOT_SUBMITTED]);
}

#[test]
fn corpus_test_refuses_a_name_only_in_a_comment() {
    let tests = "#[test]\nfn xss_covers_text() {\n    // The field note=x\n    let reply = post(\"/form\", \"other=1\");\n}\n";

    assert_eq!(faults_of_note_test(tests), [NOTE_NOT_SUBMITTED]);
}

#[test]
fn corpus_test_refuses_a_name_that_only_contains_the_field_name() {
    let tests = "#[test]\nfn xss_covers_text() {\n    let reply = post(\"/form\", \"footnote=1&note_id=2\");\n}\n";

    assert_eq!(faults_of_note_test(tests), [NOTE_NOT_SUBMITTED]);
}

#[test]
fn corpus_test_accepts_a_form_key_a_pair_or_a_struct_literal() {
    let bodies = [
        "let reply = post(\"/form\", &format!(\"a=1&note={payload}\"));",
        "let reply = post_pairs(\"/form\", &[(\"note\", payload)]);",
        "let reply = post_form(&Form { note: payload });",
    ];
    for body in bodies {
        let tests = format!("#[test]\nfn xss_covers_text() {{\n    {body}\n}}\n");

        assert_eq!(faults_of_note_test(&tests), Vec::<String>::new(), "{body}");
    }
}

#[test]
fn corpus_test_accepts_a_name_in_a_helper_or_a_const_of_its_file() {
    let tests = "const FIELDS: [&str; 1] = [\"note\"];\nfn send(field: &str) {\n    post(\"/form\", field);\n}\n#[test]\nfn xss_covers_text() {\n    for field in FIELDS {\n        send(field);\n    }\n}\n";
    let in_helper = "fn send(payload: &str) {\n    post(\"/form\", &format!(\"note={payload}\"));\n}\n#[test]\nfn xss_covers_text() {\n    send(\"x\");\n}\n";

    assert_eq!(faults_of_note_test(tests), Vec::<String>::new());
    assert_eq!(faults_of_note_test(in_helper), Vec::<String>::new());
}

#[test]
fn corpus_test_does_not_follow_a_function_of_another_file() {
    let sources = [
        source("pages/form.rs", NOTE_REQUEST),
        source(
            "pages/form_tests.rs",
            "#[test]\nfn xss_covers_text() {\n    send();\n}\n",
        ),
        source(
            "pages/other_tests.rs",
            "fn send() {\n    post(\"/form\", &format!(\"note={payload}\"));\n}\n",
        ),
    ];

    assert_eq!(
        coverage_faults(&sources, NOTE_CORPUS_TABLE),
        [NOTE_NOT_SUBMITTED]
    );
}

/// A request struct with the free text fields `login` and `password`.
const LOGIN_REQUEST: &str = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    login: String,\n    password: String,\n}\n";

/// The entries that cover `login` and `password` with the corpus test `xss_covers_login`.
const LOGIN_CORPUS_TABLE: &[Entry] = &[
    (
        "pages/form.rs",
        "Form",
        "login",
        Cover::Corpus("xss_covers_login"),
    ),
    (
        "pages/form.rs",
        "Form",
        "password",
        Cover::Corpus("xss_covers_login"),
    ),
];

/// The fault of the entry `password` of `LOGIN_CORPUS_TABLE`.
const PASSWORD_NOT_SUBMITTED: &str =
    "the corpus test of pages/form.rs Form.password does not submit the field: xss_covers_login";

/// The faults of `LOGIN_CORPUS_TABLE` for a corpus test with the given body.
fn faults_of_login_test(body: &str) -> Vec<String> {
    let tests = format!("#[test]\nfn xss_covers_login() {{\n    {body}\n}}\n");
    let sources = [
        source("pages/form.rs", LOGIN_REQUEST),
        source("pages/form_tests.rs", &tests),
    ];
    coverage_faults(&sources, LOGIN_CORPUS_TABLE)
}

#[test]
fn corpus_test_refuses_a_field_that_only_holds_a_constant() {
    let body = "let reply = post(\"/form\", &format!(\"login={payload}&password=wrong\"));";

    assert_eq!(faults_of_login_test(body), [PASSWORD_NOT_SUBMITTED]);
}

#[test]
fn corpus_test_accepts_a_field_that_holds_the_corpus_value() {
    let bodies = [
        "let reply = post(\"/form\", &format!(\"login={p}&password={payload}\"));",
        "let reply = post(\"/form\", &format!(\"login={}&password={}\", p, q));",
        "let reply = post(\"/form\", &[\"login=\", &p, \"&password=\", &q].concat());",
    ];
    for body in bodies {
        assert_eq!(faults_of_login_test(body), Vec::<String>::new(), "{body}");
    }
}

#[test]
fn corpus_test_refuses_a_struct_key_with_a_literal_value() {
    let body = "let reply = post_form(&Form { login: payload, password: \"wrong\" });";

    assert_eq!(faults_of_login_test(body), [PASSWORD_NOT_SUBMITTED]);
}

#[test]
fn corpus_test_refuses_a_name_that_a_probe_reads_outside_an_assertion() {
    let body = "let reply = post(\"/form\", &format!(\"login={payload}\"));\n    let shown = reply.body.contains(\"password\");";

    assert_eq!(faults_of_login_test(body), [PASSWORD_NOT_SUBMITTED]);
}
