//! Tests of the export of a version with a parameter that a vendor alias typed (ADR-0206 (f)).

use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform, TypeExpr, UnitPath};
use recipe_domain::recipe::{LimitInput, LimitList, Ordinal, ParameterTag, ValueInput};
use recipe_domain::text::LongText;

use super::test_doubles::{RecordingWriter, port};
use super::tests::approved_input;
use super::{ExportApprovedVersion, ExportedVersion};
use crate::memory::fixtures::{Fixture, actor, meta, reference, second_engineer, text};
use crate::recipe::{AddParameter, AddParameterInput, ApproveVersion, ApproveVersionInput};

/// Add the parameter `START` of the type text `TOD` to a draft, then approve the version.
async fn approved_with_alias() -> Fixture {
    let fixture = Fixture::new();
    fixture.store.seed_time_primitive_types();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::CodesysIec, 1, "M9");
    fixture.seed_draft(&machine, 1, "R-1");
    let added = AddParameter::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            AddParameterInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                unit_path: UnitPath::parse("U1.2").unwrap(),
                tag: ParameterTag::parse("START").unwrap(),
                name_en: text("Start"),
                description_en: LongText::parse("Start of the shift").unwrap(),
                type_expr: TypeExpr::parse("TOD").unwrap(),
                s7_address: None,
                ordinal: Ordinal::parse(0).unwrap(),
                value: ValueInput::Json("\"TOD#06:00:00\"".to_owned()),
                limits: LimitList::parse(vec![LimitInput {
                    leaf_path: LeafPath::parse("").unwrap(),
                    min: Some("TOD#00:00:00".to_owned()),
                    max: Some("TOD#23:59:59".to_owned()),
                    max_length: None,
                }])
                .unwrap(),
            },
        )
        .await
        .unwrap();
    ApproveVersion::new(fixture.context())
        .execute(
            &second_engineer(),
            &meta(),
            ApproveVersionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: added.draft.row_version,
                allow_outdated: false,
            },
        )
        .await
        .unwrap();
    fixture
}

async fn export_to(fixture: &Fixture, writer: &std::sync::Arc<RecordingWriter>) -> ExportedVersion {
    ExportApprovedVersion::new(fixture.context(), port(writer))
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap()
}

#[tokio::test]
async fn adr_0206_the_export_of_a_version_with_tod_holds_time_of_day() {
    let fixture = approved_with_alias().await;
    let writer = RecordingWriter::shared();

    let exported = export_to(&fixture, &writer).await;

    assert!(exported.loadable());
    let (body, _) = writer.last();
    let types: Vec<String> = body
        .parameters()
        .iter()
        .map(|parameter| parameter.type_id().to_string())
        .collect();
    assert_eq!(types, ["TIME_OF_DAY"]);
    let canonical = body.canonical_text().unwrap();
    assert!(canonical.contains("\"TIME_OF_DAY\""), "{canonical}");
    assert!(!canonical.contains("\"TOD\""), "{canonical}");
}
