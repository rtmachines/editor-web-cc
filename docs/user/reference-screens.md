# Screens reference

This page lists the full pages of the recipe editor. It gives one row for each page. A row names the route, what the page shows, the main actions, and the permission that opens the page. Use it to find a page or an action.

## How to read the tables

- This page shows the English texts. The editor starts in Polish. Switch the language in the top bar (see [Common page parts](#common-page-parts)).
- A label in bold matches the English text of the page. The page fills in a value in braces, such as `{ $number }`, for example the version number.
- A route starts with `GET` unless the cell says otherwise.
- A button or link appears only for a role with the permission of its command (`api.md` section 3.2). The full matrix of roles and permissions is in [reference-permissions.md](reference-permissions.md).
- The permission column names the permission that opens the page (`api.md` section 3.1). `none` means that the page needs no session. `authenticated` means any signed-in user.
- Fragments are not listed. A fragment replaces a part of a page.
- The export routes and the health routes have no page. See [Export](#export).

## Common page parts

Every full page uses one layout (`design-system.md` section 10.1). From top to bottom, the layout has these parts.

- **Skip to the content**: a link. It is the first focusable element of the page.
- Top bar: the application name **Recipe editor**, the navigation, the language switch, the signed-in login, and the **Log out** button. The login page shows only the application name and the language switch.
- Navigation with the label **Main**. It shows the links that your role may open: **Recipes** and **Machines** (`RecipeView`), **Users** (`UserManage`), **Audit** (`AuditView` or `AuditViewRecipes`), and **My account** (any signed-in user). The navigation marks the link of the current page. A session that must change its password sees no navigation links.
- Language switch: the buttons **Polski** and **English**, with the accessible name `Language`. Select **English** to show the English texts. The choice stays in the cookie `recipe_lang` (`api.md` section 8). The switch works without a session and in read-only mode (`api.md` section 2.1).
- Signed-in user: the text **Signed in as** and the login. The button **Log out** ends the session and opens the login page (`api.md` section 2.1).
- Read-only banner: a warning under the top bar. It shows when the free disk space is low or unknown. The editor blocks every change. Commands answer with `E-WEB-006`, except log in, log out, and export (`api.md` section 5.1).
- Error panel: the area above the page heading. An error shows its code `E-<CTX>-<nnn>`, one sentence, and the request id (`api.md` section 5.2).
- Toast: a green message under the error panel after a successful action. An example is "Unit { $path } is added."
- Page heading: one `h1` with the title of the page. The browser tab shows the page title, then **Recipe editor**.
- Error page: when a full page fails, the page shows the heading **The request failed** and the link **Open the recipe list**. It has no navigation and no language switch. A form that the server refuses without HTMX shows the same heading with the list of bad fields (`api.md` section 5.3).

## Status badges

The editor shows three version states. A badge shows a color, an icon, and a word together. Color alone never shows a state (`design-system.md` section 9).

| Version state | Word | Icon | Color |
|---|---|---|---|
| `Draft` | **Draft** | Pencil | Amber |
| `Approved` | **Approved** | Check mark in a circle | Green |
| `Retired` | **Retired** | Archive box | Grey |

The grey color and the archive box also show these states: **Deleted** (a recipe), **Archived** (a machine or a unit), and **Discarded** (a draft).

## Auth and language (`api.md` section 2.1)

`POST /lang` and `POST /logout` answer with a redirect. They have no page.

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| Log in | `GET /login` | Fields **Login** and **Password**. The top bar shows the language switch only. | **Log in** | none |

## Machines and units (`api.md` section 2.2)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| Machines | `GET /machines` | The machines of the line. Filters: **Code or name contains** and **Show the archived machines**. Columns: **Code**, **Name**, **PLC platform**, **State**. Each row opens its machine. | **Search**, **New machine** | `RecipeView` |
| New machine | `GET /machines/new` | Fields **Machine code**, **Machine name**, **PLC platform**, and **Name of the main unit**. | **Create machine** | `MachineManage` |
| Machine { $code } | `GET /machines/{code}`. An archived machine uses `GET /machines/by-id/{uuid}`. | The code, **Machine name**, **PLC platform**, and the unit tree under the heading **Unit tree**. An archived machine shows **Archived on** and a read-only notice. | **Rename machine**, **Add a main unit**, **Open the UDT types**, and **Archive machine { $code }** in the disclosure **Archive the machine** | `RecipeView` |
| Unit tree of machine { $code } | `GET /machines/{code}/units` | The unit tree of the machine. Each node opens its unit. | **Add a main unit**, **Add a sub-unit** | `RecipeView` |
| New unit of machine { $code } | `GET /machines/{code}/units/new` | The list **Parent unit**, the segment field, and the field **Unit name**. | **Add unit** | `UnitManage` |
| Unit { $path } | `GET /machines/{code}/units/{path}` | The unit path, the parent unit, the list **Sub-units**, and the field **Unit name**. An archived unit shows the state **Archived**. | **Rename unit**, and **Archive unit { $path }** in the disclosure **Archive the unit** | `RecipeView` |

## Types (`api.md` section 2.3)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| UDT types of machine { $code } | `GET /machines/{code}/types` | One row for each revision. Columns: **Name**, **Revision**, **Members**, **State**. The latest revision of each name shows **latest**. | **New UDT type** | `RecipeView` |
| New UDT type (or New revision of { $name }) | `GET /machines/{code}/types/new`. With `from`, the form of a new revision. | The field **Type name** and the member table. Each row holds a member name and a type expression. | **Create type** (new type), **Create revision** (new revision), **Add a member** | `UdtManage` |
| Type { $name }@{ $revision } | `GET /machines/{code}/types/{name}`. An older revision uses `?rev=`. | The type tree of the revision and the list **Revisions**. | **New revision**, **Show the use of this revision** | `RecipeView` |
| Use of { $name }@{ $revision } | `GET /machines/{code}/types/{name}/usage` | The recipes and parameters that use the revision, directly or through another type. Columns: **Recipe**, **Version**, **State**, **Tag**, **Type**. | A link back to the type page | `RecipeView` |

## Recipes (`api.md` section 2.4)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| Recipes | `GET /recipes` | Filters: **Search**, **Machine**, **State**, and **Show deleted recipes**. Columns: **Reference number**, **Name**, **Machine**, **Draft**, **Approved version**. Each row opens its recipe. | **Show**, **New recipe**, and **Restore { $reference }** in each deleted row (`RecipeRestore`) | `RecipeView` |
| New recipe | `GET /recipes/new` | Fields **Machine**, **Reference number**, **Name**, and **Change note**. | **Create recipe** | `RecipeCreate` |
| Recipe { $reference } | `GET /recipes/{ref}` | The machine, **Name**, the versions with their states, and the link **History**. A deleted recipe shows **Deleted** and no form. | **New version**, **History**, **Rename recipe**, and **Delete recipe { $reference }** in the disclosure **Delete this recipe** | `RecipeView` |

## Versions (`api.md` section 2.5)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| New version of recipe { $reference } | `GET /recipes/{ref}/versions/new` | Fields **Base version** and **Change note**. The new draft copies the parameters of the base version. | **Create version** | `VersionCreate` |
| Version { $number } of recipe { $reference } | `GET /recipes/{ref}/versions/{n}` | The fields **State**, **Change note**, **Author**, **Created at**, **Approver**, **Approved at**, **Last change**, and **PLC platform**. The table **Parameters**, grouped by unit, with 100 rows per page. The page also holds the links and forms of the list below this table. | See the list below this table. | `RecipeView` |
| Approval of version { $number } of recipe { $reference } | `GET /recipes/{ref}/versions/{n}/approve` | The approval notice, the diff with the approved version, and the list **Outdated parameters**. A blocker replaces the form. | **Approve version { $number }**, **Back to version { $number }**, and the checkbox **Approve with the outdated parameters** | `VersionApprove` |
| Changes of version { $new } of recipe { $reference } against version { $old } | `GET /recipes/{ref}/versions/{n}/diff/{m}` | The changes between two versions, matched by parameter key. The changes group by kind: definition, value, limits, and translations. | A link back to the version page | `RecipeView` |

A version page shows these items from top to bottom. Each item appears only for the state and the role that its command allows.

- Under the fields: **Export version { $number } as JSON** for an approved or a retired version. A retired version shows a notice before the link.
- In the next row of links, which has no heading: **Approve version { $number }** for a draft.
- In the same row: **Compare with the approved version { $number }**, **Compare with the base version { $number }**, and **History**. **History** shows for every version.
- Above the table **Parameters**: the table **Outdated parameters** and the rebind forms of a draft. Each form has the heading **Rebind { $from } to { $to }**. It also has the button **Rebind parameters**. When the forms exceed the limit `form.rebind_leaves`, a size notice replaces the inputs and the button ([ADR-0044](../adr/0044-bound-of-the-rebind-forms.md)).
- Under the table **Parameters**: the link **Add a parameter** for a draft.
- The section **Actions** stands at the end of the page. It holds one of the two items below.
- **Retire version { $number }** in the disclosure **Retire this version** for an approved version.
- **Discard draft { $number }** in the disclosure **Discard this draft** for a draft.

## Parameters (`api.md` section 2.6)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| New parameter | `GET /recipes/{ref}/versions/{n}/parameters/new` | The form of a new parameter. See the list below this table. | **Add parameter**, **Show value fields** | `ParameterEdit` |
| Parameter { $tag } | `GET /recipes/{ref}/versions/{n}/parameters/{key}` | The parameter summary, the value grid, and the translations. On a draft with `ParameterEdit`, the edit form, the translation forms, and the removal. The edit form has the same fields as the form of a new parameter. | **Save parameter**, **Show value fields**, **Delete parameter { $tag }** in the disclosure **Delete the parameter** (draft), and a link back to the version page | `RecipeView` |

The form of a new parameter has these fields from top to bottom. The edit form of a parameter has the same fields.

- **Unit**: a list of the live units of the machine. The form marks it as required.
- **Tag**: a text field with a hint. The form marks it as required.
- **English name** and **English description**: text fields. The form marks both as required.
- **Type**: a text field with a list of suggested types. The form marks it as required.
- **PLC address**: a text field with a hint. It shows only for a Siemens platform. The form marks it as required only for **Siemens S7-300**.
- **Order in the unit**: a text field with a hint. The form marks it as required.
- The value grid with the caption **Values and process limits**. Until you enter a type and select **Show value fields**, the form shows the text "Enter the type and then show the value fields."

The button **Show value fields** shows the grid for the type that you entered. The first button of the form is **Add parameter**, or **Save parameter** in the edit form. The key Enter in a field saves the form.

## Export (`api.md` section 2.7)

The export routes return JSON. They have no page. The version page holds the export link of each approved or retired version. See [export-format.md](export-format.md) for the document.

## Users (`api.md` section 2.8)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| My account | `GET /me` | The **Login** and the **Role** of the signed-in user. | **Change password** (link) | `authenticated` |
| Change password | `GET /me/password` | Fields **Current password** and **New password**, with the password rules. The only page that a session with a required password change can open. | **Change password** | `authenticated` |
| Users | `GET /users` | Filter **Login contains**. Columns: **Login**, **Role**, **Status**, **Password change**. Each row opens its user. | **Search**, **New user**, **Next page** (link) | `UserManage` |
| New user | `GET /users/new` | Fields **Login** and **Role**. | **Create user** | `UserManage` |
| User created | Answer of `POST /users`. It is not a `GET` route. | The login and the one-time password. The page shows the password once. | **Back to the user** (link) | `UserManage` |
| User | `GET /users/{login}` | The fields **Login**, **Role**, **Status**, and **Password change** describe the account. **Status** shows **Active** or **Inactive**. The fields **Created**, **Deactivated**, **Failed logins in the window**, and **Last failed login** show its history. | **Set role** (field **New role**), **Deactivate**, **Activate**, **Reset password**, and **Unlock login** (`LoginUnlock`) | `UserManage` |

## Audit and security events (`api.md` section 2.9)

| Screen | Route | What it shows | Main actions | Permission to open |
|---|---|---|---|---|
| Audit log | `GET /audit` | The newest entries first. Filters: **Object type**, **User login**, **Action** (**Every action**), **From (UTC, YYYY-MM-DD)**, and **To (UTC, YYYY-MM-DD)**. Each row opens its entry. A user with `AuditViewRecipes` sees recipe entries only. | **Show**, **Clear the filters** (link), and **Security events** (link, `AuditView`) | `AuditView` or `AuditViewRecipes` |
| Audit entry { $id } | `GET /audit/{id}` | The fields **Time**, **User**, **Action**, **Object type**, **Object**, **Aggregate**, **Request id**, **Client address**, **Session reference**, and **Position in the chain**. The states **State before** and **State after** as formatted JSON. | **Entries of this object**, and **Entries of the whole aggregate** (links) | `AuditView` or `AuditViewRecipes` |
| History of recipe { $reference } | `GET /recipes/{ref}/history` | The audit entries of one recipe, newest first, without state bodies. | **Entries of the whole recipe** (link) | `RecipeView` |
| Security events | `GET /security-events` | Filters: **Kind** (**Every kind**), **Login**, **Client address**, **From (UTC, YYYY-MM-DD)**, and **To (UTC, YYYY-MM-DD)**. Columns: **Time**, **Count**, **User agent**, **Request id**, and **Detail**. A row with a count above 1 shows the login of its first event. | **Show**, **Clear the filters** (link) | `AuditView` |
