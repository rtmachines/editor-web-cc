//! Use cases of the Plant Model (`domain-model.md` section 5.1).

mod add_unit;
mod archive_machine;
mod archive_unit;
mod create_machine;
mod define_udt;
mod derived_lock;
mod ensure_derived;
mod queries;
mod rename_machine;
#[cfg(test)]
mod short_rows;

pub use define_udt::{DefineUdt, DefineUdtInput, DefineUdtMode, DefineUdtOutput};
pub(crate) use derived_lock::{derived_type_locks, rebind_locks};
pub(crate) use ensure_derived::DerivedTypes;
pub(crate) use queries::type_text;
mod rename_unit;
mod resolve;

pub use add_unit::{AddUnit, AddUnitInput, AddUnitOutput};
pub use archive_machine::{ArchiveMachine, ArchiveMachineInput, ArchiveMachineOutput};
pub use archive_unit::{ArchiveUnit, ArchiveUnitInput, ArchiveUnitOutput};
pub use create_machine::{CreateMachine, CreateMachineInput, CreateMachineOutput};
pub use queries::{
    GetMachine, GetMachineForm, GetMachineInput, GetTypeTree, GetTypeUsage, GetUdtForm, GetUnit,
    GetUnitForm, GetUnitTree, ListMachines, ListMachinesInput, ListTypes, MachineForm,
    MachineFormInput, MachineList, MachineRef, MachineView, TypeList, TypeListRow, TypeRef,
    TypeUsage, TypeUsageInput, TypeUsageLine, TypeView, TypesInput, UdtForm, UdtFormInput,
    UnitForm, UnitFormInput, UnitRef, UnitTreeInput, UnitView,
};
pub use rename_machine::{RenameMachine, RenameMachineInput, RenameMachineOutput};
pub use rename_unit::{RenameUnit, RenameUnitInput, RenameUnitOutput};
