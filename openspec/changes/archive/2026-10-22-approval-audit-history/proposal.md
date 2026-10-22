# Proposal

## Why

A draft cannot become Approved in the web UI. Nobody can retire a version, discard a draft, delete or restore a recipe, compare two versions, or read the history and the audit log. Step P3.4 adds these pages. A second person sees every changed limit before the approval (ADR-0022). The change satisfies R09, R10, R13, R17, R20, R23, R24, R25, R26, R27, R28, R30, R31, R35, R36, R38, R39, and R40.

The change uses A13, A14, A20, A24, A26, A28, A29, A33, A34, A36, A40, A42, A43, and A44.

## What Changes

- Approval preview: the diff to the current Approved version, limit changes first, the outdated parameters with `allow_outdated`, and the blockers of the approval (ADR-0049).
- Actions approve, retire, discard, delete, and restore with CSRF, `hx-sync`, and `row_version` where the command takes it. Retire, discard, and delete stand in a danger disclosure. Restore stands in each deleted row of the recipe list.
- `recipe-domain` gets a pure diff of two versions by `ParameterKey`. The pages group the changes by kind, with pages per kind. Both read one `REPEATABLE READ` snapshot, and only one page of pairs loads (ADR-0050).
- Read ports with in-memory and `sqlx` adapters for the audit log, the recipe history, and the security events. They have a cursor, filters, and the scope of A20 (ADR-0051).
- Pages in `pl` and `en`: history, audit list, audit entry with formatted JSON, and security events. The navigation gets the link "Audit".
- New settings `page.audit`, `page.history`, `page.security_events`, `page.diff`, `diff.leaf_rows`, and `diff.permits`. New diff states and icons.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: approval preview, diff page, action forms, history, audit and security event pages, and the navigation link.
- `recipe-management`: diff of two versions, preview with blockers, and snapshot reads.
- `audit`: audit, history, and security event queries with cursor, filters, and the scope of A20.

## Impact

- Crates `recipe-domain`, `recipe-app`, `recipe-db`, and `recipe-web`. No migration, index, grant, or dependency.
- ADR-0049 to ADR-0051. Changes in `api.md`, `reliability.md`, `design-system.md`, and `domain-model.md`.

## Out of scope

- Export (step P3.5).
- A toast after the `303` answers of discard, delete, and restore (`api.md` section 2 gives these answers).
- A time zone other than UTC in the audit filters.
- A load test of the preview and the diff (step P4.2).
- The deferred items of step P3.3.
