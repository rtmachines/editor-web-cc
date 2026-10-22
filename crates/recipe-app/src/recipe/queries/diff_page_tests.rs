//! Tests of the shared page core of `DiffVersions` and `GetApprovalPreview` (task 6.2, design D4).

use async_trait::async_trait;
use recipe_domain::plant::{PlatformTypes, PlcPlatform, TypeTrees, UnitPath};
use recipe_domain::recipe::{
    DiffKind, Ordinal, Parameter, ParameterKey, ParameterTag, RecipeVersionId,
};

use super::diff::DiffChoice;
use super::diff_page::{build_page, read_page};
use super::diff_tests::{kinds, section_keys, seed};
use crate::error::StorageError;
use crate::memory::fixtures::{Fixture, param, pkey, vid};
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
