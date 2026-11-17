//! Stored rows of the earlier release and the rules of `siemens-type-rules` (`design.md`,
//! section "Stored data matrix").
//!
//! Each rule narrows what a command accepts, but a stored row stays readable (ADR-0120). The
//! ignored test `stored_rules_write_the_fixture` builds the seven rows of the matrix with the use
//! cases of the code before the rules. It writes a full plain dump of that database and the
//! content hash of each approved version. The tests of task 9.2 load the dump into an empty
//! database and run the migrator of a later release on it.
//!
//! A plain dump creates each table with its constraints before it loads the data. It creates
//! the foreign keys and the triggers after the data. Thus no trigger runs on a row of the dump.

use std::path::PathBuf;
use std::process::Command;

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::identity::new_request_id;
use recipe_app::plant::{DefineUdt, DefineUdtInput, DefineUdtMode};
use recipe_app::recipe::{
    AddParameter, AddParameterInput, ApproveVersion, ApproveVersionInput, CreateRecipe,
    CreateRecipeInput, GetVersion, GetVersionInput,
};
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::{LeafPath, MemberName, S7Address, TypeExpr, TypeName, UnitPath};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterTag, ReferenceNumber, ValueInput,
};

use crate::atomic::world::{long_text, text};
use crate::support::app::App;
use crate::support::identity::PASSWORD;
use crate::support::server;

/// The login of the engineer, who edits each draft.
const ENGINEER: &str = "engineer";
/// The directory of the two fixture files, from the root of the crate.
const FIXTURE_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/storage/fixtures");
/// The levels of the struct `ST_Top` of the nesting row. The count of ADR-0179 is nine.
const NESTING_LEVELS: u32 = 9;

/// The data of one row of the matrix.
struct Case {
    /// The reference number of the recipe.
    reference: &'static str,
    /// The platform of the machine.
    platform: &'static str,
    /// The structs that `DefineUdt` creates before the parameter, the innermost first.
    structs: Vec<(String, Vec<(&'static str, String)>)>,
    /// The type text of the parameter.
    type_text: &'static str,
    /// The S7 address of the parameter, or `None`.
    address: Option<&'static str>,
    /// The JSON text of the value.
    value: String,
    /// The limits of the leaves.
    limits: Vec<LimitInput>,
}

/// The seven rows of the matrix, in the order of `design.md`.
fn matrix() -> Vec<Case> {
    vec![
        Case {
            reference: "SR-NESTED-ARRAY",
            platform: "S7_300",
            structs: Vec::new(),
            type_text: "ARRAY[1..2] OF ARRAY[1..3] OF BYTE",
            address: Some("DB1.DBB0"),
            value: "[[1,2,3],[4,5,6]]".to_owned(),
            limits: range_limits(
                &["[1][1]", "[1][2]", "[1][3]", "[2][1]", "[2][2]", "[2][3]"],
                "0",
                "255",
            ),
        },
        Case {
            reference: "SR-ARRAY-BOUND",
            platform: "S7_300",
            structs: Vec::new(),
            type_text: "ARRAY[32760..32770] OF BYTE",
            address: Some("DB1.DBB0"),
            value: "[0,1,2,3,4,5,6,7,8,9,10]".to_owned(),
            limits: range_limits(&bound_paths(), "0", "255"),
        },
        Case {
            reference: "SR-NESTING",
            platform: "S7_1500",
            structs: nesting_structs(),
            type_text: "ST_Top@1",
            address: None,
            value: nesting_value(),
            limits: range_limits(&[nesting_leaf_path().as_str()], "0", "100"),
        },
        Case {
            reference: "SR-WCHAR",
            platform: "S7_1500",
            structs: Vec::new(),
            type_text: "WCHAR",
            address: None,
            value: "\"\\uE000\"".to_owned(),
            limits: Vec::new(),
        },
        Case {
            reference: "SR-WSTRING",
            platform: "S7_1200",
            structs: Vec::new(),
            type_text: "WSTRING(10)",
            address: None,
            value: "\"A\\uD000\"".to_owned(),
            limits: vec![length_limit("", 10)],
        },
        Case {
            reference: "SR-RESERVED-NAME",
            platform: "S7_1500",
            structs: vec![(
                "PID_Compact".to_owned(),
                vec![("setpoint", "REAL".to_owned())],
            )],
            type_text: "PID_Compact@1",
            address: None,
            value: r#"{"setpoint":1.5}"#.to_owned(),
            limits: range_limits(&[".setpoint"], "0", "10"),
        },
        Case {
            reference: "SR-BOOL-3D",
            platform: "S7_300",
            structs: Vec::new(),
            type_text: "ARRAY[1..2,1..2,1..2] OF BOOL",
            address: Some("DB1.DBB0"),
            value: "[[[true,false],[false,true]],[[true,true],[false,false]]]".to_owned(),
            limits: Vec::new(),
        },
    ]
}

/// A numeric limit on each listed leaf.
fn range_limits(paths: &[&str], min: &str, max: &str) -> Vec<LimitInput> {
    paths
        .iter()
        .map(|path| LimitInput {
            leaf_path: LeafPath::parse(path).unwrap(),
            min: Some(min.to_owned()),
            max: Some(max.to_owned()),
            max_length: None,
        })
        .collect()
}

/// A length limit on one leaf.
fn length_limit(path: &str, length: u16) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: Some(length),
    }
}

/// The leaf paths of `ARRAY[32760..32770] OF BYTE`.
fn bound_paths() -> Vec<&'static str> {
    vec![
        "[32760]", "[32761]", "[32762]", "[32763]", "[32764]", "[32765]", "[32766]", "[32767]",
        "[32768]", "[32769]", "[32770]",
    ]
}

/// The path of the `DINT` leaf of `ST_Top@1`.
fn nesting_leaf_path() -> String {
    let steps = ".next".repeat(usize::try_from(NESTING_LEVELS - 1).unwrap());
    format!("{steps}.leaf")
}

/// The name of the struct at a level. Level 1 is `ST_Top`.
fn level_name(level: u32) -> String {
    if level == 1 {
        "ST_Top".to_owned()
    } else {
        format!("ST_L{level}")
    }
}

/// The nine structs of the nesting row, the innermost first. Each one holds the next level in
/// the member `next`. The innermost holds a `DINT` in the member `leaf`.
fn nesting_structs() -> Vec<(String, Vec<(&'static str, String)>)> {
    let mut structs = vec![(
        level_name(NESTING_LEVELS),
        vec![("leaf", "DINT".to_owned())],
    )];
    for level in (1..NESTING_LEVELS).rev() {
        let inner = level.checked_add(1).unwrap();
        structs.push((
            level_name(level),
            vec![("next", format!("{}@1", level_name(inner)))],
        ));
    }
    structs
}

/// The value of `ST_Top@1`: eight objects `next` and one leaf.
fn nesting_value() -> String {
    let opened = r#"{"next":"#.repeat(usize::try_from(NESTING_LEVELS - 1).unwrap());
    let closed = "}".repeat(usize::try_from(NESTING_LEVELS - 1).unwrap());
    format!(r#"{opened}{{"leaf":7}}{closed}"#)
}

/// The data of a new request.
fn new_meta() -> RequestMeta {
    RequestMeta::new(new_request_id().unwrap(), None, None, None)
}

/// The row version of the version 1 of a recipe, read through `GetVersion`.
async fn row_version(app: &App, reference: &ReferenceNumber) -> i64 {
    let input = GetVersionInput {
        reference: reference.clone(),
        number: 1,
        rebind_bound: 0,
    };
    GetVersion::new(app.context())
        .execute(&app.user, &new_meta(), input)
        .await
        .unwrap()
        .detail
        .row_version
}

/// Create the structs, the recipe, and the parameter of a case as the engineer. Then approve the
/// version as the administrator.
async fn approve_case(app: &mut App, engineer: &CurrentUser, case: &Case) {
    let seed = app.seed_machine(case.platform, &["U1"]).await;
    for (name, members) in &case.structs {
        let input = DefineUdtInput {
            code: seed.code,
            name: TypeName::parse(name).unwrap(),
            members: members
                .iter()
                .map(|(member, type_text)| {
                    (
                        MemberName::parse(member).unwrap(),
                        TypeExpr::parse(type_text).unwrap(),
                    )
                })
                .collect(),
            mode: DefineUdtMode::New,
        };
        DefineUdt::new(app.context())
            .execute(&app.user, &new_meta(), input)
            .await
            .unwrap();
    }
    let reference = ReferenceNumber::parse(case.reference).unwrap();
    let created = CreateRecipeInput {
        machine_code: seed.code,
        reference: reference.clone(),
        name: text(case.reference),
        change_note: long_text("Stored row of the matrix"),
    };
    CreateRecipe::new(app.context())
        .execute(engineer, &new_meta(), created)
        .await
        .unwrap();
    let parameter = AddParameterInput {
        reference: reference.clone(),
        number: 1,
        expected_row_version: row_version(app, &reference).await,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("VALUE").unwrap(),
        name_en: text("Value"),
        description_en: long_text("Value of the matrix row"),
        type_expr: TypeExpr::parse(case.type_text).unwrap(),
        s7_address: case
            .address
            .map(|address| S7Address::parse(address).unwrap()),
        ordinal: Ordinal::parse(1).unwrap(),
        value: ValueInput::Json(case.value.clone()),
        limits: LimitList::parse(case.limits.clone()).unwrap(),
    };
    AddParameter::new(app.context())
        .execute(engineer, &new_meta(), parameter)
        .await
        .unwrap();
    let approval = ApproveVersionInput {
        reference: reference.clone(),
        number: 1,
        expected_row_version: row_version(app, &reference).await,
        allow_outdated: false,
    };
    ApproveVersion::new(app.context())
        .execute(&app.user, &new_meta(), approval)
        .await
        .unwrap();
}

/// The lines `<reference> <content_sha256>` of the approved versions, ordered by reference.
async fn hash_lines(app: &mut App) -> String {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT r.reference_number, v.content_sha256 FROM recipe_version v \
         JOIN recipe r ON r.id = v.recipe_id \
         WHERE v.status = 'approved' ORDER BY r.reference_number",
    )
    .fetch_all(app.admin())
    .await
    .unwrap();
    rows.iter()
        .map(|(reference, hash)| format!("{reference} {hash}\n"))
        .collect()
}

/// The standard output of `pg_dump --format=plain` of a database: the schema and the data of
/// every table, also `_sqlx_migrations`.
fn plain_dump(database: &str) -> Vec<u8> {
    let output = Command::new("docker")
        .args(["exec", "--user", "postgres", &server().container_id])
        .args(["pg_dump", "--format=plain"])
        .arg(format!("--dbname={database}"))
        .output()
        .expect("docker exec runs");
    assert!(
        output.status.success(),
        "pg_dump failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// Write the fixture of task 9.2. Run it once with the code before the rules:
/// `cargo test -p recipe-db --test storage stored_rules_write_the_fixture -- --ignored`.
/// A second run gives other UUIDs.
#[tokio::test]
#[ignore = "writes the fixture files once, with the code before the rules"]
async fn stored_rules_write_the_fixture() {
    let mut app = App::create().await;
    let id = app
        .seed_user(ENGINEER, Role::ProcessEngineer, PASSWORD)
        .await;
    let engineer = CurrentUser::new(id, Login::parse(ENGINEER).unwrap(), Role::ProcessEngineer);
    for case in matrix() {
        approve_case(&mut app, &engineer, &case).await;
    }
    let hashes = hash_lines(&mut app).await;
    assert_eq!(hashes.lines().count(), matrix().len());

    let dump = plain_dump(app.database_name());

    let directory = PathBuf::from(FIXTURE_DIRECTORY);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("stored_rules.sql"), dump).unwrap();
    std::fs::write(directory.join("stored_rules.hashes"), hashes).unwrap();
}
