//! Resolved type trees of the Plant Model (`domain-model.md` section 2.1).
//!
//! A tree is the fully expanded shape of one PLC type. `resolve` builds it
//! from the rows of one platform and never recurses (`reliability.md` section 2.2).

use std::collections::{BTreeMap, BTreeSet};

use super::limits::{MAX_DEPTH, MAX_LEAVES};
use super::{
    ArrayDimensions, DerivedTypeReason, LeafClass, MachineId, MemberName, PlantError, PlcPlatform,
    PlcType, PlcTypeId, Primitive, RecordShape, TypeLimit, TypeName,
};

/// The type rows that one machine can use.
///
/// A row of another platform or of another machine is absent, so a reference
/// to it resolves as "not on the platform" (invariant I6).
#[derive(Clone, Debug)]
pub struct PlatformTypes {
    machine: MachineId,
    platform: PlcPlatform,
    rows: BTreeMap<PlcTypeId, PlcType>,
}

impl PlatformTypes {
    /// Keep the rows that `machine` can use on `platform`.
    ///
    /// A row is kept when its platform set contains `platform` and it is global or belongs to
    /// `machine`.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` with the reason `RowShape` when two kept rows share one identifier.
    pub fn collect(
        machine: MachineId,
        platform: PlcPlatform,
        rows: impl IntoIterator<Item = PlcType>,
    ) -> Result<Self, PlantError> {
        let mut kept = BTreeMap::new();
        for row in rows {
            let is_visible = row.platforms().contains(&platform)
                && row.machine().is_none_or(|owner| owner == machine);
            if !is_visible {
                continue;
            }
            let id = row.id();
            if kept.insert(id, row).is_some() {
                return Err(row_shape(id));
            }
        }
        Ok(Self {
            machine,
            platform,
            rows: kept,
        })
    }

    /// Return the machine of these rows.
    #[must_use]
    pub const fn machine(&self) -> MachineId {
        self.machine
    }

    /// Return the platform of these rows.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.platform
    }

    /// Return the row with this identifier, if it exists on the platform.
    #[must_use]
    pub fn get(&self, id: PlcTypeId) -> Option<&PlcType> {
        self.rows.get(&id)
    }
}

/// One node of a resolved type tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeNode {
    /// A scalar leaf.
    Leaf {
        /// Row of the leaf type.
        type_id: PlcTypeId,
        /// Primitive of the leaf.
        primitive: Primitive,
        /// Leaf class of the primitive.
        leaf_class: LeafClass,
        /// Size of the leaf in bytes.
        size_bytes: u32,
        /// Length of a string leaf.
        string_length: Option<u16>,
    },
    /// An array of one element type.
    Array {
        /// Row of the array type.
        type_id: PlcTypeId,
        /// Dimensions of the array.
        dimensions: ArrayDimensions,
        /// Tree of one element.
        element: Box<TypeNode>,
    },
    /// A struct with ordered members.
    Struct {
        /// Row of the struct type.
        type_id: PlcTypeId,
        /// Name of the struct.
        name: TypeName,
        /// Revision of the struct.
        revision: u32,
        /// Members in declaration order.
        members: Vec<(MemberName, TypeNode)>,
    },
}

/// The resolved tree of one root type with its leaf count and depth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeTree {
    root: TypeNode,
    leaf_count: u32,
    depth: u32,
}

impl TypeTree {
    /// Return the root node.
    #[must_use]
    pub const fn root(&self) -> &TypeNode {
        &self.root
    }

    /// Return the number of scalar leaves.
    #[must_use]
    pub const fn leaf_count(&self) -> u32 {
        self.leaf_count
    }

    /// Return the number of JSON levels of a value of this type.
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth
    }

    /// Resolve the tree of `root` against the rows of one platform.
    ///
    /// The function uses an explicit stack and never recurses. Pass 1 detects
    /// cycles and counts leaves and depth. Pass 2 builds the nodes.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` for a missing identifier, `E-PLT-005` for a cycle, and
    /// `E-PLT-018` above the leaf or depth limit.
    pub fn resolve(root: PlcTypeId, types: &PlatformTypes) -> Result<Self, PlantError> {
        let (order, metrics) = walk(root, types)?;
        let root_metrics = metrics.get(&root).ok_or_else(|| row_shape(root))?;
        let leaf_count = u32::try_from(root_metrics.leaves)
            .map_err(|_| root_too_large(root, types, TypeLimit::Leaves))?;
        let rows = order
            .iter()
            .map(|id| types.get(*id).ok_or_else(|| not_on_platform(*id)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut built = Built::new(rows.iter().copied())
            .ok_or_else(|| root_too_large(root, types, TypeLimit::Leaves))?;
        for row in rows {
            let node = build_node(row, &mut built)?;
            built.nodes.insert(row.id(), node);
        }
        let root_node = built.nodes.remove(&root).ok_or_else(|| row_shape(root))?;
        Ok(Self {
            root: root_node,
            leaf_count,
            depth: root_metrics.depth,
        })
    }
}

/// Counts of one finished row.
#[derive(Clone, Copy, Debug)]
struct Metrics {
    leaves: u64,
    depth: u32,
}

/// One row on the current path of the walk and the children not yet visited.
struct Frame {
    id: PlcTypeId,
    children: std::vec::IntoIter<PlcTypeId>,
}

fn not_on_platform(id: PlcTypeId) -> PlantError {
    PlantError::DerivedTypeInvalid {
        type_expr: id.to_string(),
        reason: DerivedTypeReason::NotOnPlatform,
    }
}

fn row_shape(id: PlcTypeId) -> PlantError {
    PlantError::DerivedTypeInvalid {
        type_expr: id.to_string(),
        reason: DerivedTypeReason::RowShape,
    }
}

/// Build the error of an arithmetic overflow. `walk` replaces the name.
fn too_large(id: PlcTypeId, limit: TypeLimit) -> PlantError {
    PlantError::TypeTooLarge {
        type_name: id.to_string(),
        limit,
    }
}

/// Render the root type as text, such as `ST_Zone` or `ARRAY[1..8] OF INT`.
///
/// The loop follows only array elements. It stops at the first struct or primitive.
fn render_root(root: PlcTypeId, types: &PlatformTypes) -> String {
    let mut parts = Vec::new();
    let mut current = root;
    for _ in 0..=types.rows.len() {
        match types.get(current).map(PlcType::shape) {
            Some(RecordShape::Array {
                element,
                dimensions,
            }) => {
                parts.push(format!("ARRAY[{dimensions}] OF "));
                current = *element;
            }
            Some(RecordShape::Struct { name, revision, .. }) => {
                parts.push(format!("{name}@{revision}"));
                break;
            }
            Some(RecordShape::Primitive {
                primitive,
                string_length,
                ..
            }) => {
                parts.push(match string_length {
                    Some(length) => format!("{}({length})", primitive.name()),
                    None => primitive.name().to_owned(),
                });
                break;
            }
            None => {
                parts.push(current.to_string());
                break;
            }
        }
    }
    parts.concat()
}

/// Build `E-PLT-018` for the root type.
fn root_too_large(root: PlcTypeId, types: &PlatformTypes, limit: TypeLimit) -> PlantError {
    PlantError::TypeTooLarge {
        type_name: render_root(root, types),
        limit,
    }
}

/// Stop a row that is above a size limit of `reliability.md` section 7.11.
fn check_limits(
    metrics: Metrics,
    root: PlcTypeId,
    types: &PlatformTypes,
) -> Result<(), PlantError> {
    if metrics.leaves > u64::from(MAX_LEAVES) {
        return Err(root_too_large(root, types, TypeLimit::Leaves));
    }
    if metrics.depth > MAX_DEPTH {
        return Err(root_too_large(root, types, TypeLimit::Depth));
    }
    Ok(())
}

fn child_ids(row: &PlcType) -> Vec<PlcTypeId> {
    match row.shape() {
        RecordShape::Primitive { .. } => Vec::new(),
        RecordShape::Array { element, .. } => vec![*element],
        RecordShape::Struct { members, .. } => members.iter().map(|m| m.type_id).collect(),
    }
}

fn enter(
    id: PlcTypeId,
    types: &PlatformTypes,
    on_path: &mut BTreeSet<PlcTypeId>,
    stack: &mut Vec<Frame>,
) -> Result<(), PlantError> {
    let row = types.get(id).ok_or_else(|| not_on_platform(id))?;
    on_path.insert(id);
    stack.push(Frame {
        id,
        children: child_ids(row).into_iter(),
    });
    Ok(())
}

/// Name the first struct on the cycle that `child` closes.
fn cycle_error(child: PlcTypeId, stack: &[Frame], types: &PlatformTypes) -> PlantError {
    let name = stack
        .iter()
        .skip_while(|frame| frame.id != child)
        .find_map(|frame| match types.get(frame.id).map(PlcType::shape) {
            Some(RecordShape::Struct { name, .. }) => Some(name.clone()),
            _ => None,
        });
    match name {
        Some(type_name) => PlantError::TypeCycle { type_name },
        None => row_shape(child),
    }
}

/// Count the leaves and the depth of a row whose children are finished.
fn measure(row: &PlcType, done: &BTreeMap<PlcTypeId, Metrics>) -> Result<Metrics, PlantError> {
    let id = row.id();
    let child = |child_id: &PlcTypeId| done.get(child_id).copied().ok_or_else(|| row_shape(id));
    match row.shape() {
        RecordShape::Primitive { .. } => Ok(Metrics {
            leaves: 1,
            depth: 0,
        }),
        RecordShape::Array {
            element,
            dimensions,
        } => {
            let element = child(element)?;
            let leaves = dimensions
                .element_count()
                .and_then(|count| element.leaves.checked_mul(count))
                .ok_or_else(|| too_large(id, TypeLimit::Leaves))?;
            let depth = u32::try_from(dimensions.len())
                .ok()
                .and_then(|levels| element.depth.checked_add(levels))
                .ok_or_else(|| too_large(id, TypeLimit::Depth))?;
            Ok(Metrics { leaves, depth })
        }
        RecordShape::Struct { members, .. } => {
            let mut leaves = 0_u64;
            let mut deepest = 0_u32;
            for member in members {
                let metrics = child(&member.type_id)?;
                leaves = leaves
                    .checked_add(metrics.leaves)
                    .ok_or_else(|| too_large(id, TypeLimit::Leaves))?;
                deepest = deepest.max(metrics.depth);
            }
            let depth = deepest
                .checked_add(1)
                .ok_or_else(|| too_large(id, TypeLimit::Depth))?;
            Ok(Metrics { leaves, depth })
        }
    }
}

/// Pass 1: walk depth first, detect cycles, and measure each row.
///
/// Returns the rows in finish order. A child always precedes its parents.
fn walk(
    root: PlcTypeId,
    types: &PlatformTypes,
) -> Result<(Vec<PlcTypeId>, BTreeMap<PlcTypeId, Metrics>), PlantError> {
    let mut on_path = BTreeSet::new();
    let mut stack = Vec::new();
    let mut done = BTreeMap::new();
    let mut order = Vec::new();
    enter(root, types, &mut on_path, &mut stack)?;
    while let Some(frame) = stack.last_mut() {
        if let Some(child) = frame.children.next() {
            if done.contains_key(&child) {
                continue;
            }
            if on_path.contains(&child) {
                return Err(cycle_error(child, &stack, types));
            }
            enter(child, types, &mut on_path, &mut stack)?;
        } else if let Some(finished) = stack.pop() {
            let row = types
                .get(finished.id)
                .ok_or_else(|| not_on_platform(finished.id))?;
            let metrics = match measure(row, &done) {
                Err(PlantError::TypeTooLarge { limit, .. }) => {
                    return Err(root_too_large(root, types, limit));
                }
                other => other?,
            };
            check_limits(metrics, root, types)?;
            on_path.remove(&finished.id);
            done.insert(finished.id, metrics);
            order.push(finished.id);
        }
    }
    Ok((order, done))
}

/// The finished nodes of pass 2 and the number of parent uses that each node still has.
///
/// The last use moves a node out of the map. An earlier use clones it. This design avoids a
/// copy of every shared subtree, which is a cost for a struct that many members share.
struct Built {
    nodes: BTreeMap<PlcTypeId, TypeNode>,
    uses: BTreeMap<PlcTypeId, usize>,
}

impl Built {
    /// Count the parent uses of every child in `rows`, which pass 1 finished.
    ///
    /// Returns `None` when a count overflows. The size limits of pass 1 make this unreachable.
    fn new<'a>(rows: impl IntoIterator<Item = &'a PlcType>) -> Option<Self> {
        let mut uses: BTreeMap<PlcTypeId, usize> = BTreeMap::new();
        for row in rows {
            for child in child_ids(row) {
                let count = uses.entry(child).or_insert(0);
                *count = count.checked_add(1)?;
            }
        }
        Some(Self {
            nodes: BTreeMap::new(),
            uses,
        })
    }

    /// Return the node of `id` for one parent use. The last use moves the node.
    fn take(&mut self, id: PlcTypeId) -> Option<TypeNode> {
        match self.uses.get(&id).and_then(|uses| uses.checked_sub(1)) {
            Some(left) if left > 0 => {
                self.uses.insert(id, left);
                self.nodes.get(&id).cloned()
            }
            _ => {
                self.uses.remove(&id);
                self.nodes.remove(&id)
            }
        }
    }
}

fn build_node(row: &PlcType, built: &mut Built) -> Result<TypeNode, PlantError> {
    let type_id = row.id();
    Ok(match row.shape() {
        RecordShape::Primitive {
            primitive,
            leaf_class,
            size_bytes,
            string_length,
        } => TypeNode::Leaf {
            type_id,
            primitive: *primitive,
            leaf_class: *leaf_class,
            size_bytes: *size_bytes,
            string_length: *string_length,
        },
        RecordShape::Array {
            element,
            dimensions,
        } => TypeNode::Array {
            type_id,
            dimensions: dimensions.clone(),
            element: Box::new(built.take(*element).ok_or_else(|| row_shape(type_id))?),
        },
        RecordShape::Struct {
            name,
            revision,
            members,
            ..
        } => {
            let mut nodes = Vec::with_capacity(members.len());
            for member in members {
                let child = built
                    .take(member.type_id)
                    .ok_or_else(|| row_shape(type_id))?;
                nodes.push((member.name.clone(), child));
            }
            TypeNode::Struct {
                type_id,
                name: name.clone(),
                revision: *revision,
                members: nodes,
            }
        }
    })
}

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod prop;

#[cfg(test)]
pub(crate) mod strategy;

#[cfg(test)]
mod tests {
    use super::fixtures::{
        PLATFORM, array_row, id, int_row_on, machine, other_machine, primitive_row, string_row,
        struct_row, struct_row_of, types,
    };
    use super::*;
    use crate::error_codes::ErrorCode;

    const INT: u128 = 1;
    const REAL: u128 = 2;
    const BOOL: u128 = 3;

    fn seeds() -> Vec<PlcType> {
        vec![
            primitive_row(INT, Primitive::Int),
            primitive_row(REAL, Primitive::Real),
            primitive_row(BOOL, Primitive::Bool),
        ]
    }

    fn with_seeds(mut rows: Vec<PlcType>) -> PlatformTypes {
        let mut all = seeds();
        all.append(&mut rows);
        types(all)
    }

    fn member_names(node: &TypeNode) -> Vec<String> {
        match node {
            TypeNode::Struct { members, .. } => {
                members.iter().map(|(name, _)| name.to_string()).collect()
            }
            _ => Vec::new(),
        }
    }

    #[test]
    fn st_zone_of_data_model_9_1_has_six_leaves_and_depth_two() {
        let rows = vec![
            array_row(10, INT, &[(1, 3)]),
            string_row(11, 16),
            struct_row(
                20,
                "ST_Zone",
                &[
                    ("temp_sp", REAL),
                    ("active", BOOL),
                    ("ramps", 10),
                    ("label", 11),
                ],
            ),
        ];
        let tree = TypeTree::resolve(id(20), &with_seeds(rows)).unwrap();
        assert_eq!(tree.leaf_count(), 6);
        assert_eq!(tree.depth(), 2);
        assert_eq!(
            member_names(tree.root()),
            ["temp_sp", "active", "ramps", "label"]
        );
    }

    #[test]
    fn primitive_root_has_one_leaf_and_depth_zero() {
        let tree = TypeTree::resolve(id(INT), &with_seeds(Vec::new())).unwrap();
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.depth(), 0);
        assert!(matches!(
            tree.root(),
            TypeNode::Leaf {
                primitive: Primitive::Int,
                size_bytes: 2,
                ..
            }
        ));
    }

    #[test]
    fn two_dimensional_array_has_four_leaves_and_depth_two() {
        let rows = vec![array_row(10, INT, &[(0, 1), (1, 2)])];
        let tree = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap();
        assert_eq!(tree.leaf_count(), 4);
        assert_eq!(tree.depth(), 2);
    }

    #[test]
    fn array_of_array_has_six_leaves_and_depth_two() {
        let rows = vec![array_row(10, INT, &[(1, 3)]), array_row(11, 10, &[(1, 2)])];
        let tree = TypeTree::resolve(id(11), &with_seeds(rows)).unwrap();
        assert_eq!(tree.leaf_count(), 6);
        assert_eq!(tree.depth(), 2);
    }

    #[test]
    fn shared_struct_appears_once_per_member() {
        let rows = vec![
            struct_row(10, "ST_Inner", &[("v", INT)]),
            struct_row(11, "ST_Outer", &[("a", 10), ("b", 10)]),
        ];
        let tree = TypeTree::resolve(id(11), &with_seeds(rows)).unwrap();
        assert_eq!(tree.leaf_count(), 2);
        assert_eq!(tree.depth(), 2);
        let members = match tree.root() {
            TypeNode::Struct { members, .. } => members.as_slice(),
            _ => &[],
        };
        assert_eq!(members.len(), 2);
        assert_eq!(members.first().map(|m| &m.1), members.get(1).map(|m| &m.1));
    }

    #[test]
    fn struct_cycle_over_two_structs_gives_type_cycle() {
        let rows = vec![
            struct_row(10, "ST_A", &[("b", 11)]),
            struct_row(11, "ST_B", &[("a", 10)]),
        ];
        let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::PLT_005);
        assert!(matches!(
            &error,
            PlantError::TypeCycle { type_name } if type_name.as_str() == "ST_A"
        ));
    }

    #[test]
    fn cycle_through_an_array_names_the_first_struct_on_the_cycle() {
        let rows = vec![
            struct_row(10, "ST_A", &[("x", INT), ("arr", 11)]),
            array_row(11, 10, &[(1, 2)]),
        ];
        let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
        assert!(matches!(error, PlantError::TypeCycle { .. }));
    }

    #[test]
    fn cycle_of_arrays_only_gives_row_shape() {
        let rows = vec![array_row(10, 11, &[(1, 2)]), array_row(11, 10, &[(1, 2)])];
        let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::PLT_013);
        assert!(matches!(
            error,
            PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::RowShape,
                ..
            }
        ));
    }

    #[test]
    fn missing_member_id_gives_not_on_platform() {
        let rows = vec![struct_row(10, "ST_A", &[("x", INT), ("y", 99)])];
        let error = TypeTree::resolve(id(10), &with_seeds(rows)).unwrap_err();
        assert_eq!(error.code(), ErrorCode::PLT_013);
        let missing = id(99).to_string();
        assert!(matches!(
            &error,
            PlantError::DerivedTypeInvalid { type_expr, reason: DerivedTypeReason::NotOnPlatform }
                if *type_expr == missing
        ));
    }

    #[test]
    fn missing_root_id_gives_not_on_platform() {
        let error = TypeTree::resolve(id(99), &with_seeds(Vec::new())).unwrap_err();
        assert!(matches!(
            error,
            PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::NotOnPlatform,
                ..
            }
        ));
    }

    #[test]
    fn row_of_another_platform_is_absent() {
        let mut rows = seeds();
        rows.push(int_row_on(50, PlcPlatform::S7_1200));
        rows.push(struct_row(10, "ST_A", &[("x", 50)]));
        let collected = types(rows);
        assert_eq!(collected.platform(), PLATFORM);
        assert!(collected.get(id(50)).is_none());
        assert!(collected.get(id(INT)).is_some());
        let error = TypeTree::resolve(id(10), &collected).unwrap_err();
        assert!(matches!(
            error,
            PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::NotOnPlatform,
                ..
            }
        ));
    }

    #[test]
    fn struct_of_another_machine_is_absent() {
        let mut rows = seeds();
        rows.push(struct_row_of(
            other_machine(),
            50,
            "ST_Other",
            &[("x", INT)],
        ));
        rows.push(struct_row(10, "ST_A", &[("o", 50)]));
        let collected = types(rows);
        assert_eq!(collected.machine(), machine());
        assert!(collected.get(id(50)).is_none());
        let error = TypeTree::resolve(id(10), &collected).unwrap_err();
        assert!(matches!(
            error,
            PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::NotOnPlatform,
                ..
            }
        ));
    }

    #[test]
    fn global_primitive_and_own_struct_are_kept() {
        let mut rows = seeds();
        rows.push(struct_row(10, "ST_A", &[("x", INT)]));
        let collected = types(rows);
        assert!(collected.get(id(INT)).is_some());
        assert!(collected.get(id(10)).is_some());
    }

    #[test]
    fn duplicate_identifier_is_a_row_shape_error() {
        let mut rows = seeds();
        rows.push(primitive_row(INT, Primitive::Int));
        let error = PlatformTypes::collect(machine(), PLATFORM, rows).unwrap_err();
        assert_eq!(error.code(), ErrorCode::PLT_013);
        assert!(matches!(
            error,
            PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::RowShape,
                ..
            }
        ));
    }
}
