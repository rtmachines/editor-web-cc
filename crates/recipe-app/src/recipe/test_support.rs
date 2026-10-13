//! Helpers that the tests of several recipe use cases share.

use recipe_domain::plant::{LeafPath, Machine, PlcPlatform, UnitPath, UnitUsage};
use recipe_domain::recipe::{LimitInput, ParameterKey, Recipe, RecipeDeleter, StoredParameter};

use crate::memory::fixtures::{Fixture, parameter_on, pkey, timestamp, uid, user, vid};

/// The number of the version that [`seed_speed_draft`] stores.
pub(super) const DRAFT_VERSION: u128 = 101;

/// The key of the parameter `SPEED` that [`seed_speed_draft`] stores.
pub(super) fn speed_key() -> ParameterKey {
    pkey(1)
}

/// Store the machine `M1` on `TwinCat3`, the draft `R-1` (version 101), and the parameter `SPEED`.
///
/// `SPEED` is an `INT` with the value 120, the limit 10 to 250 on the leaf path `""`, and a
/// Polish translation. It sits on the unit 101. The draft has the row version 1.
pub(super) fn seed_speed_draft(fixture: &Fixture) -> Machine {
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let speed = parameter_on(PlcPlatform::TwinCat3, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    machine
}

/// Return the stored parameters of the draft that [`seed_speed_draft`] stores.
pub(super) fn stored_parameters(fixture: &Fixture) -> Vec<StoredParameter> {
    fixture.store.state().parameters_of(vid(DRAFT_VERSION))
}

/// Archive the unit at `path` of `machine`. The unit must hold no reference.
pub(super) fn archive_unit(machine: &mut Machine, path: &str) {
    machine
        .archive_unit(
            &UnitPath::parse(path).unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
}

/// A process limit on the leaf path `""` with the given bounds.
pub(super) fn bounds(min: &str, max: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    }
}

/// A `TwinCat3` draft with the parameter `SPEED` and no S7 address.
pub(super) fn twincat_draft() -> Fixture {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    fixture
}

/// A draft of an `S7_300` machine with no parameter.
pub(super) fn s7_draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
}

/// A draft of an `S7_300` machine with the `INT` parameter `SPEED` at `DB1.DBW0`.
pub(super) fn s7_draft_with_speed() -> Fixture {
    let fixture = s7_draft();
    let speed = parameter_on(PlcPlatform::S7_300, 1, "SPEED", uid(101), 0);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    fixture
}

/// The state of a draft that a rejected command must leave unchanged.
///
/// The type solves one problem: the table tests of several draft commands assert the same
/// "nothing was written" rule for the parameters, the row version, and the audit log.
pub(super) struct DraftSnapshot {
    parameters: String,
    row_version: Option<i64>,
}

impl DraftSnapshot {
    /// Capture the parameters and the row version of the draft.
    pub(super) fn capture(fixture: &Fixture) -> Self {
        Self {
            parameters: format!("{:?}", stored_parameters(fixture)),
            row_version: fixture
                .store
                .state()
                .version_row_version(vid(DRAFT_VERSION)),
        }
    }

    /// Assert that the draft, its editors, and the audit log did not change.
    pub(super) fn assert_unchanged(&self, fixture: &Fixture) {
        let now = Self::capture(fixture);
        assert_eq!(now.parameters, self.parameters);
        assert_eq!(now.row_version, self.row_version);
        assert!(fixture.store.audit_entries().is_empty());
        assert!(
            fixture
                .store
                .state()
                .editors_of(vid(DRAFT_VERSION))
                .is_empty()
        );
    }
}

/// Store recipe `n` of `machine` as a deleted recipe whose draft is discarded.
///
/// The recipe starts as the live draft of `Fixture::seed_draft`. The helper then deletes it
/// as an administrator and stores the rows again.
pub(super) fn seed_deleted(
    fixture: &Fixture,
    machine: &Machine,
    n: u128,
    reference_text: &str,
) -> Recipe {
    let (mut recipe, _) = fixture.seed_draft(machine, n, reference_text);
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, machine.platform(), user(2));
    recipe
}
