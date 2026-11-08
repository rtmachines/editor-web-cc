//! S7 rules of a draft (design D10, invariants I5 and I20, ADR-0024).

use std::collections::BTreeMap;

use crate::plant::{
    PlantError, PlcPlatform, PlcTypeId, S7Address, S7Layout, S7Start, S7Width, TypeLimit, TypeTree,
    TypeTrees,
};
use crate::recipe::{Draft, DraftError, Parameter, ParameterKey, ParameterTag, RecipeError};

/// Number of bits in one byte.
const BITS_PER_BYTE: u64 = 8;

/// The size of one data block in bytes on every platform (ADR-0116 (a)).
///
/// The number is a fact of the platform family, not a setting.
const S7_DATA_BLOCK_BYTES: u32 = 65_536;

/// The highest data block number of an S7-300 controller (ADR-0116 (c)).
///
/// The number is a fact of the platform family, not a setting.
const S7_300_MAX_DATA_BLOCK: u16 = 16_000;

/// The highest data block number of an S7-1200 and an S7-1500 controller (ADR-0140 (a)).
///
/// The number is a fact of the platform family, not a setting.
const S7_1X00_MAX_DATA_BLOCK: u16 = 59_999;

/// Whether a whole-draft check runs the address rules of ADR-0116.
///
/// A check for approval runs them. A check after `replace_parameters` skips them, so a stored
/// address of another parameter does not refuse a command (ADR-0116 (e1)).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AddressRules {
    Run,
    Skip,
}

/// Which overlaps a whole-draft check reports (ADR-0149).
///
/// The type solves one problem: a rule change can leave a stored overlap in a draft. A command
/// must not lock the draft because of a pair that it does not touch.
#[derive(Clone, Copy)]
enum OverlapScope<'a> {
    /// Every overlap of the draft. Approval uses this scope (ADR-0149 (c)).
    Every,
    /// Only an overlap of a pair that holds one of these parameters (ADR-0149 (a), (b)).
    Written(&'a [ParameterKey]),
}

impl OverlapScope<'_> {
    /// Whether an overlap that holds the parameter `key` is reported.
    fn reports(self, key: ParameterKey) -> bool {
        match self {
            Self::Every => true,
            Self::Written(written) => written.contains(&key),
        }
    }
}

/// The bits that one parameter occupies in its data block. The end is exclusive.
struct Occupied<'a> {
    db: u16,
    start: u64,
    end: u64,
    tag: &'a ParameterTag,
    address: S7Address,
    /// Whether an overlap that holds this parameter is reported. [`occupy`] sets `true`.
    reported: bool,
}

impl Draft {
    /// Check the S7 rules for the whole draft after a command wrote the parameters `written`
    /// (I5, I20, ADR-0024, ADR-0149 (a)).
    ///
    /// The method checks each parameter in tag order for the platform rule, the width, and the
    /// alignment. The platform rule needs no type tree, so it covers every parameter. A parameter
    /// whose type tree failed to resolve skips the other rules, because the snapshot check reports
    /// it. Then the method finds the first overlap in each data block. It reports only an
    /// overlap of a pair that holds a parameter of `written`. A stored overlap of two other
    /// parameters does not refuse the command (ADR-0149 (a), (d)). An empty `written` reports no
    /// overlap.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform, or when they
    /// do not hold the type of a parameter. Returns `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, or `E-RCP-023` for the first parameter
    /// that breaks a rule. Otherwise returns `E-RCP-022` for the first overlap that holds a
    /// parameter of `written`. Returns a `PlantError` when the layout of a type overflows.
    pub(super) fn check_s7_layout(
        &self,
        trees: &TypeTrees,
        written: &[ParameterKey],
    ) -> Result<(), DraftError> {
        self.check_layout(trees, AddressRules::Skip, OverlapScope::Written(written))
    }

    /// Check the S7 rules for approval: the rules of [`Draft::check_s7_layout`] and the data
    /// block rule and end rule of ADR-0116 for every parameter.
    ///
    /// Each parameter gets its checks in the order of ADR-0116 (f). The method does not run the
    /// rules for a command that keeps a stored address (ADR-0116 (e1)). This keeps a rule change
    /// from locking a draft. The method reports every overlap of the draft, also a stored overlap
    /// of two parameters that no command wrote (ADR-0149 (c)).
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Draft::check_s7_layout`], with `E-RCP-022` for the first overlap
    /// of the draft. Returns `E-RCP-057` for the first
    /// parameter whose data block number is too high for the platform. Returns `E-RCP-056` for the
    /// first parameter that ends after byte 65535 of its data block.
    pub(crate) fn check_s7_layout_for_approval(&self, trees: &TypeTrees) -> Result<(), DraftError> {
        self.check_layout(trees, AddressRules::Run, OverlapScope::Every)
    }

    /// Check the S7 rules of every parameter and then the overlaps that `scope` reports.
    fn check_layout(
        &self,
        trees: &TypeTrees,
        rules: AddressRules,
        scope: OverlapScope<'_>,
    ) -> Result<(), DraftError> {
        let parameters = self.parameters();
        let mut occupied = Vec::new();
        for parameter in &parameters {
            check_platform_rule(self.platform(), parameter.tag(), parameter.s7_address())?;
            if rules == AddressRules::Run {
                check_data_block_rule(self.platform(), parameter.tag(), parameter.s7_address())?;
            }
            let Some(tree) = self.held_tree(trees, parameter.plc_type())? else {
                continue;
            };
            check_tree_rules(parameter.tag(), parameter.s7_address(), tree)?;
            if rules == AddressRules::Run {
                check_end_rule(parameter.tag(), parameter.s7_address(), tree)?;
            }
            if let Some(address) = parameter.s7_address() {
                occupied.push(Occupied {
                    reported: scope.reports(parameter.key()),
                    ..occupy(parameter.tag(), parameter.plc_type(), address, tree)?
                });
            }
        }
        find_overlap(occupied)
    }
}

/// Check the platform rule, the data block rule, the width, the alignment, and the end rule of
/// one parameter.
///
/// The function does not look at the other parameters of the draft. A command calls it for a new
/// parameter before it checks the whole draft.
pub(crate) fn check_definition(
    platform: PlcPlatform,
    tag: &ParameterTag,
    address: Option<S7Address>,
    tree: &TypeTree,
) -> Result<(), DraftError> {
    check_platform_rule(platform, tag, address)?;
    check_data_block_rule(platform, tag, address)?;
    check_tree_rules(tag, address, tree)?;
    check_end_rule(tag, address, tree)
}

/// Check the end rule of ADR-0116 (a). The rule runs after the alignment rule.
///
/// The offset plus the layout size is at most 65 536 bytes. An address with a bit number uses one
/// bit, so it always fits. An overflow of the sum counts as a failure of the rule.
pub(in crate::recipe) fn check_end_rule(
    tag: &ParameterTag,
    address: Option<S7Address>,
    tree: &TypeTree,
) -> Result<(), DraftError> {
    let Some(address) = address else {
        return Ok(());
    };
    if address.bit().is_some() {
        return Ok(());
    }
    let size = S7Layout::of(tree)?.size_bytes();
    let fits = u32::from(address.offset())
        .checked_add(size)
        .is_some_and(|end| end <= S7_DATA_BLOCK_BYTES);
    if fits {
        return Ok(());
    }
    Err(RecipeError::S7AddressPastBlockEnd {
        tag: tag.clone(),
        address,
        size,
    }
    .into())
}

/// Check the data block rule of ADR-0116 (c) and ADR-0140 (a). The rule needs no type tree.
///
/// The data block number is at most 16000 on `S7_300` and at most 59999 on `S7_1200` and
/// `S7_1500`. `CodesysIec` and `TwinCat3` have no S7 address, so the rule sets no limit there (I5).
pub(in crate::recipe) fn check_data_block_rule(
    platform: PlcPlatform,
    tag: &ParameterTag,
    address: Option<S7Address>,
) -> Result<(), RecipeError> {
    let (Some(address), Some(max)) = (address, max_data_block(platform)) else {
        return Ok(());
    };
    if address.db() <= max {
        return Ok(());
    }
    Err(RecipeError::S7DataBlockOutOfRange {
        tag: tag.clone(),
        address,
        max,
    })
}

/// The highest data block number of a platform, or `None` when the platform keeps the grammar.
const fn max_data_block(platform: PlcPlatform) -> Option<u16> {
    match platform {
        PlcPlatform::S7_300 => Some(S7_300_MAX_DATA_BLOCK),
        PlcPlatform::S7_1200 | PlcPlatform::S7_1500 => Some(S7_1X00_MAX_DATA_BLOCK),
        PlcPlatform::CodesysIec | PlcPlatform::TwinCat3 => None,
    }
}

/// Check the platform rule I5. The rule needs no type tree.
fn check_platform_rule(
    platform: PlcPlatform,
    tag: &ParameterTag,
    address: Option<S7Address>,
) -> Result<(), RecipeError> {
    match (address, platform) {
        (None, PlcPlatform::S7_300) => Err(RecipeError::S7AddressRequired { tag: tag.clone() }),
        (Some(_), PlcPlatform::CodesysIec | PlcPlatform::TwinCat3) => {
            Err(RecipeError::S7AddressNotAllowed {
                tag: tag.clone(),
                platform,
            })
        }
        _ => Ok(()),
    }
}

/// Check the width and the alignment of the address of one parameter.
fn check_tree_rules(
    tag: &ParameterTag,
    address: Option<S7Address>,
    tree: &TypeTree,
) -> Result<(), DraftError> {
    let Some(address) = address else {
        return Ok(());
    };
    let expected = S7Width::expected_for(tree);
    if expected != address.width() {
        return Err(RecipeError::S7AddressWidthMismatch {
            tag: tag.clone(),
            address,
            width: expected,
        }
        .into());
    }
    let layout = S7Layout::of(tree)?;
    if layout.start() == S7Start::Even && !address.offset().is_multiple_of(2) {
        return Err(RecipeError::S7AddressMisaligned {
            tag: tag.clone(),
            address,
        }
        .into());
    }
    Ok(())
}

/// Compute the bits that a parameter occupies. A `BOOL` occupies one bit.
fn occupy<'a>(
    tag: &'a ParameterTag,
    plc_type: PlcTypeId,
    address: S7Address,
    tree: &TypeTree,
) -> Result<Occupied<'a>, PlantError> {
    let bits = match address.bit() {
        Some(_) => Some(1),
        None => u64::from(S7Layout::of(tree)?.size_bytes()).checked_mul(BITS_PER_BYTE),
    };
    let start = u64::from(address.offset())
        .checked_mul(BITS_PER_BYTE)
        .and_then(|first_bit| first_bit.checked_add(u64::from(address.bit().unwrap_or(0))));
    let end = start
        .zip(bits)
        .and_then(|(first_bit, width)| first_bit.checked_add(width));
    let (Some(start), Some(end)) = (start, end) else {
        return Err(PlantError::TypeTooLarge {
            type_name: plc_type.to_string(),
            limit: TypeLimit::Leaves,
        });
    };
    Ok(Occupied {
        db: address.db(),
        start,
        end,
        tag,
        address,
        reported: true,
    })
}

/// Find every parameter whose bytes overlap the bytes of another parameter of the same data block
/// (ADR-0149 (e), design D10).
///
/// The function solves one problem: the export check must report each parameter of an overlap and
/// not only the first pair. It skips a parameter without an S7 address. It reuses [`occupy`] and
/// the sweep of [`find_overlap`].
///
/// The result maps the key of each overlapping parameter to the tag of one parameter that shares
/// bytes with it.
///
/// # Errors
///
/// Returns the key of the parameter and the [`PlantError`] when the occupied bits of the
/// parameter overflow. The error is in a `Box`, because a [`PlantError`] is large.
pub(crate) fn overlap_partners<'a>(
    entries: impl IntoIterator<Item = (&'a Parameter, &'a TypeTree)>,
) -> Result<BTreeMap<ParameterKey, ParameterTag>, (ParameterKey, Box<PlantError>)> {
    let mut items = Vec::new();
    for (parameter, tree) in entries {
        let Some(address) = parameter.s7_address() else {
            continue;
        };
        let occupied = occupy(parameter.tag(), parameter.plc_type(), address, tree)
            .map_err(|error| (parameter.key(), Box::new(error)))?;
        items.push((occupied, parameter.key()));
    }
    items.sort_by(|(a, _), (b, _)| {
        (a.db, a.start, a.end, a.tag).cmp(&(b.db, b.start, b.end, b.tag))
    });
    let mut partners = BTreeMap::new();
    let mut reach: Option<&(Occupied<'_>, ParameterKey)> = None;
    for entry in &items {
        let (item, key) = entry;
        let earlier = reach.filter(|(earlier, _)| earlier.db == item.db);
        if let Some((earlier, earlier_key)) = earlier
            && item.start < earlier.end
        {
            partners.entry(*key).or_insert_with(|| earlier.tag.clone());
            partners
                .entry(*earlier_key)
                .or_insert_with(|| item.tag.clone());
        }
        reach = earlier
            .filter(|(earlier, _)| earlier.end > item.end)
            .or(Some(entry));
    }
    Ok(partners)
}

/// Find the first overlap of a pair that holds a reported parameter.
///
/// A parameter is reported when its flag `reported` is `true`. The method sweeps the sorted
/// intervals of each data block. The error names the later parameter of the sort order as `tag` and
/// the earlier parameter as `other_tag`. The earlier parameter is the one with the largest end so
/// far. The sweep skips a pair without a reported parameter. Then it continues with the item of
/// the largest end. The sweep therefore finds a pair with a reported parameter whenever one
/// exists.
fn find_overlap(mut occupied: Vec<Occupied<'_>>) -> Result<(), DraftError> {
    occupied.sort_by(|a, b| (a.db, a.start, a.end, a.tag).cmp(&(b.db, b.start, b.end, b.tag)));
    let mut reach: Option<&Occupied<'_>> = None;
    for item in &occupied {
        let earlier = reach.filter(|earlier| earlier.db == item.db);
        if let Some(earlier) = earlier
            && item.start < earlier.end
            && (item.reported || earlier.reported)
        {
            return Err(RecipeError::S7AddressOverlap {
                tag: item.tag.clone(),
                other_tag: earlier.tag.clone(),
                address: item.address,
            }
            .into());
        }
        reach = earlier
            .filter(|earlier| earlier.end > item.end)
            .or(Some(item));
    }
    Ok(())
}

#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod prop;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod written_overlap_tests;
