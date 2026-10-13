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
/// The platforms with `WSTRING(254)`.
const S7_WIDE: &[PlcPlatform] = &[PlcPlatform::S7_1200, PlcPlatform::S7_1500];

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
const SEEDS: [Seed; 10] = [
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
];

/// Return the identifier of a seed row, or `None` when the row is not in the seed.
///
/// The seed has `BOOL`, `INT`, `DINT`, `REAL`, `LREAL`, `TIME`, `STRING(254)`, `STRING(80)`,
/// `WSTRING(254)`, and `WSTRING(80)`. A primitive without length takes `None`.
#[must_use]
pub fn seed_type_id(primitive: Primitive, string_length: Option<u16>) -> Option<PlcTypeId> {
    let index = SEEDS
        .iter()
        .position(|row| row.primitive == primitive && row.string_length == string_length)?;
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
    /// The rows are `BOOL`, `INT`, `DINT`, `REAL`, `LREAL`, `TIME`, the seed `STRING(254)` and
    /// `STRING(80)`, and the seed `WSTRING(254)` and `WSTRING(80)`, with the platforms of the
    /// matrix. [`seed_type_id`] gives their identifiers. `seed_type` adds more rows.
    pub fn seed_primitive_types(&self) {
        for row in SEEDS.iter().filter_map(seed_row) {
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
