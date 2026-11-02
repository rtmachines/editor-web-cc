# Proposal

## Why

Release 0.3.0 lists four gaps of the web UI in its section "Known gaps". The project owner also found a fifth gap: the parameter table of the version page shows no value. A user must open each parameter page to read one value. The change satisfies R08, R12, R13, R14, R17, R27, and R39.

The change uses A43.

## What Changes

- The parameter table of the version page gets the column Value. An elementary parameter shows the text of its value. A composite parameter shows a word from the bundles. A long text ends after a fixed count of characters.
- The header of an Approved or a Retired version shows `content_sha256`.
- The version page gets a compare form with two lists of version numbers. The new route `GET /recipes/{ref}/compare` answers `303` to the diff route of ADR-0050.
- The success toast of each command after which the draft stays a live draft names the new `row_version`. `SetParameterValue` and `ReplaceLimits` get a success toast. In total, eight toasts name the change number.
- The type page of a machine on an S7 platform shows the S7 layout of the type. The layout lists the offset of each member and the size of the type.
- ADR-0108 records the five decisions. ADR-0043 (j) and ADR-0050 (l) get "Amended by" notes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the version page, its parameter table, its compare form, the success toasts of the draft commands, and the type pages.

## Impact

- Crates: `recipe-domain` (member offsets of the S7 layout), `recipe-app` (`VersionDetail`, the new port method `parameter_values`, `value_text`, `AddParameterOutput`), and `recipe-db` (two read statements). Also `recipe-web` (templates, pages, bundles `pl` and `en`, one route).
- Values: the new port method `parameter_values(version, keys)` reads the values of one page. Only `ListParameters` calls it. `parameter_page` and `GetVersion` load no value.
- Documents: ADR-0108, `api.md` sections 2.3, 2.5, and 2.6, `reliability.md` section 7.11 (one new row), `design-system.md`, and `CHANGELOG.md`.
- E2E: the Playwright suite checks the width of the version page and of the type page at 320 px.
- No migration. No new error code. No new permission. No new dependency.

## Out of scope

- The limits of a parameter in the parameter table. The parameter page shows them.
- The elements of an array in the S7 layout. An array shows one row.
- The S7 layout on a machine with `CodesysIec` or `TwinCat3`.
- A filter or a sort of the parameter table by value.
- The other gaps of release 0.3.0: the soak test, the seven `#[cfg(unix)]` tests, and the open low-severity review items.
