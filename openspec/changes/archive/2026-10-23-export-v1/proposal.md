# Proposal

## Why

The use cases `ExportApprovedVersion` and `ExportVersionByNumber` exist, but no route, no writer, and no schema file exist. A user cannot download a document, and the transfer team has no schema. Step P3.5 closes this gap. The change satisfies R03, R18, R24, R27, R28, R30, R31, R36, R39, and R40.

The change uses A2, A4, A8, A24, A26, A43, and A44.

## What Changes

- `crates/recipe-export/schema/export-v1.schema.json` (JSON Schema 2020-12), closed on each object with fixed fields (ADR-0019).
- A document writer in `recipe-export` with the field order and the presence rule of `api.md` 6.1. The keys of a struct value follow `types[].members[]`.
- Golden tests of the example of `api.md` 6.2 and of a `Retired` version. Both pass the schema.
- The flow `ExportVersion` writes the document bytes before the commit of `VersionExported`. `E-EXP-004` and `E-WEB-011` therefore write no entry.
- The routes of `api.md` 2.7 with `Content-Disposition`, `Cache-Control: no-store`, and only `GET` (ADR-0054).
- JSON error bodies in the UI language. A missing session and a forced password change keep their redirects (ADR-0053).
- A guard refuses same-site and HTMX requests with `403 E-WEB-007` (ADR-0055).
- At most `export.permits` exports run at the same time (ADR-0056).
- A type reference of the hashed body is a domain type.
- An export link on the version page (ADR-0052).
- A how-to guide `docs/user/check-export-document.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `export`: the schema file, the field and member order, the error answers, the method rule, the writer before the commit, and the permits.
- `web-ui`: the export link on the version page and the guard of the export routes.

## Impact

- Crates `recipe-domain`, `recipe-app`, `recipe-export`, `recipe-web`. `recipe-db` gets tests only.
- A new dev-dependency `jsonschema` 0.60.0 of `recipe-export` without default features.
- ADR-0052 to ADR-0056. Changes in `api.md` sections 2.7, 5.1, 5.3, 7.3 and `reliability.md` sections 7.4, 7.11. An amendment note on ADR-0049.
- No migration, no grant, no index, and no new error code.

## Out of scope

- The counter of `E-EXP-002` for the job `alerts` (step P4.1).
- `serve`, Compose, and the health listener (step P4.1).
- The Playwright tests of the browser checks (step P4.2).
- A coded `404` for an unknown path or method (step P4.2).
- A signature field and an API token for a transfer system (ADR-0019).
- A route that serves the schema file.
