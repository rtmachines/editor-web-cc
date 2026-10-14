//! Shared fixtures of the atomic tests of the recipe and draft commands (tasks 10.2 and 10.3).
//!
//! A `World` is a `TwinCat3` machine with the unit `U1`, one recipe, and its live draft. The rows
//! come from `recipe_migrator`, so a test fails in the command and not in the seed.

use recipe_app::auth::CurrentUser;
use recipe_domain::identity::{Login, Role, UserId};
use recipe_domain::recipe::{ParameterKey, ReferenceNumber};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::support::app::App;
use crate::support::fixtures::{self, TWIN_CAT};

/// Counts the live and the deleted draft versions of a recipe.
pub const DRAFT_ROWS: &str = "SELECT count(*) FROM recipe_version WHERE recipe_id = $1 \
                              AND status = 'draft' AND deleted_at IS NULL";

/// Parse a short text.
pub fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// Parse a long text.
pub fn long_text(value: &str) -> LongText {
    LongText::parse(value).unwrap()
}

/// A parameter of the draft.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    /// The row identifier.
    pub id: Uuid,
    /// The key in the URL.
    pub key: ParameterKey,
}

/// A machine, a recipe, and its live draft.
#[derive(Debug, Clone)]
pub struct World {
    /// The machine.
    pub machine: Uuid,
    /// The unit `U1`.
    pub unit: Uuid,
    /// The recipe.
    pub recipe: Uuid,
    /// The live draft, number 1.
    pub version: Uuid,
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The user who created the recipe and the draft.
    pub creator: Uuid,
}

impl World {
    /// Insert the machine, the recipe, and an empty live draft created by the user of `app`.
    pub async fn create(app: &mut App) -> Self {
        let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
        let creator = *app.user.user().as_uuid();
        let unit: Uuid = sqlx::query_scalar("SELECT id FROM unit WHERE machine_id = $1")
            .bind(seed.id)
            .fetch_one(app.migrator())
            .await
            .unwrap();
        let recipe = fixtures::recipe(app.migrator(), seed.id, TWIN_CAT, creator).await;
        let version = fixtures::draft_version(app.migrator(), recipe, TWIN_CAT, 1, creator).await;
        let reference: String =
            sqlx::query_scalar("SELECT reference_number FROM recipe WHERE id = $1")
                .bind(recipe)
                .fetch_one(app.migrator())
                .await
                .unwrap();
        Self {
            machine: seed.id,
            unit,
            recipe,
            version,
            reference: ReferenceNumber::parse(&reference).unwrap(),
            creator,
        }
    }

    /// Like `create`, plus the `DINT` parameter `SPEED` with the limit 0 to 10 and a translation.
    pub async fn with_parameter(app: &mut App) -> (Self, Param) {
        let world = Self::create(app).await;
        let param = world.add_speed(app, world.version, "SPEED").await;
        (world, param)
    }

    /// Insert a `DINT` parameter with a limit and a Polish translation into a draft.
    pub async fn add_speed(&self, app: &mut App, version: Uuid, tag: &str) -> Param {
        let dint = fixtures::primitive(app.migrator(), "DINT").await;
        let id = fixtures::parameter(app.migrator(), version, TWIN_CAT, self.unit, dint, tag).await;
        fixtures::limit(app.migrator(), id, version, "").await;
        fixtures::translation(app.migrator(), id, version, "pl").await;
        let key: Uuid = sqlx::query_scalar("SELECT parameter_key FROM parameter WHERE id = $1")
            .bind(id)
            .fetch_one(app.migrator())
            .await
            .unwrap();
        Param {
            id,
            key: ParameterKey::from(key),
        }
    }

    /// The row version of the live draft.
    pub async fn row_version(&self, app: &mut App) -> i64 {
        row_version(app, self.version).await
    }
}

/// Read the row version of a version.
pub async fn row_version(app: &mut App, version: Uuid) -> i64 {
    app.count(
        "SELECT row_version FROM recipe_version WHERE id = $1",
        version,
    )
    .await
}

/// Insert a second administrator, who can approve what the user of `app` created.
pub async fn second_user(app: &mut App) -> CurrentUser {
    let id = fixtures::user(app.migrator()).await;
    let login: String = sqlx::query_scalar("SELECT login FROM app_user WHERE id = $1")
        .bind(id)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    CurrentUser::new(
        UserId::from(id),
        Login::parse(&login).unwrap(),
        Role::Administrator,
    )
}

/// The status of a version.
pub async fn status(app: &mut App, version: Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM recipe_version WHERE id = $1")
        .bind(version)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

/// Count the audit rows of an action.
pub async fn actions_named(app: &mut App, action: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action = $1")
        .bind(action)
        .fetch_one(app.admin())
        .await
        .unwrap()
}
