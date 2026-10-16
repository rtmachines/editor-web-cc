//! The approval data of design D13: `Fixture::seed_approval_recipes`.
//!
//! The function solves one problem: the tests of the approval, diff, history, and audit pages need
//! recipes that went through the real use cases, so the audit log, the version states, and the
//! authors are the ones a user would cause. The data comes on top of `seed_recipes`:
//!
//! - The Administrator `admin`, the ProcessEngineer `anowak`, and the second ProcessEngineer
//!   `jsmith`, who edits nothing. Each has a session.
//! - `R-500` on `M7`. `anowak` creates it. `admin` approves version 1. Draft version 2 is based on
//!   version 1, and `anowak` edits it. It has a changed limit `[*]` of `SPEED_TABLE`, a changed
//!   value `.head.a` of `LINE_A` (a struct in a struct), the added parameter `ADDED`, and the
//!   parameter `OLD_ONLY`, which only version 1 holds.
//! - `R-600` on `M7` with a retired version 1 and no approved version.
//! - `R-700`, a deleted recipe on the archived machine `M12`. A use case cannot create a recipe on
//!   an archived machine, so the function seeds this recipe, as `seed_recipes` seeds `R-300`.
//!
//! The function is async, because the use cases are. Call it once for each fixture.

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::context::AppContext;
use recipe_app::ports::{Isolation, NewUser, Transaction as _};
use recipe_app::recipe::{
    AddParameter, AddParameterInput, ApproveVersion, ApproveVersionInput, CreateRecipe,
    CreateRecipeInput, CreateVersion, CreateVersionInput, RemoveParameter, RemoveParameterInput,
    ReplaceLimits, ReplaceLimitsInput, RetireVersion, RetireVersionInput, SetParameterValue,
    SetParameterValueInput,
};
use recipe_domain::audit::RequestId;
use recipe_domain::identity::{Login, Role, UserId};
use recipe_domain::plant::{LeafPath, MachineCode, PlcPlatform, S7Address, TypeExpr, UnitPath};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredRecipe, StoredVersionHeader, ValueInput, VersionState,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{RecipeIds, text};
use crate::test_support::{Fixture, TestSession};

/// The number of the user `admin`. The numbers of the sessions are the same.
const ADMIN_NUMBER: u8 = 41;
/// The number of the user `anowak`.
const ANOWAK_NUMBER: u8 = 42;
/// The number of the user `jsmith`.
const JSMITH_NUMBER: u8 = 43;
/// The first field of the request identifier of a seed command. The last field is the step.
const REQUEST_BASE: u128 = 0x7357_0100_0000_0000_0000_0000_0000_0000;
/// The identifier of the recipe `R-700`.
const R700_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0700);
/// The identifier of the retired version 1 of `R-700`.
const R700_VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0701);
/// The instant of the deletion of `R-700`, in Unix seconds.
const R700_DELETED_AT_SECONDS: i64 = 700_000;
/// The upper bound of the limits of `SPEED` and `SPEED_TABLE`.
const SPEED_MAX: &str = "3000";

/// What `Fixture::seed_approval_recipes` returns.
///
/// The type solves one problem: a test needs the sessions and the identifiers of the seeded rows,
/// and the row version that a form of the draft must send.
pub(crate) struct ApprovalData {
    /// The identifiers of `seed_recipes`.
    pub(crate) recipes: RecipeIds,
    /// The session of the Administrator `admin`.
    pub(crate) admin: TestSession,
    /// The session of the ProcessEngineer `anowak`, who created and edited `R-500`.
    pub(crate) anowak: TestSession,
    /// The session of the ProcessEngineer `jsmith`, who edited nothing.
    pub(crate) jsmith: TestSession,
    /// The identifier of `admin`.
    pub(crate) admin_id: UserId,
    /// The identifier of `anowak`.
    pub(crate) anowak_id: UserId,
    /// The identifier of `jsmith`.
    pub(crate) jsmith_id: UserId,
    /// The recipe `R-500`.
    pub(crate) r500: RecipeId,
    /// The approved version 1 of `R-500`.
    pub(crate) r500_v1: RecipeVersionId,
    /// The draft version 2 of `R-500`.
    pub(crate) r500_v2: RecipeVersionId,
    /// The row version of the draft version 2 after the last edit.
    pub(crate) r500_v2_row_version: i64,
    /// The recipe `R-600`.
    pub(crate) r600: RecipeId,
    /// The retired version 1 of `R-600`.
    pub(crate) r600_v1: RecipeVersionId,
    /// The deleted recipe `R-700` of the archived machine `M12`.
    pub(crate) r700: RecipeId,
}

impl ApprovalData {
    /// Return `anowak` as the user of a use case.
    pub(crate) fn anowak_user(&self) -> CurrentUser {
        person(self.anowak_id, "anowak", Role::ProcessEngineer)
    }

    /// Return `admin` as the user of a use case.
    pub(crate) fn admin_user(&self) -> CurrentUser {
        person(self.admin_id, "admin", Role::Administrator)
    }

    /// Return `jsmith` as the user of a use case.
    pub(crate) fn jsmith_user(&self) -> CurrentUser {
        person(self.jsmith_id, "jsmith", Role::ProcessEngineer)
    }
}

/// Build the user of a use case.
fn person(id: UserId, login: &str, role: Role) -> CurrentUser {
    CurrentUser::new(id, Login::parse(login).unwrap(), role)
}

/// Return the identifier of the user with the number `number`.
fn user_id(number: u8) -> UserId {
    UserId::from(Uuid::from_u128(u128::from(number)))
}

/// The fields of one parameter of the seed: the fields that differ from parameter to parameter.
struct Spec {
    tag: &'static str,
    ordinal: u32,
    type_text: &'static str,
    address: &'static str,
    value: &'static str,
    /// The limit rows: leaf path, minimum, and maximum.
    limits: Vec<(&'static str, &'static str, &'static str)>,
}

/// Build the limit rows of a request.
fn limit_list(rows: &[(&str, &str, &str)]) -> LimitList {
    let rows = rows
        .iter()
        .map(|(path, min, max)| LimitInput {
            leaf_path: LeafPath::parse(path).unwrap(),
            min: Some((*min).to_owned()),
            max: Some((*max).to_owned()),
            max_length: None,
        })
        .collect();
    LimitList::parse(rows).unwrap()
}

/// The parameters of version 1 of `R-500`. `LINE_A` is a struct in a struct (`ST_Cell@1`).
fn r500_specs() -> Vec<Spec> {
    vec![
        Spec {
            tag: "SPEED",
            ordinal: 0,
            type_text: "INT",
            address: "DB10.DBW0",
            value: "1200",
            limits: vec![("", "0", SPEED_MAX)],
        },
        Spec {
            tag: "OLD_ONLY",
            ordinal: 1,
            type_text: "INT",
            address: "DB10.DBW2",
            value: "5",
            limits: vec![("", "0", "10")],
        },
        Spec {
            tag: "SPEED_TABLE",
            ordinal: 2,
            type_text: "ARRAY[1..8] OF INT",
            address: "DB10.DBB10",
            value: "[100,200,300,400,500,600,700,800]",
            limits: vec![("[*]", "0", SPEED_MAX)],
        },
        Spec {
            tag: "LINE_A",
            ordinal: 3,
            type_text: "ST_Cell@1",
            address: "DB11.DBB0",
            value: r#"{"head":{"a":1,"b":2.5},"count":3}"#,
            limits: vec![
                (".head.a", "0", "100"),
                (".head.b", "0", "10"),
                (".count", "0", "10"),
            ],
        },
    ]
}

/// The parameter that only version 2 of `R-500` holds.
fn added_spec() -> Spec {
    Spec {
        tag: "ADDED",
        ordinal: 4,
        type_text: "INT",
        address: "DB10.DBW30",
        value: "7",
        limits: vec![("", "0", "10")],
    }
}

/// The parameter `ZONE` of the older struct revision `ST_Zone@2`. `ST_Zone@3` supersedes it, so
/// the parameter is outdated (A29). Each leaf has a limit.
fn zone_spec() -> Spec {
    Spec {
        tag: "ZONE",
        ordinal: 5,
        type_text: "ST_Zone@2",
        address: "DB12.DBB0",
        value: r#"{"speed":1,"temp_max":2.5}"#,
        limits: vec![(".speed", "0", SPEED_MAX), (".temp_max", "0", "100")],
    }
}

/// The first request step of the edits after the seed. The steps of the seed stay below it.
const EDIT_STEP: u128 = 1_000;

/// The runner of the seed commands: the context, the fixture, and a counter of the requests.
struct Runner<'a> {
    fixture: &'a Fixture,
    ctx: AppContext,
    step: u128,
}

impl<'a> Runner<'a> {
    fn new(fixture: &'a Fixture) -> Self {
        Self {
            fixture,
            ctx: fixture.state.identity().app().clone(),
            step: 0,
        }
    }

    /// Return the data of the next request. Each command gets its own request identifier.
    fn meta(&mut self) -> RequestMeta {
        self.step = self.step.checked_add(1).unwrap();
        RequestMeta::new(
            RequestId::from(Uuid::from_u128(REQUEST_BASE | self.step)),
            None,
            None,
            None,
        )
    }

    /// Return the row version of the version `number` of `reference` and its identifier.
    async fn header(&self, reference: &str, number: u32) -> (RecipeVersionId, i64) {
        let reference = ReferenceNumber::parse(reference).unwrap();
        let recipe = self
            .fixture
            .store
            .begin_memory(Isolation::ReadOnly)
            .unwrap()
            .recipes()
            .recipe_by_reference(&reference)
            .await
            .unwrap()
            .unwrap();
        let header = recipe
            .versions()
            .iter()
            .find(|header| header.number() == number)
            .unwrap();
        (header.id(), header.row_version())
    }
}

/// The draft that a run of edits changes: the reference, the number, and the row version.
struct Draft {
    reference: ReferenceNumber,
    number: u32,
    row_version: i64,
}

impl Runner<'_> {
    /// Create the recipe `reference` on `M7` for `user`. Return its identifiers.
    async fn create_recipe(
        &mut self,
        user: &CurrentUser,
        reference: &str,
        name: &str,
    ) -> (RecipeId, RecipeVersionId) {
        let meta = self.meta();
        let input = CreateRecipeInput {
            machine_code: MachineCode::parse("M7").unwrap(),
            reference: ReferenceNumber::parse(reference).unwrap(),
            name: text(name),
            change_note: LongText::parse("First version").unwrap(),
        };
        let out = CreateRecipe::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
        (out.recipe, out.version)
    }

    /// Add the parameter `spec` to `draft`. Return its key.
    async fn add(&mut self, user: &CurrentUser, draft: &mut Draft, spec: &Spec) -> ParameterKey {
        let meta = self.meta();
        let input = AddParameterInput {
            reference: draft.reference.clone(),
            number: draft.number,
            expected_row_version: draft.row_version,
            unit_path: UnitPath::parse("U1").unwrap(),
            tag: ParameterTag::parse(spec.tag).unwrap(),
            name_en: text(spec.tag),
            description_en: LongText::parse("A parameter of the approval data").unwrap(),
            type_expr: TypeExpr::parse(spec.type_text).unwrap(),
            s7_address: Some(S7Address::parse(spec.address).unwrap()),
            ordinal: Ordinal::parse(spec.ordinal).unwrap(),
            value: ValueInput::Json(spec.value.to_owned()),
            limits: limit_list(&spec.limits),
        };
        let out = AddParameter::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
        draft.row_version = out.draft.row_version;
        out.key
    }

    /// Replace the limits of the parameter `key` of `draft`.
    async fn replace_limits(
        &mut self,
        user: &CurrentUser,
        draft: &mut Draft,
        key: ParameterKey,
        rows: &[(&str, &str, &str)],
    ) {
        let meta = self.meta();
        let input = ReplaceLimitsInput {
            reference: draft.reference.clone(),
            number: draft.number,
            expected_row_version: draft.row_version,
            key,
            limits: limit_list(rows),
        };
        let out = ReplaceLimits::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
        draft.row_version = out.row_version;
    }

    /// Set the whole value of the parameter `key` of `draft`.
    async fn set_value(
        &mut self,
        user: &CurrentUser,
        draft: &mut Draft,
        key: ParameterKey,
        json: &str,
    ) {
        let meta = self.meta();
        let input = SetParameterValueInput {
            reference: draft.reference.clone(),
            number: draft.number,
            expected_row_version: draft.row_version,
            key,
            value: ValueInput::Json(json.to_owned()),
        };
        let out = SetParameterValue::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
        draft.row_version = out.row_version;
    }

    /// Remove the parameter `key` from `draft`.
    async fn remove(&mut self, user: &CurrentUser, draft: &mut Draft, key: ParameterKey) {
        let meta = self.meta();
        let input = RemoveParameterInput {
            reference: draft.reference.clone(),
            number: draft.number,
            expected_row_version: draft.row_version,
            key,
        };
        let out = RemoveParameter::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
        draft.row_version = out.row_version;
    }

    /// Approve the version `number` of `reference` with its current row version.
    async fn approve(&mut self, user: &CurrentUser, reference: &str, number: u32) {
        let (_id, row_version) = self.header(reference, number).await;
        let meta = self.meta();
        let input = ApproveVersionInput {
            reference: ReferenceNumber::parse(reference).unwrap(),
            number,
            expected_row_version: row_version,
            allow_outdated: false,
        };
        let _outcome = ApproveVersion::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
    }

    /// Retire the approved version `number` of `reference` with its current row version.
    async fn retire(&mut self, user: &CurrentUser, reference: &str, number: u32) {
        let (_id, row_version) = self.header(reference, number).await;
        let meta = self.meta();
        let input = RetireVersionInput {
            reference: ReferenceNumber::parse(reference).unwrap(),
            number,
            expected_row_version: row_version,
        };
        RetireVersion::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
    }

    /// Start a draft of `R-500` based on version 1 for `user`. Return it.
    async fn start_version_2(&mut self, user: &CurrentUser) -> Draft {
        let meta = self.meta();
        let input = CreateVersionInput {
            reference: ReferenceNumber::parse("R-500").unwrap(),
            based_on: Some(1),
            change_note: LongText::parse("Second version").unwrap(),
        };
        let out = CreateVersion::new(self.ctx.clone())
            .execute(user, &meta, input)
            .await
            .unwrap();
        let (_id, row_version) = self.header("R-500", out.number).await;
        Draft {
            reference: ReferenceNumber::parse("R-500").unwrap(),
            number: out.number,
            row_version,
        }
    }
}

/// Seed the deleted recipe `R-700` on the archived machine `M12`.
fn seed_r700(fixture: &Fixture, ids: &RecipeIds, creator: UserId) -> RecipeId {
    let deleted_at =
        Timestamp::from(OffsetDateTime::from_unix_timestamp(R700_DELETED_AT_SECONDS).unwrap());
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(R700_ID),
        machine: ids.plant.m12,
        reference: ReferenceNumber::parse("R-700").unwrap(),
        name: Text::parse("Archived line recipe").unwrap(),
        deleted_at: Some(deleted_at),
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(R700_VERSION_ID),
            number: 1,
            state: VersionState::Retired,
            row_version: 1,
        }],
    })
    .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::TwinCat3, creator);
    recipe.id()
}

impl Fixture {
    /// Seed the users, the sessions, and the recipes of design D13. Return what the tests need.
    ///
    /// The function calls `seed_recipes` first. It runs the real use cases, so the audit log holds
    /// the entries of every command. Call it once for each fixture.
    pub(crate) async fn seed_approval_recipes(&self) -> ApprovalData {
        let recipes = self.seed_recipes();
        let numbers = [
            (ADMIN_NUMBER, "admin", Role::Administrator),
            (ANOWAK_NUMBER, "anowak", Role::ProcessEngineer),
            (JSMITH_NUMBER, "jsmith", Role::ProcessEngineer),
        ];
        for (number, login, role) in numbers {
            self.store.seed_user(&NewUser {
                id: user_id(number),
                login: Login::parse(login).unwrap(),
                password_hash: recipe_domain::identity::PasswordHash::parse(
                    crate::test_support::HASH,
                )
                .unwrap(),
                role,
                must_change_password: false,
            });
        }
        let (admin_id, anowak_id, jsmith_id) = (
            user_id(ADMIN_NUMBER),
            user_id(ANOWAK_NUMBER),
            user_id(JSMITH_NUMBER),
        );
        let ids = Seeded::run(self, admin_id, anowak_id).await;
        ApprovalData {
            recipes,
            admin: self.session_of(ADMIN_NUMBER),
            anowak: self.session_of(ANOWAK_NUMBER),
            jsmith: self.session_of(JSMITH_NUMBER),
            admin_id,
            anowak_id,
            jsmith_id,
            r500: ids.r500,
            r500_v1: ids.r500_v1,
            r500_v2: ids.r500_v2,
            r500_v2_row_version: ids.r500_v2_row_version,
            r600: ids.r600,
            r600_v1: ids.r600_v1,
            r700: seed_r700(self, &recipes, anowak_id),
        }
    }

    /// Add the outdated parameter `ZONE` to the draft version 2 of `R-500` as `user`.
    ///
    /// `row_version` is the current row version of the draft. Return the new row version. The
    /// user becomes an editor of the draft. Call it once for each fixture.
    pub(crate) async fn add_zone_to_r500(&self, user: &CurrentUser, row_version: i64) -> i64 {
        let mut runner = Runner {
            fixture: self,
            ctx: self.state.identity().app().clone(),
            step: EDIT_STEP,
        };
        let mut draft = Draft {
            reference: ReferenceNumber::parse("R-500").unwrap(),
            number: 2,
            row_version,
        };
        let _key = runner.add(user, &mut draft, &zone_spec()).await;
        draft.row_version
    }
}

/// The identifiers that the commands of the seed create.
struct Seeded {
    r500: RecipeId,
    r500_v1: RecipeVersionId,
    r500_v2: RecipeVersionId,
    r500_v2_row_version: i64,
    r600: RecipeId,
    r600_v1: RecipeVersionId,
}

impl Seeded {
    /// Run the commands of `R-500` and `R-600` as `anowak` and `admin`.
    async fn run(fixture: &Fixture, admin_id: UserId, anowak_id: UserId) -> Self {
        let admin = person(admin_id, "admin", Role::Administrator);
        let anowak = person(anowak_id, "anowak", Role::ProcessEngineer);
        let mut runner = Runner::new(fixture);
        let (r500, r500_v1) = runner.create_recipe(&anowak, "R-500", "Press recipe").await;
        let mut draft = Draft {
            reference: ReferenceNumber::parse("R-500").unwrap(),
            number: 1,
            row_version: runner.header("R-500", 1).await.1,
        };
        let mut keys = Vec::new();
        for spec in r500_specs() {
            keys.push((spec.tag, runner.add(&anowak, &mut draft, &spec).await));
        }
        runner.approve(&admin, "R-500", 1).await;
        let (r500_v2, r500_v2_row_version) =
            Self::edit_version_2(&mut runner, &anowak, &keys).await;
        let (r600, r600_v1) = Self::retired_r600(&mut runner, &anowak, &admin).await;
        Self {
            r500,
            r500_v1,
            r500_v2,
            r500_v2_row_version,
            r600,
            r600_v1,
        }
    }

    /// Start and edit the draft version 2 of `R-500`. Return its identifier and row version.
    async fn edit_version_2(
        runner: &mut Runner<'_>,
        anowak: &CurrentUser,
        keys: &[(&'static str, ParameterKey)],
    ) -> (RecipeVersionId, i64) {
        let key_of = |tag: &str| keys.iter().find(|(known, _)| *known == tag).unwrap().1;
        let mut draft = runner.start_version_2(anowak).await;
        let limits = [("[*]", "10", SPEED_MAX)];
        runner
            .replace_limits(anowak, &mut draft, key_of("SPEED_TABLE"), &limits)
            .await;
        let moved = r#"{"head":{"a":2,"b":2.5},"count":3}"#;
        runner
            .set_value(anowak, &mut draft, key_of("LINE_A"), moved)
            .await;
        runner.remove(anowak, &mut draft, key_of("OLD_ONLY")).await;
        runner.add(anowak, &mut draft, &added_spec()).await;
        let (id, row_version) = runner.header("R-500", draft.number).await;
        (id, row_version)
    }

    /// Create `R-600` with one parameter, approve version 1, and retire it.
    async fn retired_r600(
        runner: &mut Runner<'_>,
        anowak: &CurrentUser,
        admin: &CurrentUser,
    ) -> (RecipeId, RecipeVersionId) {
        let (recipe, version) = runner.create_recipe(anowak, "R-600", "Spare recipe").await;
        let mut draft = Draft {
            reference: ReferenceNumber::parse("R-600").unwrap(),
            number: 1,
            row_version: runner.header("R-600", 1).await.1,
        };
        let spec = r500_specs().remove(0);
        runner.add(anowak, &mut draft, &spec).await;
        runner.approve(admin, "R-600", 1).await;
        runner.retire(admin, "R-600", 1).await;
        (recipe, version)
    }
}
