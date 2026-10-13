//! Property tests of the S7 rules on random parameter sets in one data block
//! (`reliability.md` section 6).

use proptest::prelude::*;

use super::super::support::{stored_draft_with, stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{id, machine, on_platform, primitive_row, string_row, types_for};
use crate::plant::{PlcPlatform, PlcType, Primitive, S7Address, TypeTrees};
use crate::recipe::{Draft, DraftError, StoredParameter, VersionState};

/// Number of cases of each property.
const CASES: u32 = 64;
/// Bits in one byte.
const BITS_PER_BYTE: u8 = 8;
/// Smallest and largest number of parameters in a generated set.
const MIN_PARAMETERS: usize = 1;
const MAX_PARAMETERS: usize = 12;
/// Largest number of unused bytes before a parameter.
const MAX_GAP: u32 = 4;

/// The kinds of parameter in a generated set. Each kind has its own S7 size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Bool,
    Byte,
    Int,
    Dint,
    Real,
    Lreal,
    String3,
    String10,
}

impl Kind {
    const ALL: [Self; 8] = [
        Self::Bool,
        Self::Byte,
        Self::Int,
        Self::Dint,
        Self::Real,
        Self::Lreal,
        Self::String3,
        Self::String10,
    ];

    /// The kinds that need an even start.
    const WIDE: [Self; 6] = [
        Self::Int,
        Self::Dint,
        Self::Real,
        Self::Lreal,
        Self::String3,
        Self::String10,
    ];

    /// The number of the type row of the kind.
    const fn type_n(self) -> u128 {
        match self {
            Self::Bool => 1,
            Self::Byte => 2,
            Self::Int => 3,
            Self::Dint => 4,
            Self::Real => 5,
            Self::Lreal => 6,
            Self::String3 => 7,
            Self::String10 => 8,
        }
    }

    /// The size in bytes. A `BOOL` has the size of its byte.
    const fn size_bytes(self) -> u32 {
        match self {
            Self::Bool | Self::Byte => 1,
            Self::Int => 2,
            Self::Dint | Self::Real => 4,
            Self::Lreal => 8,
            Self::String3 => 5,
            Self::String10 => 12,
        }
    }

    const fn is_wide(self) -> bool {
        !matches!(self, Self::Bool | Self::Byte)
    }
}

/// Where a generated parameter starts.
#[derive(Clone, Copy, Debug)]
struct Placement {
    kind: Kind,
    offset: u32,
    bit: u8,
}

impl Placement {
    /// The address of the placement in data block 1.
    fn address(self) -> S7Address {
        let text = match self.kind {
            Kind::Bool => format!("DB1.DBX{}.{}", self.offset, self.bit),
            Kind::Byte | Kind::Lreal | Kind::String3 | Kind::String10 => {
                format!("DB1.DBB{}", self.offset)
            }
            Kind::Int => format!("DB1.DBW{}", self.offset),
            Kind::Dint | Kind::Real => format!("DB1.DBD{}", self.offset),
        };
        S7Address::parse(&text).unwrap()
    }
}

fn kind() -> impl Strategy<Value = Kind> {
    prop::sample::select(Kind::ALL.to_vec())
}

fn wide_kind() -> impl Strategy<Value = Kind> {
    prop::sample::select(Kind::WIDE.to_vec())
}

/// A set of 1 to 12 kinds, each with a gap of unused bytes before it.
fn items() -> impl Strategy<Value = Vec<(Kind, u32)>> {
    prop::collection::vec((kind(), 0..=MAX_GAP), MIN_PARAMETERS..=MAX_PARAMETERS)
}

/// Place the items one after the other without overlap.
///
/// A wide kind starts at an even byte. The `BOOL` items share a byte at distinct bits.
/// The result holds the placements and the first free byte after the last one.
fn place(items: &[(Kind, u32)]) -> (Vec<Placement>, u32) {
    let mut cursor = 0_u32;
    let mut bool_slot: Option<(u32, u8)> = None;
    let mut placed = Vec::new();
    for (kind, gap) in items {
        cursor = cursor.checked_add(*gap).unwrap();
        if *kind == Kind::Bool {
            let (offset, bit) = match bool_slot {
                Some((offset, bit)) if bit < BITS_PER_BYTE => (offset, bit),
                _ => {
                    let offset = cursor;
                    cursor = cursor.checked_add(1).unwrap();
                    (offset, 0)
                }
            };
            bool_slot = Some((offset, bit.checked_add(1).unwrap()));
            placed.push(Placement {
                kind: *kind,
                offset,
                bit,
            });
            continue;
        }
        if kind.is_wide() {
            cursor = cursor.next_multiple_of(2);
        }
        placed.push(Placement {
            kind: *kind,
            offset: cursor,
            bit: 0,
        });
        cursor = cursor.checked_add(kind.size_bytes()).unwrap();
    }
    (placed, cursor)
}

fn rows() -> Vec<PlcType> {
    vec![
        primitive_row(Kind::Bool.type_n(), Primitive::Bool),
        primitive_row(Kind::Byte.type_n(), Primitive::Byte),
        primitive_row(Kind::Int.type_n(), Primitive::Int),
        primitive_row(Kind::Dint.type_n(), Primitive::Dint),
        primitive_row(Kind::Real.type_n(), Primitive::Real),
        primitive_row(Kind::Lreal.type_n(), Primitive::Lreal),
        string_row(Kind::String3.type_n(), 3),
        string_row(Kind::String10.type_n(), 10),
    ]
}

fn trees() -> TypeTrees {
    let ids = Kind::ALL.map(|kind| id(kind.type_n()));
    TypeTrees::resolve(
        ids,
        &types_for(
            machine(),
            PlcPlatform::S7_300,
            on_platform(PlcPlatform::S7_300, rows()),
        ),
    )
}

/// Check a set of placements on an S7-300 draft. The tags are `P00`, `P01`, and so on.
fn check(placements: &[Placement]) -> Result<(), DraftError> {
    let parameters: Vec<StoredParameter> = placements
        .iter()
        .enumerate()
        .map(|(index, placement)| {
            let n = u128::try_from(index).unwrap().checked_add(1).unwrap();
            let tag = format!("P{index:02}");
            stored_parameter(
                n,
                &tag,
                id(placement.kind.type_n()),
                Some(placement.address()),
            )
        })
        .collect();
    let stored = stored_draft_with(VersionState::LiveDraft, PlcPlatform::S7_300, parameters);
    Draft::load(stored).unwrap().check_s7_layout(&trees())
}

/// Return the code of the error, or `None` when the check passed.
fn code_of(result: Result<(), DraftError>) -> Option<ErrorCode> {
    result.err().map(|error| error.code())
}

/// Move `victim` onto the first bytes of `target`.
///
/// A wide victim starts at the even byte at or below the offset of the target. It keeps its
/// alignment and still covers the first byte of the target.
fn move_onto(victim: Placement, target: Placement) -> Placement {
    let offset = if victim.kind.is_wide() {
        target.offset & !1
    } else {
        target.offset
    };
    let bit = if victim.kind == Kind::Bool && target.kind == Kind::Bool {
        target.bit
    } else {
        0
    };
    Placement {
        kind: victim.kind,
        offset,
        bit,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    #[test]
    fn a_set_without_overlap_and_with_aligned_starts_passes(items in items()) {
        let (placed, _) = place(&items);
        prop_assert_eq!(code_of(check(&placed)), None);
    }

    #[test]
    fn a_parameter_moved_onto_another_always_gives_e_rcp_022(
        items in prop::collection::vec((kind(), 0..=MAX_GAP), 2..=MAX_PARAMETERS),
        victim_pick in any::<prop::sample::Index>(),
        target_pick in any::<prop::sample::Index>(),
    ) {
        let (mut placed, _) = place(&items);
        let victim = victim_pick.index(placed.len());
        let shift = target_pick.index(placed.len().checked_sub(1).unwrap());
        let target = if shift >= victim { shift.checked_add(1).unwrap() } else { shift };
        placed[victim] = move_onto(placed[victim], placed[target]);
        prop_assert_eq!(code_of(check(&placed)), Some(ErrorCode::RCP_022));
    }

    #[test]
    fn a_wide_parameter_at_an_odd_offset_always_gives_e_rcp_023(
        items in items(),
        kind in wide_kind(),
    ) {
        let (mut placed, end) = place(&items);
        let odd = if end.is_multiple_of(2) { end.checked_add(1).unwrap() } else { end };
        placed.push(Placement { kind, offset: odd, bit: 0 });
        prop_assert_eq!(code_of(check(&placed)), Some(ErrorCode::RCP_023));
    }
}
