//! Tests of the layout of every full page, in both languages (tasks 5.2 to 5.4, design D5, D6).
//!
//! Each test names a scenario of the delta spec `web-ui`. The pages come from the router of the
//! fixture, so the tests read the real markup.

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::test_helpers::{assert_clean_html, field_value, set_cookie_of};
pub(super) use crate::test_support::get_page;
use crate::test_support::{Fixture, Reply, TestSession, icon_tag, text};

/// The languages of the tests with the value of the cookie `recipe_lang`.
pub(super) const LANGS: [(Lang, &str); 2] = [(Lang::Pl, "pl"), (Lang::En, "en")];
/// The three scripts that a page loads, in the order of `base.html`.
const SCRIPTS: [&str; 3] = [
    "<script src=\"/static/htmx.min.js\"></script>",
    "<script src=\"/static/row-version.js\" defer></script>",
    "<script src=\"/static/tree.js\" defer></script>",
];

// --- Helpers ------------------------------------------------------------------------------------

/// A fixture with the Administrator `user1` and a session of that user.
pub(super) fn administrator() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    (fixture, session)
}

/// Return the byte position of `needle` in `body`. The test fails when the text is missing.
fn at(body: &str, needle: &str) -> usize {
    assert!(body.contains(needle), "missing {needle} in {body}");
    body.find(needle).unwrap()
}

/// Assert the regions of `design-system.md` section 10.1 in their order.
pub(super) fn assert_layout(body: &str, lang: Lang, code: &str) {
    let skip = format!(
        "<a class=\"skip-link\" href=\"#content\">{}</a>",
        text(lang, "skip-to-content")
    );
    let order = [
        at(body, &skip),
        at(body, "<header class=\"topbar\">"),
        at(body, "<main id=\"content\">"),
        at(body, "<div id=\"error-panel\">"),
        at(
            body,
            "<div id=\"toast\" role=\"status\" aria-live=\"polite\">",
        ),
        at(body, "<h1"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    assert_eq!(body.matches("<h1").count(), 1);
    assert!(body.contains(&format!("<html lang=\"{code}\">")));
    let name = text(lang, "app-name");
    assert!(body.contains(&format!("<span class=\"app-name\">{name}</span>")));
    assert!(body.contains(&format!(" · {name}</title>")), "{body}");
    assert!(body.contains("<body hx-history=\"false\""));
    assert!(body.contains("<meta name=\"htmx-config\""));
}

/// Assert the two style sheets in their order, the three scripts, and no inline style.
pub(super) fn assert_assets(body: &str) {
    let app = at(body, "<link rel=\"stylesheet\" href=\"/static/app.css\">");
    let components = at(
        body,
        "<link rel=\"stylesheet\" href=\"/static/components.css\">",
    );
    assert!(app < components, "app.css comes first: {body}");
    assert_eq!(body.matches("<link").count(), 2, "{body}");
    assert_eq!(body.matches("<script").count(), 3, "{body}");
    let positions: Vec<usize> = SCRIPTS.iter().map(|tag| at(body, tag)).collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!body.contains("<style"));
    assert_clean_html(body);
}

// --- Spec "Page layout" -------------------------------------------------------------------------

#[tokio::test]
async fn page_layout_of_the_login_page() {
    let fixture = Fixture::new();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, None, code, "/login").await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_layout(&reply.body, lang, code);
        assert_assets(&reply.body);
        assert!(!reply.body.contains("hx-headers"));
        assert!(
            reply
                .body
                .contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>"),
            "the toast of the login page is not empty"
        );
    }
}

#[tokio::test]
async fn page_layout_of_the_profile_page() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me").await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_layout(&reply.body, lang, code);
        assert_assets(&reply.body);
        let headers = format!("hx-headers='{{\"X-CSRF-Token\":\"{}\"}}'", session.csrf);
        assert!(reply.body.contains(&headers), "{}", reply.body);
    }
}

#[tokio::test]
async fn page_layout_of_the_password_page() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me/password").await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_layout(&reply.body, lang, code);
        assert_assets(&reply.body);
    }
}

#[tokio::test]
async fn page_layout_of_the_user_list() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/users").await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_layout(&reply.body, lang, code);
        assert_assets(&reply.body);
    }
}

#[tokio::test]
async fn page_layout_top_bar_shows_the_login_and_the_logout_form() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me").await;
        let top = reply.body.split("</header>").next().unwrap();
        let signed_in = format!(
            "<span class=\"user-login\">{} <span class=\"data\">user1</span></span>",
            text(lang, "user-signed-in")
        );
        assert!(top.contains(&signed_in), "{top}");
        assert!(top.contains("<form method=\"post\" action=\"/logout\">"));
        assert!(top.contains(&format!(
            "<button type=\"submit\">{}</button>",
            text(lang, "logout-submit")
        )));
        let main = reply.body.split("</header>").nth(1).unwrap();
        assert!(!main.contains("action=\"/logout\""), "logout form in main");
    }
}

#[tokio::test]
async fn page_layout_login_page_has_no_logout_form_and_no_login() {
    let fixture = Fixture::new();
    let reply = get_page(&fixture, None, "en", "/login").await;
    assert!(!reply.body.contains("action=\"/logout\""));
    assert!(!reply.body.contains("user-login"));
}

#[tokio::test]
async fn page_layout_of_the_login_page_after_a_failed_login() {
    let fixture = Fixture::new();
    let reply = crate::test_support::post_pre_session(
        &fixture,
        "/login",
        "login=nobody&password=wrong+password+here",
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_layout(&reply.body, Lang::Pl, "pl");
    assert_assets(&reply.body);
    assert!(set_cookie_of(&reply, "__Host-recipe_csrf").is_some());
}

#[tokio::test]
async fn page_layout_of_the_full_error_page() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/users/nobody").await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let body = &reply.body;
        assert_layout(body, lang, code);
        assert_assets(body);
        let name = text(lang, "app-name");
        let title = text(lang, "error-page-title");
        assert!(body.contains(&format!("<title>E-WEB-005 · {title} · {name}</title>")));
        assert!(body.contains(&format!("<h1>{title}</h1>")), "{body}");
        let panel = at(body, "<div id=\"error-panel\">");
        let after_panel = body.get(panel..).unwrap();
        assert!(after_panel.contains("E-WEB-005"), "error not in the panel");
        let home = format!(
            "<a class=\"target-link\" href=\"/recipes\">{}</a>",
            text(lang, "error-page-home")
        );
        assert!(body.contains(&home), "{body}");
        assert!(!body.contains("nav-link"));
        assert!(!body.contains("<nav"));
        assert!(!body.contains("action=\"/lang\""));
        assert!(!body.contains("action=\"/logout\""));
        assert!(!body.contains("hx-headers"));
    }
}

// --- Specs "Navigation by permission", "Language switch", "Language labels" ---------------------

/// Return the markup of the navigation of a page, or an empty text.
fn nav_of(body: &str) -> String {
    let Some((_before, rest)) = body.split_once("<nav") else {
        return String::new();
    };
    rest.split("</nav>").next().unwrap().to_owned()
}

/// Return the markup of the language switch of a page, or an empty text.
fn switch_of(body: &str) -> String {
    let Some((_before, rest)) = body.split_once("<form class=\"lang-switch\"") else {
        return String::new();
    };
    rest.split("</form>").next().unwrap().to_owned()
}

/// Return the markup of the link `path` in the navigation.
fn link(path: &str, current: bool, word: &str) -> String {
    let aria = if current {
        " aria-current=\"page\""
    } else {
        ""
    };
    format!("<a class=\"nav-link\" href=\"{path}\"{aria}>{word}</a>")
}

#[tokio::test]
async fn page_nav_of_an_operator_has_recipes_machines_and_account_and_no_users() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me").await;
        let nav = nav_of(&reply.body);
        let label = text(lang, "nav-label");
        assert!(nav.starts_with(&format!(" aria-label=\"{label}\"><ul class=\"nav-list\">")));
        assert!(nav.contains(&link("/recipes", false, &text(lang, "nav-recipes"))));
        assert!(nav.contains(&link("/machines", false, &text(lang, "nav-machines"))));
        assert!(nav.contains(&link("/me", true, &text(lang, "nav-me"))));
        assert!(!nav.contains("/users"), "{nav}");
        assert_eq!(nav.matches("aria-current").count(), 1);
    }
}

#[tokio::test]
async fn page_nav_of_an_administrator_has_five_links_in_order() {
    let (fixture, session) = administrator();
    let reply = get_page(&fixture, Some(&session), "en", "/me").await;
    let nav = nav_of(&reply.body);
    let positions = [
        at(&nav, &link("/recipes", false, "Recipes")),
        at(&nav, &link("/machines", false, "Machines")),
        at(&nav, &link("/users", false, "Users")),
        at(&nav, &link("/audit", false, "Audit")),
        at(&nav, &link("/me", true, "My account")),
    ];
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{nav}");
    assert_eq!(nav.matches("<li").count(), 5, "{nav}");
}

#[tokio::test]
async fn page_nav_marks_the_machine_list_as_current_on_the_machine_list() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::ProcessEngineer, false);
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/machines").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let nav = nav_of(&reply.body);
        let word = text(lang, "nav-machines");
        assert!(nav.contains(&link("/machines", true, &word)), "{nav}");
        assert_eq!(nav.matches("aria-current").count(), 1, "{nav}");
    }
}

#[tokio::test]
async fn page_nav_marks_the_user_list_as_current_on_the_user_list() {
    let (fixture, session) = administrator();
    let reply = get_page(&fixture, Some(&session), "en", "/users").await;
    let nav = nav_of(&reply.body);
    assert!(nav.contains(&link("/users", true, "Users")), "{nav}");
    assert!(nav.contains(&link("/me", false, "My account")));
}

#[tokio::test]
async fn page_nav_marks_the_account_as_current_on_the_password_page() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let reply = get_page(&fixture, Some(&session), "en", "/me/password").await;
    let nav = nav_of(&reply.body);
    assert!(nav.contains(&link("/me", true, "My account")), "{nav}");
    assert!(nav.contains(&link("/recipes", false, "Recipes")), "{nav}");
    assert_eq!(nav.matches("aria-current").count(), 1);
}

#[tokio::test]
async fn page_nav_is_empty_while_the_password_must_change_but_logout_stays() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, true);
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me/password").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("<nav"));
        assert!(!reply.body.contains("nav-link"));
        let top = reply.body.split("</header>").next().unwrap();
        assert!(top.contains("action=\"/logout\""));
        assert!(top.contains(&format!(">{}</button>", text(lang, "logout-submit"))));
    }
}

#[tokio::test]
async fn page_switch_on_the_user_list_sends_next_with_the_query_and_the_token() {
    let (fixture, session) = administrator();
    let reply = get_page(&fixture, Some(&session), "en", "/users?q=ab").await;
    let switch = switch_of(&reply.body);
    assert!(
        switch.contains(" method=\"post\" action=\"/lang\""),
        "{switch}"
    );
    assert!(
        switch.contains(
            "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/users?q=ab\">"
        )
    );
    let token = format!(
        "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
        session.csrf
    );
    assert!(switch.contains(&token));
    assert!(!switch.contains("csrf_pre"));
    assert!(!switch.contains("hx-"), "{switch}");
    assert!(switch.contains(&format!(
        "aria-label=\"{}\"",
        text(Lang::En, "lang-switch-label")
    )));
}

#[tokio::test]
async fn page_switch_on_the_login_page_sends_the_nonce_of_the_cookie() {
    let fixture = Fixture::new();
    let reply = get_page(&fixture, None, "en", "/login").await;
    let switch = switch_of(&reply.body);
    let nonce = field_value(&switch, "csrf_pre");
    let cookie = set_cookie_of(&reply, "__Host-recipe_csrf").unwrap();
    assert!(
        cookie.starts_with(&format!("__Host-recipe_csrf={nonce}.")),
        "{cookie}"
    );
    assert!(
        switch.contains("<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/login\">")
    );
    assert!(!switch.contains("csrf_token"));
    assert_eq!(
        field_value(&reply.body.replace(&switch, ""), "csrf_pre"),
        nonce
    );
}

#[tokio::test]
async fn page_switch_is_absent_from_the_page_of_a_one_time_password() {
    let (fixture, session) = administrator();
    fixture.seed_account(2, Role::Operator, "correct horse battery", false);
    let reply = crate::test_support::post_fields(
        &fixture.router(),
        "/users/user2/reset-password",
        Some(&session),
        &[("cookie", "recipe_lang=en")],
        "",
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("id=\"one-time-password\""));
    assert!(!reply.body.contains("action=\"/lang\""));
    assert!(!reply.body.contains("lang-switch"));
    assert!(reply.body.contains("action=\"/logout\""));
}

#[tokio::test]
async fn page_switch_labels_name_each_language_in_that_language() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let english = get_page(&fixture, Some(&session), "en", "/me").await;
    let switch = switch_of(&english.body);
    assert!(switch.contains(
        "<button type=\"submit\" name=\"lang\" value=\"pl\" lang=\"pl\">Polski</button>"
    ));
    assert!(switch.contains(
        "<button type=\"submit\" name=\"lang\" value=\"en\" lang=\"en\" aria-current=\"true\">English</button>"
    ));
    let polish = get_page(&fixture, Some(&session), "pl", "/me").await;
    let switch = switch_of(&polish.body);
    assert!(switch.contains(
        "<button type=\"submit\" name=\"lang\" value=\"pl\" lang=\"pl\" aria-current=\"true\">Polski</button>"
    ));
    assert!(switch.contains(
        "<button type=\"submit\" name=\"lang\" value=\"en\" lang=\"en\">English</button>"
    ));
    assert_eq!(switch.matches("aria-current").count(), 1);
}

#[tokio::test]
async fn page_switch_escapes_quotes_in_next() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let reply = get_page(&fixture, Some(&session), "en", "/me?x='onfocus=alert(1)'").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .contains("name=\"next\" value=\"/me?x=&#39;onfocus=alert(1)&#39;\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("value=\"/me?x='"));
}

#[tokio::test]
async fn page_switch_after_a_failed_login_keeps_the_nonce_of_the_new_cookie() {
    let fixture = Fixture::new();
    let reply = crate::test_support::post_pre_session(
        &fixture,
        "/login",
        "login=user1&password=wrong",
        &[],
    )
    .await;
    assert!(reply.body.contains("<html lang="), "{}", reply.body);
    let switch = switch_of(&reply.body);
    assert!(
        switch.contains(" method=\"post\" action=\"/lang\""),
        "{switch}"
    );
    let nonce = field_value(&switch, "csrf_pre");
    let cookie = set_cookie_of(&reply, "__Host-recipe_csrf").unwrap();
    assert!(
        cookie.starts_with(&format!("__Host-recipe_csrf={nonce}.")),
        "{cookie}"
    );
    assert!(
        switch.contains("<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/login\">")
    );
    assert!(!switch.contains("csrf_token"));
}

// --- Task 5.4: user pages with the components ---------------------------------------------------

/// Send `POST path` with an empty form, as `session`, in the language `code`.
async fn post_page(
    fixture: &Fixture,
    session: &TestSession,
    code: &str,
    path: &str,
    htmx: bool,
) -> Reply {
    let cookie = format!("recipe_lang={code}");
    let mut headers = vec![("cookie", cookie.as_str())];
    if htmx {
        headers.push(("hx-request", "true"));
    }
    crate::test_support::post_fields(&fixture.router(), path, Some(session), &headers, "").await
}

/// A world with the Administrator `user1` and the inactive Operator `user2`.
async fn world_with_inactive_user() -> (Fixture, TestSession) {
    let (fixture, session) = administrator();
    fixture.seed_account(2, Role::Operator, "correct horse battery", false);
    crate::test_support::deactivate_user(&fixture, "user2").await;
    (fixture, session)
}

#[tokio::test]
async fn page_users_list_is_a_table_frame_with_a_caption_and_scoped_headers() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/users").await;
        let caption = text(lang, "users-title");
        let frame = format!(
            "<div class=\"table-wrap\" role=\"region\" tabindex=\"0\" aria-labelledby=\"users-caption\"><table><caption id=\"users-caption\">{caption}</caption>"
        );
        assert!(reply.body.contains(&frame), "{}", reply.body);
        assert_eq!(reply.body.matches("<th scope=\"col\">").count(), 4);
        assert_eq!(reply.body.matches("<th>").count(), 0);
    }
}

#[tokio::test]
async fn page_users_list_shows_the_state_of_a_user_by_icon_and_word() {
    let (fixture, session) = world_with_inactive_user().await;
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/users").await;
        let inactive = format!(
            "<span class=\"badge badge-inactive\">{}",
            icon_tag("user-inactive")
        );
        let word = format!("<span>{}</span></span>", text(lang, "user-status-inactive"));
        let at_inactive = at(&reply.body, &inactive);
        assert!(reply.body.get(at_inactive..).unwrap().contains(&word));
        let active = format!(
            "<span class=\"badge badge-active\">{}",
            icon_tag("user-active")
        );
        assert!(reply.body.contains(&active), "{}", reply.body);
    }
}

#[tokio::test]
async fn page_users_list_uses_target_links_for_the_stand_alone_links() {
    let (fixture, session) = administrator();
    let reply = get_page(&fixture, Some(&session), "en", "/users").await;
    assert!(
        reply
            .body
            .contains("<a class=\"target-link\" href=\"/users/new\">New user</a>")
    );
    assert!(!reply.body.contains(" pattern="));
    assert!(!reply.body.contains(" required"));
}

#[tokio::test]
async fn page_user_detail_shows_the_state_with_the_badge_and_a_result_area_without_a_role() {
    let (fixture, session) = world_with_inactive_user().await;
    let reply = get_page(&fixture, Some(&session), "en", "/users/user2").await;
    let badge = format!(
        "<span class=\"badge badge-inactive\">{}",
        icon_tag("user-inactive")
    );
    assert!(reply.body.contains(&badge), "{}", reply.body);
    assert!(reply.body.contains("<div id=\"user-result\"></div>"));
}

#[tokio::test]
async fn page_user_new_form_uses_the_field_macro_for_the_login() {
    let (fixture, session) = administrator();
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/users/new").await;
        let label = text(lang, "user-new-label-login");
        let word = text(lang, "form-required");
        let head = format!(
            "<div class=\"field\"><label for=\"login\">{label} <span class=\"field-required\">{word}</span></label>"
        );
        assert!(reply.body.contains(&head), "{}", reply.body);
        assert!(reply.body.contains("<input id=\"login\" name=\"login\" type=\"text\" class=\"data\" autocapitalize=\"none\" spellcheck=\"false\" maxlength=\"64\" autocomplete=\"off\" aria-required=\"true\" aria-describedby=\"login-hint\">"));
        let select = format!(
            "<div class=\"field\"><label for=\"role\">{}</label><select id=\"role\" name=\"role\">",
            text(lang, "user-new-label-role")
        );
        assert!(reply.body.contains(&select), "{}", reply.body);
        assert!(!reply.body.contains(" minlength="));
    }
}

#[tokio::test]
async fn page_user_command_success_with_htmx_puts_the_sentence_into_the_toast() {
    let (fixture, session) = world_with_inactive_user().await;
    for (lang, code) in LANGS {
        let reply = post_page(&fixture, &session, code, "/users/user2/activate", true).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let args = |key: &str| {
            let bundles = Bundles::load().unwrap();
            let mut args = fluent::FluentArgs::new();
            args.set("login", "user2");
            Texts::new(&bundles, lang).with(key, &args)
        };
        let icon = format!(
            "{}<path d=\"M5 12.5l4.5 4.5L19 7\"/></svg>",
            icon_tag("alert-success")
        );
        let toast = format!(
            "<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">{icon}<span>{}</span></div></div>",
            args("user-activated")
        );
        assert!(reply.body.contains(&toast), "{}", reply.body);
        assert!(
            reply
                .body
                .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>")
        );
        assert_eq!(reply.body.matches("hx-swap-oob").count(), 2);
        assert!(!reply.body.contains("role="), "{}", reply.body);
        assert!(!reply.body.contains("<html"));
        // The sentence appears once: the content for #user-result holds only the back link.
        assert_eq!(reply.body.matches(&args("user-activated")).count(), 1);
        crate::test_support::deactivate_user(&fixture, "user2").await;
    }
}

#[tokio::test]
async fn page_user_command_success_without_htmx_puts_the_sentence_into_the_status_region() {
    let (fixture, session) = world_with_inactive_user().await;
    let reply = post_page(&fixture, &session, "en", "/users/user2/activate", false).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let toast = "<div id=\"toast\" role=\"status\" aria-live=\"polite\"><div class=\"toast\">";
    let start = at(&reply.body, toast);
    let region = reply.body.get(start..).unwrap();
    assert!(region.starts_with(&format!("{toast}{}", icon_tag("alert-success"))));
    assert_eq!(reply.body.matches("User user2 is active.").count(), 1);
    assert!(
        region
            .split("</main>")
            .next()
            .unwrap()
            .contains("User user2 is active.")
    );
    assert!(!reply.body.contains("hx-swap-oob"));
}

#[tokio::test]
async fn page_user_one_time_password_stays_in_the_result_with_a_notice_without_a_role() {
    let (fixture, session) = administrator();
    fixture.seed_account(2, Role::Operator, "correct horse battery", false);
    for (lang, code) in LANGS {
        let reply = post_page(
            &fixture,
            &session,
            code,
            "/users/user2/reset-password",
            true,
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let icon = format!(
            "{}<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 11v6\"/><path d=\"M12 7.5v.01\"/></svg>",
            icon_tag("alert-info")
        );
        let notice = format!(
            "<div class=\"alert alert-info\">{icon}<span>{}</span></div>",
            text(lang, "user-password-once")
        );
        assert!(reply.body.contains(&notice), "{}", reply.body);
        let password = at(&reply.body, "id=\"one-time-password\"");
        let toast = at(&reply.body, "<div id=\"toast\"");
        assert!(password < toast, "the password stays in the result");
        assert!(!reply.body.contains("role="));
    }
}

#[tokio::test]
async fn page_password_page_shows_the_must_change_notice_without_a_role() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, true);
    for (lang, code) in LANGS {
        let reply = get_page(&fixture, Some(&session), code, "/me/password").await;
        let main = reply.body.split("<main").nth(1).unwrap();
        let notice = format!(
            "<div class=\"alert alert-info\">{}<circle",
            icon_tag("alert-info")
        );
        assert!(main.contains(&notice), "{main}");
        assert!(main.contains(&format!(
            "<span>{}</span></div>",
            text(lang, "password-must-change")
        )));
        assert!(!main.contains("role=\"note\""));
        assert!(!main.contains("class=\"notice\""));
    }
}

#[tokio::test]
async fn page_password_changed_fragment_puts_its_sentence_into_the_toast() {
    let fixture = Fixture::new();
    fixture.seed_account(1, Role::Operator, "correct horse battery", true);
    let session = fixture.session_of(1);
    let body = "current_password=correct+horse+battery&password=another+long+passphrase";
    let cookie = format!("{}; recipe_lang=en", session.cookie);
    let reply = crate::test_support::post_fields(
        &fixture.router(),
        "/me/password",
        Some(&session),
        &[("cookie", cookie.as_str()), ("hx-request", "true")],
        body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply
            .body
            .starts_with("<div id=\"toast\" hx-swap-oob=\"innerHTML\"><div class=\"toast\">")
    );
    assert!(reply.body.contains(&text(Lang::En, "password-changed")));
    assert!(
        reply
            .body
            .contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>")
    );
    assert!(!reply.body.contains("role="));
}

#[tokio::test]
async fn page_switch_is_absent_from_the_full_error_page_of_a_get() {
    let (fixture, session) = administrator();
    let reply = get_page(&fixture, Some(&session), "en", "/users/nobody").await;
    assert!(switch_of(&reply.body).is_empty());
}
