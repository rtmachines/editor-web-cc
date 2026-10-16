//! Tests of `GetRecipeHistory` (spec "Recipe history query", ADR-0051 (g)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::{RecipeDeleter, RecipeError};

use super::test_world::{M12, R100, R300, context_with_pages, entry_id, ids, world};
use super::{GetRecipeHistory, RecipeHistory, RecipeHistoryInput};
use crate::blocking::PageSizes;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, draft_recipe, meta, reference, rid, timestamp, user,
};
use crate::ports::{AggregateId, AuditFilter, AuditScope, Isolation, UnitOfWork};

/// Run `GetRecipeHistory` as `role` on the recipe `text` with `before`.
async fn history(
    fixture: &Fixture,
    role: Role,
    text: &str,
    before: Option<u64>,
) -> Result<RecipeHistory, AppError> {
    let input = RecipeHistoryInput {
        reference: reference(text),
        before: before.map(entry_id),
    };
    GetRecipeHistory::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
}

/// Store `R-300` of `M12` as a deleted recipe. The world holds its entry `RecipeDeleted` (id 6).
fn seed_deleted_r300(fixture: &Fixture) {
    let mut recipe = draft_recipe((R300, M12), "R-300", 30_001);
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::TwinCat3, user(2));
}

#[tokio::test]
async fn an_operator_reads_the_entries_of_the_recipe_and_its_parameters_newest_first() {
    let fixture = world().await;
    let found = history(&fixture, Role::Operator, "R-100", None)
        .await
        .unwrap();
    assert_eq!(found.recipe.reference(), &reference("R-100"));
    assert_eq!(ids(&found.page.rows), [8, 5, 4, 3, 2]);
    assert!(!found.page.has_older);
}

#[tokio::test]
async fn the_cursor_gives_the_older_entries_of_the_history() {
    let fixture = world().await;
    let pages = PageSizes {
        history: 2,
        ..PageSizes::default()
    };
    let query = GetRecipeHistory::new(context_with_pages(&fixture, pages));
    let operator = actor(Role::Operator);
    let mut seen = Vec::new();
    for before in [None, Some(5), Some(3)] {
        let input = RecipeHistoryInput {
            reference: reference("R-100"),
            before: before.map(entry_id),
        };
        let page = query.execute(&operator, &meta(), input).await.unwrap().page;
        seen.push((ids(&page.rows), page.has_older));
    }
    assert_eq!(
        seen,
        [(vec![8, 5], true), (vec![4, 3], true), (vec![2], false)]
    );
}

#[tokio::test]
async fn a_last_page_that_fills_the_page_size_tells_that_no_older_entry_exists() {
    let fixture = world().await;
    let pages = PageSizes {
        history: 2,
        ..PageSizes::default()
    };
    let input = RecipeHistoryInput {
        reference: reference("R-100"),
        before: Some(entry_id(4)),
    };
    let found = GetRecipeHistory::new(context_with_pages(&fixture, pages))
        .execute(&actor(Role::Operator), &meta(), input)
        .await
        .unwrap();
    assert_eq!(ids(&found.page.rows), [3, 2]);
    assert!(!found.page.has_older);
}

#[tokio::test]
async fn a_history_row_is_the_row_of_the_audit_list_without_a_state() {
    let fixture = world().await;
    let found = history(&fixture, Role::Administrator, "R-100", None)
        .await
        .unwrap();
    let filter = AuditFilter {
        aggregate: Some(AggregateId::from(*rid(R100).as_uuid())),
        ..AuditFilter::default()
    };
    let uow: &dyn UnitOfWork = &fixture.store;
    let mut tx = uow.begin(Isolation::ReadOnly).await.unwrap();
    let listed = tx
        .audit_reads()
        .entries(&filter, AuditScope::All, None, 10)
        .await
        .unwrap();
    assert_eq!(found.page, listed);
}

#[tokio::test]
async fn a_deleted_recipe_gives_e_rcp_012_to_every_role_that_cannot_restore_it() {
    let fixture = world().await;
    seed_deleted_r300(&fixture);
    for role in [Role::ProcessEngineer, Role::Operator] {
        let error = history(&fixture, role, "R-300", None).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::RCP_012, "{role:?}");
        assert!(
            matches!(&error, AppError::Recipe(RecipeError::RecipeDeleted { .. })),
            "{role:?}"
        );
    }
}

#[tokio::test]
async fn the_administrator_reads_the_history_of_a_deleted_recipe() {
    let fixture = world().await;
    seed_deleted_r300(&fixture);
    let found = history(&fixture, Role::Administrator, "R-300", None)
        .await
        .unwrap();
    assert!(found.recipe.is_deleted());
    assert_eq!(ids(&found.page.rows), [6]);
}

#[tokio::test]
async fn an_unknown_reference_gives_not_found() {
    let fixture = world().await;
    let error = history(&fixture, Role::Operator, "R-999", None)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Recipe(found) } if found == reference("R-999")
    ));
}
