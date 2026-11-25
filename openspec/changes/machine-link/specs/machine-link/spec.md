## ADDED Requirements

### Requirement: Adapter link without a PLC connection

The system SHALL receive the state of each machine and give its load requests only through the adapter routes under `/adapter/v1/`. The adapter SHALL start every connection. The system SHALL NOT open a connection to an adapter or to a PLC (R03, A47, A55).

#### Scenario: Status report of an adapter

- **GIVEN** the live machine `M12` and the `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends a status report for `M12` through `POST /adapter/v1/reports`
- **THEN** the system stores the state of `M12`
- **AND** the system starts no connection to an adapter or to a PLC

#### Scenario: Load request of a person

- **GIVEN** the live machine `M12` with a fresh link state `ready_for_changeover`
- **WHEN** an Operator runs `RequestRecipeLoad` for the approved version 3 of the recipe `R-100`
- **THEN** the system stores one `LoadRequest` in the state `pending`
- **AND** the system starts no connection to an adapter or to a PLC

### Requirement: Permission of the adapter routes

`ReportMachineState`, `ClaimLoadRequest`, and `ReportLoadResult` SHALL check the permission `MachineLinkReport` as their first statement. The roles `ExportClient` and Administrator SHALL hold it. Over HTTP, only an `ExportClient` account SHALL reach the adapter routes (R26, R28, A45, A55).

#### Scenario: Report of a person account

- **GIVEN** the Operator account `op1` with a valid password
- **WHEN** a client sends `POST /adapter/v1/reports` with the Basic credentials of `op1`
- **THEN** the system returns `401 E-IDN-003`
- **AND** the system stores no link state

### Requirement: Basic check of the adapter routes

The adapter routes SHALL authenticate a request only with the HTTP Basic credentials of an active `ExportClient` account, by the Basic check of the transfer routes. The navigation guard SHALL run before the Basic check. A `401` answer SHALL have the header `WWW-Authenticate: RecipeAdapter realm="adapter"` (R28, A42, A45, A55).

#### Scenario: Missing header

- **GIVEN** the live machine `M12`
- **WHEN** a client sends `POST /adapter/v1/reports` without an `Authorization` header
- **THEN** the system returns `401 E-WEB-012` as the JSON error body
- **AND** the answer has the header `WWW-Authenticate: RecipeAdapter realm="adapter"`

#### Scenario: Cross-site request

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a request with `Sec-Fetch-Site: cross-site` and the Basic credentials of `plc-m12` reaches `POST /adapter/v1/reports`
- **THEN** the system returns `403 E-WEB-007` as the JSON error body
- **AND** the system runs no password check

#### Scenario: HTMX request

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a request with `HX-Request: true` and the Basic credentials of `plc-m12` reaches `POST /adapter/v1/reports`
- **THEN** the system returns `403 E-WEB-007`
- **AND** the system writes one `csrf_rejected` security event

### Requirement: JSON answers of the adapter routes

Every error answer of a route under `/adapter/v1/` SHALL be the JSON error body of the export routes. This SHALL apply to the navigation guard, the Basic check, the body limit, the request timeout, the read-only gate, and the handler. Only the backstop panic layer SHALL keep its `text/plain` answer. The request timeout SHALL apply to the adapter routes (R10, R30, A55).

#### Scenario: Request timeout

- **GIVEN** a claim of `plc-m12` that runs longer than the request timeout
- **WHEN** the request timeout ends the request
- **THEN** the system returns `503 E-WEB-009` as the JSON error body

#### Scenario: Read-only gate

- **GIVEN** the application in read-only mode and a `pending` request of `M12`
- **WHEN** `plc-m12` claims the request
- **THEN** the system returns `503 E-WEB-006` as the JSON error body
- **AND** the system runs no password check

### Requirement: Body of an adapter route

An adapter route SHALL accept only the media type `application/json`, with no parameter or with `charset=utf-8`, and a body up to `http.body_limit.adapter`. A body above the limit SHALL give `413 E-WEB-003`. Another media type or a JSON syntax error SHALL give `422 E-LNK-009` with the path `$`. The claim body SHALL be the empty object `{}` (R38, A55).

#### Scenario: Form body

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends `POST /adapter/v1/reports` with `Content-Type: application/x-www-form-urlencoded`
- **THEN** the system returns `422 E-LNK-009` with the path `$`

#### Scenario: JSON with the UTF-8 parameter

- **GIVEN** the active `ExportClient` account `plc-m12` and the live machine `M12`
- **WHEN** `plc-m12` sends a valid status report with `Content-Type: application/json; charset=utf-8`
- **THEN** the system stores the link state of `M12`

#### Scenario: Syntax error

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends the body `{"run_id":` to `POST /adapter/v1/reports`
- **THEN** the system returns `422 E-LNK-009` with the path `$`

#### Scenario: Body above the limit

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends a body that is longer than `http.body_limit.adapter`
- **THEN** the system returns `413 E-WEB-003`

#### Scenario: Claim with a body

- **GIVEN** a `pending` request of `M12` before its claim deadline
- **WHEN** `plc-m12` claims it with the body `{"force": true}`
- **THEN** the system returns `422 E-LNK-009` with the path `$`
- **AND** the request keeps the state `pending`

### Requirement: Field errors of an adapter body

An unknown field, a missing field, and a value outside its grammar SHALL give `422 E-LNK-009` with a path of the schema. The path of an unknown field SHALL be the path of its parent object. The answer SHALL NOT repeat a field name that the client sent (R38, A55).

#### Scenario: Unknown field

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends a status report with the extra field `"speed": 3` in the first entry
- **THEN** the system returns `422 E-LNK-009` with the path `$.machines[0]`
- **AND** the answer does not hold the text `speed`
- **AND** the system stores no link state

#### Scenario: Script in a reference number

- **GIVEN** the active `ExportClient` account `plc-m12` and the live machine `M12`
- **WHEN** `plc-m12` sends a status report whose `loaded.reference_number` is `<script>alert(1)</script>`
- **THEN** the system returns `422 E-LNK-009` with the path `$.machines[0].loaded.reference_number`
- **AND** the system stores no link state

### Requirement: Status report of many machines

A status report SHALL hold `run_id`, `seq`, and 1 to `machine_link.report_max_machines` machine entries, each machine code once. Each entry SHALL hold the code, the state `ready_for_changeover`, `running`, `stopped`, or `fault`, the loaded recipe or `null`, and the claim or `null`. The system SHALL refuse an empty list, a longer list, or a repeated code with `422 E-LNK-010` (R38, A55).

#### Scenario: Two machines in one report

- **GIVEN** the live machines `M12` and `M13`
- **WHEN** `plc-m12` sends one status report with an entry for `M12` and an entry for `M13`
- **THEN** the system stores the link state of `M12` and of `M13`
- **AND** the system runs one password check

#### Scenario: Repeated code

- **GIVEN** the live machine `M12`
- **WHEN** `plc-m12` sends a status report with two entries for `M12`
- **THEN** the system returns `422 E-LNK-010`
- **AND** the system stores no link state

#### Scenario: Empty list

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends a status report with an empty list `machines`
- **THEN** the system returns `422 E-LNK-010`

#### Scenario: Too many entries

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends a status report with one entry more than `machine_link.report_max_machines`
- **THEN** the system returns `422 E-LNK-010`
- **AND** the system stores no link state

### Requirement: Result of each report entry

The system SHALL apply each valid entry of a status report and SHALL give one result per entry. An unknown machine code SHALL give the entry result `E-PLT-019`. An archived machine SHALL give `E-PLT-010`. An entry with the stored `run_id` and the stored account and a `seq` that is not higher than the stored `seq` SHALL give `E-LNK-012`. An entry with an entry result SHALL change nothing (R31, R38, A55).

#### Scenario: One unknown code

- **GIVEN** the live machine `M12` and no machine `M99`
- **WHEN** `plc-m12` sends a status report with entries for `M12` and `M99`
- **THEN** the system stores the link state of `M12`
- **AND** the entry result of `M99` is `E-PLT-019`

#### Scenario: Old report after a newer one

- **GIVEN** the stored link state of `M12` with the run id `A` and the `seq` 50
- **WHEN** `plc-m12` sends a status report for `M12` with the run id `A` and the `seq` 49
- **THEN** the entry result of `M12` is `E-LNK-012`
- **AND** the stored link state of `M12` keeps the `seq` 50

### Requirement: Run and account of a report

An entry with another `run_id` or another account than the stored row SHALL give `E-LNK-016` while the stored row is fresh. A row SHALL be stale when the time of the transaction is equal to or later than its report time plus `machine_link.stale_after`. The entry SHALL replace a stale row (R31, A55).

#### Scenario: New run after a stale row

- **GIVEN** the stored link state of `M12` with the run id `A`, the `seq` 50, and a report time 31 seconds ago
- **WHEN** `plc-m12` sends a status report for `M12` with the run id `B` and the `seq` 0
- **THEN** the system stores the run id `B` and the `seq` 0 for `M12`

#### Scenario: New run at the stale limit

- **GIVEN** the stored link state of `M12` with the run id `A` and a report time exactly 30 seconds ago
- **WHEN** `plc-m12` sends a status report for `M12` with the run id `B` and the `seq` 0
- **THEN** the system stores the run id `B` for `M12`

#### Scenario: New run within the stale limit

- **GIVEN** the stored link state of `M12` with the run id `A` and a report time 5 seconds ago
- **WHEN** `plc-m12` sends a status report for `M12` with the run id `B` and the `seq` 0
- **THEN** the entry result of `M12` is `E-LNK-016`
- **AND** the stored link state of `M12` keeps the run id `A`

#### Scenario: Old run after a new run

- **GIVEN** the stored link state of `M12` with the run id `B`, the `seq` 0, and a report time 2 seconds ago
- **WHEN** a late status report of `plc-m12` for `M12` arrives with the run id `A` and the `seq` 51
- **THEN** the entry result of `M12` is `E-LNK-016`
- **AND** the stored state, the report time, and the account of `M12` stay the values of `B`

#### Scenario: Report of a second account

- **GIVEN** the stored link state of `M12` from `plc-m12` with a report time 5 seconds ago
- **WHEN** the `ExportClient` account `plc-m13` sends a status report for `M12`
- **THEN** the entry result of `M12` is `E-LNK-016`

### Requirement: Status report without an audit entry

`ReportMachineState` SHALL be an adapter report and not a command. It SHALL write the link state and the lease of a claimed request in its own short transaction through the port `MachineLinkStore`. It SHALL write no audit entry (A55).

#### Scenario: Report changes the state

- **GIVEN** the stored link state `running` of `M12`
- **WHEN** `plc-m12` sends a status report with the state `ready_for_changeover` for `M12`
- **THEN** the stored link state of `M12` is `ready_for_changeover`
- **AND** the audit log holds no new entry

### Requirement: Lock order of a status report

A status report SHALL run in one transaction and SHALL sort its entries by machine id once. It SHALL lock each `machine` row `FOR SHARE`, then run each upsert, also a first insert, then each lease update, each step in that order. It SHALL check the archive state under the lock. A failed write SHALL leave no entry applied (R29, R31, A55).

#### Scenario: Two reports in reverse order

- **GIVEN** the live machines `M12` and `M13`
- **WHEN** one report lists `M12` and then `M13`, and a parallel report lists `M13` and then `M12`
- **AND** a barrier holds the first report between its two upserts
- **THEN** both reports end without a deadlock

#### Scenario: Store error during a report

- **GIVEN** a status report for `M12` and `M13` and a write that fails at the entry of `M13`
- **WHEN** `plc-m12` sends the report
- **THEN** the system returns `503` as the JSON error body
- **AND** the stored link state of `M12` does not change

### Requirement: Pending load request in the report answer

The result of an applied entry SHALL hold the load request of the machine in the effective state `pending`, or `null`. It SHALL also hold the load request in the stored state `claimed` by the reporting account, or `null`, also in the effective state `unknown`. Each request SHALL hold its id, the machine code, the reference, the version, its `content_sha256`, and the path of the v1 document. That path SHALL be a route that accepts HTTP Basic (A55).

#### Scenario: Pending request

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` before its claim deadline, for the version 3 of `R-100`
- **WHEN** `plc-m12` sends a status report for `M12`
- **THEN** the entry result of `M12` holds the id of the request, `R-100`, the version 3, and its `content_sha256`
- **AND** it holds the path `/export/v1/machines/M12/recipes/R-100/approved`

#### Scenario: Open claim after a restart

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed` by `plc-m12` after its result deadline
- **AND** a stale link state of `M12`
- **WHEN** `plc-m12` sends a status report for `M12` with a new run id
- **THEN** the entry result of `M12` holds that request in the field `claimed`

### Requirement: Lease of a claimed load request

A report entry can name a request in the stored state `claimed` by the reporting account in `claim`. The entry SHALL then set the result deadline to the later of the stored deadline and the report time plus `machine_link.result_timeout`. This SHALL also apply after the old result deadline. A `claim` that names another request SHALL renew nothing (R31, A55).

#### Scenario: Report renews the claim

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed` by `plc-m12` with its result deadline in 10 seconds
- **WHEN** `plc-m12` sends a status report for `M12` that names the request in `claim`
- **THEN** the result deadline of the request is 120 seconds after the report
- **AND** the audit log holds no new entry

#### Scenario: Claim of another account

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed` by `plc-m12` and a stale link state of `M12`
- **WHEN** `plc-m13` sends a status report for `M12` that names the request in `claim`
- **THEN** the result deadline of the request does not change
- **AND** the entry result holds the id of the request in `claim_lost`

#### Scenario: Report that waited for a lock

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed` by `plc-m12` and a later lease of a parallel report
- **WHEN** a report of `plc-m12` that started earlier gets its lock after that lease
- **THEN** the result deadline of the request does not move back
- **AND** the report applies

### Requirement: Link state with its age

`GetMachineLink` SHALL check the permission `RecipeView` first. It SHALL give the effective link state, the time of the last report, and the loaded recipe of a machine. It SHALL also give the active load request and the last finished load request. The effective state SHALL be `no_link` when the machine has no report or its row is stale by the rule of the run (R39, A55).

#### Scenario: Report at the stale limit

- **GIVEN** the last status report of `M12` exactly 30 seconds ago and `machine_link.stale_after` of 30 seconds
- **WHEN** an Operator runs `GetMachineLink` for `M12`
- **THEN** the effective link state is `no_link`

#### Scenario: Old report

- **GIVEN** the last status report of `M12` 31 seconds ago and `machine_link.stale_after` of 30 seconds
- **WHEN** an Operator runs `GetMachineLink` for `M12`
- **THEN** the effective link state is `no_link`
- **AND** the query gives the time of the last report

#### Scenario: Loaded recipe compared with the approved version

- **GIVEN** the approved version 3 of `R-100` and a fresh report of `M12` that names `R-100` with the `content_sha256` of version 2
- **WHEN** an Operator runs `GetMachineLink` for `M12`
- **THEN** the query marks the loaded recipe as another version than the approved version

#### Scenario: Last finished request

- **GIVEN** a `LoadRequest` of `M12` in the state `succeeded` and no active request
- **WHEN** an Operator runs `GetMachineLink` for `M12`
- **THEN** the query gives that request as the last finished request with its state and its time

#### Scenario: Role without the view permission

- **GIVEN** a test role without `RecipeView`
- **WHEN** the generated matrix test runs `GetMachineLink` with that role
- **THEN** the query returns `E-WEB-002` before it reads data

### Requirement: Lost claim in the report answer

The result of each entry of a known live machine SHALL hold `claim_lost`. This includes an entry with the entry result `E-LNK-012` or `E-LNK-016`. The value SHALL be the id of `claim` when that id names no request of the machine in the stored state `claimed` by the reporting account. Otherwise it SHALL be `null`. The lease SHALL not change (R31, A55).

#### Scenario: Old process after a release and a new claim

- **GIVEN** the request `L1` of `M12` in the state `released` and the request `L2` of `M12` in the state `claimed` by `plc-m12`
- **WHEN** an old process of `plc-m12` sends a status report for `M12` that names `L1` in `claim`
- **THEN** the entry result holds `L1` in `claim_lost`
- **AND** the result deadline of `L2` does not change

#### Scenario: Old run with a fresh row of a new run

- **GIVEN** the request `L1` of `M12` in the state `released`
- **AND** the stored link state of `M12` with the run id `B` and a report time 2 seconds ago
- **WHEN** an old process of `plc-m12` with the run id `A` sends a status report for `M12` that names `L1` in `claim`
- **THEN** the entry result of `M12` is `E-LNK-016`
- **AND** the entry result holds `L1` in `claim_lost`
- **AND** the stored state, the report time, and the run id of `M12` stay the values of `B`

#### Scenario: Claim unknown after a restore

- **GIVEN** the live machine `M12` and no load request with the id `L7`
- **WHEN** `plc-m12` sends a status report for `M12` that names `L7` in `claim`
- **THEN** the system stores the link state of `M12`
- **AND** the entry result holds `L7` in `claim_lost`

### Requirement: Last finished load request

`GetMachineLink` SHALL give as the last finished request the request with the latest finish time. A tie SHALL take the later request time, then the higher id. The finish time SHALL be the result time, the end time, or the release time of its stored state. An effective `expired` request SHALL use its claim deadline. The query SHALL read all parts in one statement (R39, A55).

#### Scenario: Two finished requests and one lazy expiry

- **GIVEN** a `failed` request of `M12` with the result time 10:00 and a `succeeded` request with the result time 10:05
- **AND** a stored `pending` request of `M12` with the claim deadline 10:07
- **WHEN** an Operator runs `GetMachineLink` for `M12` at 10:10
- **THEN** the last finished request is the `pending` request in the effective state `expired`

### Requirement: Scope of a load request

A load request SHALL cover the whole approved version. The system SHALL give the adapter only the path of the v1 document of the version and no path of a unit document (A55).

#### Scenario: Claim answer

- **GIVEN** a `pending` request of `M12` for the approved version 3 of `R-100` before its claim deadline
- **WHEN** `plc-m12` claims it
- **THEN** the answer holds the path `/export/v1/machines/M12/recipes/R-100/approved`
- **AND** the answer holds no path under `/export/v2/`

### Requirement: Load request of an approved version

`RequestRecipeLoad` SHALL check the permission `RecipeLoadRequest` first. It SHALL store one `LoadRequest` in the state `pending` with the version and its `content_sha256`. It SHALL refuse a version that is not `Approved` with `E-LNK-014` and a version with `loadable = false` with `E-LNK-015`. It SHALL refuse a link state that is not fresh with `E-LNK-001` and a state other than `ready_for_changeover` with `E-LNK-002`. It SHALL refuse an archived machine with `E-PLT-010` (R18, R28, R29, A55).

#### Scenario: Request of a retired version

- **GIVEN** the retired version 2 of `R-100` on `M12` and a fresh link state `ready_for_changeover`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 2 of `R-100`
- **THEN** the use case rejects the request with `E-LNK-014`
- **AND** the system stores no `LoadRequest`

#### Scenario: Version that is not loadable

- **GIVEN** the approved version 3 of `R-100` with `loadable = false` and a fresh link state `ready_for_changeover` of `M12`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 3 of `R-100`
- **THEN** the use case rejects the request with `E-LNK-015`
- **AND** the system stores no `LoadRequest`

#### Scenario: Machine that runs

- **GIVEN** the approved version 3 of `R-100` on `M12` and a fresh link state `running`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 3 of `R-100`
- **THEN** the use case rejects the request with `E-LNK-002`

#### Scenario: No link

- **GIVEN** the approved version 3 of `R-100` on `M12` and no status report within `machine_link.stale_after`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 3 of `R-100`
- **THEN** the use case rejects the request with `E-LNK-001`

### Requirement: Loadable check before the transaction

After the permission check, `RequestRecipeLoad` SHALL read its `load_request_id`. A known id SHALL get the answer of the retry rule with no permit. For a new id, it SHALL run the `loadable` check before its transaction, with a permit of `export.permits` and the bound `timeout.export`. A busy gate or the timeout SHALL give `503 E-WEB-009` and SHALL write nothing. A version of a machine with the string encoding `Utf8` SHALL give `E-LNK-015` (R18, R30, A55).

#### Scenario: Machine with the encoding Utf8

- **GIVEN** the live machine `M14` with the string encoding `Utf8`, a fresh link state `ready_for_changeover`, and the approved version 1 of its recipe `R-300`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 1 of `R-300`
- **THEN** the use case rejects the request with `E-LNK-015`
- **AND** the system stores no `LoadRequest`

#### Scenario: Busy export gate

- **GIVEN** the approved version 3 of `R-100` on `M12` and every permit of `export.permits` held for longer than `timeout.export`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 3 of `R-100`
- **THEN** the system returns `503 E-WEB-009`
- **AND** the use case opens no transaction

#### Scenario: Retry of a known request while the export gate is busy

- **GIVEN** the request `L1` of version 3 of `R-100` on `M12` in the state `succeeded`, and every permit of `export.permits` held
- **WHEN** an Operator runs `RequestRecipeLoad` again with the id `L1`
- **THEN** the system returns the stored request `L1` with the state `succeeded`
- **AND** the use case takes no export permit

### Requirement: Retry of a load request

`RequestRecipeLoad` SHALL use the `load_request_id` of the load form as the id of the new request. A known id with the same machine, recipe, and version SHALL give the stored request with its state and SHALL write nothing. A known id with another machine, recipe, or version SHALL give `409 E-LNK-018`, also when two requests with one id run in parallel (R29, R31, A55).

#### Scenario: Retry after a lost answer

- **GIVEN** a `LoadRequest` of `M12` with the id `L1` that ended in the state `succeeded`
- **WHEN** the browser sends the load form with the `load_request_id` `L1` again
- **THEN** the system returns the stored request `L1`
- **AND** the audit log holds no new `RecipeLoadRequested` entry

#### Scenario: Known id with another version

- **GIVEN** a `LoadRequest` of `M12` with the id `L1` for the version 3 of `R-100`
- **WHEN** a client sends the load form of version 2 of `R-100` with the `load_request_id` `L1`
- **THEN** the use case rejects the request with `E-LNK-018`

#### Scenario: One id for two machines in parallel

- **GIVEN** the approved version 3 of `R-100` on `M12` and the approved version 1 of `R-200` on `M13`
- **WHEN** two Operators send the load forms of both versions with the same `load_request_id` `L1` at the same time
- **THEN** one request `L1` exists and the other answer is `409 E-LNK-018`
- **AND** the audit log holds one `RecipeLoadRequested` entry

### Requirement: One active load request per machine

A machine SHALL have at most one active `LoadRequest`. An active request is a request in the effective state `pending`, `claimed`, or `unknown`. `RequestRecipeLoad` SHALL refuse a second active request with `E-LNK-003`. A partial unique index on the stored states `pending` and `claimed` SHALL enforce the rule (R29, R31, A55).

#### Scenario: Second request

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` before its claim deadline
- **WHEN** a ProcessEngineer runs `RequestRecipeLoad` for `M12`
- **THEN** the use case rejects the request with `E-LNK-003`

#### Scenario: Request after an unknown result

- **GIVEN** a `LoadRequest` of `M12` in the stored state `claimed` after its result deadline
- **WHEN** an Operator runs `RequestRecipeLoad` for the approved version 3 of `R-100` on `M12`
- **THEN** the use case rejects the request with `E-LNK-003`
- **AND** the old request keeps the stored state `claimed`

#### Scenario: Direct insert of a second active row

- **GIVEN** a row of `load_request` of `M12` in the state `pending`
- **WHEN** a session of `recipe_app` inserts a second row of `M12` in the state `pending`
- **THEN** the database rejects the insert with SQLSTATE `23505`

### Requirement: Insert of a load request

The trigger `load_request_insert` SHALL refuse a new row that is not `pending` or that has a claimant, result, or release column with `E-LNK-031`. It SHALL refuse a version whose recipe belongs to another machine with `E-LNK-032` (R29, A55).

#### Scenario: Insert in a final state

- **GIVEN** the machine `M12` and the approved version 3 of `R-100`
- **WHEN** a session of `recipe_app` inserts a row of `load_request` of `M12` in the state `succeeded`
- **THEN** the database rejects the insert with the trigger code `E-LNK-031`

#### Scenario: Insert with a release marker

- **GIVEN** the machine `M12` and the approved version 3 of its recipe `R-100`
- **WHEN** a session of `recipe_app` inserts a `pending` row of `M12` with `released_at`
- **THEN** the database rejects the insert with the trigger code `E-LNK-031`

#### Scenario: Version of another machine

- **GIVEN** the machine `M12` and the recipe `R-200` of the machine `M13`
- **WHEN** a session of `recipe_app` inserts a `pending` row of `M12` for a version of `R-200`
- **THEN** the database rejects the insert with the trigger code `E-LNK-032`

#### Scenario: Valid insert

- **GIVEN** the machine `M12` and the approved version 3 of its recipe `R-100`
- **WHEN** a session of `recipe_app` inserts a `pending` row of `M12` for version 3 without a claimant column
- **THEN** the database stores the row

### Requirement: Transitions of a load request

The stored state of a `LoadRequest` SHALL change only by the transition table of ADR-0224. The table allows `pending` to `claimed`, `cancelled`, or `expired`, and `claimed` to `succeeded`, `failed`, or `released`. It allows `released` to `succeeded` or `failed`, and a new result deadline on a `claimed` row. Only `CancelRecipeLoad` and `ArchiveMachine` SHALL store `cancelled`. The trigger `load_request_transition` SHALL refuse every other transition with `E-LNK-030` (R29, R31, A55).

#### Scenario: Illegal transition

- **GIVEN** a row of `load_request` in the state `succeeded`
- **WHEN** a session of `recipe_app` sets the state to `pending`
- **THEN** the database rejects the update with the trigger code `E-LNK-030`

### Requirement: Column rules of a load request

The trigger `load_request_transition` SHALL refuse a change of an identity column with `E-LNK-033` and bad claimant or release columns with `E-LNK-034`. It SHALL refuse bad result columns with `E-LNK-035` and a bad lease with `E-LNK-036`. It SHALL refuse a change of a set claimant or release column with `E-LNK-037`. It SHALL check no deadline. `recipe_app` SHALL have no `UPDATE` grant on an identity column (R29, A55).

#### Scenario: Change of the version by the application role

- **GIVEN** a row of `load_request` in the state `pending`
- **WHEN** a session of `recipe_app` changes its `recipe_version_id`
- **THEN** the database rejects the update with SQLSTATE `42501`

#### Scenario: Change of the version by the table owner

- **GIVEN** a row of `load_request` in the state `pending`
- **WHEN** a session of the table owner changes its `recipe_version_id`
- **THEN** the database rejects the update with the trigger code `E-LNK-033`

#### Scenario: Claim without a claimant

- **GIVEN** a row of `load_request` in the state `pending`
- **WHEN** a session of `recipe_app` sets the state to `claimed` without `claimed_by`
- **THEN** the database rejects the update with the trigger code `E-LNK-034`

#### Scenario: Success without a hash

- **GIVEN** a row of `load_request` in the state `claimed`
- **WHEN** a session of `recipe_app` sets the state to `succeeded` without the reported hash
- **THEN** the database rejects the update with the trigger code `E-LNK-035`

#### Scenario: Earlier result deadline

- **GIVEN** a row of `load_request` in the state `claimed`
- **WHEN** a session of `recipe_app` sets an earlier `result_deadline`
- **THEN** the database rejects the update with the trigger code `E-LNK-036`

#### Scenario: Change of the claimant

- **GIVEN** a row of `load_request` in the state `claimed` by `plc-m12`
- **WHEN** a session of `recipe_app` sets `claimed_by` to the account `plc-m13`
- **THEN** the database rejects the update with the trigger code `E-LNK-037`

#### Scenario: Release without its actor

- **GIVEN** a row of `load_request` in the state `claimed`
- **WHEN** a session of `recipe_app` sets the state to `released` with `released_at` and without `released_by`
- **THEN** the database rejects the update with the trigger code `E-LNK-034`

#### Scenario: Late result that changes the release marker

- **GIVEN** a row of `load_request` in the state `released`
- **WHEN** a session of `recipe_app` sets the state to `failed` and changes `released_at`
- **THEN** the database rejects the update with the trigger code `E-LNK-037`

#### Scenario: Trigger code after the storage map

- **GIVEN** a row of `load_request` in the state `claimed`
- **WHEN** a session of `recipe_app` sets the state to `succeeded` without the reported hash
- **THEN** the error has SQLSTATE `23514` and the code `E-LNK-035` in the database
- **AND** `StorageError` and the HTTP answer keep the code `E-LNK-035` with the status `422`

### Requirement: Expiry of a load request

A request in the stored state `pending` after its claim deadline SHALL count as `expired`. A request in the stored state `claimed` after its result deadline SHALL count as `unknown` and SHALL stay active. The next `RequestRecipeLoad` or `ArchiveMachine` of the machine SHALL store `expired` for an overdue `pending` request and SHALL write one `RecipeLoadExpired` entry. No write SHALL store `unknown` (R20, R31, A55).

#### Scenario: Request after an expired request

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` after its claim deadline
- **WHEN** an Operator runs `RequestRecipeLoad` for the approved version 3 of `R-100` on `M12`
- **THEN** the old request has the state `expired`
- **AND** the system stores a new `pending` request
- **AND** the audit log holds one `RecipeLoadExpired` entry and one `RecipeLoadRequested` entry

#### Scenario: Claim after the deadline

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` after its claim deadline
- **WHEN** `plc-m12` sends `POST /adapter/v1/load-requests/{request_id}/claim` for it
- **THEN** the system returns `409 E-LNK-005`
- **AND** the request keeps the stored state `pending`

### Requirement: Effective state at each write

Every write of a load request SHALL read the row under its lock. It SHALL first compute the effective state with the start time of its transaction, also after a wait for a lock. A deadline SHALL count as passed when that time is equal to or later than the deadline (R29, R31, A55).

#### Scenario: Claim at the deadline

- **GIVEN** a `pending` request of `M12` whose claim deadline equals the time of the claim transaction
- **WHEN** `plc-m12` claims it
- **THEN** the system returns `409 E-LNK-005`

#### Scenario: Claim that waited for a lock

- **GIVEN** a `pending` request of `M12` and a claim of `plc-m12` that starts 0.5 seconds before the claim deadline
- **WHEN** the claim waits for a row lock until 0.5 seconds after the claim deadline
- **THEN** the request has the state `claimed` by `plc-m12`

#### Scenario: Cancel and claim in parallel

- **GIVEN** a `pending` request of `M12` before its claim deadline
- **WHEN** an Operator cancels it and `plc-m12` claims it at the same time
- **THEN** exactly one of the two writes changes the request
- **AND** the other write gets `409 E-LNK-006`

### Requirement: Claim of a load request

`ClaimLoadRequest` SHALL change a `pending` request before its claim deadline to `claimed` by the account. It SHALL set the result deadline and SHALL write one `RecipeLoadClaimed` entry. A repeated claim by the claimant SHALL give `200` with the stored claim and SHALL write nothing, also after the claim deadline (R20, R29, A55).

#### Scenario: First claim

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` before its claim deadline
- **WHEN** `plc-m12` claims it
- **THEN** the request has the state `claimed` by `plc-m12`
- **AND** the audit log holds one `RecipeLoadClaimed` entry with the actor `plc-m12`

#### Scenario: Repeated claim after the claim deadline

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12` after its claim deadline
- **WHEN** `plc-m12` claims it again
- **THEN** the system returns `200` with the claimed request
- **AND** the audit log holds no new entry

### Requirement: Refusal of a claim

`ClaimLoadRequest` SHALL refuse a request in another stored state than `pending` with `409 E-LNK-006`, except a request claimed by this account. It SHALL refuse a version that is no longer `Approved` with `409 E-LNK-014`. It SHALL NOT run the `loadable` check. A refusal SHALL change nothing (R29, R38, A55).

#### Scenario: Claim of a cancelled request

- **GIVEN** a `LoadRequest` in the state `cancelled`
- **WHEN** `plc-m12` claims it
- **THEN** the system returns `409 E-LNK-006`

#### Scenario: Version retired after the request

- **GIVEN** a `pending` request of `M12` for version 3 of `R-100` and a later `RetireVersion` of version 3
- **WHEN** `plc-m12` claims the request
- **THEN** the system returns `409 E-LNK-014`
- **AND** the request keeps the state `pending`

#### Scenario: Version that is not loadable after the request

- **GIVEN** a `pending` request of `M12` whose version 3 now has `loadable = false`
- **WHEN** `plc-m12` claims the request
- **THEN** the request has the state `claimed` by `plc-m12`

### Requirement: Load request id of an adapter route

A malformed or unknown `{request_id}` of an adapter route SHALL give `404 E-LNK-004` as the JSON error body. A malformed or unknown `{request_id}` of a UI route SHALL give `404 E-WEB-005` (R38, A55).

#### Scenario: Unknown id

- **GIVEN** no load request with the id `L9`
- **WHEN** `plc-m12` claims `L9`
- **THEN** the system returns `404 E-LNK-004` as the JSON error body

#### Scenario: Malformed id of a result

- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` sends `POST /adapter/v1/load-requests/abc/result`
- **THEN** the system returns `404 E-LNK-004` as the JSON error body

### Requirement: Result of a load request

`ReportLoadResult` SHALL accept a result only from the claiming account and only for a request in the stored state `claimed` or `released`. It SHALL store `succeeded` or `failed` and SHALL write one `RecipeLoadSucceeded` or `RecipeLoadFailed` entry. A `succeeded` result with another `content_sha256` than the request SHALL be stored as `failed` with the reason `hash_mismatch`. The system SHALL keep the received body and SHALL compare a repeated result with it (R20, R31, A55).

#### Scenario: Load with the same hash

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12` with the hash `H3`
- **WHEN** `plc-m12` sends the result `succeeded` with the hash `H3`
- **THEN** the request has the state `succeeded`
- **AND** the audit log holds one `RecipeLoadSucceeded` entry

#### Scenario: Load with another hash

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12` with the hash `H3`
- **WHEN** `plc-m12` sends the result `succeeded` with the hash `H2`
- **THEN** the request has the state `failed` with the reason `hash_mismatch`
- **AND** the answer gives the stored outcome `failed`

#### Scenario: Repeated result after a hash mismatch

- **GIVEN** a `LoadRequest` in the state `failed` with the reason `hash_mismatch` after the result `succeeded` with the hash `H2`
- **WHEN** `plc-m12` sends the result `succeeded` with the hash `H2` again
- **THEN** the system returns `200` with the stored outcome `failed`
- **AND** the audit log holds no new entry

#### Scenario: Late result after the release

- **GIVEN** a `LoadRequest` in the state `released` that `plc-m12` claimed
- **WHEN** `plc-m12` sends the result `failed` with the reason `interlock_refused` and `written` `none`
- **THEN** the request has the state `failed` with the reason `interlock_refused`
- **AND** the request keeps its release time

#### Scenario: Late result on an archived machine

- **GIVEN** a `LoadRequest` of `M12` in the state `released` that `plc-m12` claimed, and a later `ArchiveMachine` of `M12`
- **WHEN** `plc-m12` sends the result `failed` with the reason `adapter_restarted` and `written` `unknown`
- **THEN** the request has the state `failed`
- **AND** the audit log holds one `RecipeLoadFailed` entry

#### Scenario: Result of another account

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12`
- **WHEN** the `ExportClient` account `plc-m13` sends a result for it
- **THEN** the system returns `409 E-LNK-013`

#### Scenario: Result of a request that was never claimed

- **GIVEN** a `LoadRequest` in the state `pending`
- **WHEN** `plc-m12` sends a result for it
- **THEN** the system returns `409 E-LNK-008`

#### Scenario: Repeated result

- **GIVEN** a `LoadRequest` in the state `succeeded` after a result of `plc-m12` with the hash `H3`
- **WHEN** `plc-m12` sends the same result again
- **THEN** the system returns `200` with the stored outcome
- **AND** the audit log holds no new entry

#### Scenario: Different second result

- **GIVEN** a `LoadRequest` in the state `succeeded` after a result of `plc-m12`
- **WHEN** `plc-m12` sends the result `failed`
- **THEN** the system returns `409 E-LNK-007`

### Requirement: Result body without free text

A result body SHALL hold `outcome`. A `succeeded` result SHALL hold `content_sha256` of 64 lower-case hexadecimal digits. A `failed` result SHALL hold `reason` and `written` (`none`, `partial`, `all`, or `unknown`) and no `content_sha256`. It MAY hold `vendor_code`, an integer from 0 to 4294967295. The body SHALL hold no free text. The schema SHALL refuse the reason `hash_mismatch` (R38, A55).

#### Scenario: Failure with a text field

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12`
- **WHEN** `plc-m12` sends the result `failed` with the extra field `"detail": "password=secret"`
- **THEN** the system returns `422 E-LNK-009`
- **AND** the answer, the log, and the audit log do not hold the text `secret`

#### Scenario: Failure with a hash

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12`
- **WHEN** `plc-m12` sends the result `failed` with the reason `write_failed`, `written` `partial`, and a `content_sha256`
- **THEN** the system returns `422 E-LNK-009` with the path `$`

#### Scenario: Reason of the editor

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12`
- **WHEN** `plc-m12` sends the result `failed` with the reason `hash_mismatch`
- **THEN** the system returns `422 E-LNK-009` with the path `$.reason`

#### Scenario: Hash with a wrong length or character

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12`
- **WHEN** `plc-m12` sends the result `succeeded` with a hash of 63 digits, or with the character `G`
- **THEN** the system returns `422 E-LNK-009` with the path `$.content_sha256`
- **AND** the request stores no received body

#### Scenario: Restart after a partial write

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12` and an adapter that restarted during the write
- **WHEN** `plc-m12` sends the result `failed` with the reason `adapter_restarted` and `written` `unknown`
- **THEN** the request has the state `failed` with `written` `unknown`

#### Scenario: Bad credentials and a bad body

- **GIVEN** a `LoadRequest` in the state `claimed` by `plc-m12`
- **WHEN** a client sends a result with a wrong Basic password and a JSON syntax error
- **THEN** the system returns `401 E-IDN-003`

### Requirement: Cancel of a pending load request

`CancelRecipeLoad` SHALL check the permission `RecipeLoadRequest` first. It SHALL change a request in the effective state `pending` to `cancelled` and SHALL write one `RecipeLoadCancelled` entry. It SHALL refuse a request in another stored state with `E-LNK-006` and an overdue `pending` request with `E-LNK-005`. A repeated cancel of a `cancelled` request SHALL write nothing (R20, R28, A55).

#### Scenario: Cancel before the claim

- **GIVEN** a `LoadRequest` of `M12` in the state `pending`
- **WHEN** an Operator runs `CancelRecipeLoad` for it
- **THEN** the request has the state `cancelled`
- **AND** a status report of `M12` gets no pending request

#### Scenario: Cancel after the claim

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed`
- **WHEN** an Operator runs `CancelRecipeLoad` for it
- **THEN** the use case rejects the request with `E-LNK-006`

#### Scenario: Cancel of an expired request

- **GIVEN** a `LoadRequest` of `M12` in the stored state `pending` after its claim deadline
- **WHEN** an Operator runs `CancelRecipeLoad` for it
- **THEN** the use case rejects the request with `E-LNK-005`
- **AND** the request keeps the stored state `pending`

#### Scenario: Repeated cancel

- **GIVEN** a `LoadRequest` of `M12` in the state `cancelled`
- **WHEN** an Operator runs `CancelRecipeLoad` for it again
- **THEN** the use case succeeds
- **AND** the audit log holds no new entry

### Requirement: Release of a load request with an unknown result

`ReleaseRecipeLoad` SHALL check the permission `RecipeLoadRequest` first. It SHALL change a request in the effective state `unknown` to `released`, store the release time and the actor, and write one `RecipeLoadReleased` entry. A request with a release time SHALL give success and write nothing, also after a late result. Any other request outside `unknown` SHALL give `409 E-LNK-017` (R20, R28, R31, A55).

#### Scenario: Release after a check of the machine

- **GIVEN** a `LoadRequest` of `M12` in the stored state `claimed` after its result deadline
- **WHEN** an Operator runs `ReleaseRecipeLoad` for it
- **THEN** the request has the state `released`
- **AND** the audit log holds one `RecipeLoadReleased` entry
- **AND** `RequestRecipeLoad` for `M12` no longer gets `E-LNK-003`

#### Scenario: Release of a running claim

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed` before its result deadline
- **WHEN** an Operator runs `ReleaseRecipeLoad` for it
- **THEN** the use case rejects the request with `E-LNK-017`

#### Scenario: Repeated release

- **GIVEN** a `LoadRequest` of `M12` in the state `released`
- **WHEN** an Operator runs `ReleaseRecipeLoad` for it again
- **THEN** the use case succeeds
- **AND** the audit log holds no new entry

#### Scenario: Repeated release after a lost answer and a late result

- **GIVEN** the request `L1` of `M12` that an Operator released, whose answer got lost, and that a late result then changed to `succeeded`
- **AND** a newer `pending` request `L2` of `M12`
- **WHEN** the Operator runs `ReleaseRecipeLoad` for `L1` again
- **THEN** the use case succeeds
- **AND** `L1` keeps the state `succeeded`, `L2` keeps the state `pending`, and the audit log holds no new entry

### Requirement: Audit entries of a load request

Each transition of a load request SHALL write one entry with the aggregate type `machine` and the entity type `load_request`. The entry SHALL store the full row. `RecipeLoadRequested` SHALL store only `after_state`. Every other action SHALL store `before_state` and `after_state`. The system SHALL use seven actions that start with `RecipeLoad`. No state object SHALL hold a secret (R20, A55).

#### Scenario: Names of the actions

- **GIVEN** the list of the audit actions
- **WHEN** the action test reads the actions of a load request
- **THEN** they are `RecipeLoadRequested`, `RecipeLoadExpired`, `RecipeLoadCancelled`, `RecipeLoadReleased`, `RecipeLoadClaimed`, `RecipeLoadSucceeded`, and `RecipeLoadFailed`

#### Scenario: Entry of a claim

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` before its claim deadline
- **WHEN** `plc-m12` claims it
- **THEN** the `RecipeLoadClaimed` entry has a `before_state` with the full row in the state `pending`
- **AND** its `after_state` holds the full row with the state `claimed`, the id of `plc-m12` as the claimant, and the result deadline

#### Scenario: Two entries of one commit

- **GIVEN** a `LoadRequest` of `M12` in the state `pending` after its claim deadline
- **WHEN** an Operator runs `RequestRecipeLoad` for `M12`
- **THEN** the `RecipeLoadExpired` entry has the `after_state` `expired`
- **AND** the `RecipeLoadRequested` entry has no `before_state` and the `after_state` `pending`

#### Scenario: Entry of a hash mismatch

- **GIVEN** a `LoadRequest` of `M12` in the state `claimed` by `plc-m12` with the hash `H3`
- **WHEN** `plc-m12` sends the result `succeeded` with the hash `H2`
- **THEN** the `RecipeLoadFailed` entry has the full `claimed` row in `before_state`
- **AND** its `after_state` holds the state `failed`, the reason `hash_mismatch`, and the received hash `H2`

#### Scenario: Entry of a release

- **GIVEN** a `LoadRequest` of `M12` in the stored state `claimed` after its result deadline
- **WHEN** an Operator runs `ReleaseRecipeLoad` for it
- **THEN** the `RecipeLoadReleased` entry has the full `claimed` row in `before_state`
- **AND** its `after_state` holds the state `released`, the release time, and the id of the Operator

### Requirement: Adapter link in read-only mode

In read-only mode, the system SHALL apply status reports with their lease and load results. It SHALL write the audit entry of each result. `RequestRecipeLoad`, `CancelRecipeLoad`, `ReleaseRecipeLoad`, and `ClaimLoadRequest` SHALL return `503 E-WEB-006`. They SHALL change no load request and SHALL write no audit entry (A24, A55).

#### Scenario: Report with a full disk

- **GIVEN** the application in read-only mode and the live machine `M12`
- **WHEN** `plc-m12` sends a status report for `M12`
- **THEN** the system stores the link state of `M12`

#### Scenario: Claim with a full disk

- **GIVEN** the application in read-only mode and a `pending` request of `M12`
- **WHEN** `plc-m12` claims it
- **THEN** the system returns `503 E-WEB-006`
- **AND** the request keeps the state `pending`
- **AND** the system runs no password check

#### Scenario: Result with a full disk

- **GIVEN** the application in read-only mode and a request of `M12` in the state `claimed` by `plc-m12`
- **WHEN** `plc-m12` sends the result `succeeded` with the hash of the request
- **THEN** the request has the state `succeeded`
- **AND** the audit log holds one `RecipeLoadSucceeded` entry

#### Scenario: Release with a full disk

- **GIVEN** the application in read-only mode and a request of `M12` in the stored state `claimed` after its result deadline
- **WHEN** an Operator runs `ReleaseRecipeLoad` for it
- **THEN** the use case rejects the command with `E-WEB-006`
- **AND** the request keeps the stored state `claimed`

### Requirement: Failed write of a load request

A `RequestRecipeLoad`, `CancelRecipeLoad`, `ReleaseRecipeLoad`, `ClaimLoadRequest`, `ReportLoadResult`, or `ArchiveMachine` that fails after its first write SHALL leave no partial write (R29).

#### Scenario: Audit insert fails

- **GIVEN** a `pending` request of `M12` and an audit insert that fails
- **WHEN** `plc-m12` claims the request
- **THEN** the request keeps the state `pending`
- **AND** the audit log holds no new entry

#### Scenario: Audit insert of an archive fails

- **GIVEN** the live machine `M12` with a `pending` request, before or after its claim deadline, and an audit insert that fails
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** `M12` stays live
- **AND** the request keeps the stored state `pending`
