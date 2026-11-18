//! The limits of a parameter after a rebind (design D13, step 6, ADR-0018).

use super::level::{kept_members, struct_level};
use super::map::MapFault;
use crate::plant::{LeafSegment, RuleSet, TypeTree};
use crate::recipe::draft::{check_faults, stored_limits_under};
use crate::recipe::{ParameterTag, ParameterValue, RecipeError, StoredLimit};

/// The inputs that map the limits of one parameter.
pub(super) struct LimitMapping<'a> {
    pub(super) tag: &'a ParameterTag,
    pub(super) old_tree: &'a TypeTree,
    pub(super) new_tree: &'a TypeTree,
    pub(super) rows: &'a [StoredLimit],
}

/// Map the stored limits of a parameter to the planned tree.
///
/// The function first checks every stored row against the old tree. The member segment of a
/// limit path follows the index segments of the array chain. A limit in a kept member stays as it
/// is. A limit in a removed member or in a member of another type shape is an entry. A limit
/// never disappears without a decision (ADR-0018). The kept rows then pass the check against the
/// planned tree. A kept member has the same type shape in both trees, so a row that passes the old
/// tree also passes the planned tree. This second check guards only against a bug in the mapping,
/// and it gives the defect of the first kept row that it rejects. The rows are stored data
/// that the rebind does not change, so the checks use the earlier rule set (ADR-0120 (d)).
///
/// # Errors
///
/// Returns the defect `E-RCP-*` of the first stored row that the old tree rejects. A row on a
/// path that the old tree does not have is such a defect. Otherwise it returns the defect of the
/// first kept row that the planned tree rejects. Otherwise it returns `E-RCP-029` as an entry
/// for each dropped limit, in the order of the rows.
pub(super) fn map_limits(input: &LimitMapping<'_>) -> Result<Vec<StoredLimit>, MapFault> {
    stored_limits_under(input.tag, input.rows, input.old_tree, RuleSet::Earlier)
        .map_err(MapFault::Defect)?;
    let (entries, kept_rows) = split_rows(input);
    stored_limits_under(input.tag, &kept_rows, input.new_tree, RuleSet::Earlier)
        .map_err(MapFault::Defect)?;
    if entries.is_empty() {
        Ok(kept_rows)
    } else {
        Err(MapFault::Entries(entries))
    }
}

/// Check the kept value against the kept limits and return the first fault (invariant I7).
///
/// The check reads the stored leaves at the kept paths. A kept member has the same leaves and the
/// same limit paths in the old and the planned tree. The check therefore uses the old tree. It runs
/// when the request has entries too, because a new leaf has no stored value (ADR-0195 (l), (n)).
/// It returns the first fault in the order of `ParameterValue::check_all_under`. It returns
/// `None` at once when the mapping keeps no row. The caller passes the old value that it already
/// parsed.
pub(super) fn kept_limit_defect(
    input: &LimitMapping<'_>,
    value: &ParameterValue,
) -> Option<RecipeError> {
    let (_, kept_rows) = split_rows(input);
    if kept_rows.is_empty() {
        return None;
    }
    let limits = match stored_limits_under(input.tag, &kept_rows, input.old_tree, RuleSet::Earlier)
    {
        Ok(limits) => limits,
        Err(error) => return Some(error),
    };
    check_faults(
        input.tag,
        value,
        &limits,
        (input.old_tree, RuleSet::Earlier),
    )
    .into_iter()
    .next()
}

/// Split the rows into the entries of the dropped limits and the kept rows.
fn split_rows(input: &LimitMapping<'_>) -> (Vec<RecipeError>, Vec<StoredLimit>) {
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
    (entries, kept_rows)
}

#[cfg(test)]
mod tests;
