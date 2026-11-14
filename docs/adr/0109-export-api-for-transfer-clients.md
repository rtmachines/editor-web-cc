# ADR-0109: Export API for transfer clients with HTTP Basic

Status: Accepted. Amended by ADR-0119 (an `Approved` version is loadable only when it breaks no current rule). Amended by ADR-0164 (item (ah): a second pick of the approved version after a concurrent approval)
Date: 2026-11-02
Deciders: project owner

## Context

A transfer client is an external service, for example a PLC adapter for a Siemens S7-300. It must pull the `Approved` version of a recipe over HTTP with a checksum. A person does not start the transfer.

The two export routes of `api.md` section 2.7 need a session cookie (ADR-0014). A session comes only from the login form. A program cannot use it without a browser.

The earlier plan expected API tokens. ADR-0002 named "API tokens for a transfer system" as a later change. ADR-0019 names "Per-system API tokens for a transfer system" as a later step. `overview.md` section 8 has the YAGNI row "No API tokens until a transfer system exists". A transfer system now exists.

The project owner made four decisions on 2026-11-02:

1. The machine scope is one recipe with all its units, because a recipe belongs to exactly one machine (A10). A route lists the recipes of a machine with an `Approved` version. A second route returns the full v1 document of one of these recipes.
2. The client authenticates with HTTP Basic: the login and the password of a technical account. Only the export routes for the client accept it. The UI keeps the session cookie.
3. A unit scope route returns the parameters of one unit and of its descendant units. It carries `scope_sha256` and the `content_sha256` of the whole version.
4. The format is the export document of `api.md` section 6. This ADR states whether the unit route breaks v1 under ADR-0019.

## Decision drivers

- A program pulls an approved recipe without a browser and without a person (R03, A2).
- Only an `Approved` version is loadable, and the client detects a changed file (R18, R31).
- Each export writes one audit entry with the actor (R20, A8).
- The client account cannot change data and cannot open the UI (R24, R28).
- The password check must not block the logins of the UI (R30).
- No browser keeps the client credentials for a later request (R39, ADR-0055).

## Considered options

### Option A: Per-system API token

The Administrator creates a random token in the UI. The database stores its SHA-256. The client sends `Authorization: Bearer <token>`.

- Pros: a check costs one SHA-256 and one row read. No Argon2 call on each request. The token never reaches a login form.
- Cons: a new table, a new UI page, a new rotation procedure, and a new secret type in the redaction rules.

### Option B: Token from a CLI subcommand

The operator runs `recipe-server issue-token` on the host. The subcommand writes the token hash to the database.

- Pros: no new UI page.
- Cons: a shell on the host for each rotation. The same table and the same check as Option A. The Administrator of the UI cannot revoke a token.

### Option C: HTTP Basic of a technical account

A user with the new role `ExportClient` holds a password. The client sends `Authorization: Basic` with the login and the password on each request.

- Pros: the account uses `app_user`, the password policy, `ResetPassword`, `DeactivateUser`, and the login throttle. No new secret type. No new table.
- Cons: each request runs one Argon2id check. A client that sends the password over plain HTTP exposes it.

## Decision

The project uses **Option C**. This is the decision of the project owner. The project owner rejects Options A and B for this release.

Technical account:

The row A45 of `overview.md` section 9 gets this text: "A technical account with the role `ExportClient` exports `Approved` versions for a transfer client with HTTP Basic. It holds only `VersionExport`. It cannot log in to the UI, and no person uses it. The three roles of R22 stay the roles of persons". The source column is ADR-0109.

- (a) The role `ExportClient` is a fourth value of `Role`. The SQL value is `export_client`. It is the role of a technical account, not of a person. The three roles of R22 stay the roles of persons (A45).
- (b) `ExportClient` holds one permission: `VersionExport`. It holds no `RecipeView`. The other roles keep their permissions.
- (c) `Login` refuses an `ExportClient` account after the full password check. The answer is `401 E-IDN-003` with the body and the timing of A42. The `security_event` row of kind `login_failed` holds the reason `technical_account` in `detail`.
- (d) `CreateUser` with the role `ExportClient` generates the password and shows it once, as for every role. It does not set `must_change_password`. The generated password is the credential of the client.
- (e) `ResetPassword` of an `ExportClient` account generates a new password and does not set `must_change_password`. This is the rotation of the credential. A `CHECK` on `app_user` refuses `must_change_password = true` for the role `export_client`.
- (f) `SetUserRole` refuses a change to `ExportClient` and a change from `ExportClient` with `409 E-IDN-011`. A person and a client never share one account. The last administrator rule does not change (ADR-0096).

Routes:

- (g) `GET /export/v1/machines/{code}/recipes` runs the query `ListExportableRecipes` with `VersionExport`. The JSON body names the machine and lists each recipe of the machine with an `Approved` version.
  - The object `machine` holds `code`, `plc_platform`, and `archived`.
  - Each row holds `reference_number`, `name`, `version_number`, `approved_at`, and `content_sha256`. The rows come in the byte order of the reference number.
  - The list has no pages. A19 limits the count of recipes.
  - The query runs inside `timeout.export`. A query that passes the limit gives `503 E-WEB-009`. ADR-0072 removes the request timeout from the export paths, so this limit is the time bound of the list (R30).
- (h) `GET /export/v1/machines/{code}/recipes/{ref}/approved` runs `ExportVersion` with `VersionExport`. It returns the v1 document of `api.md` section 6 without a change. A recipe of another machine gives `404 E-WEB-005`.
- (i) `GET /export/v2/machines/{code}/recipes/{ref}/approved/units/{path}` runs `ExportVersion` with a unit scope and `VersionExport`. It returns the v2 document of items (s) to (aa).
- (j) `{code}` names the live machine with that code. When no live machine has the code, `{code}` names the machine with that code that the system archived last. Recipes of an archived machine stay exportable (`domain-model.md` section 1). The project owner decided this on 2026-11-02.
  - A reused code hides the older archived machine from the three routes. The session routes of item (k) still export its recipes.
- (ja) `{path}` follows the rule of `UnitPath` in `api.md` section 1. A path that names no unit of the machine gives `404 E-WEB-005`. On a live machine, an archived unit counts as no unit. On an archived machine, every unit of the machine counts.
- (k) The two routes of `api.md` section 2.7 do not change.

Authentication:

- (l) Each export route has one scheme. The three routes of items (g) to (i) accept only HTTP Basic, and they never read the session cookie. The two routes of item (k) accept only the session cookie, and they never read `Authorization`. A request with both credentials therefore uses the scheme of its route.
- (m) The Basic check accepts only an active `ExportClient` account. The rules of the check follow:
  - An unknown login, an inactive account, an account of a person, and a wrong password give `401 E-IDN-003`. The body, the status, and the hash work are the same (A42).
  - A login that is a `TypedLogin` but not a `Login` counts as unknown. The check runs the dummy hash, as `Login` does.
  - The check takes the export Basic permit, then the Argon2 permit, then runs T1 of `Login` (ADR-0014, ADR-0035). The hash check runs in `spawn_blocking`.
  - After the hash check, T2 of the Basic check locks the `app_user` row `FOR NO KEY UPDATE`. It checks again that the account is active, has the role `ExportClient`, and has the same password hash.
  - When T2 finds no change, the port `LoginAttemptStore` sets `succeeded = true` in T2. T2 writes no audit entry and creates no session. It runs also in read-only mode.
  - When T2 finds a change, the check fails with `401 E-IDN-003`. A password reset or a deactivation during the hash check therefore fails the request.
  - On failure, the check writes the `security_event` of `Login` through `SecurityEventSink` (A33). `detail` holds the reason and the scheme `basic`.
  - A throttled check gives `429 E-IDN-005` with `Retry-After`. A permit timeout gives `503 E-WEB-009` with `Retry-After`.
  - The Basic check is not a command. Its T1 and T2 are an exception to the rule of one transaction per command, as T1 and T2 of `Login` are (ADR-0035 (b)).
- (ma) The parser of the header refuses these cases with `401 E-WEB-012` and runs no hash:
  - more than one `Authorization` header, or a header longer than `export.basic_header_max`. The parser checks the length before it decodes,
  - a scheme other than `Basic`. The parser compares the scheme without a difference between upper and lower case,
  - text that is not standard Base64 with padding, or decoded bytes that are not UTF-8,
  - decoded text without `:`. The parser splits the text at the first `:`, so the password can hold `:`,
  - a login outside `TypedLogin` (1 to 64 characters), a password outside 1 to 128 characters, or a control character in either part.
- (mb) A successful check gives a `BasicPrincipal` that holds the `CurrentUser` of the account. It holds no session id, no session reference, no epoch, and no CSRF token. The `RequestMeta` of the request holds no session reference.
- (n) At most `export.basic_permits` Basic checks wait for or hold an Argon2 permit at the same time. The value is 1 (`reliability.md` section 7.11). The UI logins therefore keep 3 of the 4 Argon2 permits.
- (o) A `401` answer of a Basic route has the header `WWW-Authenticate: RecipeExport realm="export"`. A browser does not know the scheme `RecipeExport`. It shows no password dialog and keeps no credentials. The client sends `Authorization: Basic` on the first request without a challenge. The answer keeps the header, because RFC 7235 section 3.1 requires `WWW-Authenticate` on a `401` and the project owner refused only a `Basic` challenge.
- (p) On a Basic route, the navigation guard of ADR-0055 runs first. The Basic check runs after the guard and before the use case. A rejected cross-site request therefore runs no hash.

Error answers:

- (q) The rules of ADR-0053 (a), ADR-0055 (a), and ADR-0072 (a) apply to every path under `/export/`. That prefix holds `/export/v1/` and `/export/v2/`.
- (r) A Basic route never answers `303`. A failed check gives a `401` JSON body of ADR-0053 (a). `message` uses the UI language of the request, and the default is `pl` (`api.md` section 8).

Unit scope document:

- (s) The v2 document holds every field of the v1 document and two more fields: `scope` and `scope_sha256`. `scope` is the object `{"unit_path": <path>}`.
- (t) `parameters[]` holds each parameter of the version whose `unit_path` equals `{path}` or starts with `{path}` and a `.`. `types[]` holds the types that these parameters use, by the rule of `api.md` section 6.1. The array orders of `api.md` section 6.3 apply.
- (u) `content_sha256` is the stored hash of the whole version. `scope_sha256` is the lower-case hex SHA-256 of the RFC 8785 (JCS) form of the scope hashed body.
- (v) The scope hashed body holds `machine.code`, `machine.plc_platform`, `recipe.reference_number`, `version.number`, `scope.unit_path`, `types[]`, and `parameters[]` without `name_en`. The fields follow the rules of `api.md` section 6.1.
- (w) The read phase of `ExportVersion` reads the unit check of item (ja) before the blocking task. The blocking task then runs four steps in this order:
  1. It rebuilds the hashed body of the whole version and compares its hash with the stored value. A difference gives `500 E-EXP-002`.
  2. It filters the parameters of item (t).
  3. It builds `types[]` of the filtered parameters.
  4. It builds the scope hashed body and computes `scope_sha256`. The database does not store `scope_sha256`.
- (x) A unit of item (ja) without a parameter in the scope gives `404 E-EXP-005`. The client never gets an empty load.
- (y) The answer has `Content-Disposition: attachment; filename="recipe_{ref}_v{n}_{path}.json"` and `Cache-Control: no-store`.

Version of the format:

- (z) The unit document breaks v1 under ADR-0019 for two reasons. First, the v1 schema file refuses an extra field at the top level (`additionalProperties: false`). Second, `api.md` section 6.3 tells a consumer to build the hashed body from the document and to compare it with `content_sha256`. A subset of the parameters fails that check.
- (aa) The unit document therefore is the export document v2. `schema_version` is `"2"`, the route prefix is `/export/v2/`, and the schema file is `crates/recipe-export/schema/export-v2.schema.json`. A v2 consumer compares its own hash with `scope_sha256`. The v1 routes and the v1 document stay.

Audit and read-only mode:

- (ab) Each export of items (h) and (i) writes one `VersionExported` entry (A8). The actor is the `ExportClient` account, and `exported_by` holds its login. A unit export adds `scope_unit_path` and `scope_sha256` to the entry. `ListExportableRecipes` writes no audit entry.
- (ac) In read-only mode, the three routes stay available (A24). The Basic check keeps its `login_attempt` rows and its `security_event` rows, as `Login` does.

Transport:

- (ad) Caddy terminates TLS for the client (A18). The client reaches the public listener on TCP port 443 and uses `https://`.
- (ae) The HTTP site of Caddy answers each path under `/export/` with `403` and no redirect. This item amends ADR-0065 (d). A client with an `http://` address therefore fails on the first request.
  - The runbook tells the operator to rotate a password that went over plain HTTP.
  - The Caddyfile has no `log_credentials` option, so the access log of Caddy holds no `Authorization` value.
- (af) The operator gives the client the certificate of the plant CA and permits TCP port 443 from the client host. A device in front of Caddy follows ADR-0065 (m).
- (ag) The client reads the list of item (g) and compares `content_sha256` with its last value. It runs an export only after a change. The runbook states this rule, because each export writes one audit entry.
- (ah) An approval can occur between the read of the list and the export. The client therefore trusts `version.number` and `content_sha256` of the document, not the values of the list row. The runbook states this rule.

## Consequences

### Positive

- A transfer client pulls an approved recipe or one unit of it with no browser and no person.
- The account uses the existing password, throttle, deactivation, and audit rules.
- The client checks `content_sha256` or `scope_sha256` before it writes to a PLC.

### Negative

- Each request of a client costs one Argon2id check, about 19 MiB and some tens of milliseconds.
- A client that does not send `Authorization: Basic` without a challenge cannot connect. Some HTTP libraries wait for a challenge.
- A fourth role value needs a migration of the `CHECK` on `app_user.role` and a new column of the permission matrix.
- The project keeps two schema files and two consumer rules.

### Risks and mitigations

- Risk: a client polls the export route and fills the audit log. Mitigation: item (ag) and the list route, which writes no audit entry.
- Risk: a client sends the password over plain HTTP. Mitigation: item (ae) and the rotation through `ResetPassword`.
- Risk: a flood of Basic checks blocks the UI logins. Mitigation: item (n) and the per-IP budget of the login throttle.
- Risk: a later change lets a person log in with an `ExportClient` account. Mitigation: a test of item (c) and a test of each human role on a Basic route.
- Risk: the guard of item (p) runs before the Basic check, so a cross-site request without credentials now writes a `csrf_rejected` event. Mitigation: the kind `csrf_rejected` aggregates per client address, kind, and minute (`data-model.md` section 3.8). A flood adds at most one row per minute and address.
- Risk: a reset or a deactivation during the hash check. Mitigation: T2 of item (m) locks the user row and checks the state again.

## Related

- Amends ADR-0013 (fourth role) and ADR-0014 (second scheme for the technical account).
- Amends ADR-0019 (Basic in place of the planned tokens, document v2 for the unit scope).
- Amends ADR-0053 (a) and (d), ADR-0055 (a) and (e), and ADR-0072 (a) (prefix `/export/`, order of the guard, `401` on the Basic routes).
- Amends ADR-0065 (d) (no redirect of the paths under `/export/`).
- ADR-0019 superseded ADR-0002, and ADR-0014 superseded ADR-0004. This ADR ends their token plan.
- ADR-0035, ADR-0052, ADR-0054, ADR-0056, ADR-0065, ADR-0096.
- `api.md` sections 1, 2.7, 3, 5, 6, 7. `reliability.md` sections 1.2 and 7.11. `domain-model.md` sections 2.3, 5.3, 5.4, 5.5. `data-model.md` sections 3.5, 6, 10.
- `overview.md` section 7: R03, R18, R20, R22, R24, R28, R30, R31. Section 8: YAGNI. Section 9: A2, A8, A10, A18, A19, A24, A33, A42, A45.
