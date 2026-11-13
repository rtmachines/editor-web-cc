//! Race of two `DefineUdt` revisions with the same changed members (ADR-0159 (c), E-PLT-052).
//!
//! Both commands take the class-2 advisory key of the struct name before they read the type
//! rows. The first command inserts revision 2 and waits at the audit insert. The second command
//! waits for the class-2 key of the first. After the first commit the second command reads
//! revision 2, finds the same members, and ends with `E-PLT-052`. A command that compared the
//! members before it took the lock would insert a second revision 2.

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::context::AppContext;
use recipe_app::error::AppError;
use recipe_app::plant::{DefineUdt, DefineUdtInput, DefineUdtMode, DefineUdtOutput};
use recipe_domain::plant::{MemberName, PlantError, TypeExpr, TypeName};

use crate::support::app::{App, Seeded};
use crate::support::fixtures::TWIN_CAT;
use crate::support::gate;

/// The name of the struct.
const STRUCT_NAME: &str = "ST_X";
/// The number of commands that wait: one at the gate and one for the class-2 key.
const WAITING_COMMANDS: i64 = 2;

/// The revision numbers of the struct rows of the machine, in ascending order.
const STRUCT_REVISIONS: &str = "SELECT revision FROM plc_type \
    WHERE machine_id = $1 AND kind = 'struct' AND name = 'ST_X' ORDER BY revision";
/// The number of struct rows of the machine.
const STRUCT_ROWS: &str = "SELECT count(*) FROM plc_type WHERE machine_id = $1 AND kind = 'struct'";

/// The input of `DefineUdt` for `ST_X` with the members `(name, type)` in `mode`.
fn define_input(seed: &Seeded, members: &[(&str, &str)], mode: DefineUdtMode) -> DefineUdtInput {
    DefineUdtInput {
        code: seed.code,
        name: TypeName::parse(STRUCT_NAME).unwrap(),
        members: members
            .iter()
            .map(|(name, expr)| {
                (
                    MemberName::parse(name).unwrap(),
                    TypeExpr::parse(expr).unwrap(),
                )
            })
            .collect(),
        mode,
    }
}

/// Start `DefineUdt` as the user of the fixture.
fn define(
    app: &App,
    input: DefineUdtInput,
) -> impl Future<Output = Result<DefineUdtOutput, AppError>> + Send + 'static {
    let context: AppContext = app.context();
    let user: CurrentUser = app.user.clone();
    let meta: RequestMeta = app.meta.clone();
    async move { DefineUdt::new(context).execute(&user, &meta, input).await }
}

/// Two revisions with the same changed members run at one time. One command creates revision 2
/// and the other gets `E-PLT-052` for revision 2.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_equal_revisions_create_one_revision_and_give_plt_052_for_the_other() {
    let mut app = App::create_with(&gate::pool()).await;
    let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
    let first_revision = define_input(&seed, &[("label", "INT")], DefineUdtMode::New);
    assert_eq!(define(&app, first_revision).await.unwrap().revision, 1);
    let before = app.head().await;
    // The first command holds its uncommitted revision and waits before its audit insert.
    app.install_gate("audit_log", "INSERT", "true").await;

    let changed = [("label", "INT"), ("extra", "DINT")];
    let (left, right) = (
        define(&app, define_input(&seed, &changed, DefineUdtMode::Revision)),
        define(&app, define_input(&seed, &changed, DefineUdtMode::Revision)),
    );
    app.close_gate().await;
    let first = tokio::spawn(left);
    app.wait_until_at_gate().await;
    let second = tokio::spawn(right);
    // The second command waits for the class-2 key of the first command.
    app.wait_until_gate_holds(WAITING_COMMANDS).await;
    app.open_gate().await;
    let (first, second) = (first.await.unwrap(), second.await.unwrap());

    let created = first.unwrap();
    assert_eq!(created.revision, 2);
    let error = second.unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Plant(PlantError::RevisionUnchanged { type_name, revision: 2 })
                if type_name.as_str() == STRUCT_NAME
        ),
        "{error:?}"
    );
    assert_eq!(app.count(STRUCT_ROWS, seed.id).await, 2);
    let revisions: Vec<i32> = sqlx::query_scalar(STRUCT_REVISIONS)
        .bind(seed.id)
        .fetch_all(app.admin())
        .await
        .unwrap();
    assert_eq!(revisions, [1, 2]);
    app.assert_chain_moved(&before, &["UdtDefined"]).await;
}
