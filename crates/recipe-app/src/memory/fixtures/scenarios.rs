//! Seeded fixtures and valid requests of the use cases, shared by the permission and audit tests.
//!
//! A fixture holds the rows that a success path needs. A request is the valid input of the
//! same scenario. The tests of `auth::first_statement` and `audit::coverage` use both.

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::plant::{
    LeafPath, MachineCode, MemberName, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape,
    TypeExpr, TypeMember, TypeName, UnitPath,
};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterTag, RecipeDeleter, StoredParameter, StoredValue,
    Translation,
};
use serde_json::json;

use super::{
    Fixture, lang, long_text, meta, mid, parameter_on, pkey, reference, second_engineer, text, tid,
    timestamp, uid, user, vid,
};
use crate::export::{ExportApprovedInput, ExportByNumberInput};
use crate::memory::seed_type_id;
use crate::plant::{
    AddUnitInput, ArchiveMachineInput, ArchiveUnitInput, CreateMachineInput, DefineUdtInput,
    DefineUdtMode, RenameMachineInput, RenameUnitInput,
};
use crate::recipe::{
    AddParameterInput, ApproveVersion, ApproveVersionInput, CreateRecipeInput, CreateVersionInput,
    DeleteRecipeInput, DiscardDraftInput, RebindTypeRevisionInput, RemoveParameterInput,
    RemoveTranslationInput, RenameRecipeInput, ReplaceLimitsInput, RestoreRecipeInput,
    RetireVersionInput, SetParameterValueInput, UpdateParameterDefinitionInput,
    UpsertTranslationInput,
};

/// The PLC platform of the scenarios.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
/// The identifier of `ST_Zone` revision 1.
const ZONE_1: u128 = 901;
/// The identifier of `ST_Zone` revision 2.
const ZONE_2: u128 = 902;
/// The key number of the parameter `ZONE1` of the rebind scenario.
const ZONE_PARAMETER: u128 = 11;

/// A fixture with the machine `M1` and its units `U1`, `U1.2`, and `U1.2.1`, and no recipe.
pub(crate) fn plant() -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture
}

/// A fixture with the live draft `R-1` (version 101) and the parameter `SPEED` (key 1) on `U1`.
pub(crate) fn draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(101), &parameter_on(PLATFORM, 1, "SPEED", uid(101), 0));
    fixture
}

/// A fixture with recipe `R-1` whose version 1 is retired and version 2 is approved.
pub(crate) fn retired() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    fixture
}

/// A fixture with recipe `R-1` whose version 1 is approved and version 2 is a live draft.
///
/// The draft holds the parameter `SPEED` (key 1).
pub(crate) fn approved_and_draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(102), &parameter_on(PLATFORM, 1, "SPEED", uid(101), 0));
    fixture
}

/// A fixture with the deleted recipe `R-1` whose live draft is discarded.
pub(crate) fn deleted() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    let (mut recipe, _) = fixture.seed_draft(&machine, 1, "R-1");
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, machine.platform(), user(2));
    fixture
}

/// A fixture with the live draft `R-1` and the parameter `ZONE1` of `ST_Zone` revision 1.
///
/// The revisions 1 and 2 of `ST_Zone` are stored. Revision 2 adds the member `extra`.
pub(crate) fn zone_draft() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_type(&zone_row(ZONE_1, 1, None, &["setpoint"]));
    fixture
        .store
        .seed_type(&zone_row(ZONE_2, 2, Some(ZONE_1), &["setpoint", "extra"]));
    let zone = StoredParameter {
        plc_type: tid(ZONE_1),
        value: StoredValue::from(json!({"setpoint": 1})),
        limits: Vec::new(),
        ..parameter_on(PLATFORM, ZONE_PARAMETER, "ZONE1", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(101), &zone);
    fixture
}

/// A fixture with the approved version 1 of `R-1`, approved by `anowak`.
///
/// The approval writes audit entries. A test that counts entries must skip them.
pub(crate) async fn approved_version() -> Fixture {
    let fixture = draft();
    ApproveVersion::new(fixture.context())
        .execute(&second_engineer(), &meta(), approve_input(1))
        .await
        .unwrap();
    fixture
}

/// Build a `ST_Zone` revision with the given members of type `INT`.
fn zone_row(id: u128, revision: u32, supersedes: Option<u128>, members: &[&str]) -> PlcType {
    let members: Vec<TypeMember> = members
        .iter()
        .map(|name| TypeMember {
            name: MemberName::parse(name).unwrap(),
            type_id: seed_type_id(Primitive::Int, None).unwrap(),
        })
        .collect();
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: supersedes.map(tid),
            members,
        },
    })
    .unwrap()
}

/// A limit row on the leaf path `""` with the given bounds.
fn bounds(min: &str, max: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    }
}

/// The request of `CreateMachine` for the new machine `M7` on `S7_300`.
pub(crate) fn create_machine_input() -> CreateMachineInput {
    CreateMachineInput {
        code: MachineCode::parse("M7").unwrap(),
        name: text("Press 7"),
        platform: PlcPlatform::S7_300,
        main_unit_name: text("Feeder"),
    }
}

/// The request of `RenameMachine` for `M1`.
pub(crate) fn rename_machine_input() -> RenameMachineInput {
    RenameMachineInput {
        code: MachineCode::parse("M1").unwrap(),
        name: text("Press 2"),
    }
}

/// The request of `ArchiveMachine` for `M1`.
pub(crate) fn archive_machine_input() -> ArchiveMachineInput {
    ArchiveMachineInput {
        code: MachineCode::parse("M1").unwrap(),
    }
}

/// The request of `AddUnit` that adds `U1.5` under `U1` of `M1`.
pub(crate) fn add_unit_input() -> AddUnitInput {
    AddUnitInput {
        code: MachineCode::parse("M1").unwrap(),
        parent_path: Some(UnitPath::parse("U1").unwrap()),
        segment: "5".to_owned(),
        name: text("Five"),
    }
}

/// The request of `RenameUnit` for `U1.2` of `M1`.
pub(crate) fn rename_unit_input() -> RenameUnitInput {
    RenameUnitInput {
        code: MachineCode::parse("M1").unwrap(),
        path: UnitPath::parse("U1.2").unwrap(),
        name: text("Two b"),
    }
}

/// The request of `ArchiveUnit` for `U1.2` of `M1`, which also archives `U1.2.1`.
pub(crate) fn archive_unit_input() -> ArchiveUnitInput {
    ArchiveUnitInput {
        code: MachineCode::parse("M1").unwrap(),
        path: UnitPath::parse("U1.2").unwrap(),
    }
}

/// The request of `DefineUdt` for the new struct `ST_Tank` with one `STRING(20)` member.
pub(crate) fn define_udt_input() -> DefineUdtInput {
    DefineUdtInput {
        code: MachineCode::parse("M1").unwrap(),
        name: TypeName::parse("ST_Tank").unwrap(),
        members: vec![(
            MemberName::parse("label").unwrap(),
            TypeExpr::parse("STRING(20)").unwrap(),
        )],
        mode: DefineUdtMode::New,
    }
}

/// The request of `CreateRecipe` for the new recipe `R-2` on `M1`.
pub(crate) fn create_recipe_input() -> CreateRecipeInput {
    CreateRecipeInput {
        machine_code: MachineCode::parse("M1").unwrap(),
        reference: reference("R-2"),
        name: text("Second"),
        change_note: long_text("Initial"),
    }
}

/// The request of `RenameRecipe` for `R-1`.
pub(crate) fn rename_recipe_input() -> RenameRecipeInput {
    RenameRecipeInput {
        reference: reference("R-1"),
        name: text("Renamed"),
    }
}

/// The request of `DeleteRecipe` for `R-1`.
pub(crate) fn delete_recipe_input() -> DeleteRecipeInput {
    DeleteRecipeInput {
        reference: reference("R-1"),
    }
}

/// The request of `RestoreRecipe` for `R-1`.
pub(crate) fn restore_recipe_input() -> RestoreRecipeInput {
    RestoreRecipeInput {
        reference: reference("R-1"),
    }
}

/// The request of `CreateVersion` that starts an empty draft of `R-1`.
pub(crate) fn create_version_input() -> CreateVersionInput {
    CreateVersionInput {
        reference: reference("R-1"),
        based_on: None,
        change_note: long_text("Next"),
    }
}

/// The request of `DiscardDraft` for the draft 1 of `R-1`.
pub(crate) fn discard_draft_input() -> DiscardDraftInput {
    DiscardDraftInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
    }
}

/// The request of `ApproveVersion` for version `number` of `R-1`.
pub(crate) fn approve_input(number: u32) -> ApproveVersionInput {
    ApproveVersionInput {
        reference: reference("R-1"),
        number,
        expected_row_version: 1,
        allow_outdated: false,
    }
}

/// The request of `RetireVersion` for the approved version 2 of `R-1`.
pub(crate) fn retire_version_input() -> RetireVersionInput {
    RetireVersionInput {
        reference: reference("R-1"),
        number: 2,
        expected_row_version: 1,
    }
}

/// The request of `AddParameter` for the tag `FEED` on the unit `U1` of the draft 1.
pub(crate) fn add_parameter_input() -> AddParameterInput {
    AddParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("FEED").unwrap(),
        name_en: text("Feed"),
        description_en: long_text("Feed rate"),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value_json: "120".to_owned(),
        limits: LimitList::parse(vec![bounds("10", "250")]).unwrap(),
    }
}

/// The request of `UpdateParameterDefinition` that renames the parameter `SPEED`.
pub(crate) fn update_definition_input() -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("SPEED").unwrap(),
        name_en: text("Speed"),
        description_en: long_text("Drive speed"),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(0).unwrap(),
        retype: None,
    }
}

/// The request of `SetParameterValue` that sets `SPEED` to 130.
pub(crate) fn set_value_input() -> SetParameterValueInput {
    SetParameterValueInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        value_json: "130".to_owned(),
    }
}

/// The request of `ReplaceLimits` that sets the bounds of `SPEED` to 0 and 200.
pub(crate) fn replace_limits_input() -> ReplaceLimitsInput {
    ReplaceLimitsInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        limits: LimitList::parse(vec![bounds("0", "200")]).unwrap(),
    }
}

/// The request of `UpsertTranslation` that adds the German name of `SPEED`.
pub(crate) fn upsert_translation_input() -> UpsertTranslationInput {
    UpsertTranslationInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        translation: Translation {
            lang: lang("de"),
            name: text("Speed"),
            description: Some(long_text("Drive speed")),
        },
    }
}

/// The request of `RemoveTranslation` that removes the Polish name of `SPEED`.
pub(crate) fn remove_translation_input() -> RemoveTranslationInput {
    RemoveTranslationInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
        lang: lang("pl"),
    }
}

/// The request of `RemoveParameter` that removes `SPEED`.
pub(crate) fn remove_parameter_input() -> RemoveParameterInput {
    RemoveParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        key: pkey(1),
    }
}

/// The request of `RebindTypeRevision` from `ST_Zone` revision 1 to revision 2 of `ZONE1`.
pub(crate) fn rebind_input() -> RebindTypeRevisionInput {
    RebindTypeRevisionInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        from: tid(ZONE_1),
        to: tid(ZONE_2),
        new_member_values: BTreeMap::from([(
            (pkey(ZONE_PARAMETER), LeafPath::parse(".extra").unwrap()),
            json!(7),
        )]),
    }
}

/// The request of `ExportApprovedVersion` for `R-1`.
pub(crate) fn export_approved_input() -> ExportApprovedInput {
    ExportApprovedInput {
        reference: reference("R-1"),
    }
}

/// The request of `ExportVersionByNumber` for version 1 of `R-1`.
pub(crate) fn export_number_input() -> ExportByNumberInput {
    ExportByNumberInput {
        reference: reference("R-1"),
        number: 1,
    }
}
