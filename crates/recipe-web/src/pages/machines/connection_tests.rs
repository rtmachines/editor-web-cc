//! Tests of `GET /machines/{code}/connection/edit` (task 7.1 of `plc-connection-config`, ADR-0154,
//! design D8). The tests do not use the password of a record, because the form never shows it.

use axum::http::StatusCode;
use recipe_app::ports::SealedSecret;
use recipe_domain::plant::{
    CommMode, DataBlockNumber, Machine, MachineCode, MachineId, PlcAddress, PlcConnection, PlcHost,
    PlcLogin, PlcPlatform, Rack, Slot, TcpPort, UnitId,
};
use recipe_domain::text::Text;
use uuid::Uuid;

use crate::i18n::Lang;
use crate::pages::test_helpers::{
    assert_clean_html, assert_error, assert_no_user_text_in_hx_attributes, field_value, hx_pairs,
};
use crate::test_support::plant::{World, cookie_of, main_of, world};
use crate::test_support::text;

/// The first field of the identifier of a machine that `seed` makes.
const MACHINE_TAG: u32 = 0x7357_0030;
/// The first field of the identifier of a unit that `seed` makes.
const UNIT_TAG: u32 = 0x7357_0031;
/// The text that a sealed password of a test record holds. It must never reach a page.
const SEALED_TEXT: &str = "S3cret-PLC";

/// The `hx-*` attributes that the connection form may write. Each value is a literal.
const ALLOWED_HX: [(&str, &str); 5] = [
    ("hx-boost", "true"),
    ("hx-target", "#connection-status"),
    ("hx-swap", "innerHTML"),
    ("hx-push-url", "false"),
    ("hx-sync", "closest form:drop"),
];

/// The ids of the inputs of the S7 shape.
const S7_FIELDS: [&str; 3] = ["host", "rack", "slot"];
/// The ids of the inputs of the IEC shape.
const IEC_FIELDS: [&str; 6] = ["host", "port", "address", "comm_mode", "login", "password"];

/// Make a live machine `M<number>` of `platform` and keep it in the store.
pub(super) fn seed(world: &World, number: u32, platform: PlcPlatform) -> MachineId {
    let id = MachineId::from(Uuid::from_fields(MACHINE_TAG, 0, 0, &id_tail(number)));
    let unit = UnitId::from(Uuid::from_fields(UNIT_TAG, 0, 0, &id_tail(number)));
    let machine = Machine::create(
        id,
        MachineCode::parse(&format!("M{number}")).unwrap(),
        Text::parse("Spare line").unwrap(),
        platform,
        platform
            .max_data_block()
            .map(|_| DataBlockNumber::parse("1").unwrap()),
        (unit, Text::parse("Infeed").unwrap()),
    )
    .unwrap();
    world.fixture.store.seed_machine(&machine);
    id
}

/// Return the last eight bytes of an identifier that holds `number`.
fn id_tail(number: u32) -> [u8; 8] {
    let bytes = number.to_be_bytes();
    [0, 0, 0, 0, bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Return the form of the page as markup: from its opening tag to its closing tag.
fn form_of(body: &str) -> &str {
    let (_before, rest) = body.split_once("<form id=\"connection-form\"").unwrap();
    rest.split("</form>").next().unwrap()
}

/// Return the tag `<input id="id" ...>` of the form, or `None` when the form has no such input.
fn input_of<'a>(form: &'a str, id: &str) -> Option<&'a str> {
    let marker = format!("<input id=\"{id}\"");
    let (_before, rest) = form.split_once(&marker)?;
    rest.split('>').next()
}

/// Return the markup of the `select` with the id `comm_mode`.
fn mode_select(form: &str) -> &str {
    let (_before, rest) = form.split_once("<select id=\"comm_mode\"").unwrap();
    rest.split("</select>").next().unwrap()
}

/// Assert that the form has an input for each id of `shown` and none for each id of `hidden`.
fn assert_inputs(form: &str, shown: &[&str], hidden: &[&str]) {
    for id in shown {
        let present =
            input_of(form, id).is_some() || form.contains(&format!("<select id=\"{id}\""));
        assert!(present, "no input {id}: {form}");
    }
    for id in hidden {
        assert!(
            !form.contains(&format!("id=\"{id}\"")),
            "the input {id} is not a field of the platform: {form}"
        );
        assert!(
            !form.contains(&format!("name=\"{id}\"")),
            "the field {id} is not a field of the platform: {form}"
        );
    }
}

/// Return the ids of `all` that are not in `shown`.
fn others<'a>(all: &[&'a str], shown: &[&str]) -> Vec<&'a str> {
    all.iter()
        .copied()
        .filter(|id| !shown.contains(id))
        .collect()
}

/// A record of a `TwinCat3` machine with the mode `Ads` and a login.
pub(super) fn ads_record() -> PlcConnection {
    PlcConnection::Iec {
        host: PlcHost::parse("192.168.1.10").unwrap(),
        port: TcpPort::parse("48898").unwrap(),
        address: PlcAddress::parse("192.168.1.10.1.1").unwrap(),
        mode: CommMode::Ads,
        login: Some(PlcLogin::parse("adapter").unwrap()),
    }
}

// --- Spec "Connection of a machine": the fields of the platform --------------------------------

#[tokio::test]
async fn connection_form_of_an_s7_machine_shows_the_s7_fields_only() {
    let world = world();
    for session in [&world.engineer, &world.admin] {
        let reply = world.get_en(session, "/machines/M7/connection/edit").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let form = form_of(main_of(&reply.body));
        assert_inputs(form, &S7_FIELDS, &others(&IEC_FIELDS, &S7_FIELDS));
        assert!(
            form.contains("action=\"/machines/M7/connection\""),
            "{form}"
        );
        assert!(form.contains("method=\"post\""), "{form}");
    }
}

#[tokio::test]
async fn connection_form_of_a_twincat_machine_shows_the_iec_fields_only() {
    let world = world();
    seed(&world, 101, PlcPlatform::TwinCat3);
    let reply = world
        .get_en(&world.engineer, "/machines/M101/connection/edit")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = form_of(main_of(&reply.body));
    assert_inputs(form, &IEC_FIELDS, &others(&S7_FIELDS, &IEC_FIELDS));
    assert!(
        form.contains("action=\"/machines/M101/connection\""),
        "{form}"
    );
}

#[tokio::test]
async fn connection_form_offers_the_modes_of_the_platform() {
    let world = world();
    seed(&world, 101, PlcPlatform::TwinCat3);
    seed(&world, 102, PlcPlatform::CodesysIec);
    let expected = [
        ("M101", vec!["", "Ads", "OpcUa", "ModbusTcp"]),
        ("M102", vec!["", "OpcUa", "CodesysV3", "ModbusTcp"]),
    ];
    for (code, modes) in expected {
        let path = format!("/machines/{code}/connection/edit");
        let reply = world.get_en(&world.engineer, &path).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let select = mode_select(form_of(main_of(&reply.body)));
        let values: Vec<&str> = select
            .split("<option value=\"")
            .skip(1)
            .map(|option| option.split('"').next().unwrap())
            .collect();
        assert_eq!(values, modes, "{code}: {select}");
        // Only the empty first option is selected, so the user chooses the mode.
        assert_eq!(select.matches(" selected").count(), 1, "{select}");
        assert!(select.contains("<option value=\"\" selected>"), "{select}");
    }
}

// --- Spec "Secrecy of the connection password": the empty password input -----------------------

#[tokio::test]
async fn connection_form_password_input_is_a_new_password_input_without_a_value() {
    let world = world();
    let id = seed(&world, 101, PlcPlatform::TwinCat3);
    // A stored record with a sealed password: the page may show the state and never the text.
    let sealed = SealedSecret::from_bytes(SEALED_TEXT.as_bytes().to_vec());
    world
        .fixture
        .store
        .seed_connection(id, &ads_record(), Some(sealed));
    for session in [&world.engineer, &world.admin] {
        let reply = world
            .get_en(session, "/machines/M101/connection/edit")
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let form = form_of(main_of(&reply.body));
        let input = input_of(form, "password").unwrap();
        assert!(input.contains("name=\"password\""), "{input}");
        assert!(input.contains("type=\"password\""), "{input}");
        assert!(input.contains("autocomplete=\"new-password\""), "{input}");
        assert!(!input.contains("value"), "the input has no value: {input}");
        assert!(!reply.body.contains(SEALED_TEXT), "{}", reply.body);
    }
}

#[tokio::test]
async fn connection_form_password_hint_tells_whether_the_record_holds_a_password() {
    let world = world();
    let with_password = seed(&world, 101, PlcPlatform::TwinCat3);
    seed(&world, 102, PlcPlatform::TwinCat3);
    let sealed = SealedSecret::from_bytes(SEALED_TEXT.as_bytes().to_vec());
    world
        .fixture
        .store
        .seed_connection(with_password, &ads_record(), Some(sealed));
    for lang in [Lang::En, Lang::Pl] {
        let stored = world
            .get(
                &world.engineer,
                "/machines/M101/connection/edit",
                &[cookie_of(lang)],
            )
            .await;
        let none = world
            .get(
                &world.engineer,
                "/machines/M102/connection/edit",
                &[cookie_of(lang)],
            )
            .await;
        let stored_text = text(lang, "connection-password-stored");
        let none_text = text(lang, "connection-password-none");
        assert!(stored.body.contains(&stored_text), "{}", stored.body);
        assert!(!stored.body.contains(&none_text), "{}", stored.body);
        assert!(none.body.contains(&none_text), "{}", none.body);
        assert!(!none.body.contains(&stored_text), "{}", none.body);
    }
}

// --- Spec "Connection of a machine": the stored values and the lock token ----------------------

#[tokio::test]
async fn connection_form_without_a_record_is_empty_and_carries_row_version_0() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M7/connection/edit")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert_eq!(field_value(body, "row_version"), "0");
    assert_eq!(field_value(body, "csrf_token"), world.engineer.csrf);
    let form = form_of(main_of(body));
    assert!(
        form.contains("<input type=\"hidden\" name=\"row_version\" value=\"0\">"),
        "{form}"
    );
    for id in S7_FIELDS {
        let input = input_of(form, id).unwrap();
        assert!(!input.contains("value="), "{id} is empty: {input}");
    }
}

#[tokio::test]
async fn connection_form_with_a_record_shows_its_values_and_its_row_version() {
    let world = world();
    let id = seed(&world, 101, PlcPlatform::TwinCat3);
    world.fixture.store.seed_connection(id, &ads_record(), None);
    let reply = world
        .get_en(&world.engineer, "/machines/M101/connection/edit")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(field_value(&reply.body, "row_version"), "1");
    let form = form_of(main_of(&reply.body));
    for (field, value) in [
        ("host", "192.168.1.10"),
        ("port", "48898"),
        ("address", "192.168.1.10.1.1"),
        ("login", "adapter"),
    ] {
        let input = input_of(form, field).unwrap();
        assert!(input.contains(&format!(" value=\"{value}\"")), "{input}");
    }
    let select = mode_select(form);
    assert!(
        select.contains("<option value=\"Ads\" selected>"),
        "{select}"
    );
}

#[tokio::test]
async fn connection_form_of_an_s7_record_shows_host_rack_and_slot() {
    let world = world();
    let id = seed(&world, 103, PlcPlatform::S7_1500);
    let record = PlcConnection::S7 {
        host: PlcHost::parse("plc-1.plant.local").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("2").unwrap(),
    };
    world.fixture.store.seed_connection(id, &record, None);
    let reply = world
        .get_en(&world.engineer, "/machines/M103/connection/edit")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = form_of(main_of(&reply.body));
    for (field, value) in [("host", "plc-1.plant.local"), ("rack", "0"), ("slot", "2")] {
        let input = input_of(form, field).unwrap();
        assert!(input.contains(&format!(" value=\"{value}\"")), "{input}");
    }
}

#[tokio::test]
async fn connection_form_of_an_s7_machine_has_no_data_block_input_with_or_without_a_record() {
    // The data block number belongs to the machine (ADR-0156, design D8).
    let world = world();
    let id = seed(&world, 104, PlcPlatform::S7_1500);
    let record = PlcConnection::S7 {
        host: PlcHost::parse("plc-1.plant.local").unwrap(),
        rack: Rack::parse("0").unwrap(),
        slot: Slot::parse("2").unwrap(),
    };
    for stored in [false, true] {
        if stored {
            world.fixture.store.seed_connection(id, &record, None);
        }
        let reply = world
            .get_en(&world.engineer, "/machines/M104/connection/edit")
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let form = form_of(main_of(&reply.body));
        assert!(input_of(form, "db_number").is_none(), "{stored}: {form}");
        assert!(!form.contains("name=\"db_number\""), "{stored}: {form}");
    }
}

// --- Texts, markup, and refusals ---------------------------------------------------------------

#[tokio::test]
async fn connection_form_labels_and_title_come_from_the_bundles_in_both_languages() {
    let world = world();
    for lang in [Lang::En, Lang::Pl] {
        let reply = world
            .get(
                &world.engineer,
                "/machines/M7/connection/edit",
                &[cookie_of(lang)],
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        for key in [
            "field-plc-host",
            "field-plc-rack",
            "field-plc-slot",
            "connection-submit",
        ] {
            assert!(main.contains(&text(lang, key)), "{key}: {main}");
        }
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn connection_form_has_literal_hx_attributes_the_sync_rule_and_no_user_text_in_them() {
    let world = world();
    let id = seed(&world, 101, PlcPlatform::TwinCat3);
    world.fixture.store.seed_connection(id, &ads_record(), None);
    for path in [
        "/machines/M7/connection/edit",
        "/machines/M101/connection/edit",
    ] {
        let reply = world.get_en(&world.engineer, path).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        for needle in ["M7", "M101", "192.168.1.10", "adapter"] {
            assert_no_user_text_in_hx_attributes(&reply.body, needle);
        }
        let pairs = hx_pairs(main);
        assert!(pairs.contains(&("hx-sync".to_owned(), "closest form:drop".to_owned())));
        for (name, value) in &pairs {
            let known = ALLOWED_HX.contains(&(name.as_str(), value.as_str()));
            assert!(known, "{name}={value} is not a literal of the design");
        }
        assert!(!form_of(main).contains("method=\"get\""));
    }
}

#[tokio::test]
async fn connection_form_of_an_unknown_machine_gives_404_e_web_005() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M999/connection/edit")
        .await;
    assert_error(
        &reply,
        StatusCode::NOT_FOUND,
        "E-WEB-005",
        "unknown machine",
    );
}

#[tokio::test]
async fn connection_form_of_an_archived_machine_gives_404_e_web_005() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/M12/connection/edit")
        .await;
    assert_error(
        &reply,
        StatusCode::NOT_FOUND,
        "E-WEB-005",
        "archived machine",
    );
}

#[tokio::test]
async fn connection_form_of_a_bad_code_gives_404_e_web_005() {
    let world = world();
    let reply = world
        .get_en(&world.engineer, "/machines/not-a-code/connection/edit")
        .await;
    assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", "bad code");
}

#[tokio::test]
async fn connection_form_for_an_operator_gives_403_and_no_form() {
    let world = world();
    let reply = world
        .get_en(&world.operator, "/machines/M7/connection/edit")
        .await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
    assert!(!reply.body.contains("connection-form"), "{}", reply.body);
}
