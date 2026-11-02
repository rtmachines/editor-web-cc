# Proposal

## Why

A transfer client, for example a PLC adapter for an S7-300, must pull the `Approved` version of a recipe over HTTP with a checksum. The export routes need a session cookie, which only a browser gets. The change satisfies R03, R18, R20, R24, R28, R30, and R31. It changes the reading of R22.

The change uses A2, A8, A10, A18, A19, A24, A33, A42, and the new A45.

## What Changes

- A new role `ExportClient` for a technical account. It holds `VersionExport` only and cannot log in to the UI.
- Three new routes accept only HTTP Basic of an active `ExportClient` account:
  - `GET /export/v1/machines/{code}/recipes` lists the recipes of a machine with an `Approved` version and their `content_sha256`.
  - `GET /export/v1/machines/{code}/recipes/{ref}/approved` returns the v1 document.
  - `GET /export/v2/machines/{code}/recipes/{ref}/approved/units/{path}` returns the v2 document of one unit and its descendants, with `scope_sha256`.
- **BREAKING** for a v2 consumer only: the export document v2 adds `scope` and `scope_sha256`. The v1 document and the v1 routes do not change.
- An archived machine stays exportable on the new routes.
- Three new error codes: `E-EXP-005`, `E-IDN-011`, and `E-WEB-012`.
- ADR-0109 records the decisions.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `export`: the machine list, the machine route, and the unit scope document v2. Also the Basic check and the audit actor of a client export.
- `identity-access`: the role `ExportClient`, its permission, its refused UI login, its password, and the refused role change.
- `web-ui`: the user form offers the role `ExportClient`. The password page of that role says that the password is permanent. The set-role form never offers it.
- `operations`: the HTTP site of the proxy refuses `/export/` paths without a redirect.

## Impact

- Crates: the five library crates. `recipe-server` gets the setting `export.basic_permits` when it reads the settings of the export.
- Migration: two `CHECK` constraints on `app_user`.
- Deploy: both Caddyfiles. A runbook in `docs/ops/`.
- Documents: ADR-0109 and five files of `docs/architecture/`.
- No new dependency. No new table.

## Out of scope

- API tokens and a CLI token (ADR-0109, rejected options).
- Mutual TLS on the proxy.
- A signature of the export document (ADR-0019, next step).
- A push to a client (R03).
- A unit scope without the descendant units.
- Export of a `Retired` version through HTTP Basic.
- Pages of the machine list.
