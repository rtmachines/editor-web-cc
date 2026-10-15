//! Tests of `GET /machines/new` and `POST /machines` (task 7.2, specs "Machine form and creation"
//! and "Content answer of a page").

use axum::http::StatusCode;

use crate::i18n::Lang;
use crate::pages::test_helpers::{assert_clean_html, assert_no_user_text_in_hx_attributes};
use crate::test_support::plant::{cookie_of, world};
use crate::test_support::text;

/// A valid body of the form.
const GOOD: &str = "code=M99&name=Press+line+2&plc_platform=S7_300&main_unit_name=Infeed";
/// The opening tag of the create form (design D6).
const FORM_TAG: &str = "<form method=\"post\" action=\"/machines\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The five platforms: the value of the form and the key of the text.
const PLATFORMS: [(&str, &str); 5] = [
    ("CodesysIec", "platform-codesys-iec"),
    ("TwinCat3", "platform-twincat3"),
    ("S7_1200", "platform-s7-1200"),
    ("S7_1500", "platform-s7-1500"),
    ("S7_300", "platform-s7-300"),
];

/// A body with one bad field, the name of the field in the error, and the key of the reason.
const BAD_FIELDS: [(&str, &str, &str); 8] = [
    (
        "code=X7&name=A&plc_platform=S7_300&main_unit_name=B",
        "code",
        "reason-machine-code",
    ),
    (
        "code=M0&name=A&plc_platform=S7_300&main_unit_name=B",
        "code",
        "reason-machine-code",
    ),
    (
        "code=M99&name=+&plc_platform=S7_300&main_unit_name=B",
        "machine_name",
        "reason-name",
    ),
    (
        "code=M99&name=%01&plc_platform=S7_300&main_unit_name=B",
        "machine_name",
        "reason-name",
    ),
    (
        "code=M99&name=A&plc_platform=S7-300&main_unit_name=B",
        "plc_platform",
        "reason-platform",
    ),
    (
        "code=M99&name=A&plc_platform=&main_unit_name=B",
        "plc_platform",
        "reason-platform",
    ),
    (
        "code=M99&name=A&plc_platform=s7_300&main_unit_name=B",
        "plc_platform",
        "reason-platform",
    ),
    (
        "code=M99&name=A&plc_platform=S7_300&main_unit_name=+",
        "main_unit_name",
        "reason-name",
    ),
];

/// Return the markup of the create form of a page.
fn form_of(body: &str) -> &str {
    let (_before, rest) = body
        .split_once("<form method=\"post\" action=\"/machines\"")
        .unwrap();
    rest.split("</form>").next().unwrap()
}

// --- GET /machines/new --------------------------------------------------------------------------

#[tokio::test]
async fn machines_form_holds_the_four_fields_with_their_ids_names_labels_and_hints() {
    let world = world();
    let fields = [
        ("code", "code", "field-code"),
        ("machine_name", "name", "field-machine-name"),
        ("plc_platform", "plc_platform", "field-plc-platform"),
        ("main_unit_name", "main_unit_name", "field-main-unit-name"),
    ];
    let hints = [
        ("code", "machine-code-hint"),
        ("plc_platform", "machine-platform-hint"),
        ("main_unit_name", "machine-main-unit-hint"),
    ];
    for lang in [Lang::En, Lang::Pl] {
        let path = "/machines/new";
        let reply = world.get(&world.engineer, path, &[cookie_of(lang)]).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let form = form_of(&reply.body);
        for (id, name, label) in fields {
            let head = format!("<label for=\"{id}\">{}", text(lang, label));
            assert!(form.contains(&head), "{lang} {id}: {form}");
            assert!(
                form.contains(&format!("id=\"{id}\" name=\"{name}\"")),
                "{id}"
            );
        }
        for (id, key) in hints {
            let hint = format!(
                "<span class=\"field-hint\" id=\"{id}-hint\">{}</span>",
                text(lang, key)
            );
            assert!(form.contains(&hint), "{lang} {id}: {form}");
        }
        assert!(!form.contains("id=\"machine_name-hint\""), "{form}");
    }
}

/// RQ-2: the platform list is required, so it has the required mark and `aria-required`.
#[tokio::test]
async fn machines_form_marks_the_platform_list_as_required() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.engineer, "/machines/new", &[cookie_of(lang)])
            .await;
        let form = form_of(&reply.body);
        let label = format!(
            "<label for=\"plc_platform\">{} <span class=\"field-required\">{}</span></label>",
            text(lang, "field-plc-platform"),
            text(lang, "form-required")
        );
        assert!(form.contains(&label), "{lang}: {form}");
        assert!(form.contains("<select id=\"plc_platform\" name=\"plc_platform\" aria-required=\"true\" aria-describedby=\"plc_platform-hint\">"), "{lang}: {form}");
        assert_eq!(form.matches("aria-required=\"true\"").count(), 4, "{form}");
    }
}

#[tokio::test]
async fn machines_form_starts_the_platform_list_with_the_selected_empty_option() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.engineer, "/machines/new", &[cookie_of(lang)])
            .await;
        let form = form_of(&reply.body);
        let choose = format!(
            "<option value=\"\" selected>{}</option>",
            text(lang, "field-platform-choose")
        );
        let (_before, options) = form.split_once("<select").unwrap();
        let options = options.split("</select>").next().unwrap();
        let first = options.split_once('>').unwrap().1;
        assert!(first.starts_with(&choose), "{lang}: {options}");
        for (value, key) in PLATFORMS {
            let option = format!("<option value=\"{value}\">{}</option>", text(lang, key));
            assert!(form.contains(&option), "{lang} {value}: {form}");
        }
        assert_eq!(form.matches("<option").count(), 6);
        assert_eq!(form.matches(" selected").count(), 1, "{form}");
    }
}

#[tokio::test]
async fn machines_form_posts_with_literal_hx_attributes_the_token_and_one_primary_button() {
    let world = world();
    let reply = world.get_en(&world.engineer, "/machines/new").await;
    assert!(reply.body.contains(FORM_TAG), "{}", reply.body);
    let form = form_of(&reply.body);
    let token = format!(
        "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
        world.engineer.csrf
    );
    assert!(form.contains(&token), "{form}");
    assert_eq!(form.matches(" hx-").count(), 4, "{form}");
    assert!(
        form.contains("maxlength=\"8\""),
        "the code has at most 8 characters"
    );
    assert_eq!(form.matches("btn-primary").count(), 1, "{form}");
    assert!(form.contains(">Create machine</button>"), "{form}");
    assert!(!form.contains("method=\"get\""));
    assert_no_user_text_in_hx_attributes(&reply.body, "M7");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn machines_form_page_has_the_title_and_marks_the_machine_list_as_current() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(&world.admin, "/machines/new", &[cookie_of(lang)])
            .await;
        let title = format!(
            "<title>{} · {}</title>",
            text(lang, "machine-new-title"),
            text(lang, "app-name")
        );
        assert!(reply.body.contains(&title), "{lang}: {}", reply.body);
        let current = format!(
            "<a class=\"nav-link\" href=\"/machines\" aria-current=\"page\">{}</a>",
            text(lang, "nav-machines")
        );
        assert!(reply.body.contains(&current), "{lang}: {}", reply.body);
        assert_eq!(reply.body.matches("<h1").count(), 1);
    }
}

#[tokio::test]
async fn machines_form_is_forbidden_for_an_operator() {
    let world = world();
    let reply = world.get_en(&world.operator, "/machines/new").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
}

#[tokio::test]
async fn machines_form_content_answer_has_the_title_the_heading_and_the_field_next() {
    let world = world();
    let reply = world.get_content(&world.engineer, "/machines/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>New machine · Recipe editor</title>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>New machine</h1>")
    );
    let next = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/machines/new\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(next), "{}", reply.body);
    assert!(reply.body.contains("<div id=\"error-panel\"></div>"));
    for frame in ["class=\"topbar\"", "class=\"skip-link\"", "<main", "<html"] {
        assert!(!reply.body.contains(frame), "{frame}");
    }
    assert!(
        reply.body.contains(FORM_TAG),
        "the form is part of the content"
    );
}

// --- POST /machines -----------------------------------------------------------------------------

#[tokio::test]
async fn machines_create_makes_an_s7_300_machine_and_redirects_to_its_page() {
    let world = world();
    let before = world.audit_count();
    let reply = world.post(&world.engineer, "/machines", GOOD, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/machines/M99");
    let entries = world.audit_count().checked_sub(before).unwrap();
    assert_eq!(entries, 2, "one entry for the machine and one for U1");
    let list = world.get_en(&world.engineer, "/machines?q=M99").await;
    assert!(
        list.body.contains("<a href=\"/machines/M99\">M99</a>"),
        "{}",
        list.body
    );
    assert!(list.body.contains("<td>Press line 2</td>"), "{}", list.body);
    assert!(
        list.body.contains("<td>Siemens S7-300</td>"),
        "{}",
        list.body
    );
}

#[tokio::test]
async fn machines_create_accepts_the_token_field_of_a_plain_form_and_an_htmx_request() {
    let world = world();
    let body = format!("{GOOD}&csrf_token={}", world.engineer.csrf);
    let plain = world.post(&world.engineer, "/machines", &body, false).await;
    assert_eq!(plain.status, StatusCode::SEE_OTHER, "{}", plain.body);
    let htmx_body = "code=M98&name=Second&plc_platform=TwinCat3&main_unit_name=Infeed";
    let htmx = world.post(&world.admin, "/machines", htmx_body, true).await;
    assert_eq!(htmx.status, StatusCode::SEE_OTHER, "{}", htmx.body);
    assert_eq!(htmx.header("location"), "/machines/M98");
}

#[tokio::test]
async fn machines_create_is_forbidden_for_an_operator_and_creates_nothing() {
    let world = world();
    let before = world.audit_count();
    let reply = world.post(&world.operator, "/machines", GOOD, false).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn machines_create_with_the_code_of_a_live_machine_gives_409_in_the_error_panel() {
    let world = world();
    let body = "code=M7&name=Again&plc_platform=S7_300&main_unit_name=Infeed";
    let before = world.audit_count();
    let reply = world.post(&world.engineer, "/machines", body, true).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-PLT-001"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert!(!reply.body.contains("<form"), "the fragment holds no form");
    assert_eq!(world.audit_count(), before);
}

#[tokio::test]
async fn machines_create_names_the_field_and_the_reason_of_each_bad_value() {
    let world = world();
    let before = world.audit_count();
    for (body, field, reason) in BAD_FIELDS {
        let reply = world.post(&world.engineer, "/machines", body, true).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(reply.body.contains("E-WEB-008"), "{body}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{body}");
        let label = text(Lang::En, &format!("field-{}", field.replace('_', "-")));
        assert!(reply.body.contains(&label), "{body}: {}", reply.body);
        assert!(reply.body.contains(&format!("href=\"#{field}\"")), "{body}");
        assert!(reply.body.contains(&text(Lang::En, reason)), "{body}");
        assert!(!reply.body.contains("<form"), "{body}");
    }
    assert_eq!(
        world.audit_count(),
        before,
        "a rejected form creates nothing"
    );
}

#[tokio::test]
async fn machines_create_rejects_a_missing_or_an_unknown_field() {
    let world = world();
    let bodies = [
        "code=M99&name=A&plc_platform=S7_300",
        "code=M99&name=A&plc_platform=S7_300&main_unit_name=B&extra=1",
        "",
    ];
    for body in bodies {
        let reply = world.post(&world.engineer, "/machines", body, true).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(reply.body.contains("E-WEB-008"), "{body}: {}", reply.body);
    }
}

#[tokio::test]
async fn machines_create_stores_a_name_with_markup_and_the_list_shows_it_escaped() {
    let world = world();
    let body = "code=M97&name=%3Cb%3Ex%3C%2Fb%3E&plc_platform=S7_300&main_unit_name=Infeed";
    let reply = world.post(&world.engineer, "/machines", body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    let list = world.get_en(&world.engineer, "/machines?q=M97").await;
    assert!(
        list.body.contains("<td>&#60;b&#62;x&#60;/b&#62;</td>"),
        "{}",
        list.body
    );
    assert!(!list.body.contains("<b>x</b>"));
}
