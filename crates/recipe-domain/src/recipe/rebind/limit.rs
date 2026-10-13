//! The limits of a parameter after a rebind (design D13, step 6, ADR-0018).

use super::level::{kept_members, struct_level};
use crate::plant::{LeafSegment, TypeTree};
use crate::recipe::draft::stored_limits;
use crate::recipe::{ParameterTag, RecipeError, StoredLimit};

/// The inputs that map the limits of one parameter.
pub(super) struct LimitMapping<'a> {
    pub(super) tag: &'a ParameterTag,
    pub(super) old_tree: &'a TypeTree,
    pub(super) new_tree: &'a TypeTree,
    pub(super) rows: &'a [StoredLimit],
}

/// Map the stored limits of a parameter to the planned tree.
///
/// The member segment of a limit path follows the index segments of the array chain. A limit in
/// a kept member stays as it is. A limit in a removed member or in a member with a changed type
/// is an entry. A limit never disappears without a decision (ADR-0018). The kept limits
/// must pass the check against the planned tree.
///
/// # Errors
///
/// Returns `E-RCP-029` for each dropped limit, in the order of the rows. Then it returns the
/// error of the first kept limit that the planned tree rejects.
pub(super) fn map_limits(input: &LimitMapping<'_>) -> Result<Vec<StoredLimit>, Vec<RecipeError>> {
    let mut entries = Vec::new();
    let mut kept_rows = Vec::new();
    match (struct_level(input.old_tree), struct_level(input.new_tree)) {
        (Some(source), Some(target)) => {
            let kept = kept_members(&source, &target);
            let depth = source.chain.len();
            for row in input.rows {
                match row.leaf_path.segments().get(depth) {
                    Some(LeafSegment::Member(name)) if !kept.contains(name) => {
                        entries.push(RecipeError::RebindDropsLimit {
                            tag: input.tag.clone(),
                            leaf: row.leaf_path.clone(),
                        });
                    }
                    _ => kept_rows.push(row.clone()),
                }
            }
        }
        _ => kept_rows.extend_from_slice(input.rows),
    }
    match stored_limits(input.tag, &kept_rows, input.new_tree) {
        Ok(_) if entries.is_empty() => Ok(kept_rows),
        Ok(_) => Err(entries),
        Err(error) => {
            entries.push(error);
            Err(entries)
        }
    }
}

#[cfg(test)]
mod tests;
