//! Tests that the export documents hold no connection data (task 8.6 of `plc-connection-config`,
//! ADR-0154 (j), design D9).
//!
//! The documents are the v1 list, the v1 document of a version, and the v2 unit document. They hold
//! no connection field and no connection text. A connection change leaves each document byte for
//! byte the same, so `content_sha256` and every `s7_address` stay as they were.

use axum::http::StatusCode;
use recipe_app::ports::CredentialCipher;
use recipe_domain::plant::{MachineId, PlcConnection, PlcHost, PlcPassword, Rack, Slot};
use serde_json::Value;

use super::tests::{client_get, seeded};
use crate::test_support::Fixture;

/// The list route of the live machine `M7`.
const M7_LIST: &str = "/export/v1/machines/M7/recipes";
/// The v1 document route of `R-500` on `M7`.
const M7_R500: &str = "/export/v1/machines/M7/recipes/R-500/approved";
/// The v1 document route of `R-800` on `M7`. Its parameters hold `s7_address` values.
const M7_R800: &str = "/export/v1/machines/M7/recipes/R-800/approved";
/// The v2 document route of the unit `U1.2` of `R-800`.
const M7_R800_UNIT: &str = "/export/v2/machines/M7/recipes/R-800/approved/units/U1.2";
/// The v2 document route of the main unit `U1` of `R-800`.
const M7_R800_MAIN: &str = "/export/v2/machines/M7/recipes/R-800/approved/units/U1";
/// The host of the first record. No document may hold this text.
const HOST_A: &str = "plc-a.plant.local";
/// The host of the second record. No document may hold this text.
const HOST_B: &str = "plc-b.plant.local";
/// A password text that the machine `M7` never stores in a way that a document shows.
const PLC_PASSWORD: &str = "S3cret-PLC";
/// The keys that only the connection document of `api.md` section 6.5 holds.
///
/// `address` is a connection key. The key `s7_address` of a parameter is not in this list.
const CONNECTION_KEYS: [&str; 11] = [
    "connection",
    "host",
    "port",
    "address",
    "comm_mode",
    "login",
    "password",
    "rack",
    "slot",
    "db_number",
    "plc_connection",
];

/// Return an S7 record on the host `host`, with the rack and slot of `variant`.
fn s7_record(host: &str, variant: &str) -> PlcConnection {
    PlcConnection::S7 {
        host: PlcHost::parse(host).unwrap(),
        rack: Rack::parse(variant).unwrap(),
        slot: Slot::parse("3").unwrap(),
    }
}

/// Store `connection` for `machine`. The store replaces any earlier record of the machine.
fn store_connection(fixture: &Fixture, machine: MachineId, connection: &PlcConnection) {
    let sealed = fixture
        .cipher
        .seal(machine, &PlcPassword::parse(PLC_PASSWORD).unwrap())
        .ok();
    fixture.store.seed_connection(machine, connection, sealed);
}

/// Read every document route and return each body with its route.
async fn read_documents(fixture: &Fixture) -> Vec<(&'static str, String)> {
    let mut documents = Vec::new();
    for route in [M7_LIST, M7_R500, M7_R800, M7_R800_UNIT, M7_R800_MAIN] {
        let reply = client_get(fixture, route).await;
        assert_eq!(reply.status, StatusCode::OK, "{route}: {}", reply.body);
        documents.push((route, reply.body));
    }
    documents
}

/// Collect every object key of `value`, at any depth.
fn collect_keys<'a>(value: &'a Value, keys: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                keys.push(key.as_str());
                collect_keys(inner, keys);
            }
        }
        Value::Array(items) => items.iter().for_each(|inner| collect_keys(inner, keys)),
        _ => {}
    }
}

/// Return the connection keys that a document body holds, in the order of the key list.
fn connection_keys_in(body: &str) -> Vec<&'static str> {
    let value: Value = serde_json::from_str(body).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);
    CONNECTION_KEYS
        .into_iter()
        .filter(|forbidden| keys.contains(forbidden))
        .collect()
}

/// Return the text of every value of the key `name`, at any depth, in the order of the body.
fn values_of(body: &str, name: &str) -> Vec<String> {
    fn walk(value: &Value, name: &str, found: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    if key == name {
                        found.push(inner.to_string());
                    }
                    walk(inner, name, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|inner| walk(inner, name, found)),
            _ => {}
        }
    }
    let value: Value = serde_json::from_str(body).unwrap();
    let mut found = Vec::new();
    walk(&value, name, &mut found);
    found
}

/// Assert that no document holds a connection key or a connection text.
fn assert_no_connection_data(documents: &[(&str, String)], what: &str) {
    for (route, body) in documents {
        assert!(
            connection_keys_in(body).is_empty(),
            "{what}: {route}: {:?}",
            connection_keys_in(body)
        );
        for text in [HOST_A, HOST_B, PLC_PASSWORD] {
            assert!(!body.contains(text), "{what}: {route}: {text}");
        }
    }
}

#[test]
fn the_key_check_finds_a_connection_key_at_any_depth_and_ignores_s7_address() {
    let nested = r#"{"a":[{"b":{"host":"h"}}],"s7_address":"DB1.DBB0"}"#;
    let clean = r#"{"s7_address":"DB1.DBB0","parameters":[{"tag":"T"}]}"#;

    assert_eq!(connection_keys_in(nested), vec!["host"]);
    assert!(connection_keys_in(clean).is_empty());
    assert_eq!(values_of(nested, "s7_address"), vec!["\"DB1.DBB0\""]);
}

#[tokio::test]
async fn the_documents_of_a_machine_with_a_connection_hold_no_connection_field() {
    let (fixture, data) = seeded().await;
    fixture.seed_unit_recipe(&data).await;
    store_connection(&fixture, data.recipes.plant.m7, &s7_record(HOST_A, "0"));

    let documents = read_documents(&fixture).await;

    assert_no_connection_data(&documents, "with a connection");
    let version = &documents
        .iter()
        .find(|(route, _)| *route == M7_R800)
        .unwrap()
        .1;
    assert!(version.contains("\"schema_version\": \"1\""), "{version}");
    let unit = &documents
        .iter()
        .find(|(route, _)| *route == M7_R800_UNIT)
        .unwrap()
        .1;
    assert!(unit.contains("\"schema_version\": \"2\""), "{unit}");
}

#[tokio::test]
async fn a_connection_change_leaves_every_document_and_hash_and_s7_address_unchanged() {
    let (fixture, data) = seeded().await;
    fixture.seed_unit_recipe(&data).await;
    let machine = data.recipes.plant.m7;
    let without = read_documents(&fixture).await;
    store_connection(&fixture, machine, &s7_record(HOST_A, "0"));
    let first = read_documents(&fixture).await;
    store_connection(&fixture, machine, &s7_record(HOST_B, "1"));
    let second = read_documents(&fixture).await;

    for ((route, before), ((_, one), (_, two))) in
        without.iter().zip(first.iter().zip(second.iter()))
    {
        assert_eq!(
            before, one,
            "{route}: the first record changed the document"
        );
        assert_eq!(one, two, "{route}: the second record changed the document");
    }
    let version = |documents: &[(&str, String)]| {
        documents
            .iter()
            .find(|(route, _)| *route == M7_R800)
            .unwrap()
            .1
            .clone()
    };
    let hashes = values_of(&version(&second), "content_sha256");
    let addresses = values_of(&version(&second), "s7_address");
    assert_eq!(hashes, values_of(&version(&without), "content_sha256"));
    assert_eq!(hashes.len(), 1, "{hashes:?}");
    assert_eq!(addresses, values_of(&version(&without), "s7_address"));
    assert!(!addresses.is_empty(), "R-800 holds s7_address values");
    assert_no_connection_data(&second, "after the change");
}
