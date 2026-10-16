//! Tests of `GET /machines/{code}/units/new` (task 9.1, specs "Addition of a unit", "Content answer
//! of a page", and "Plant forms without user data in an hx attribute").

use axum::http::StatusCode;

use super::test_world::world;
use crate::i18n::Lang;
use crate::pages::test_helpers::{
    assert_clean_html, assert_no_user_text_in_hx_attributes, hx_pairs,
};
use crate::test_support::plant::{ENGLISH, HTMX, World, cookie_of, main_of, text_with};
use crate::test_support::text;

/// The opening tag of the form of the fragment (design D6). The fixed `id` serves the out-of-band
/// swap of an error answer (ADR-0041 (e)).
const FORM_TAG: &str = "<form id=\"unit-add-form\" method=\"post\" action=\"/machines/M7/units\" hx-boost=\"true\" hx-target=\"#unit-tree\" hx-swap=\"innerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The opening tag of the form of the page. The page has no `#unit-tree`, so the form is plain.
const PLAIN_FORM_TAG: &str =
    "<form id=\"unit-add-form\" method=\"post\" action=\"/machines/M7/units\">";
/// The number of live units of `M7` with depth 1 to 4. Each one is a parent in the list.
const M7_PARENTS: usize = 42;

/// Return the markup of the form of a fragment or a page, from `<form` to `</form>`.
fn form_of(body: &str) -> &str {
    let (_before, rest) = body
        .split_once("<form id=\"unit-add-form\" method=\"post\" action=\"/machines/")
        .unwrap();
    rest.split("</form>").next().unwrap()
}

/// Return the markup of the input with the `id`, from `<input` to `>`.
fn input_of<'a>(markup: &'a str, id: &str) -> &'a str {
    let marker = format!("<input id=\"{id}\"");
    let (_before, rest) = markup.split_once(&marker).unwrap();
    rest.split('>').next().unwrap()
}

/// Send the HTMX request for the form with the parent `parent` as the engineer.
async fn fragment(world: &World, query: &str) -> String {
    let path = format!("/machines/M7/units/new{query}");
    let reply = world.get_form(&world.engineer, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{query}: {}", reply.body);
    reply.body
}

// --- The fragment for #unit-form ----------------------------------------------------------------

#[tokio::test]
async fn units_form_fragment_names_the_parent_in_an_h2_and_nothing_else_around_it() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX, ("hx-target", "unit-form")];
        let reply = world
            .get(
                &world.engineer,
                "/machines/M7/units/new?parent_path=U1.2",
                &headers,
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let heading = text_with(lang, "unit-new-under", "path", "U1.2");
        assert!(
            reply.body.starts_with(&format!("<h2>{heading}</h2>")),
            "{lang}: {}",
            reply.body
        );
        for frame in ["<html", "<h1", "<main", "class=\"topbar\"", "<title"] {
            assert!(!reply.body.contains(frame), "{lang}: {frame}");
        }
    }
}

#[tokio::test]
async fn units_form_fragment_lists_none_and_the_live_parents_and_selects_the_chosen_one() {
    let world = world();
    let body = fragment(&world, "?parent_path=U1.2").await;
    let form = form_of(&body);
    assert!(
        form.contains("<label for=\"parent_path\">Parent unit</label>"),
        "{form}"
    );
    assert!(
        form.contains("<select id=\"parent_path\" name=\"parent_path\">"),
        "{form}"
    );
    let none = "<option value=\"\">None. The unit is a main unit.</option>";
    assert!(form.contains(none), "{form}");
    assert!(
        form.contains("<option value=\"U1.2\" selected>U1.2 Unit U1.2</option>"),
        "{form}"
    );
    assert_eq!(form.matches(" selected").count(), 1, "{form}");
    assert_eq!(
        form.matches("<option").count(),
        M7_PARENTS.checked_add(1).unwrap(),
        "none and the units of depth 1 to 4"
    );
    assert!(
        !form.contains("<option value=\"U1.2.4.5.6\""),
        "a unit of depth 5 is no parent"
    );
}

#[tokio::test]
async fn units_form_fragment_without_a_parent_selects_none_and_proposes_the_next_main_number() {
    let world = world();
    let body = fragment(&world, "").await;
    assert!(body.starts_with("<h2>New main unit</h2>"), "{body}");
    let form = form_of(&body);
    assert!(
        form.contains("<option value=\"\" selected>None. The unit is a main unit.</option>"),
        "{form}"
    );
    assert_eq!(form.matches(" selected").count(), 1, "{form}");
    assert!(input_of(form, "segment").contains("value=\"2\""), "{form}");
}

#[tokio::test]
async fn units_form_proposes_the_lowest_number_that_no_child_uses() {
    let world = world();
    let cases = [
        ("U1", "5"),
        ("U1.2", "5"),
        ("U1.2.4", "6"),
        ("U1.2.4.5", "7"),
    ];
    for (parent, expected) in cases {
        let body = fragment(&world, &format!("?parent_path={parent}")).await;
        let segment = input_of(form_of(&body), "segment");
        assert!(
            segment.contains(&format!("value=\"{expected}\"")),
            "{parent}: {segment}"
        );
    }
}

#[tokio::test]
async fn units_form_does_not_propose_the_number_of_an_archived_sibling() {
    let world = world();
    let path = "/machines/M31/units/new";
    let reply = world.get_form(&world.engineer, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let segment = input_of(form_of(&reply.body), "segment").to_owned();
    assert!(
        segment.contains("value=\"3\""),
        "U1 and U2 use 1 and 2 (U2 is archived): {segment}"
    );
}

#[tokio::test]
async fn units_form_fragment_holds_the_hint_the_name_field_the_token_and_one_primary_button() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX, ("hx-target", "unit-form")];
        let reply = world
            .get(
                &world.engineer,
                "/machines/M7/units/new?parent_path=U1.2",
                &headers,
            )
            .await;
        let form = form_of(&reply.body);
        let label = format!("<label for=\"segment\">{}", text(lang, "field-segment"));
        assert!(form.contains(&label), "{lang}: {form}");
        let hint = format!(
            "<span class=\"field-hint\" id=\"segment-hint\">{}</span>",
            text(lang, "unit-segment-hint")
        );
        assert!(form.contains(&hint), "{lang}: {form}");
        let segment = input_of(form, "segment");
        assert!(segment.contains("name=\"segment\""), "{segment}");
        assert!(
            segment.contains("aria-describedby=\"segment-hint\""),
            "{segment}"
        );
        assert!(segment.contains("aria-required=\"true\""), "{segment}");
        assert!(segment.contains("maxlength=\"2\""), "{segment}");
        let label = format!("<label for=\"unit_name\">{}", text(lang, "field-unit-name"));
        assert!(form.contains(&label), "{lang}: {form}");
        let name = input_of(form, "unit_name");
        assert!(name.contains("name=\"name\""), "{name}");
        assert!(!name.contains("autofocus"), "{name}");
        let token = format!(
            "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
            world.engineer.csrf
        );
        assert!(form.contains(&token), "{form}");
        let button = format!(
            "<button type=\"submit\" id=\"unit-create-submit\" class=\"btn btn-primary\">{}</button>",
            text(lang, "unit-create-submit")
        );
        assert_eq!(form.matches(&button).count(), 1, "{lang}: {form}");
        assert_eq!(form.matches("btn-primary").count(), 1, "{form}");
        assert!(!form.contains("method=\"get\""));
    }
}

#[tokio::test]
async fn units_form_fragment_gives_the_focus_to_the_first_input() {
    let world = world();
    let body = fragment(&world, "?parent_path=U1.2").await;
    assert_eq!(body.matches("autofocus").count(), 1, "{body}");
    let segment = input_of(form_of(&body), "segment");
    assert!(segment.contains(" autofocus"), "{segment}");
}

#[tokio::test]
async fn units_form_fragment_post_to_the_collection_with_the_literal_hx_attributes() {
    let world = world();
    let body = fragment(&world, "?parent_path=U1.2").await;
    assert!(body.contains(FORM_TAG), "{body}");
    let pairs = hx_pairs(&body);
    let expected = [
        ("hx-boost", "true"),
        ("hx-target", "#unit-tree"),
        ("hx-swap", "innerHTML"),
        ("hx-push-url", "false"),
        ("hx-sync", "closest form:drop"),
    ];
    assert_eq!(pairs.len(), expected.len(), "{pairs:?}");
    for (name, value) in expected {
        assert!(
            pairs.contains(&(name.to_owned(), value.to_owned())),
            "{name}"
        );
    }
    for needle in ["M7", "U1.2", "Infeed", "Press line"] {
        assert_no_user_text_in_hx_attributes(&body, needle);
    }
    assert_clean_html(&body);
}

// --- Spec "All numbers in use" ------------------------------------------------------------------

#[tokio::test]
async fn units_form_without_a_free_number_shows_the_sentence_and_no_create_button() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let headers = [cookie_of(lang), HTMX, ("hx-target", "unit-form")];
        let reply = world
            .get(
                &world.engineer,
                "/machines/M30/units/new?parent_path=U1",
                &headers,
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let sentence = text(lang, "unit-no-free-segment");
        assert!(reply.body.contains(&sentence), "{lang}: {}", reply.body);
        let heading = text_with(lang, "unit-new-under", "path", "U1");
        assert!(
            reply.body.contains(&format!("<h2>{heading}</h2>")),
            "{lang}"
        );
        for forbidden in [
            "type=\"submit\"",
            &text(lang, "unit-create-submit"),
            "id=\"segment\"",
        ] {
            assert!(!reply.body.contains(forbidden), "{lang}: {forbidden}");
        }
    }
}

#[tokio::test]
async fn units_form_of_a_machine_with_a_full_sub_tree_still_proposes_a_main_number() {
    let world = world();
    let reply = world
        .get_form(&world.engineer, "/machines/M30/units/new")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("type=\"submit\""), "{}", reply.body);
    let segment = input_of(form_of(&reply.body), "segment");
    assert!(segment.contains("value=\"2\""), "{segment}");
}

// --- Errors of the query ------------------------------------------------------------------------

#[tokio::test]
async fn units_form_with_a_parent_of_depth_5_gives_422_e_plt_002_in_the_error_panel() {
    let world = world();
    let path = "/machines/M7/units/new?parent_path=U1.2.4.5.6";
    let reply = world
        .get(
            &world.engineer,
            path,
            &[ENGLISH, HTMX, ("hx-target", "unit-form")],
        )
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(reply.body.contains("E-PLT-002"), "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert!(!reply.body.contains("<form"), "{}", reply.body);
}

#[tokio::test]
async fn units_form_with_a_parent_that_is_not_live_gives_404_e_web_005() {
    let world = world();
    let cases = [
        "/machines/M7/units/new?parent_path=U1.9",
        "/machines/M31/units/new?parent_path=U2",
        "/machines/M99/units/new",
        "/machines/M12/units/new",
        "/machines/X7/units/new",
    ];
    for path in cases {
        let reply = world.get_form(&world.engineer, path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert!(reply.body.contains("E-WEB-005"), "{path}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{path}");
    }
}

#[tokio::test]
async fn units_form_gives_the_code_of_unit_path_for_a_bad_parent_and_422_for_a_bad_query() {
    let world = world();
    let cases = [
        ("parent_path=X", "E-PLT-017"),
        ("parent_path=", "E-PLT-017"),
        ("parent_path=U0", "E-PLT-017"),
        ("parent_path=U1.2.4.5.6.7", "E-PLT-002"),
        ("csrf_token=abc", "E-WEB-008"),
        ("other=1", "E-WEB-008"),
        ("parent_path=U1&parent_path=U2", "E-WEB-008"),
    ];
    for (query, code) in cases {
        let path = format!("/machines/M7/units/new?{query}");
        let reply = world.get_form(&world.engineer, &path).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        assert!(reply.body.contains(code), "{query}: {}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "#error-panel", "{query}");
    }
}

#[tokio::test]
async fn units_form_is_forbidden_for_an_operator() {
    let world = world();
    let reply = world
        .get_form(&world.operator, "/machines/M7/units/new")
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
    assert!(!reply.body.contains("<form"));
}

// --- The page and the content answer ------------------------------------------------------------

#[tokio::test]
async fn units_form_page_has_the_layout_the_heading_and_the_form_without_focus_on_the_heading() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.engineer,
                "/machines/M7/units/new?parent_path=U1.2",
                &[cookie_of(lang)],
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let title = text_with(lang, "unit-new-title", "code", "M7");
        assert!(reply.body.contains(&format!("<h1>{title}</h1>")), "{lang}");
        assert!(reply.body.contains(&format!(
            "<title>{title} · {}</title>",
            text(lang, "app-name")
        )));
        assert_eq!(reply.body.matches("<h1").count(), 1);
        assert!(reply.body.contains("class=\"topbar\""));
        let current = format!(
            "<a class=\"nav-link\" href=\"/machines\" aria-current=\"page\">{}</a>",
            text(lang, "nav-machines")
        );
        assert!(reply.body.contains(&current), "{lang}");
        let main = main_of(&reply.body);
        let heading = text_with(lang, "unit-new-under", "path", "U1.2");
        assert!(
            main.contains(&format!("<h2>{heading}</h2>")),
            "{lang}: {main}"
        );
        assert!(main.contains(PLAIN_FORM_TAG), "{lang}: {main}");
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn units_form_page_has_a_plain_form_because_the_page_has_no_unit_tree_region() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M7/units/new?parent_path=U1.2")
        .await;
    let main = main_of(&reply.body);
    assert!(!main.contains("id=\"unit-tree\""), "{main}");
    assert!(!main.contains("hx-"), "{main}");
    assert_eq!(
        main.matches("autofocus").count(),
        1,
        "a full page focuses the first input and not the heading: {main}"
    );
    assert!(input_of(form_of(main), "segment").contains(" autofocus"));
    assert!(
        main.contains("href=\"/machines/M7\""),
        "a link back to the machine: {main}"
    );
}

#[tokio::test]
async fn units_form_page_answers_an_htmx_request_for_another_target_with_the_page() {
    let world = world();
    for target in ["unit-tree", "error-panel", "other"] {
        let headers = [ENGLISH, HTMX, ("hx-target", target)];
        let reply = world
            .get(&world.engineer, "/machines/M7/units/new", &headers)
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{target}");
        assert!(reply.body.contains("<html"), "{target}: {}", reply.body);
        assert!(
            reply.body.contains("<h1>New unit of machine M7</h1>"),
            "{target}"
        );
    }
}

#[tokio::test]
async fn units_form_content_answer_has_the_title_the_heading_with_focus_and_the_field_next() {
    let world = world();
    let reply = world
        .get_content(&world.engineer, "/machines/M7/units/new?parent_path=U1.2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<title>New unit of machine M7 · Recipe editor</title>")
    );
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>New unit of machine M7</h1>")
    );
    let next = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/machines/M7/units/new?parent_path=U1.2\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(next), "{}", reply.body);
    for frame in ["class=\"topbar\"", "class=\"skip-link\"", "<main", "<html"] {
        assert!(!reply.body.contains(frame), "{frame}");
    }
    assert!(reply.body.contains(PLAIN_FORM_TAG), "{}", reply.body);
    assert_eq!(
        reply.body.matches("autofocus").count(),
        1,
        "only the heading takes the focus"
    );
}
