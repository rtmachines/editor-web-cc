//! Connection settings of a machine PLC (`domain-model.md` section 2.1, ADR-0154).
//!
//! Each value object of the connection record has its own submodule.
//! The submodule `shape` holds the platform shape and its constructor.

mod ams_net_id;
mod comm_mode;
mod data_block_number;
mod fields;
mod host;
mod password_plan;
mod plc_address;
mod plc_login;
mod plc_password;
mod rack;
mod range;
mod shape;
mod slot;
mod tcp_port;
mod text;

pub use ams_net_id::{AmsNetId, AmsNetIdReason};
pub use comm_mode::{CommMode, ModeReason};
pub use data_block_number::DataBlockNumber;
pub use fields::ConnectionFields;
pub use host::{HostReason, PlcHost};
pub use password_plan::{PasswordPlan, PasswordState};
pub use plc_address::PlcAddress;
pub use plc_login::PlcLogin;
pub use plc_password::PlcPassword;
pub use rack::Rack;
pub use range::RangeReason;
pub use shape::{ConnectionPlan, PlcConnection};
pub use slot::Slot;
pub use tcp_port::TcpPort;
pub use text::ConnectionTextReason;
