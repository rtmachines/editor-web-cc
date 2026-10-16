//! Tests of `ListRecipes`, `ListDeletedRecipes`, `GetRecipe`, and `GetRecipeForm` (design D4).
//!
//! The specs are "Recipe list", "Recipe page", and "Deleted recipe" (ADR-0043).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform};
use recipe_domain::recipe::{VersionState, VersionState::Approved, VersionState::LiveDraft};

use super::recipes::{
    GetRecipe, GetRecipeForm, GetRecipeInput, ListDeletedRecipes, ListRecipes, RecipeList,
    RecipeListInput, RecipeView,
};
use super::test_world::{seed_deleted, seed_recipe, seed_users};
use crate::auth::Operation;
use crate::blocking::{AppSettings, PageSizes};
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{Fixture, actor, meta, reference, scenarios};
use crate::ports::Isolation;

fn code(value: &str) -> MachineCode {
    MachineCode::parse(value).unwrap()
}

/// The request for page `page` with no filter.
fn plain(page: u32) -> RecipeListInput {
    RecipeListInput {
        text: None,
        machine: None,
        status: None,
        page,
    }
}

/// The settings with `size` rows on a page of the recipe list and of the version list.
fn pages_of(size: u32) -> AppSettings {
    AppSettings {
        pages: PageSizes {
            recipes: size,
            versions: size,
            ..PageSizes::default()
        },
        ..AppSettings::default()
    }
}

async fn list(fixture: &Fixture, role: Role, input: RecipeListInput) -> RecipeList {
    ListRecipes::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
        .unwrap()
}

fn references(list: &RecipeList) -> Vec<String> {
    list.rows
        .iter()
        .map(|row| row.reference.to_string())
        .collect()
}

/// A fixture with the machines M1 and M2 and four recipes.
///
/// `R-1` and `R-10` are on M1 (a live draft, and an approved version with a draft).
/// `R-2` is on M2 (retired and approved). `R-3` is on M1 and has a draft.
fn world() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let first = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let second = fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    seed_recipe(&fixture, &first, (1, "R-1"), &[LiveDraft]);
    seed_recipe(&fixture, &first, (10, "R-10"), &[Approved, LiveDraft]);
    seed_recipe(
        &fixture,
        &second,
        (2, "R-2"),
        &[VersionState::Retired, Approved],
    );
    seed_recipe(&fixture, &first, (3, "R-3"), &[LiveDraft]);
    fixture
}

#[test]
fn the_queries_declare_their_operations() {
    assert_eq!(ListRecipes::OPERATION, Operation::ListRecipes);
    assert_eq!(ListDeletedRecipes::OPERATION, Operation::ListDeletedRecipes);
    assert_eq!(GetRecipe::OPERATION, Operation::GetRecipe);
    assert_eq!(GetRecipeForm::OPERATION, Operation::GetRecipeForm);
}

#[tokio::test]
async fn each_role_gets_the_rows_in_one_read_only_transaction_and_no_forbidden_event() {
    for role in [Role::Administrator, Role::ProcessEngineer, Role::Operator] {
        let fixture = world();

        let result = list(&fixture, role, plain(0)).await;

        assert_eq!(
            references(&result),
            ["R-1", "R-10", "R-2", "R-3"],
            "{role:?}"
        );
        assert!(fixture.sink.events().is_empty(), "{role:?}");
        assert_eq!(
            fixture.store.journal(),
            [JournalEntry::Begin(Isolation::ReadOnly)],
            "{role:?}"
        );
        assert!(fixture.store.audit_entries().is_empty(), "{role:?}");
    }
}

#[tokio::test]
async fn a_row_names_the_machine_and_the_numbers_of_the_approved_version_and_the_draft() {
    let fixture = world();

    let result = list(&fixture, Role::Operator, plain(0)).await;

    let row = result.rows.get(1).unwrap();
    assert_eq!(row.reference.to_string(), "R-10");
    assert_eq!(row.machine_code, code("M1"));
    assert_eq!((row.approved, row.draft), (Some(1), Some(2)));
    assert!(!row.deleted);
    let second = result.rows.get(2).unwrap();
    assert_eq!(second.machine_code, code("M2"));
    assert_eq!((second.approved, second.draft), (Some(2), None));
}

#[tokio::test]
async fn the_filters_reach_the_port() {
    let fixture = world();
    let by_text = RecipeListInput {
        text: Some(crate::memory::fixtures::text("r-1")),
        ..plain(0)
    };
    let by_machine = RecipeListInput {
        machine: Some(code("M2")),
        ..plain(0)
    };
    let by_status = RecipeListInput {
        status: Some(LiveDraft),
        ..plain(0)
    };

    let text = list(&fixture, Role::Operator, by_text).await;
    let machine = list(&fixture, Role::Operator, by_machine).await;
    let status = list(&fixture, Role::Operator, by_status).await;

    assert_eq!(references(&text), ["R-1", "R-10"]);
    assert_eq!(references(&machine), ["R-2"]);
    assert_eq!(references(&status), ["R-1", "R-10", "R-3"]);
}

#[tokio::test]
async fn a_machine_code_without_a_live_machine_gives_not_found_in_both_lists() {
    let fixture = world();
    let input = RecipeListInput {
        machine: Some(code("M99")),
        ..plain(0)
    };

    let live = ListRecipes::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input.clone())
        .await
        .unwrap_err();
    let with_deleted = ListDeletedRecipes::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input)
        .await
        .unwrap_err();

    for error in [live, with_deleted] {
        assert_eq!(error.code(), ErrorCode::WEB_005, "{error:?}");
        assert!(
            matches!(&error, AppError::NotFound { entity } if *entity == Missing::Machine(code("M99"))),
            "{error:?}"
        );
    }
}

#[tokio::test]
async fn the_page_size_cuts_the_list_and_has_next_tells_about_the_next_page() {
    let fixture = world();
    let run = |page| {
        let query = ListRecipes::new(fixture.context_with(pages_of(3)));
        async move {
            query
                .execute(&actor(Role::Operator), &meta(), plain(page))
                .await
                .unwrap()
        }
    };

    let first = run(0).await;
    let second = run(1).await;
    let beyond = run(5).await;

    assert_eq!(references(&first), ["R-1", "R-10", "R-2"]);
    assert_eq!((first.page, first.has_next), (0, true));
    assert_eq!(references(&second), ["R-3"]);
    assert_eq!((second.page, second.has_next), (1, false));
    assert!(beyond.rows.is_empty());
    assert!(!beyond.has_next);
}

#[tokio::test]
async fn list_recipes_hides_the_deleted_recipes_and_list_deleted_recipes_shows_them() {
    let fixture = world();
    seed_deleted(&fixture, 4, "R-DEL");

    let live = list(&fixture, Role::Administrator, plain(0)).await;
    let with_deleted = ListDeletedRecipes::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), plain(0))
        .await
        .unwrap();

    assert_eq!(references(&live), ["R-1", "R-10", "R-2", "R-3"]);
    assert_eq!(
        references(&with_deleted),
        ["R-1", "R-10", "R-2", "R-3", "R-DEL"]
    );
    let flags: Vec<_> = with_deleted.rows.iter().map(|row| row.deleted).collect();
    assert_eq!(flags, [false, false, false, false, true]);
}

#[tokio::test]
async fn list_deleted_recipes_is_refused_for_a_process_engineer() {
    let fixture = world();

    let error = ListDeletedRecipes::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), plain(0))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert_eq!(fixture.sink.events().len(), 1);
}

#[tokio::test]
async fn get_recipe_returns_the_recipe_the_machine_and_the_newest_versions_first() {
    let fixture = world();

    let view: RecipeView = GetRecipe::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            GetRecipeInput {
                reference: reference("R-10"),
                page: 0,
            },
        )
        .await
        .unwrap();

    assert_eq!(view.recipe.reference().to_string(), "R-10");
    assert_eq!(view.machine.code(), code("M1"));
    let numbers: Vec<_> = view.versions.rows.iter().map(|row| row.number).collect();
    assert_eq!(numbers, [2, 1]);
    assert!(!view.versions.has_next);
    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
}

#[tokio::test]
async fn get_recipe_cuts_the_versions_at_the_page_size() {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let states = [VersionState::Retired, VersionState::Retired, Approved];
    seed_recipe(&fixture, &machine, (1, "R-1"), &states);
    let run = |page| {
        let context = fixture.context_with(pages_of(2));
        async move {
            GetRecipe::new(context)
                .execute(
                    &actor(Role::Operator),
                    &meta(),
                    GetRecipeInput {
                        reference: reference("R-1"),
                        page,
                    },
                )
                .await
                .unwrap()
        }
    };

    let first = run(0).await;
    let second = run(1).await;

    let numbers = |view: &RecipeView| {
        view.versions
            .rows
            .iter()
            .map(|r| r.number)
            .collect::<Vec<_>>()
    };
    assert_eq!(numbers(&first), [3, 2]);
    assert!(first.versions.has_next);
    assert_eq!(numbers(&second), [1]);
    assert!(!second.versions.has_next);
}

#[tokio::test]
async fn get_recipe_of_an_unknown_reference_is_not_found() {
    let fixture = world();

    let error = GetRecipe::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            GetRecipeInput {
                reference: reference("R-99"),
                page: 0,
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Recipe(ref found) } if found.to_string() == "R-99"
    ));
}

#[tokio::test]
async fn get_recipe_of_a_deleted_recipe_is_refused_except_for_the_role_that_restores() {
    let fixture = scenarios::deleted();
    seed_users(&fixture);
    let run = |role| {
        let query = GetRecipe::new(fixture.context());
        async move {
            query
                .execute(
                    &actor(role),
                    &meta(),
                    GetRecipeInput {
                        reference: reference("R-1"),
                        page: 0,
                    },
                )
                .await
        }
    };

    let operator = run(Role::Operator).await.unwrap_err();
    let engineer = run(Role::ProcessEngineer).await.unwrap_err();
    let administrator = run(Role::Administrator).await.unwrap();

    assert_eq!(operator.code(), ErrorCode::RCP_012);
    assert_eq!(engineer.code(), ErrorCode::RCP_012);
    assert!(administrator.recipe.is_deleted());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn get_recipe_form_lists_the_live_machines_in_the_order_of_the_code_number() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 10, "M10");
    fixture.seed_machine_on(PlcPlatform::S7_300, 2, "M2");
    let mut archived = crate::memory::fixtures::machine(3, "M3");
    archived
        .archive(crate::memory::fixtures::timestamp(), None)
        .unwrap();
    fixture.store.seed_machine(&archived);

    let form = GetRecipeForm::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            super::recipes::RecipeFormInput {
                purpose: crate::auth::RecipeFormPurpose::CreateRecipe,
            },
        )
        .await
        .unwrap();

    let codes: Vec<_> = form.machines.iter().map(|m| m.code.to_string()).collect();
    assert_eq!(codes, ["M2", "M10"]);
    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
}
