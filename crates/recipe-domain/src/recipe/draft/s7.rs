//! S7 rules of a draft (design D10, invariants I5 and I20, ADR-0024).

use crate::plant::{
    PlantError, PlcPlatform, PlcTypeId, S7Address, S7Layout, S7Start, S7Width, TypeLimit, TypeTree,
    TypeTrees,
};
use crate::recipe::{Draft, DraftError, ParameterTag, RecipeError};

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

/// Whether a whole-draft check runs the address rules of ADR-0116.
///
/// A check for approval runs them. A check after `replace_parameters` skips them, so a stored
/// address of another parameter does not refuse a command (ADR-0116 (e1)).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AddressRules {
    Run,
    Skip,
}

/// The bits that one parameter occupies in its data block. The end is exclusive.
struct Occupied<'a> {
    db: u16,
    start: u64,
    end: u64,
    tag: &'a ParameterTag,
    address: S7Address,
}

impl Draft {
    /// Check the S7 rules for the whole draft (I5, I20, ADR-0024).
    ///
    /// The method checks each parameter in tag order for the platform rule, the width, and the
    /// alignment. The platform rule needs no type tree, so it covers every parameter. A parameter
    /// whose type tree failed to resolve skips the other rules, because the snapshot check reports
    /// it. Then the method looks for the first overlap in each data block.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform, or when they
    /// do not hold the type of a parameter. Returns `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, or `E-RCP-023` for the first parameter
    /// that breaks a rule. Otherwise returns `E-RCP-022` for the first overlap.
    /// Returns a `PlantError` when the layout of a type overflows.
    pub fn check_s7_layout(&self, trees: &TypeTrees) -> Result<(), DraftError> {
        self.check_layout(trees, AddressRules::Skip)
    }

    /// Check the S7 rules for approval: the rules of [`Draft::check_s7_layout`] and the data
    /// block rule and end rule of ADR-0116 for every parameter.
    ///
    /// Each parameter gets its checks in the order of ADR-0116 (f). The method does not run the
    /// rules for a command that keeps a stored address (ADR-0116 (e1)). This keeps a rule change
    /// from locking a draft.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Draft::check_s7_layout`]. Returns `E-RCP-057` for the first
    /// parameter whose data block number is too high for the platform. Returns `E-RCP-056` for the
    /// first parameter that ends after byte 65535 of its data block.
    pub(crate) fn check_s7_layout_for_approval(&self, trees: &TypeTrees) -> Result<(), DraftError> {
        self.check_layout(trees, AddressRules::Run)
    }

    /// Check the S7 rules of every parameter and then the overlaps of the whole draft.
    fn check_layout(&self, trees: &TypeTrees, rules: AddressRules) -> Result<(), DraftError> {
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
                occupied.push(occupy(
                    parameter.tag(),
                    parameter.plc_type(),
                    address,
                    tree,
                )?);
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

/// Check the data block rule of ADR-0116 (c). The rule needs no type tree.
///
/// On `S7_300`, the data block number is at most 16000. The other platforms keep `1..=65535`.
pub(in crate::recipe) fn check_data_block_rule(
    platform: PlcPlatform,
    tag: &ParameterTag,
    address: Option<S7Address>,
) -> Result<(), RecipeError> {
    match address {
        Some(address)
            if platform == PlcPlatform::S7_300 && address.db() > S7_300_MAX_DATA_BLOCK =>
        {
            Err(RecipeError::S7DataBlockOutOfRange {
                tag: tag.clone(),
                address,
            })
        }
        _ => Ok(()),
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
    })
}

/// Find the first overlap with a sweep over the sorted intervals of each data block.
///
/// The later parameter in the sort order is the reported one. The earlier parameter is the one
/// with the largest end so far.
fn find_overlap(mut occupied: Vec<Occupied<'_>>) -> Result<(), DraftError> {
    occupied.sort_by(|a, b| (a.db, a.start, a.end, a.tag).cmp(&(b.db, b.start, b.end, b.tag)));
    let mut reach: Option<&Occupied<'_>> = None;
    for item in &occupied {
        if let Some(earlier) = reach.filter(|earlier| earlier.db == item.db)
            && item.start < earlier.end
        {
            return Err(RecipeError::S7AddressOverlap {
                tag: item.tag.clone(),
                other_tag: earlier.tag.clone(),
                address: item.address,
            }
            .into());
        }
        reach = Some(item);
    }
    Ok(())
}

#[cfg(test)]
mod prop;
#[cfg(test)]
mod tests;
