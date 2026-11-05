//! Identifiers of the Plant Model context (`domain-model.md` section 2.1).

use std::fmt;

use uuid::Uuid;

/// Stable identity of a machine (domain-model.md section 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineId(Uuid);

impl From<Uuid> for MachineId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl MachineId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for MachineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable identity of a unit (domain-model.md section 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(Uuid);

impl From<Uuid> for UnitId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl UnitId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable identity of a PLC type (domain-model.md section 2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlcTypeId(Uuid);

impl From<Uuid> for PlcTypeId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl PlcTypeId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for PlcTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests;
