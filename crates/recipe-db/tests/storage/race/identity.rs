//! Identity races (task 5.8, design D19, `reliability.md` section 6, row "Concurrency").
//!
//! Each test makes the order of two or more commands certain without a sleep. A test trigger of
//! the superuser makes a command wait at a known statement (the gate of `support::gate`), and a
//! poll of `pg_locks` or `pg_stat_activity` proves that the other command reached its own point.
//! Only then the test opens the gate. A test trigger that calls `nextval` on a sequence counts
//! the runs of a command, because a sequence ignores the rollback.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use recipe_app::error::AppError;
use recipe_app::identity::{
    Authenticated, BlockingHasher, BootstrapOutcome, ChangeOwnPassword, ChangeOwnPasswordInput,
    DeactivateUser, HashGate, IdentityContext, Login, LoginInput, ResetPassword, SetUserRole,
    SetUserRoleInput, authenticate, bootstrap_administrator,
};
use recipe_app::ports::{AttemptKey, AttemptOutcome};
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{
    Login as UserLogin, NewPassword, Password, Role, SessionId, ThrottlePolicy, TypedLogin,
};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::support::app::App;
use crate::support::gate::{self, assert_no_deadlock, error_code};
use crate::support::identity::{FastHasher, PASSWORD, PERMIT_WAIT};

/// The address of the requests.
const CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 9, 9, 9));
/// The budget of one address in the window (`reliability.md` section 7.11).
const ADDRESS_BUDGET: i32 = 100;
/// A session waits for the class-1 advisory lock of a login in this database.
const WAITING_FOR_LOGIN_LOCK: &str = "SELECT count(*) FROM pg_locks \
    WHERE locktype = 'advisory' AND NOT granted AND classid = 1::oid AND objsubid = 2 \
      AND database = (SELECT oid FROM pg_database WHERE datname = current_database())";
/// The failed attempts of the address of the tests.
const ADDRESS_FAILURES: &str =
    "SELECT count(*) FROM login_attempt WHERE client_ip = '10.9.9.9'::inet AND NOT succeeded";
/// The number of active Administrators.
const ACTIVE_ADMINISTRATORS: &str =
    "SELECT count(*) FROM app_user WHERE role = 'administrator' AND is_active";
/// The sessions of the user `anna`.
const ANNA_SESSIONS: &str =
    "SELECT count(*) FROM session s JOIN app_user u ON u.id = s.user_id WHERE u.login = 'anna'";
/// A session waits for the guard lock of the last Administrator (class 4, key 0) in this database.
const WAITING_FOR_GUARD_LOCK: &str = "SELECT count(*) FROM pg_locks \
    WHERE locktype = 'advisory' AND NOT granted AND classid = 4::oid AND objid = 0::oid \
      AND objsubid = 2 \
      AND database = (SELECT oid FROM pg_database WHERE datname = current_database())";
/// The sessions of the user `target`.
const TARGET_SESSIONS: &str =
    "SELECT count(*) FROM session s JOIN app_user u ON u.id = s.user_id WHERE u.login = 'target'";
/// The audit entries of a role change.
const AUDIT_ROLE_SET: &str = "SELECT count(*) FROM audit_log WHERE action = 'UserRoleSet'";
/// The audit entries of a deactivation.
const AUDIT_DEACTIVATED: &str = "SELECT count(*) FROM audit_log WHERE action = 'UserDeactivated'";
/// A password that meets the policy.
const NEW_PASSWORD: &str = "Replacement-password-2024";

fn login(text: &str) -> UserLogin {
    UserLogin::parse(text).unwrap()
}

fn attempt_key(login: &str) -> AttemptKey {
    AttemptKey {
        login: TypedLogin::parse(login).unwrap(),
        client_ip: CLIENT,
    }
}

fn login_input(name: &str, old_session: Option<SessionId>) -> LoginInput {
    LoginInput {
        login: TypedLogin::parse(name).unwrap(),
        password: Password::parse(PASSWORD).unwrap(),
        client_ip: CLIENT,
        old_session,
    }
}

// Race 1: two failed logins of one login count each other.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_failed_logins_of_one_login_count_each_other_and_both_rows_exist() {
    let mut app = App::create_with(&gate::pool()).await;
    let store = app.attempts();
    app.install_gate("login_attempt", "INSERT", "true").await;
    app.close_gate().await;
    let policy = ThrottlePolicy::default();
    let start = move |store: Arc<dyn recipe_app::ports::LoginAttemptStore>| async move {
        store.begin_attempt(&attempt_key("anna"), &policy).await
    };

    // The first attempt holds the class-1 lock of the login and waits before its insert.
    let first = tokio::spawn(start(store.clone()));
    app.wait_until_at_gate().await;
    let second = tokio::spawn(start(store));
    app.poll_at_least(
        WAITING_FOR_LOGIN_LOCK,
        1,
        "the second attempt waits for the lock",
    )
    .await;
    app.open_gate().await;

    let first: AttemptOutcome = first.await.unwrap().unwrap();
    let second: AttemptOutcome = second.await.unwrap().unwrap();
    assert_eq!(first.count.pair_failures, 0);
    assert_eq!(second.count.pair_failures, 1);
    assert_eq!(second.count.address_failures, 1);
    assert!(first.attempt.is_some() && second.attempt.is_some());
    assert_eq!(app.rows(ADDRESS_FAILURES).await, 2);
}

// Race 2: the budget of an address holds up to the number of permits.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_failed_logins_of_other_logins_pass_the_budget_by_at_most_the_permits() {
    let mut app = App::create_with(&gate::pool()).await;
    let permits = 4_usize;
    let ctx = app.identity_with(Arc::new(FastHasher), permits, PERMIT_WAIT);
    app.seed_attempts("seeded", "10.9.9.9", ADDRESS_BUDGET - 1)
        .await;
    app.install_gate("login_attempt", "INSERT", "true").await;
    app.close_gate().await;

    // Ten unknown logins from one address. Four of them get a permit and wait before the insert.
    let calls: Vec<_> = (0..10)
        .map(|n| {
            let (ctx, meta) = (ctx.clone(), app.meta.clone());
            let input = login_input(&format!("burst-{n}"), None);
            tokio::spawn(async move { Login::new(ctx).execute(&meta, input).await })
        })
        .collect();
    app.wait_until_gate_holds(i64::try_from(permits).unwrap())
        .await;
    assert_eq!(
        app.rows(ADDRESS_FAILURES).await,
        i64::from(ADDRESS_BUDGET - 1),
        "no attempt inserted its row yet"
    );
    app.open_gate().await;

    let mut codes = Vec::new();
    for call in calls {
        codes.push(error_code(&call.await.unwrap()));
    }
    let failed = codes.iter().filter(|c| **c == ErrorCode::IDN_003).count();
    let throttled = codes.iter().filter(|c| **c == ErrorCode::IDN_005).count();
    assert_eq!((failed, throttled), (permits, 10 - permits), "{codes:?}");
    let rows = app.rows(ADDRESS_FAILURES).await;
    assert!(rows <= i64::from(ADDRESS_BUDGET) + 3, "{rows} rows");
    assert_eq!(rows, i64::from(ADDRESS_BUDGET - 1) + 4);
}

// Race 3: two Administrators demote each other.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_second_demotion_waits_for_the_guard_lock_and_then_is_refused_for_the_demoted_actor() {
    let mut app = App::create_with(&gate::pool()).await;
    let ctx = app.identity();
    let first_admin = app.admin_session(&ctx).await;
    let first_login = first_admin.user().login().clone();
    let second = app
        .seed_user("second-admin", Role::Administrator, PASSWORD)
        .await;
    let token = app.seed_session(second).await;
    let second_admin = app.sign_in(&ctx, &token).await;
    // The first command holds the guard lock and the row of its target. It waits before its update.
    app.install_gate("app_user", "UPDATE", "true").await;
    app.close_gate().await;

    let one = demote(&ctx, &app, first_admin, login("second-admin"));
    app.wait_until_at_gate().await;
    let two = demote(&ctx, &app, second_admin, first_login);
    app.poll_at_least(
        WAITING_FOR_GUARD_LOCK,
        1,
        "the second demotion waits for the guard lock",
    )
    .await;
    app.open_gate().await;

    let results = [one.await.unwrap(), two.await.unwrap()];
    assert!(results[0].is_ok(), "{results:?}");
    // The first command demoted the actor of the second one, so the second one loses the permission.
    assert_eq!(error_code(&results[1]), ErrorCode::WEB_002);
    assert_eq!(app.rows(ACTIVE_ADMINISTRATORS).await, 1);
    assert_eq!(app.rows(AUDIT_ROLE_SET).await, 1);
}

/// Start `SetUserRole` that demotes `target` to Operator.
fn demote(
    ctx: &IdentityContext,
    app: &App,
    auth: Authenticated,
    target: UserLogin,
) -> JoinHandle<Result<(), AppError>> {
    let (ctx, meta) = (ctx.clone(), app.meta.clone());
    tokio::spawn(async move {
        let input = SetUserRoleInput {
            login: target,
            role: Role::Operator,
        };
        SetUserRole::new(ctx).execute(&auth, &meta, input).await
    })
}

// Race 3a: an Administrator who is demoted while the command of that Administrator waits.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_administrator_demoted_during_the_wait_for_the_guard_lock_gets_forbidden() {
    let mut app = App::create_with(&gate::pool()).await;
    let ctx = app.identity();
    let first_admin = app.admin_session(&ctx).await;
    let first_login = first_admin.user().login().clone();
    let second = app
        .seed_user("second-admin", Role::Administrator, PASSWORD)
        .await;
    let token = app.seed_session(second).await;
    let second_admin = app.sign_in(&ctx, &token).await;
    app.seed_user("target", Role::Operator, PASSWORD).await;
    // The second Administrator demotes the first one and waits before its update.
    app.install_gate("app_user", "UPDATE", "true").await;
    app.close_gate().await;

    let demotion = demote(&ctx, &app, second_admin, first_login);
    app.wait_until_at_gate().await;
    // The first Administrator passed the permission check and now waits for the guard lock.
    let command = set_role(
        &ctx,
        &app,
        first_admin,
        login("target"),
        Role::ProcessEngineer,
    );
    app.poll_at_least(
        WAITING_FOR_GUARD_LOCK,
        1,
        "the command of the first Administrator waits for the guard lock",
    )
    .await;
    app.open_gate().await;

    demotion.await.unwrap().unwrap();
    let result = command.await.unwrap();
    assert_eq!(error_code(&result), ErrorCode::WEB_002);
    assert_eq!(app.rows(AUDIT_ROLE_SET).await, 1, "only the demotion");
    assert_eq!(
        app.text("SELECT role FROM app_user WHERE login = 'target'")
            .await,
        "operator"
    );
}

// Race 3b: three Administrators, two deactivations that both commit in turn.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_second_of_two_deactivations_waits_for_the_guard_lock_and_then_commits() {
    let mut app = App::create_with(&gate::pool()).await;
    let ctx = app.identity();
    let first_admin = app.admin_session(&ctx).await;
    let first_login = first_admin.user().login().clone();
    app.seed_user("second-admin", Role::Administrator, PASSWORD)
        .await;
    let third = app
        .seed_user("third-admin", Role::Administrator, PASSWORD)
        .await;
    let token = app.seed_session(third).await;
    let third_admin = app.sign_in(&ctx, &token).await;
    // The first command deactivates the second Administrator and waits before its update.
    app.install_gate("app_user", "UPDATE", "true").await;
    app.close_gate().await;

    let one = deactivate(&ctx, &app, first_admin, login("second-admin"));
    app.wait_until_at_gate().await;
    let two = deactivate(&ctx, &app, third_admin, first_login);
    app.poll_at_least(
        WAITING_FOR_GUARD_LOCK,
        1,
        "the second deactivation waits for the guard lock",
    )
    .await;
    app.open_gate().await;

    one.await.unwrap().unwrap();
    two.await.unwrap().unwrap();
    assert_eq!(app.rows(ACTIVE_ADMINISTRATORS).await, 1);
    assert_eq!(
        app.text("SELECT login FROM app_user WHERE is_active AND role = 'administrator'")
            .await,
        "third-admin"
    );
    assert_eq!(app.rows(AUDIT_DEACTIVATED).await, 2);
}

/// Start `DeactivateUser` on `target`.
fn deactivate(
    ctx: &IdentityContext,
    app: &App,
    auth: Authenticated,
    target: UserLogin,
) -> JoinHandle<Result<(), AppError>> {
    let (ctx, meta) = (ctx.clone(), app.meta.clone());
    tokio::spawn(async move {
        DeactivateUser::new(ctx)
            .execute(&auth, &meta, &target)
            .await
    })
}

/// Start `SetUserRole` that sets `role` for `target`.
fn set_role(
    ctx: &IdentityContext,
    app: &App,
    auth: Authenticated,
    target: UserLogin,
    role: Role,
) -> JoinHandle<Result<(), AppError>> {
    let (ctx, meta) = (ctx.clone(), app.meta.clone());
    tokio::spawn(async move {
        let input = SetUserRoleInput {
            login: target,
            role,
        };
        SetUserRole::new(ctx).execute(&auth, &meta, input).await
    })
}

// Race 4: a command on a target that sends requests.

/// The command that the test runs on the user `target`.
#[derive(Clone, Copy)]
enum Change {
    SetRole,
    Deactivate,
}

/// Run `change` on `target`, hold it before its update, and let `target` touch its session
/// three times. Each touch commits. The command then needs one run and ends every session.
async fn touch_during_change(change: Change) -> App {
    let mut app = App::create_with(&gate::pool()).await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let target = app.seed_user("target", Role::Operator, PASSWORD).await;
    let token = app.seed_session(target).await;
    app.install_run_counter().await;
    app.install_counted_gate("app_user", "BEFORE", "UPDATE", "true")
        .await;
    app.close_gate().await;
    let command = match change {
        Change::SetRole => set_role(&ctx, &app, auth, login("target"), Role::ProcessEngineer),
        Change::Deactivate => deactivate(&ctx, &app, auth, login("target")),
    };
    // The command holds the row of the user and waits before its update.
    app.wait_until_at_gate().await;

    for _ in 0..3 {
        let touched = authenticate(&ctx, &token).await.unwrap();
        assert!(touched.is_some(), "the session is valid until the commit");
    }
    app.open_gate().await;

    let result = command.await.unwrap();
    assert_no_deadlock(&result);
    result.unwrap();
    assert_eq!(app.counted_runs().await, 1, "one run, no retry");
    assert_eq!(app.rows(TARGET_SESSIONS).await, 0);
    assert!(authenticate(&ctx, &token).await.unwrap().is_none());
    app
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deactivate_user_runs_once_while_the_target_session_sends_requests() {
    let mut app = touch_during_change(Change::Deactivate).await;

    assert_eq!(
        app.text("SELECT is_active::text FROM app_user WHERE login = 'target'")
            .await,
        "false"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_user_role_runs_once_while_the_target_session_sends_requests() {
    let mut app = touch_during_change(Change::SetRole).await;

    assert_eq!(
        app.text("SELECT role FROM app_user WHERE login = 'target'")
            .await,
        "process_engineer"
    );
}

// Race 5: a touch after the delete of the sessions.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_touch_after_the_delete_waits_for_the_commit_and_finds_no_session() {
    let mut app = App::create_with(&gate::pool()).await;
    let ctx = app.identity();
    let auth = app.admin_session(&ctx).await;
    let target = app.seed_user("target", Role::Operator, PASSWORD).await;
    let token = app.seed_session(target).await;
    // The command deleted the session rows and waits before its audit insert.
    app.install_gate("audit_log", "INSERT", "true").await;
    app.close_gate().await;
    let command = set_role(&ctx, &app, auth, login("target"), Role::ProcessEngineer);
    app.wait_until_at_gate().await;

    let touch_ctx = ctx.clone();
    let touch = tokio::spawn(async move { authenticate(&touch_ctx, &token).await });
    app.wait_until_blocked_by_gate().await;
    app.open_gate().await;

    command.await.unwrap().unwrap();
    assert!(touch.await.unwrap().unwrap().is_none());
    assert_eq!(app.rows(AUDIT_ROLE_SET).await, 1);
}

// Race 6: a reset commits between the check and the final transaction.

/// Seed `anna` with a session, an Administrator session, and a blocked context.
struct ResetScene {
    app: App,
    blocked: recipe_app::identity::IdentityContext,
    hasher: Arc<BlockingHasher>,
    admin: recipe_app::identity::Authenticated,
    token: recipe_domain::identity::SessionToken,
    original_hash: String,
}

async fn reset_scene() -> ResetScene {
    let mut app = App::create_with(&gate::pool()).await;
    let fast = app.identity();
    let admin = app.admin_session(&fast).await;
    let anna = app.seed_user("anna", Role::Operator, PASSWORD).await;
    let token = app.seed_session(anna).await;
    let hasher = Arc::new(BlockingHasher::new());
    let blocked = app.identity_with(hasher.clone(), 4, PERMIT_WAIT);
    let original_hash = app.password_hash_of("anna").await;
    ResetScene {
        app,
        blocked,
        hasher,
        admin,
        token,
        original_hash,
    }
}

impl ResetScene {
    /// Wait until the blocked use case sits in its password check, then reset the password.
    async fn reset_while_checking(&mut self) -> String {
        assert!(self.hasher.wait_started(1).await, "the check started");
        let fast = self.app.identity();
        ResetPassword::new(fast)
            .execute(&self.admin, &self.app.meta, &login("anna"))
            .await
            .unwrap();
        let reset_hash = self.app.password_hash_of("anna").await;
        assert_ne!(reset_hash, self.original_hash);
        reset_hash
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_after_a_reset_that_commits_in_between_creates_no_session() {
    let mut scene = reset_scene().await;
    let (ctx, meta) = (scene.blocked.clone(), scene.app.meta.clone());
    let input = login_input("anna", None);
    let attempt = tokio::spawn(async move { Login::new(ctx).execute(&meta, input).await });

    let reset_hash = scene.reset_while_checking().await;
    scene.hasher.release();

    let result = attempt.await.unwrap();
    assert_eq!(error_code(&result), ErrorCode::IDN_003);
    assert_eq!(scene.app.rows(ANNA_SESSIONS).await, 0);
    assert_eq!(scene.app.password_hash_of("anna").await, reset_hash);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn change_own_password_after_a_reset_that_commits_in_between_keeps_the_reset() {
    let mut scene = reset_scene().await;
    let auth = scene.app.sign_in(&scene.blocked, &scene.token).await;
    let (ctx, meta) = (scene.blocked.clone(), scene.app.meta.clone());
    let input = ChangeOwnPasswordInput {
        client_ip: CLIENT,
        current_password: Password::parse(PASSWORD).unwrap(),
        new_password: Password::parse(NEW_PASSWORD).unwrap(),
    };
    let attempt = tokio::spawn(async move {
        ChangeOwnPassword::new(ctx)
            .execute(&auth, &meta, input)
            .await
    });

    let reset_hash = scene.reset_while_checking().await;
    scene.hasher.release();

    let result = attempt.await.unwrap();
    assert_eq!(error_code(&result), ErrorCode::WEB_004);
    assert_eq!(scene.app.rows(ANNA_SESSIONS).await, 0);
    assert_eq!(scene.app.password_hash_of("anna").await, reset_hash);
}

// Race 7: two parallel create-admin runs.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_parallel_bootstraps_with_two_logins_create_one_administrator() {
    let mut app = App::create_with(&gate::pool()).await;
    sqlx::query("DELETE FROM app_user")
        .execute(app.admin())
        .await
        .unwrap();
    let hash_gate = HashGate::new(4, PERMIT_WAIT, Arc::new(FastHasher));
    let uow = app.unit_of_work();
    let run = |name: &'static str| {
        let (uow, hash_gate) = (uow.clone(), hash_gate.clone());
        tokio::spawn(async move {
            bootstrap_administrator(
                uow.as_ref(),
                &hash_gate,
                login(name),
                NewPassword::parse(NEW_PASSWORD).unwrap(),
                RequestId::from(Uuid::new_v4()),
            )
            .await
        })
    };
    // Both runs count zero Administrators, and both wait before their insert.
    app.install_gate("app_user", "INSERT", "true").await;
    // The second gate orders the commits, as in race 3.
    app.install_second_gate("audit_log", "INSERT", "true").await;
    app.close_gate().await;
    app.close_second_gate().await;

    let one = run("first-admin");
    let two = run("second-admin");
    app.wait_until_gate_holds(2).await;
    app.open_gate().await;
    app.wait_until_at_second_gate().await;
    app.wait_until_blocked_by_gate().await;
    app.open_second_gate().await;

    let outcomes: [Result<BootstrapOutcome, AppError>; 2] =
        [one.await.unwrap(), two.await.unwrap()];
    let created = outcomes
        .iter()
        .filter(|o| matches!(o, Ok(BootstrapOutcome::Created)))
        .count();
    let existing = outcomes
        .iter()
        .filter(|o| matches!(o, Ok(BootstrapOutcome::AdministratorExists)))
        .count();
    assert_eq!((created, existing), (1, 1), "{outcomes:?}");
    assert_eq!(app.rows(ACTIVE_ADMINISTRATORS).await, 1);
}
