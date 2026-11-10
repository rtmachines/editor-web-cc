use super::*;
use std::collections::BTreeSet;

/// Every role, person or technical. `Role::ALL` holds the person roles only.
const EVERY_ROLE: [Role; 4] = [
    Role::Administrator,
    Role::ProcessEngineer,
    Role::Operator,
    Role::ExportClient,
];

/// Return the place of a role in `EVERY_ROLE`. The `match` has no wildcard arm.
/// A new variant breaks the build here, so the author adds it to the list.
const fn position(role: Role) -> usize {
    match role {
        Role::Administrator => 0,
        Role::ProcessEngineer => 1,
        Role::Operator => 2,
        Role::ExportClient => 3,
    }
}

fn set(items: &[Permission]) -> BTreeSet<Permission> {
    items.iter().copied().collect()
}

#[test]
fn every_role_lists_each_variant_once_in_order() {
    for (index, role) in EVERY_ROLE.iter().enumerate() {
        assert_eq!(position(*role), index, "{role}");
    }
}

#[test]
fn administrator_holds_every_permission() {
    assert_eq!(Role::Administrator.permissions().len(), 19);
    assert_eq!(
        set(Role::Administrator.permissions()),
        set(&Permission::ALL)
    );
}

#[test]
fn process_engineer_holds_the_fourteen_permissions_of_the_matrix() {
    use Permission::*;
    let expected = set(&[
        RecipeView,
        VersionExport,
        VersionExportHistory,
        RecipeCreate,
        RecipeEdit,
        RecipeDelete,
        VersionCreate,
        ParameterEdit,
        VersionApprove,
        VersionDiscard,
        MachineManage,
        UnitManage,
        UdtManage,
        AuditViewRecipes,
    ]);
    assert_eq!(expected.len(), 14);
    assert_eq!(Role::ProcessEngineer.permissions().len(), 14);
    assert_eq!(set(Role::ProcessEngineer.permissions()), expected);
}

#[test]
fn operator_holds_view_and_export_only() {
    assert_eq!(
        set(Role::Operator.permissions()),
        set(&[Permission::RecipeView, Permission::VersionExport])
    );
    assert_eq!(Role::Operator.permissions().len(), 2);
}

#[test]
fn has_agrees_with_permissions_for_every_pair() {
    for role in EVERY_ROLE {
        for permission in Permission::ALL {
            assert_eq!(
                role.has(permission),
                role.permissions().contains(&permission),
                "{role} / {permission}"
            );
        }
    }
}

#[test]
fn permission_all_lists_nineteen_distinct_variants() {
    assert_eq!(Permission::ALL.len(), 19);
    assert_eq!(set(&Permission::ALL).len(), 19);
}

#[test]
fn permission_displays_its_variant_name() {
    assert_eq!(
        Permission::VersionExportHistory.to_string(),
        "VersionExportHistory"
    );
    assert_eq!(Permission::AuditViewRecipes.to_string(), "AuditViewRecipes");
}

#[test]
fn role_displays_its_variant_name() {
    assert_eq!(Role::ProcessEngineer.to_string(), "ProcessEngineer");
}

#[test]
fn process_engineer_text_parses() {
    assert_eq!(Role::parse("process_engineer"), Ok(Role::ProcessEngineer));
}

#[test]
fn variant_name_admin_and_empty_text_give_a_reason() {
    assert_eq!(Role::parse("ProcessEngineer"), Err(RoleReason::Unknown));
    assert_eq!(Role::parse("admin"), Err(RoleReason::Unknown));
    assert_eq!(Role::parse(""), Err(RoleReason::Unknown));
}

#[test]
fn as_sql_and_parse_round_trip() {
    for role in EVERY_ROLE {
        assert_eq!(Role::parse(role.as_sql()), Ok(role));
    }
    assert_eq!(Role::Administrator.as_sql(), "administrator");
    assert_eq!(Role::ProcessEngineer.as_sql(), "process_engineer");
    assert_eq!(Role::Operator.as_sql(), "operator");
}

#[test]
fn export_client_holds_version_export_only() {
    assert_eq!(
        set(Role::ExportClient.permissions()),
        set(&[Permission::VersionExport])
    );
    assert_eq!(Role::ExportClient.permissions().len(), 1);
}

#[test]
fn export_client_text_parses_and_round_trips() {
    assert_eq!(Role::parse("export_client"), Ok(Role::ExportClient));
    assert_eq!(Role::ExportClient.as_sql(), "export_client");
    assert_eq!(Role::ExportClient.to_string(), "ExportClient");
}

#[test]
fn only_export_client_is_not_a_person() {
    assert!(!Role::ExportClient.is_person());
    assert!(Role::Administrator.is_person());
    assert!(Role::ProcessEngineer.is_person());
    assert!(Role::Operator.is_person());
}

#[test]
fn role_reason_displays_a_sentence() {
    assert!(!RoleReason::Unknown.to_string().is_empty());
}

#[test]
fn api_3_1_permission_names_equal_the_enum_of_the_document_in_order() {
    // Oracle: the enum of `api.md` section 3.1, typed by hand.
    let documented = [
        "RecipeView",
        "RecipeCreate",
        "RecipeEdit",
        "RecipeDelete",
        "RecipeRestore",
        "VersionCreate",
        "VersionApprove",
        "VersionRetire",
        "VersionDiscard",
        "VersionExport",
        "VersionExportHistory",
        "ParameterEdit",
        "MachineManage",
        "UnitManage",
        "UdtManage",
        "UserManage",
        "LoginUnlock",
        "AuditView",
        "AuditViewRecipes",
    ];
    let names: Vec<String> = Permission::ALL.iter().map(ToString::to_string).collect();
    assert_eq!(names, documented);
}

#[test]
fn api_3_1_permission_names_are_distinct() {
    let names: BTreeSet<String> = Permission::ALL.iter().map(ToString::to_string).collect();
    assert_eq!(names.len(), Permission::ALL.len());
}
