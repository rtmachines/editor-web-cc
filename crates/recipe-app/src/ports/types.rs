//! Port of the PLC type catalog (`data-model.md` sections 1.4, 3.3, and 6).

use async_trait::async_trait;
use recipe_domain::plant::{MachineId, PlcType, PlcTypeId, PlcTypeRecord, TypeName};
use recipe_domain::recipe::{ParameterTag, ReferenceNumber, VersionStatus};

use super::RowImage;
use crate::error::StorageError;

/// One parameter that uses a type revision (`TypeCatalog::type_usage`).
///
/// The type solves one problem: both adapters must return the same five columns of a row.
/// A struct names them, so the adapters agree on their order and their meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeUsageRow {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version.
    pub number: u32,
    /// The status of the version. A live draft has the status `Draft`.
    pub status: VersionStatus,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The type of the parameter. It holds the revision that the request names.
    pub parameter_type: PlcTypeId,
}

/// One class-2 advisory lock key of a derived type (`data-model.md` section 1.4).
///
/// The type solves one problem: two commands must take the class-2 keys in one order,
/// or they can deadlock. The derived `Ord` is that order. All struct names come first,
/// sorted by name. Then come `STRING`, `WSTRING`, and `ARRAY`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DerivedTypeLock {
    /// The key of a struct name.
    Struct(TypeName),
    /// The key `STRING`.
    String,
    /// The key `WSTRING`.
    Wstring,
    /// The key `ARRAY`.
    Array,
}

impl DerivedTypeLock {
    /// Return the `name` part of the key: the struct name, `STRING`, `WSTRING`, or `ARRAY`.
    #[must_use]
    pub fn key_text(&self) -> &str {
        match self {
            Self::Struct(name) => name.as_str(),
            Self::String => "STRING",
            Self::Wstring => "WSTRING",
            Self::Array => "ARRAY",
        }
    }
}

/// The result of `TypeCatalog::ensure_derived`.
///
/// The type solves one problem: `EnsureDerivedType` writes a `DerivedTypeCreated` entry
/// only for a row that it inserted, so the caller must tell a new row from an old one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ensured {
    /// The identifier of the row, new or existing.
    pub id: PlcTypeId,
    /// The image of the row when this call inserted it, and `None` for an existing row.
    pub created: Option<RowImage>,
}

/// Port of the PLC type rows.
///
/// The port solves one problem: the commands that define or resolve types need the rows
/// of a machine, the class-2 advisory locks, and the idempotent insert of derived rows.
/// No method takes a row lock, because `recipe_app` has no `UPDATE` grant on `plc_type`.
#[async_trait]
pub trait TypeCatalog: Send {
    /// Read the global rows and the rows of `machine` without a lock (`EnsureDerivedType`).
    ///
    /// The use case builds `PlatformTypes::collect` from the result.
    async fn type_rows(&mut self, machine: MachineId) -> Result<Vec<PlcType>, StorageError>;

    /// Read one page of the parameters that use the type `type_id` without a lock (`GetTypeUsage`).
    ///
    /// A parameter uses the type when its type is `type_id`. It also uses the type when its
    /// type holds `type_id` through a member or an array element, on any level.
    /// The result has one row for each parameter, also when its type holds `type_id` twice.
    /// The rows come from the recipes of `machine` that are not deleted.
    /// They come from the versions that are not discarded.
    /// The rows come in the byte order of the reference number, then the newest version first,
    /// then in the byte order of the tag.
    /// The page starts after `offset` rows and holds at most `limit` rows.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Other` (`E-DB-008`) when `offset` is above `i64::MAX`.
    /// Both adapters return this error and never an empty list for such an offset.
    async fn type_usage(
        &mut self,
        machine: MachineId,
        type_id: PlcTypeId,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<TypeUsageRow>, StorageError>;

    /// Run `pg_advisory_xact_lock(2, hashtext(machine_id || ':' || name))` for each key.
    ///
    /// The adapter takes the keys in the given order. The caller sorts them.
    async fn lock_derived_types(
        &mut self,
        machine: MachineId,
        keys: &[DerivedTypeLock],
    ) -> Result<(), StorageError>;

    /// Run `INSERT plc_type ... ON CONFLICT DO NOTHING` and then `SELECT` the row (`EnsureDerivedType`).
    ///
    /// The record is a `STRING(n)`, a `WSTRING(n)`, or an array row. An existing row of
    /// the same shape gives its id and no image. Any other record gives `Constraint`
    /// with `E-DB-003`.
    async fn ensure_derived(&mut self, record: PlcTypeRecord) -> Result<Ensured, StorageError>;

    /// Run `INSERT plc_type` and `INSERT plc_type_member` for a struct (`DefineUdt`).
    ///
    /// The image holds the member rows. The same machine, name, and revision give
    /// `Duplicate` with `plc_type_named_unique`.
    async fn insert_struct(&mut self, row: &PlcType) -> Result<RowImage, StorageError>;
}
