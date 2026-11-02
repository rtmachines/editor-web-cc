//! The recipe `R-800` of `M7` for the tests of the unit scope export (`api.md` section 6.4).
//!
//! The function solves one problem: the unit scope tests need an approved version with parameters
//! on a unit, on its descendants, on a sibling, and on a unit whose path only starts with the same
//! text. The data comes from the real use cases, so the stored hash is the real hash.

use recipe_domain::plant::{UnitId, UnitPath};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::Text;
use uuid::Uuid;

use super::{ApprovalData, Draft, Runner, Spec};
use crate::test_support::Fixture;

/// The identifier of the unit `U1.20` of `M7`.
const SIBLING_UNIT_ID: Uuid = Uuid::from_u128(0x7357_0002_0007_0000_0000_0000_0000_0020);
/// The first request step of the seed commands of `R-800`.
const UNIT_STEP: u128 = 2_000;
/// The reference number of the recipe.
pub(crate) const UNIT_RECIPE: &str = "R-800";

/// One parameter of `R-800`: the unit, the tag, the type, the address, and the value.
struct UnitSpec {
    unit: &'static str,
    tag: &'static str,
    type_text: &'static str,
    address: &'static str,
    value: &'static str,
    /// The limit rows: leaf path, minimum, and maximum.
    limits: &'static [(&'static str, &'static str, &'static str)],
}

/// The parameters of `R-800`. Two parameters use a struct, so a test sees which types a scope keeps.
const UNIT_SPECS: [UnitSpec; 4] = [
    UnitSpec {
        unit: "U1",
        tag: "SPEED",
        type_text: "ST_Zone@3",
        address: "DB31.DBB0",
        value: r#"{"speed":1,"temp_max":2.5,"temp_min":0}"#,
        limits: &[
            (".speed", "0", "3000"),
            (".temp_max", "0", "100"),
            (".temp_min", "0", "100"),
        ],
    },
    UnitSpec {
        unit: "U1.2",
        tag: "ZONE1",
        type_text: "ST_Cell@1",
        address: "DB32.DBB0",
        value: r#"{"head":{"a":1,"b":2.5},"count":3}"#,
        limits: &[
            (".head.a", "0", "100"),
            (".head.b", "0", "10"),
            (".count", "0", "10"),
        ],
    },
    UnitSpec {
        unit: "U1.2.4",
        tag: "ZONE2",
        type_text: "DINT",
        address: "DB30.DBD8",
        value: "7",
        limits: &[("", "0", "3000")],
    },
    UnitSpec {
        unit: "U1.20",
        tag: "FEED",
        type_text: "INT",
        address: "DB30.DBW12",
        value: "3",
        limits: &[("", "0", "3000")],
    },
];

impl Fixture {
    /// Seed the approved version 1 of `R-800` on `M7` and the sibling unit `U1.20`.
    ///
    /// `SPEED` stands on `U1`, `ZONE1` on `U1.2`, `ZONE2` on `U1.2.4`, and `FEED` on `U1.20`. The
    /// path `U1.2` is a text prefix of `U1.20`, but `U1.20` is no descendant of `U1.2`. The unit
    /// `U1.3` holds no parameter. Call it once for each fixture, after `seed_approval_recipes`.
    pub(crate) async fn seed_unit_recipe(&self, data: &ApprovalData) {
        self.add_sibling_unit(data);
        let (anowak, admin) = (data.anowak_user(), data.admin_user());
        let mut runner = Runner {
            fixture: self,
            ctx: self.state.identity().app().clone(),
            step: UNIT_STEP,
        };
        let _ids = runner
            .create_recipe(&anowak, UNIT_RECIPE, "Unit recipe")
            .await;
        let mut draft = Draft {
            reference: ReferenceNumber::parse(UNIT_RECIPE).unwrap(),
            number: 1,
            row_version: runner.header(UNIT_RECIPE, 1).await.1,
        };
        for (index, unit_spec) in UNIT_SPECS.iter().enumerate() {
            let spec = Spec {
                tag: unit_spec.tag,
                ordinal: u32::try_from(index).unwrap(),
                type_text: unit_spec.type_text,
                address: unit_spec.address,
                value: unit_spec.value,
                limits: unit_spec.limits.to_vec(),
            };
            runner
                .add_on(&anowak, &mut draft, unit_spec.unit, &spec)
                .await;
        }
        runner.approve(&admin, UNIT_RECIPE, 1).await;
    }

    /// Add the live unit `U1.20` below `U1` of `M7`.
    fn add_sibling_unit(&self, data: &ApprovalData) {
        let mut machine = self.store.state().machine(data.recipes.plant.m7).unwrap();
        let parent = UnitPath::parse("U1").unwrap();
        machine
            .add_unit(
                UnitId::from(SIBLING_UNIT_ID),
                Some(&parent),
                "20",
                Text::parse("Unit U1.20").unwrap(),
            )
            .unwrap();
        self.store.seed_machine(&machine);
    }
}
