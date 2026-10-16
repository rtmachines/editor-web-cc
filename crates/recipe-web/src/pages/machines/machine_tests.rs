//! Tests of `GET /machines/{code}` and `GET /machines/by-id/{uuid}` (task 7.3, specs "Machine
//! page", "Content answer of a page", and "Plant forms without user data in an hx attribute").

use axum::http::StatusCode;

use super::test_world::seed_machine_with_archived_unit;
use crate::i18n::Lang;
use crate::pages::test_helpers::{
    assert_clean_html, assert_no_user_text_in_hx_attributes, hx_pairs,
};
use crate::test_support::plant::{ENGLISH, M7_ID, M12_ID, cookie_of, main_of, text_with, world};
use crate::test_support::{icon_tag, text};

/// The opening tag of the rename form (design D6). The fixed `id` serves the out-of-band swap.
const RENAME_TAG: &str = "<form id=\"machine-rename-form\" method=\"post\" action=\"/machines/M7/rename\" hx-boost=\"true\" hx-target=\"#machine-name\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The opening tag of the archive form (design D6).
const ARCHIVE_TAG: &str = "<form method=\"post\" action=\"/machines/M7/archive\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The link "Add a main unit" with its literal `hx-*` attributes (design D6).
const ADD_MAIN_UNIT: &str = "<a class=\"target-link\" href=\"/machines/M7/units/new\" hx-boost=\"true\" hx-target=\"#unit-form\" hx-swap=\"innerHTML\" hx-push-url=\"false\">Add a main unit</a>";

/// The `hx-*` attributes that the machine page may write. Each value is a literal.
const ALLOWED_HX: [(&str, &str); 8] = [
    ("hx-boost", "true"),
    ("hx-target", "#machine-name"),
    ("hx-target", "#content"),
    ("hx-target", "#unit-form"),
    ("hx-swap", "innerHTML"),
    ("hx-push-url", "false"),
    ("hx-push-url", "true"),
    ("hx-sync", "closest form:drop"),
];

/// Return the path of the archived machine `M12` by identifier.
fn archived_path() -> String {
    format!("/machines/by-id/{M12_ID}")
}

/// Return true when `markup` holds a form that sends to a path under `/machines/M7`.
fn has_machine_form(markup: &str) -> bool {
    markup.contains("action=\"/machines/M7")
}

// --- Spec "Machine page": the live machine ------------------------------------------------------

#[tokio::test]
async fn machines_page_of_an_operator_shows_code_name_platform_and_the_tree_without_a_form() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.operator, "/machines/M7", &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let title = text_with(lang, "machine-title", "code", "M7");
        assert!(main.contains(&format!("<h1>{title}</h1>")), "{main}");
        assert!(main.contains("<dd class=\"data\">M7</dd>"), "{main}");
        assert!(
            main.contains("<span id=\"machine-name\">Press line</span>"),
            "{main}"
        );
        assert!(
            main.contains(&format!("<dd>{}</dd>", text(lang, "platform-s7-300"))),
            "{main}"
        );
        assert!(main.contains("<div id=\"unit-tree\">"), "{main}");
        assert!(!has_machine_form(main), "{main}");
        assert!(!main.contains("<form"), "an Operator gets no form: {main}");
        assert!(!main.contains("/units/new"), "{main}");
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn machines_page_title_and_labels_come_from_the_bundles() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines/M7").await;
    assert!(
        reply
            .body
            .contains("<title>Machine M7 · Recipe editor</title>")
    );
    let main = main_of(&reply.body);
    for label in ["Machine code", "Machine name", "PLC platform"] {
        assert!(
            main.contains(&format!("<dt>{label}</dt>")),
            "{label}: {main}"
        );
    }
    let types = "<a class=\"target-link\" href=\"/machines/M7/types\">Open the UDT types</a>";
    assert!(main.contains(types), "{main}");
    assert!(main.contains("<h2>Unit tree</h2>"), "{main}");
    assert!(
        main.contains("href=\"/machines\""),
        "a link back to the list: {main}"
    );
}

#[tokio::test]
async fn machines_page_of_an_engineer_has_the_rename_form_with_literal_hx_attributes() {
    let world = world();
    let reply = world.get_en(&world.engineer, "/machines/M7").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains(RENAME_TAG), "{main}");
    let (_before, rest) = main.split_once(RENAME_TAG).unwrap();
    let form = rest.split("</form>").next().unwrap();
    let token = format!(
        "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
        world.engineer.csrf
    );
    assert!(form.contains(&token), "{form}");
    assert!(
        form.contains("<label for=\"machine_name\">Machine name"),
        "{form}"
    );
    assert!(form.contains("id=\"machine_name\" name=\"name\""), "{form}");
    assert!(form.contains(">Rename machine</button>"), "{form}");
    assert!(
        form.contains("<button type=\"submit\" id=\"machine-rename-submit\">"),
        "{form}"
    );
    assert!(!form.contains("method=\"get\""));
}

/// Spec "Rename and archive of a machine", scenario "Rename form with the current name" (D20, U9).
#[tokio::test]
async fn machines_page_rename_input_holds_the_current_name() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M7").await;
        let main = main_of(&reply.body);
        let (_before, rest) = main.split_once(RENAME_TAG).unwrap();
        let form = rest.split("</form>").next().unwrap();
        let input = form.split("<input id=\"machine_name\"").nth(1).unwrap();
        let input = input.split('>').next().unwrap();
        assert!(input.contains(" value=\"Press line\""), "{input}");
        assert_eq!(input.matches("value=").count(), 1, "{input}");
    }
}

#[tokio::test]
async fn machines_page_of_an_engineer_has_the_archive_form_inside_a_details_element() {
    let world = world();
    let reply = world.get_en(&world.engineer, "/machines/M7").await;
    let main = main_of(&reply.body);
    assert!(main.contains(ARCHIVE_TAG), "{main}");
    let (before, rest) = main.split_once(ARCHIVE_TAG).unwrap();
    assert!(before.ends_with("</summary>"), "{before}");
    assert!(
        before.contains("<details class=\"danger\"><summary>"),
        "{before}"
    );
    assert!(
        before.contains("<span>Archive the machine</span>"),
        "{before}"
    );
    let form = rest.split("</details>").next().unwrap();
    let warning = text_with(Lang::En, "machine-archive-warning", "code", "M7");
    assert!(form.contains(&warning), "{form}");
    assert!(form.contains(">Archive machine M7</button>"), "{form}");
    assert!(form.contains(&format!("value=\"{}\"", world.engineer.csrf)));
    assert!(!main.contains(" open"), "the disclosure starts closed");
}

#[tokio::test]
async fn machines_page_of_an_engineer_has_the_link_to_add_a_main_unit_and_an_empty_unit_form() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M7").await;
        let main = main_of(&reply.body);
        assert!(main.contains(ADD_MAIN_UNIT), "{main}");
        assert!(main.contains("<div id=\"unit-form\"></div>"), "{main}");
        assert!(main.contains(RENAME_TAG) && main.contains(ARCHIVE_TAG));
    }
}

#[tokio::test]
async fn machines_page_hx_attributes_are_literals_and_hold_no_user_text() {
    let world = world();
    let reply = world.get_en(&world.engineer, "/machines/M7").await;
    for needle in ["M7", "Press line", "U1"] {
        assert_no_user_text_in_hx_attributes(&reply.body, needle);
    }
    assert_clean_html(&reply.body);
    let pairs = hx_pairs(main_of(&reply.body));
    assert!(
        pairs.len() >= 13,
        "rename form, archive form, and add link: {pairs:?}"
    );
    for (name, value) in &pairs {
        let known = ALLOWED_HX.contains(&(name.as_str(), value.as_str()));
        assert!(known, "{name}={value} is not a literal of the design");
    }
}

// --- Spec "Machine page": by identifier ---------------------------------------------------------

#[tokio::test]
async fn machines_page_of_a_live_machine_by_identifier_is_the_live_page() {
    let world = world();
    let path = format!("/machines/by-id/{M7_ID}");
    let reply = world.get_en(&world.engineer, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(!main.contains("badge"), "{main}");
    assert!(
        main.contains(RENAME_TAG) && main.contains(ARCHIVE_TAG),
        "{main}"
    );
    assert!(main.contains("<h1>Machine M7</h1>"));
}

#[tokio::test]
async fn machines_page_of_an_archived_machine_shows_the_large_badge_the_notice_and_the_date() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.engineer, &archived_path(), &[cookie_of(lang)])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        let badge = format!(
            "<span class=\"badge badge-large badge-retired\">{}",
            icon_tag("status-retired")
        );
        assert!(main.contains(&badge), "{lang}: {main}");
        assert!(main.contains(&format!(
            "<span>{}</span></span>",
            text(lang, "status-archived")
        )));
        let notice = text(lang, "machine-archived-notice");
        assert!(
            main.contains(&format!("<span>{notice}</span>")),
            "{lang}: {main}"
        );
        assert!(main.contains(&format!("<dt>{}</dt>", text(lang, "machine-archived-at"))));
        assert!(
            main.contains("<dd class=\"data\">1970-01-06 18:53 UTC</dd>"),
            "{main}"
        );
        assert!(main.contains("<dd class=\"data\">M12</dd>"), "{main}");
    }
}

#[tokio::test]
async fn machines_page_of_an_archived_machine_has_no_form_and_no_link_below_its_code() {
    let world = world();
    for session in [&world.engineer, &world.admin, &world.operator] {
        let reply = world.get_en(session, &archived_path()).await;
        let main = main_of(&reply.body);
        for forbidden in ["<form", "<details", "<input", "hx-"] {
            assert!(!main.contains(forbidden), "found {forbidden}: {main}");
        }
        assert!(!reply.body.contains("/machines/M12"), "{}", reply.body);
        assert!(!reply.body.contains("/units/new"), "{}", reply.body);
        assert!(main.contains("<div id=\"unit-tree\">"), "{main}");
    }
}

#[tokio::test]
async fn machines_page_by_identifier_rejects_a_bad_or_an_unknown_identifier_with_404() {
    let world = world();
    let compact = M7_ID.simple().to_string();
    let bad = [
        "/machines/by-id/xyz".to_owned(),
        format!("/machines/by-id/{compact}"),
        format!("/machines/by-id/{}", M7_ID.to_string().to_uppercase() + "0"),
        "/machines/by-id/00000000-0000-0000-0000-000000000001".to_owned(),
    ];
    for path in bad {
        let reply = world.get_en(&world.engineer, &path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert!(reply.body.contains("E-WEB-005"), "{path}: {}", reply.body);
    }
}

// --- Spec "Machine page": the code --------------------------------------------------------------

#[tokio::test]
async fn machines_page_rejects_a_malformed_or_an_unknown_code_with_404() {
    let world = world();
    for code in ["X7", "m7", "M0", "M01", "M12345678", "M7x", "M99", "M12"] {
        let reply = world
            .get_en(&world.operator, &format!("/machines/{code}"))
            .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{code}");
        assert!(reply.body.contains("E-WEB-005"), "{code}: {}", reply.body);
        assert!(!reply.body.contains("Press line"), "{code}");
    }
}

#[tokio::test]
async fn machines_page_bad_code_in_an_htmx_request_goes_to_the_error_panel() {
    let world = world();
    let headers = [ENGLISH, ("hx-request", "true")];
    let reply = world.get(&world.engineer, "/machines/X7", &headers).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert!(reply.body.contains("E-WEB-005"), "{}", reply.body);
}

// --- Archive message of a unit ------------------------------------------------------------------

#[tokio::test]
async fn machines_page_shows_the_message_of_an_archived_unit_in_the_toast() {
    let world = world();
    seed_machine_with_archived_unit(&world.fixture);
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.engineer,
                "/machines/M20?archived=U3",
                &[cookie_of(lang)],
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let sentence = text_with(lang, "unit-archived-toast", "path", "U3");
        let toast = format!(
            "<div id=\"toast\" role=\"status\" aria-live=\"polite\"><div class=\"toast\">{}",
            icon_tag("alert-success")
        );
        assert!(reply.body.contains(&toast), "{lang}: {}", reply.body);
        assert!(
            reply.body.contains(&format!("<span>{sentence}</span>")),
            "{lang}"
        );
    }
}

#[tokio::test]
async fn machines_page_shows_no_message_for_a_live_or_an_unknown_unit() {
    let world = world();
    seed_machine_with_archived_unit(&world.fixture);
    for path in ["U1", "U1.1", "U9", "U1.2.3"] {
        let reply = world
            .get_en(&world.engineer, &format!("/machines/M20?archived={path}"))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        let empty = "<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>";
        assert!(reply.body.contains(empty), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn machines_page_rejects_a_bad_archived_path_and_an_unknown_key() {
    let world = world();
    let cases = [
        ("archived=X", "E-PLT-017"),
        ("archived=U0", "E-PLT-017"),
        ("archived=U1.2.3.4.5.6", "E-PLT-002"),
        ("other=1", "E-WEB-008"),
        ("archived=U1&archived=U2", "E-WEB-008"),
    ];
    for (query, code) in cases {
        let path = format!("/machines/M7?{query}");
        let reply = world
            .get(&world.engineer, &path, &[ENGLISH, ("hx-request", "true")])
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        assert!(reply.body.contains(code), "{query}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{query}");
    }
}

// --- Spec "Content answer of a page" ------------------------------------------------------------

#[tokio::test]
async fn machines_page_content_answer_has_title_regions_heading_and_the_field_next() {
    let world = world();
    let reply = world.get_content(&world.engineer, "/machines/M7").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Machine M7 · Recipe editor</title>")
    );
    assert!(reply.body.contains("<div id=\"error-panel\"></div>"));
    assert!(
        reply
            .body
            .contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Machine M7</h1>")
    );
    let next = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/machines/M7\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(next), "{}", reply.body);
    for frame in ["class=\"topbar\"", "class=\"skip-link\"", "<main", "<html"] {
        assert!(!reply.body.contains(frame), "{frame}");
    }
    assert!(
        reply.body.contains(RENAME_TAG),
        "the forms are part of the content"
    );
}

#[tokio::test]
async fn machines_page_by_identifier_has_the_content_answer_too() {
    let world = world();
    let reply = world.get_content(&world.operator, &archived_path()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Machine M12</h1>")
    );
    assert!(!reply.body.contains("class=\"topbar\""));
    let next = format!(
        "name=\"next\" value=\"{}\" hx-swap-oob=\"true\"",
        archived_path()
    );
    assert!(reply.body.contains(&next), "{}", reply.body);
}
