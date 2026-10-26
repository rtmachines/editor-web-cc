# Compare two versions of a recipe

## Purpose and reader

This guide shows how to compare two versions of one recipe and how to read the changes.

The reader is a process engineer, an operator, or an auditor. You want to know what changed between version `m` and version `n` before you approve, load, or review a version.

The UI has the languages Polish (default) and English. This guide describes the English texts. Use the buttons **Polski** and **English** in the page header to switch the language.

## Sources

This guide links to the rules and does not repeat them.

- [`api.md`](../architecture/api.md) section 2.5 gives the route of the diff page.
- [`api.md`](../architecture/api.md) section 2.9 gives the route of the history page.
- [`api.md`](../architecture/api.md) section 1 gives the answer for a bad address value.
- [`reliability.md`](../architecture/reliability.md) section 1.2 gives the error codes.
- [ADR-0050](../adr/0050-diff-of-two-versions.md) gives the content and the grouping of the diff.
- [ADR-0051](../adr/0051-audit-history-and-security-event-views.md) gives the content of the history page.
- [`design-system.md`](../architecture/design-system.md) sections 8 and 9 give the states and the icons of the diff.

## Prerequisites

- You have a session. Every role can read a diff, because every role has the permission `RecipeView` ([`api.md`](../architecture/api.md) section 3.3).
- You know the reference number of the recipe, for example `R-500`.
- The recipe has two versions at least.

## Procedure A: open the diff from a version page

Use this procedure when you compare a version with the approved version or with its base version.

1. Open **Recipes** in the main menu. Open your recipe.
2. Find the table **Versions**. Select the number of the version in the column **Version**.
3. Find the row of links under the version data. Select **Compare with the approved version N** or **Compare with the base version N**.
4. Read the diff page. The title names the new version, the recipe, and the old version.

The version page shows a link only when that version exists and differs from the shown version. A version that has no base and no other approved version shows no compare link. Then use procedure B.

The compare links put the higher number on the new side. A diff of version 3 against version 1 therefore opens the address `/recipes/R-500/versions/3/diff/1`.

## Procedure B: choose any two versions

The pages have no form to choose two version numbers. You type the address of the diff page.

1. Find the two version numbers. Use the table **Versions** of the recipe page or procedure C.
2. Decide which version is old and which is new. The old version is the one that you compare against.
3. Open the address `/recipes/{ref}/versions/{n}/diff/{m}`. Replace `{ref}` with the reference number.
4. Replace `{n}` with the number of the new version. Replace `{m}` with the number of the old version.

For example, `/recipes/R-500/versions/4/diff/2` shows the changes from version 2 to version 4. The route accepts any two existing versions of the recipe. A version number counts inside one recipe, so you cannot compare a version of another recipe.

## Procedure C: find version numbers in the history

Use this procedure when you do not know which version holds a change.

1. Open the recipe page. Select **History**. The version page has the same link.
2. Read the table **Entries of the history**. The newest entry stands first.
3. Find the entries with the object **Version** and an action such as **Version created** or **Version approved**. The object shows the version number.
4. Select **Older entries** to read older entries. Select **Newest entries** to go back.
5. Use the numbers in procedure B.

The history shows the time, the user, the action, and the object. It does not show the old and new values. A page holds at most 100 entries ([`reliability.md`](../architecture/reliability.md) section 7.11). The text **No older entries.** replaces the link **Older entries** on the last page.

## Read the diff

The page has a summary list and one table for each section that holds a change. A section without a change does not appear.

1. Read the list under **Summary of the changes**. Each item names a section and gives the number of parameters, and for some sections the number of rows.
2. Select the name of a section. The link goes to the table on this page or to the first page of that section.
3. In each table, read the old side and the new side in the same row.
4. Read the last column **Change**. It shows the state of the row.

The summary ends with the count **Parameters without a change**. If the two versions have the same parameters, the page shows the text "Versions N and M have the same parameters."

The sections come in this order. Each state shows a color, an icon, and a word, so you do not need the color.

| Section | Word of the state | Icon | What the row shows |
|---|---|---|---|
| **Process limits** | **Limit changed** | Two vertical bars with a dot between them | One limit row per path. The old and the new limit stand side by side. |
| **Values** | **Value changed** | Two arrows that point in opposite directions | One value per path. The old value, the new value, and the limit of the new version stand side by side. |
| **Definitions** | **Definition changed** | Two arrows that point in opposite directions | One changed field of a parameter, for example **Tag**, **Unit**, **Type**, or **S7 address**. |
| **Translations** | **Translation changed** | Two arrows that point in opposite directions | One language. The name and the description of both sides stand side by side. |
| **Added parameters** | **Added** | Plus sign in a square | A parameter that only the new version holds. |
| **Parameters only in version N** | **Only in the old version** | Minus sign in a square | A parameter that only the old version holds. |

Note these points when you read a table:

- The column **Parameter** shows the tag. Select it to open the parameter in the version that holds it.
- The columns **Version N** show the old and the new side. An empty side shows the word **none**.
- A limit shows as "N to M", "at least N", "at most N", or "Maximum length: N".
- The table **Process limits** has a red frame. It holds every changed limit, also the limits of parameters that only one version holds.
- A parameter can appear in several sections. For example, a changed value and a changed limit stand in two sections.
- When the type of a parameter changes, the tables show every value and every limit row of both sides. This includes the rows with an equal text ([ADR-0050](../adr/0050-diff-of-two-versions.md) point (c)).
- A structured value in the sections **Added parameters** and **Parameters only in version N** shows the count **Leaves: N** and not the value.
- A row **More rows: N** means that the table cuts the rows of one parameter. Select **Parameter in version N** to read the whole parameter.

## Large diffs

One page shows at most 50 parameters over all its sections ([`reliability.md`](../architecture/reliability.md) section 7.11). A section that the limit cuts has a link under the tables with the text **Pages of the section …**.

1. Select the link of the section. The page shows only that section.
2. Select **Next page** or **Previous page** under the table. The pager shows **Page N**.
3. Select **Back to version N** at the end of the page to return to the version page.

## Result

The page shows the changes from the old version to the new version. You know which limits, values, definitions, and translations changed, and which parameters the new version added or lost.

The diff is a read. It changes nothing in the recipe.

## Errors on this path

| Code | You see | What to do |
|---|---|---|
| `E-WEB-005` | "Version 9 of recipe R-500 does not exist." or "Recipe R-999 does not exist." The status is `404`. | Read the table **Versions** of the recipe. Type a number that exists in this recipe. Version numbers start at 1, and `0` or a text is not valid. |
| `E-WEB-008` | "Field Kind is not valid: …". The status is `422`. | Use the address without the part after `?`. The page links never add a bad `kind` or `page`. |
| `E-RCP-012` | "Recipe R-700 is deleted. An administrator can restore it." The status is `409`. | Ask an Administrator. Only an Administrator reads the diff and the history of a deleted recipe. |
| `E-WEB-009` | "The request took too long and was stopped. Try again." The status is `503`. | Open the page again. The server may run other diffs at the same time ([ADR-0050](../adr/0050-diff-of-two-versions.md) point (g)). |
| `E-WEB-011` | "The operation … did not complete. Try again." The status is `500`. | Open the page again. If the error repeats, report the `request_id` that the error shows. |

Two more messages are not errors:

- "Page N is after the last page." appears when the address holds a page after the last page. Select **Go to the first page**.
- "The section “…” has no change." appears when the address holds a section that has no change. Select **Go to the first page**.

The history page gives `E-WEB-005` for an unknown recipe and `E-RCP-012` for a deleted recipe, as described above.
