//! Shared builders of the tests of the recipe root.

use time::OffsetDateTime;
use uuid::Uuid;

use super::*;

pub(super) fn rid(n: u128) -> RecipeId {
    RecipeId::from(Uuid::from_u128(n))
}

pub(super) fn vid(n: u128) -> RecipeVersionId {
    RecipeVersionId::from(Uuid::from_u128(n))
}

pub(super) fn machine_id() -> MachineId {
    MachineId::from(Uuid::from_u128(900))
}

pub(super) fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-1").unwrap()
}

pub(super) fn name(text: &str) -> Text {
    Text::parse(text).unwrap()
}

pub(super) fn ts(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

/// A stored header. The identifier derives from the number.
pub(super) fn header(number: u32, state: VersionState, row_version: i64) -> StoredVersionHeader {
    StoredVersionHeader {
        id: vid(u128::from(number)),
        number,
        state,
        row_version,
    }
}

pub(super) fn stored(
    deleted_at: Option<Timestamp>,
    versions: Vec<StoredVersionHeader>,
) -> StoredRecipe {
    StoredRecipe {
        id: rid(1),
        machine: machine_id(),
        reference: reference(),
        name: name("Recipe"),
        deleted_at,
        versions,
    }
}

/// Largest number for which `filled` adds the missing headers.
const FILL_LIMIT: u32 = 1_000;

/// The headers plus a discarded draft for every missing number below the largest one.
///
/// Storage never has a gap in the numbers, so a test names only the versions that it needs.
pub(super) fn filled(mut versions: Vec<StoredVersionHeader>) -> Vec<StoredVersionHeader> {
    let largest = versions.iter().map(|v| v.number).max().unwrap_or(0);
    if largest > FILL_LIMIT {
        return versions;
    }
    let missing: Vec<u32> = (1..=largest)
        .filter(|number| versions.iter().all(|v| v.number != *number))
        .collect();
    for number in missing {
        versions.push(StoredVersionHeader {
            id: vid(u128::from(number).checked_add(5_000).unwrap()),
            number,
            state: VersionState::Discarded,
            row_version: 2,
        });
    }
    versions
}

/// A live recipe with the given headers. Missing numbers get a discarded draft.
pub(super) fn recipe(versions: Vec<StoredVersionHeader>) -> Recipe {
    Recipe::try_from(stored(None, filled(versions))).unwrap()
}

/// A deleted recipe with the given headers. Missing numbers get a discarded draft.
pub(super) fn deleted_recipe(versions: Vec<StoredVersionHeader>) -> Recipe {
    Recipe::try_from(stored(Some(ts(1_000)), filled(versions))).unwrap()
}

/// The states of the headers of a recipe, in number order.
pub(super) fn states(recipe: &Recipe) -> Vec<(u32, VersionState, i64)> {
    recipe
        .versions()
        .iter()
        .map(|v| (v.number(), v.state(), v.row_version()))
        .collect()
}

/// A deleted recipe whose headers `TryFrom<StoredRecipe>` would reject.
///
/// The tests of the order of the checks need a deleted recipe with a live draft or an approved
/// version. The value cannot come from storage, so the builder skips the check.
pub(super) fn deleted_unchecked(versions: Vec<StoredVersionHeader>) -> Recipe {
    let mut headers: Vec<VersionHeader> = filled(versions)
        .into_iter()
        .map(|h| VersionHeader {
            id: h.id,
            number: h.number,
            state: h.state,
            row_version: h.row_version,
        })
        .collect();
    headers.sort_by_key(VersionHeader::number);
    Recipe {
        id: rid(1),
        machine: machine_id(),
        reference: reference(),
        name: name("Recipe"),
        deleted_at: Some(ts(1_000)),
        versions: headers,
    }
}
