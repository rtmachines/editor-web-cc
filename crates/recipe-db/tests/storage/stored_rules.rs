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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::export::{
    ExportApprovedInput, ExportApprovedVersion, ExportScope, ExportedVersion,
};
use recipe_app::identity::new_request_id;
use recipe_app::plant::{DefineUdt, DefineUdtInput, DefineUdtMode, GetTypeTree, TypeRef};
use recipe_app::recipe::{
    AddParameter, AddParameterInput, ApproveVersion, ApproveVersionInput, CreateRecipe,
    CreateRecipeInput, CreateVersion, CreateVersionInput, GetVersion, GetVersionInput,
};
use recipe_db::migrate;
use recipe_db::settings::MigrateSettings;
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::{
    LeafPath, MachineCode, MemberName, S7Address, TypeExpr, TypeName, UnitPath,
};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterTag, ReferenceNumber, ValueInput,
};

use crate::atomic::world::{long_text, text};
use crate::support::app::App;
use crate::support::database::DumpDirectory;
use crate::support::identity::PASSWORD;
use crate::support::writers::DocumentFactsWriter;
use crate::support::{Role as DbRole, TestDatabase, server};

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

// Task 9.2: the stored rows of the matrix on the migrator of this release.

/// The stored fixture: the full plain dump of the earlier release.
const FIXTURE_DUMP: &str = "stored_rules.sql";
/// The stored fixture: one line `<reference> <content_sha256>` for each case.
const FIXTURE_HASHES: &str = "stored_rules.hashes";

/// What a row of the matrix expects after the upgrade.
struct Expectation {
    /// The reference number of the recipe.
    reference: &'static str,
    /// The value of `loadable` in the matrix.
    loadable: bool,
    /// The struct that `GetTypeTree` reads, or `None` for a row without a struct.
    struct_name: Option<&'static str>,
}

/// The stored hash of a reference, from the fixture file.
fn stored_hash(reference: &str) -> String {
    let path = Path::new(FIXTURE_DIRECTORY).join(FIXTURE_HASHES);
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .find_map(|line| {
            let (name, hash) = line.split_once(' ')?;
            (name == reference).then(|| hash.to_owned())
        })
        .ok_or_else(|| format!("the fixture has no hash for {reference}"))
        .unwrap()
}

/// Run a `docker exec` command in the database container with a file on the standard input.
fn exec_with_input(args: &[&str], input: &Path) -> Output {
    let file = std::fs::File::open(input).unwrap();
    Command::new("docker")
        .args(["exec", "-i", "--user", "postgres", &server().container_id])
        .args(args)
        .stdin(Stdio::from(file))
        .output()
        .expect("docker exec runs")
}

/// Assert the exit status 0 of a command and show its standard error otherwise.
fn assert_success(what: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{what} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Steps 1 to 3: create an empty database, load the stored dump, and run the migrator.
async fn restored_and_upgraded() -> App {
    let database = TestDatabase::create_empty().await;
    let dump = Path::new(FIXTURE_DIRECTORY).join(FIXTURE_DUMP);
    let name = database.name().to_owned();
    let loaded = exec_with_input(
        &[
            "psql",
            "--set",
            "ON_ERROR_STOP=1",
            "--single-transaction",
            "--quiet",
            &format!("--dbname={name}"),
        ],
        &dump,
    );
    assert_success("psql", &loaded);
    migrate(
        &database.target(DbRole::Migrator),
        &MigrateSettings::default(),
    )
    .await
    .unwrap();
    App::over(database).await
}

/// The rows of `parameter` and `plc_type` as text. A stored type text and a stored value do not
/// change in the upgrade, so the text is equal before and after.
async fn stored_rows(app: &mut App, reference: &str) -> (Vec<String>, Vec<String>) {
    let parameters = sqlx::query_scalar(
        "SELECT to_jsonb(p)::text FROM parameter p \
         JOIN recipe_version v ON v.id = p.recipe_version_id \
         JOIN recipe r ON r.id = v.recipe_id \
         WHERE r.reference_number = $1 ORDER BY v.number, p.tag",
    )
    .bind(reference)
    .fetch_all(app.admin())
    .await
    .unwrap();
    let types = sqlx::query_scalar("SELECT to_jsonb(t)::text FROM plc_type t ORDER BY t.id")
        .fetch_all(app.admin())
        .await
        .unwrap();
    (parameters, types)
}

/// The export of the approved version of a reference through the adapter.
async fn export_of(app: &App, reference: &str) -> ExportedVersion {
    let input = ExportApprovedInput {
        reference: ReferenceNumber::parse(reference).unwrap(),
        machine: None,
        scope: ExportScope::Version,
    };
    ExportApprovedVersion::new(app.context(), DocumentFactsWriter::port())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap()
}

/// Assert the flag and the hash inside the bytes of the document, not only the fields of the use case.
fn assert_document(exported: &ExportedVersion, reference: &str, loadable: bool) {
    let document: serde_json::Value = serde_json::from_slice(&exported.document).unwrap();
    assert_eq!(document["loadable"], serde_json::Value::Bool(loadable));
    assert_eq!(document["content_sha256"], stored_hash(reference));
}

/// Step 4: `GetVersion` and, for a struct row, `GetTypeTree` read the stored row.
async fn read_row(app: &mut App, expected: &Expectation) {
    let input = GetVersionInput {
        reference: ReferenceNumber::parse(expected.reference).unwrap(),
        number: 1,
        rebind_bound: 0,
    };
    let view = GetVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    assert_eq!(
        view.detail.content_sha256.map(|hash| hash.to_string()),
        Some(stored_hash(expected.reference))
    );
    assert_eq!(view.rule_codes.is_empty(), expected.loadable);
    let Some(name) = expected.struct_name else {
        return;
    };
    let code: String = sqlx::query_scalar(
        "SELECT m.code FROM machine m JOIN recipe r ON r.machine_id = m.id \
         WHERE r.reference_number = $1",
    )
    .bind(expected.reference)
    .fetch_one(app.admin())
    .await
    .unwrap();
    let tree = TypeRef {
        code: MachineCode::parse(&code).unwrap(),
        name: TypeName::parse(name).unwrap(),
        revision: None,
    };
    let type_view = GetTypeTree::new(app.context())
        .execute(&app.user, &app.meta, tree)
        .await
        .unwrap();
    assert_eq!(type_view.revision, 1);
}

/// Step 6: `CreateVersion` copies the approved version into a draft with the same rows.
async fn copy_row(app: &mut App, reference: &str) {
    let input = CreateVersionInput {
        reference: ReferenceNumber::parse(reference).unwrap(),
        based_on: Some(1),
        change_note: long_text("Copy of the stored row"),
    };
    let created = CreateVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    assert_eq!(created.number, 2);
    let copied: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT p.tag, p.s7_address, p.value::text FROM parameter p \
         JOIN recipe_version v ON v.id = p.recipe_version_id \
         JOIN recipe r ON r.id = v.recipe_id \
         WHERE r.reference_number = $1 AND v.number = $2 ORDER BY p.tag",
    )
    .bind(reference)
    .bind(1_i32)
    .fetch_all(app.admin())
    .await
    .unwrap();
    let draft: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT p.tag, p.s7_address, p.value::text FROM parameter p \
         JOIN recipe_version v ON v.id = p.recipe_version_id \
         JOIN recipe r ON r.id = v.recipe_id \
         WHERE r.reference_number = $1 AND v.number = $2 ORDER BY p.tag",
    )
    .bind(reference)
    .bind(2_i32)
    .fetch_all(app.admin())
    .await
    .unwrap();
    assert_eq!(copied.len(), 1);
    assert_eq!(draft, copied);
}

/// Steps 7 and 8: a custom dump, a restore into a second empty database, and the same export.
async fn backup_and_restore(app: &App, reference: &str, expected: &ExportedVersion) {
    let files = DumpDirectory::create();
    let dump = Command::new("docker")
        .args(["exec", "--user", "postgres", &server().container_id])
        .args(["pg_dump", "--format=custom"])
        .arg(format!("--dbname={}", app.database_name()))
        .output()
        .expect("docker exec runs");
    assert_success("pg_dump", &dump);
    let file = files.file(&format!("{}.dump", app.database_name()));
    let mut handle = std::fs::File::create(&file).unwrap();
    handle.write_all(&dump.stdout).unwrap();
    drop(handle);
    let copy = TestDatabase::create_empty().await;
    let restored = exec_with_input(
        &[
            "pg_restore",
            "--exit-on-error",
            "--single-transaction",
            &format!("--dbname={}", copy.name()),
        ],
        &file,
    );
    assert_success("pg_restore", &restored);
    let copied = App::over(copy).await;
    let again = export_of(&copied, reference).await;
    assert_document(&again, reference, expected.loadable());
    assert_eq!(&again, expected);
}

/// The eight steps of task 9.2 for one row of the matrix.
async fn check_stored_row(expected: Expectation) {
    let mut app = restored_and_upgraded().await;
    let before = stored_rows(&mut app, expected.reference).await;
    read_row(&mut app, &expected).await;
    let exported = export_of(&app, expected.reference).await;
    assert_eq!(
        exported.content_sha256.to_string(),
        stored_hash(expected.reference)
    );
    assert_eq!(exported.loadable(), expected.loadable);
    assert_document(&exported, expected.reference, expected.loadable);
    assert_eq!(stored_rows(&mut app, expected.reference).await, before);
    copy_row(&mut app, expected.reference).await;
    backup_and_restore(&app, expected.reference, &exported).await;
}

#[tokio::test]
async fn adr_0177_stored_nested_array_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-NESTED-ARRAY",
        loadable: true,
        struct_name: None,
    })
    .await;
}

#[tokio::test]
async fn adr_0178_stored_array_bound_exports_with_its_hash_and_is_not_loadable() {
    check_stored_row(Expectation {
        reference: "SR-ARRAY-BOUND",
        loadable: false,
        struct_name: None,
    })
    .await;
}

#[tokio::test]
async fn adr_0179_stored_nesting_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-NESTING",
        loadable: true,
        struct_name: Some("ST_Top"),
    })
    .await;
}

#[tokio::test]
async fn adr_0180_stored_wchar_exports_with_its_hash_and_is_not_loadable() {
    check_stored_row(Expectation {
        reference: "SR-WCHAR",
        loadable: false,
        struct_name: None,
    })
    .await;
}

#[tokio::test]
async fn adr_0180_stored_wstring_exports_with_its_hash_and_is_not_loadable() {
    check_stored_row(Expectation {
        reference: "SR-WSTRING",
        loadable: false,
        struct_name: None,
    })
    .await;
}

#[tokio::test]
async fn adr_0186_stored_reserved_struct_name_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-RESERVED-NAME",
        loadable: true,
        struct_name: Some("PID_Compact"),
    })
    .await;
}

#[tokio::test]
async fn adr_0187_stored_bool_array_3d_exports_with_its_hash_and_is_not_loadable() {
    check_stored_row(Expectation {
        reference: "SR-BOOL-3D",
        loadable: false,
        struct_name: None,
    })
    .await;
}
