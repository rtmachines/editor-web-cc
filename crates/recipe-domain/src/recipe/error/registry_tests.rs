//! Registry test of decision D14 (`design.md`): the domain rows of `reliability.md` section 1.2
//! match the variants of `RecipeError`, their display texts, and `REGISTRY`.

use super::*;
use crate::error_codes::REGISTRY;
use crate::identity::Login;
use crate::plant::{LeafPath, PlcPlatform, S7Address, S7Width, TypeName};
use crate::recipe::{
    LangCode, ParameterKey, ParameterTag, RebindTargetReason, SnapshotError, SnapshotItem,
    VersionStatus,
};
use std::collections::BTreeSet;

/// The number of `E-RCP` domain rows: `E-RCP-001..029` and `E-RCP-050..054`.
const DOC_ROW_COUNT: usize = 34;

/// One domain row of `reliability.md` section 1.2 that an owner `RecipeError::` names.
struct DocRow {
    code: String,
    variant: String,
    message: String,
}

/// One sample of a variant and the display text of each of its fields.
type Sample = (RecipeError, Vec<(&'static str, String)>);

/// Return the message of a table cell without its trailing note in parentheses.
fn message_of(cell: &str) -> &str {
    let text = cell.trim();
    match text.strip_suffix(')').and_then(|head| head.rfind(" (")) {
        Some(index) => text.get(..index).unwrap_or(text),
        None => text,
    }
}

/// Return the `E-RCP` domain rows of `reliability.md` section 1.2 in document order.
fn doc_rows() -> Vec<DocRow> {
    const DOC: &str = include_str!("../../../../../docs/architecture/reliability.md");
    DOC.lines()
        .filter(|line| line.starts_with("| `E-RCP-"))
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = cells.get(1)?.trim_matches('`');
            let owner = cells.get(2)?.trim_matches('`');
            let variant = owner
                .strip_prefix("RecipeError::")?
                .split([' ', '{', '('])
                .next()?;
            let message = message_of(cells.get(4)?);
            Some(DocRow {
                code: code.to_owned(),
                variant: variant.to_owned(),
                message: message.to_owned(),
            })
        })
        .collect()
}

/// Replace each `{field}` of a template with the text of its value.
fn fill(template: &str, fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

/// Return the name of the variant of an error, from its `Debug` text.
fn variant_name(error: &RecipeError) -> String {
    format!("{error:?}")
        .split([' ', '{', '('])
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-1").unwrap()
}

fn tag() -> ParameterTag {
    ParameterTag::parse("SPEED").unwrap()
}

fn leaf() -> LeafPath {
    LeafPath::parse(".zones[2]").unwrap()
}

fn address() -> S7Address {
    S7Address::parse("DB1.DBW0").unwrap()
}

fn zone() -> TypeName {
    TypeName::parse("ST_Zone").unwrap()
}

fn key() -> ParameterKey {
    ParameterKey::parse("6f1f4a8e-5b1c-4d3e-9f2a-0123456789ab").unwrap()
}

fn login() -> Login {
    Login::parse("anna").unwrap()
}

/// Samples of the states of a recipe, the identity of a draft, and the lists.
fn recipe_samples() -> Vec<Sample> {
    vec![
        (
            RecipeError::DuplicateReference {
                reference: reference(),
            },
            vec![("reference", reference().to_string())],
        ),
        (
            RecipeError::InvalidLangCode {
                lang: "xx-".to_owned(),
            },
            vec![("lang", "xx-".to_owned())],
        ),
        (
            RecipeError::RecipeDeleted {
                reference: reference(),
            },
            vec![("reference", reference().to_string())],
        ),
        (
            RecipeError::RecipeHasApprovedVersion {
                reference: reference(),
            },
            vec![("reference", reference().to_string())],
        ),
        (
            RecipeError::RecipeHasRetiredVersion {
                reference: reference(),
            },
            vec![("reference", reference().to_string())],
        ),
        (
            RecipeError::RecipeNotDeleted {
                reference: reference(),
            },
            vec![("reference", reference().to_string())],
        ),
        (
            RecipeError::ApproverEditedDraft {
                reference: reference(),
                number: 4,
                login: login(),
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "4".to_owned()),
                ("login", login().to_string()),
            ],
        ),
        (
            RecipeError::VersionNotDraft {
                reference: reference(),
                number: 2,
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "2".to_owned()),
            ],
        ),
        (
            RecipeError::VersionNotApproved {
                reference: reference(),
                number: 2,
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "2".to_owned()),
            ],
        ),
        (
            RecipeError::VersionNotEditable {
                reference: reference(),
                number: 2,
                status: VersionStatus::Approved,
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "2".to_owned()),
                ("status", VersionStatus::Approved.to_string()),
            ],
        ),
        (
            RecipeError::DraftAlreadyExists {
                reference: reference(),
                number: 3,
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "3".to_owned()),
            ],
        ),
        (
            RecipeError::DraftHasNoParameters {
                reference: reference(),
                number: 3,
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "3".to_owned()),
            ],
        ),
        (
            RecipeError::SizeBudgetExceeded {
                budget: Budget::Parameters,
                limit: 4000,
                actual: 4001,
            },
            vec![
                ("budget", Budget::Parameters.to_string()),
                ("limit", "4000".to_owned()),
                ("actual", "4001".to_owned()),
            ],
        ),
        (
            RecipeError::ValidationFailed {
                items: vec![
                    SnapshotItem::Version {
                        error: SnapshotError::Recipe(RecipeError::RecipeDeleted {
                            reference: reference(),
                        }),
                    },
                    SnapshotItem::Parameter {
                        tag: tag(),
                        error: SnapshotError::Recipe(RecipeError::RecipeDeleted {
                            reference: reference(),
                        }),
                    },
                    SnapshotItem::Parameter {
                        tag: tag(),
                        error: SnapshotError::Recipe(RecipeError::RecipeDeleted {
                            reference: reference(),
                        }),
                    },
                ],
            },
            vec![("count", "1".to_owned())],
        ),
        (
            RecipeError::TranslationNotFound {
                reference: reference(),
                number: 2,
                tag: tag(),
                lang: LangCode::parse("de-AT").unwrap(),
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "2".to_owned()),
                ("tag", tag().to_string()),
                ("lang", "de-AT".to_owned()),
            ],
        ),
        (
            RecipeError::ParameterNotFound {
                reference: reference(),
                number: 2,
                key: key(),
            },
            vec![
                ("reference", reference().to_string()),
                ("number", "2".to_owned()),
                ("key", key().to_string()),
            ],
        ),
    ]
}

/// Samples of the parameters, their values, their addresses, and their limits.
fn parameter_samples() -> Vec<Sample> {
    vec![
        (
            RecipeError::DuplicateTag {
                tag: tag(),
                number: 2,
            },
            vec![("tag", tag().to_string()), ("number", "2".to_owned())],
        ),
        (
            RecipeError::S7AddressRequired { tag: tag() },
            vec![("tag", tag().to_string())],
        ),
        (
            RecipeError::S7AddressOverlap {
                tag: tag(),
                other_tag: ParameterTag::parse("TEMP").unwrap(),
                address: address(),
            },
            vec![
                ("tag", tag().to_string()),
                ("other_tag", "TEMP".to_owned()),
                ("address", address().to_string()),
            ],
        ),
        (
            RecipeError::S7AddressMisaligned {
                tag: tag(),
                address: address(),
            },
            vec![
                ("tag", tag().to_string()),
                ("address", address().to_string()),
            ],
        ),
        (
            RecipeError::S7AddressNotAllowed {
                tag: tag(),
                platform: PlcPlatform::S7_300,
            },
            vec![
                ("tag", tag().to_string()),
                ("platform", PlcPlatform::S7_300.to_string()),
            ],
        ),
        (
            RecipeError::S7AddressWidthMismatch {
                tag: tag(),
                address: address(),
                width: S7Width::W,
            },
            vec![
                ("tag", tag().to_string()),
                ("address", address().to_string()),
                ("width", S7Width::W.to_string()),
            ],
        ),
        (
            RecipeError::ValueOutOfLimit {
                tag: tag(),
                leaf: leaf(),
                value: "150".to_owned(),
                min: "0".to_owned(),
                max: "100".to_owned(),
            },
            vec![
                ("tag", tag().to_string()),
                ("leaf", leaf().to_string()),
                ("value", "150".to_owned()),
                ("min", "0".to_owned()),
                ("max", "100".to_owned()),
            ],
        ),
        (
            RecipeError::ValueShapeMismatch {
                tag: tag(),
                leaf: leaf(),
                expected: "true or false".to_owned(),
            },
            vec![
                ("tag", tag().to_string()),
                ("leaf", leaf().to_string()),
                ("expected", "true or false".to_owned()),
            ],
        ),
        (
            RecipeError::LimitOutsideTypeRange {
                tag: tag(),
                leaf: leaf(),
            },
            vec![("tag", tag().to_string()), ("leaf", leaf().to_string())],
        ),
        (
            RecipeError::LeafPathNotFound {
                tag: tag(),
                leaf: leaf(),
            },
            vec![("tag", tag().to_string()), ("leaf", leaf().to_string())],
        ),
        (
            RecipeError::MinGreaterThanMax {
                tag: tag(),
                leaf: leaf(),
                min: "10".to_owned(),
                max: "5".to_owned(),
            },
            vec![
                ("tag", tag().to_string()),
                ("leaf", leaf().to_string()),
                ("min", "10".to_owned()),
                ("max", "5".to_owned()),
            ],
        ),
        (
            RecipeError::LimitNotApplicable {
                tag: tag(),
                leaf: leaf(),
            },
            vec![("tag", tag().to_string()), ("leaf", leaf().to_string())],
        ),
        (
            RecipeError::TypeNotOnPlatform {
                tag: tag(),
                type_name: "WSTRING(10)".to_owned(),
                platform: PlcPlatform::S7_300,
            },
            vec![
                ("tag", tag().to_string()),
                ("type_name", "WSTRING(10)".to_owned()),
                ("platform", PlcPlatform::S7_300.to_string()),
            ],
        ),
        (
            RecipeError::LimitMissing {
                tag: tag(),
                leaf: leaf(),
            },
            vec![("tag", tag().to_string()), ("leaf", leaf().to_string())],
        ),
        (
            RecipeError::TypeRevisionOutdated {
                tag: tag(),
                type_name: zone(),
                current: 1,
                newest: 2,
            },
            vec![
                ("tag", tag().to_string()),
                ("type_name", zone().to_string()),
                ("current", "1".to_owned()),
                ("newest", "2".to_owned()),
            ],
        ),
    ]
}

/// Samples of the rebind errors.
fn rebind_samples() -> Vec<Sample> {
    vec![
        (
            RecipeError::RebindNeedsValue {
                tag: tag(),
                leaf: leaf(),
            },
            vec![("tag", tag().to_string()), ("leaf", leaf().to_string())],
        ),
        (
            RecipeError::RebindDropsLimit {
                tag: tag(),
                leaf: leaf(),
            },
            vec![("tag", tag().to_string()), ("leaf", leaf().to_string())],
        ),
        (
            RecipeError::RebindTargetInvalid {
                type_name: "ST_Zone@2".to_owned(),
                reason: RebindTargetReason::NotNewer,
            },
            vec![
                ("type_name", "ST_Zone@2".to_owned()),
                ("reason", RebindTargetReason::NotNewer.to_string()),
            ],
        ),
    ]
}

/// Return one sample of each variant of `RecipeError` that a domain row names.
fn samples() -> Vec<Sample> {
    let mut all = recipe_samples();
    all.extend(parameter_samples());
    all.extend(rebind_samples());
    all
}

/// Assert that every `E-RCP` code below `E-RCP-030` or in `E-RCP-050..069` has a doc row.
fn assert_registry_codes_have_rows(rows: &[DocRow]) {
    for entry in REGISTRY {
        let text = entry.as_str();
        let Some(number) = text
            .strip_prefix("E-RCP-")
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        if number < 30 || (50..=69).contains(&number) {
            assert!(
                rows.iter().any(|row| row.code == text),
                "{text} has no row in reliability.md section 1.2"
            );
        }
    }
}

#[test]
fn registry_matches_reliability() {
    let rows = doc_rows();
    assert_eq!(rows.len(), DOC_ROW_COUNT);

    let samples = samples();
    assert_eq!(samples.len(), DOC_ROW_COUNT);

    let mut sample_codes: BTreeSet<&str> = BTreeSet::new();
    for (error, fields) in &samples {
        let code = error.code().as_str();
        let row = rows
            .iter()
            .find(|row| row.code == code)
            .expect("the code of a sample has a doc row");
        assert_eq!(variant_name(error), row.variant, "{code}");
        let text = fill(&row.message, fields);
        assert!(!text.contains('{'), "{code} has an unfilled field");
        assert_eq!(error.to_string(), text, "{code}");
        assert!(
            REGISTRY.iter().any(|entry| entry.as_str() == code),
            "{code} is not in REGISTRY"
        );
        assert!(sample_codes.insert(code), "{code} has two samples");
    }

    let doc_codes: BTreeSet<&str> = rows.iter().map(|row| row.code.as_str()).collect();
    assert_eq!(sample_codes, doc_codes);
    assert_registry_codes_have_rows(&rows);
}
