# Edit a parameter in a draft

## Purpose and reader

This guide shows how to change the value or a process limit of one parameter in a draft version and how to save the change.

The reader is a process engineer or an administrator who edits a recipe. The guide also shows what you see when the save fails and what you do next.

The editor shows Polish by default. The buttons **Polski** and **English** in the top bar change the language. This guide uses the English texts.

## Sources

This guide links to the rules and does not repeat them.

- [`api.md`](../architecture/api.md) section 2.5 gives the routes of the versions.
- [`api.md`](../architecture/api.md) section 2.6 gives the routes of the parameters.
- [`api.md`](../architecture/api.md) section 3.3 gives the permission of each role.
- [`api.md`](../architecture/api.md) section 4 gives the concurrency protocol.
- [`api.md`](../architecture/api.md) section 5 gives the error answers.
- [`reliability.md`](../architecture/reliability.md) section 1.2 is the only list of error codes.
- [ADR-0008](../adr/0008-optimistic-concurrency-row-version.md) gives the reason for the change number.
- [ADR-0039](../adr/0039-parameter-value-form.md) gives the form of a value and the leaf grid.
- [ADR-0041](../adr/0041-error-lists-of-a-form.md) gives the error lists of a form.

## Prerequisites

- You have a session. Log in first.
- You have the permission `ParameterEdit`. The roles Administrator and ProcessEngineer have it. The role Operator does not have it (`api.md` section 3.3, [`reference-permissions.md`](reference-permissions.md)).
- To create a draft, you also need the permission `VersionCreate`.
- The version is a draft. The page of an approved or retired version has no form (`api.md` section 2.6).

Without the permission, the parameter page shows the values and the limits as read-only text. The editor refuses a save with `403 E-WEB-002`.

## Procedure

### Open the draft

1. Select **Recipes** in the top bar.
2. Find the recipe in the list. Use the field **Search** and the button **Show** to narrow the list.
3. Look at the column **Draft** of the recipe. If it shows a version number, open that number.
4. If the column shows **none**, open the reference number of the recipe. Then select **New version**.
5. Select a **Base version**, write a **Change note**, and select **Create version**.

The editor opens the new draft after step 5 (`api.md` section 2.5). A recipe holds one draft at a time. A second draft gives `409 E-RCP-011`. The message names the number of the existing draft. Open that draft.

The page of the version shows the badge **Draft** at the top.

### Find the parameter

1. Look at the table **Parameters**.
2. If the draft has many parameters, the table shows one unit.
3. Use the links above the table to change the unit.
4. Open the tag of the parameter.

The table shows the columns **Tag**, **Name**, **Type**, **Unit**, and **State**. A platform with PLC addresses adds the column **PLC address**. Each link above the table names a unit and its number of parameters.

The page of the parameter opens. Its title is **Parameter** and the tag. It shows the definition and the form to edit it.

### Change a value or a limit

1. Find the table **Values and process limits**.
2. Write the new value in the column **Value** of the row. For a BOOL leaf, select **TRUE** or **FALSE**.
3. To change a process limit, write the new number in the column **Minimum** or **Maximum** of the same row.
4. For a text leaf, change **Maximum length**.

Each row of the table is one leaf of the type. The columns are **Path**, **Value**, **Minimum**, **Maximum**, **Maximum length**, and **Type**.

Follow the hint above the table for the form of a number or of a time. A number uses a dot, for example `1.5`. A duration or an instant uses an IEC literal, for example `T#1s` (`domain-model.md` section 2.1).

A cell with the words **not applicable** has no limit for that leaf. An array with many elements shows its elements in closed groups. Open a group with its toggle button.

The save sends the values and the limits together (ADR-0039). You can change a value and its limit in one save.

If you change the field **Type**, the editor does not save. It shows the value fields of the new type and the notice "The type changed. Enter the values of the new type." Enter the values and save again.

### Save

1. Select **Save parameter**. Or press Enter in an input field of the form.
2. Wait for the toast. Do not select **Save parameter** again while the request runs. The browser drops a second click.

Enter in an input field uses the first button of the form. This button is **Save parameter** (ADR-0039). The button **Show value fields** does not save.

## Result

After a successful save, the page shows these items:

- A toast with the text "Parameter `T_HOLD` is saved." The text names the tag of your parameter.
- The updated definition at the top of the page.
- The form with the stored texts. The editor writes a number in its canonical form, so `1.50` becomes `1.5`.
- An empty error panel.

The editor also sets a new change number in the hidden fields of all forms on the page (`api.md` section 4, ADR-0008). You do not see this number. The next save uses it.

Every save changes the change number of the whole draft. A save of a different parameter by another user also changes it.

## Errors on this path

Every error shows its code, one sentence, and a reference. Report the code and the reference to the administrator when the sentence says so (`api.md` section 5.2).

### Field errors

A save with bad fields gives `422`. The editor writes nothing. It shows the panel at the top of the page with the code, the number of errors, and one line for each error. Each line names the tag and the leaf, and it links to the input (ADR-0041).

The same error also stands under its input, with the code. The panel shows at most 20 lines. The form keeps the texts that you wrote. A group of the grid with an error is open.

Fix every marked input and save again. The codes that you meet on this form are:

| Code | What it means | What you do |
|---|---|---|
| `E-RCP-004` | The value is outside the process limit | Write a value inside the limit. Or widen the limit in the same save. |
| `E-RCP-005` | The text does not match the type | Write the value in the form that the message expects. Use a dot in a number. |
| `E-RCP-006` | The limit is outside the range of the data type | Write a limit inside the range of the type. |
| `E-RCP-018` | The minimum is greater than the maximum | Make the minimum smaller than the maximum. |
| `E-WEB-008` | A definition field is not valid | Read the reason in the message. Change the field. |

`reliability.md` section 1.2 gives the full text of each code.

### Conflict

The code `409 E-WEB-001` means that another user saved the draft after you opened the page. The change number of your form is old.

The panel shows these items:

- The current change number.
- The login of the last editor and the time of the last change. If the editor cannot find them, it shows "The system cannot show the last change."
- The warning "Your changes are not saved. The reload discards them."
- The link **Reload version**.

Your input stays in the form until you reload. To continue:

1. Write down the values that you typed.
2. Select **Reload version**.
3. Find the parameter again. Read its current value and limits. The other user may have changed them.
4. Apply your change again and save.

The conflict also happens when the other user changed a different parameter. A draft is one unit of work (`api.md` section 4).

### Other errors

| Code | What you see | What you do |
|---|---|---|
| `E-RCP-002` | The version is not a draft | Open the recipe. Use its draft or create a new draft. |
| `E-RCP-014` | The new tag is already in use in this version | Choose a different tag. |
| `E-WEB-004` | "Log in to continue." | Log in again. |
| `E-WEB-006` | Read-only mode | See below. |

### Read-only mode

The code `503 E-WEB-006` means that the free disk space of the server is low or unknown. The page shows the banner "Read-only mode" on every page.

The editor saves nothing in this mode. You can still view and export. Do not try again. Contact the administrator and give the reference of the error.

## Outdated parameter

A new revision of a UDT type makes a parameter **Outdated** when that parameter uses an older revision. The version page lists these parameters under **Outdated parameters**.

The rebind moves the parameters to the newest revision. It is not part of this guide. Read the rules in [ADR-0018](../adr/0018-udt-revision-rebind.md) and [ADR-0044](../adr/0044-bound-of-the-rebind-forms.md). The routes are in `api.md` section 2.5. [`how-to-add-a-udt.md`](how-to-add-a-udt.md) describes how a new revision comes into existence.

## Next steps

- [`how-to-approve-a-version.md`](how-to-approve-a-version.md) shows how a different user approves the draft.
- [`how-to-compare-two-versions.md`](how-to-compare-two-versions.md) shows how to read the changes of the draft.
