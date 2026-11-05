//! The resolved trees of the types that a draft uses (`domain-model.md` section 2.1).

use std::collections::BTreeMap;

use super::{
    MachineId, PlantError, PlatformTypes, PlcPlatform, PlcTypeId, RecordShape, TypeName, TypeNode,
    TypeTree,
};

/// A struct node whose revision is lower than the newest revision of its name (ADR-0018).
///
/// The fields are the data of `E-RCP-024`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outdated {
    type_name: TypeName,
    current: u32,
    newest: u32,
}

impl Outdated {
    /// Return the name of the struct.
    #[must_use]
    pub const fn type_name(&self) -> &TypeName {
        &self.type_name
    }

    /// Return the revision that the tree uses.
    #[must_use]
    pub const fn current(&self) -> u32 {
        self.current
    }

    /// Return the newest revision of the struct name.
    #[must_use]
    pub const fn newest(&self) -> u32 {
        self.newest
    }
}

/// The resolved trees of many type identifiers on one machine.
///
/// The type solves one problem: a use case resolves each type once and then asks many
/// questions without a second walk of the rows. A failed resolution keeps its `PlantError`.
#[derive(Debug)]
pub struct TypeTrees {
    machine: MachineId,
    platform: PlcPlatform,
    trees: BTreeMap<PlcTypeId, Result<TypeTree, PlantError>>,
    newest: BTreeMap<TypeName, u32>,
    labels: BTreeMap<PlcTypeId, String>,
}

impl TypeTrees {
    /// Resolve each distinct identifier against `types`.
    ///
    /// The result keeps the tree or the error of every identifier. It also keeps the newest
    /// revision of each struct name among the kept rows.
    #[must_use]
    pub fn resolve(ids: impl IntoIterator<Item = PlcTypeId>, types: &PlatformTypes) -> Self {
        let mut trees = BTreeMap::new();
        let mut labels = BTreeMap::new();
        for id in ids {
            if trees.contains_key(&id) {
                continue;
            }
            trees.insert(id, TypeTree::resolve(id, types));
            if let Some(label) = types.label(id) {
                labels.insert(id, label);
            }
        }
        let mut newest: BTreeMap<TypeName, u32> = BTreeMap::new();
        for row in types.rows() {
            if let RecordShape::Struct { name, revision, .. } = row.shape() {
                let top = newest.entry(name.clone()).or_insert(*revision);
                *top = (*top).max(*revision);
            }
        }
        Self {
            machine: types.machine(),
            platform: types.platform(),
            trees,
            newest,
            labels,
        }
    }

    /// Return the machine of the types.
    #[must_use]
    pub const fn machine(&self) -> MachineId {
        self.machine
    }

    /// Return the platform of the types.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.platform
    }

    /// Return the result of the resolution, or `None` when the identifier was not asked.
    #[must_use]
    pub fn get(&self, id: PlcTypeId) -> Option<&Result<TypeTree, PlantError>> {
        self.trees.get(&id)
    }

    /// Return the tree, or `None` when the identifier failed or was not asked.
    #[must_use]
    pub fn tree(&self, id: PlcTypeId) -> Option<&TypeTree> {
        self.trees.get(&id).and_then(|result| result.as_ref().ok())
    }

    /// Return the label of the type, or the identifier text when no row is known.
    #[must_use]
    pub fn label(&self, id: PlcTypeId) -> String {
        self.labels
            .get(&id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    /// Return the first struct node with a newer revision, or `None`.
    ///
    /// The walk is depth first in declaration order and starts at the root. It uses an explicit
    /// stack. An identifier without a tree gives `None`.
    #[must_use]
    pub fn outdated(&self, id: PlcTypeId) -> Option<Outdated> {
        let mut stack = vec![self.tree(id)?.root()];
        while let Some(node) = stack.pop() {
            match node {
                TypeNode::Leaf { .. } => {}
                TypeNode::Array { element, .. } => stack.push(element.as_ref()),
                TypeNode::Struct {
                    name,
                    revision,
                    members,
                    ..
                } => {
                    let newest = self.newest.get(name).copied().unwrap_or(*revision);
                    if *revision < newest {
                        return Some(Outdated {
                            type_name: name.clone(),
                            current: *revision,
                            newest,
                        });
                    }
                    stack.extend(members.iter().rev().map(|(_, child)| child));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
