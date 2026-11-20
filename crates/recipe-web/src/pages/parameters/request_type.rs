//! The parse of a type text that a parameter form sends (ADR-0206 (b), (d3)).

use recipe_domain::plant::{PlantError, RuleSet, TypeExpr};

/// Parse the type text `text` of a request of a parameter form.
///
/// The function solves one problem: the field `type` can hold a new text or the stored reference
/// that an edit form sends again, such as `Int@1`. The current rule set maps the alias `TOD` to
/// `TIME_OF_DAY` and refuses a struct name that is a reserved word. So the function returns the
/// result of [`TypeExpr::parse`] when it is `Ok`. Otherwise it parses the text again with the
/// earlier rule set, and the line of a refused text does not change. The function reads no row, so
/// it cannot tell an unchanged reference from a new text. The form query reads the rows and checks
/// the reference (`GetParameterForm`).
///
/// # Errors
///
/// Returns the `PlantError` of the earlier rule set when neither rule set parses `text`.
pub(crate) fn request_type_expr(text: &str) -> Result<TypeExpr, PlantError> {
    TypeExpr::parse(text).or_else(|_refused| TypeExpr::from_stored(text, RuleSet::Earlier))
}
