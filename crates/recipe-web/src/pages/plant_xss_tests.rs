//! The XSS corpus through each field and path value of the plant pages (task 11.1, design D14).
//!
//! Each field and path value gets every payload of `tests/web/xss-corpus.txt` and one NUL payload.
//! A value that the server echoes shows as escaped text. A value that the server refuses gives its
//! error code. No answer holds the raw payload, no `hx-*` attribute holds it, and each `<svg` of an
//! answer is the fixed tag of its icon.

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use super::machines::test_world::seed_machines;
use super::xss_kit::{assert_error_panel, assert_refused, assert_safe, has_control};
use super::xss_tests::{query_pages, with_query};
use crate::pages::test_helpers::{askama_escape, encode, field_value, payloads};
use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields};

/// The header of an HTMX request.
const HTMX: (&str, &str) = ("hx-request", "true");
/// The form of a valid new machine. A test replaces one value.
const MACHINE_FIELDS: [(&str, &str); 4] = [
    ("code", "M99"),
    ("name", "Created"),
    ("plc_platform", "S7_300"),
    ("main_unit_name", "Infeed"),
];
/// The raw query values that `http` accepts. Each one follows `q=` of the machine list.
const RAW_QUERY_VALUES: [&str; 5] = [
    "'onfocus=alert(1)'",
    "`onfocus=alert(1)`",
    "{{7*7}}",
    "\\'};alert(1);//",
    "javascript:alert(1)",
];

/// A `GET` target with `{}` where the payload goes, and the code that the answer holds.
///
/// A query value or a path value that no payload can satisfy gives this code.
const REFUSED_GETS: [(&str, &str); 19] = [
    ("/machines?show_archived={}", "E-WEB-008"),
    ("/machines?page={}", "E-WEB-008"),
    ("/machines?archived={}", "E-WEB-008"),
    ("/machines/M7?archived={}", "E-PLT-017"),
    ("/machines/M7/units/new?parent_path={}", "E-PLT-017"),
    ("/machines/M7/types/new?from={}", "E-WEB-005"),
    ("/machines/M7/types/ST_Zone?rev={}", "E-WEB-005"),
    ("/machines/M7/types/ST_Zone/usage?rev={}", "E-WEB-005"),
    ("/machines/M7/types/ST_Zone/usage?page={}", "E-WEB-008"),
    ("/machines/{}", "E-WEB-005"),
    ("/machines/by-id/{}", "E-WEB-005"),
    ("/machines/{}/units", "E-WEB-005"),
    ("/machines/{}/units/new", "E-WEB-005"),
    ("/machines/{}/types", "E-WEB-005"),
    ("/machines/{}/types/new", "E-WEB-005"),
    ("/machines/M7/units/{}", "E-PLT-017"),
    ("/machines/M7/types/{}", "E-WEB-005"),
    ("/machines/M7/types/{}/usage", "E-WEB-005"),
    ("/machines/M7/types/new?csrf_token={}", "E-WEB-008"),
];

/// A `POST` target with `{}` where the payload goes, a valid body, and the code of the answer.
const REFUSED_POSTS: [(&str, &str, &str); 7] = [
    ("/machines/{}/rename", "name=Renamed", "E-WEB-005"),
    ("/machines/{}/archive", "", "E-WEB-005"),
    (
        "/machines/{}/units",
        "parent_path=&segment=5&name=A",
        "E-WEB-005",
    ),
    ("/machines/M7/units/{}/rename", "name=Renamed", "E-PLT-017"),
    ("/machines/M7/units/{}/archive", "", "E-PLT-017"),
    (
        "/machines/{}/types",
        "name=ST_P&members[0].name=a&members[0].type=INT",
        "E-WEB-005",
    ),
    (
        "/machines/M7/types/{}/revisions",
        "members[0].name=a&members[0].type=INT",
        "E-WEB-005",
    ),
];

/// Fill the hole `{}` of a target with the encoded payload.
fn fill(target: &str, payload: &str) -> String {
    target.replace("{}", &encode(payload))
}

/// The state of a test: the plant data of `seed_plant` and the session of an Administrator.
struct Plant {
    fixture: Fixture,
    admin: TestSession,
}

/// Build a fixture with the Administrator `user1` and the data of `seed_plant`.
fn plant() -> Plant {
    let fixture = Fixture::new();
    let admin = fixture.seed_session(1, Role::Administrator, false);
    fixture.seed_plant();
    Plant { fixture, admin }
}

impl Plant {
    /// Send `GET path` as the Administrator.
    async fn get(&self, path: &str) -> Reply {
        let cookie = [("cookie", self.admin.cookie.as_str())];
        get_with(&self.fixture.router(), path, &cookie).await
    }

    /// Send the form `body` to `path` as an HTMX request of the Administrator.
    async fn post(&self, path: &str, body: &str) -> Reply {
        let router = self.fixture.router();
        post_fields(&router, path, Some(&self.admin), &[HTMX], body).await
    }
}

/// Return the form body of a new machine with the field `field` set to `value`.
fn machine_body(field: &str, value: &str) -> String {
    MACHINE_FIELDS
        .iter()
        .map(|(name, valid)| {
            let text = if *name == field { value } else { valid };
            format!("{name}={}", encode(text))
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Return the tag of the element with the `id`, up to the end of the tag.
fn tag_of<'a>(body: &'a str, id: &str) -> &'a str {
    let marker = format!("id=\"{id}\"");
    let (_before, rest) = body.split_once(&marker).unwrap();
    rest.split_once('>').unwrap().0
}

// --- Refused query values and path values ---------------------------------------------------------

#[tokio::test]
async fn plant_xss_query_values_and_path_values_of_get_pages_give_their_error_code() {
    let plant = plant();
    for payload in payloads() {
        for (target, code) in REFUSED_GETS {
            let path = fill(target, &payload);
            let reply = plant.get(&path).await;
            assert_refused(&reply, code, &payload, &format!("GET {path}"));
        }
    }
}

#[tokio::test]
async fn plant_xss_path_values_of_post_routes_give_their_error_code() {
    let plant = plant();
    for payload in payloads() {
        for (target, body, code) in REFUSED_POSTS {
            let path = fill(target, &payload);
            let reply = plant.post(&path, body).await;
            let what = format!("POST {path}");
            assert_refused(&reply, code, &payload, &what);
            assert_error_panel(&reply, &what);
        }
    }
}

// --- The search text of the machine list ------------------------------------------------------------

#[tokio::test]
async fn plant_xss_search_text_of_the_machine_list_is_echoed_escaped() {
    let plant = plant();
    for payload in payloads() {
        let path = format!("/machines?q={}", encode(&payload));
        let reply = plant.get(&path).await;
        let what = format!("GET {path}");
        if has_control(&payload) {
            assert_refused(&reply, "E-WEB-008", &payload, &what);
            continue;
        }
        assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        let input = format!("value=\"{}\"", askama_escape(&payload));
        assert!(reply.body.contains(&input), "{what}: {}", reply.body);
        assert_safe(&reply, &payload, &what);
        assert_eq!(field_value(&reply.body, "next"), askama_escape(&path));
    }
}

#[tokio::test]
async fn plant_xss_search_text_is_encoded_in_the_pager_links() {
    // 101 machines carry the payload as name, so the list has a second page (100 rows a page).
    for payload in payloads().iter().filter(|payload| !has_control(payload)) {
        let plant = plant();
        seed_machines(&plant.fixture, 101, payload);
        let query = encode(payload);
        // Page 1 links to page 2. Page 2 links back to page 1.
        for (suffix, target) in [("", 2), ("&page=2", 1)] {
            let path = format!("/machines?q={query}{suffix}");
            let reply = plant.get(&path).await;
            assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
            let link = format!("href=\"/machines?q={query}&#38;page={target}\">");
            assert!(reply.body.contains(&link), "{path}: {}", reply.body);
            assert_safe(&reply, payload, &path);
            assert_eq!(field_value(&reply.body, "next"), askama_escape(&path));
        }
    }
}

#[tokio::test]
async fn plant_xss_next_field_of_the_switch_escapes_the_raw_bytes_of_the_search_query() {
    let plant = plant();
    for raw in RAW_QUERY_VALUES {
        let path = format!("/machines?q={raw}");
        let reply = plant.get(&path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        assert_eq!(field_value(&reply.body, "next"), askama_escape(&path));
        assert_safe(&reply, raw, &path);
    }
}

#[tokio::test]
async fn plant_xss_next_field_of_each_plant_page_that_accepts_a_query_holds_the_escaped_corpus() {
    let plant = plant();
    let pages = query_pages(&plant.fixture, &plant.admin).await;
    let plant_pages: Vec<&(&str, String)> = pages
        .iter()
        .filter(|(pattern, _path)| pattern.starts_with("/machines"))
        .collect();
    assert!(plant_pages.len() >= 5, "{pages:?}");
    for payload in payloads() {
        for (pattern, page) in &plant_pages {
            let path = with_query(page, "x", &encode(&payload));
            let reply = plant.get(&path).await;
            let what = format!("GET {pattern} {path}");
            assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
            assert_safe(&reply, &payload, &what);
            assert_eq!(
                field_value(&reply.body, "next"),
                askama_escape(&path),
                "{what}"
            );
        }
    }
}

// --- Fields that the server accepts and shows again ---------------------------------------------------

#[tokio::test]
async fn plant_xss_machine_fields_that_the_server_refuses_give_their_error_code() {
    for payload in payloads() {
        for field in ["code", "plc_platform"] {
            let plant = plant();
            let reply = plant
                .post("/machines", &machine_body(field, &payload))
                .await;
            let what = format!("POST /machines {field}");
            assert_refused(&reply, "E-WEB-008", &payload, &what);
            assert_error_panel(&reply, &what);
            if field == "code" && !payload.is_empty() {
                // ADR-0041 (e): the out-of-band form shows the sent text escaped.
                let value = format!("value=\"{}\"", askama_escape(&payload));
                assert!(reply.body.contains(&value), "{what}: {}", reply.body);
            }
        }
    }
}

#[tokio::test]
async fn plant_xss_machine_names_are_stored_and_shown_escaped() {
    // The machine name shows on the machine page and in the list. The main unit shows in the tree.
    let shown_on = [
        ("name", ["/machines/M99", "/machines"]),
        ("main_unit_name", ["/machines/M99", "/machines/M99/units"]),
    ];
    for payload in payloads() {
        for (field, pages) in shown_on {
            let plant = plant();
            let reply = plant
                .post("/machines", &machine_body(field, &payload))
                .await;
            let what = format!("POST /machines {field}");
            if has_control(&payload) {
                assert_refused(&reply, "E-WEB-008", &payload, &what);
                continue;
            }
            assert_eq!(
                reply.status,
                StatusCode::SEE_OTHER,
                "{what}: {}",
                reply.body
            );
            for page in pages {
                let shown = plant.get(page).await;
                assert_eq!(shown.status, StatusCode::OK, "{what} {page}");
                assert!(
                    shown.body.contains(&askama_escape(&payload)),
                    "{what} {page}"
                );
                assert_safe(&shown, &payload, &format!("{what} {page}"));
            }
        }
    }
}

#[tokio::test]
async fn plant_xss_rename_name_is_shown_escaped_in_the_answer_and_on_the_page() {
    let targets = [
        ("/machines/M7/rename", "/machines/M7"),
        ("/machines/M7/units/U1.2/rename", "/machines/M7/units/U1.2"),
    ];
    for payload in payloads() {
        for (action, page) in targets {
            let plant = plant();
            let body = format!("name={}", encode(&payload));
            let reply = plant.post(action, &body).await;
            if has_control(&payload) {
                assert_refused(&reply, "E-WEB-008", &payload, action);
                assert_error_panel(&reply, action);
                continue;
            }
            assert_eq!(reply.status, StatusCode::OK, "{action}: {}", reply.body);
            assert!(reply.body.contains(&askama_escape(&payload)), "{action}");
            assert_safe(&reply, &payload, action);
            let shown = plant.get(page).await;
            assert!(shown.body.contains(&askama_escape(&payload)), "{page}");
            assert_safe(&shown, &payload, page);
        }
    }
}

// --- The form of a new unit -------------------------------------------------------------------------

/// Return the form body of a new unit under `U1.2` with one replaced value.
fn unit_body(field: &str, value: &str) -> String {
    let pairs = [("parent_path", "U1.2"), ("segment", "5"), ("name", "Added")];
    pairs
        .iter()
        .map(|(name, valid)| {
            let text = if *name == field { value } else { valid };
            format!("{name}={}", encode(text))
        })
        .collect::<Vec<_>>()
        .join("&")
}

#[tokio::test]
async fn plant_xss_unit_fields_that_the_server_refuses_give_their_error_code() {
    let plant = plant();
    for payload in payloads() {
        for (field, code) in [("parent_path", "E-PLT-017"), ("segment", "E-PLT-017")] {
            let reply = plant
                .post("/machines/M7/units", &unit_body(field, &payload))
                .await;
            let what = format!("POST units {field}");
            assert_refused(&reply, code, &payload, &what);
            assert_error_panel(&reply, &what);
        }
    }
}

#[tokio::test]
async fn plant_xss_unit_name_is_shown_escaped_in_the_new_tree() {
    for payload in payloads() {
        let plant = plant();
        let reply = plant
            .post("/machines/M7/units", &unit_body("name", &payload))
            .await;
        if has_control(&payload) {
            assert_refused(&reply, "E-WEB-008", &payload, "POST units name");
            continue;
        }
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_safe(&reply, &payload, "POST units name");
        let shown = plant.get("/machines/M7/units/U1.2.5").await;
        assert!(
            shown.body.contains(&askama_escape(&payload)),
            "{}",
            shown.body
        );
        assert_safe(&shown, &payload, "GET unit U1.2.5");
    }
}

// --- The UDT form ------------------------------------------------------------------------------------

/// Return the UDT form body with the given pairs. Every key and value is percent-encoded.
fn udt_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// The fields of a valid UDT form. A test replaces one value or adds one field.
const UDT_FIELDS: [(&str, &str); 3] = [
    ("name", "ST_P"),
    ("members[0].name", "a"),
    ("members[0].type", "INT"),
];
/// The UDT fields that no payload can satisfy, and the code of the answer.
const REFUSED_UDT_KEYS: [(&str, &str); 4] = [
    ("name", "E-WEB-008"),
    ("members[0].name", "E-WEB-008"),
    ("members[0].type", "E-PLT-013"),
    ("add", "E-WEB-008"),
];

/// Return the valid UDT form body with `payload` as the value of `key`.
///
/// A key that the valid form does not hold, such as `add`, is added.
fn udt_body_with(key: &str, payload: &str) -> String {
    let mut pairs: Vec<(&str, &str)> = UDT_FIELDS
        .iter()
        .map(|(name, valid)| (*name, if *name == key { payload } else { valid }))
        .collect();
    if !UDT_FIELDS.iter().any(|(name, _valid)| *name == key) {
        pairs.push((key, payload));
    }
    udt_body(&pairs)
}

#[tokio::test]
async fn plant_xss_udt_fields_that_the_server_refuses_give_their_error_code() {
    let plant = plant();
    for payload in payloads() {
        for (key, code) in REFUSED_UDT_KEYS {
            let body = udt_body_with(key, &payload);
            let reply = plant.post("/machines/M7/types", &body).await;
            let what = format!("POST types {key}");
            assert_refused(&reply, code, &payload, &what);
            assert_error_panel(&reply, &what);
        }
    }
}

#[tokio::test]
async fn plant_xss_udt_row_index_gives_form_error() {
    let plant = plant();
    for payload in payloads() {
        let key = format!("members[{payload}].name");
        let pairs = [("name", "ST_P"), (key.as_str(), "a")];
        let reply = plant.post("/machines/M7/types", &udt_body(&pairs)).await;
        assert_refused(&reply, "E-WEB-008", &payload, "POST types index");
        assert_error_panel(&reply, "POST types index");
    }
}

#[tokio::test]
async fn plant_xss_udt_rows_with_add_are_echoed_escaped() {
    for payload in payloads() {
        let plant = plant();
        let pairs = [
            ("name", payload.as_str()),
            ("members[0].name", payload.as_str()),
            ("members[0].type", payload.as_str()),
            ("add", "1"),
        ];
        let reply = plant.post("/machines/M7/types", &udt_body(&pairs)).await;
        let what = "POST types add=1";
        assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        assert_safe(&reply, &payload, what);
        let value = format!("value=\"{}\"", askama_escape(&payload));
        for id in ["type_name", "members-0-name", "members-0-type"] {
            assert!(tag_of(&reply.body, id).contains(&value), "{what} {id}");
        }
    }
}

#[tokio::test]
async fn plant_xss_revision_rows_with_add_are_echoed_escaped() {
    for payload in payloads() {
        let plant = plant();
        let pairs = [
            ("members[0].name", payload.as_str()),
            ("members[0].type", payload.as_str()),
            ("add", "1"),
        ];
        let path = "/machines/M7/types/ST_Zone/revisions";
        let reply = plant.post(path, &udt_body(&pairs)).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        assert_safe(&reply, &payload, path);
        let value = format!("value=\"{}\"", askama_escape(&payload));
        assert!(tag_of(&reply.body, "members-0-name").contains(&value));
        assert!(tag_of(&reply.body, "members-0-type").contains(&value));
    }
}
