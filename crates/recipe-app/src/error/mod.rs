//! Error types of the application layer (`reliability.md` section 1.1).

mod app;
mod mapping;
mod missing;
mod storage;

pub use app::{AppError, TaskStage, TimeoutStage};
pub use mapping::{
    APP_USER_LOGIN_UNIQUE, MACHINE_CODE_LIVE_UNIQUE, PARAMETER_TAG_UNIQUE,
    PLC_CONNECTION_MACHINE_PLATFORM, PLC_CONNECTION_MACHINE_UNIQUE, PLC_TYPE_MEMBER_NAME_CI_UNIQUE,
    PLC_TYPE_MEMBER_NAME_UNIQUE, PLC_TYPE_NAMED_UNIQUE, PLC_TYPE_STRUCT_NAME_CI_UNIQUE,
    RECIPE_REFERENCE_UNIQUE, RECIPE_VERSION_ONE_LIVE_DRAFT, UNIT_PARENT_EXISTS, UNIT_PATH_UNIQUE,
    conflict, map_storage,
};
pub use missing::Missing;
pub use storage::{AuditError, LockFailure, StorageError};
