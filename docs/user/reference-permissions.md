# Permissions by role

This page lists the roles of the recipe editor and the actions that each role may perform. It states the rules and the refusals only. For the steps of an action, use the [user guide index](README.md).

The UI starts in Polish. To read the English texts of this page, choose **English** in the **Language** switch.

## Roles

Each role maps to a fixed set of permissions (ADR-0013).

| Role | Description |
|---|---|
| **Administrator** (`Administrator`) | Keeps users, machines, and released versions accurate. This role holds every permission. |
| **Process engineer** (`ProcessEngineer`) | Changes values and limits, and approves the drafts of other users. |
| **Operator** (`Operator`) | Finds the approved version of a recipe, reads its values, and exports it. |

## Permissions by action

The table lists each action with its permission name. The permission names are the variants of the `Permission` enum (`api.md` section 3.1). [`api.md`](../architecture/api.md) section 3.3 is the authority for every cell of this table.

`yes` means that the role may take the action. `no` means that the role may not take it. `yes (c)` means that the role may take the action only under the condition that follows the table.

| Action | Permission | Administrator | Process engineer | Operator |
|---|---|---|---|---|
| View recipes, versions, parameters, machines, units, types, and UDTs | `RecipeView` | yes | yes | yes |
| View recipe history | `RecipeView` | yes | yes | yes |
| Export the **Approved** version | `VersionExport` | yes | yes | yes |
| Export a **Retired** version by number | `VersionExportHistory` | yes | yes | no |
| Create a recipe | `RecipeCreate` | yes | yes | no |
| Rename a recipe | `RecipeEdit` | yes | yes | no |
| Delete a recipe | `RecipeDelete` | yes (c) | yes (c) | no |
| Restore a deleted recipe and list deleted recipes | `RecipeRestore` | yes | no | no |
| Create a new draft version | `VersionCreate` | yes | yes | no |
| Add, edit, or delete parameters, limits, and translations of a draft, and rebind a type revision | `ParameterEdit` | yes | yes | no |
| Approve a version | `VersionApprove` | yes (c) | yes (c) | no |
| Retire a version without replacement | `VersionRetire` | yes | no | no |
| Discard a draft | `VersionDiscard` | yes | yes | no |
| Create, rename, and archive a machine | `MachineManage` | yes | yes | no |
| Add, rename, and archive a unit | `UnitManage` | yes | yes | no |
| Define a UDT revision | `UdtManage` | yes | yes | no |
| Create users, set roles, activate, deactivate, and reset passwords | `UserManage` | yes | no | no |
| Unlock a login | `LoginUnlock` | yes | no | no |
| View the full audit log and the security events | `AuditView` | yes | no | no |
| View the audit entries of recipes, versions, and parameters | `AuditViewRecipes` | yes | yes | no |
| Log in | none | yes | yes | yes |
| Log out | authenticated | yes | yes | yes |
| Change own password | authenticated | yes | yes | yes |

Conditions of the `yes (c)` cells:

- **Delete a recipe**, Administrator: only when the recipe has no **Approved** version (`E-RCP-010`).
- **Delete a recipe**, Process engineer: only when the recipe has no **Approved** version and no **Retired** version (`E-RCP-026`).
- **Approve a version**, Administrator and Process engineer: the approver must not have created or edited the draft (`E-RCP-015`).

An approval retires the previous **Approved** version (`api.md` section 3.3).

## Rules that you see

- The server checks the permission of every action. Each form checks the permission of its action before it shows any field (ADR-0046).
- No user holds a permission that differs from the permissions of the role (ADR-0013).
- The user who created or edited a draft cannot approve it. The server refuses with `E-RCP-015`, and a different user must approve the draft (ADR-0022, invariant I16 in [`domain-model.md`](../architecture/domain-model.md)).
- Only an Administrator and a Process engineer can export a **Retired** version by number. For the export format and its integrity rules, see ADR-0019.
- The last active Administrator cannot be deactivated, and the role of that user cannot change. The server refuses with `E-IDN-002` (ADR-0096).

## Refusals

When the role lacks the permission of an action, the server returns HTTP status `403` with the code `E-WEB-002`. The text names the refused action (`api.md` section 5.1):

> Your role does not permit the operation {operation}.

The server records the refusal as a security event (`api.md` section 3.2). If the acting user loses the permission or the active state during a user change, the server also refuses with `E-WEB-002` (ADR-0096).

The table lists the other refusal codes that the rules of this page can show. The codes and their texts are in [`reliability.md`](../architecture/reliability.md) section 1.2.

| Code | HTTP status | English text | When it appears |
|---|---|---|---|
| `E-WEB-002` | `403` | Your role does not permit the operation {operation}. | The role lacks the permission of the action. |
| `E-WEB-004` | `401` | Log in to continue. | There is no session, or the session expired. |
| `E-WEB-006` | `503` | The system is in read-only mode because the free disk space is low or unknown. Contact the administrator. | The server refuses every change of data, except log in, log out, and export. |
| `E-RCP-010` | `409` | Recipe {reference} has an approved version. An Administrator can retire the version. | An Administrator deletes a recipe that has an **Approved** version. |
| `E-RCP-015` | `409` | User {login} created or edited draft {number} of recipe {reference}. A different user must approve it. | The approver created or edited the draft. |
| `E-RCP-026` | `409` | Recipe {reference} has a retired version. Only an administrator can delete it. | A Process engineer deletes a recipe that has a **Retired** version. |
| `E-IDN-002` | `409` | {login} is the last active administrator. Add another administrator first. | A user action deactivates or changes the role of the last active Administrator. |

## Related pages

- [How to approve a version](how-to-approve-a-version.md)
- [Screens](reference-screens.md)
- [Why versions are immutable](explanation-why-versions-are-immutable.md)
