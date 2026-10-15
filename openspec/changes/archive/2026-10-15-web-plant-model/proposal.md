# Proposal

## Why

The plant model has commands in `recipe-app` and `recipe-db`, but no page shows a machine, a unit tree, or a UDT type. A process engineer cannot build the unit tree of a machine or define a UDT before step P3.3 adds parameters. `recipe-app` has no query of `api.md` sections 2.2 and 2.3.

This change satisfies R05, R06, R07, R12, R13, R15, R19, R24, R28, R38, and R39. It uses A9, A17, A24, A30, A32, A38, and A44.

## What Changes

- ADR-0037 records the decisions. `design-system.md`, `api.md` sections 2.1 to 2.3, and `reliability.md` section 7.11 hold them.
- `recipe-app`: the ten queries of `api.md` sections 2.2 and 2.3. Each query checks its permission first.
- `recipe-app`: read ports for the machine list and the type usage, with memory adapters. Three new variants of `Missing`.
- `recipe-db`: the sqlx adapters of the two read ports, with contract tests. No migration and no new grant.
- `recipe-web`: the pages and fragments of `api.md` sections 2.2 and 2.3. They include the whole unit tree and the UDT form with member rows.
- `recipe-web`: the navigation link "Machines", the state "Archived", and the style sheet `components.css`.
- `examples/dev_server.rs` of `recipe-web` for the test in a browser.
- New texts in both bundles. The XSS corpus goes through every new text field.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: the machine list, the type list, the type tree of a revision, and the type usage.
- `web-ui`: the plant pages, the member rows, the static files, and the layout. Also the navigation, the archived state, and the field error lines of a list.

## Out of scope

- The recipe editor, the parameter pages, and the full `/recipes` list (P3.3).
- The approval, the diff, the history (P3.4), and the export pages (P3.5).
- `recipe-server serve`, Compose, and the jobs (P4.1).
- The rebind of a draft. The usage page only lists the parameters.
- The review items R6, D9, M7, and M9 (P4.2), and R2-3, R2-5, and C6 (P4.1).
- An error line under each member input (P3.3).

## Impact

- `crates/recipe-app`, `crates/recipe-db`, and `crates/recipe-web`.
- No new crate, dependency, or migration. The CSP and the scripts do not change.
