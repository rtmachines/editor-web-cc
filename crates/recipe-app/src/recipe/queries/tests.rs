use recipe_domain::identity::Role;

use super::{ListRecipes, RecipeList};
use crate::memory::fixtures::{Fixture, actor, meta};

#[tokio::test]
async fn each_role_gets_the_recipe_list_and_no_forbidden_event() {
    for role in [Role::Administrator, Role::ProcessEngineer, Role::Operator] {
        let fixture = Fixture::new();
        let list = ListRecipes::new(fixture.context())
            .execute(&actor(role), &meta())
            .await
            .unwrap();
        assert_eq!(list, RecipeList, "{role:?}");
        assert!(fixture.sink.events().is_empty(), "{role:?}");
    }
}

#[tokio::test]
async fn the_query_writes_no_audit_entry() {
    let fixture = Fixture::new();
    ListRecipes::new(fixture.context())
        .execute(&actor(Role::Operator), &meta())
        .await
        .unwrap();
    assert!(fixture.store.audit_entries().is_empty());
}
