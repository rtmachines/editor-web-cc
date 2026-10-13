//! Port of the PLC type catalog (`data-model.md` sections 1.4, 3.3, and 6).

use async_trait::async_trait;
use recipe_domain::plant::{MachineId, PlcType, PlcTypeId, PlcTypeRecord, TypeName};

use super::RowImage;
use crate::error::StorageError;

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
