//! Tests of the shared page core of `DiffVersions` and `GetApprovalPreview` (task 6.2, design D4).

use std::collections::BTreeSet;

use async_trait::async_trait;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    LeafPath, PlatformTypes, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape,
    TypeTrees, UnitPath,
};
use recipe_domain::recipe::{
    DiffKind, Ordinal, Parameter, ParameterKey, ParameterTag, RecipeVersionId, RuleField,
    StoredLimit, StoredParameter, StoredValue,
};
use serde_json::json;

use super::diff::{DiffChoice, DiffPage};
use super::diff_page::{build_page, read_page};
use super::diff_tests::{copy, kinds, section_keys, seed};
use crate::error::{AppError, StorageError};
use crate::memory::fixtures::{Fixture, limit, param, pkey, tid, vid};
use crate::ports::{ChangeFlags, DiffReads, DiffSides, Isolation, Transaction};

/// The live draft version 2 of `R-1`.
const NEW: u128 = 102;

#[tokio::test]
async fn a_first_approval_gives_every_parameter_as_added_with_its_limit_rows() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    seed(&fixture, &[], &[param(1, 101, "A"), param(2, 101, "B")]);
    let mut tx = fixture
        .store
        .begin_memory(Isolation::RepeatableRead)
        .unwrap();
    let choice = DiffChoice {
        kind: None,
        page: 0,
    };

    let read = read_page(tx.diff_reads(), (None, vid(NEW)), true, (choice, 50))
        .await
        .unwrap();
    let rows = tx.types().type_rows(machine.id()).await.unwrap();
    drop(tx);
    let types = PlatformTypes::collect(machine.id(), PlcPlatform::TwinCat3, rows).unwrap();
    let old: Vec<Parameter> = read.sides.old.into_iter().map(Parameter::from).collect();
    let new: Vec<Parameter> = read.sides.new.into_iter().map(Parameter::from).collect();
    let trees = TypeTrees::resolve(new.iter().map(Parameter::plc_type), &types);
    let page = build_page(
        read.plan,
        read.summary,
        (&old, &new),
        (&machine, &types, &trees),
        1_000,
    )
    .unwrap();

    assert_eq!(kinds(&page), vec![DiffKind::Limits, DiffKind::Added]);
    assert_eq!(
        section_keys(&page, DiffKind::Limits),
        vec![pkey(1), pkey(2)]
    );
    assert_eq!(section_keys(&page, DiffKind::Added), vec![pkey(1), pkey(2)]);
    assert_eq!(page.summary.added, 2);
    assert_eq!(page.summary.limit_rows, 2);
    assert!(old.is_empty());
}

/// A `DiffReads` double with fixed flags. It records the keys of each call of `pairs`.
struct Recording {
    flags: Vec<ChangeFlags>,
    asked: Vec<Vec<ParameterKey>>,
}

#[async_trait]
impl DiffReads for Recording {
    async fn change_flags(
        &mut self,
        _old: Option<RecipeVersionId>,
        _new: RecipeVersionId,
    ) -> Result<Vec<ChangeFlags>, StorageError> {
        Ok(self.flags.clone())
    }

    async fn pairs(
        &mut self,
        _old: Option<RecipeVersionId>,
        _new: Option<RecipeVersionId>,
        keys: &[ParameterKey],
    ) -> Result<DiffSides, StorageError> {
        self.asked.push(keys.to_vec());
        Ok(DiffSides::default())
    }
}

/// The flags of key `n` with a changed value. `limits` also puts it into the limits section.
fn changed_value(n: u128, limits: bool) -> ChangeFlags {
    ChangeFlags {
        key: pkey(n),
        unit_path: UnitPath::parse("U1").unwrap(),
        ordinal: Ordinal::parse(0).unwrap(),
        tag: ParameterTag::parse(&format!("P{n}")).unwrap(),
        definition: false,
        value: true,
        limits,
        translations: false,
        only_old: false,
        only_new: false,
        limit_rows: u32::from(limits),
        definition_rows: 0,
        translation_rows: 0,
    }
}

#[tokio::test]
async fn only_the_distinct_keys_of_the_page_go_to_pairs() {
    let mut reads = Recording {
        flags: vec![
            changed_value(1, true),
            changed_value(2, false),
            changed_value(3, false),
        ],
        asked: Vec::new(),
    };
    let choice = DiffChoice {
        kind: None,
        page: 0,
    };

    // The page of 3 entries holds limits 1, values 1, and values 2.
    read_page(&mut reads, (Some(vid(101)), vid(NEW)), true, (choice, 3))
        .await
        .unwrap();

    assert_eq!(reads.asked, vec![vec![pkey(1), pkey(2)]]);
}

/// The type identifier of the `DATE` row of the rule fault tests.
const DATE_TYPE: u128 = 910;

/// The stored `DATE` count of `D#2169-01-17`. Only the earlier rule set allows it on `S7_1500`.
const EARLIER_ONLY_DAY: i64 = 72_700;

/// A stored `DATE` count that fits every rule set.
const FIRST_DAY: i64 = 10_000;

/// A stored `DATE` count above the end of the earlier rule set. It is a storage defect.
const BEYOND_EARLIER: i64 = 72_900;

/// The `DATE` row on the S7 platforms. `S7_300` keeps the end 72 683 in both rule sets.
fn date_type() -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(DATE_TYPE),
        machine: None,
        platforms: BTreeSet::from([
            PlcPlatform::S7_300,
            PlcPlatform::S7_1200,
            PlcPlatform::S7_1500,
        ]),
        shape: RecordShape::Primitive {
            primitive: Primitive::Date,
            leaf_class: Primitive::Date.leaf_class(),
            size_bytes: Primitive::Date.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap()
}

/// A `DATE` parameter with the stored count `count` and the upper bound `max`, if any.
fn day(count: i64, max: Option<i64>) -> StoredParameter {
    let limits = max.map(|bound| StoredLimit {
        min: None,
        max: Some(bound.into()),
        ..limit()
    });
    StoredParameter {
        plc_type: tid(DATE_TYPE),
        value: StoredValue::from(json!(count)),
        limits: limits.into_iter().collect(),
        ..param(1, 101, "DAY")
    }
}

/// Build the diff page of version 1 with `old` and version 2 with `new` on an `S7_1500` machine.
async fn s7_1500_page(old: &StoredParameter, new: &StoredParameter) -> Result<DiffPage, AppError> {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture.store.seed_type(&date_type());
    seed(
        &fixture,
        std::slice::from_ref(old),
        std::slice::from_ref(new),
    );
    let mut tx = fixture
        .store
        .begin_memory(Isolation::RepeatableRead)
        .unwrap();
    let choice = DiffChoice {
        kind: None,
        page: 0,
    };
    let read = read_page(
        tx.diff_reads(),
        (Some(vid(101)), vid(NEW)),
        true,
        (choice, 50),
    )
    .await?;
    let rows = tx.types().type_rows(machine.id()).await?;
    drop(tx);
    let types = PlatformTypes::collect(machine.id(), PlcPlatform::S7_1500, rows).unwrap();
    let old: Vec<Parameter> = read.sides.old.into_iter().map(Parameter::from).collect();
    let new: Vec<Parameter> = read.sides.new.into_iter().map(Parameter::from).collect();
    let trees = TypeTrees::resolve(old.iter().chain(&new).map(Parameter::plc_type), &types);
    build_page(
        read.plan,
        read.summary,
        (&old, &new),
        (&machine, &types, &trees),
        1_000,
    )
}

#[tokio::test]
async fn a_stored_limit_of_the_earlier_set_only_gives_the_row_the_rule_fault_e_rcp_006() {
    let old = day(FIRST_DAY, Some(20_000));
    let new = copy(&day(FIRST_DAY, Some(EARLIER_ONLY_DAY)));

    let page = s7_1500_page(&old, &new).await.unwrap();

    assert_eq!(section_keys(&page, DiffKind::Limits), vec![pkey(1)]);
    let faults = page.faults_of(pkey(1)).unwrap();
    assert!(faults.old.is_empty(), "{faults:?}");
    let found: Vec<(ErrorCode, RuleField)> = faults
        .new
        .iter()
        .map(|fault| (fault.code(), fault.field().clone()))
        .collect();
    let whole = LeafPath::parse("").unwrap();
    assert_eq!(found, [(ErrorCode::RCP_006, RuleField::LimitMax(whole))]);
}

#[tokio::test]
async fn a_stored_count_above_the_earlier_set_still_gives_db_008() {
    let old = day(FIRST_DAY, None);
    let new = copy(&day(BEYOND_EARLIER, None));

    let result = s7_1500_page(&old, &new).await;

    assert_eq!(
        result.as_ref().err().map(AppError::code),
        Some(ErrorCode::DB_008),
        "{result:?}"
    );
}
