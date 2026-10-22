//! The diff of the definition of one parameter (ADR-0050 (c)).

use super::rows::{DefinitionChange, DiffPart};
use crate::recipe::Parameter;

/// Return one row for each field of the definition that differs.
///
/// The rows come in the order tag, unit, type, S7 address, ordinal, English name, and English
/// description. The type compares by identifier and the row holds both identifiers.
pub(super) fn definition_rows(old: &Parameter, new: &Parameter) -> Vec<DiffPart> {
    let changes = [
        (old.tag() != new.tag()).then(|| DefinitionChange::Tag {
            old: old.tag().clone(),
            new: new.tag().clone(),
        }),
        (old.unit() != new.unit()).then(|| DefinitionChange::Unit {
            old: old.unit(),
            new: new.unit(),
        }),
        (old.plc_type() != new.plc_type()).then(|| DefinitionChange::Type {
            old: old.plc_type(),
            new: new.plc_type(),
        }),
        (old.s7_address() != new.s7_address()).then(|| DefinitionChange::S7Address {
            old: old.s7_address(),
            new: new.s7_address(),
        }),
        (old.ordinal() != new.ordinal()).then(|| DefinitionChange::Ordinal {
            old: old.ordinal(),
            new: new.ordinal(),
        }),
        (old.name_en() != new.name_en()).then(|| DefinitionChange::NameEn {
            old: old.name_en().clone(),
            new: new.name_en().clone(),
        }),
        (old.description_en() != new.description_en()).then(|| DefinitionChange::DescriptionEn {
            old: old.description_en().clone(),
            new: new.description_en().clone(),
        }),
    ];
    changes
        .into_iter()
        .flatten()
        .map(DiffPart::Definition)
        .collect()
}
