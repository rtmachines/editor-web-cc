//! Tests of `GET /machines/{code}/units/{path}` (task 9.3, specs "Unit page", "Content answer of
//! a page", and "Plant forms without user data in an hx attribute").

use axum::http::StatusCode;

use super::test_world::world;
use crate::i18n::Lang;
use crate::pages::test_helpers::{
    assert_clean_html, assert_no_user_text_in_hx_attributes, hx_pairs,
};
use crate::test_support::plant::{cookie_of, main_of, text_with};
use crate::test_support::{icon_tag, text};

/// The opening tag of the rename form (design D6).
const RENAME_TAG: &str = "<form method=\"post\" action=\"/machines/M7/units/U1.2/rename\" hx-boost=\"true\" hx-target=\"#unit-name\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The opening tag of the archive form (design D6).
const ARCHIVE_TAG: &str = "<form method=\"post\" action=\"/machines/M7/units/U1.2/archive\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The number of live units under `U1.2`: 4 children, 5 under `U1.2.4`, and 6 under `U1.2.4.5`.
const U1_2_DESCENDANTS: &str = "15";

// --- Spec "Unit page": the content --------------------------------------------------------------

#[tokio::test]
async fn units_page_shows_the_machine_the_path_the_name_the_parent_and_the_title() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.operator,
                "/machines/M7/units/U1.2",
                &[cookie_of(lang)],
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let title = text_with(lang, "unit-title", "path", "U1.2");
        assert!(reply.body.contains(&format!("<h1>{title}</h1>")), "{lang}");
        assert!(reply.body.contains(&format!(
            "<title>{title} · {}</title>",
            text(lang, "app-name")
        )));
        let main = main_of(&reply.body);
        let machine = format!(
            "<dt>{}</dt>\n<dd><a class=\"target-link\" href=\"/machines/M7\">M7</a></dd>",
            text(lang, "unit-machine")
        );
        assert!(main.contains(&machine), "{lang}: {main}");
        let path = format!(
            "<dt>{}</dt>\n<dd class=\"data\">U1.2</dd>",
            text(lang, "unit-path")
        );
        assert!(main.contains(&path), "{lang}: {main}");
        let name = format!(
            "<dt>{}</dt>\n<dd><span id=\"unit-name\">Unit U1.2</span></dd>",
            text(lang, "field-unit-name")
        );
        assert!(main.contains(&name), "{lang}: {main}");
        assert!(
            main.contains("<dt>") && main.contains("href=\"/machines/M7/units/U1\""),
            "the parent links to the unit page: {main}"
        );
        assert_eq!(reply.body.matches("<h1").count(), 1);
        let current = format!(
            "<a class=\"nav-link\" href=\"/machines\" aria-current=\"page\">{}</a>",
            text(lang, "nav-machines")
        );
        assert!(reply.body.contains(&current), "{lang}");
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn units_page_of_a_main_unit_says_that_it_has_no_parent() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines/M7/units/U1").await;
    let main = main_of(&reply.body);
    assert!(main.contains("None. The unit is a main unit."), "{main}");
}

#[tokio::test]
async fn units_page_lists_the_live_children_as_links() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines/M7/units/U1.2")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("<h2>Sub-units</h2>"), "{main}");
    for child in ["U1.2.1", "U1.2.2", "U1.2.3", "U1.2.4"] {
        let link = format!("href=\"/machines/M7/units/{child}\"");
        assert!(main.contains(&link), "{child}: {main}");
    }
    assert!(
        !main.contains("href=\"/machines/M7/units/U1.2.4.5\""),
        "{main}"
    );
    assert!(!main.contains("The unit has no sub-unit."));
}

/// Design D20 (U6): the parent link and the sub-unit links stand in a column with the target gap.
#[tokio::test]
async fn units_page_puts_the_parent_link_and_the_sub_unit_links_in_link_lists() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines/M7/units/U1.2")
        .await;
    let main = main_of(&reply.body);
    assert!(
        main.contains(
            "<dd class=\"link-list\"><a class=\"target-link\" href=\"/machines/M7/units/U1\">"
        ),
        "{main}"
    );
    let (_before, rest) = main.split_once("<ul class=\"link-list\">").unwrap();
    let list = rest.split("</ul>").next().unwrap();
    assert_eq!(
        list.matches("<li><a class=\"target-link\"").count(),
        4,
        "{list}"
    );
    assert_eq!(main.matches("class=\"link-list\"").count(), 2, "{main}");
}

#[tokio::test]
async fn units_page_of_a_main_unit_keeps_the_text_without_a_link_list_for_the_parent() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines/M7/units/U1").await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("<dd>None. The unit is a main unit.</dd>"),
        "{main}"
    );
}

#[tokio::test]
async fn units_page_of_a_leaf_shows_the_empty_sentence() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines/M7/units/U1.2.1")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("The unit has no sub-unit."), "{main}");
}

#[tokio::test]
async fn units_page_escapes_the_name_of_the_unit() {
    let world = world();
    let payload = "<img src=x onerror=alert(1)>";
    let body = "parent_path=U1.1&segment=9&name=%3Cimg+src%3Dx+onerror%3Dalert(1)%3E";
    let added = world
        .post(&world.engineer, "/machines/M7/units", body, false)
        .await;
    assert_eq!(added.status, StatusCode::SEE_OTHER, "{}", added.body);
    let reply = world
        .get_en(&world.engineer, "/machines/M7/units/U1.1.9")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains(payload), "{}", reply.body);
    assert!(reply.body.contains("&#60;img src=x onerror=alert(1)&#62;"));
    assert_clean_html(&reply.body);
}

// --- The forms ----------------------------------------------------------------------------------

#[tokio::test]
async fn units_page_of_an_engineer_has_the_rename_form_with_the_literal_hx_attributes() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M7/units/U1.2").await;
        let main = main_of(&reply.body);
        assert!(main.contains(RENAME_TAG), "{main}");
        let (_before, rest) = main.split_once(RENAME_TAG).unwrap();
        let form = rest.split("</form>").next().unwrap();
        let token = format!(
            "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
            session.csrf
        );
        assert!(form.contains(&token), "{form}");
        assert!(
            form.contains("<label for=\"unit_name\">Unit name"),
            "{form}"
        );
        assert!(form.contains("id=\"unit_name\" name=\"name\""), "{form}");
        assert!(form.contains(">Rename unit</button>"), "{form}");
    }
}

/// Spec "Rename and archive of a machine" (D20, U9): the unit rename input holds the current name.
#[tokio::test]
async fn units_page_rename_input_holds_the_current_name() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M7/units/U1.2")
        .await;
    let main = main_of(&reply.body);
    let (_before, rest) = main.split_once(RENAME_TAG).unwrap();
    let form = rest.split("</form>").next().unwrap();
    let input = form.split("<input id=\"unit_name\"").nth(1).unwrap();
    let input = input.split('>').next().unwrap();
    assert!(input.contains(" value=\"Unit U1.2\""), "{input}");
    assert_eq!(input.matches("value=").count(), 1, "{input}");
}

#[tokio::test]
async fn units_page_of_an_engineer_has_the_archive_form_with_the_warning_and_the_count() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.engineer,
                "/machines/M7/units/U1.2",
                &[cookie_of(lang)],
            )
            .await;
        let main = main_of(&reply.body);
        assert!(main.contains(ARCHIVE_TAG), "{lang}: {main}");
        let (before, rest) = main.split_once(ARCHIVE_TAG).unwrap();
        assert!(
            before.contains("<details class=\"danger\"><summary>"),
            "{before}"
        );
        assert!(before.contains(&format!(
            "<span>{}</span>",
            text(lang, "unit-archive-summary")
        )));
        let form = rest.split("</details>").next().unwrap();
        assert!(
            form.contains(&text(lang, "unit-archive-warning")),
            "{lang}: {form}"
        );
        let count = text_with(lang, "unit-archive-count", "count", U1_2_DESCENDANTS);
        assert!(form.contains(&count), "{lang}: {form}");
        let button = text_with(lang, "unit-archive-submit", "path", "U1.2");
        assert!(
            form.contains(&format!(">{button}</button>")),
            "{lang}: {form}"
        );
        assert!(!main.contains(" open"));
    }
}

#[tokio::test]
async fn units_page_of_an_operator_has_no_form() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines/M7/units/U1.2")
        .await;
    let main = main_of(&reply.body);
    for forbidden in ["<form", "<details", "<input", "hx-"] {
        assert!(!main.contains(forbidden), "{forbidden}: {main}");
    }
}

#[tokio::test]
async fn units_page_hx_attributes_are_literals_and_hold_no_user_text() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M7/units/U1.2")
        .await;
    for needle in ["M7", "U1.2", "Unit U1.2", "Press line"] {
        assert_no_user_text_in_hx_attributes(&reply.body, needle);
    }
    let pairs = hx_pairs(main_of(&reply.body));
    assert_eq!(
        pairs.len(),
        9,
        "five of the rename form and four of the archive form: {pairs:?}"
    );
    for (name, value) in &pairs {
        let known = matches!(
            (name.as_str(), value.as_str()),
            ("hx-boost", "true")
                | ("hx-target", "#unit-name" | "#content")
                | ("hx-swap", "innerHTML")
                | ("hx-push-url", "false" | "true")
                | ("hx-sync", "closest form:drop")
        );
        assert!(known, "{name}={value}");
    }
}

// --- An archived unit ---------------------------------------------------------------------------

#[tokio::test]
async fn units_page_of_an_archived_unit_shows_the_badge_and_no_form() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.engineer,
                "/machines/M31/units/U2",
                &[cookie_of(lang)],
            )
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
        for forbidden in ["<form", "<details", "<input", "hx-"] {
            assert!(!main.contains(forbidden), "{forbidden}: {main}");
        }
    }
}

#[tokio::test]
async fn units_page_of_a_live_unit_shows_no_archived_badge() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M31/units/U1")
        .await;
    assert!(!main_of(&reply.body).contains("badge"));
}

// --- Errors -------------------------------------------------------------------------------------

#[tokio::test]
async fn units_page_with_six_segments_gives_422_e_plt_002() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines/M7/units/U1.2.4.5.6.7")
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-002"), "{}", reply.body);
}

#[tokio::test]
async fn units_page_gives_the_code_of_each_bad_value() {
    let world = world();
    let cases = [
        (
            "/machines/M7/units/X",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "/machines/M7/units/U0",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-PLT-017",
        ),
        (
            "/machines/M7/units/U1.9",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        ("/machines/M99/units/U1", StatusCode::NOT_FOUND, "E-WEB-005"),
        ("/machines/M12/units/U1", StatusCode::NOT_FOUND, "E-WEB-005"),
        ("/machines/X7/units/U1", StatusCode::NOT_FOUND, "E-WEB-005"),
    ];
    for (path, status, code) in cases {
        let reply = world
            .get(
                &world.operator,
                path,
                &[("cookie", "recipe_lang=en"), ("hx-request", "true")],
            )
            .await;
        assert_eq!(reply.status, status, "{path}");
        assert!(reply.body.contains(code), "{path}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{path}");
    }
}

// --- Spec "Content answer of a page" ------------------------------------------------------------

#[tokio::test]
async fn units_page_content_answer_has_the_title_the_heading_with_focus_and_the_field_next() {
    let world = world();
    let reply = world
        .get_content(&world.engineer, "/machines/M7/units/U1.2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>Unit U1.2 · Recipe editor</title>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Unit U1.2</h1>")
    );
    let next = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/machines/M7/units/U1.2\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(next), "{}", reply.body);
    assert!(reply.body.contains("<div id=\"error-panel\"></div>"));
    assert!(
        reply
            .body
            .contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>")
    );
    for frame in ["class=\"topbar\"", "class=\"skip-link\"", "<main", "<html"] {
        assert!(!reply.body.contains(frame), "{frame}");
    }
    assert!(
        reply.body.contains(RENAME_TAG),
        "the forms are part of the content"
    );
}
