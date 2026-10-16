//! Tests of `Fixture::seed_recipes` and `Fixture::seed_large_draft` (design D12, task 8.3).
//!
//! Each test checks one item of the data of D12. The items are the machine `M20` and the
//! parameters of the draft of `R-100`. They are also the revision 3 of `ST_Zone`, the recipes
//! `R-200` and `R-300`, and the large draft.

use std::collections::BTreeSet;

use recipe_app::auth::RequestMeta;
use recipe_app::identity::authenticate;
use recipe_app::memory::{MemoryTransaction, seed_type_id};
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_app::recipe::{GetVersion, GetVersionInput};
use recipe_domain::audit::RequestId;
use recipe_domain::identity::{Role, SessionToken};
use recipe_domain::plant::{
    MachineCode, PlatformTypes, PlcPlatform, PlcTypeId, Primitive, RecordShape, TypeTree,
};
use recipe_domain::recipe::{ParameterKey, ReferenceNumber, StoredParameter, VersionState};
use uuid::Uuid;

use super::{
    GRID_KEY, HOLD_KEY, LABEL_KEY, LINE_A_KEY, LINE_KEY, RecipeIds, SPEED_KEY, SPEED_TABLE_KEY,
    ZONES_KEY,
};
use crate::test_support::Fixture;
use crate::test_support::plant::LINE_TAG;

/// Seed a fixture and return it with the fixed identifiers.
fn seeded() -> (Fixture, RecipeIds) {
    let fixture = Fixture::new();
    let ids = fixture.seed_recipes();
    (fixture, ids)
}

/// Open a read transaction on the store of `fixture`.
fn begin(fixture: &Fixture) -> MemoryTransaction {
    fixture.store.begin_memory(Isolation::ReadOnly).unwrap()
}

/// Return the parameters of the draft of `R-100`.
async fn draft_parameters(fixture: &Fixture, ids: &RecipeIds) -> Vec<StoredParameter> {
    begin(fixture)
        .recipes()
        .version_parameters(ids.plant.draft)
        .await
        .unwrap()
}

/// Return the parameter with the tag `tag`.
fn tagged<'a>(parameters: &'a [StoredParameter], tag: &str) -> &'a StoredParameter {
    parameters
        .iter()
        .find(|parameter| parameter.tag.as_str() == tag)
        .expect("the tag is seeded")
}

/// Resolve the type tree of `type_id` on the S7-300 of `M7`.
async fn tree_of(fixture: &Fixture, ids: &RecipeIds, type_id: PlcTypeId) -> TypeTree {
    let rows = begin(fixture)
        .types()
        .type_rows(ids.plant.m7)
        .await
        .unwrap();
    let types = PlatformTypes::collect(ids.plant.m7, PlcPlatform::S7_300, rows).unwrap();
    TypeTree::resolve(type_id, &types).unwrap()
}

#[test]
fn seed_recipes_returns_the_same_fixed_identifiers_on_each_call() {
    let (_first_fixture, first) = seeded();
    let (_second_fixture, second) = seeded();
    assert_eq!(first, second);
}

#[test]
fn seed_recipes_keeps_the_plant_data() {
    let (_fixture, ids) = seeded();
    assert_eq!(ids.plant, Fixture::new().seed_plant());
}

#[test]
fn seed_recipes_m20_is_a_live_twincat3_machine_with_the_units_u1_and_u2() {
    let (fixture, ids) = seeded();
    let machine = fixture.store.state().machine(ids.m20).unwrap();
    assert_eq!(machine.code(), MachineCode::parse("M20").unwrap());
    assert_eq!(machine.platform(), PlcPlatform::TwinCat3);
    assert!(machine.is_live());
    let paths: Vec<String> = machine
        .units()
        .map(|unit| unit.path().to_string())
        .collect();
    assert_eq!(paths.len(), 2, "{paths:?}");
    assert!(paths.contains(&"U1".to_owned()) && paths.contains(&"U2".to_owned()));
}

#[tokio::test]
async fn seed_recipes_the_draft_of_r100_holds_line_and_the_seven_parameters_of_d12() {
    let (fixture, ids) = seeded();
    let parameters = draft_parameters(&fixture, &ids).await;
    let mut tags: Vec<&str> = parameters.iter().map(|p| p.tag.as_str()).collect();
    tags.sort_unstable();
    let mut expected = [
        LINE_TAG,
        "SPEED",
        "LABEL",
        "HOLD",
        "SPEED_TABLE",
        "GRID",
        "LINE_A",
        "ZONES",
    ];
    expected.sort_unstable();
    assert_eq!(tags, expected);
    let ordinals: BTreeSet<u32> = parameters.iter().map(|p| p.ordinal.value()).collect();
    assert_eq!(ordinals.len(), parameters.len(), "an ordinal repeats");
    let units: BTreeSet<_> = parameters.iter().map(|p| p.unit).collect();
    assert_eq!(units.len(), 2, "the parameters stand on two units");
}

#[tokio::test]
async fn seed_recipes_the_keys_are_the_fixed_keys_of_the_constants() {
    let (fixture, ids) = seeded();
    let parameters = draft_parameters(&fixture, &ids).await;
    let keys = [
        (LINE_TAG, LINE_KEY),
        ("SPEED", SPEED_KEY),
        ("LABEL", LABEL_KEY),
        ("HOLD", HOLD_KEY),
        ("SPEED_TABLE", SPEED_TABLE_KEY),
        ("GRID", GRID_KEY),
        ("LINE_A", LINE_A_KEY),
        ("ZONES", ZONES_KEY),
    ];
    for (tag, key) in keys {
        assert_eq!(
            tagged(&parameters, tag).key,
            ParameterKey::from(key),
            "{tag}"
        );
    }
    let distinct: BTreeSet<Uuid> = keys.iter().map(|(_tag, key)| *key).collect();
    assert_eq!(distinct.len(), keys.len());
}

#[tokio::test]
async fn seed_recipes_each_parameter_has_its_type_and_its_leaf_count() {
    let (fixture, ids) = seeded();
    let parameters = draft_parameters(&fixture, &ids).await;
    let leaves = [
        ("SPEED", 1),
        ("LABEL", 1),
        ("HOLD", 1),
        ("SPEED_TABLE", 8),
        ("GRID", 4),
        ("LINE_A", 3),
        ("ZONES", 8),
    ];
    for (tag, count) in leaves {
        let tree = tree_of(&fixture, &ids, tagged(&parameters, tag).plc_type).await;
        assert_eq!(tree.leaf_count(), count, "{tag}");
    }
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let time = seed_type_id(Primitive::Time, None).unwrap();
    assert_eq!(tagged(&parameters, "SPEED").plc_type, int);
    assert_eq!(tagged(&parameters, "HOLD").plc_type, time);
    assert_eq!(tagged(&parameters, "ZONES").plc_type, ids.plant.zones_array);
}

#[tokio::test]
async fn seed_recipes_label_is_a_string_of_20_and_the_arrays_have_the_dimensions_of_d12() {
    let (fixture, ids) = seeded();
    let parameters = draft_parameters(&fixture, &ids).await;
    let rows = begin(&fixture)
        .types()
        .type_rows(ids.plant.m7)
        .await
        .unwrap();
    let shape = |tag: &str| {
        let id = tagged(&parameters, tag).plc_type;
        rows.iter()
            .find(|row| row.id() == id)
            .unwrap()
            .shape()
            .clone()
    };
    assert!(matches!(
        shape("LABEL"),
        RecordShape::Primitive {
            primitive: Primitive::String,
            string_length: Some(20),
            ..
        }
    ));
    let dimensions = |tag: &str| match shape(tag) {
        RecordShape::Array { dimensions, .. } => Some(dimensions.to_string()),
        _other => None,
    };
    assert_eq!(dimensions("SPEED_TABLE").as_deref(), Some("1..8"));
    assert_eq!(dimensions("GRID").as_deref(), Some("0..1,1..2"));
}

#[tokio::test]
async fn seed_recipes_speed_has_a_limit_0_to_3000_and_speed_table_a_limit_for_all_elements() {
    let (fixture, ids) = seeded();
    let parameters = draft_parameters(&fixture, &ids).await;
    let speed = &tagged(&parameters, "SPEED").limits;
    assert_eq!(speed.len(), 1);
    let limit = speed.first().unwrap();
    assert!(limit.leaf_path.is_empty());
    assert_eq!(limit.min.map(|n| n.to_string()), Some("0".to_owned()));
    assert_eq!(limit.max.map(|n| n.to_string()), Some("3000".to_owned()));
    let table = &tagged(&parameters, "SPEED_TABLE").limits;
    assert_eq!(table.len(), 1);
    assert_eq!(table.first().unwrap().leaf_path.to_string(), "[*]");
    for tag in ["LABEL", "HOLD", "GRID", "LINE_A", "ZONES"] {
        assert!(tagged(&parameters, tag).limits.is_empty(), "{tag}");
    }
}

#[tokio::test]
async fn seed_recipes_zone_3_has_one_more_member_than_zone_2_and_supersedes_it() {
    let (fixture, ids) = seeded();
    let rows = begin(&fixture)
        .types()
        .type_rows(ids.plant.m7)
        .await
        .unwrap();
    let members = |id: PlcTypeId| match rows.iter().find(|row| row.id() == id).unwrap().shape() {
        RecordShape::Struct {
            members,
            revision,
            supersedes,
            ..
        } => Some((members.len(), *revision, *supersedes)),
        _other => None,
    };
    let (two, revision_two, _) = members(ids.plant.zone_v2).unwrap();
    let (three, revision_three, supersedes) = members(ids.zone_v3).unwrap();
    assert_eq!((revision_two, revision_three), (2, 3));
    assert_eq!(three, two.checked_add(1).unwrap());
    assert_eq!(supersedes, Some(ids.plant.zone_v2));
}

#[tokio::test]
async fn seed_recipes_zones_is_outdated_and_the_other_new_parameters_are_not() {
    let (fixture, _ids) = seeded();
    let _admin = fixture.seed_session(1, Role::Administrator, false);
    let token = SessionToken::from_bytes([1; 16]);
    let auth = authenticate(fixture.state.identity(), &token)
        .await
        .unwrap()
        .unwrap();
    let meta = RequestMeta::new(RequestId::from(Uuid::nil()), None, None, None);
    let input = GetVersionInput {
        reference: ReferenceNumber::parse("R-100").unwrap(),
        number: 3,
        rebind_bound: 16_384,
    };
    let app = fixture.state.identity().app().clone();
    let view = GetVersion::new(app)
        .execute(auth.user(), &meta, input)
        .await
        .unwrap();
    let zones = view
        .outdated
        .iter()
        .find(|parameter| parameter.tag.as_str() == "ZONES")
        .expect("ZONES is outdated");
    assert_eq!(zones.type_name.to_string(), "ST_Zone");
    assert_eq!((zones.current, zones.newest), (2, 3));
    for parameter in &view.outdated {
        let tag = parameter.tag.as_str();
        assert!(tag == "ZONES" || tag == LINE_TAG, "{tag} is outdated");
    }
}

#[tokio::test]
async fn seed_recipes_r200_has_one_live_draft_without_parameters_on_m20() {
    let (fixture, ids) = seeded();
    let mut tx = begin(&fixture);
    let reference = ReferenceNumber::parse("R-200").unwrap();
    let recipe = tx
        .recipes()
        .recipe_by_reference(&reference)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recipe.id(), ids.r200);
    assert_eq!(recipe.machine(), ids.m20);
    assert!(!recipe.is_deleted());
    let states: Vec<(u32, VersionState)> = recipe
        .versions()
        .iter()
        .map(|header| (header.number(), header.state()))
        .collect();
    assert_eq!(states, [(1, VersionState::LiveDraft)]);
    let parameters = tx
        .recipes()
        .version_parameters(ids.r200_draft)
        .await
        .unwrap();
    assert!(parameters.is_empty());
}

#[tokio::test]
async fn seed_recipes_r300_is_a_deleted_recipe_of_m20() {
    let (fixture, ids) = seeded();
    let mut tx = begin(&fixture);
    let reference = ReferenceNumber::parse("R-300").unwrap();
    let recipe = tx
        .recipes()
        .recipe_by_reference(&reference)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recipe.id(), ids.r300);
    assert_eq!(recipe.machine(), ids.m20);
    assert!(recipe.is_deleted());
}

// --- The large draft -----------------------------------------------------------------------------

#[tokio::test]
async fn seed_large_draft_holds_4000_parameters_in_40_units() {
    let fixture = Fixture::new();
    let large = fixture.seed_large_draft();
    assert_eq!((large.parameters, large.units), (4000, 40));
    let mut tx = begin(&fixture);
    let reference = ReferenceNumber::parse(large.reference).unwrap();
    let recipe = tx
        .recipes()
        .recipe_by_reference(&reference)
        .await
        .unwrap()
        .unwrap();
    let draft = recipe.version(large.number).unwrap();
    assert_eq!(draft.state(), VersionState::LiveDraft);
    let parameters = tx.recipes().version_parameters(draft.id()).await.unwrap();
    assert_eq!(parameters.len(), 4000);
    let units: BTreeSet<_> = parameters.iter().map(|p| p.unit).collect();
    assert_eq!(units.len(), 40);
    let tags: BTreeSet<&str> = parameters.iter().map(|p| p.tag.as_str()).collect();
    assert_eq!(tags.len(), 4000, "a tag repeats");
}

#[test]
fn seed_large_draft_machine_has_40_live_units_and_leaves_the_small_fixture_alone() {
    let fixture = Fixture::new();
    let large = fixture.seed_large_draft();
    let machine = fixture.store.state().machine(large.machine).unwrap();
    assert!(machine.is_live());
    assert!(machine.units().count() >= 40);
    assert!(machine.units().all(|unit| unit.is_live()));
}

#[test]
fn seed_large_draft_returns_the_same_identifiers_on_each_call() {
    let first = Fixture::new().seed_large_draft();
    let second = Fixture::new().seed_large_draft();
    assert_eq!(first, second);
}
