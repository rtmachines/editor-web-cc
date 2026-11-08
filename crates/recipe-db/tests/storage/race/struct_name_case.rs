//! Race of two `DefineUdt` commands whose struct names differ only in letter case
//! (ADR-0143 (j), `design.md` D4, spec scenario "Two names in two cases at one time").
//!
//! The class-2 advisory keys differ by case, so the two commands do not wait for each other and
//! both read a machine without the struct. The first command inserts its row and waits at the
//! audit insert. The second command then waits for the unique index entry of the first row.
//! After the first commit the index `plc_type_struct_name_ci_unique` rejects the second row.
//! The index is the only guard, and the adapter maps the violation to `E-PLT-015`.

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::context::AppContext;
use recipe_app::error::AppError;
use recipe_app::plant::{DefineUdt, DefineUdtInput, DefineUdtMode, DefineUdtOutput};
use recipe_domain::plant::{MemberName, PlantError, TypeExpr, TypeName};

use crate::support::app::{App, Seeded};
use crate::support::fixtures::TWIN_CAT;
use crate::support::gate;

/// The struct rows of the machine, with the name as stored.
const STRUCT_NAMES: &str = "SELECT name FROM plc_type WHERE machine_id = $1 AND kind = 'struct'";
/// The number of struct rows of the machine.
const STRUCT_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1 AND kind = 'struct'";

/// The input of a new struct `name` with one `INT` member on the seeded machine.
fn define_input(seed: &Seeded, name: &str) -> DefineUdtInput {
    DefineUdtInput {
        code: seed.code,
        name: TypeName::parse(name).unwrap(),
        members: vec![(
            MemberName::parse("label").unwrap(),
            TypeExpr::parse("INT").unwrap(),
        )],
        mode: DefineUdtMode::New,
    }
}

/// Start `DefineUdt` for a new struct `name` as the user of the fixture.
fn define(
    app: &App,
    seed: &Seeded,
    name: &str,
) -> impl Future<Output = Result<DefineUdtOutput, AppError>> + Send + 'static {
    let context: AppContext = app.context();
    let user: CurrentUser = app.user.clone();
    let meta: RequestMeta = app.meta.clone();
    let input = define_input(seed, name);
    async move { DefineUdt::new(context).execute(&user, &meta, input).await }
}

/// `ST_Zone` and `st_zone` run at one time. One command creates its struct and the other gets
/// `E-PLT-015` from the index.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_new_structs_in_two_cases_create_one_struct_and_give_plt_015_for_the_other() {
    let mut app = App::create_with(&gate::pool()).await;
    let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let before = app.head().await;
    // The first command holds its uncommitted struct row and waits before its audit insert.
    app.install_gate("audit_log", "INSERT", "true").await;

    let (upper, lower) = (
        define(&app, &seed, "ST_Zone"),
        define(&app, &seed, "st_zone"),
    );
    let (first, second) = gate::run(&mut app, upper, lower).await;

    let created = first.unwrap();
    assert_eq!(created.revision, 1);
    let error = second.unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Plant(PlantError::DuplicateTypeName { type_name })
                if type_name.as_str() == "st_zone"
        ),
        "{error:?}"
    );
    assert_eq!(app.count(STRUCT_ROWS, seed.id).await, 1);
    let stored: Vec<String> = sqlx::query_scalar(STRUCT_NAMES)
        .bind(seed.id)
        .fetch_all(app.admin())
        .await
        .unwrap();
    assert_eq!(stored, ["ST_Zone"]);
    app.assert_chain_moved(&before, &["UdtDefined"]).await;
}
