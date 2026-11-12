//! Use cases of the Plant Model (`domain-model.md` section 5.1).

mod add_unit;
mod archive_machine;
mod archive_unit;
mod create_machine;
mod current_type;
mod define_udt;
mod derived_lock;
mod ensure_derived;
mod queries;
mod rename_machine;
#[cfg(test)]
mod short_rows;

pub(crate) use current_type::require_current_type;
pub use define_udt::{DefineUdt, DefineUdtInput, DefineUdtMode, DefineUdtOutput};
pub(crate) use derived_lock::{derived_type_locks, rebind_locks};
pub(crate) use ensure_derived::DerivedTypes;
pub(crate) use queries::{expr_names_row, type_text};
mod rename_unit;
mod resolve;
mod set_machine_data_block;
mod set_plc_connection;

pub use add_unit::{AddUnit, AddUnitInput, AddUnitOutput};
pub use archive_machine::{ArchiveMachine, ArchiveMachineInput, ArchiveMachineOutput};
pub use archive_unit::{ArchiveUnit, ArchiveUnitInput, ArchiveUnitOutput};
pub use create_machine::{CreateMachine, CreateMachineInput, CreateMachineOutput};
pub use queries::{
    GetMachine, GetMachineForm, GetMachineInput, GetPlcConnectionForm, GetTypeTree, GetTypeUsage,
    GetUdtForm, GetUnit, GetUnitForm, GetUnitTree, ListMachines, ListMachinesInput, ListTypes,
    MachineConnection, MachineForm, MachineFormInput, MachineList, MachineRef, MachineView,
    PlcConnectionForm, PlcConnectionFormInput, TypeList, TypeListRow, TypeRef, TypeUsage,
    TypeUsageInput, TypeUsageLine, TypeView, TypesInput, UdtForm, UdtFormInput, UnitForm,
    UnitFormInput, UnitRef, UnitTreeInput, UnitView,
};
pub use rename_machine::{RenameMachine, RenameMachineInput, RenameMachineOutput};
pub use rename_unit::{RenameUnit, RenameUnitInput, RenameUnitOutput};
pub use set_machine_data_block::{
    SetMachineDataBlock, SetMachineDataBlockInput, SetMachineDataBlockOutput,
};
pub use set_plc_connection::{SetPlcConnection, SetPlcConnectionInput, SetPlcConnectionOutput};
