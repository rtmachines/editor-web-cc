//! Tests of `DiffVersions` with stored rows that do not fit their types (ADR-0050 (c), design D9).
//!
//! Every write path checks a value and a limit row, so such a row is corrupt data. The diff stops
//! with `E-DB-008` and shows no row with the stored text.

use std::collections::BTreeSet;
use std::error::Error as _;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    MemberName, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember,
    TypeName,
};
use recipe_domain::recipe::{StoredLimit, StoredParameter, StoredValue};
use serde_json::json;

use super::{VersionDiff, copy, first_page, input, run, seed, world};
use crate::error::AppError;
use crate::memory::fixtures::{limit, mid, param, tid};
use crate::memory::seed_type_id;

/// Assert that `result` is `E-DB-008`, and not a page with a row.
fn assert_db_008(result: &Result<VersionDiff, AppError>) {
    assert_eq!(
        result.as_ref().err().map(AppError::code),
        Some(ErrorCode::DB_008),
        "{result:?}"
    );
}

/// A `TIME` parameter `HOLD` without a limit and with the stored count `count`.
fn hold(n: u128, count: i64) -> StoredParameter {
    StoredParameter {
        plc_type: seed_type_id(Primitive::Time, None).unwrap(),
        value: StoredValue::from(json!(count)),
        limits: Vec::new(),
        ..param(n, 101, "HOLD")
    }
}

/// Return a copy of `parameter` with the stored count `count`.
fn with_count(parameter: &StoredParameter, count: i64) -> StoredParameter {
    StoredParameter {
        value: StoredValue::from(json!(count)),
        ..copy(parameter)
    }
}

/// Run the diff of version 1 with `old` and version 2 with `new`.
async fn diff_of(
    old: &[StoredParameter],
    new: &[StoredParameter],
) -> Result<VersionDiff, AppError> {
    let fixture = world();
    seed(&fixture, old, new);
    run(&fixture, input(first_page())).await
}

#[tokio::test]
async fn a_stored_time_count_that_does_not_fit_gives_db_008_and_no_raw_text() {
    let old = hold(1, 5_000);
    let broken = with_count(&old, -1_000);

    assert_db_008(&diff_of(&[old], &[broken]).await);
}

#[tokio::test]
async fn a_stored_time_count_that_does_not_fit_keeps_the_domain_fault_as_its_source() {
    let old = hold(1, 5_000);
    let broken = with_count(&old, -1_000);

    let error = diff_of(&[old], &[broken]).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(!error.to_string().contains("HOLD"), "{error}");
    let chain: Vec<String> = std::iter::successors(error.source(), |&cause| cause.source())
        .map(ToString::to_string)
        .collect();
    assert!(chain.iter().any(|text| text.contains("HOLD")), "{chain:?}");
}

#[tokio::test]
async fn a_stored_time_count_of_the_old_version_that_does_not_fit_gives_db_008() {
    let broken = hold(1, -1_000);
    let new = with_count(&broken, 5_000);

    assert_db_008(&diff_of(&[broken], &[new]).await);
}

#[tokio::test]
async fn an_added_or_a_removed_parameter_that_does_not_fit_gives_db_008() {
    let broken = hold(1, -1_000);

    assert_db_008(&diff_of(&[], std::slice::from_ref(&broken)).await);
    assert_db_008(&diff_of(&[broken], &[]).await);
}

#[tokio::test]
async fn a_stored_time_bound_that_does_not_fit_gives_db_008_and_no_count() {
    let bound = |min: i64| StoredLimit {
        min: Some(min.into()),
        max: Some(60_000_i64.into()),
        ..limit()
    };
    let old = StoredParameter {
        limits: vec![bound(1_000)],
        ..hold(1, 5_000)
    };
    let broken = StoredParameter {
        limits: vec![bound(-1_000)],
        ..copy(&old)
    };

    assert_db_008(&diff_of(&[old], &[broken]).await);
}

#[tokio::test]
async fn a_time_leaf_of_a_struct_that_does_not_fit_gives_db_008_and_no_raw_text() {
    let fixture = world();
    let time = seed_type_id(Primitive::Time, None).unwrap();
    fixture
        .store
        .seed_type(&struct_type(903, "ST_Hold", &[("hold", time)]));
    let old = StoredParameter {
        plc_type: tid(903),
        value: StoredValue::from(json!({"hold": 5_000})),
        ..hold(1, 0)
    };
    let broken = StoredParameter {
        value: StoredValue::from(json!({"hold": -1_000})),
        ..copy(&old)
    };
    seed(&fixture, &[old], &[broken]);

    assert_db_008(&run(&fixture, input(first_page())).await);
}

/// The struct `name` revision 1 on `M1` with the members `members`.
fn struct_type(id: u128, name: &str, members: &[(&str, PlcTypeId)]) -> PlcType {
    let members = members
        .iter()
        .map(|(member, type_id)| TypeMember {
            name: MemberName::parse(member).unwrap(),
            type_id: *type_id,
        })
        .collect();
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members,
        },
    })
    .unwrap()
}
