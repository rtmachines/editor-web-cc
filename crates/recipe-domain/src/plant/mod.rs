//! The Plant Model context of `domain-model.md` section 2.1.
//!
//! This module holds the value objects and the error type of machines,
//! units, and PLC types.

mod array_dimension;
mod error;
mod ids;
mod iec_literal;
mod leaf_path;
mod lexical;
pub mod limits;
mod machine;
mod machine_code;
mod platform;
mod plc_type;
mod primitive;
mod s7_address;
mod s7_layout;
mod s7_width;
mod string_length;
mod type_expr;
mod type_name;
mod type_tree;
mod unit_path;
mod unit_usage;

pub use array_dimension::{ArrayDimension, ArrayDimensionReason, ArrayDimensions};
pub use error::{DerivedTypeReason, PlantError, TypeLimit};
pub use ids::{MachineId, PlcTypeId, UnitId};
pub use iec_literal::{IecLiteral, IecLiteralReason, TimeType};
pub use leaf_path::{LeafPath, LeafPathReason, LeafSegment, LeafTarget};
pub use machine::{Machine, StoredMachine, StoredUnit, Unit};
pub use machine_code::{MachineCode, MachineCodeReason};
pub use platform::{PlcPlatform, PlcPlatformReason};
pub use plc_type::{PlcType, PlcTypeRecord, RecordShape, TypeKind, TypeMember};
pub use primitive::{LeafClass, LeafClassReason, Primitive, PrimitiveReason};
pub use s7_address::{S7Address, S7AddressReason, S7Width};
pub use s7_layout::{S7Layout, S7Start};
pub use string_length::{StringKind, StringLength, StringLengthReason};
pub use type_expr::{BaseExpr, TypeExpr};
pub use type_name::{MemberName, MemberNameReason, TypeName, TypeNameReason};
pub use type_tree::{PlatformTypes, TypeNode, TypeTree};
pub use unit_path::UnitPath;
pub use unit_usage::UnitUsage;
