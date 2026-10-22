//! Tests of the lines of `WebError::Fields` (task 7.1, design D9, ADR-0041).

use recipe_app::ports::NonEmpty;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, MachineCode, PlantError, UnitId};
use recipe_domain::recipe::{ParameterTag, RecipeError, RecipeErrors, SnapshotError, SnapshotItem};
use uuid::Uuid;

use super::lists::{FieldError, item_lines, leaf_lines, line_view};
use super::view::{field_key, has_input, input_id};
use super::{FieldReason, RowField, WebError};
use crate::i18n::{Bundles, Lang};

/// The field ids of the recipe forms of design D6 that have an input with that `id`.
const D6_INPUT_IDS: [&str; 16] = [
    "recipe_q",
    "recipe_machine",
    "recipe_status",
    "show_deleted",
    "machine_code",
    "reference_number",
    "recipe_name",
    "change_note",
    "based_on",
    "unit_path",
    "tag",
    "name_en",
    "description_en",
    "type",
    "s7_address",
    "ordinal",
];

fn tag(text: &str) -> ParameterTag {
    ParameterTag::parse(text).unwrap()
}

fn leaf(text: &str) -> LeafPath {
    LeafPath::parse(text).unwrap()
}

/// The fault `E-RCP-004` of the leaf `path` of `SPEED_TABLE` with the value `value`.
fn out_of_limit(path: &str, value: &str) -> RecipeError {
    RecipeError::ValueOutOfLimit {
        tag: tag("SPEED_TABLE"),
        leaf: leaf(path),
        value: value.to_owned(),
        min: "0".to_owned(),
        max: "3000".to_owned(),
    }
}

/// The input id of a leaf: `value-` and the index of the leaf. A path `[n]` gives `value-n`.
fn value_input(path: &LeafPath) -> Option<String> {
    let text = path.to_string();
    let index = text.strip_prefix('[')?.strip_suffix(']')?;
    Some(format!("value-{index}"))
}

fn message(line: &FieldError, lang: Lang) -> String {
    let bundles = Bundles::load().unwrap();
    line_view(line, &bundles, lang).message
}

#[test]
fn lists_fields_have_the_code_of_the_first_line() {
    let errors = RecipeErrors::from(out_of_limit("[3]", "3500"));
    let mut lines = leaf_lines(&errors, &value_input);
    lines.push(FieldError::bad_request("tag", FieldReason::Tag));
    let error = WebError::Fields {
        lines: NonEmpty::try_from(lines).unwrap(),
        form: None,
        failure: None,
    };
    assert_eq!(error.code(), ErrorCode::RCP_004);
    let first_web = WebError::Fields {
        lines: NonEmpty::new(FieldError::bad_request("tag", FieldReason::Tag)),
        form: None,
        failure: None,
    };
    assert_eq!(first_web.code(), ErrorCode::WEB_008);
}

#[test]
fn lists_the_display_of_fields_holds_the_english_text_of_each_line() {
    let errors = RecipeErrors::from_vec(vec![
        out_of_limit("[3]", "3500"),
        out_of_limit("[6]", "4000"),
    ])
    .unwrap();
    let lines = NonEmpty::try_from(leaf_lines(&errors, &value_input)).unwrap();
    let text = WebError::Fields {
        lines,
        form: None,
        failure: None,
    }
    .to_string();
    assert!(text.contains("Value 3500 of SPEED_TABLE[3]"), "{text}");
    assert!(text.contains("Value 4000 of SPEED_TABLE[6]"), "{text}");
}

#[test]
fn lists_the_log_text_of_100_lines_holds_the_count_and_only_the_first_20_lines() {
    let entries = (1..=100)
        .map(|index| out_of_limit(&format!("[{index}]"), "3500"))
        .collect();
    let errors = RecipeErrors::from_vec(entries).unwrap();
    let lines = NonEmpty::try_from(leaf_lines(&errors, &value_input)).unwrap();
    let text = WebError::Fields {
        lines,
        form: None,
        failure: None,
    }
    .to_string();
    assert_eq!(text.lines().count(), 21, "{text}");
    assert_eq!(
        text.lines().next(),
        Some("100 errors. The first 20 follow.")
    );
    assert!(text.contains("SPEED_TABLE[20] "), "{text}");
    assert!(!text.contains("SPEED_TABLE[21] "), "{text}");
}

#[test]
fn lists_a_bad_request_line_names_the_field_and_the_reason_and_links_to_its_input() {
    let line = FieldError::bad_request("tag", FieldReason::Tag);
    assert_eq!(line.code(), ErrorCode::WEB_008);
    assert_eq!(line.target(), Some("tag"));
    assert_eq!(
        message(&line, Lang::En),
        "Field Tag is not valid: a tag has 1 to 64 characters from A to Z, 0 to 9, and underscore. It starts with a letter."
    );
    assert_eq!(
        message(&line, Lang::Pl),
        "Pole Znacznik jest niepoprawne: znacznik ma od 1 do 64 znak\u{f3}w z zakresu A do Z, 0 do 9 i podkre\u{15b}lenie. Zaczyna si\u{119} liter\u{105}."
    );
}

#[test]
fn lists_a_bad_request_of_the_whole_form_has_no_target() {
    let line = FieldError::bad_request("form", FieldReason::Form);
    assert_eq!(line.target(), None);
    assert_eq!(line.code(), ErrorCode::WEB_008);
}

#[test]
fn lists_a_bad_row_line_names_the_row_and_links_to_the_input_of_the_row() {
    let line = FieldError::bad_row(2, RowField::Type, FieldReason::MemberRows);
    assert_eq!(line.code(), ErrorCode::WEB_008);
    assert_eq!(line.target(), Some("members-2-type"));
    let english = message(&line, Lang::En);
    assert!(
        english.starts_with("Field Type of member 3 is not valid: "),
        "{english}"
    );
    let polish = message(&line, Lang::Pl);
    assert!(
        polish.starts_with("Pole Typ sk\u{142}adnika 3 jest niepoprawne: "),
        "{polish}"
    );
}

#[test]
fn lists_a_bad_input_line_takes_its_label_key_and_its_input_id() {
    let line = FieldError::bad_input(
        "field-value-leaf",
        vec![("leaf", "[3]".to_owned())],
        FieldReason::ValueKey,
        "value-3".to_owned(),
    );
    assert_eq!(line.target(), Some("value-3"));
    let english = message(&line, Lang::En);
    assert!(
        english.starts_with("Field Value [3] is not valid: each value field"),
        "{english}"
    );
}

#[test]
fn lists_each_entry_of_recipe_errors_gives_one_line_in_order_with_its_leaf_input() {
    let errors = RecipeErrors::from_vec(vec![
        out_of_limit("[3]", "3500"),
        out_of_limit("[6]", "4000"),
        RecipeError::DuplicateTag {
            tag: tag("SPEED_TABLE"),
            number: 3,
        },
    ])
    .unwrap();
    let lines = leaf_lines(&errors, &value_input);
    let targets: Vec<Option<&str>> = lines.iter().map(FieldError::target).collect();
    assert_eq!(targets, [Some("value-3"), Some("value-6"), None]);
    let codes: Vec<ErrorCode> = lines.iter().map(FieldError::code).collect();
    assert_eq!(
        codes,
        [ErrorCode::RCP_004, ErrorCode::RCP_004, ErrorCode::RCP_014]
    );
}

#[test]
fn lists_a_leaf_line_names_the_tag_and_the_leaf_in_both_languages() {
    let errors = RecipeErrors::from(out_of_limit("[3]", "3500"));
    let lines = leaf_lines(&errors, &value_input);
    let line = lines.first().unwrap();
    assert_eq!(
        message(line, Lang::En),
        "Value 3500 of SPEED_TABLE[3] is outside the process limit 0 to 3000."
    );
    assert_eq!(
        message(line, Lang::Pl),
        "Warto\u{15b}\u{107} 3500 parametru SPEED_TABLE[3] jest poza limitem procesowym od 0 do 3000."
    );
}

#[test]
fn lists_a_leaf_without_an_input_gives_a_line_without_target() {
    let errors = RecipeErrors::from(out_of_limit("[3]", "3500"));
    let lines = leaf_lines(&errors, &|_path: &LeafPath| None);
    assert_eq!(lines.first().unwrap().target(), None);
}

/// The items of a failed snapshot check: a parameter, the version, and an unknown unit.
fn items() -> Vec<SnapshotItem> {
    vec![
        SnapshotItem::Parameter {
            tag: tag("SPEED"),
            error: SnapshotError::Recipe(RecipeError::LimitMissing {
                tag: tag("SPEED"),
                leaf: leaf("[2]"),
            }),
        },
        SnapshotItem::Version {
            error: SnapshotError::Plant(PlantError::MachineArchived {
                code: MachineCode::parse("M7").unwrap(),
            }),
        },
        SnapshotItem::UnknownUnit {
            tag: tag("HOLD"),
            unit: UnitId::from(Uuid::nil()),
        },
    ]
}

fn row_of(tag: &ParameterTag) -> Option<String> {
    Some(format!("parameter-{tag}"))
}

#[test]
fn lists_each_item_of_e_rcp_020_gives_one_line_with_its_own_code() {
    let lines = item_lines(&items(), &row_of);
    let codes: Vec<ErrorCode> = lines.iter().map(FieldError::code).collect();
    assert_eq!(
        codes,
        [ErrorCode::RCP_016, ErrorCode::PLT_010, ErrorCode::PLT_020]
    );
    let targets: Vec<Option<&str>> = lines.iter().map(FieldError::target).collect();
    assert_eq!(
        targets,
        [Some("parameter-SPEED"), None, Some("parameter-HOLD")]
    );
}

#[test]
fn lists_an_item_line_names_the_parameter_and_the_leaf() {
    let lines = item_lines(&items(), &row_of);
    let bundles = Bundles::load().unwrap();
    let views: Vec<_> = lines
        .iter()
        .map(|line| line_view(line, &bundles, Lang::En))
        .collect();
    let first = views.first().unwrap();
    assert_eq!(
        first.message,
        "SPEED[2] has no process limit. Set a limit before approval."
    );
    assert_eq!(first.subject, "");
    let version = views.get(1).unwrap();
    assert_eq!(version.message, "Machine M7 is archived.");
    assert_eq!(version.subject, "");
    let unit = views.get(2).unwrap();
    assert_eq!(unit.subject, "HOLD");
    assert_eq!(
        unit.message,
        format!(
            "Unit {} does not exist on this machine. Select an existing unit.",
            Uuid::nil()
        )
    );
}

#[test]
fn lists_a_line_view_has_the_code_and_the_target_of_its_line() {
    let bundles = Bundles::load().unwrap();
    let line = FieldError::bad_request("tag", FieldReason::Tag);
    let view = line_view(&line, &bundles, Lang::En);
    assert_eq!(view.code, "E-WEB-008");
    assert_eq!(view.target, "tag");
    let whole = FieldError::bad_request("form", FieldReason::Form);
    assert_eq!(line_view(&whole, &bundles, Lang::En).target, "");
}

#[test]
fn lists_every_field_id_of_design_d6_has_an_input_and_a_label_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for id in D6_INPUT_IDS {
        assert!(has_input(id), "{id}");
        assert_eq!(input_id(id), id);
        for lang in [Lang::Pl, Lang::En] {
            let key = field_key(id);
            assert!(bundles.message(lang, &key, None).is_some(), "{key} {lang}");
        }
    }
}
