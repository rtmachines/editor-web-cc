//! Use of units by recipes (`domain-model.md` section 5.1, `ArchiveUnit`).

use std::collections::BTreeMap;

use crate::recipe::ReferenceNumber;

use super::UnitId;

/// The units that a live draft or an approved version uses.
///
/// The use case loads this resolved value before it calls `Machine::archive_unit`
/// (`domain-model.md` section 5.1 and `data-model.md` section 6, `ArchiveUnit`).
/// The type solves one problem: the machine aggregate has no access to recipes,
/// but the archive of a unit must know which recipe still uses it.
/// When several recipes use one unit, the smallest reference number is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitUsage {
    drafts: BTreeMap<UnitId, ReferenceNumber>,
    approved: BTreeMap<UnitId, ReferenceNumber>,
}

impl UnitUsage {
    /// Record that the live draft of the recipe `reference` uses `unit`.
    pub fn add_draft(&mut self, unit: UnitId, reference: ReferenceNumber) {
        keep_smallest(&mut self.drafts, unit, reference);
    }

    /// Record that the approved version of the recipe `reference` uses `unit`.
    pub fn add_approved(&mut self, unit: UnitId, reference: ReferenceNumber) {
        keep_smallest(&mut self.approved, unit, reference);
    }

    /// Return the recipe whose live draft uses `unit`.
    pub(super) fn draft_of(&self, unit: UnitId) -> Option<&ReferenceNumber> {
        self.drafts.get(&unit)
    }

    /// Return the recipe whose approved version uses `unit`.
    pub(super) fn approved_of(&self, unit: UnitId) -> Option<&ReferenceNumber> {
        self.approved.get(&unit)
    }
}

/// Insert `reference` for `unit` unless a smaller reference is already there.
fn keep_smallest(
    map: &mut BTreeMap<UnitId, ReferenceNumber>,
    unit: UnitId,
    reference: ReferenceNumber,
) {
    map.entry(unit)
        .and_modify(|known| {
            if reference < *known {
                *known = reference.clone();
            }
        })
        .or_insert(reference);
}
