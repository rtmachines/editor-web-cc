//! Shared builders of the tests of the diff.

use crate::plant::fixtures::{
    PLATFORM, array_row, id, machine, primitive_row, string_row, struct_row, struct_row_revision,
    types_for,
};
use crate::plant::{Primitive, TypeTrees};
use crate::recipe::{
    DiffPair, DiffRows, LangCode, Parameter, ParameterKey, RecipeError, StoredParameter,
    Translation, diff_pairs,
};
use crate::text::Text;

pub(super) const INT: u128 = 1;
pub(super) const REAL: u128 = 10;
pub(super) const TIME: u128 = 11;
/// `ARRAY[1..8] OF INT`.
pub(super) const TABLE: u128 = 12;
/// `ST_Zone` with the member `setpoint` of type `REAL`.
pub(super) const ZONE: u128 = 13;
/// `ST_Line` with the member `zone` of type `ST_Zone`.
pub(super) const LINE: u128 = 14;
/// `ST_Pair` with the members `a` and `b` of type `INT`.
pub(super) const PAIR: u128 = 15;
/// `ST_PairC` with the members `a` and `c` of type `INT`.
pub(super) const PAIR_C: u128 = 16;
pub(super) const DINT: u128 = 17;
/// `ARRAY[1..8] OF DINT`, with the bounds of [`TABLE`].
pub(super) const DINT_TABLE: u128 = 18;
/// `ST_Zone` revision 2 with the members `setpoint` and `ramp` of type `REAL`.
pub(super) const ZONE_2: u128 = 19;
/// `ARRAY[1..2] OF ST_Zone` revision 1.
pub(super) const ZONES_1: u128 = 20;
/// `ARRAY[1..2] OF ST_Zone` revision 2.
pub(super) const ZONES_2: u128 = 21;
/// `ARRAY[1..10] OF INT`.
pub(super) const TEN: u128 = 22;
/// `ST_Order` with the members `zeta` and `alpha` of type `INT`, in this order.
pub(super) const ORDER: u128 = 23;
pub(super) const LTIME: u128 = 24;
/// `DATE_AND_TIME`, an instant with a count of seconds on TwinCAT 3.
pub(super) const DT: u128 = 25;
/// `STRING(10)`.
pub(super) const NAME: u128 = 26;
/// `ARRAY[1..2] OF INT`, the element of [`GRID`].
pub(super) const ROW2: u128 = 27;
/// `ARRAY[1..2] OF ARRAY[1..2] OF INT`.
pub(super) const GRID: u128 = 28;
/// A type that the trees do not hold.
pub(super) const MISSING: u128 = 99;

/// The value of `diff.leaf_rows` (`reliability.md` section 7.11).
pub(super) const ROWS: u32 = 1_000;

/// The trees of every type of the diff tests.
pub(super) fn trees() -> TypeTrees {
    let rows = vec![
        primitive_row(INT, Primitive::Int),
        primitive_row(REAL, Primitive::Real),
        primitive_row(TIME, Primitive::Time),
        array_row(TABLE, INT, &[(1, 8)]),
        struct_row(ZONE, "ST_Zone", &[("setpoint", REAL)]),
        struct_row(LINE, "ST_Line", &[("zone", ZONE)]),
        struct_row(PAIR, "ST_Pair", &[("a", INT), ("b", INT)]),
        struct_row(PAIR_C, "ST_PairC", &[("a", INT), ("c", INT)]),
        primitive_row(DINT, Primitive::Dint),
        array_row(DINT_TABLE, DINT, &[(1, 8)]),
        struct_row_revision(ZONE_2, "ST_Zone", 2, &[("setpoint", REAL), ("ramp", REAL)]),
        array_row(ZONES_1, ZONE, &[(1, 2)]),
        array_row(ZONES_2, ZONE_2, &[(1, 2)]),
        array_row(TEN, INT, &[(1, 10)]),
        struct_row(ORDER, "ST_Order", &[("zeta", INT), ("alpha", INT)]),
        primitive_row(LTIME, Primitive::Ltime),
        primitive_row(DT, Primitive::DateAndTime),
        string_row(NAME, 10),
        array_row(ROW2, INT, &[(1, 2)]),
        array_row(GRID, ROW2, &[(1, 2)]),
    ];
    let ids = [
        INT, REAL, TIME, TABLE, ZONE, LINE, PAIR, PAIR_C, DINT, DINT_TABLE, ZONE_2, ZONES_1,
        ZONES_2, TEN, ORDER, LTIME, DT, NAME, ROW2, GRID,
    ]
    .map(id);
    TypeTrees::resolve(ids, &types_for(machine(), PLATFORM, rows))
}

/// A translation into `lang` with the name `name` and no description.
pub(super) fn translation(lang: &str, name: &str) -> Translation {
    Translation {
        lang: LangCode::parse(lang).unwrap(),
        name: Text::parse(name).unwrap(),
        description: None,
    }
}

/// Return the keys in the order of `new`, then in the order of the keys only in `old`.
fn keys_of(old: &[Parameter], new: &[Parameter]) -> Vec<ParameterKey> {
    let new_keys: Vec<ParameterKey> = new.iter().map(Parameter::key).collect();
    let old_only = old
        .iter()
        .map(Parameter::key)
        .filter(|key| !new_keys.contains(key));
    new_keys.iter().copied().chain(old_only).collect()
}

/// Convert stored parameters into parameters.
fn parameters(stored: Vec<StoredParameter>) -> Vec<Parameter> {
    stored.into_iter().map(Parameter::from).collect()
}

/// Diff the parameters with `leaf_rows` rows for each part, with the keys of [`keys_of`].
pub(super) fn diff_of(
    old: Vec<StoredParameter>,
    new: Vec<StoredParameter>,
    leaf_rows: u32,
) -> DiffRows {
    let (old, new) = (parameters(old), parameters(new));
    let pairs = DiffPair::match_by_key(&keys_of(&old, &new), &old, &new);
    diff_pairs(&pairs, &trees(), leaf_rows)
}

/// Check the values of the pairs of the parameters, with the keys of [`keys_of`].
pub(super) fn check_of(
    old: Vec<StoredParameter>,
    new: Vec<StoredParameter>,
) -> Result<(), RecipeError> {
    let (old, new) = (parameters(old), parameters(new));
    let trees = trees();
    DiffPair::match_by_key(&keys_of(&old, &new), &old, &new)
        .iter()
        .try_for_each(|pair| pair.check_stored(&trees))
}
