//! The sweep of every new page of P3.4 in `pl` and `en` (task 13.2, design D10).
//!
//! The sweep reads the approval preview, the diff page, the version page with its action area, the
//! history page, the audit list, the audit entry, the security events, the recipe list with its
//! deleted rows, and the recipe page. It also reads the HTMX content answers of the approve and the
//! retire `POST`. The states are richer than the states of `sweep_tests`: a blocked preview, an
//! outdated parameter, a paged section, an empty and a filled event list, a deleted recipe, and a
//! filter that the page refuses.
//!
//! The sweep reads each page as a full page only. A boosted `GET` gets the same `main` from the same
//! template, and the shell comes from `Layout::for_session` and `base.html`. So the full page
//! covers the boosted answer.
//!
//! Each check is a function that returns a list of problems. The sweep tests assert that the list
//! is empty. The negative tests give each function a hand-made bad page and assert the problem.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr};

use axum::http::StatusCode;
use recipe_app::blocking::AppSettings;
use recipe_app::identity::new_request_id;
use recipe_app::ports::SecurityEventSink as _;
use recipe_domain::audit::{SecurityEvent, UserAgent};

use super::audit::test_world::{AuditWorld, audit_world};
use super::page_tests::{LANGS, assert_assets, assert_layout};
use super::recipe_sweep_tests::{
    Answer as RecipeAnswer, assert_literal, assert_urls_without_token, attribute_values, forms,
};
use super::sweep_tests::get_page_patterns;
use super::test_helpers::{CSP, Tag, assert_clean_html, hx_attributes, small_pages, tags};
use crate::components_tests::assert_fixed_svg_tags;
use crate::i18n::Lang;
use crate::test_support::plant::{HTMX, TARGET_CONTENT, cookie_of, main_of};
use crate::test_support::{Fixture, Reply, TestSession, get_page, post_fields};

/// The route pattern of the approval preview.
const APPROVE: &str = "/recipes/{ref}/versions/{n}/approve";
/// The route pattern of the diff page.
const DIFF: &str = "/recipes/{ref}/versions/{n}/diff/{m}";
/// The route pattern of the version page.
const VERSION: &str = "/recipes/{ref}/versions/{n}";
/// The route pattern of the history page.
const HISTORY: &str = "/recipes/{ref}/history";
/// The route pattern of the recipe list. Its restore form of a deleted row is new.
const RECIPES: &str = "/recipes";
/// The route pattern of the recipe page. Its delete form is new.
const RECIPE: &str = "/recipes/{ref}";
/// The route pattern of the audit list.
const AUDIT: &str = "/audit";
/// The route pattern of the audit entry.
const ENTRY: &str = "/audit/{id}";
/// The route pattern of the security events.
const EVENTS: &str = "/security-events";
/// The path of the recipe list with the deleted recipes. The row `R-700` holds the restore form.
const DELETED_LIST: &str = "/recipes?show_deleted=1";
/// The route pattern of the approve `POST`.
const APPROVE_POST: &str = "POST /recipes/{ref}/versions/{n}/approve";
/// The route pattern of the retire `POST`.
const RETIRE_POST: &str = "POST /recipes/{ref}/versions/{n}/retire";
/// The full `GET` pages of the sweep. The test checks that each one has an answer.
const SWEPT: [&str; 9] = [
    APPROVE, DIFF, VERSION, HISTORY, RECIPES, RECIPE, AUDIT, ENTRY, EVENTS,
];
/// The path of the preview of the draft version 2 of `R-500`.
const R500_APPROVE: &str = "/recipes/R-500/versions/2/approve";
/// The path of the diff of the draft version 2 of `R-500` against version 1.
const R500_DIFF: &str = "/recipes/R-500/versions/2/diff/1";
/// The pages that the Administrator reads in the world with the approval data.
const ADMIN_PAGES: [(&str, &str); 15] = [
    (APPROVE, R500_APPROVE),
    (DIFF, R500_DIFF),
    (VERSION, "/recipes/R-500/versions/1"),
    (VERSION, "/recipes/R-500/versions/2"),
    (VERSION, "/recipes/R-600/versions/1"),
    (HISTORY, "/recipes/R-500/history"),
    (HISTORY, "/recipes/R-600/history"),
    (HISTORY, "/recipes/R-700/history"),
    (RECIPES, DELETED_LIST),
    (RECIPE, "/recipes/R-500"),
    (RECIPE, "/recipes/R-700"),
    (AUDIT, "/audit"),
    (AUDIT, "/audit?action=VersionApproved"),
    (EVENTS, "/security-events"),
    (EVENTS, "/security-events?kind=forbidden"),
];
/// The pages that the ProcessEngineer `anowak` reads. He edited the draft, so the preview blocks.
const ENGINEER_PAGES: [(&str, &str); 3] = [
    (APPROVE, R500_APPROVE),
    (VERSION, "/recipes/R-500/versions/2"),
    (AUDIT, "/audit"),
];
/// The pages that a filter refuses with `422 E-WEB-008`. The page shows the form again.
const REFUSED_PAGES: [(&str, &str); 2] = [
    (AUDIT, "/audit?from=not-a-date"),
    (EVENTS, "/security-events?client_ip=not-an-address"),
];
/// The ids that the shell of a full page holds. A content answer refers to them.
const SHELL_IDS: [&str; 3] = ["toast", "error-panel", "content"];
/// The `aria-*` attributes whose value is a list of ids.
const ARIA_REFERENCES: [&str; 3] = ["aria-describedby", "aria-labelledby", "aria-controls"];
/// The attributes that a field of a form must not have (design-system.md section 10.5).
const FORBIDDEN_ATTRIBUTES: [&str; 4] = ["required", "pattern", "minlength", "placeholder"];
/// The values of `scope` that a table header may have.
const SCOPES: [&str; 4] = ["col", "row", "colgroup", "rowgroup"];
/// The types of `input` that need no label.
const UNLABELLED_TYPES: [&str; 5] = ["hidden", "submit", "button", "reset", "image"];
/// The attributes whose value is a URL, for the count of the URL test.
const URL_ATTRIBUTES: [&str; 2] = ["href", "action"];
/// The `hx-target` literals that the swept pages hold.
const TARGETS: [&str; 3] = ["#content", "#parameter-form", "#recipe-name"];
/// The `hx-*` literals that the swept pages hold.
///
/// `assert_literal` of the recipe sweep decides which literal is valid. The sweep also asserts that
/// each pair of this list is on a page, so a removed link or form fails the sweep.
const HELD_LITERALS: [(&str, &[&str]); 7] = [
    ("hx-boost", &["true"]),
    ("hx-target", &TARGETS),
    ("hx-swap", &["innerHTML"]),
    ("hx-push-url", &["true", "false"]),
    ("hx-sync", &["closest form:drop"]),
    ("hx-swap-oob", &["true", "innerHTML"]),
    ("hx-history", &["false"]),
];
/// The client address of the seeded security event.
const EVENT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 77));
/// The status of an answer that refuses its input.
const REFUSED: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
/// The count of diff parameters on each page of the paged world.
const ONE_PER_PAGE: u32 = 1;

// --- The checks ---------------------------------------------------------------------------------

/// Return the level of a heading start tag, or `None` for any other tag.
fn heading_level(tag: &Tag) -> Option<u8> {
    let digit = tag.name.strip_prefix('h').filter(|_| !tag.closing)?;
    let level = digit.parse::<u8>().ok()?;
    (1..=6).contains(&level).then_some(level)
}

/// Return the problems of the headings: not exactly one `h1`, or a level that skips one.
fn heading_problems(tags: &[Tag]) -> Vec<String> {
    let levels: Vec<u8> = tags.iter().filter_map(heading_level).collect();
    let mut problems = Vec::new();
    let ones = levels.iter().filter(|level| **level == 1).count();
    if ones != 1 {
        problems.push(format!("{ones} h1 elements"));
    }
    let mut previous = 0_u8;
    for level in levels {
        if level > previous.saturating_add(1) {
            problems.push(format!("h{level} follows h{previous}"));
        }
        previous = level;
    }
    problems
}

/// Return the problems of the tables: a table with no `caption`, and a `th` with no valid `scope`.
fn table_problems(tags: &[Tag]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut captions: Vec<bool> = Vec::new();
    for tag in tags {
        match (tag.name.as_str(), tag.closing) {
            ("table", false) => captions.push(false),
            ("caption", false) => match captions.last_mut() {
                Some(has_caption) => *has_caption = true,
                None => problems.push("caption outside a table".to_owned()),
            },
            ("table", true) => {
                if captions.pop() == Some(false) {
                    problems.push("table with no caption".to_owned());
                }
            }
            ("th", false) if !tag.attribute("scope").is_some_and(|s| SCOPES.contains(&s)) => {
                problems.push(format!("th with scope {:?}", tag.attribute("scope")));
            }
            _other => {}
        }
    }
    problems
}

/// Return true when the tag is a field that needs a label.
fn is_labelled_field(tag: &Tag) -> bool {
    let kind = tag.attribute("type").unwrap_or_default();
    match tag.name.as_str() {
        "select" | "textarea" => !tag.closing,
        "input" => !UNLABELLED_TYPES
            .iter()
            .any(|t| t.eq_ignore_ascii_case(kind)),
        _other => false,
    }
}

/// Return the problems of the labels: a field that no `label for` names and no `label` wraps.
fn label_problems(tags: &[Tag]) -> Vec<String> {
    let is_label = |tag: &&Tag| tag.name == "label" && !tag.closing;
    let targets: BTreeSet<&str> = tags
        .iter()
        .filter(is_label)
        .filter_map(|tag| tag.attribute("for"))
        .collect();
    let mut open_labels = 0_usize;
    let mut problems = Vec::new();
    for tag in tags {
        if tag.name == "label" {
            open_labels = if tag.closing {
                open_labels.saturating_sub(1)
            } else {
                open_labels.saturating_add(1)
            };
        }
        let id = tag.attribute("id").unwrap_or_default();
        let named = !id.is_empty() && targets.contains(id);
        if is_labelled_field(tag) && open_labels == 0 && !named {
            problems.push(format!("<{}> id={id:?} has no label", tag.name));
        }
    }
    problems
}

/// Return the problems of the `aria-*` references: an empty value, and an id that no element has.
///
/// `shell` lists the ids that exist outside the answer, for a content answer that the browser
/// swaps into a page.
fn aria_problems(tags: &[Tag], shell: &[&str]) -> Vec<String> {
    let page_ids = tags.iter().filter_map(|tag| tag.attribute("id"));
    let ids: BTreeSet<&str> = page_ids.chain(shell.iter().copied()).collect();
    let mut problems = Vec::new();
    for (tag, name) in tags
        .iter()
        .flat_map(|tag| ARIA_REFERENCES.map(|name| (tag, name)))
    {
        let Some(value) = tag.attribute(name) else {
            continue;
        };
        if value.trim().is_empty() {
            problems.push(format!("{name} is empty"));
        }
        let missing = value.split_whitespace().filter(|id| !ids.contains(id));
        problems.extend(missing.map(|id| format!("{name} names the missing id {id}")));
    }
    problems
}

/// Return the problems of the attributes `required`, `pattern`, `minlength`, and `placeholder`.
fn forbidden_problems(tags: &[Tag]) -> Vec<String> {
    let pairs = tags
        .iter()
        .flat_map(|tag| FORBIDDEN_ATTRIBUTES.map(|name| (tag, name)));
    let found = pairs.filter(|(tag, name)| tag.attribute(name).is_some());
    found
        .map(|(tag, name)| format!("<{}> has the attribute {name}", tag.name))
        .collect()
}

/// Return the forms of `body` that hold the field `csrf_token` and do not send `POST`.
///
/// A form with no `method` sends `GET`. A `GET` form puts the token into the URL (ADR-0037 h).
fn form_problems(body: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for (_start, block) in forms(body) {
        let parsed = tags(block);
        let method = parsed.first().and_then(|form| form.attribute("method"));
        let sends_post = method.is_some_and(|method| method.eq_ignore_ascii_case("post"));
        let has_token =
            |tag: &Tag| tag.name == "input" && tag.attribute("name") == Some("csrf_token");
        if parsed.iter().any(has_token) && !sends_post {
            problems.push(format!("a form with a token sends {method:?}"));
        }
    }
    problems
}

// --- The answers of the sweep -----------------------------------------------------------------

/// One answer of the sweep.
struct Answer {
    /// The language, the method, the path, and the state.
    what: String,
    /// The language of the answer.
    lang: (Lang, &'static str),
    /// The route pattern, or the method and the pattern of a `POST`.
    pattern: &'static str,
    /// The status, the headers, and the body of the answer.
    reply: Reply,
    /// The status that the sweep expects.
    expected: StatusCode,
    /// True for a full page, false for a fragment that the browser swaps into a page.
    full: bool,
}

impl Answer {
    /// Return the tags of the body.
    fn tags(&self) -> Vec<Tag> {
        tags(&self.reply.body)
    }

    /// Return the answer in the form of the recipe sweep, so that its shared checks can read it.
    fn shared(&self) -> RecipeAnswer {
        let Reply {
            status,
            headers,
            body,
        } = &self.reply;
        let reply = Reply {
            status: *status,
            headers: headers.clone(),
            body: body.clone(),
        };
        RecipeAnswer {
            what: self.what.clone(),
            reply,
            statuses: vec![self.expected],
        }
    }

    /// Return true when the answer must hold one `h1`: a full page or a content answer.
    fn has_heading(&self) -> bool {
        self.full || self.expected == StatusCode::OK
    }

    /// Return the ids that exist outside the answer.
    fn shell(&self) -> &'static [&'static str] {
        if self.full { &[] } else { &SHELL_IDS }
    }
}

/// Sends the requests of one world in one language and keeps the answers.
struct Reader<'a> {
    /// The world that answers the requests.
    fixture: &'a Fixture,
    /// The language of the requests.
    lang: (Lang, &'static str),
    /// The answers that the reader kept, in the order of the requests.
    answers: Vec<Answer>,
}

impl<'a> Reader<'a> {
    /// Start a reader with no answer.
    fn new(fixture: &'a Fixture, lang: (Lang, &'static str)) -> Self {
        Self {
            fixture,
            lang,
            answers: Vec::new(),
        }
    }

    /// Keep one answer.
    fn keep(&mut self, pattern: &'static str, what: String, reply: Reply, expected: StatusCode) {
        let lang = self.lang;
        let full = !pattern.starts_with("POST");
        self.answers.push(Answer {
            what,
            lang,
            pattern,
            reply,
            expected,
            full,
        });
    }

    /// Read a full page as `session`. A refused filter gives `422`, any other page `200`.
    async fn get(&mut self, session: &TestSession, (pattern, path): (&'static str, &str)) {
        let refused = REFUSED_PAGES.iter().any(|(_pattern, known)| *known == path);
        let expected = if refused { REFUSED } else { StatusCode::OK };
        let reply = get_page(self.fixture, Some(session), self.lang.1, path).await;
        let what = format!("{} GET {path}", self.lang.1);
        self.keep(pattern, what, reply, expected);
    }

    /// Send an HTMX `POST` that targets `main` as `session`.
    async fn post(
        &mut self,
        session: &TestSession,
        request: (&'static str, &str, &str),
        expected: StatusCode,
    ) {
        let (pattern, path, body) = request;
        let headers = [cookie_of(self.lang.0), HTMX, TARGET_CONTENT];
        let router = self.fixture.router();
        let reply = post_fields(&router, path, Some(session), &headers, body).await;
        let what = format!("{} POST {path} {body}", self.lang.1);
        self.keep(pattern, what, reply, expected);
    }
}

/// Return the ids of the audit entries that the list `/audit` shows on its first page.
fn entry_ids(list: &Reply) -> Vec<u64> {
    let rows = main_of(&list.body).split("href=\"/audit/").skip(1);
    rows.filter_map(|rest| rest.split('"').next()?.parse().ok())
        .collect()
}

/// Record one `Forbidden` event, so that the event list has a row.
async fn record_event(world: &AuditWorld) {
    let agent = Some(UserAgent::from("Sweep/1.0"));
    let id = new_request_id().unwrap();
    let event = SecurityEvent::forbidden(world.data.admin_id, "Op", id, Some(EVENT_ADDRESS), agent);
    world.fixture.sink.record(event).await.unwrap();
}

/// Read the pages of the world with the approval data.
async fn world_answers(lang: (Lang, &'static str)) -> Vec<Answer> {
    let world = audit_world().await;
    record_event(&world).await;
    let (admin, anowak) = (&world.data.admin, &world.data.anowak);
    let mut reader = Reader::new(&world.fixture, lang);
    for page in ADMIN_PAGES.into_iter().chain(REFUSED_PAGES) {
        reader.get(admin, page).await;
    }
    for page in ENGINEER_PAGES {
        reader.get(anowak, page).await;
    }
    let ids = entry_ids(&world.get_en(admin, "/audit").await);
    assert!(ids.len() >= 5, "the seed writes entries: {ids:?}");
    for id in ids {
        reader.get(admin, (ENTRY, &format!("/audit/{id}"))).await;
    }
    reader.answers
}

/// Read the large preview, the paged diff, and the empty event list.
async fn paged_answers(lang: (Lang, &'static str)) -> Vec<Answer> {
    let (fixture, data) = small_pages(ONE_PER_PAGE, AppSettings::default().blocking).await;
    let mut reader = Reader::new(&fixture, lang);
    let rv = data.r500_v2_row_version;
    let paged_preview = format!("{R500_APPROVE}?kind=limits&page=2&rv={rv}");
    let paged_diff = format!("{R500_DIFF}?kind=limits&page=2");
    let pages = [
        (APPROVE, R500_APPROVE),
        (EVENTS, "/security-events"),
        (APPROVE, &paged_preview),
        (DIFF, &paged_diff),
    ];
    for page in pages {
        reader.get(&data.admin, page).await;
    }
    reader.answers
}

/// Read the outdated preview, the refused approval, the approval, and the retire.
async fn command_answers(lang: (Lang, &'static str)) -> Vec<Answer> {
    let world = audit_world().await;
    let (data, anowak) = (&world.data, world.data.anowak_user());
    let row_version = data.r500_v2_row_version;
    let rv = world.fixture.add_zone_to_r500(&anowak, row_version).await;
    let (mut reader, admin) = (Reader::new(&world.fixture, lang), &data.admin);
    reader.get(admin, (APPROVE, R500_APPROVE)).await;
    let body = format!("row_version={rv}");
    let request = (APPROVE_POST, R500_APPROVE, body.as_str());
    reader.post(admin, request, REFUSED).await;
    let body = format!("row_version={rv}&allow_outdated=true");
    let request = (APPROVE_POST, R500_APPROVE, body.as_str());
    reader.post(admin, request, StatusCode::OK).await;
    let body = format!("row_version={}", world.row_version_of("R-500", 2).await);
    let request = (
        RETIRE_POST,
        "/recipes/R-500/versions/2/retire",
        body.as_str(),
    );
    reader.post(admin, request, StatusCode::OK).await;
    reader.answers
}

/// Collect every answer of the sweep in both languages.
async fn answers() -> Vec<Answer> {
    let mut all = Vec::new();
    for lang in LANGS {
        all.extend(world_answers(lang).await);
        all.extend(paged_answers(lang).await);
        all.extend(command_answers(lang).await);
    }
    all
}

/// Assert that `problems` is empty.
fn assert_none(problems: &[String], what: &str) {
    assert!(problems.is_empty(), "{what}: {problems:?}");
}

/// Assert that `check` finds no problem in any answer.
fn assert_clean(answers: &[Answer], check: impl Fn(&Answer) -> Vec<String>) {
    for answer in answers {
        assert_none(&check(answer), &answer.what);
    }
}

/// Return how many tags of the answers satisfy `keep`. A sweep that sees none checks nothing.
fn count_tags(answers: &[Answer], keep: impl Fn(&Tag) -> bool) -> usize {
    let all = answers.iter().flat_map(Answer::tags);
    all.filter(|tag| keep(tag)).count()
}

// --- The sweep tests ----------------------------------------------------------------------------

#[tokio::test]
async fn approval_sweep_every_new_page_has_an_answer_in_both_languages() {
    let patterns = get_page_patterns();
    for pattern in SWEPT {
        assert!(patterns.contains(&pattern), "{pattern} is no GET page");
    }
    let answers = answers().await;
    for (_lang, code) in LANGS {
        for pattern in SWEPT.into_iter().chain([APPROVE_POST, RETIRE_POST]) {
            let has = |a: &Answer| a.pattern == pattern && a.lang.1 == code;
            assert!(answers.iter().any(has), "{code}: no answer for {pattern}");
        }
    }
}

#[tokio::test]
async fn approval_sweep_the_list_of_deleted_recipes_shows_the_restore_form_in_both_languages() {
    let answers = answers().await;
    for (_lang, code) in LANGS {
        let is_list = |a: &&Answer| a.lang.1 == code && a.what.ends_with(DELETED_LIST);
        let list = answers.iter().find(is_list).unwrap();
        let restore = "action=\"/recipes/R-700/restore\"";
        assert!(list.reply.body.contains(restore), "{code}: no restore form");
    }
}

#[tokio::test]
async fn approval_sweep_every_answer_has_status_csp_and_clean_markup_in_both_languages() {
    for answer in answers().await {
        let (what, reply, lang) = (&answer.what, &answer.reply, answer.lang);
        assert_eq!(reply.status, answer.expected, "{what}: {}", reply.body);
        assert_eq!(reply.header("content-security-policy"), CSP, "{what}");
        assert_clean_html(&reply.body);
        assert_fixed_svg_tags(&reply.body);
        if answer.full {
            assert_assets(&reply.body);
            assert_layout(&reply.body, lang.0, lang.1);
        } else {
            assert!(!reply.body.contains("<script"), "{what}: script");
        }
    }
}

#[tokio::test]
async fn approval_sweep_each_hx_value_is_a_literal_of_the_list() {
    let mut seen = BTreeSet::new();
    for answer in answers().await {
        for attribute in hx_attributes(&answer.reply.body) {
            assert_literal(&attribute, &answer.what);
            seen.insert((attribute.name, attribute.value));
        }
    }
    let listed = HELD_LITERALS
        .iter()
        .flat_map(|(name, values)| values.iter().map(|v| (*name, *v)));
    let held = |(name, value): &(&str, &str)| seen.contains(&(name.to_string(), value.to_string()));
    let stale: Vec<_> = listed.filter(|pair| !held(pair)).collect();
    assert!(stale.is_empty(), "no answer holds: {stale:?}");
}

#[tokio::test]
async fn approval_sweep_no_url_holds_the_csrf_token_or_the_session_value() {
    let world = audit_world().await;
    // The token and the cookie of a user number are fixed (`test_support.rs`). So the secrets of
    // this world equal the secrets of the worlds that `answers()` builds.
    let sessions = [&world.data.admin, &world.data.anowak, &world.operator];
    let values = sessions.map(|s| s.cookie.split_once('=').unwrap().1);
    let tokens = sessions.map(|s| s.csrf.as_str());
    let secrets = [tokens, values].concat();
    let answers = answers().await;
    for answer in &answers {
        let shared = answer.shared();
        for secret in &secrets {
            assert_urls_without_token(&shared, secret);
        }
    }
    let urls_of = |answer: &Answer| {
        let count = |name: &&str| attribute_values(&answer.reply.body, name).len();
        URL_ATTRIBUTES.iter().map(count).sum::<usize>()
    };
    let links: usize = answers.iter().map(urls_of).sum();
    assert!(links > 1000, "only {links} href and action values");
}

#[tokio::test]
async fn approval_sweep_each_form_with_a_token_sends_post_and_a_get_form_holds_none() {
    let answers = answers().await;
    assert_clean(&answers, |answer| form_problems(&answer.reply.body));
    let all = answers.iter().flat_map(|answer| forms(&answer.reply.body));
    let (with_token, get_forms) = all.fold((0_usize, 0_usize), |(token, get), (tag, block)| {
        let has_token = usize::from(block.contains("name=\"csrf_token\""));
        let is_get = usize::from(tag.contains(" method=\"get\""));
        (token.saturating_add(has_token), get.saturating_add(is_get))
    });
    assert!(with_token > 100, "only {with_token} token forms");
    assert!(get_forms > 0, "the sweep saw no GET form");
}

#[tokio::test]
async fn approval_sweep_each_page_has_one_h1_and_no_skipped_heading_level() {
    let all = answers().await;
    let headed: Vec<Answer> = all.into_iter().filter(Answer::has_heading).collect();
    assert_clean(&headed, |answer| heading_problems(&answer.tags()));
    assert!(headed.len() > 60, "{} answers with a heading", headed.len());
}

#[tokio::test]
async fn approval_sweep_each_table_has_a_caption_and_each_th_a_scope() {
    let answers = answers().await;
    assert_clean(&answers, |answer| table_problems(&answer.tags()));
    let tables = count_tags(&answers, |tag| tag.name == "table" && !tag.closing);
    let headers = count_tags(&answers, |tag| tag.name == "th" && !tag.closing);
    assert!(tables > 40 && headers > 100, "{tables} {headers}");
}

#[tokio::test]
async fn approval_sweep_each_field_has_a_label() {
    let answers = answers().await;
    assert_clean(&answers, |answer| label_problems(&answer.tags()));
    let fields = count_tags(&answers, is_labelled_field);
    assert!(fields > 20, "the sweep checked only {fields} fields");
}

#[tokio::test]
async fn approval_sweep_each_aria_reference_names_an_existing_id() {
    let answers = answers().await;
    assert_clean(&answers, |a| aria_problems(&a.tags(), a.shell()));
    let referring = |tag: &Tag| ARIA_REFERENCES.iter().any(|n| tag.attribute(n).is_some());
    let references = count_tags(&answers, referring);
    assert!(references > 50, "too few references: {references}");
}

#[tokio::test]
async fn approval_sweep_no_tag_has_required_pattern_minlength_or_placeholder() {
    let answers = answers().await;
    assert_clean(&answers, |answer| forbidden_problems(&answer.tags()));
    let inputs = count_tags(&answers, |tag| tag.name == "input");
    assert!(inputs > 20, "the sweep saw only {inputs} inputs");
}

// --- The checks find a bad page -----------------------------------------------------------------

/// Assert that each case of `cases` gives a problem that holds the wanted text.
///
/// A case is a bad page and the problem. A check that finds nothing in a bad page would let the
/// sweep pass on a bad answer.
fn reject(check: impl Fn(&[Tag]) -> Vec<String>, cases: &[(&str, &str)]) {
    for (html, wanted) in cases {
        let problems = check(&tags(html));
        let found = problems.iter().any(|problem| problem.contains(wanted));
        assert!(found, "{wanted:?} is not in {problems:?} for {html}");
    }
}

#[test]
fn approval_sweep_check_finds_two_h1_none_and_a_skipped_level() {
    let cases = [
        ("<h1>A</h1><h1>B</h1>", "2 h1 elements"),
        ("<h2>A</h2>", "0 h1 elements"),
        ("<h1>A</h1><h3>B</h3>", "h3 follows h1"),
        ("<h2>A</h2><h1>B</h1>", "h2 follows h0"),
        ("<!-- <h1>A</h1> -->", "0 h1 elements"),
    ];
    reject(heading_problems, &cases);
}

#[test]
fn approval_sweep_check_finds_a_table_with_no_caption_and_a_th_with_no_scope() {
    let cases = [
        ("<table><th scope=\"col\">A</th></table>", "no caption"),
        ("<table><caption>C<th>A</th></table>", "scope None"),
        ("<table><caption>C<th scope=\"x\">A</th>", "Some(\"x\")"),
        ("<table><caption>C<td><table></table>", "no caption"),
    ];
    reject(table_problems, &cases);
}

#[test]
fn approval_sweep_check_finds_a_field_with_no_label() {
    let cases = [
        ("<input id=\"a\" type=\"text\">", "no label"),
        ("<label for=\"b\">B</label><input id=\"a\">", "\"a\" has no"),
        ("<select id=\"s\"></select>", "<select>"),
        ("<textarea></textarea>", "<textarea>"),
        ("<label>L</label><input id=\"c\">", "\"c\" has no"),
        ("<!-- <label for=q>Q</label> --><input id=q>", "\"q\" has"),
    ];
    reject(label_problems, &cases);
}

#[test]
fn approval_sweep_check_finds_an_aria_reference_to_a_missing_id() {
    let cases = [
        ("<p id=\"a\"><i aria-describedby=\"a b\">", "missing id b"),
        ("<div aria-labelledby=\"cap\"></div>", "missing id cap"),
        ("<ul aria-controls=\"x\"></ul>", "missing id x"),
        ("<ul aria-controls=\"\"></ul>", "is empty"),
    ];
    reject(|tags| aria_problems(tags, &["y"]), &cases);
}

#[test]
fn approval_sweep_check_finds_required_pattern_minlength_and_placeholder() {
    let cases = [
        ("<input required>", "attribute required"),
        ("<input pattern=\"[a-z]+\">", "attribute pattern"),
        ("<input minlength=\"3\">", "attribute minlength"),
        ("<input placeholder=\"\">", "attribute placeholder"),
        ("<input type=text required/>", "attribute required"),
    ];
    reject(forbidden_problems, &cases);
}

#[test]
fn approval_sweep_check_finds_a_token_field_in_a_form_that_does_not_send_post() {
    let token = "<input type=\"hidden\" name=\"csrf_token\" value=\"t\">";
    let bad = [
        format!("<form method=\"get\" action=\"/x\">{token}</form>"),
        format!("<form action=\"/x\">{token}</form>"),
        format!("<form method=\"put\">{token}</form>"),
    ];
    for html in bad {
        let problems = form_problems(&html);
        assert_eq!(problems.len(), 1, "{html}: {problems:?}");
    }
    let good = format!("<form method='POST'>{token}</form><form method=\"get\"><input></form>");
    assert_none(&form_problems(&good), "good");
}

#[test]
fn approval_sweep_reader_keeps_a_valueless_attribute_and_a_value_with_a_greater_than_sign() {
    let found = tags("<input required value='a>b' type=text /><br/></p>");
    let input = found.first().unwrap();
    assert_eq!(input.attribute("required"), Some(""));
    assert_eq!(input.attribute("value"), Some("a>b"));
    assert_eq!(input.attribute("type"), Some("text"));
    assert_eq!(found.len(), 3, "input, br, and the closing p");
    assert!(found.last().unwrap().closing);
}

#[test]
fn approval_sweep_reader_skips_a_comment_and_a_doctype() {
    let found = tags("<!doctype html><!-- <a href=\"/x\"> -- > --><p>A</p><!--><b>");
    let names: Vec<&str> = found.iter().map(|tag| tag.name.as_str()).collect();
    assert_eq!(names, ["p", "p", "b"], "no tag inside a comment");
}

#[test]
fn approval_sweep_reader_keeps_a_less_than_sign_in_a_value_and_a_leading_slash() {
    let found = tags("<a title=\"x <b> y\" href=/audit/1 data-n='<i>'>t</a>");
    let link = found.first().unwrap();
    assert_eq!(link.attribute("title"), Some("x <b> y"));
    assert_eq!(link.attribute("href"), Some("/audit/1"));
    assert_eq!(link.attribute("data-n"), Some("<i>"));
    assert_eq!(found.len(), 2, "the link and its end tag");
}
