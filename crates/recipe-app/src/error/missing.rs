//! The entity that a URL value names and that does not exist (`api.md` section 1).

use std::fmt;

use recipe_domain::identity::Login;
use recipe_domain::plant::{MachineCode, MachineId, TypeName, UnitPath};
use recipe_domain::recipe::ReferenceNumber;

/// An entity that a request names and that the database does not hold.
///
/// The type solves one problem: `E-WEB-005` must name the missing entity in one fixed form.
/// `Display` gives the text of `{entity}` in the message of `E-WEB-005`.
/// `E-WEB-005` has this one owner. A path value that does not parse uses `MalformedLogin`,
/// `MalformedMachine`, or `MalformedType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    /// A machine, found by its code.
    Machine(MachineCode),
    /// A machine, found by its identifier.
    MachineId(MachineId),
    /// A unit of a machine.
    Unit {
        /// The code of the machine.
        code: MachineCode,
        /// The path of the unit.
        path: UnitPath,
    },
    /// A type of a machine.
    Type {
        /// The code of the machine.
        code: MachineCode,
        /// The name of the type.
        name: TypeName,
    },
    /// A recipe.
    Recipe(ReferenceNumber),
    /// A version of a recipe.
    Version {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
    },
    /// A user.
    User(Login),
    /// A user that a `{login}` path value names, when the value is not a login.
    ///
    /// No use case runs for it. The variant holds no text, so the typed value never reaches a
    /// message or a log line.
    MalformedLogin,
    /// A machine that a `{code}` or `{uuid}` path value names, when the value does not parse.
    ///
    /// No use case runs for it. The variant holds no text, so the typed value never reaches a
    /// message or a log line.
    MalformedMachine,
    /// A type that a `{name}`, `from`, or `rev` value names, when the value does not parse.
    ///
    /// No use case runs for it. The variant holds no text, so the typed value never reaches a
    /// message or a log line.
    MalformedType,
    /// A revision of a type of a machine.
    TypeRevision {
        /// The code of the machine.
        code: MachineCode,
        /// The name of the type.
        name: TypeName,
        /// The number of the revision.
        revision: u32,
    },
}

impl Missing {
    /// The version `number` of the recipe `reference`.
    #[must_use]
    pub fn version(reference: &ReferenceNumber, number: u32) -> Self {
        Self::Version {
            reference: reference.clone(),
            number,
        }
    }

    /// The machine with the identifier `id`.
    #[must_use]
    pub const fn machine_id(id: MachineId) -> Self {
        Self::MachineId(id)
    }
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine(code) => write!(f, "Machine {code}"),
            Self::MachineId(id) => write!(f, "Machine {id}"),
            Self::Unit { code, path } => write!(f, "Unit {path} of machine {code}"),
            Self::Type { code, name } => write!(f, "Type {name} of machine {code}"),
            Self::Recipe(reference) => write!(f, "Recipe {reference}"),
            Self::Version { reference, number } => {
                write!(f, "Version {number} of recipe {reference}")
            }
            Self::User(login) => write!(f, "User {login}"),
            Self::MalformedLogin => f.write_str("User"),
            Self::MalformedMachine => f.write_str("Machine"),
            Self::MalformedType => f.write_str("Type"),
            Self::TypeRevision {
                code,
                name,
                revision,
            } => write!(f, "Revision {revision} of type {name} of machine {code}"),
        }
    }
}

#[cfg(test)]
mod tests;
