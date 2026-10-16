//! The purposes of the form queries (ADR-0046 (b) and (c), `api.md` section 3.2).
//!
//! The input of a form query names the command that it serves, the purpose. The query checks the
//! permission of that command, and a `forbidden` event names that command.

use super::Operation;

/// The command that one form query serves.
///
/// The trait solves one problem: a form query must check the permission of the command that the
/// user tries, not one permission for every command. Each form query has its own purpose type, so
/// a purpose of another form query does not compile.
pub trait FormPurpose: Copy + 'static {
    /// The form query that serves the purposes of this type.
    const FORM: Operation;
    /// Every purpose of the form query, in the order of ADR-0046 (c).
    const ALL: &'static [Self];

    /// Return the command of the purpose. The form query checks its permission.
    fn operation(self) -> Operation;
}

/// The purpose of `GetMachineForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineFormPurpose {
    /// The form of `CreateMachine`.
    CreateMachine,
    /// The form of `RenameMachine`.
    RenameMachine,
}

impl FormPurpose for MachineFormPurpose {
    const FORM: Operation = Operation::GetMachineForm;
    const ALL: &'static [Self] = &[Self::CreateMachine, Self::RenameMachine];

    fn operation(self) -> Operation {
        match self {
            Self::CreateMachine => Operation::CreateMachine,
            Self::RenameMachine => Operation::RenameMachine,
        }
    }
}

/// The purpose of `GetUnitForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitFormPurpose {
    /// The form of `AddUnit`.
    AddUnit,
    /// The form of `RenameUnit`.
    RenameUnit,
}

impl FormPurpose for UnitFormPurpose {
    const FORM: Operation = Operation::GetUnitForm;
    const ALL: &'static [Self] = &[Self::AddUnit, Self::RenameUnit];

    fn operation(self) -> Operation {
        match self {
            Self::AddUnit => Operation::AddUnit,
            Self::RenameUnit => Operation::RenameUnit,
        }
    }
}

/// The purpose of `GetUdtForm`: a new type and a new revision both run `DefineUdt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UdtFormPurpose {
    /// The form of `DefineUdt`.
    DefineUdt,
}

impl FormPurpose for UdtFormPurpose {
    const FORM: Operation = Operation::GetUdtForm;
    const ALL: &'static [Self] = &[Self::DefineUdt];

    fn operation(self) -> Operation {
        match self {
            Self::DefineUdt => Operation::DefineUdt,
        }
    }
}

/// The purpose of `GetRecipeForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeFormPurpose {
    /// The form of `CreateRecipe`. It needs `RecipeCreate`.
    CreateRecipe,
    /// The form of `RenameRecipe`. It needs `RecipeEdit`.
    RenameRecipe,
}

impl FormPurpose for RecipeFormPurpose {
    const FORM: Operation = Operation::GetRecipeForm;
    const ALL: &'static [Self] = &[Self::CreateRecipe, Self::RenameRecipe];

    fn operation(self) -> Operation {
        match self {
            Self::CreateRecipe => Operation::CreateRecipe,
            Self::RenameRecipe => Operation::RenameRecipe,
        }
    }
}

/// The purpose of `GetVersionForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionFormPurpose {
    /// The form of `CreateVersion`.
    CreateVersion,
}

impl FormPurpose for VersionFormPurpose {
    const FORM: Operation = Operation::GetVersionForm;
    const ALL: &'static [Self] = &[Self::CreateVersion];

    fn operation(self) -> Operation {
        match self {
            Self::CreateVersion => Operation::CreateVersion,
        }
    }
}

/// The purpose of `GetParameterForm`: one of the eight commands of the `ParameterEdit` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterFormPurpose {
    /// The form of `AddParameter`.
    AddParameter,
    /// The form of `UpdateParameterDefinition`.
    UpdateParameterDefinition,
    /// The route of `SetParameterValue`.
    SetParameterValue,
    /// The route of `ReplaceLimits`.
    ReplaceLimits,
    /// The route of `UpsertTranslation`.
    UpsertTranslation,
    /// The route of `RemoveTranslation`.
    RemoveTranslation,
    /// The route of `RemoveParameter`.
    RemoveParameter,
    /// The route of `RebindTypeRevision`.
    RebindTypeRevision,
}

impl FormPurpose for ParameterFormPurpose {
    const FORM: Operation = Operation::GetParameterForm;
    const ALL: &'static [Self] = &[
        Self::AddParameter,
        Self::UpdateParameterDefinition,
        Self::SetParameterValue,
        Self::ReplaceLimits,
        Self::UpsertTranslation,
        Self::RemoveTranslation,
        Self::RemoveParameter,
        Self::RebindTypeRevision,
    ];

    fn operation(self) -> Operation {
        match self {
            Self::AddParameter => Operation::AddParameter,
            Self::UpdateParameterDefinition => Operation::UpdateParameterDefinition,
            Self::SetParameterValue => Operation::SetParameterValue,
            Self::ReplaceLimits => Operation::ReplaceLimits,
            Self::UpsertTranslation => Operation::UpsertTranslation,
            Self::RemoveTranslation => Operation::RemoveTranslation,
            Self::RemoveParameter => Operation::RemoveParameter,
            Self::RebindTypeRevision => Operation::RebindTypeRevision,
        }
    }
}

impl Operation {
    /// Return the commands that a form query serves (ADR-0046 (c)).
    ///
    /// Every other operation gives an empty slice. The requirement of a form query allows a role
    /// that may run at least one of its purposes.
    #[must_use]
    pub const fn purposes(self) -> &'static [Self] {
        match self {
            Self::GetMachineForm => &[Self::CreateMachine, Self::RenameMachine],
            Self::GetUnitForm => &[Self::AddUnit, Self::RenameUnit],
            Self::GetUdtForm => &[Self::DefineUdt],
            Self::GetRecipeForm => &[Self::CreateRecipe, Self::RenameRecipe],
            Self::GetVersionForm => &[Self::CreateVersion],
            Self::GetParameterForm => &[
                Self::AddParameter,
                Self::UpdateParameterDefinition,
                Self::SetParameterValue,
                Self::ReplaceLimits,
                Self::UpsertTranslation,
                Self::RemoveTranslation,
                Self::RemoveParameter,
                Self::RebindTypeRevision,
            ],
            _other => &[],
        }
    }
}
