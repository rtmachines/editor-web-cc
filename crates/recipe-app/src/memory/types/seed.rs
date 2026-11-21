//! Seed rows of the global primitive catalog (`data-model.md` section 7).

use std::collections::BTreeSet;

use recipe_domain::plant::{
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind,
    StringLength,
};
use uuid::Uuid;

use crate::memory::MemoryStore;
use crate::memory::rows::TypeRow;

/// The platforms of the S7 family that have the 254-character defaults.
const S7_ALL: &[PlcPlatform] = &[
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];
/// Every platform.
const ALL: &[PlcPlatform] = &PlcPlatform::ALL;
/// The platforms of the IEC family that have the 80-character defaults.
const IEC: &[PlcPlatform] = &[PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];
/// Every platform except `S7_300`.
const NOT_S7_300: &[PlcPlatform] = &[
    PlcPlatform::CodesysIec,
    PlcPlatform::TwinCat3,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];
/// Every platform except `S7_1200`.
const NOT_S7_1200: &[PlcPlatform] = &[
    PlcPlatform::CodesysIec,
    PlcPlatform::TwinCat3,
    PlcPlatform::S7_1500,
    PlcPlatform::S7_300,
];
/// The platforms with `WSTRING(254)`.
const S7_WIDE: &[PlcPlatform] = &[PlcPlatform::S7_1200, PlcPlatform::S7_1500];

/// The platforms with `LTIME`: the IEC family and `S7_1500`.
const LTIME_PLATFORMS: &[PlcPlatform] = &[
    PlcPlatform::CodesysIec,
    PlcPlatform::TwinCat3,
    PlcPlatform::S7_1500,
];

/// The high bits that mark a seed identifier.
const SEED_ID_PREFIX: u128 = 0x5eed_0000_0000_0000_0000_0000_0000_0000;

/// One seed row: the primitive, the string length, and the platforms of the matrix.
struct Seed {
    primitive: Primitive,
    string_length: Option<u16>,
    platforms: &'static [PlcPlatform],
}

const fn seed(
    primitive: Primitive,
    string_length: Option<u16>,
    platforms: &'static [PlcPlatform],
) -> Seed {
    Seed {
        primitive,
        string_length,
        platforms,
    }
}

/// The seed rows. The position in this list gives the identifier.
///
/// Append a new row at the end, so that no identifier changes.
const SEEDS: [Seed; 11] = [
    seed(Primitive::Bool, None, ALL),
    seed(Primitive::Int, None, ALL),
    seed(Primitive::Dint, None, ALL),
    seed(Primitive::Real, None, ALL),
    seed(Primitive::Lreal, None, NOT_S7_300),
    seed(Primitive::Time, None, ALL),
    seed(Primitive::String, Some(254), S7_ALL),
    seed(Primitive::String, Some(80), IEC),
    seed(Primitive::Wstring, Some(254), S7_WIDE),
    seed(Primitive::Wstring, Some(80), IEC),
    seed(Primitive::Ltime, None, LTIME_PLATFORMS),
];

/// The seed rows of the time-of-day and date-and-time primitives.
///
/// [`MemoryStore::seed_primitive_types`] leaves them out, so a test that counts the seed rows
/// stays as it is. [`MemoryStore::seed_time_primitive_types`] inserts them. Their numbers start at
/// [`TIME_SEED_OFFSET`], so they never share an identifier with a row of [`SEEDS`].
const TIME_SEEDS: [Seed; 4] = [
    seed(Primitive::TimeOfDay, None, ALL),
    seed(Primitive::DateAndTime, None, NOT_S7_1200),
    seed(Primitive::Ltod, None, LTIME_PLATFORMS),
    seed(Primitive::Ldt, None, LTIME_PLATFORMS),
];

/// The index that the first row of [`TIME_SEEDS`] gets in the numbering of the seed identifiers.
const TIME_SEED_OFFSET: usize = 100;

/// Return the index of the row of `primitive` with `string_length` in `rows`.
fn position(rows: &[Seed], primitive: Primitive, string_length: Option<u16>) -> Option<usize> {
    rows.iter()
        .position(|row| row.primitive == primitive && row.string_length == string_length)
}

/// Return the identifier of a seed row, or `None` when the row is not in the seed.
///
/// The seed has `BOOL`, `INT`, `DINT`, `REAL`, `LREAL`, `TIME`, `STRING(254)`, `STRING(80)`,
/// `WSTRING(254)`, `WSTRING(80)`, and `LTIME`. It also gives the identifiers of the rows that
/// [`MemoryStore::seed_time_primitive_types`] inserts: `TIME_OF_DAY`, `DATE_AND_TIME`, `LTOD`, and
/// `LDT`. A primitive without length takes `None`.
#[must_use]
pub fn seed_type_id(primitive: Primitive, string_length: Option<u16>) -> Option<PlcTypeId> {
    let index = position(&SEEDS, primitive, string_length).or_else(|| {
        position(&TIME_SEEDS, primitive, string_length)?.checked_add(TIME_SEED_OFFSET)
    })?;
    let number = u128::try_from(index).ok()?.checked_add(1)?;
    Some(PlcTypeId::from(Uuid::from_u128(SEED_ID_PREFIX | number)))
}

fn string_kind(primitive: Primitive) -> Option<StringKind> {
    match primitive {
        Primitive::String => Some(StringKind::String),
        Primitive::Wstring => Some(StringKind::Wstring),
        _ => None,
    }
}

/// Return the S7 size of a seed row.
fn size_bytes(row: &Seed) -> Option<u32> {
    let (Some(kind), Some(length)) = (string_kind(row.primitive), row.string_length) else {
        return row.primitive.fixed_size_bytes();
    };
    let platform = *row.platforms.first()?;
    StringLength::parse(kind, platform, u32::from(length))
        .ok()?
        .size_bytes(kind)
}

/// Build the valid type row of a seed row.
fn seed_row(row: &Seed) -> Option<PlcType> {
    let record = PlcTypeRecord {
        id: seed_type_id(row.primitive, row.string_length)?,
        machine: None,
        platforms: row.platforms.iter().copied().collect::<BTreeSet<_>>(),
        shape: RecordShape::Primitive {
            primitive: row.primitive,
            leaf_class: row.primitive.leaf_class(),
            size_bytes: size_bytes(row)?,
            string_length: row.string_length,
        },
    };
    PlcType::try_from(record).ok()
}

impl MemoryStore {
    /// Insert the global primitive rows of `data-model.md` section 7 that the tests need.
    ///
    /// The rows are `BOOL`, `INT`, `DINT`, `REAL`, `LREAL`, `TIME`, `LTIME`, and the seed `STRING`
    /// and `WSTRING` rows of 254 and 80 characters. Each row has the platforms of the matrix.
    /// [`seed_type_id`] gives their identifiers. `seed_type` adds more rows.
    pub fn seed_primitive_types(&self) {
        for row in SEEDS.iter().filter_map(seed_row) {
            self.seed_type(&row);
        }
    }

    /// Insert the rows `TIME_OF_DAY`, `DATE_AND_TIME`, `LTOD`, and `LDT` of the global catalog.
    ///
    /// [`MemoryStore::seed_primitive_types`] does not insert them. Each row has the platforms of
    /// the matrix. [`seed_type_id`] gives their identifiers.
    pub fn seed_time_primitive_types(&self) {
        for row in TIME_SEEDS.iter().filter_map(seed_row) {
            self.seed_type(&row);
        }
    }

    /// Insert one type row into the committed state, without a constraint check.
    ///
    /// The time of the clock is the `created_at` of the row.
    pub fn seed_type(&self, plc_type: &PlcType) {
        self.update_state(|state, now| {
            let row = TypeRow {
                plc_type: plc_type.clone(),
                created_at: now,
            };
            state.types.insert(plc_type.id(), row);
        });
    }
}
