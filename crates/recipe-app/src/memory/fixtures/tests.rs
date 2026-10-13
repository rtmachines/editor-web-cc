use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{SecurityEvent, SessionRef, UserAgent};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, Primitive, S7Address, UnitPath};
use recipe_domain::recipe::VersionState;

use super::*;
use crate::memory::{Fault, seed_type_id};
use crate::ports::{Clock, SecurityEventSink};

#[test]
fn the_actors_have_their_roles_and_logins() {
    let admin = actor(Role::Administrator);
    assert_eq!(admin.user(), user(1));
    assert_eq!(admin.login().as_str(), "admin");
    assert_eq!(admin.role(), Role::Administrator);

    let engineer = actor(Role::ProcessEngineer);
    assert_eq!(engineer.user(), user(2));
    assert_eq!(engineer.login().as_str(), "jkowalski");
    assert_eq!(engineer.role(), Role::ProcessEngineer);

    let second = second_engineer();
    assert_eq!(second.user(), user(3));
    assert_eq!(second.login().as_str(), "anowak");
    assert_eq!(second.role(), Role::ProcessEngineer);

    let operator = actor(Role::Operator);
    assert_eq!(operator.user(), user(4));
    assert_eq!(operator.login().as_str(), "mhoffman");
    assert_eq!(operator.role(), Role::Operator);
}

#[test]
fn meta_has_the_fixed_request_data() {
    let data = meta();
    assert_eq!(
        data.client_ip(),
        Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)))
    );
    assert!(data.user_agent().is_some());
    assert_eq!(
        data.session_ref(),
        Some(&SessionRef::parse("0123456789abcdef").unwrap())
    );
}

#[test]
fn a_new_fixture_holds_the_int_type() {
    let fixture = Fixture::new();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    assert!(fixture.store.state().plc_type(int).is_some());
}

#[test]
fn machine_on_has_three_live_units_in_a_chain() {
    let machine = machine_on(PlcPlatform::S7_300, 7, "M7");
    assert_eq!(machine.platform(), PlcPlatform::S7_300);
    assert_eq!(machine.units().count(), 3);
    for path in ["U1", "U1.2", "U1.2.1"] {
        let unit = machine.unit(&UnitPath::parse(path).unwrap());
        assert!(unit.is_some_and(|unit| unit.is_live()), "{path}");
    }
}

#[test]
fn seed_machine_on_stores_the_machine() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::CodesysIec, 9, "M9");
    assert_eq!(fixture.store.state().machine(machine.id()), Some(machine));
}

#[test]
fn seed_draft_stores_version_one_as_a_live_draft() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let (recipe, version) = fixture.seed_draft(&machine, 5, "R-5");
    let state = fixture.store.state();
    let row = state.versions.get(&version).unwrap();
    assert_eq!(row.state().unwrap(), VersionState::LiveDraft);
    assert_eq!(row.row_version, 1);
    assert_eq!(row.number, 1);
    assert_eq!(row.created_by, user(2));
    assert_eq!(row.recipe, recipe.id());
    assert!(state.recipes.contains_key(&recipe.id()));
}

#[test]
fn seed_approved_and_draft_stores_an_approved_version_one_and_a_draft() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    let (_recipe, approved, draft) = fixture.seed_approved_and_draft(&machine, 6, "R-6");
    let state = fixture.store.state();
    let first = state.versions.get(&approved).unwrap();
    let second = state.versions.get(&draft).unwrap();
    assert_eq!(first.number, 1);
    assert_eq!(first.state().unwrap(), VersionState::Approved);
    assert_eq!(second.number, 2);
    assert_eq!(second.state().unwrap(), VersionState::LiveDraft);
}

#[test]
fn seed_retired_stores_a_retired_version_one_and_an_approved_version_two() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 3, "M3");
    let (_recipe, retired, approved) = fixture.seed_retired(&machine, 7, "R-7");
    let state = fixture.store.state();
    let first = state.versions.get(&retired).unwrap();
    let second = state.versions.get(&approved).unwrap();
    assert_eq!(first.number, 1);
    assert_eq!(first.state().unwrap(), VersionState::Retired);
    assert_eq!(second.number, 2);
    assert_eq!(second.state().unwrap(), VersionState::Approved);
}

#[test]
fn an_s7_300_parameter_has_an_s7_address_and_a_type_of_its_platform() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 8, "M8");
    let (_recipe, version) = fixture.seed_draft(&machine, 8, "R-8");
    let parameter = parameter_on(PlcPlatform::S7_300, 1, "A", uid(801), 0);
    assert_eq!(
        parameter.s7_address,
        Some(S7Address::parse("DB1.DBW0").unwrap())
    );
    fixture.store.seed_parameter(version, &parameter);
    let state = fixture.store.state();
    assert!(state.parameters.contains_key(&pid(1)));
    let plc_type = state.plc_type(parameter.plc_type).unwrap();
    assert!(plc_type.platforms().contains(&PlcPlatform::S7_300));
}

#[test]
fn a_twincat3_parameter_has_no_s7_address() {
    let parameter = parameter_on(PlcPlatform::TwinCat3, 2, "B", uid(801), 0);
    assert!(parameter.s7_address.is_none());
}

#[test]
fn the_fixture_clock_is_the_clock_of_the_store() {
    let fixture = Fixture::new();
    fixture.clock.advance(time::Duration::seconds(5));
    assert_eq!(fixture.store.clock().now(), fixture.clock.now());
}

#[tokio::test]
async fn the_fixture_sink_fails_with_e_db_005() {
    let fixture = Fixture::new();
    fixture.sink.fail_with(Fault::Unavailable);
    let event = SecurityEvent::forbidden(
        user(2),
        "CreateMachine",
        meta().request_id(),
        None,
        Some(UserAgent::from("test")),
    );
    let error = fixture.sink.record(event).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
}
