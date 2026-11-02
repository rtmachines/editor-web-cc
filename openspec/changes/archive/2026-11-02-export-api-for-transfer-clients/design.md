# Design

## Context

See `proposal.md`, section "Why". ADR-0109 holds the decisions. This design links to its items and does not repeat them.

The state of the code before the change:

| Area | State | Place |
|---|---|---|
| Export prefix | One constant `/export/v1/`. The error layer, the `HEAD` rule, the guard, and the timeout layer read it | `recipe-web/src/export_path.rs` |
| Navigation guard | A layer on each export row. It runs after the session layer and the gates | `recipe-web/src/export_guard.rs`, `routes.rs` |
| Session layer | `Access` gives `SessionNeed` for each route. `Authenticated` holds the session id, the session reference, the epoch, and the CSRF token | `recipe-web/src/session.rs`, `recipe-app/src/identity/authenticate.rs` |
| Login | Argon2 permit, T1 under the class-1 lock, hash in `spawn_blocking`, T2. A login outside the grammar counts as unknown | `recipe-app/src/identity/login.rs`, `hasher.rs`, `gate.rs` |
| Attempt port | `LoginAttemptStore::begin_attempt` runs T1 | `recipe-app/src/ports/attempt.rs` |
| Password reset | The port method `reset_password` always sets `must_change_password = true` | `recipe-app/src/ports/user.rs`, `recipe-db/src/transaction/identity/users.rs` (`RESET_PASSWORD`) |
| Export | `ExportVersion` with the export permit, rebuild, hash compare, audit. `DocumentWriter::write` writes v1 | `recipe-app/src/export/flow.rs`, `recipe-app/src/ports/document_writer.rs`, `recipe-domain/src/export/` |
| Document | v1 schema file, JSON writer, file name | `recipe-export/schema/export-v1.schema.json`, `recipe-export/src/` |
| Request data | `RequestMeta` holds an optional `session_ref` | `recipe-app/src/auth/user.rs` |
| Role | Three values. SQL `CHECK app_user_role_value` | `recipe-domain/src/identity/role.rs`, `data-model.md` section 3.5 |

The last migration is `0008_audit_log_insert_columns.sql`.

## Goals / Non-Goals

**Goals:**

- Give a transfer client the three routes of ADR-0109 (g) to (i) with HTTP Basic.
- Reuse the login throttle, the Argon2 semaphore, the security events, and the export flow. Add no table.
- Write the normative documents before the code that follows them.

**Non-Goals:**

- A token or a CLI token. ADR-0109 rejects them.
- Mutual TLS on the proxy.
- A change of the v1 document, the v1 schema file, or the two session export routes.
- A cache of verified credentials. Each request runs one Argon2id check.

## Decisions

### D1. Role and permission

`Role::ExportClient` maps to `{VersionExport}` (ADR-0109 (a), (b)). `Role::permissions()` stays a pure function. `Role` gains `is_person()`. `Login` and the Basic check call it.

Task 2.1 adds the variant to `recipe-domain` and fixes the compile only. The UI role lists keep the three person roles until task 6.9 adds the option to the user form. The set-role form never offers `ExportClient`.

Alternative: a new permission `RecipeTransfer`. It needs a change of the rule "Administrator holds every permission". Every role with `VersionExport` may already export an approved version (A4). The Basic check limits the routes to `ExportClient` (ADR-0109 (m)).

### D2. Basic check as a use case of the identity module

`recipe-app` gets `authenticate_basic(ctx, BasicCredentials, RequestMeta) -> Result<BasicPrincipal, AppError>`. It runs these steps:

1. Take the Basic permit (`export.basic_permits`), then the Argon2 permit. Both use `argon2.acquire_timeout`.
2. Run T1 of `Login` through `LoginAttemptStore::begin_attempt`. A throttled check returns `LoginThrottled`.
3. Read the user in a read-only transaction. A `TypedLogin` outside `Login` counts as unknown, as in `login.rs` (`read_user`).
4. Run the hash check in `spawn_blocking`. An unknown or inactive user gets the dummy hash (A42). The verifier uses the parameters of `argon2.m_cost`, `t_cost`, and `p_cost` in every case.
5. Refuse a person account after the hash check (`Role::is_person`).
6. Run T2 through the new port operation `LoginAttemptStore::confirm_basic(attempt, user, expected_hash)` (ADR-0109 (m)). T2 locks `app_user` `FOR NO KEY UPDATE`. It checks `is_active`, the role `export_client`, and `password_hash = expected_hash`. Then it sets `succeeded = true` and commits.
7. On a failure in steps 3 to 6, write the event through `SecurityEventSink`.

T2 returns `Confirmed` or `Changed { reason }`. `Changed` gives `InvalidCredentials` (`E-IDN-003`). T2 writes no audit entry, so it uses the port and not `UnitOfWork::commit`. It runs in read-only mode, like T1.

`BasicPrincipal { user: CurrentUser }` is the result (ADR-0109 (mb)). It holds no session value. The use case never builds an `Authenticated`. `RequestMeta` of a Basic request holds `session_ref = None`, so the audit entry of the export has no session reference.

`BasicCredentials` holds `TypedLogin` and the password. Its hand-written `Debug` redacts the password.

Alternative: run the Basic check inside `ExportVersion`. The list query would then repeat it, and the permission check would no longer be the first statement.

### D3. Header parser, route rows, and layers

`recipe-web` parses the header by the contract of ADR-0109 (ma). The parser checks the count of `Authorization` headers and the length before it decodes. It uses the standard Base64 alphabet with padding. It never logs the header. A parse fault is `WebError::MissingBasicCredentials` (`E-WEB-012`).

`Access` gets the variant `Basic(Operation)`. The session layer gives it `SessionNeed::Ignore` (ADR-0109 (l)). A new layer `basic_layer` runs D2 and stores `BasicPrincipal` as a request extension. A new extractor `BasicUser` reads it. A handler of a Basic row uses `BasicUser`, and a handler of a session row keeps its extractor of `Authenticated`.

The layer order of a Basic row is: navigation guard, `basic_layer`, handler (ADR-0109 (p)). The two session rows keep their order. The must-change-password gate skips a Basic row, because the `CHECK` of D6 keeps the flag false.

`EXPORT_PATH_PREFIX` becomes `/export/` (ADR-0109 (q)). The error layer gives a Basic row a `401` JSON body and the header of ADR-0109 (o). It never gives a Basic row the `303` of ADR-0053 (d).

### D4. List query

`ListExportableRecipes { machine: MachineCode }` checks `VersionExport` first. The use case wraps its read in `timeout.export` (ADR-0109 (g)). It reads in `READ COMMITTED READ ONLY` through a new port method `exportable_recipes(machine) -> Result<Option<ExportableRecipes>, StorageError>`.

The statement resolves the machine by ADR-0109 (j): the live machine with the code, else the machine with that code and the latest `archived_at`. It joins `recipe` and `recipe_version` with `status = 'approved'` and `deleted_at IS NULL`. It orders by `reference_number` with `COLLATE "C"`. No machine gives `None`, and the use case returns `NotFound` (`E-WEB-005`). `recipe-export` writes the JSON body.

The port trait, the result type, and the memory adapter come first (task 3.6). The PostgreSQL statement follows (task 4.3).

### D5. Scoped export

`ExportVersion` gets an input `ExportScope::{Version, Unit(UnitPath)}` and the machine filter of ADR-0109 (h) and (j). The flow keeps the export permit, the timeout, and the transaction of ADR-0056.

The read phase reads the version and the unit check of ADR-0109 (ja) before the blocking task. A failed unit check returns `NotFound` and starts no blocking task. The blocking task runs the four steps of ADR-0109 (w):

1. Rebuild the whole hashed body and compare it with the stored hash.
2. Filter `parameters[]` with `UnitPath::is_self_or_descendant_of`. This function compares segments, not text.
3. Build `types[]` of the filtered parameters with the closure function of the v1 builder.
4. Build `ScopeBody` in `recipe-domain/src/export/` and hash it with the same JCS function as `ExportBody`.

An empty filter result returns `ExportError::EmptyScope` (`E-EXP-005`). The port `DocumentWriter` gets the method `write_scoped(&self, body: &ScopeBody, facts: &ScopeFacts) -> Result<Vec<u8>, ExportError>`. `recipe-export` implements it with the v2 field order.

### D6. Database

Two migrations, so that the scan of the check never runs in the file that adds it:

- `0009_export_client_role.sql` drops `app_user_role_value` and adds it again with `export_client` as `NOT VALID`. It adds `app_user_export_client_password CHECK (role <> 'export_client' OR NOT must_change_password)` as `NOT VALID`.
- `0010_validate_export_client_checks.sql` runs `VALIDATE CONSTRAINT` on both constraints.

A `NOT VALID` constraint checks each new and each changed row at once. `ALTER TABLE ... ADD CONSTRAINT` takes an `ACCESS EXCLUSIVE` lock for a short time and scans nothing. `VALIDATE CONSTRAINT` takes `SHARE UPDATE EXCLUSIVE` and scans `app_user` (A19: fewer than 100 rows). No grant changes, so `grants.expected` does not change.

The port method `reset_password` gets the argument `must_change: bool`. `ResetPassword` passes `false` for `ExportClient` and `true` for every other role. Both adapters write the value. The statement `RESET_PASSWORD` binds it.

### D7. Audit image

`VersionExported.after_state` gets the optional keys `scope_unit_path` and `scope_sha256` (ADR-0109 (ab)). The schema version is `"2"` for a unit export. The entry of a Basic export holds no session reference. `data-model.md` section 10 gets the two keys.

### D8. Documents

Task group 1 applies these edits before the code.

| Document | Edit |
|---|---|
| `overview.md` | Section 2: the client pulls over HTTPS. Section 7: R03, R22, R28 mechanisms. Section 8: YAGNI row. Section 9: row A45 with the text of ADR-0109, "Technical account". Change the row A42: a Basic check of a person account also gets `E-IDN-003` |
| `api.md` | Section 1: prefix `/export/`. Section 2.7: three rows, the Basic rule, the archived machine. Section 2.8 rows `CreateUser` (line 223) and `ResetPassword` (line 228): no flag for `ExportClient`. Section 3: column `ExportClient`, row `ListExportableRecipes`. Section 5.1: `E-EXP-005` in the 404 family, `E-WEB-012` in the 401 family, `401` on Basic rows. Section 6: subsection 6.4 for v2. Section 7: Basic check, T2, parser |
| `reliability.md` | Section 1.2: rows `E-EXP-005`, `E-IDN-011`, `E-WEB-012`. The note of `E-IDN-003` names the Basic check. Section 7.11: rows `export.basic_permits` = 1 and `export.basic_header_max` = 1024 bytes |
| `domain-model.md` | Section 2.3: role `ExportClient`. Section 2.4 (line 442): the actor of `VersionExported` can be an `ExportClient` account. Section 5: the commit-path row of T2 of the Basic check, without an audit entry. Section 5.3 lines 632 and 636: no flag for `ExportClient`, and `SetUserRole` with `E-IDN-011`. Sections 5.4 and 5.5: `ExportVersion`, `ListExportableRecipes`. Section 8: copy of A45 |
| `data-model.md` | Section 3.5: two `CHECK` constraints. Section 6: rows of `CreateUser` (line 1975), `ResetPassword` (line 1979), `ExportVersion`, `ListExportableRecipes`, and T2 of the Basic check. Section 10: audit keys |
| `docs/ops/` | Runbook of a transfer client: account, rotation, TLS trust, port 443, preemptive header, polling rule, trust of the document values (ADR-0109 (ag), (ah)) |
| `.claude/rules/web.md`, `.claude/rules/sql.md`, `CLAUDE.md` | Prefix `/export/`. T1 and T2 of the Basic check as an exception to one transaction per command. These files are rules of the project, so the user approves the edit first |

Text of the three new codes in `reliability.md` section 1.2:

| Code | Variant | HTTP | Message |
|---|---|---|---|
| `E-EXP-005` | `ExportError::EmptyScope { reference, number, path }` | 404 | Version {number} of recipe {reference} has no parameter in unit {path} or below it. |
| `E-IDN-011` | `IdentityError::TechnicalRoleChange { login }` | 409 | The role of {login} cannot change to or from ExportClient. Create a new user. |
| `E-WEB-012` | `WebError::MissingBasicCredentials` | 401 | The request has no valid Basic credentials. Send the login and the password of a transfer account. |

No variant holds a password, a hash, or a header value.

## Risks / Trade-offs

- [The v1 tests fix the prefix `/export/v1/`] → Task 6.1 changes the constant first. The existing guard, `HEAD`, and timeout tests must pass with the new prefix.
- [A client library waits for a `Basic` challenge] → The runbook names the preemptive header. ADR-0109 (o) keeps the browser safe.
- [One Basic permit makes a busy client wait] → The client polls the list once per interval. A `503` carries `Retry-After`.
- [Other verifier work on a refusal shows an account] → Task 3.4 counts the verifier calls and compares their parameters. It does not rely on time.
- [A reset or a deactivation during the hash check] → T2 of D2 locks the user row. Tasks 3.5 and 4.4 test both races.
- [The guard writes a `csrf_rejected` row for a cross-site request without credentials] → The kind aggregates per address, kind, and minute (`data-model.md` section 3.8).
- [A reused machine code hides the older archived machine] → The session routes still export its recipes (ADR-0109 (j)).
- [A fourth role breaks a `match` on `Role`] → The compiler finds each `match`. Task 2.1 adds the variant first.

## Migration Plan

1. Deploy the release. `migrate` applies `0009` and `0010` before `app` starts (`reliability.md` section 7.10).
2. The Administrator creates an `ExportClient` account and gives the password to the client operator.
3. The operator configures the client with `https://`, the plant CA, and the account.

Rollback: the previous image cannot parse the role value `export_client` in any row that it reads. A deactivation of the account does not help, because the row stays. Roll back by a restore of the backup from before the upgrade (`reliability.md` section 7.10). Another way is a later release that reads the value.

## Open Questions

- The client gets the JSON message in `pl`, because it sends no `recipe_lang` cookie. The code stays stable. The owner can ask for `en` later without a spec change of the codes.
