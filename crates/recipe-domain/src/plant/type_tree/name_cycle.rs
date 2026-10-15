//! Check of the struct names of a type tree (ADR-0038).

use super::{PlantError, TypeName, TypeNode, TypeTree};

/// One step of the walk. `Leave` closes the struct that the walk entered before.
enum Step<'a> {
    Enter(&'a TypeNode),
    Leave,
}

impl TypeTree {
    /// Reject a tree in which one struct name occurs twice on one path from the root.
    ///
    /// Two revisions of one struct are the same PLC type, so a struct that holds an earlier
    /// revision of itself contains itself (invariant I8, ADR-0038). The same struct on two
    /// sibling paths is not a cycle. `resolve` does not run this check, so a read of a stored
    /// row never fails because of it. The walk uses an explicit stack and never recurses.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-005` with the repeated struct name.
    pub fn check_struct_names(&self) -> Result<(), PlantError> {
        let mut steps = vec![Step::Enter(self.root())];
        let mut path: Vec<&TypeName> = Vec::new();
        while let Some(step) = steps.pop() {
            let node = match step {
                Step::Enter(node) => node,
                Step::Leave => {
                    path.pop();
                    continue;
                }
            };
            match node {
                TypeNode::Leaf { .. } => {}
                TypeNode::Array { element, .. } => steps.push(Step::Enter(element)),
                TypeNode::Struct { name, members, .. } => {
                    if path.contains(&name) {
                        return Err(PlantError::TypeCycle {
                            type_name: name.clone(),
                        });
                    }
                    path.push(name);
                    steps.push(Step::Leave);
                    steps.extend(members.iter().rev().map(|(_, child)| Step::Enter(child)));
                }
            }
        }
        Ok(())
    }
}
