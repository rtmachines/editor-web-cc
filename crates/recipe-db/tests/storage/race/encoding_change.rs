//! Races of `SetMachineStringEncoding` and `CreateRecipe` on two connections (task 11.13,
//! ADR-0202 (d3), requirement "No new version during a change of the STRING encoding").
//!
//! Each test holds the first command at a gate inside its transaction, after its machine lock.
//! The second command starts and waits for that lock. The test proves the wait with
//! `pg_stat_activity` before it opens the gate. No command may end with `E-DB-001`.

use std::future::Future;

use recipe_app::error::AppError;
use recipe_app::plant::{SetMachineStringEncoding, SetMachineStringEncodingInput};
use recipe_app::recipe::{CreateRecipe, CreateRecipeInput, CreateRecipeOutput};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineCode, StringEncoding};
use recipe_domain::recipe::ReferenceNumber;
use uuid::Uuid;

use crate::atomic::world::{long_text, text};
use crate::support::app::App;
use crate::support::fixtures::TWIN_CAT;
use crate::support::gate::{self, assert_no_deadlock, error_code};

/// The condition of the gate on the statement that sets the encoding `Utf8`.
const SETTING_UTF8: &str = "NEW.string_encoding = 'Utf8'";
/// The encoding of a machine.
const MACHINE_ENCODING: &str = "SELECT string_encoding FROM machine WHERE id = $1";
/// The encoding of the machine of a recipe.
const RECIPE_MACHINE_ENCODING: &str = "SELECT m.string_encoding FROM recipe r \
     JOIN machine m ON m.id = r.machine_id WHERE r.id = $1";
/// Counts the recipes of a machine.
const RECIPE_ROWS: &str = "SELECT count(*) FROM recipe WHERE machine_id = $1";
/// Counts the audit entries of the encoding change.
const CHANGE_ENTRIES: &str =
    "SELECT count(*) FROM audit_log WHERE action = 'MachineStringEncodingSet'";

/// Start the change to `Utf8` as a future that owns its data.
fn set_utf8(
    app: &App,
    code: MachineCode,
) -> impl Future<Output = Result<StringEncoding, AppError>> + Send + 'static {
    let (ctx, user, meta) = (app.context(), app.user.clone(), app.meta.clone());
    async move {
        SetMachineStringEncoding::new(ctx)
            .execute(
                &user,
                &meta,
                SetMachineStringEncodingInput {
                    code,
                    string_encoding: StringEncoding::Utf8,
                },
            )
            .await
            .map(|output| output.string_encoding)
    }
}

/// Start `CreateRecipe` as a future that owns its data.
fn create_recipe(
    app: &App,
    code: MachineCode,
) -> impl Future<Output = Result<CreateRecipeOutput, AppError>> + Send + 'static {
    let (ctx, user, meta) = (app.context(), app.user.clone(), app.meta.clone());
    async move {
        CreateRecipe::new(ctx)
            .execute(
                &user,
                &meta,
                CreateRecipeInput {
                    machine_code: code,
                    reference: ReferenceNumber::parse("ENC-1").unwrap(),
                    name: text("Recipe"),
                    change_note: long_text("First version"),
                },
            )
            .await
    }
}

/// Read one text column of one row as the superuser.
async fn text_of(app: &mut App, query: &str, id: Uuid) -> String {
    sqlx::query_scalar(query)
        .bind(id)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

/// The change holds the machine lock. `CreateRecipe` waits and then creates its recipe on a `Utf8` machine.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adr_0202_encoding_change_and_create_recipe_serialize_when_the_change_goes_first() {
    let mut app = App::create_with(&gate::pool()).await;
    let machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let change = set_utf8(&app, machine.code);
    let recipe = create_recipe(&app, machine.code);
    app.install_gate("machine", "UPDATE", SETTING_UTF8).await;

    let (change, recipe) = gate::run(&mut app, change, recipe).await;

    assert_no_deadlock(&change);
    assert_no_deadlock(&recipe);
    assert_eq!(change.unwrap(), StringEncoding::Utf8);
    let recipe = recipe.unwrap().recipe;
    assert_eq!(
        text_of(&mut app, RECIPE_MACHINE_ENCODING, *recipe.as_uuid()).await,
        "Utf8"
    );
    assert_eq!(app.count(RECIPE_ROWS, machine.id).await, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(CHANGE_ENTRIES)
            .fetch_one(app.admin())
            .await
            .unwrap(),
        1
    );
}

/// `CreateRecipe` holds the shared machine lock. The change waits and then finds the version row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adr_0202_encoding_change_and_create_recipe_serialize_when_the_recipe_goes_first() {
    let mut app = App::create_with(&gate::pool()).await;
    let machine = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let recipe = create_recipe(&app, machine.code);
    let change = set_utf8(&app, machine.code);
    app.install_gate("recipe", "INSERT", "true").await;

    let (recipe, change) = gate::run(&mut app, recipe, change).await;

    assert_no_deadlock(&recipe);
    assert_no_deadlock(&change);
    recipe.unwrap();
    assert_eq!(error_code(&change), ErrorCode::PLT_054);
    assert_eq!(
        text_of(&mut app, MACHINE_ENCODING, machine.id).await,
        "Latin1"
    );
    assert_eq!(app.count(RECIPE_ROWS, machine.id).await, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(CHANGE_ENTRIES)
            .fetch_one(app.admin())
            .await
            .unwrap(),
        0
    );
}
