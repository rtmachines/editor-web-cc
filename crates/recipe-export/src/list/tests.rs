use recipe_domain::export::{ExportableRecipe, ExportableRecipes, Sha256};
use recipe_domain::plant::{MachineCode, PlcPlatform};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::Value;
use time::OffsetDateTime;

use super::write_list;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// 2026-09-30T11:00:05.75Z: the writer drops the fraction.
fn approved_instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_790_766_005)
        .unwrap()
        .replace_nanosecond(750_000_000)
        .unwrap()
}

fn recipe(reference: &str, number: u32) -> ExportableRecipe {
    ExportableRecipe {
        reference: ReferenceNumber::parse(reference).unwrap(),
        name: Text::parse("Name \u{17c}\"q\"").unwrap(),
        version_number: number,
        approved_at: Timestamp::from(approved_instant()),
        content_sha256: Sha256::parse(HASH).unwrap(),
    }
}

fn list(archived: bool, recipes: Vec<ExportableRecipe>) -> ExportableRecipes {
    ExportableRecipes {
        code: MachineCode::parse("M12").unwrap(),
        platform: PlcPlatform::S7_300,
        archived,
        recipes,
    }
}

fn written(list: &ExportableRecipes) -> String {
    String::from_utf8(write_list(list).unwrap()).unwrap()
}

#[test]
fn list_body_has_the_exact_bytes_of_the_pretty_object() {
    let body = written(&list(false, vec![recipe("R-100", 3)]));

    let expected = format!(
        r#"{{
  "machine": {{
    "code": "M12",
    "plc_platform": "S7_300",
    "archived": false
  }},
  "recipes": [
    {{
      "reference_number": "R-100",
      "name": "Name ż\"q\"",
      "version_number": 3,
      "approved_at": "2026-09-30T11:00:05Z",
      "content_sha256": "{HASH}"
    }}
  ]
}}"#
    );
    assert_eq!(body, expected);
}

#[test]
fn list_body_keeps_the_field_archived_true_for_an_archived_machine() {
    let value: Value = serde_json::from_str(&written(&list(true, vec![]))).unwrap();

    assert_eq!(value["machine"]["archived"], Value::Bool(true));
}

#[test]
fn list_body_of_a_machine_without_recipes_has_an_empty_array() {
    let value: Value = serde_json::from_str(&written(&list(false, vec![]))).unwrap();

    assert_eq!(value["recipes"], Value::Array(vec![]));
}

#[test]
fn list_body_keeps_the_order_of_the_rows() {
    let rows = vec![recipe("A1", 1), recipe("B1", 2), recipe("C1", 3)];
    let value: Value = serde_json::from_str(&written(&list(false, rows))).unwrap();

    let references: Vec<&str> = value["recipes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["reference_number"].as_str().unwrap())
        .collect();
    assert_eq!(references, ["A1", "B1", "C1"]);
}

#[test]
fn list_body_has_no_newline_at_the_end() {
    assert!(!written(&list(false, vec![recipe("R1", 1)])).ends_with('\n'));
}
