# Design

## Context

- An external adapter loads approved recipes into the PLCs. It pulls the documents with HTTP Basic of an `ExportClient` account (ADR-0109) and the connection record (ADR-0154 (j), (jc)).
- R03 says "No machine communication". A47 reads it as "no crate holds a PLC protocol client". ADR-0019 says that the transfer is out of scope.
- `api.md` §1 permits JSON only under `/export/`. `web.md` permits only form bodies on a mutating route and requires the CSRF token and the `Origin` check.
- The Basic check runs one Argon2id hash on each request (`api.md` §7.7). `export.basic_permits` is 1 of the 4 `argon2.permits`. The UI logins keep the other 3.
- A24 blocks every command in read-only mode except the commands that it names.
- `heartbeat` already names a supervised job (`reliability.md` §2.4). This design therefore uses the term "status report" for the message of the adapter.
- "Command" names a use case in this project. The entity of a load is therefore a `LoadRequest`, not a command.

The owner decided these points on 2026-11-25, in the first round:

1. The adapter reports the machine state and receives load requests. The editor opens no connection.
2. `machine-link` is the eighth capability.
3. HTTP Basic on each request, with an own permit pool. `ExportClient` gets the new permission. There is no new role.
4. In read-only mode, the editor accepts status reports and results. New load requests are refused.
5. There is no confirmation on the HMI. The PLC interlock is the final authority.
6. This change is planning only until `value-body-memory-load` and change B end.

The owner decided these points on 2026-11-25, after the first plan review:

7. A load request covers the whole version only (Q1).
8. A request with an unknown result blocks the machine. A status report renews the claim, and a person releases the request after a check of the machine (Q2).
9. Every person role holds `RecipeLoadRequest`. ADR-0224 amends ADR-0013 (Q3).
10. The values of D11 are accepted (Q4).

The owner decided these points on 2026-11-25, after the second plan review:

11. The `loadable` check runs only in `RequestRecipeLoad`, before its transaction. The claim does not run it (Q1).
12. A machine with the string encoding `Utf8` gets no load in this change. A later change adds a whole-version v2 path (Q2).
13. The configuration of the adapter binds each machine code to its PLC. Risks and the how-to cover a reused code (Q3).
14. The adapter routes give `404 E-LNK-004` for a bad `{request_id}`. The UI routes give `404 E-WEB-005` (Q4).
15. The layer stack of `routes.rs` stays. A claim in read-only mode gets `503 E-WEB-006` before the Basic check (Q5).
16. A rollback of the image only is permitted only while `audit_log` holds 0 entries of the seven new actions (Q6, as ADR-0202 (q)).

## Goals / Non-Goals

**Goals:**

- Show the state of each machine, the recipe in its PLC, and the age of that information.
- Let a person ask for the load of an approved version, with an audit entry for each step.
- Keep R03 true in the reading of A55: no crate holds a protocol client, and the editor opens no connection.
- Keep the export documents, `content_sha256`, and the export routes unchanged.

**Non-Goals:**

- The adapter program, the PLC handshake, and the interlock. `api.md` §10 states the duties of the adapter only.
- A history of the reported states, trends, or alarms.
- A push channel from the editor (SSE, WebSocket, a broker).
- A binding of an account to some machines.
- A load of one unit or of a part of a version.
- A load for a machine with the string encoding `Utf8`. Its v1 document always has `loadable = false` (ADR-0202 (l), decision 12).

## Decisions

### D1. Direction and reading of R03 (decided)

The adapter starts every connection over HTTPS through Caddy (A18, ADR-0109 (ad) to (af)). The editor never opens a connection to an adapter or to a PLC. No crate holds a PLC protocol client.

Assumption A55 records this reading: "An external adapter sends the state of each machine and claims the load requests of the editor over HTTPS. The adapter starts every connection. The PLC interlock decides each load." The row of R03 in `overview.md` §7 cites A55. ADR-0224 amends ADR-0019 (the transfer is a duty of the adapter, not of the editor). A2 stays true.

The HTTP site of Caddy answers each path under `/adapter/` with `403` and no redirect, as for `/export/`. A client with an `http://` address thus fails on its first request and sends no password in clear text again. This amends ADR-0065 (d) and ADR-0109 (ae).

Rejected:

- A message broker (MQTT, Sparkplug B). It adds a stateful service on the one host, with its own backup, ACL, and TLS. It helps only with several consumers or several lines.
- A connection from the editor to the adapter (gRPC, HTTP). It breaks A47 and needs a route from L3 into the OT zone.
- Long polling. Each held request keeps a connection and a Basic permit cycle. The answer of the status report gives the same result with no held connection.

### D2. Account and permissions (decided)

- `Permission::MachineLinkReport` is new. `ExportClient` holds it. Administrator holds it too, because Administrator holds every permission (R26). Over HTTP, only an `ExportClient` account reaches the adapter routes, because the Basic check refuses a person account. `ReportMachineState`, `ClaimLoadRequest`, and `ReportLoadResult` check it first.
- `ExportClient` holds `VersionExport` and `MachineLinkReport`. This amends ADR-0109 (b). One account per adapter serves both the transfer routes and the adapter routes.
- `Permission::RecipeLoadRequest` is new. Administrator, ProcessEngineer, and Operator hold it (decision 9). `ExportClient` does not hold it. `RequestRecipeLoad`, `CancelRecipeLoad`, and `ReleaseRecipeLoad` check it first.
- A load changes no recipe data, so R24 stays true for the Operator. ADR-0224 amends the fixed permission list of the Operator in ADR-0013.
- `GetMachineLink` is a query with `RecipeView`.

Rejected: a new role `AdapterClient`. It changes about fifteen requirements of the role rules, the login, the password reset, the role change, and the user forms. The risk that it removes is small, because an `ExportClient` account already reads the PLC passwords.

### D3. Basic check and its permits (decided)

The adapter routes use the Basic check of `api.md` §7.7 without change, except for the permit. A Basic check of a route under `/adapter/v1/` takes one of `adapter.basic_permits` permits (1). A Basic check of a transfer route keeps `export.basic_permits` (1). `argon2.permits` stays 4, and the UI logins keep 2 of them. This amends ADR-0109 (n).

One status report holds the entries of many machines (D4). One adapter therefore runs one hash per `machine_link.report_interval`, not one hash per machine.

Rejected: a token from a Basic exchange. It adds a secret type, a table, and a rotation rule. The batch report keeps the hash rate low without it.

### D4. Adapter routes and the JSON contract (decided)

| Method | Path | Use case | Answer |
|---|---|---|---|
| POST | `/adapter/v1/reports` | `ReportMachineState` | `200` with one result per entry |
| POST | `/adapter/v1/load-requests/{request_id}/claim` | `ClaimLoadRequest` | `200` with the claimed request |
| POST | `/adapter/v1/load-requests/{request_id}/result` | `ReportLoadResult` | `200` with the stored outcome |

The routes use the layer stack of `routes.rs` without a new order (decision 15). A request under `/adapter/v1/` passes these layers in this order:

1. The body limit layer gives `413 E-WEB-003` to an announced body above `http.body_limit.adapter`. The adapter routes get their rows in `route_limit`.
2. The access layer reads the route class `Access::Adapter(Operation)`. The session layer reads no cookie on such a route.
3. The CSRF layer and the `Origin` check skip each route of the class `Access::Adapter`. They do not compare the path text.
4. The gate layer applies the read-only gate of A24 (D9). It refuses the claim with `503 E-WEB-006` before any hash.
5. The navigation guard of ADR-0055 refuses a `Sec-Fetch-Site` other than `same-origin` or `none`, and `HX-Request: true`. It gives `403 E-WEB-007` and a `csrf_rejected` event. This amends ADR-0055 (a).
6. The Basic check gives `401` with `WWW-Authenticate: RecipeAdapter realm="adapter"`, `429 E-IDN-005`, or `503 E-WEB-009` when it gets no permit. This amends ADR-0109 (o).
7. The handler reads the body with the JSON extractor. A chunked body above `http.body_limit.adapter` gives `413 E-WEB-003` here. The use case checks the permission first.

Rules of the route class:

- A test lists every route and proves that each person route still needs the token. This amends ADR-0035 and `api.md` §7.3.
- The test also sends `POST /adapter/v1/../users/mhoffman/deactivate` and its encoded forms. None of them reaches a handler without the token.
- Every layer above answers a JSON error body, also the timeout and the A24 gate. Only the backstop panic layer keeps its `text/plain` answer (ADR-0057). This amends ADR-0053 (a).
- The request timeout of ADR-0058 applies, with a JSON answer `503 E-WEB-009`. The repeated claim and the repeated result are idempotent, so a lost answer is safe. This amends ADR-0058 (g).
- The routes read no cookie and need no CSRF token and no `Origin` header. An answer sets no `HX-*` header (an exception to `api.md` §5.3).
- `{request_id}` is a `LoadRequestId`. On an adapter route, a malformed or unknown value gives `404 E-LNK-004`. On a UI route, it gives `404 E-WEB-005` (decision 14).

JSON error body (amendment of ADR-0053 (a)):

```json
{ "code": "E-LNK-009", "message": "…", "request_id": "01J…", "path": "$" }
```

```json
{ "code": "E-LNK-009", "message": "…", "request_id": "01J…", "path": "$.machines[0].state" }
```

- `path` is present only with `E-LNK-009`. Every other code has the three fields of ADR-0053 (a).
- The adapter routes read no cookie. `message` therefore uses the default language `pl` (R10). This amends ADR-0053 (b), `api.md` §8, and the web-ui requirement "Language cookie value".
- A `401` has `WWW-Authenticate: RecipeAdapter realm="adapter"`. A `429 E-IDN-005` has `Retry-After` with the delay of the login throttle. A `503` has `Retry-After` with `http.retry_after`.

Rules of the body:

- The media type is `application/json`, with no parameter or with `charset=utf-8`. Another media type or a JSON syntax error gives `422 E-LNK-009` with the path `$`.
- The structs use `deny_unknown_fields`. A missing field or a value outside its grammar gives `422 E-LNK-009` with the path of the field, for example `$.machines[0].state`.
- An unknown field gives `422 E-LNK-009` with the path of its parent object. An error path names only paths of the schema, never a name that the client sent.
- The claim body is the empty object `{}`.
- Each `content_sha256` of a request body is 64 lower-case hexadecimal digits (`api.md` §6.3). Another length or character gives `422 E-LNK-009`.
- The JSON Schema files are `crates/recipe-web/schema/adapter-v1-*.schema.json`. Golden tests pin each request and each answer.

Status report body:

```json
{
  "run_id": "6f1c…",
  "seq": 1042,
  "machines": [
    {
      "code": "M12",
      "state": "ready_for_changeover",
      "loaded": { "reference_number": "R-100", "version": 3, "content_sha256": "ab12…" },
      "claim": null
    }
  ]
}
```

- `run_id` is a UUID. The adapter makes a new one at each start. `seq` rises by one per sent report, from 0 to `i64::MAX`. A resent report also takes the next `seq`.
- `state` is one of `ready_for_changeover`, `running`, `stopped`, `fault`. `loaded` is `null` when the adapter does not know the loaded recipe.
- `claim` is the `LoadRequestId` that the adapter executes on this machine, or `null`. It renews the claim (D6).
- An entry list holds 1 to `machine_link.report_max_machines` entries, each code once. An empty list, a longer list, or a repeated code gives `422 E-LNK-010`.
- `reference_number` parses into its value object. `version` is an integer from 1 to `i32::MAX`. A bad value gives `422 E-LNK-009`.

Entry results:

- An unknown code gives `E-PLT-019`. An archived machine gives `E-PLT-010`.
- The same `run_id` and the same account with a `seq` that is not higher than the stored one gives `E-LNK-012`.
- Another `run_id` or another account gives `E-LNK-016` while the stored row is fresh (D5).
- An entry with an entry result changes nothing. The other entries apply.
- An entry of a known live machine gets `claim_lost` also when its entry result is `E-LNK-012` or `E-LNK-016`. The server reads it and writes nothing. An old run thus learns that it lost its claim while the row of a new run is fresh.
- The result of an applied entry holds `pending`, `claimed`, and `claim_lost`. `pending` is the request of the machine in the effective state `pending`, or `null`.
- `claimed` is the request of the machine in the stored state `claimed` by this account, or `null`. This includes the effective state `unknown`. A restarted adapter thus finds its open claim with its first applied report.
- `claim_lost` holds the id of `claim` when that id names no request of this machine in the stored state `claimed` by this account. Otherwise it is `null`.
- `claim_lost` thus covers a released request, a request that a restore removed, and a request of another account. The lease does not change.
- An entry with `E-PLT-019` or `E-PLT-010` holds no `claim_lost`, because the machine is unknown or archived.

The fields `pending` and `claimed`, and the answer of the claim route, hold `request_id`, `machine_code`, `reference_number`, `version`, `content_sha256`, and `document`. `document` is the path of the v1 document, `/export/v1/machines/{code}/recipes/{ref}/approved` (ADR-0109 (h)). This route accepts HTTP Basic (`api.md` §2.7). The claim answer also holds `result_deadline`. No answer holds a parameter value or a secret.

### D5. Link state and freshness (decided)

- Table `machine_link_state`, one row per machine. `ReportMachineState` is an adapter report, not a command. It writes through the new port `MachineLinkStore::record` in its own short transaction, with no audit entry.
- `MachineLinkStore` is a fifth write port next to the four write ports of ADR-0015. It is a store write like `LoginAttemptStore`, not a `UnitOfWork` commit. ADR-0224 amends ADR-0015.
- The link state is operational state, not a change of plant or recipe data. R20 therefore needs no audit entry for it. I13 names the report as a case without an entry (D7). `domain-model.md` §5 gets the port row.

| Column | Type | Null | Rule |
|---|---|---|---|
| `machine_id` | `UUID` primary key, foreign key to `machine` | no | Frozen. No `id` column (exception of `data-model.md` §1.1) |
| `state` | `TEXT` `CHECK` of the four states | no | Each report |
| `run_id` | `UUID` | no | Each report |
| `seq` | `BIGINT` `CHECK (seq >= 0)` | no | Each report |
| `loaded_reference`, `loaded_version`, `loaded_sha256` | `TEXT`, `INTEGER`, `TEXT` | yes | Each report. A `CHECK` requires all three or none. `loaded_reference` has the pattern of `recipe_reference_format`, `loaded_version` a `CHECK (loaded_version >= 1)`, and `loaded_sha256` the grammar of D4 |
| `reported_by` | `UUID` foreign key to `app_user`, with an index | no | Each report |
| `received_at` | `TIMESTAMPTZ` | no | Each report. It also serves as `updated_at` (exception of `data-model.md` §1.1) |

- `recipe_app` gets `INSERT` and `SELECT`, and `UPDATE` on every column except `machine_id`. It gets no `DELETE`.
- A report writes `received_at = GREATEST(received_at, now())`. A report that waits for a lock thus never moves the time back.
- A report of the stored `run_id` and the stored account replaces the row only with a higher `seq`.
- A report of another `run_id` or another account replaces the row only when the stored row is stale. A late report of an old run, or a report of a second adapter, therefore cannot overwrite a fresh row.
- A row is stale when `now()` is equal to or later than `received_at + machine_link.stale_after`. Otherwise it is fresh. The run rule, the effective link state, and `E-LNK-001` use this one rule.
- After a restart, an adapter waits up to `stale_after` plus one `report_interval` until its first report applies. `api.md` §10 states this.
- The effective link state is `no_link` when there is no row or the row is stale. Otherwise it is the reported state. Only queries compute it. No job writes it.
- The panel compares `loaded_sha256` with the stored hash of the approved version of that recipe. It shows "approved version", "other version", or "unknown recipe".

Rejected: a store in the memory of the process. It needs a second source of truth and loses the state at each restart. The row write per report is small. It changes no indexed column, so PostgreSQL can make it a HOT update.

### D6. Lifecycle of a load request (decided)

A load request covers the whole approved version (decision 7). The adapter uses the v1 document of ADR-0109 (h), never a v2 unit document.

A v1 document of a machine with the string encoding `Utf8` always has `loadable = false` (ADR-0202 (l)). Such a machine therefore gets no load in this change (decision 12). The pre-check of D7 refuses it with `E-LNK-015`.

Stored states: `pending`, `claimed`, `succeeded`, `failed`, `cancelled`, `expired`, `released`.

| From | To | By |
|---|---|---|
| (new) | `pending` | `RequestRecipeLoad` |
| `pending` | `claimed` | `ClaimLoadRequest`, before `claim_deadline` |
| `pending` | `cancelled` | `CancelRecipeLoad`, before `claim_deadline` |
| `pending` | `cancelled` | `ArchiveMachine` of the machine, before `claim_deadline` |
| `pending` | `expired` | `RequestRecipeLoad` or `ArchiveMachine` of the machine, after `claim_deadline` |
| `claimed` | `claimed` with a new `result_deadline` | a status report that names the claim |
| `claimed` | `succeeded`, `failed` | `ReportLoadResult` |
| `claimed` | `released` | `ReleaseRecipeLoad`, after `result_deadline` |
| `released` | `succeeded`, `failed` | a late `ReportLoadResult` |

Columns of `load_request`:

| Column | Type | Null | Set by | `UPDATE` grant |
|---|---|---|---|---|
| `id` | `UUID` primary key, no default | no | `RequestRecipeLoad` with the `load_request_id` of the form | no |
| `machine_id` | `UUID` foreign key to `machine` | no | insert, frozen | no |
| `recipe_version_id` | `UUID` foreign key to `recipe_version`, with an index | no | insert, frozen | no |
| `content_sha256` | `TEXT`, grammar of D4 | no | insert, frozen | no |
| `requested_by` | `UUID` foreign key to `app_user`, with an index | no | insert, frozen | no |
| `requested_at`, `claim_deadline` | `TIMESTAMPTZ` | no | insert, frozen | no |
| `state` | `TEXT` `CHECK` of the seven stored states | no | each transition | yes |
| `claimed_by` | `UUID` foreign key to `app_user`, with an index | yes | the claim, then frozen | yes |
| `claimed_at` | `TIMESTAMPTZ` | yes | the claim, then frozen | yes |
| `result_deadline` | `TIMESTAMPTZ` | yes | the claim and each lease | yes |
| `ended_at` | `TIMESTAMPTZ` | yes | `cancelled` or a stored `expired` | yes |
| `released_by` | `UUID` foreign key to `app_user`, with an index | yes | the release, kept after a late result | yes |
| `released_at` | `TIMESTAMPTZ` | yes | the release, kept after a late result | yes |
| `reported_at` | `TIMESTAMPTZ` | yes | the first result | yes |
| `received_outcome`, `received_sha256`, `received_reason`, `received_written`, `vendor_code` | `TEXT`, `TEXT`, `TEXT`, `TEXT`, `BIGINT` | yes | the first result: the parsed body of D8 | yes |
| `reason`, `written` | `TEXT`, `TEXT` | yes | the first result: the stored outcome of `failed` | yes |
| `created_at`, `updated_at` | `TIMESTAMPTZ` | no | insert, and the trigger of `data-model.md` §1.1 at each update | no |

- "The received body" is the four `received_*` columns and `vendor_code`. It holds the parsed body, never the raw text.
- A `CHECK` limits `written` and `received_written` to `none`, `partial`, `all`, and `unknown`. A `CHECK` limits `reason` to the reasons of D8 and `hash_mismatch`, and `received_reason` to the reasons of D8.
- A `CHECK` limits `content_sha256` and `received_sha256` to 64 lower-case hexadecimal digits.
- A `CHECK` limits `received_outcome` to `succeeded` and `failed`, and `vendor_code` to 0 to 4294967295 (D8).
- The identity columns are `id`, `machine_id`, `recipe_version_id`, `content_sha256`, `requested_by`, `requested_at`, and `claim_deadline`.
- The claimant columns are `claimed_by` and `claimed_at`. The result columns are `reported_at`, the received body, `reason`, and `written`.
- The release columns are `released_by` and `released_at`. Only the transition `claimed` to `released` sets them, both together. They are frozen after that.
- An index on `(machine_id, requested_at)` serves the foreign key and the reads of `GetMachineLink`.
- `data-model.md` §1.1 gets an exception row: `id` has no default, because the load form gives it (D10).

Deadlines and effective states:

- `claim_deadline = requested_at + machine_link.claim_timeout`. A claim sets `result_deadline = claimed_at + machine_link.result_timeout`.
- A status report of the claimant that names the claim in `claim` writes `result_deadline = GREATEST(result_deadline, now() + machine_link.result_timeout)`. This is a lease. It also applies after the old `result_deadline`.
- A deadline has passed when `now()` of the transaction is equal to or later than the deadline.
- `now()` is the start time of the transaction (`data-model.md` §1.1). A decision can therefore lag the wall clock by its lock waits. `db.lock_timeout` bounds each lock wait.
- Effective states: a stored `pending` request after `claim_deadline` is `expired`. A stored `claimed` request after `result_deadline` is `unknown`. Every other effective state equals the stored state.
- `expired` means "not started". `unknown` means "the result did not come". No write stores `unknown`.
- An active request is a request in the effective state `pending`, `claimed`, or `unknown`. A request with an unknown result therefore blocks the machine (decision 8).
- A partial unique index permits one row per machine in the stored state `pending` or `claimed`. The effective state `unknown` is a stored `claimed`, so the index covers it.
- Only `RequestRecipeLoad` and `ArchiveMachine` store `expired`, each with its own audit entry. No new job is necessary.
- Only `CancelRecipeLoad` and `ArchiveMachine` store `cancelled`, each with its own audit entry.
- A person ends an `unknown` request with `ReleaseRecipeLoad` after a check of the machine. A late result after the release still applies and records the outcome.
- A request is business history. `recipe_app` gets no `DELETE` on `load_request`.
- The new invariants I-n of `domain-model.md` §3 are one active request per machine and the transition table. They also hold the frozen identity columns and the trigger rules (D12).

Last finished request:

- A finished request is a request in the stored state `succeeded`, `failed`, `cancelled`, `expired`, or `released`, or in the effective state `expired`.
- Its finish time is `reported_at` for `succeeded` and `failed`, `ended_at` for `cancelled` and a stored `expired`, and `released_at` for `released`. An effective `expired` request has its `claim_deadline`.
- The last finished request has the latest finish time. A tie takes the later `requested_at`, then the higher `id`.
- `GetMachineLink` reads the link state, the active request, and the last finished request in one statement, so they come from one snapshot.

Triggers:

- The insert trigger `load_request_insert` requires the state `pending` and no claimant, result, or release column. It requires a version whose recipe belongs to the machine of the row.
- The update trigger `load_request_transition` checks only the state pairs of the transition table, the columns, and the lease. The deadlines are rules of the use cases, not of the trigger.
- It refuses a change of an identity column, a transition outside the table, and a change of a set claimant or release column. It also refuses bad claimant or release columns, bad result columns, and a bad lease (D12).
- `recipe_app` gets no `UPDATE` grant on an identity column. An update of such a column by `recipe_app` therefore gives `42501`. The trigger code covers a writer with the grant.

### D7. Commands, locks, and audit (decided)

| Use case | Locks (ADR-0015 order) | Audit actions |
|---|---|---|
| `RequestRecipeLoad` | `machine` `FOR NO KEY UPDATE`, `recipe` `FOR KEY SHARE`, the approved `recipe_version` `FOR SHARE`, the active `load_request` `FOR UPDATE` | `RecipeLoadRequested`, and `RecipeLoadExpired` when it ends an overdue request |
| `CancelRecipeLoad` | `machine` `FOR NO KEY UPDATE`, `load_request` `FOR UPDATE` | `RecipeLoadCancelled` |
| `ReleaseRecipeLoad` | `machine` `FOR NO KEY UPDATE`, `load_request` `FOR UPDATE` | `RecipeLoadReleased` |
| `ClaimLoadRequest` | `machine` `FOR SHARE`, `recipe_version` `FOR SHARE`, `load_request` `FOR UPDATE` | `RecipeLoadClaimed` |
| `ReportLoadResult` | `load_request` `FOR UPDATE` only | `RecipeLoadSucceeded` or `RecipeLoadFailed` |
| `ArchiveMachine` | its current locks, then the active `load_request` `FOR UPDATE` | `MachineArchived`, and `RecipeLoadCancelled` when it ends a live `pending` request, or `RecipeLoadExpired` when it ends an overdue one |
| `ReportMachineState` | the entries sorted once by machine id. Each `machine` `FOR SHARE`, then each `machine_link_state` upsert, then each lease update of `load_request`, each step in that order | none |

Lock rules (amendment of ADR-0015):

- `machine_link_state` and `load_request` are child rows of the machine. `machine_link_state` comes before `load_request`.
- A load command takes `recipe` `FOR KEY SHARE`, not `FOR UPDATE`, because it changes no recipe row.
- `ReportLoadResult` takes no machine lock. It reads and writes only its `load_request` row, and no rule of the result reads a machine column.
- `ReportMachineState` runs every upsert, also a first insert, in the sorted order. A second report thus waits on the first row and cannot deadlock.

Every write reads its rows under the lock and computes the effective state first, with `now()` of its transaction. It then applies the first rule that matches.

Pre-check of `RequestRecipeLoad` (decision 11):

- After the permission check, the use case reads the `load_request_id` in a short `READ COMMITTED READ ONLY` read through the read port of D7. A known id gives the answer of rule 1 or 2 at once, with no permit and no pre-check.
- For a new id, the use case runs the `loadable` check of ADR-0119 (j), (k) before its transaction, as the version page does (ADR-0119 (l)).
- The check runs in `spawn_blocking` with a permit of `export.permits` and the bound `timeout.export`. This amends ADR-0122.
- A busy gate or the timeout gives `503 E-WEB-009` and writes nothing. The transaction then uses the result in rule 4.

Rule order (each row is one rule. The first rule that matches gives the answer):

| Use case | # | Rule | Answer and write |
|---|---|---|---|
| `RequestRecipeLoad` | 1 | A known `load_request_id` with the same machine, recipe, and version | the stored request with its state, writes nothing |
| | 2 | A known `load_request_id` with another machine, recipe, or version | `409 E-LNK-018` |
| | 3 | An archived machine, or a version that is not `Approved` | `E-PLT-010`, or `E-LNK-014` |
| | 4 | The pre-check gave `loadable = false` | `E-LNK-015` |
| | 5 | A link state that is stale, or a state other than `ready_for_changeover` | `E-LNK-001`, or `E-LNK-002` |
| | 6 | An active request, also in the effective state `unknown` | `E-LNK-003`. An overdue `pending` request becomes `expired` first |
| | 7 | Otherwise | a `pending` request with the `load_request_id` as its primary key |
| `ClaimLoadRequest` | 1 | The stored state `claimed` by this account | `200` with the stored claim, writes nothing, also after the claim deadline |
| | 2 | A stored state other than `pending` | `409 E-LNK-006` |
| | 3 | A `pending` request after its claim deadline | `409 E-LNK-005` |
| | 4 | An archived machine, or a version that is no longer `Approved` | `E-PLT-010`, or `409 E-LNK-014` |
| | 5 | Otherwise | `claimed` by this account |
| `CancelRecipeLoad` | 1 | A request of another machine than `{code}` | `404 E-WEB-005` |
| | 2 | The stored state `cancelled` | `200`, writes nothing |
| | 3 | A stored state other than `pending` | `409 E-LNK-006` |
| | 4 | A `pending` request after its claim deadline | `409 E-LNK-005`, writes nothing |
| | 5 | Otherwise | `cancelled` |
| `ReleaseRecipeLoad` | 1 | A request of another machine than `{code}` | `404 E-WEB-005` |
| | 2 | A request with `released_at`, also after a late result | `200`, writes nothing |
| | 3 | An effective state other than `unknown` | `409 E-LNK-017` |
| | 4 | Otherwise | `released` |
| `ReportLoadResult` | 1 | A request that was never claimed (`pending`, `cancelled`, `expired`) | `409 E-LNK-008` |
| | 2 | A request of another claimant | `409 E-LNK-013` |
| | 3 | A request in `succeeded` or `failed` | the parsed body equals the received body: `200` with the stored outcome, writes nothing. Otherwise `409 E-LNK-007` |
| | 4 | A request in `claimed` or `released` | the outcome of D8, also after `result_deadline` |

- The claim does not run the `loadable` check (decision 11). The adapter checks `loadable` of the document (`api.md` §6.3). A document with `loadable = false` gives the result `failed`, `document_changed`, and `written` `none`.
- `RequestRecipeLoad` inserts with `ON CONFLICT (id) DO NOTHING`, then reads the stored row and applies rule 1 or 2. Two parallel requests with one id for two machines thus get `E-LNK-018`, not `23505`.
- Rule 1 also runs after the `machine` lock, so a parallel retry for the same machine waits for the first commit.
- When the insert finds a conflict, the command reads the stored row in the same transaction. It then rolls the transaction back, also a `RecipeLoadExpired` write of rule 6, and gives the answer of rule 1 or 2.
- The answer of rule 1 names the stored state, for example `succeeded`. A second press of an old form thus does not look like a new request.

`ArchiveMachine`:

- A live `pending` request (before its claim deadline) becomes `cancelled`. The commit holds one `RecipeLoadCancelled` entry. Its actor is the person who runs `ArchiveMachine`.
- An overdue `pending` request becomes `expired`. The commit holds one `RecipeLoadExpired` entry.
- A request in the effective state `claimed` or `unknown` gives `409 E-LNK-003`. The archive then writes nothing.
- A `released` request is not active, so `ArchiveMachine` accepts the machine. A late result can then land on a request of an archived machine. It applies, because the result does not read the machine (lock rules above).

Audit entries:

- `aggregate_type` is `machine` and `entity_type` is `load_request` (`data-model.md` §10).
- Each entry stores the full `load_request` row, as ADR-0005 and the audit rule "Full row for other actions" require. `RecipeLoadRequested` is a create and stores only `after_state`. Every other action stores both states.
- The row holds no secret. It holds account ids, not passwords or tokens.
- I13 names the successful paths without an entry. They are the status report with its lease and the repeated load request, claim, result, cancel, and release.
- The editor status is always older than the PLC state. The checks of `RequestRecipeLoad` only prevent a useless request. The adapter and the PLC interlock make the final decision (`api.md` §10).

### D8. Result and its hash (decided)

Result body of a success: `{"outcome": "succeeded", "content_sha256": "…"}`.

Result body of a failure: `{"outcome": "failed", "reason": "interlock_refused", "written": "none", "vendor_code": 1793}`.

- `content_sha256` is required with `succeeded` and refused with `failed`. It is 64 lower-case hexadecimal digits (D4). `reason` and `written` are required with `failed` and refused with `succeeded`.
- `reason` is one of `machine_not_ready`, `interlock_refused`, `plc_unreachable`, `write_failed`, `verify_failed`, `document_changed`, `adapter_restarted`, `other`.
- `written` is `none`, `partial`, `all`, or `unknown`. It states how much of the version the adapter wrote before the failure. `unknown` states that the adapter does not know it, for example after a crash during a write.
- `vendor_code` is optional. It is an integer from 0 to 4294967295, the error code of the PLC or of its protocol library.
- The body holds no free text. The panel shows the word of the reason from the bundle and the vendor code as a number.
- A `succeeded` result with another `content_sha256` than the request is stored as `failed` with the reason `hash_mismatch` and `written` `all`. Only the editor sets `hash_mismatch`. The request schema refuses it, and the column `CHECK` permits it.
- The row keeps the parsed received body next to the stored outcome (D6). A repeated result compares its parsed body with the received body, not with the stored outcome.
- `document_changed` with `written` `none` covers a change of the approved version after the claim, and a document with `loadable = false`.

Duties of the adapter for a result (`api.md` §10):

- Before it writes, the adapter applies `api.md` §6.3 to the document. It checks `loadable`, rebuilds the hashed body, and compares the SHA-256.
- It also compares `version.number` and `content_sha256` of the document with the request (ADR-0109 (ah)).
- Before it sends `succeeded`, it reads back every leaf of the v1 document from the PLC and compares each leaf.
- On `claim_lost` (D4), it stops every write of that request at once and sends no `succeeded`.
- After a restart, it sends `failed`, `adapter_restarted`, and `written` `unknown` for its open claim, unless it read back the PLC first.
- When it finds a request in `claimed` whose claim answer it never got, it sends `failed`, `machine_not_ready`, and `none`. It does not start that load.

### D9. Read-only mode (decided)

| Route or use case | In read-only mode |
|---|---|
| `ReportMachineState` | applies with its lease, no audit entry |
| `ReportLoadResult` | applies, writes its audit entry |
| `ClaimLoadRequest` | `503 E-WEB-006` from the gate layer, before the Basic check. A load does not start in read-only mode |
| `RequestRecipeLoad`, `CancelRecipeLoad`, `ReleaseRecipeLoad` | `503 E-WEB-006` |
| `ArchiveMachine` | `503 E-WEB-006`, as today |
| `GetMachineLink` | applies |

- This amends A24, ADR-0020, and ADR-0061 (e). `RecipeLoadSucceeded` and `RecipeLoadFailed` join the audit actions of read-only mode.
- The gate layer of `recipe-web` refuses the claim (decision 15). The claim thus runs no hash, changes no `load_request` row, and writes no audit entry. Under `/adapter/v1/`, the answer is the JSON error body.
- `recipe-app` keeps one `runs_in_read_only` row per use case in the generated matrix. The use cases add no second read-only check.
- Three normative lists change with the matrix. They are the web-ui requirement "Read-only error response", the operations scenario "Comparison in read-only mode", and the `E-WEB-006` row of `api.md` §5.1.
- The page shows the read-only banner and no load, cancel, or release form (`web-ui`, "Controls of a blocked command in read-only mode").

### D10. UI (decided)

The machine page of a live machine holds two parts of the link panel:

- The status part `#machine-link-panel` shows the link state and the loaded recipe with its comparison (D5). It shows the state of the active and the last finished request (D6). It also shows the time of the last report and the time of its own render.
- The action part `#machine-link-actions` holds the cancel form and the release form. It is outside the polled element, so a poll never removes a form or its focus.

Polling:

- The status part has the literal attributes `hx-get=""`, `hx-trigger="every 5s"`, `hx-target="this"`, `hx-swap="outerHTML"`, `hx-include="find input"`, and `data-passive`. The vendored HTMX 2 sends an empty `hx-get` to `location.href`.
- The status part holds the hidden input `machine_id` with the id of the machine. The poll thus sends `machine_id` as a query key.
- No `hx-*` attribute holds user data. ADR-0224 amends ADR-0037, because this request does not come from a form or a link. A template test requires that `every 5s` equals `ui.link_poll_interval`.
- A passive request is a `GET` of `/machines/{code}` or `/machines/by-id/{uuid}` with `HX-Request: true` and `HX-Target: machine-link-panel`. Any other method or route with these headers is a normal request.
- A passive request runs `GetMachineLink` for a live machine whose id equals `machine_id`. It gives only the status part.
- The session layer checks a passive request with the new session port method of `data-model.md` §3.6. It checks expiry, epoch, and account like the normal statement, but it moves neither `last_seen_at` nor `expires_at`. This amends ADR-0014 and `api.md` §7.2.
- A passive request for an archived or missing machine, or with a missing, malformed, or other `machine_id`, gives `200` with a terminal status part. That part says "The machine changed. Reload the page" and has no `hx-get` and no `hx-trigger`, so the swap ends the polling.
- A passive request that fails in the error layer gives `204`, so the status part keeps its content and its render time. `E-WEB-004` keeps `HX-Redirect: /login`.
- A panic that the inner panic layer catches also gives `204` to a passive request. This amends `reliability.md` §2.4 rule 3 and ADR-0057 (b). The backstop panic layer keeps its plain `500`.
- `request-state.js` reads `event.detail.elt`, not `event.detail.target`. HTMX applies `HX-Retarget` to the target before `htmx:beforeSwap`, so a check of the target would fail.
- `event.detail.elt` is the target as HTMX found it before `HX-Retarget`. It equals the requesting element only because the status part has `hx-target="this"`. A change of that attribute needs `event.detail.requestConfig.elt`.
- In `htmx:beforeSwap`, the script drops the swap of an answer with a status of 400 or higher when `event.detail.elt` has `data-passive`. This covers the `502` of the proxy and the backstop panic answer.
- In `htmx:beforeRequest`, the script checks `data-passive` first. A poll therefore neither shows nor removes the message `E-WEB-015`. The cap of `request-state.js` becomes 45 lines. This amends ADR-0158 and `api.md` §9.
- The page of an archived machine holds no link panel.

Forms:

- The version page holds the load form only for an `Approved` version of a live machine with a `loadable` version. For a `Utf8` machine, the page shows a notice that this change gives the machine no load (decision 12).
- The load form is `POST /recipes/{ref}/versions/{n}/load`. It has `hx-sync="closest form:drop"`, no `hx-disabled-elt`, and the literals `hx-target="#version-load"` and `hx-swap="outerHTML"` (ADR-0049).
- The load form holds the hidden field `load_request_id`, a UUID that the server makes at each render. A retry of a lost answer thus gives the stored request (D7).
- A success replaces `#version-load` with the fragment of the request and a fresh load form with a new `load_request_id`. The fragment names the state of the request and links to the machine page.
- The action part holds a cancel form for an effective `pending` request: `POST /machines/{code}/load-requests/{request_id}/cancel`.
- The action part holds a release form for an effective `unknown` request: `POST /machines/{code}/load-requests/{request_id}/release`. The form shows the instruction to check the machine before the release.
- The cancel form and the release form have `hx-sync="closest form:drop"`, no `hx-disabled-elt`, and the literals `hx-target="#machine-link-actions"`, `hx-swap="outerHTML"`, and `hx-push-url="false"`. Their success gives a fresh action part.
- Without `hx-push-url="false"`, a boosted success pushes the form URL, and the empty `hx-get` of the poll then requests that URL.
- The three handlers build their answer from the result of the command and run no read after the commit. ADR-0163 therefore does not apply to them.
- The command result of the cancel and the release holds the active request of the machine after the change. The command reads it inside its transaction.
- Without HTMX, the load form gives `303` to the machine page of the recipe. The cancel and release forms give `303` to `/machines/{code}`.
- The action part renders with the page and after each answer of its forms. The status part says to reload the page when it shows `unknown` and the action part has no release form. It says so only when the mode permits the release, so not in read-only mode.
- A person without `RecipeLoadRequest` sees no load, cancel, or release form.
- The texts come from the bundles `pl` and `en`, also the words of `written` and of each reason.

### D11. Settings (decided)

The owner accepted these values on 2026-11-25 (decision 10).

| Key | Value | Use |
|---|---|---|
| `adapter.basic_permits` | 1 | D3 |
| `machine_link.report_interval` | 10 s | The interval that `api.md` §10 asks of the adapter. The editor does not enforce it |
| `machine_link.stale_after` | 30 s | D5, three intervals |
| `machine_link.claim_timeout` | 60 s | D6 |
| `machine_link.result_timeout` | 120 s | D6, the claim time and the lease of each report |
| `machine_link.report_max_machines` | 32 | D4 |
| `http.body_limit.adapter` | 64 KiB | D4 |
| `ui.link_poll_interval` | 5 s | D10 |

These are constants in `reliability.md` §7.11, not configuration keys. A later change can make one configurable.

### D12. Error codes (decided rows of `reliability.md` §1.2)

`LinkError` in `recipe-domain` owns the prefix `E-LNK`. It is the fifth error enum of `recipe-domain`. The rows go into §1.2 before the variants. The trigger codes use the block `E-LNK-030..049`, as the other trigger blocks of `reliability.md` §1.1. The CTX list of §1.2 gets `LNK` (Machine Link) and the ranges `E-LNK-001..029` and `E-LNK-030..049`.

| Code | Status | Meaning |
|---|---|---|
| `E-LNK-001` | 409 | No fresh status report (D5) |
| `E-LNK-002` | 409 | The machine does not report `ready_for_changeover` |
| `E-LNK-003` | 409 | The machine has an active load request |
| `E-LNK-004` | 404 | The load request of an adapter route does not exist, or its id is malformed |
| `E-LNK-005` | 409 | The claim deadline passed |
| `E-LNK-006` | 409 | The request is not `pending` |
| `E-LNK-007` | 409 | A second result differs from the received result |
| `E-LNK-008` | 409 | Result of a request that was never claimed |
| `E-LNK-009` | 422 | Bad body, media type, or JSON syntax of an adapter route |
| `E-LNK-010` | 422 | An empty entry list, too many entries, or a code twice in one report |
| `E-LNK-012` | - | A status report that is not newer than the stored one. Only an entry result of a `200` answer |
| `E-LNK-013` | 409 | Result from an account that did not claim the request |
| `E-LNK-014` | 409 | The version is not `Approved` |
| `E-LNK-015` | 409 | The version is not loadable |
| `E-LNK-016` | - | Another run or account reported the machine while the row is fresh. Only an entry result of a `200` answer |
| `E-LNK-017` | 409 | The request has no unknown result |
| `E-LNK-018` | 409 | The load request id names another request |
| `E-LNK-030` | 409 | Trigger `load_request_transition`: a transition outside the table |
| `E-LNK-031` | 422 | Trigger `load_request_insert`: a new row is not `pending` or has a claimant, result, or release column |
| `E-LNK-032` | 422 | Trigger `load_request_insert`: the version belongs to a recipe of another machine |
| `E-LNK-033` | 409 | Trigger `load_request_transition`: a change of an identity column |
| `E-LNK-034` | 422 | Trigger `load_request_transition`: a claimed row without its claimant columns, or release columns outside the release or with one field missing |
| `E-LNK-035` | 422 | Trigger `load_request_transition`: result columns that do not fit the state |
| `E-LNK-036` | 409 | Trigger `load_request_transition`: a lease on a row that is not `claimed`, or an earlier `result_deadline` |
| `E-LNK-037` | 409 | Trigger `load_request_transition`: a change of the claimant columns after the claim, or of the release columns after the release |

- `E-LNK-011` stays free.
- A shape rule gives `422` and a frozen or state rule gives `409`, as the trigger rows of `E-PLT-030..038` do.
- The insert trigger checks `E-LNK-031` before `E-LNK-032`. The update trigger checks `E-LNK-033`, `E-LNK-030`, `E-LNK-037`, `E-LNK-034`, `E-LNK-035`, and `E-LNK-036`, in this order. The first failed rule gives the code.
- The trigger code map of `crates/recipe-db/src/error.rs` gets the prefix `E-LNK-`. An `E-LNK-03x` trigger code thus does not become `E-DB-003`.
- The UI routes map the same variants to the error fragments of `api.md` §5.

### D13. Documents (decided)

- ADR-0224 records D1 to D12 and the decisions 1 to 16. The Status lines of the amended ADRs get "Amended by ADR-0224".
- ADR-0224 amends ADR-0013 (Operator permission), ADR-0014 (passive request), ADR-0015 (child rows, lock rules, fifth write port), and ADR-0019 (transfer scope).
- It amends ADR-0020 (read-only actions), ADR-0035 (CSRF and `Origin` exemption), ADR-0037 (passive poll element), and ADR-0051 (d), (e).
- It amends ADR-0053 (a) (JSON body with `path`), ADR-0053 (b) (no cookie on an adapter route), and ADR-0055 (a).
- It amends ADR-0057 (b) (`204` for a passive request), ADR-0058 (g), ADR-0061 (e), and ADR-0065 (d).
- It amends ADR-0109 (b), (m), (n), (o), (ae), ADR-0122 (the pre-check permit), ADR-0154 (a), and ADR-0158 (line cap and passive element).
- These are 18 ADRs.
- ADR-0051 (d), (e): the audit filters, the select, the entry page, and the labels get `load_request` and the seven actions. The label of a load request is the machine code, the reference, and the version. A role with only `AuditViewRecipes` keeps its predicate `aggregate_type = 'recipe'`, so it does not see these entries.
- `overview.md`: the rows R03, R24, A24, and A45 cite A55 and ADR-0224. `domain-model.md` §8 holds identical copies of A24, A45, and A55.
- `api.md` gets §10 "Adapter link": the routes, the JSON contract, and the timing. It also changes §1 (both 404 codes of `{request_id}`), §2, §3, §5.1 (`E-WEB-006`), and §8 (no cookie on an adapter route).
- It also changes §5.3 (no `HX-*` header under `/adapter/v1/`), §7.2 (passive request), §7.3 (CSRF exemption), §7.6 (body limit), §7.7, and §9 (45 lines).
- `data-model.md` changes §1.1 (exception rows), §1.2, §2 (the ERD), §3 (the DDL), §3.6 (the passive session statement), §4, §4.11, §5.2, §6, and §10.
- `reliability.md` changes §1.1, §1.2, §2.4 rule 3 (`204` for a passive request), §4.2 (the JSON body of an adapter route), §7.8, and §7.11.
- `api.md` §10 gives the duties of the adapter of D8. The adapter honors the interlock, loads the whole v1 document, and runs one claim at a time per machine.
- The adapter names the claim in each report while it runs, and each sent report takes the next `seq`. It never writes again after a lost answer. It sends the result again.
- The configuration of the adapter binds each machine code to its PLC for the reports (decision 13). The adapter reads the connection record only before a load (ADR-0154 (jc)).
- The adapter checks the HTTPS scheme and the certificate before it sends the credentials. After a restart, it can wait up to `stale_after` plus one `report_interval` until it sees its claim.
- `.claude/rules/openspec.md` and `openspec/config.yaml` name eight capabilities. `.claude/rules/web.md` and the hard rules of `CLAUDE.md` get the exceptions of D4, D9, and D10.
- `.claude/rules/web.md` adds the load, cancel, and release forms to its `hx-sync` form list. `.claude/rules/rust.md` and `CLAUDE.md` name `LinkError` as the fifth error enum.
- `docs/ops/` gets a how-to: create the `ExportClient` account of an adapter, open TCP port 443 from the adapter host, and roll back the code.
- The how-to also covers a reused machine code and a restore (Risks, Migration Plan).

### D14. Numbers (decided)

ADR-0224 and A55 are provisional. ADR-0222 is held by `value-body-memory-load`, and A54 by change B. The apply step takes the next free numbers of `docs/adr/README.md` and `overview.md` §9 and updates this change.

## Risks / Trade-offs

- Risk: a stolen `ExportClient` password can send false states and false results. Mitigation: the same password already gives the PLC passwords (ADR-0154). Every claim and result has an audit entry with the account. The PLC interlock still decides each load.
- Risk: any `ExportClient` account can claim the request of any machine. A misconfigured adapter can thus claim a load that it cannot do. The owner put the binding of accounts to machines out of scope. The audit entry names the account, and the PLC interlock still decides.
- Risk: a reused machine code can bind a late report to the new machine. The new machine has no row yet, so the run rule of D5 does not stop the report. Mitigation: the configuration of the adapter binds each code to its PLC (decision 13).
- The how-to tells the operator to update the configuration of each adapter before a reused code goes live. A residual risk stays for a wrong adapter configuration.
- Risk: the status of the editor is older than the PLC state. Mitigation: `stale_after`, the time of the last report in the panel, and the final decision in the PLC (D7).
- Risk: an adapter crash after a write leaves a request `claimed`. Mitigation: the `claimed` field of the report answer gives the claim to the restarted adapter. It sends `written` `unknown` unless it read back the PLC (D8). After `result_deadline`, a person releases the request after a check of the machine.
- Risk: a person releases a request while the adapter still writes. Mitigation: the release form names the check of the machine. The lease keeps a running load out of `unknown`, `claim_lost` stops the old process (D4), and the PLC interlock decides each load.
- Risk: an adapter that names a claim in each report but sends no result blocks the machine. Mitigation: the panel shows the claim and its account. A restart of the adapter ends the lease.
- Risk: a restored database can lack a request that the adapter still executes. A person could then ask for a second load. Mitigation: the how-to stops the adapters before a restore.
- After a restore, the operator compares the adapter, the machine, and the database before the first claim. `claim_lost` (D4) stops a claim that the database does not know.
- A restored `pending` request is always effectively `expired`, because `claim_timeout` is shorter than any restore.
- Risk: a decision can lag the wall clock by its lock waits (D6). Mitigation: `db.lock_timeout` bounds each lock wait. The deadlines are soft rules of the UI, and the interlock decides each load.
- Trade-off: one row write per machine per report also in read-only mode. The volume is small. The owner accepted it (decision 4).
- Trade-off: the UI logins keep 2 Argon2 permits instead of 3. One line has few persons who log in at the same time.
- Trade-off: a session with only an open machine page ends after the idle timeout. This is the intended security control of ADR-0014.
- Trade-off: the effective state of a request comes from the deadlines. A stored `pending` row can stay until the next `RequestRecipeLoad` or `ArchiveMachine` of its machine.
- Trade-off: a CODESYS or TwinCAT machine with `Utf8` gets no load until a later change adds a v2 path (decision 12).

## Migration Plan

- One forward migration creates `machine_link_state`, `load_request`, the two triggers, the indexes, and the grants. It changes no existing table. The format checks of `audit_log.action` and `entity_type` already permit the new values.
- The migration also updates `deploy/pgtools/restore-code.expected` and `restore-roles.expected` (ADR-0103 (m)) for the new functions, triggers, `CHECK` constraints, and grants.
- No backfill. A machine without a row shows `no_link`.
- A rollback is a code rollback only. The tables, their rows, and their grants stay, because `load_request` is business history. No migration drops them.
- The previous image reads an unknown audit action as corrupt. A rollback of the image only is therefore permitted only while `audit_log` holds 0 entries of the seven new actions (decision 16, as ADR-0202 (q)).
- The how-to gives the count query. With a count above 0, the operator rolls forward and does not roll back.
- Before the count, the operator stops the adapters and removes the container of `app` (as the runbook "Roll back on the newer schema"). The count then runs through the `db` container. The writers stay stopped until the previous image starts.
- At a code rollback, the `recipe-pgtools` image of the newer schema stays (ADR-0083 (c)).

## Open Questions

- None.
