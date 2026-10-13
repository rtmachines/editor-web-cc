//! Diffs of a parameter change (design D9, `data-model.md` sections 9.2 and 10).
//!
//! Five audit actions store a diff of the changed fields and not a full row. A diff has two
//! objects, one for the state before and one for the state after. An unchanged field is absent.
//! A field that one side does not hold is `null` on that side.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{DerivedTypeReason, PlantError, PlcTypeId, TypeTree, TypeTrees};
use recipe_domain::recipe::{Parameter, ParameterChange};
use serde_json::{Map, Value, json};

use crate::error::AppError;
use leaves::stored_leaves;

mod leaves;

/// The fields of one side of a diff, keyed by column name, leaf path, or language code.
type Fields = BTreeMap<String, Value>;

/// The two objects of a diff: the changed fields before and after the change.
///
/// The type solves one problem: an audit entry of a diff action needs both states in one value
/// that the builders fill in the same way.
#[derive(Clone, Debug, PartialEq)]
pub struct StateDiff {
    /// The changed fields as they were before the change.
    pub before: Value,
    /// The changed fields as they are after the change.
    pub after: Value,
}

/// Build the diff of `ParameterValueSet`: `{"value": {"<leaf path>": <leaf>}}`.
///
/// # Errors
///
/// Returns `AppError::Plant` when `trees` has no tree for a type of the change.
pub fn value_diff(change: &ParameterChange, trees: &TypeTrees) -> Result<StateDiff, AppError> {
    value_diff_of(change.before(), change.after(), trees)
}

/// Build the diff of `LimitsReplaced`: `{"limits": {"<leaf path>": {...} or null}}`.
#[must_use]
pub fn limit_diff(change: &ParameterChange) -> StateDiff {
    limit_diff_of(change.before(), change.after())
}

/// Build the diff of `TranslationUpserted`: `{"translations": {"<lang>": {...} or null}}`.
#[must_use]
pub fn translation_diff(change: &ParameterChange) -> StateDiff {
    translation_diff_of(change.before(), change.after())
}

/// Build the diff of `ParameterDefinitionUpdated`.
///
/// The diff holds the changed columns. A retype adds the changed value leaves and limits.
///
/// # Errors
///
/// Returns `AppError::Plant` when `trees` has no tree for a type of the change.
pub fn definition_diff(change: &ParameterChange, trees: &TypeTrees) -> Result<StateDiff, AppError> {
    definition_diff_of(change.before(), change.after(), trees)
}

/// Build the diff of `TypeRevisionRebound`: `plc_type_id`, the value leaves, and the limits.
///
/// # Errors
///
/// Returns `AppError::Plant` when `trees` has no tree for a type of the change.
pub fn rebind_diff(change: &ParameterChange, trees: &TypeTrees) -> Result<StateDiff, AppError> {
    rebind_diff_of(change.before(), change.after(), trees)
}

fn value_diff_of(
    before: Option<&Parameter>,
    after: Option<&Parameter>,
    trees: &TypeTrees,
) -> Result<StateDiff, AppError> {
    let mut builder = Builder::default();
    builder.value(before, after, trees)?;
    Ok(builder.finish())
}

fn limit_diff_of(before: Option<&Parameter>, after: Option<&Parameter>) -> StateDiff {
    let mut builder = Builder::default();
    builder.limits(before, after);
    builder.finish()
}

fn translation_diff_of(before: Option<&Parameter>, after: Option<&Parameter>) -> StateDiff {
    let mut builder = Builder::default();
    builder.section("translations", &translations(before), &translations(after));
    builder.finish()
}

fn definition_diff_of(
    before: Option<&Parameter>,
    after: Option<&Parameter>,
    trees: &TypeTrees,
) -> Result<StateDiff, AppError> {
    let mut builder = Builder::default();
    builder.columns(&columns(before), &columns(after));
    builder.value(before, after, trees)?;
    builder.limits(before, after);
    Ok(builder.finish())
}

fn rebind_diff_of(
    before: Option<&Parameter>,
    after: Option<&Parameter>,
    trees: &TypeTrees,
) -> Result<StateDiff, AppError> {
    let mut builder = Builder::default();
    builder.columns(&type_column(before), &type_column(after));
    builder.value(before, after, trees)?;
    builder.limits(before, after);
    Ok(builder.finish())
}

/// The two sides of a diff while the builders fill them.
#[derive(Default)]
struct Builder {
    before: Fields,
    after: Fields,
}

impl Builder {
    /// Add the changed fields as keys of the diff itself.
    fn columns(&mut self, before: &Fields, after: &Fields) {
        let (changed_before, changed_after) = changed(before, after);
        self.before.extend(changed_before);
        self.after.extend(changed_after);
    }

    /// Add the changed fields as an object under `name`. An unchanged section stays absent.
    fn section(&mut self, name: &str, before: &Fields, after: &Fields) {
        let (changed_before, changed_after) = changed(before, after);
        if changed_before.is_empty() {
            return;
        }
        self.before.insert(name.to_owned(), object(changed_before));
        self.after.insert(name.to_owned(), object(changed_after));
    }

    fn value(
        &mut self,
        before: Option<&Parameter>,
        after: Option<&Parameter>,
        trees: &TypeTrees,
    ) -> Result<(), AppError> {
        self.section(
            "value",
            &value_leaves(before, trees)?,
            &value_leaves(after, trees)?,
        );
        Ok(())
    }

    fn limits(&mut self, before: Option<&Parameter>, after: Option<&Parameter>) {
        self.section("limits", &limit_rows(before), &limit_rows(after));
    }

    fn finish(self) -> StateDiff {
        StateDiff {
            before: object(self.before),
            after: object(self.after),
        }
    }
}

fn object(fields: Fields) -> Value {
    Value::Object(fields.into_iter().collect::<Map<String, Value>>())
}

/// Return the fields that differ between the sides, with `null` for a side that lacks one.
fn changed(before: &Fields, after: &Fields) -> (Fields, Fields) {
    let keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let mut changed_before = Fields::new();
    let mut changed_after = Fields::new();
    for key in keys {
        let (old, new) = (before.get(key), after.get(key));
        if old != new {
            changed_before.insert(key.clone(), old.cloned().unwrap_or(Value::Null));
            changed_after.insert(key.clone(), new.cloned().unwrap_or(Value::Null));
        }
    }
    (changed_before, changed_after)
}

/// The columns of `parameter` that a definition update can change (`data-model.md` 3.12).
fn columns(parameter: Option<&Parameter>) -> Fields {
    let Some(parameter) = parameter else {
        return Fields::new();
    };
    let mut fields = type_column(Some(parameter));
    fields.insert("unit_id".to_owned(), json!(parameter.unit().to_string()));
    fields.insert("tag".to_owned(), json!(parameter.tag().to_string()));
    fields.insert("name_en".to_owned(), json!(parameter.name_en().as_str()));
    fields.insert(
        "description_en".to_owned(),
        json!(parameter.description_en().as_str()),
    );
    if let Some(address) = parameter.s7_address() {
        fields.insert("s7_address".to_owned(), json!(address.to_string()));
    }
    fields.insert("ordinal".to_owned(), json!(parameter.ordinal().value()));
    fields
}

fn type_column(parameter: Option<&Parameter>) -> Fields {
    parameter
        .map(|parameter| {
            Fields::from([(
                "plc_type_id".to_owned(),
                json!(parameter.plc_type().to_string()),
            )])
        })
        .unwrap_or_default()
}

/// The leaves of the value of `parameter`, read with the tree of its type.
fn value_leaves(parameter: Option<&Parameter>, trees: &TypeTrees) -> Result<Fields, AppError> {
    let Some(parameter) = parameter else {
        return Ok(Fields::new());
    };
    let tree = tree_of(trees, parameter.plc_type())?;
    Ok(stored_leaves(parameter.value().as_json(), tree))
}

/// Return the tree of `id`, or the error of its resolution (`E-PLT-013` when it has none).
fn tree_of(trees: &TypeTrees, id: PlcTypeId) -> Result<&TypeTree, AppError> {
    match trees.get(id) {
        Some(Ok(tree)) => Ok(tree),
        Some(Err(error)) => Err(AppError::Plant(error.clone())),
        None => Err(AppError::Plant(PlantError::DerivedTypeInvalid {
            type_expr: id.to_string(),
            reason: DerivedTypeReason::RowShape,
        })),
    }
}

/// The limit rows of `parameter` with the column names of `data-model.md` 3.13.
fn limit_rows(parameter: Option<&Parameter>) -> Fields {
    parameter
        .into_iter()
        .flat_map(Parameter::limits)
        .map(|limit| {
            let row = json!({
                "min_value": limit.min.map(|bound| bound.to_string()),
                "max_value": limit.max.map(|bound| bound.to_string()),
                "max_length": limit.max_length,
            });
            (limit.leaf_path.to_string(), row)
        })
        .collect()
}

/// The translations of `parameter`, keyed by language. A missing description is empty.
fn translations(parameter: Option<&Parameter>) -> Fields {
    parameter
        .into_iter()
        .flat_map(|parameter| parameter.translations().values())
        .map(|translation| {
            let row = json!({
                "name": translation.name.as_str(),
                "description": translation.description.as_ref().map_or("", |text| text.as_str()),
            });
            (translation.lang.to_string(), row)
        })
        .collect()
}

#[cfg(test)]
mod tests;
