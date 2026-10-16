//! Use cases of Recipe Management (`domain-model.md` section 5.2).

mod add_parameter;
#[cfg(test)]
mod add_parameter_errors;
mod approve_version;
mod create_recipe;
mod create_version;
mod definition_steps;
mod delete_recipe;
mod discard_retire;
mod draft_session;
mod queries;
mod rebind;
mod remove_parameter;
mod rename_restore;
mod replace_limits;
mod resolve;
mod set_value;
#[cfg(test)]
mod test_support;
mod translation;
mod update_definition;

pub use add_parameter::{AddParameter, AddParameterInput, AddParameterOutput};
pub use approve_version::{ApproveVersion, ApproveVersionInput};
pub use create_recipe::{CreateRecipe, CreateRecipeInput, CreateRecipeOutput};
pub use create_version::{CreateVersion, CreateVersionInput, CreateVersionOutput};
pub use delete_recipe::{DeleteRecipe, DeleteRecipeInput, DeleteRecipeOutput};
pub use discard_retire::{
    DiscardDraft, DiscardDraftInput, RetireVersion, RetireVersionInput, VersionOutput,
};
pub use draft_session::DraftOutput;
pub use queries::{
    GetLimits, GetParameter, GetParameterForm, GetParameterFormInput, GetRecipe, GetRecipeForm,
    GetRecipeInput, GetTranslations, GetVersion, GetVersionForm, GetVersionFormInput,
    GetVersionInput, LastChange, LimitsView, ListDeletedRecipes, ListParameters,
    ListParametersInput, ListRecipes, ListVersions, ListVersionsInput, OutdatedParameter,
    ParameterForm, ParameterList, ParameterRef, ParameterView, RecipeForm, RecipeFormInput,
    RecipeList, RecipeListInput, RecipeView, StructRevision, TranslationsView, VersionChoice,
    VersionForm, VersionList, VersionView,
};
pub use rebind::{RebindTypeRevision, RebindTypeRevisionInput};
pub use remove_parameter::{RemoveParameter, RemoveParameterInput};
pub use rename_restore::{
    RenameRecipe, RenameRecipeInput, RenameRecipeOutput, RestoreRecipe, RestoreRecipeInput,
    RestoreRecipeOutput,
};
pub use replace_limits::{ReplaceLimits, ReplaceLimitsInput};
pub use set_value::{SetParameterValue, SetParameterValueInput};
pub use translation::{
    RemoveTranslation, RemoveTranslationInput, UpsertTranslation, UpsertTranslationInput,
};
pub use update_definition::{
    RetypeInput, UpdateParameterDefinition, UpdateParameterDefinitionInput,
};
