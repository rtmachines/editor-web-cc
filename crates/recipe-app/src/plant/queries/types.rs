//! The queries `ListTypes`, `GetTypeTree`, and `GetUdtForm` (design D3, D19).
//!
//! The specs are "Type list of a machine", "Type tree of a revision", and "Form of a new revision".

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{
    Machine, MachineCode, MemberName, PlatformTypes, PlcType, PlcTypeId, Primitive, RecordShape,
    StringKind, StringLength, TypeMember, TypeName, TypeNode, TypeTree,
};

use super::type_text::type_text;
use super::{TypeRef, live_machine, pick_revision, revisions_of};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::Isolation;

/// One revision of a struct, as the type rows of a machine hold it.
struct Revision<'a> {
    id: PlcTypeId,
    name: &'a TypeName,
    number: u32,
    members: &'a [TypeMember],
}

/// Read the live machine and its type rows in one `ReadOnly` transaction.
///
/// The transaction ends when the function returns. The caller builds and resolves the types later
/// (`reliability.md` section 7.4).
async fn read_catalog(
    ctx: &AppContext,
    code: MachineCode,
) -> Result<(Machine, Vec<PlcType>), AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    let machine = live_machine(tx.plant(), code).await?;
    let rows = tx.types().type_rows(machine.id()).await?;
    Ok((machine, rows))
}

/// Return the struct revisions of the machine, grouped by name in byte order.
///
/// Inside a group, the latest revision comes first.
fn grouped(types: &PlatformTypes) -> BTreeMap<&TypeName, Vec<Revision<'_>>> {
    let mut groups: BTreeMap<&TypeName, Vec<Revision<'_>>> = BTreeMap::new();
    for row in types.rows() {
        if let RecordShape::Struct {
            name,
            revision,
            members,
            ..
        } = row.shape()
        {
            groups.entry(name).or_default().push(Revision {
                id: row.id(),
                name,
                number: *revision,
                members,
            });
        }
    }
    for group in groups.values_mut() {
        group.sort_by_key(|revision| Reverse(revision.number));
    }
    groups
}

/// The parsed request of `ListTypes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypesInput {
    /// The code of the live machine.
    pub code: MachineCode,
}

/// One struct revision in the list of `ListTypes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeListRow {
    /// The identifier of the type row.
    pub id: PlcTypeId,
    /// The name of the struct.
    pub name: TypeName,
    /// The revision, from 1.
    pub revision: u32,
    /// The number of members.
    pub members: usize,
    /// Whether this is the highest revision of the name.
    pub latest: bool,
}

/// The result of `ListTypes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeList {
    /// The live machine.
    pub machine: Machine,
    /// One row for each struct revision. The names come in byte order, and the latest revision of a name comes first.
    pub rows: Vec<TypeListRow>,
}

/// Lists the struct revisions of a machine.
///
/// The query solves one problem: an engineer must see which UDT types the machine has and which
/// revision is the latest. The rows hold structs only. A primitive row, an array row, and a row of
/// another machine are not UDT types.
#[derive(Clone)]
pub struct ListTypes {
    ctx: AppContext,
}

impl UseCase for ListTypes {
    const OPERATION: Operation = Operation::ListTypes;
}

impl ListTypes {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the struct revisions of the live machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: TypesInput,
    ) -> Result<TypeList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let (machine, rows) = read_catalog(&self.ctx, input.code).await?;
        let types = PlatformTypes::collect(machine.id(), machine.platform(), rows)?;
        let rows = grouped(&types)
            .into_values()
            .flat_map(|group| {
                group
                    .into_iter()
                    .enumerate()
                    .map(|(index, revision)| TypeListRow {
                        id: revision.id,
                        name: revision.name.clone(),
                        revision: revision.number,
                        members: revision.members.len(),
                        latest: index == 0,
                    })
            })
            .collect();
        Ok(TypeList { machine, rows })
    }
}

/// The result of `GetTypeTree`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeView {
    /// The live machine.
    pub machine: Machine,
    /// The identifier of the type row of the revision.
    pub id: PlcTypeId,
    /// The name of the struct.
    pub name: TypeName,
    /// The revision that the tree shows. It is never the `None` of the request.
    pub revision: u32,
    /// Every revision of the name, the latest first.
    pub revisions: Vec<u32>,
    /// The revision that this revision supersedes, or `None` for the first revision.
    pub supersedes: Option<u32>,
    /// The resolved tree of the revision.
    pub tree: TypeTree,
    /// The type expression of each type row in the tree, written by `type_text`.
    pub member_types: BTreeMap<PlcTypeId, String>,
}

/// Write the type expression of every node of the tree, keyed by the row of the node.
///
/// The function solves one problem: the tree page and the type form must write a type in one way.
/// The walk uses a stack, so it needs no recursion.
fn member_texts(
    types: &PlatformTypes,
    tree: &TypeTree,
) -> Result<BTreeMap<PlcTypeId, String>, StorageError> {
    let mut texts = BTreeMap::new();
    let mut pending = vec![tree.root()];
    while let Some(node) = pending.pop() {
        let text = type_text(types, node.type_id()).ok_or_else(|| {
            StorageError::invariant("a tree node has a type that the catalog does not hold")
        })?;
        texts.insert(node.type_id(), text);
        match node {
            TypeNode::Leaf { .. } => {}
            TypeNode::Array { element, .. } => pending.push(element.as_ref()),
            TypeNode::Struct { members, .. } => {
                pending.extend(members.iter().map(|(_, member)| member));
            }
        }
    }
    Ok(texts)
}

/// Shows the resolved type tree of a struct revision.
///
/// The query solves one problem: before an engineer changes a struct, the engineer must see all
/// its leaves, its earlier revisions, and the revision that it replaces. The query ends its
/// transaction before it resolves the tree (`reliability.md` section 7.4).
#[derive(Clone)]
pub struct GetTypeTree {
    ctx: AppContext,
}

impl UseCase for GetTypeTree {
    const OPERATION: Operation = Operation::GetTypeTree;
}

impl GetTypeTree {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the tree of the latest revision, or of the revision that the request names.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code, when the machine has no
    /// struct with the name, or when the struct has no such revision.
    /// Returns `AppError::Plant` when the tree has a cycle or is too large.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: TypeRef,
    ) -> Result<TypeView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let (machine, rows) = read_catalog(&self.ctx, input.code).await?;
        let types = PlatformTypes::collect(machine.id(), machine.platform(), rows)?;
        let revisions = revisions_of(&types, &input.name);
        let (revision, id) = pick_revision(&revisions, &input)?;
        let tree = TypeTree::resolve(id, &types)?;
        let member_types = member_texts(&types, &tree)?;
        Ok(TypeView {
            member_types,
            id,
            name: input.name,
            revision,
            revisions: revisions.iter().map(|(number, _)| *number).collect(),
            supersedes: supersedes_of(&types, id),
            tree,
            machine,
        })
    }
}

/// The parsed request of `GetUdtForm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UdtFormInput {
    /// The code of the live machine.
    pub code: MachineCode,
    /// The struct whose latest revision the form copies. `None` means a new struct.
    pub from: Option<TypeName>,
}

/// The data of the form `New UDT type` and `New revision`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UdtForm {
    /// The live machine.
    pub machine: Machine,
    /// The name and the number of the latest revision that the form copies, or `None` for a new struct.
    pub from: Option<(TypeName, u32)>,
    /// The members of that revision in their order, each with its type expression.
    pub members: Vec<(MemberName, String)>,
    /// The proposals of the `datalist` of the type input. Each one is a type expression.
    pub choices: Vec<String>,
}

/// Prepares the form that defines a UDT type or a new revision of one.
///
/// The query solves one problem: a new revision starts from the members of the latest revision,
/// and the type of each member must come back as text that `TypeExpr::parse` reads. The query also
/// proposes the primitives of the platform and the latest revision of each struct, so the user
/// types less.
#[derive(Clone)]
pub struct GetUdtForm {
    ctx: AppContext,
}

impl UseCase for GetUdtForm {
    const OPERATION: Operation = Operation::GetUdtForm;
}

impl GetUdtForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the members of the latest revision of `from` and the proposals of the type input.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code or when the machine has
    /// no struct with the name `from`.
    /// Returns `AppError::Storage` when the database fails or when a member has a type that the
    /// catalog does not hold.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: UdtFormInput,
    ) -> Result<UdtForm, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let (machine, rows) = read_catalog(&self.ctx, input.code).await?;
        let types = PlatformTypes::collect(machine.id(), machine.platform(), rows)?;
        let groups = grouped(&types);
        let (from, members) = match &input.from {
            Some(name) => copied_revision(&types, &groups, input.code, name)?,
            None => (None, Vec::new()),
        };
        let choices = proposals(&types, &groups, input.from.as_ref());
        Ok(UdtForm {
            machine,
            from,
            members,
            choices,
        })
    }
}

/// The origin of a form and its members.
type Copied = (Option<(TypeName, u32)>, Vec<(MemberName, String)>);

/// Return the name, the number, and the members of the latest revision of the struct `name`.
fn copied_revision(
    types: &PlatformTypes,
    groups: &BTreeMap<&TypeName, Vec<Revision<'_>>>,
    code: MachineCode,
    name: &TypeName,
) -> Result<Copied, AppError> {
    let latest = groups
        .get(name)
        .and_then(|group| group.first())
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Type {
                code,
                name: name.clone(),
            },
        })?;
    let members = latest
        .members
        .iter()
        .map(|member| {
            let text = type_text(types, member.type_id).ok_or_else(|| {
                StorageError::invariant("a member has a type that the catalog does not hold")
            })?;
            Ok((member.name.clone(), text))
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    Ok((Some((name.clone(), latest.number)), members))
}

/// Return the proposals of the type input.
///
/// First come the primitives of the platform without `STRING` and `WSTRING`. Then come `STRING`,
/// and `WSTRING` when the platform has it. The latest revision of each struct follows as
/// `<name>@<revision>`. No revision of the struct `revised` is a proposal, because a struct
/// cannot hold an earlier revision of itself (ADR-0038).
fn proposals(
    types: &PlatformTypes,
    groups: &BTreeMap<&TypeName, Vec<Revision<'_>>>,
    revised: Option<&TypeName>,
) -> Vec<String> {
    let primitives: BTreeSet<Primitive> = types
        .rows()
        .filter_map(|row| match row.shape() {
            RecordShape::Primitive { primitive, .. }
                if !matches!(primitive, Primitive::String | Primitive::Wstring) =>
            {
                Some(*primitive)
            }
            _ => None,
        })
        .collect();
    let has_wstring = StringLength::default_length(StringKind::Wstring, types.platform()).is_some();
    let strings = [Primitive::String]
        .into_iter()
        .chain(has_wstring.then_some(Primitive::Wstring));
    let structs = groups
        .iter()
        .filter(|(name, _)| Some(**name) != revised)
        .filter_map(|(_, group)| group.first())
        .map(|latest| format!("{}@{}", latest.name, latest.number));
    primitives
        .into_iter()
        .chain(strings)
        .map(|primitive| primitive.name().to_owned())
        .chain(structs)
        .collect()
}

/// Return the revision number of the row that the struct row `id` supersedes, if any.
fn supersedes_of(types: &PlatformTypes, id: PlcTypeId) -> Option<u32> {
    let RecordShape::Struct {
        supersedes: Some(previous),
        ..
    } = types.get(id)?.shape()
    else {
        return None;
    };
    match types.get(*previous)?.shape() {
        RecordShape::Struct { revision, .. } => Some(*revision),
        _ => None,
    }
}
