//! A refused changed type takes no class-2 lock (design D3 of the change `type-check-before-insert`).
//!
//! A third connection holds the class-2 lock with the key `ARRAY` of the machine. The command
//! `UpdateParameterDefinition` asks for a nested array on `S7_1500`, which the platform refuses.
//! The command checks the type before the unit lock. It ends with `E-PLT-013` and does not wait
//! for the held lock. A command that took the lock first would wait until `db.lock_timeout` and
//! would end with `E-DB-001`. The test sets no wall-clock bound.

use recipe_app::recipe::{RetypeInput, UpdateParameterDefinition, UpdateParameterDefinitionInput};
use recipe_domain::plant::{TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterKey, ParameterTag, ReferenceNumber, ValueInput,
};
use sqlx::Connection as _;
use uuid::Uuid;

use crate::atomic::world::{long_text, text};
use crate::support::Role;
use crate::support::app::App;
use crate::support::fixtures;

/// The platform of the machine. It refuses an array of an array.
const PLATFORM: &str = "S7_1500";
/// The class of the advisory locks of derived types (`data-model.md` section 1.4).
const CLASS_2: i32 = 2;
/// The refused type.
const NESTED_ARRAY: &str = "ARRAY[1..2] OF ARRAY[1..3] OF INT";

/// The rows of `plc_type` that belong to the machine.
const MACHINE_TYPE_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1";
/// The audit entries of the action `DerivedTypeCreated`.
const CREATED_ENTRIES: &str = "SELECT count(*) FROM audit_log WHERE action = 'DerivedTypeCreated'";
/// The name of the type of a parameter.
const TYPE_NAME: &str = "SELECT t.name FROM parameter p JOIN plc_type t ON t.id = p.plc_type_id \
                         WHERE p.id = $1";

/// An `S7_1500` machine with the unit `U1` and a draft that holds `SPEED` of type `INT`.
struct Fixture {
    machine: Uuid,
    version: Uuid,
    parameter: Uuid,
    key: ParameterKey,
    reference: ReferenceNumber,
}

async fn fixture(app: &mut App) -> Fixture {
    let seed = app.seed_machine(PLATFORM, &["U1"]).await;
    let creator = *app.user.user().as_uuid();
    let unit: Uuid = sqlx::query_scalar("SELECT id FROM unit WHERE machine_id = $1")
        .bind(seed.id)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    let recipe = fixtures::recipe(app.migrator(), seed.id, PLATFORM, creator).await;
    let version = fixtures::draft_version(app.migrator(), recipe, PLATFORM, 1, creator).await;
    let int = fixtures::primitive(app.migrator(), "INT").await;
    let parameter =
        fixtures::parameter(app.migrator(), version, PLATFORM, unit, int, "SPEED").await;
    let key: Uuid = sqlx::query_scalar("SELECT parameter_key FROM parameter WHERE id = $1")
        .bind(parameter)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    let reference: String = sqlx::query_scalar("SELECT reference_number FROM recipe WHERE id = $1")
        .bind(recipe)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    Fixture {
        machine: seed.id,
        version,
        parameter,
        key: ParameterKey::from(key),
        reference: ReferenceNumber::parse(&reference).unwrap(),
    }
}

/// The request that changes `SPEED` to the nested array.
fn nested_array_input(fixture: &Fixture, expected: i64) -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        reference: fixture.reference.clone(),
        number: 1,
        expected_row_version: expected,
        key: fixture.key,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("SPEED").unwrap(),
        name_en: text("Speed"),
        description_en: long_text("Speed of the belt"),
        type_expr: TypeExpr::parse(NESTED_ARRAY).unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(0).unwrap(),
        retype: Some(RetypeInput {
            value: ValueInput::Json("[[1,2,3],[1,2,3]]".to_owned()),
            limits: LimitList::parse(Vec::<LimitInput>::new()).unwrap(),
        }),
    }
}

/// A refused nested array ends with `E-PLT-013` while another connection holds the lock
/// `ARRAY`. The command writes no type row and no audit entry, and `SPEED` keeps `INT`.
#[tokio::test]
async fn a_refused_changed_type_gives_plt_013_and_does_not_wait_for_the_array_lock() {
    let mut app = App::create().await;
    let fixture = fixture(&mut app).await;
    let expected: i64 = sqlx::query_scalar("SELECT row_version FROM recipe_version WHERE id = $1")
        .bind(fixture.version)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    let type_rows = app.count(MACHINE_TYPE_ROWS, fixture.machine).await;
    let created = sqlx::query_scalar::<_, i64>(CREATED_ENTRIES)
        .fetch_one(app.admin())
        .await
        .unwrap();
    let mut holder_session = app.session(Role::Migrator).await;
    let mut holder = holder_session.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text || ':' || $3))")
        .bind(CLASS_2)
        .bind(fixture.machine.to_string())
        .bind("ARRAY")
        .execute(&mut *holder)
        .await
        .unwrap();

    let error = UpdateParameterDefinition::new(app.context())
        .execute(&app.user, &app.meta, nested_array_input(&fixture, expected))
        .await
        .unwrap_err();

    assert_eq!(error.code().as_str(), "E-PLT-013", "{error:?}");
    holder.rollback().await.unwrap();
    assert_eq!(
        app.count(MACHINE_TYPE_ROWS, fixture.machine).await,
        type_rows
    );
    let created_after = sqlx::query_scalar::<_, i64>(CREATED_ENTRIES)
        .fetch_one(app.admin())
        .await
        .unwrap();
    assert_eq!(created_after, created);
    let name: String = sqlx::query_scalar(TYPE_NAME)
        .bind(fixture.parameter)
        .fetch_one(app.admin())
        .await
        .unwrap();
    assert_eq!(name, "INT");
}
