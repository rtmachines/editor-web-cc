# Approve a version

## Purpose and reader

This guide shows how to approve a draft version of a recipe. After the approval, the version is the active version of the recipe and a PLC can load it.

The reader is a ProcessEngineer or an Administrator who reviews the draft of a colleague. The guide also shows how to retire an approved version and how to discard a draft.

The editor shows Polish by default. The buttons **Polski** and **English** in the top bar change the language. This guide uses the English texts.

## Sources

This guide links to the rules and does not repeat them.

- [`api.md`](../architecture/api.md) section 2.5 gives the routes of the approval, the retirement, and the discard.
- [`api.md`](../architecture/api.md) section 3.3 gives the permissions of each role.
- [`api.md`](../architecture/api.md) section 4 gives the rules for the row version and for the repeated approval.
- [`reliability.md`](../architecture/reliability.md) section 1.2 gives the error codes.
- [`domain-model.md`](../architecture/domain-model.md) section 4 gives the states of a version.
- [ADR-0049](../adr/0049-approval-preview-and-lifecycle-actions.md) gives the approval page and the lifecycle actions.
- [ADR-0022](../adr/0022-approval-policy.md) gives the separation of duties and the rule for process limits.
- [ADR-0018](../adr/0018-udt-revision-rebind.md) gives the rule for outdated parameters.
- [ADR-0015](../adr/0015-concurrency-locking-model.md) gives the rule for a repeated approval.

## Prerequisites

- Your role is Administrator or ProcessEngineer. These roles have the permission `VersionApprove`.
- You did not create the draft and you did not edit it. By default, the person who changed a draft cannot approve it ([ADR-0022](../adr/0022-approval-policy.md)).
- The draft has at least one parameter.
- The recipe is not deleted.

## Procedure

1. Choose **Recipes** in the main navigation. Open the recipe. Open the draft in the table **Versions**.
2. Choose the link **Approve version 4**. The number is the number of your draft. The approval page opens.
3. Look at the top of the page. If the box **The approval is not possible** shows, go to the section "Errors on this path". The page then has no form.
4. Read the section **Summary of the changes** and the sections below it. The editor compares the draft with the current approved version. The section **Process limits** highlights each changed limit. See [how to compare two versions](how-to-compare-two-versions.md).
5. Read the notice in the form **Approval**. It names the approved version that the approval retires. For a first approval, it says that every parameter is new.
6. Look for the group **Outdated parameters**. If it shows, decide what to do with the outdated parameters. See "Outdated parameters" below.
7. Choose **Approve version 4** once. The server processes the request and the version page opens.

You can choose **Go to the approval form** in the summary to jump to the form.

### Outdated parameters

A parameter is outdated when a newer revision of its type exists ([ADR-0018](../adr/0018-udt-revision-rebind.md)). The group lists each outdated parameter with a link.

Choose one of these actions:

- Rebind the parameters in the draft. Then start the review again with step 2.
- Select the box **Approve with the outdated parameters**. The hint says that the audit log records this choice.

If the group does not show, the draft has no outdated parameter. The box does not exist.

## Result

The version page shows the status **Approved**. It shows your login under **Approver** and the time under **Approved at**. A toast names the result:

- "Version 4 is approved. Version 3 is retired." when an approved version existed.
- "Version 4 is approved." when the recipe had no approved version.

The previous approved version now has the status **Retired**. A recipe has at most one approved version ([`domain-model.md`](../architecture/domain-model.md) section 4). To get the file, see [how to export a version](how-to-export-a-version.md).

### Double click

Choose the button once. The form drops a second request while the first request runs ([`api.md`](../architecture/api.md) section 4, rule 7). A second request may still reach the server. Then the version page shows the toast "Version 4 is already approved." The second request writes no audit entry ([`api.md`](../architecture/api.md) section 4, rule 8).

## Errors on this path

Each error shows its code, one sentence, and a reference. Give the reference to the administrator when you ask for help.

| Code | Status | What the page says | What to do |
|---|---|---|---|
| `E-RCP-015` | 409 | A user created or edited the draft. A different user must approve it. | Ask another Administrator or ProcessEngineer to approve the draft. The page shows this notice without a form. |
| `E-RCP-016` | 422 | A parameter leaf has no process limit. Set a limit before approval. | Open the linked parameter and set the limit. See [how to edit a parameter in a draft](how-to-edit-a-parameter-in-a-draft.md). Start the review again. |
| `E-RCP-008`, `E-RCP-020`, `E-RCP-022`, `E-RCP-023` | 422 | The draft has no parameters, a parameter is not valid, or an S7 address overlaps or starts at an odd byte. | Open each linked parameter and correct it. Start the review again. |
| `E-RCP-024` | 422 | A parameter uses an old revision of its type. The line stands under the box **Approve with the outdated parameters**. | Rebind the parameter in the draft, or select the box and choose **Approve version 4** again. |
| `E-WEB-002` | 403 | Your role does not permit the operation **Approve version**. | Ask an Administrator or a ProcessEngineer to approve the draft. The role Operator has no permission to approve. |
| `E-WEB-001` | 409 | Another user changed the version. | Choose **Reload version**. Then start the review again with step 2. |
| `E-RCP-007` | 409 | The version is not a draft. | Open the recipe. Another user may have approved or discarded the draft. |
| `E-RCP-012` | 409 | The recipe is deleted. | Ask an Administrator to restore the recipe. |
| `E-WEB-006` | 503 | The system is in read-only mode because the free disk space is low or unknown. | Wait. Ask the administrator to free the disk space. No approval is possible until then. |
| `E-WEB-007` | 403 | The form is no longer valid. | Reload the page and approve again. |
| `E-EXP-004` | 500 | The server cannot create the export document. | Contact the administrator. Give the reference. |

The page may also show the notice "The draft changed during the review. Start the review again." This notice has no code. Choose **Start the review again** to read the new diff.

## Retire an approved version

Only an Administrator can retire a version (`VersionRetire`). After the retirement, the recipe has no approved version. The export of the approved version stops until the next approval.

1. Open the version page of the approved version.
2. Go to the end of the page, to the heading **Actions**. Open **Retire this version**.
3. Read the warning. Choose **Retire version 3**. The number is the number of your version.

The version page shows the status **Retired** and the toast "Version 3 is retired."

This action can return these errors:

- `E-RCP-009`: the version is not approved.
- `E-WEB-001`, `E-WEB-002`, `E-RCP-012`, and `E-WEB-006`: see the table above.

## Discard a draft

An Administrator or a ProcessEngineer can discard a draft (`VersionDiscard`). Nobody can edit or approve a discarded draft again.

1. Open the version page of the draft.
2. Go to the end of the page, to the heading **Actions**. Open **Discard this draft**.
3. Read the warning. Choose **Discard draft 4**. The number is the number of your draft.

The recipe page opens.

This action can return these errors:

- `E-RCP-007`: the version is not a draft.
- `E-WEB-001`, `E-WEB-002`, `E-RCP-012`, and `E-WEB-006`: see the table above.

## Delete and restore a recipe

The actions to delete and to restore a recipe are on the recipe page and in the recipe list. They have no part in the approval. See [the screen reference](reference-screens.md) for the pages, and [the permission reference](reference-permissions.md) for the roles.
