//! The extra usage rows of the pager tests (task 8.4).
//!
//! The module solves one problem: the usage page shows 100 rows at most, so a pager test needs
//! more than 100 parameters that use `ST_Zone@2`. `seed_plant` seeds one. This module adds the rest
//! with `MemoryStore::seed_parameter`, which writes without a check.

use recipe_app::auth::RequestMeta;
use recipe_app::identity::authenticate;
use recipe_app::plant::{GetTypeTree, TypeRef};
use recipe_domain::audit::RequestId;
use recipe_domain::identity::SessionToken;
use recipe_domain::plant::{MachineCode, TypeName, UnitId};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, StoredParameter,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::test_support::plant::World;

/// The value of a parameter of the type `ST_Zone@2`.
const ZONE_VALUE: &str = r#"{"speed":0,"temp_max":0}"#;
/// The first field of the identifier of a unit of `seed_plant`. The next field is the machine number.
const UNIT_TAG: u32 = 0x7357_0002;
/// The machine number of `M7`.
const M7_NUMBER: u16 = 7;
/// The first field of the identifier of an extra parameter.
const PARAMETER_TAG: u128 = 0x7357_0010_0000_0000_0000_0000_0000_0000;
/// The first field of the key of an extra parameter.
const KEY_TAG: u128 = 0x7357_0011_0000_0000_0000_0000_0000_0000;

/// Add `count` parameters of the type `ST_Zone@2` to the live draft of `R-100`.
///
/// The tags are `P0001`, `P0002`, and so on. They come after the tag `LINE` in the order of the
/// usage page, so the usage list holds `count + 1` rows.
pub(super) async fn seed_zone_parameters(world: &World, count: u32) {
    let token = SessionToken::from_bytes([2; 16]);
    let identity = world.fixture.state.identity();
    let auth = authenticate(identity, &token).await.unwrap().unwrap();
    let meta = RequestMeta::new(RequestId::from(Uuid::nil()), None, None, None);
    let view = GetTypeTree::new(identity.app().clone())
        .execute(
            auth.user(),
            &meta,
            TypeRef {
                code: MachineCode::parse("M7").unwrap(),
                name: TypeName::parse("ST_Zone").unwrap(),
                revision: Some(2),
            },
        )
        .await
        .unwrap();
    let value = ParameterValue::parse(ZONE_VALUE, &view.tree)
        .unwrap()
        .to_stored();
    let unit = UnitId::from(Uuid::from_fields(
        UNIT_TAG,
        M7_NUMBER,
        0,
        &[1, 0, 0, 0, 0, 0, 0, 0],
    ));
    for index in 1..=count {
        let number = u128::from(index);
        let parameter = StoredParameter {
            id: ParameterId::from(Uuid::from_u128(PARAMETER_TAG | number)),
            key: ParameterKey::from(Uuid::from_u128(KEY_TAG | number)),
            unit,
            tag: ParameterTag::parse(&format!("P{index:04}")).unwrap(),
            name_en: Text::parse("Extra zone").unwrap(),
            description_en: LongText::parse("An extra zone for the pager test").unwrap(),
            translations: Vec::new(),
            plc_type: world.ids.zone_v2,
            s7_address: None,
            limits: Vec::new(),
            value: value.clone(),
            ordinal: Ordinal::parse(index).unwrap(),
        };
        world
            .fixture
            .store
            .seed_parameter(world.ids.draft, &parameter);
    }
}
