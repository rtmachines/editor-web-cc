//! The actors, the requests, and the commands of the scenario.
//!
//! An engineer creates and edits the drafts. The administrator of `App` approves, retires, and
//! exports the versions. Each command gets a new request. The scene records the request, the
//! actor, and the audit actions that the command must write.

use std::net::{IpAddr, Ipv4Addr};

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::export::{ExportByNumberInput, ExportVersionByNumber};
use recipe_app::identity::new_request_id;
use recipe_app::ports::VersionSummary;
use recipe_app::recipe::{
    AddParameter, AddParameterInput, ApproveVersion, ApproveVersionInput, ApproveVersionOutput,
    CreateRecipe, CreateRecipeInput, CreateVersion, CreateVersionInput, CreateVersionOutput,
    GetVersion, GetVersionInput, ListVersions, ListVersionsInput, RetireVersion,
    RetireVersionInput, SetParameterValue, SetParameterValueInput,
};
use recipe_domain::identity::{Login, Role, UserId};
use recipe_domain::plant::{LeafPath, MachineCode, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterKey, ParameterTag, RecipeVersionId, ReferenceNumber,
    ValueInput, VersionState,
};
use uuid::Uuid;

use crate::atomic::world::{long_text, text};
use crate::support::app::App;
use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::identity::PASSWORD;

/// The login of the engineer.
const ENGINEER: &str = "engineer";
/// The reference number of the recipe.
const REFERENCE: &str = "SCN-1";
/// The client address of every request.
const CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8));

/// One command of the scenario, as the audit chain must show it.
#[derive(Debug, Clone)]
pub struct Step {
    /// The request identifier of the command.
    pub request: Uuid,
    /// The user who sent the command.
    pub actor: Uuid,
    /// The audit actions of the command, in the order of the chain.
    pub actions: Vec<&'static str>,
}

/// A database with a machine, an engineer, an administrator, and the steps so far.
pub struct Scene {
    /// The test database and the administrator.
    pub app: App,
    /// The steps that the commands recorded, in the order of the commands.
    pub steps: Vec<Step>,
    engineer: CurrentUser,
    machine: MachineCode,
    reference: ReferenceNumber,
    key: Option<ParameterKey>,
}

/// Seed a process engineer through the identity fixture and return the user.
async fn engineer(app: &mut App) -> CurrentUser {
    let id = app
        .seed_user(ENGINEER, Role::ProcessEngineer, PASSWORD)
        .await;
    CurrentUser::new(id, Login::parse(ENGINEER).unwrap(), Role::ProcessEngineer)
}

/// The data of a new request from the client address of the scenario.
fn new_meta() -> RequestMeta {
    RequestMeta::new(new_request_id().unwrap(), Some(CLIENT_IP), None, None)
}

/// The limit 0 to 10 of the scalar leaf of `SPEED`.
fn speed_limits() -> LimitList {
    LimitList::parse(vec![LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some("0".to_owned()),
        max: Some("10".to_owned()),
        max_length: None,
    }])
    .unwrap()
}

impl Scene {
    /// Create the database with a `TwinCat3` machine, the unit `U1`, `DINT`, and an engineer.
    pub async fn create() -> Self {
        let mut app = App::create().await;
        let seed = app.seed_machine(TWIN_CAT, &["U1"]).await;
        fixtures::primitive(app.migrator(), "DINT").await;
        let engineer = engineer(&mut app).await;
        Self {
            app,
            steps: Vec::new(),
            engineer,
            machine: seed.code,
            reference: ReferenceNumber::parse(REFERENCE).unwrap(),
            key: None,
        }
    }

    /// Record a step of `actor` with its actions and return the data of its new request.
    fn begin(&mut self, actor: UserId, actions: &[&'static str]) -> RequestMeta {
        let meta = new_meta();
        self.steps.push(Step {
            request: *meta.request_id().as_uuid(),
            actor: *actor.as_uuid(),
            actions: actions.to_vec(),
        });
        meta
    }

    /// The engineer creates the recipe. Return the identifier of version 1.
    pub async fn create_recipe(&mut self, actions: &[&'static str]) -> RecipeVersionId {
        let meta = self.begin(self.engineer.user(), actions);
        let input = CreateRecipeInput {
            machine_code: self.machine,
            reference: self.reference.clone(),
            name: text("Scenario recipe"),
            change_note: long_text("First version"),
        };
        CreateRecipe::new(self.app.context())
            .execute(&self.engineer, &meta, input)
            .await
            .unwrap()
            .version
    }

    /// The engineer adds the `DINT` parameter `SPEED` with the value 5 to version 1.
    pub async fn add_speed(&mut self, actions: &[&'static str]) {
        let expected_row_version = self.row_version(1).await;
        let meta = self.begin(self.engineer.user(), actions);
        let input = AddParameterInput {
            reference: self.reference.clone(),
            number: 1,
            expected_row_version,
            unit_path: UnitPath::parse("U1").unwrap(),
            tag: ParameterTag::parse("SPEED").unwrap(),
            name_en: text("Speed"),
            description_en: long_text("Speed of the belt"),
            type_expr: TypeExpr::parse("DINT").unwrap(),
            s7_address: None,
            ordinal: Ordinal::parse(1).unwrap(),
            value: ValueInput::Json("5".to_owned()),
            limits: speed_limits(),
        };
        let output = AddParameter::new(self.app.context())
            .execute(&self.engineer, &meta, input)
            .await
            .unwrap();
        self.key = Some(output.key);
    }

    /// The engineer sets the value of `SPEED` in the draft `number`.
    pub async fn set_speed(&mut self, number: u32, value: &str, actions: &[&'static str]) {
        let expected_row_version = self.row_version(number).await;
        let meta = self.begin(self.engineer.user(), actions);
        let input = SetParameterValueInput {
            reference: self.reference.clone(),
            number,
            expected_row_version,
            key: self.key.unwrap(),
            value: ValueInput::Json(value.to_owned()),
        };
        SetParameterValue::new(self.app.context())
            .execute(&self.engineer, &meta, input)
            .await
            .unwrap();
    }

    /// The engineer creates a new draft on the base `based_on`.
    pub async fn new_version(
        &mut self,
        based_on: u32,
        actions: &[&'static str],
    ) -> CreateVersionOutput {
        let meta = self.begin(self.engineer.user(), actions);
        let input = CreateVersionInput {
            reference: self.reference.clone(),
            based_on: Some(based_on),
            change_note: long_text("Next version"),
        };
        CreateVersion::new(self.app.context())
            .execute(&self.engineer, &meta, input)
            .await
            .unwrap()
    }

    /// The administrator approves the draft `number`.
    pub async fn approve(&mut self, number: u32, actions: &[&'static str]) -> ApproveVersionOutput {
        let expected_row_version = self.row_version(number).await;
        let meta = self.begin(self.app.user.user(), actions);
        let input = ApproveVersionInput {
            reference: self.reference.clone(),
            number,
            expected_row_version,
            allow_outdated: false,
        };
        ApproveVersion::new(self.app.context())
            .execute(&self.app.user, &meta, input)
            .await
            .unwrap()
    }

    /// The administrator retires the approved version `number`.
    pub async fn retire(&mut self, number: u32, actions: &[&'static str]) {
        let expected_row_version = self.row_version(number).await;
        let meta = self.begin(self.app.user.user(), actions);
        let input = RetireVersionInput {
            reference: self.reference.clone(),
            number,
            expected_row_version,
        };
        RetireVersion::new(self.app.context())
            .execute(&self.app.user, &meta, input)
            .await
            .unwrap();
    }

    /// The administrator exports the version `number`. Return its content hash as text.
    ///
    /// The export rebuilds the hashed body and compares its hash with the stored hash.
    pub async fn export(&mut self, number: u32, actions: &[&'static str]) -> String {
        let meta = self.begin(self.app.user.user(), actions);
        let input = ExportByNumberInput {
            reference: self.reference.clone(),
            number,
        };
        ExportVersionByNumber::new(self.app.context())
            .execute(&self.app.user, &meta, input)
            .await
            .unwrap()
            .content_sha256
            .to_string()
    }

    /// The row version of the version `number`, read through `GetVersion`.
    async fn row_version(&self, number: u32) -> i64 {
        let input = GetVersionInput {
            reference: self.reference.clone(),
            number,
            rebind_bound: 0,
        };
        GetVersion::new(self.app.context())
            .execute(&self.app.user, &new_meta(), input)
            .await
            .unwrap()
            .detail
            .row_version
    }

    /// The rows of the versions, the newest first, read through `ListVersions`.
    pub async fn versions(&self) -> Vec<VersionSummary> {
        let input = ListVersionsInput {
            reference: self.reference.clone(),
            page: 0,
        };
        ListVersions::new(self.app.context())
            .execute(&self.app.user, &new_meta(), input)
            .await
            .unwrap()
            .rows
    }

    /// The number and the state of each version, the newest first.
    pub async fn states(&self) -> Vec<(u32, VersionState)> {
        self.versions()
            .await
            .iter()
            .map(|row| (row.number, row.state))
            .collect()
    }
}
