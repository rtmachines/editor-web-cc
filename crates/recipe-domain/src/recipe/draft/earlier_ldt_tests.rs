//! Tests of `Draft::set_value` and `Draft::replace_limits` for `LDT` and `LDATE` (ADR-0139).
//!
//! On `CodesysIec` and `TwinCat3`, the current set ends `LDT` and `LDATE` at 2^63-1 nanoseconds.
//! The earlier set (ADR-0139 (e)) ends them at 2^64-1. A stored limit of the earlier set must
//! not block a new value of the current set (ADR-0139 (f)).

use rust_decimal::Decimal;
use serde_json::json;

use super::add::support::{draft_with, limit_row, snapshot};
use super::support::{key, stored_parameter};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{id, machine, primitive_row, types_for};
use crate::plant::{LeafPath, PlcPlatform, Primitive, TypeTrees};
use crate::recipe::{Draft, LimitList, StoredLimit, ValueInput};

/// The type identifier of the `LDT` row of these tests.
const LDT: u128 = 8;

/// The type identifier of the `LDATE` row of these tests.
const LDATE: u128 = 9;

/// The platforms on which the range of ADR-0139 applies.
const PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

/// The last `LDT` count of the current set: `LDT#2262-04-11-23:47:16.854775807`.
const LAST_LDT: i64 = 9_223_372_036_854_775_807;

/// The count of `LD#2262-04-11`, the last `LDATE` day of the current set.
const LAST_LDATE: i64 = 9_223_286_400_000_000_000;

/// The count of `LDT#2300-01-01-00:00:00`. The earlier set allows it. The current set refuses it.
const EARLIER_ONLY_LDT: &str = "10413792000000000000";

/// The count of `LDT#2026-10-12-12:30:00`.
const NEW_LDT: i64 = 1_791_808_200_000_000_000;

/// The trees of one `LDT` type and one `LDATE` type on `platform`.
fn trees(platform: PlcPlatform) -> TypeTrees {
    let rows = vec![
        primitive_row(LDT, Primitive::Ldt),
        primitive_row(LDATE, Primitive::Ldate),
    ];
    TypeTrees::resolve([id(LDT), id(LDATE)], &types_for(machine(), platform, rows))
}

/// A live draft on `platform` with `STAMP` of type `LDT`, stored value 0, and the stored `max`.
///
/// The key of `STAMP` derives from 1.
fn stamp_draft(platform: PlcPlatform, stored_max: Option<&str>) -> Draft {
    let mut stored = stored_parameter(1, "STAMP", id(LDT), None);
    if let Some(max) = stored_max {
        stored.limits = vec![StoredLimit {
            leaf_path: LeafPath::parse("").unwrap(),
            min: None,
            max: Some(max.parse::<Decimal>().unwrap()),
            max_length: None,
        }];
    }
    draft_with(platform, vec![stored])
}

/// A live draft on `platform` with `DUE` of type `LDATE` and stored value 0. Its key derives from 1.
fn due_draft(platform: PlcPlatform) -> Draft {
    draft_with(platform, vec![stored_parameter(1, "DUE", id(LDATE), None)])
}

fn json_text(text: &str) -> ValueInput {
    ValueInput::Json(format!("\"{text}\""))
}

#[test]
fn set_value_stores_the_last_ldt_instant_on_both_platforms() {
    for platform in PLATFORMS {
        let mut draft = stamp_draft(platform, None);
        let change = draft
            .set_value(
                key(1),
                &json_text("LDT#2262-04-11-23:47:16.854775807"),
                &trees(platform),
            )
            .unwrap();
        assert_eq!(
            change.after().unwrap().value(),
            &json!(LAST_LDT).into(),
            "{platform:?}"
        );
    }
}

#[test]
fn set_value_stores_the_last_ldate_day_on_both_platforms() {
    for platform in PLATFORMS {
        let mut draft = due_draft(platform);
        let change = draft
            .set_value(key(1), &json_text("LD#2262-04-11"), &trees(platform))
            .unwrap();
        assert_eq!(
            change.after().unwrap().value(),
            &json!(LAST_LDATE).into(),
            "{platform:?}"
        );
    }
}

#[test]
fn set_value_with_the_instant_after_the_last_ldt_gives_e_rcp_005() {
    for platform in PLATFORMS {
        let mut draft = stamp_draft(platform, None);
        let before = snapshot(&draft);
        let error = draft
            .set_value(
                key(1),
                &json_text("LDT#2262-04-11-23:47:16.854775808"),
                &trees(platform),
            )
            .expect_err("the instant is after the range");
        assert_eq!(error.code(), ErrorCode::RCP_005, "{platform:?}: {error}");
        assert_eq!(
            snapshot(&draft),
            before,
            "{platform:?}: a failed command changed the draft"
        );
    }
}

#[test]
fn set_value_with_the_day_after_the_last_ldate_gives_e_rcp_005() {
    for platform in PLATFORMS {
        let mut draft = due_draft(platform);
        let before = snapshot(&draft);
        let error = draft
            .set_value(key(1), &json_text("LD#2262-04-12"), &trees(platform))
            .expect_err("the day is after the range");
        assert_eq!(error.code(), ErrorCode::RCP_005, "{platform:?}: {error}");
        assert_eq!(
            snapshot(&draft),
            before,
            "{platform:?}: a failed command changed the draft"
        );
    }
}

#[test]
fn replace_limits_with_a_max_after_the_last_ldt_gives_e_rcp_006() {
    for platform in PLATFORMS {
        let mut draft = stamp_draft(platform, None);
        let before = snapshot(&draft);
        let rows = LimitList::parse(vec![limit_row(
            "",
            None,
            Some("LDT#2262-04-11-23:47:16.854775808"),
        )])
        .unwrap();
        let error = draft
            .replace_limits(key(1), &rows, &trees(platform))
            .expect_err("the max is after the range");
        assert_eq!(error.code(), ErrorCode::RCP_006, "{platform:?}: {error}");
        assert_eq!(
            snapshot(&draft),
            before,
            "{platform:?}: a failed command changed the draft"
        );
    }
}

#[test]
fn a_stored_ldt_limit_of_the_earlier_set_does_not_block_set_value() {
    for platform in PLATFORMS {
        let mut draft = stamp_draft(platform, Some(EARLIER_ONLY_LDT));
        let change = draft
            .set_value(
                key(1),
                &json_text("LDT#2026-10-12-12:30:00"),
                &trees(platform),
            )
            .unwrap();
        let after = change.after().unwrap();
        assert_eq!(after.value(), &json!(NEW_LDT).into(), "{platform:?}");
        let limits = after.limits();
        assert_eq!(limits.len(), 1, "{platform:?}");
        assert_eq!(
            limits[0].max,
            Some(EARLIER_ONLY_LDT.parse::<Decimal>().unwrap()),
            "{platform:?}"
        );
    }
}
