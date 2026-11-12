//! The table of the XSS corpus coverage: the cover of each field of a request struct.

/// What covers one field of a request struct.
#[derive(Clone, Copy, Debug)]
pub(super) enum Cover {
    /// The name of the corpus test that sends the payloads through the field.
    Corpus(&'static str),
    /// The reason that the field carries no free text.
    NoText(&'static str),
}

/// One entry of the table: the file below `src/`, the struct, the wire name, and the cover.
pub(super) type Entry = (&'static str, &'static str, &'static str, Cover);

const CSRF: Cover = Cover::NoText(
    "The CSRF layer compares the token before the handler runs. No handler reads or shows it.",
);
const NUMBER: Cover =
    Cover::NoText("A number or a flag. A bad text gives a form error without the sent text.");
const LANG: Cover = Cover::NoText(
    "A closed set of two language codes. The error line does not show the sent text.",
);
const ROLE: Cover =
    Cover::NoText("A closed set of role names. The form never shows the sent text of this field.");
const FILTERS: Cover =
    Cover::Corpus("approval_xss_filter_values_that_no_payload_can_satisfy_give_e_web_008");
const PLANT_GETS: Cover =
    Cover::Corpus("plant_xss_query_values_and_path_values_of_get_pages_give_their_error_code");
const RECIPE_GETS: Cover =
    Cover::Corpus("recipe_xss_query_values_of_get_pages_give_their_error_code");
const RECIPE_REFUSED: Cover =
    Cover::Corpus("recipe_xss_fields_that_the_server_refuses_give_their_error_code");
const RECIPE_TEXTS: Cover =
    Cover::Corpus("recipe_xss_text_fields_accept_a_valid_text_and_refuse_a_control_character");
const PLANT_REFUSED: Cover =
    Cover::Corpus("plant_xss_machine_fields_that_the_server_refuses_give_their_error_code");
const PLANT_MACHINE_NAMES: Cover =
    Cover::Corpus("plant_xss_machine_names_are_stored_and_shown_escaped");
const PLANT_RENAME: Cover =
    Cover::Corpus("plant_xss_rename_name_is_shown_escaped_in_the_answer_and_on_the_page");
const PLANT_DATA_BLOCK: Cover = Cover::Corpus(
    "plant_xss_data_block_number_payloads_give_their_error_code_and_show_escaped_in_the_form",
);
const PLANT_UNIT_REFUSED: Cover =
    Cover::Corpus("plant_xss_unit_fields_that_the_server_refuses_give_their_error_code");

const PLC_NUMBER: Cover = Cover::NoText(
    "A number. A bad text gives a form error without the sent text. The form shows a number only.",
);
const PLC_GRAMMAR: Cover = Cover::NoText(
    "A value object with a closed grammar parses it. A bad text gives a form error without the sent text.",
);
const PLC_HOST_TEXT: Cover =
    Cover::Corpus("connection_xss_host_payloads_are_refused_and_shown_escaped_in_the_form");
const PLC_FREE_TEXT: Cover =
    Cover::Corpus("connection_xss_address_and_login_are_stored_and_shown_escaped_on_both_pages");
const PLC_SECRET: Cover = Cover::NoText(
    "The password is never shown, echoed, or logged. The form input has no value attribute.",
);

/// Each field of each `Deserialize` struct of `src/`, with the test or the reason that covers it.
pub(super) const TABLE: &[Entry] = &[
    ("lang.rs", "LangForm", "lang", LANG),
    (
        "pages/export_api/connection.rs",
        "ConnectionQuery",
        "recipe",
        Cover::NoText(
            "A reference number parses it. A bad text gives 404 E-WEB-005 as JSON without the sent text.",
        ),
    ),
    (
        "lang.rs",
        "LangForm",
        "next",
        Cover::Corpus("xss_corpus_next_field_of_post_lang_is_escaped"),
    ),
    ("lang.rs", "LangForm", "csrf_pre", CSRF),
    ("lang.rs", "LangForm", "csrf_token", CSRF),
    ("pages/diff.rs", "DiffQuery", "kind", FILTERS),
    ("pages/diff.rs", "DiffQuery", "page", FILTERS),
    ("pages/history.rs", "HistoryQuery", "before", FILTERS),
    (
        "pages/own.rs",
        "LoginFields",
        "login",
        Cover::Corpus("xss_corpus_login_field_of_post_login_is_escaped"),
    ),
    (
        "pages/own.rs",
        "LoginFields",
        "password",
        Cover::Corpus("xss_corpus_password_field_of_post_login_is_escaped"),
    ),
    ("pages/own.rs", "LoginFields", "csrf_pre", CSRF),
    ("pages/own.rs", "LoginFields", "csrf_token", CSRF),
    (
        "pages/own.rs",
        "PasswordFields",
        "current_password",
        Cover::Corpus("xss_corpus_current_password_field_of_post_me_password_is_escaped"),
    ),
    (
        "pages/own.rs",
        "PasswordFields",
        "password",
        Cover::Corpus("xss_corpus_password_field_of_post_me_password_is_escaped"),
    ),
    ("pages/own.rs", "PasswordFields", "csrf_token", CSRF),
    ("pages/security_events.rs", "EventsQuery", "kind", FILTERS),
    (
        "pages/security_events.rs",
        "EventsQuery",
        "login",
        Cover::Corpus(
            "approval_xss_login_filter_of_the_security_events_is_echoed_escaped_or_refused",
        ),
    ),
    (
        "pages/security_events.rs",
        "EventsQuery",
        "client_ip",
        FILTERS,
    ),
    ("pages/security_events.rs", "EventsQuery", "from", FILTERS),
    ("pages/security_events.rs", "EventsQuery", "to", FILTERS),
    ("pages/security_events.rs", "EventsQuery", "before", FILTERS),
    (
        "pages/users.rs",
        "CreateFields",
        "login",
        Cover::Corpus("xss_corpus_login_field_of_post_users_is_escaped"),
    ),
    ("pages/users.rs", "CreateFields", "role", ROLE),
    ("pages/users.rs", "CreateFields", "csrf_token", CSRF),
    ("pages/users.rs", "RoleFields", "role", ROLE),
    ("pages/users.rs", "RoleFields", "csrf_token", CSRF),
    ("pages/users.rs", "TokenFields", "csrf_token", CSRF),
    (
        "pages/approval/approve.rs",
        "ApproveFields",
        "row_version",
        NUMBER,
    ),
    (
        "pages/approval/approve.rs",
        "ApproveFields",
        "allow_outdated",
        NUMBER,
    ),
    (
        "pages/approval/approve.rs",
        "ApproveFields",
        "csrf_token",
        CSRF,
    ),
    ("pages/approval/preview.rs", "PreviewQuery", "kind", FILTERS),
    ("pages/approval/preview.rs", "PreviewQuery", "page", FILTERS),
    ("pages/approval/preview.rs", "PreviewQuery", "rv", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "aggregate", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "entity_type", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "entity_id", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "actor", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "action", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "from", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "to", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "before", FILTERS),
    (
        "pages/machines/commands.rs",
        "NameFields",
        "name",
        PLANT_RENAME,
    ),
    (
        "pages/machines/commands.rs",
        "NameFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "row_version",
        PLC_NUMBER,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "host",
        PLC_HOST_TEXT,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "rack",
        PLC_NUMBER,
    ),
    (
        "pages/machines/data_block_commands.rs",
        "DataBlockFields",
        "db_number",
        PLANT_DATA_BLOCK,
    ),
    (
        "pages/machines/data_block_commands.rs",
        "DataBlockFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "slot",
        PLC_NUMBER,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "port",
        PLC_NUMBER,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "address",
        PLC_FREE_TEXT,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "comm_mode",
        PLC_GRAMMAR,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "login",
        PLC_FREE_TEXT,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "password",
        PLC_SECRET,
    ),
    (
        "pages/machines/connection_request.rs",
        "ConnectionRequest",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "code",
        PLANT_REFUSED,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "name",
        PLANT_MACHINE_NAMES,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "plc_platform",
        PLANT_REFUSED,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "db_number",
        PLANT_REFUSED,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "main_unit_name",
        PLANT_MACHINE_NAMES,
    ),
    ("pages/machines/form.rs", "CreateFields", "csrf_token", CSRF),
    (
        "pages/machines/list.rs",
        "ListQuery",
        "q",
        Cover::Corpus("plant_xss_search_text_of_the_machine_list_is_echoed_escaped"),
    ),
    ("pages/machines/list.rs", "ListQuery", "page", PLANT_GETS),
    (
        "pages/machines/list.rs",
        "ListQuery",
        "show_archived",
        PLANT_GETS,
    ),
    (
        "pages/machines/list.rs",
        "ListQuery",
        "archived",
        PLANT_GETS,
    ),
    (
        "pages/machines/page.rs",
        "CodeQuery",
        "archived",
        PLANT_GETS,
    ),
    ("pages/parameters/new.rs", "NewQuery", "unit", RECIPE_GETS),
    (
        "pages/parameters/table.rs",
        "TableQuery",
        "unit",
        RECIPE_GETS,
    ),
    (
        "pages/parameters/table.rs",
        "TableQuery",
        "page",
        RECIPE_GETS,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "row_version",
        RECIPE_REFUSED,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "name",
        RECIPE_TEXTS,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "description",
        RECIPE_TEXTS,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "RowFields",
        "row_version",
        RECIPE_REFUSED,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "RowFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "machine_code",
        RECIPE_REFUSED,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "reference_number",
        RECIPE_REFUSED,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "name",
        RECIPE_TEXTS,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "change_note",
        RECIPE_TEXTS,
    ),
    ("pages/recipes/form.rs", "CreateFields", "csrf_token", CSRF),
    (
        "pages/recipes/list.rs",
        "ListQuery",
        "q",
        Cover::Corpus("recipe_xss_search_text_of_the_recipe_list_is_echoed_escaped"),
    ),
    ("pages/recipes/list.rs", "ListQuery", "machine", RECIPE_GETS),
    ("pages/recipes/list.rs", "ListQuery", "status", RECIPE_GETS),
    ("pages/recipes/list.rs", "ListQuery", "page", RECIPE_GETS),
    (
        "pages/recipes/list.rs",
        "ListQuery",
        "show_deleted",
        RECIPE_GETS,
    ),
    ("pages/recipes/page.rs", "PageQuery", "page", RECIPE_GETS),
    (
        "pages/recipes/rename.rs",
        "NameFields",
        "name",
        RECIPE_TEXTS,
    ),
    ("pages/recipes/rename.rs", "NameFields", "csrf_token", CSRF),
    ("pages/types/form.rs", "FormQuery", "from", PLANT_GETS),
    ("pages/types/tree.rs", "TypeQuery", "rev", PLANT_GETS),
    ("pages/types/usage.rs", "UsageQuery", "rev", PLANT_GETS),
    ("pages/types/usage.rs", "UsageQuery", "page", PLANT_GETS),
    (
        "pages/units/add.rs",
        "AddFields",
        "parent_path",
        PLANT_UNIT_REFUSED,
    ),
    (
        "pages/units/add.rs",
        "AddFields",
        "segment",
        PLANT_UNIT_REFUSED,
    ),
    (
        "pages/units/add.rs",
        "AddFields",
        "name",
        Cover::Corpus("plant_xss_unit_name_is_shown_escaped_in_the_new_tree"),
    ),
    ("pages/units/add.rs", "AddFields", "csrf_token", CSRF),
    (
        "pages/units/commands.rs",
        "NameFields",
        "name",
        PLANT_RENAME,
    ),
    ("pages/units/commands.rs", "NameFields", "csrf_token", CSRF),
    ("pages/units/form.rs", "NewQuery", "parent_path", PLANT_GETS),
    (
        "pages/users/query.rs",
        "ListQuery",
        "q",
        Cover::Corpus("xss_corpus_search_field_of_get_users_is_escaped"),
    ),
    ("pages/users/query.rs", "ListQuery", "page", NUMBER),
    (
        "pages/versions/actions.rs",
        "VersionedFields",
        "row_version",
        NUMBER,
    ),
    (
        "pages/versions/actions.rs",
        "VersionedFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/versions/compare.rs",
        "CompareQuery",
        "n",
        RECIPE_GETS,
    ),
    (
        "pages/versions/compare.rs",
        "CompareQuery",
        "m",
        RECIPE_GETS,
    ),
    (
        "pages/versions/form.rs",
        "CreateFields",
        "based_on",
        RECIPE_REFUSED,
    ),
    (
        "pages/versions/form.rs",
        "CreateFields",
        "change_note",
        RECIPE_TEXTS,
    ),
    ("pages/versions/form.rs", "CreateFields", "csrf_token", CSRF),
    (
        "pages/versions/page.rs",
        "VersionQuery",
        "unit",
        RECIPE_GETS,
    ),
    (
        "pages/versions/page.rs",
        "VersionQuery",
        "page",
        RECIPE_GETS,
    ),
];
