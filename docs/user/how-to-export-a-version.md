# How to export a version

## Purpose and reader

This guide shows how to save the JSON file of a version of a recipe. The transfer system or an engineer then uses the file.

The reader is an operator at the machine or a process engineer. The guide does not show how to read the file. [Check an export document before a PLC load](check-export-document.md) does that.

The editor shows Polish by default. The buttons **Polski** and **English** in the top bar change the language. This guide uses the English texts.

## Prerequisites

- You have a session. You logged in.
- Your role has the permission `VersionExport` to export the `Approved` version. All three roles have it (`api.md` section 3.3).
- Your role has the permission `VersionExportHistory` to export a `Retired` version. The Administrator and the ProcessEngineer have it. The Operator does not.
- The version is `Approved` or `Retired`. A `Draft` has no export.
- The recipe is not deleted.

## Procedure: export the approved version

1. Choose **Recipes** in the main menu.
2. Find the recipe in the table. The column **Approved version** shows its approved version number.
3. Choose that number. The page **Version N of recipe R** opens.
4. Make sure that the badge **Approved** stands at the top of the page.
5. Choose the link **Export version N as JSON**.
6. Save the file. The browser may save it at once or ask for a place. The settings of the browser decide.

The browser stays on the version page. The editor shows no message after a successful export (ADR-0052).

## Procedure: export a retired version

1. Choose **Recipes** in the main menu. Choose the reference number of the recipe.
2. Find the version in the table **Versions**. Its column **State** shows **Retired**.
3. Choose the number of the version.
4. Read the notice above the link. It says that the file is not loadable into a PLC.
5. Choose the link **Export version N as JSON**.
6. Save the file.

The page shows the link only to a role with `VersionExportHistory`. The Operator sees no link on a `Retired` version.

## The result

The browser saves a file with the extension `.json`. The editor writes one audit entry for each export (`api.md` section 6.3).

| Version | File name |
|---|---|
| `Approved` | `recipe_{ref}_v{n}.json` |
| `Retired` | `RETIRED_recipe_{ref}_v{n}.json` |

In the name, `{ref}` is the reference number of the recipe and `{n}` is the version number. The prefix `RETIRED_` tells you that the version is not loadable (`api.md` section 6.3).

## The content hash

The content hash is the key `content_sha256` in the file. It has 64 characters, 0 to 9 and a to f. The version page does not show the hash.

Open the file in a text editor to read the hash. The hash stays the same for one version, also after a rename or a retirement (`api.md` section 6.1). The file also holds the key `loadable`. It is `true` only for an `Approved` version.

Do not edit the file. The hash covers only the fields with "In hash: yes" in [`api.md`](../architecture/api.md) section 6.1. A changed value in one of these fields does not match the hash.

The hash does not cover `loadable`, `version.status`, `version.approved_by`, `exported_at`, and the names. A change in one of these fields does not change the hash. [Export document format](export-format.md) lists the keys. [Check an export document before a PLC load](check-export-document.md) shows the check.

## Read-only mode

The export works in read-only mode (A24). The link stays on the page. The banner **Read-only mode** says that you can export.

## Errors

A failed export does not save a file. The browser shows a JSON text in the same tab (ADR-0052, ADR-0053). The text holds `code`, `message`, and `request_id`. Write down the three values. Go back with the browser.

A fault of the server may give plain text and not JSON (`api.md` section 5.1). The text shows the code `E-WEB-010` and no `request_id`. Write down the code and the time. Give both to the administrator. The administrator finds the line in the log by the time.

| Code or status | What you see | What to do |
|---|---|---|
| `E-EXP-001` | `404`. The recipe has no approved version. | Open the recipe again. Another user may have retired the version after you opened the page. |
| `E-EXP-002` | `500`. The version does not match its stored hash. The server refuses the export. | Stop. Do not load a file from this version. Call the administrator. Give the three values. |
| `E-EXP-003` | `409`. The version is a draft or the author discarded it. | Choose an `Approved` or a `Retired` version. |
| `E-EXP-004` | `500`. The server cannot create the file. | Call the administrator. Give the three values. |
| `E-RCP-012` | `409`. The recipe has the state Deleted. | Ask an administrator to restore the recipe. |
| `E-WEB-002` | `403`. Your role does not permit the operation. | Ask your supervisor. Only a role with `VersionExportHistory` exports a `Retired` version. |
| `E-WEB-007` | `403`. The server refused a request that did not come from the editor. | Open the version page again. Use the link on the page. |
| `E-WEB-009` | `503`. The request took too long. | Wait a short time. Try again. |
| `E-WEB-010` | `500`. An internal error occurred. The text may be plain text with no `request_id`. | Call the administrator. Give the code and the time. Give the `request_id` if the text shows it. |

Many users may export at the same time. The server then lets only a few exports run together (ADR-0056). An export that waits too long gives `E-WEB-009`. Try again.

If you saved the file but you do not trust it, do not load it. This also holds after `E-EXP-002`. Call the administrator.

### A session that expired

The browser goes to the login page and shows no JSON (ADR-0053). Log in. Then open the version page again and choose the link.

## Related pages

- [Export document format](export-format.md) lists the keys of the file.
- [Check an export document before a PLC load](check-export-document.md) shows how to check a file.
