# Proposal

## Why

An external adapter loads approved recipes into the PLCs (ADR-0109, ADR-0154). The editor does not know the machine state, and a user cannot ask for a load.

On 2026-11-25 the owner decided that the adapter reports each machine state and receives load requests. The adapter starts every connection. The PLC interlock decides each load.

The change satisfies R03 in the reading of A55, and R18, R20, R28, R29, R31, R38, and R39. It uses A24, A42, A45, and A47.

## What Changes

- ADR-0224 records the adapter link and amends 18 ADRs. A55 is the new reading of R03.
- `ExportClient` also gets `MachineLinkReport`. Every person role gets `RecipeLoadRequest`.
- Routes under `/adapter/v1/` accept JSON and HTTP Basic, with their own permit and no CSRF token. Caddy refuses them over plain HTTP.
- A status report gives the state and loaded recipe of each machine and returns its requests.
- A `LoadRequest` covers the whole approved version. A machine has at most one active request.
- The adapter claims a request, renews the claim in each report, and sends the result. An unknown result blocks the machine until a person releases it.
- The machine page shows a link panel with passive polling. A loadable approved version gets a load form.
- In read-only mode, the editor accepts reports and results only.

## Capabilities

### New Capabilities

- `machine-link`: status reports, link state, load requests, and the adapter routes. The owner approved this eighth capability.

### Modified Capabilities

- `export`, `identity-access`, `web-ui`, `plant-model`, `operations`, and `audit`.

## Impact

- Five crates (all except `recipe-export`), one migration, `deploy/`, and `e2e/`.
- ADR-0224, A55, six documents of `docs/architecture/`, a how-to in `docs/ops/`, the rules files, and `CLAUDE.md`.
- A rollback of the image only needs 0 audit entries of the new actions.

## Out of scope

- An adapter program, a PLC client, a PLC interlock, and an HMI confirmation.
- A load of a part of a version.
- A load for a machine with the string encoding `Utf8`.
- A history of the reported states.
- A binding of an `ExportClient` account to some machines.
- A message broker.
- Implementation before the end of `value-body-memory-load` and change B.
