# Proposal: identity-and-sessions

## Why

The use cases of `recipe-app` need a user, but no user can log in today. This change adds the Identity and Access commands and queries, the session store, the login throttle, and the HTTP pipeline that authenticates every request.

The change satisfies R10, R20, R21, R22, R28, R29, R30, R31, R35, R36, R37, R38, and R40 of `overview.md` section 7.

It uses these Assumptions of `overview.md` section 9: A9, A14, A17, A21, A22, A23, A24, A33, A42.

## What Changes

- `recipe-domain`: `IdentityError`, the password, token, and key value objects, the login throttle rule, and the login security events.
- `recipe-app`: three ports, the Argon2id hasher behind a semaphore, the 9 commands of `domain-model.md` section 5.3, and the 6 identity queries of section 5.5.
- `recipe-db`: the `sqlx` adapters of the three ports, the retry on `40001`, and the migration `0005` that protects `security_event`.
- `recipe-web`: the security pipeline of `api.md` sections 7 and 9 and the error mapping of section 5. Unstyled pages serve the 16 routes of sections 2.1 and 2.8 in `pl` and `en`.
- `recipe-server`: the subcommand `create-admin`.
- The deferred review items C6, S6, R6, and Q4 of P2.4.
- ADR-0035 records the decisions of the user on 2026-10-14.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity-access`: add the rules of ADR-0035, for example the throttle of `ChangeOwnPassword`.
- `audit`: add the guard of `security_event`.
- `operations`: change the first administrator and add the log rule of a storage error.
- `web-ui`: add the security headers and the HTMX configuration, and narrow the language redirect.

## Impact

- Code: every crate except `recipe-export`, `migrations/0005_security_event_guard.sql`, and `deploy/db/initdb/01-roles.sh`.
- Dependencies: `hmac`, `getrandom`, `subtle`, `cookie`, and the feature `sync` of `tokio` (design D3). `tower` is a test dependency.
- Docs: ADR-0035, `reliability.md`, `data-model.md`, `domain-model.md` section 5.3, `api.md` sections 2.8, 7, and 8, and `overview.md` section 5.

## Out of scope

- The subcommand `serve`, the startup settings comparison, the health listener, and the supervised jobs, also `cleanup` with `MaintenanceStore` (P4.1).
- The design system, the style sheet, the components, and the full text bundles (P3.1).
- The queries and pages of machines, recipes, audit entries, and security events (P3.2 to P3.4).
- The disk check that turns on the read-only mode (P4.1). This change adds only the gate.
