//! The target check of a rebind and the selection of the affected parameters (design D13).

use std::collections::BTreeMap;

use super::map::map_parameters;
use super::{RebindPlan, RebindRequest, RebindTargetReason};
use crate::plant::{PlcTypeId, TypeName, TypeNode, TypeTree, TypeTrees};
use crate::recipe::{Draft, DraftError, RecipeError};

/// The struct at the root of a resolved tree.
struct StructRoot<'t> {
    tree: &'t TypeTree,
    name: &'t TypeName,
    revision: u32,
}

/// Return the tree of `id` when it resolved and its root is a struct.
fn struct_root(trees: &TypeTrees, id: PlcTypeId) -> Option<StructRoot<'_>> {
    let tree = trees.tree(id)?;
    match tree.root() {
        TypeNode::Struct { name, revision, .. } => Some(StructRoot {
            tree,
            name,
            revision: *revision,
        }),
        TypeNode::Leaf { .. } | TypeNode::Array { .. } => None,
    }
}

impl Draft {
    /// Check the target of a rebind and list the parameters that it moves.
    ///
    /// The method changes nothing. The checks run in this order. The trees must belong to the
    /// machine and the platform of the draft. The source and the target must be structs. The
    /// target must be a higher revision of the same struct name. At least one parameter must
    /// use the source. A parameter uses the source when its type is the source struct, or an
    /// array whose element chain ends in the source struct (ADR-0018). A parameter whose type
    /// has no tree in `trees` does not use the source.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform. Returns
    /// `E-RCP-052` with the reason of the first failed check. Returns `E-PLT-018` when a
    /// planned tree is above a size limit.
    pub fn plan_rebind(
        &self,
        request: &RebindRequest,
        trees: &TypeTrees,
    ) -> Result<RebindPlan, DraftError> {
        self.check_trees_belong(trees, request.from)?;
        let invalid = |reason| {
            DraftError::from(RecipeError::RebindTargetInvalid {
                type_name: trees.label(request.from),
                reason,
            })
        };
        let source = struct_root(trees, request.from)
            .ok_or_else(|| invalid(RebindTargetReason::SourceNotStruct))?;
        if trees.get(request.to).is_none() {
            return Err(invalid(RebindTargetReason::TargetNotFound));
        }
        let target = struct_root(trees, request.to)
            .ok_or_else(|| invalid(RebindTargetReason::TargetNotStruct))?;
        if source.name != target.name {
            return Err(invalid(RebindTargetReason::OtherStruct));
        }
        if target.revision <= source.revision {
            return Err(invalid(RebindTargetReason::NotNewer));
        }
        let mut planned = BTreeMap::new();
        let mut old_types = BTreeMap::new();
        for parameter in self.parameters() {
            let Some(tree) = trees.tree(parameter.plc_type()) else {
                continue;
            };
            if let Some(rebound) = tree.rebind(request.from, target.tree)? {
                planned.insert(parameter.key(), rebound);
                old_types.insert(parameter.key(), parameter.plc_type());
            }
        }
        if planned.is_empty() {
            return Err(invalid(RebindTargetReason::NotUsed));
        }
        let mapped = map_parameters(self, request, trees, &planned)?;
        Ok(RebindPlan::new(
            request.from,
            request.to,
            planned,
            old_types,
            mapped,
        ))
    }
}

#[cfg(test)]
mod tests;
