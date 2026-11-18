//! Tests of a row that a lock or a read does not find, although it exists (ADR-0195).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{TypeExpr, UnitPath};
use recipe_domain::recipe::{LimitList, Ordinal, ParameterTag, ValueInput};
use recipe_domain::text::LongText;

use crate::error::AppError;
use crate::memory::MissingRow;
use crate::memory::fixtures::{Fixture, actor, meta, reference, text};
use crate::recipe::test_support::{DRAFT_VERSION, seed_speed_draft, speed_key};
use crate::recipe::{AddParameter, AddParameterInput, SetParameterValue, SetParameterValueInput};

#[derive(Clone, Copy, Debug)]
enum Command {
    AddParameter,
    SetParameterValue,
}

async fn run(fixture: &Fixture, command: Command) -> Result<(), AppError> {
    let engineer = actor(Role::ProcessEngineer);
    match command {
        Command::AddParameter => {
            let request = AddParameterInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                unit_path: UnitPath::parse("U1.2").unwrap(),
                tag: ParameterTag::parse("FEED").unwrap(),
                name_en: text("Feed"),
                description_en: LongText::parse("Feed rate").unwrap(),
                type_expr: TypeExpr::parse("INT").unwrap(),
                s7_address: None,
                ordinal: Ordinal::parse(1).unwrap(),
                value: ValueInput::Json("120".to_owned()),
                limits: LimitList::parse(Vec::new()).unwrap(),
            };
            AddParameter::new(fixture.context())
                .execute(&engineer, &meta(), request)
                .await
                .map(|_| ())
        }
        Command::SetParameterValue => {
            let request = SetParameterValueInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                key: speed_key(),
                value: ValueInput::Json("130".to_owned()),
            };
            SetParameterValue::new(fixture.context())
                .execute(&engineer, &meta(), request)
                .await
                .map(|_| ())
        }
    }
}

#[tokio::test]
async fn a_row_that_a_lookup_misses_gives_a_storage_defect_and_commits_nothing() {
    let cases = [
        (Command::AddParameter, MissingRow::MachineShared),
        (Command::AddParameter, MissingRow::LiveUnits),
        (Command::SetParameterValue, MissingRow::Recipe),
        (Command::SetParameterValue, MissingRow::Draft),
    ];
    for (command, missing) in cases {
        let fixture = Fixture::new();
        seed_speed_draft(&fixture);
        fixture.store.miss_next(missing);
        let error = run(&fixture, command).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::DB_008, "{command:?} {missing:?}");
        assert!(fixture.store.audit_entries().is_empty());
        assert!(
            fixture
                .store
                .state()
                .editors_of(crate::memory::fixtures::vid(DRAFT_VERSION))
                .is_empty()
        );
    }
}
