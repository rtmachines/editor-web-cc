# Reliability

Status: Accepted (design stage, 2026-10-12)
Source of requirements: `general-draft.txt` (R29 to R38, R42)
Related: `overview.md`, `domain-model.md`, `data-model.md`, `api.md`, ADR-0003, ADR-0008

This document specifies how the system stays correct and available during continuous 24/7 operation. Each section names the rule and the mechanism that enforces it.

## 1. Error handling

### 1.1 Error types per crate

Each library crate owns its error types. The binary crate uses `anyhow`.

| Crate | Error type | Shape | Wraps |
|---|---|---|---|
| `recipe-domain` | `PlantError`, `RecipeError`, `IdentityError` | One `thiserror` enum per bounded context | Nothing. Domain errors are leaf errors |
| `recipe-app` | `AppError` | `thiserror` enum | `#[from] PlantError`, `#[from] RecipeError`, `#[from] IdentityError`, plus `Forbidden`, `NotFound`, `ConcurrentEdit`, `Storage(#[source] Box<dyn Error + Send + Sync>)` |
| `recipe-db` | `DbError` | `thiserror` enum | `sqlx::Error` with the SQLSTATE preserved in a field. The adapter maps a `DbError` to an `AppError` at the port boundary |
| `recipe-export` | `ExportError` | `thiserror` enum | `serde_json::Error` and `RecipeError` |
| `recipe-web` | `WebError` | `thiserror` enum | `#[from] AppError`, plus `BadRequest`, `Unauthenticated`, `PayloadTooLarge`, `Timeout` |
| `recipe-server` | `anyhow::Error` | Dynamic with context | Everything, only in `main()`, configuration loading, and wiring |

Rules:

- Every error variant carries the values that identify the failure (recipe reference, version number, tag, leaf path). It never carries a password, a password hash, a session id, or a cookie value.
- Every error type implements `Display` for the user and `Debug` for the log. `Debug` on types that hold secrets redacts them.
- Every error type has a `code(&self) -> ErrorCode` method. The code is stable. The message template can change. The code cannot.
- `?` converts between layers through `From`. A `map_err` closure appears only when it adds context.
- The crate `recipe-db` splits `sqlx::Error` by what the caller does with it, not by where it occurred. SQLSTATE `23505` becomes `DbError::Duplicate { constraint }`. SQLSTATE `23503` becomes `DbError::MissingReference { constraint }`. SQLSTATE `23514` becomes `DbError::CheckViolation { constraint }`. SQLSTATE `40001` and `40P01` become `DbError::SerializationRetry`. SQLSTATE `57014` becomes `DbError::StatementTimeout`. Any other code becomes `DbError::Other { sqlstate, source }`.
- A `DbError::SerializationRetry` is retried by the unit of work at most 3 times with a delay of 50 ms, 100 ms, 200 ms. After the third failure the use case returns `AppError::Storage`.
- An error is logged exactly once. The log statement is in the `WebError` to HTTP mapping in `recipe-web`. Lower layers add context and propagate. They do not log the error.
- The HTTP response contains the error code, the user message, and the `request_id`. It does not contain the `Debug` output, a stack trace, or an SQL fragment.

### 1.2 Error code scheme

Format: `E-<CTX>-<nnn>`. `CTX` is one of `PLT` (Plant Model), `RCP` (Recipe Management), `IDN` (Identity and Access), `AUD` (Audit), `EXP` (Export), `DB` (storage adapter), `WEB` (HTTP layer). `nnn` is a three-digit number. A number is never reused after a variant is removed.

| Code | Variant | HTTP | User message template (English, STE) |
|---|---|---|---|
| `E-PLT-001` | `PlantError::DuplicateMachineCode { code }` | 409 | Machine code {code} is already in use. Use a different code. |
| `E-PLT-002` | `PlantError::UnitDepthExceeded { path }` | 422 | Unit path {path} has more than 6 levels. Use at most 6 levels. |
| `E-PLT-003` | `PlantError::ParentNotFound { path }` | 422 | The parent unit of {path} does not exist. Create the parent unit first. |
| `E-PLT-004` | `PlantError::DuplicateUnitPath { path }` | 409 | Unit {path} already exists on this machine. |
| `E-PLT-005` | `PlantError::TypeCycle { type_name }` | 422 | Type {type_name} contains itself. Remove the circular member. |
| `E-PLT-006` | `PlantError::MemberTypeNotOnPlatform { member, platform }` | 422 | Member {member} uses a type that does not exist on {platform}. |
| `E-PLT-007` | `PlantError::PlatformLocked { code }` | 409 | Machine {code} has recipes. The PLC platform cannot change. |
| `E-RCP-001` | `RecipeError::DuplicateReference { reference }` | 409 | Reference number {reference} is already in use. |
| `E-RCP-002` | `RecipeError::VersionNotEditable { reference, number, status }` | 409 | Version {number} of recipe {reference} is {status}. Create a new draft to make changes. |
| `E-RCP-003` | `RecipeError::S7AddressRequired { tag }` | 422 | Parameter {tag} needs a PLC address. The machine uses an S7-300 controller. |
| `E-RCP-004` | `RecipeError::ValueOutOfLimit { tag, leaf, value, min, max }` | 422 | Value {value} of {tag}{leaf} is outside the process limit {min} to {max}. |
| `E-RCP-005` | `RecipeError::ValueShapeMismatch { tag, leaf, expected }` | 422 | Value of {tag}{leaf} does not match the type. Expected {expected}. |
| `E-RCP-006` | `RecipeError::LimitOutsideTypeRange { tag, leaf }` | 422 | Process limit of {tag}{leaf} is outside the range of the data type. |
| `E-RCP-007` | `RecipeError::VersionNotDraft { reference, number }` | 409 | Version {number} of recipe {reference} is not a draft. |
| `E-RCP-008` | `RecipeError::DraftHasNoParameters { reference, number }` | 422 | Version {number} of recipe {reference} has no parameters. Add at least one parameter before approval. |
| `E-RCP-009` | `RecipeError::VersionNotApproved { reference, number }` | 409 | Version {number} of recipe {reference} is not approved. |
| `E-RCP-010` | `RecipeError::RecipeHasApprovedVersion { reference }` | 409 | Recipe {reference} has an approved version. Retire the version before you delete the recipe. |
| `E-RCP-011` | `RecipeError::DraftAlreadyExists { reference, number }` | 409 | Recipe {reference} already has draft version {number}. Edit or discard that draft. |
| `E-RCP-012` | `RecipeError::RecipeDeleted { reference }` | 410 | Recipe {reference} was deleted. |
| `E-RCP-013` | `RecipeError::TypeNotOnPlatform { tag, type_name, platform }` | 422 | Parameter {tag} uses type {type_name}. This type does not exist on {platform}. |
| `E-RCP-014` | `RecipeError::DuplicateTag { tag }` | 409 | Parameter tag {tag} is already used in this version. |
| `E-IDN-001` | `IdentityError::DuplicateLogin { login }` | 409 | Login {login} is already in use. |
| `E-IDN-002` | `IdentityError::LastAdministrator` | 409 | This user is the last administrator. Add another administrator first. |
| `E-IDN-003` | `IdentityError::InvalidCredentials` | 401 | Login or password is not correct. |
| `E-IDN-004` | `IdentityError::UserInactive` | 403 | This user account is not active. |
| `E-AUD-001` | `AppError::AuditWriteFailed` | 500 | The change was not saved. The system could not record the audit entry. Contact the administrator. Reference: {request_id}. |
| `E-EXP-001` | `ExportError::VersionNotApproved { reference, number }` | 409 | Only an approved version can be exported. |
| `E-DB-001` | `DbError::SerializationRetry` (after 3 retries) | 503 | The system is busy. Try again. Reference: {request_id}. |
| `E-DB-002` | `DbError::StatementTimeout` | 503 | The operation took too long and was stopped. Try again. Reference: {request_id}. |
| `E-WEB-001` | `WebError::ConcurrentEdit { reference, number }` | 409 | Another user changed version {number} of recipe {reference}. Reload the page and apply your change again. |
| `E-WEB-002` | `WebError::Forbidden { operation }` | 403 | Your role does not permit the operation {operation}. |
| `E-WEB-003` | `WebError::PayloadTooLarge { limit }` | 413 | The request is larger than {limit}. |
| `E-WEB-004` | `WebError::Unauthenticated` | 401 | Log in to continue. |
| `E-WEB-005` | `WebError::NotFound { entity }` | 404 | {entity} does not exist. |

The full code list lives in `crates/recipe-domain/src/error_codes.md` after implementation. A test checks that every variant has a unique code.

Reserved ranges for database triggers (`data-model.md` section 4): `E-PLT-010` to `E-PLT-029` and `E-RCP-030` to `E-RCP-039`. `recipe-db` maps a `RAISE EXCEPTION` message that starts with one of these codes to `DbError::Constraint { code, message }`. The HTTP layer returns 409 for them. A trigger fires only when the domain check was bypassed, so this is a defence in depth, not the first line.

`E-WEB-006` is the read-only mode of section 7.9 (Assumption A24).

### 1.3 Reference shape

```rust
#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("reference number {reference} is already in use")]
    DuplicateReference { reference: ReferenceNumber },
    #[error("version {number} of recipe {reference} is {status}")]
    VersionNotEditable { reference: ReferenceNumber, number: u32, status: VersionStatus },
    #[error("parameter {tag} needs a PLC address on an S7-300 machine")]
    S7AddressRequired { tag: ParameterTag },
    // ... one variant per row in 1.2
}

impl RecipeError {
    pub fn code(&self) -> ErrorCode;
}
```

## 2. No-panic policy and lints

### 2.1 Workspace lints

Every crate has `#![forbid(unsafe_code)]`. The workspace `Cargo.toml` sets these lints. Every crate inherits them with `[lints] workspace = true`.

| Lint | Level | Reason |
|---|---|---|
| `clippy::unwrap_used` | deny | An `unwrap` on input-derived data is a panic path (R32) |
| `clippy::expect_used` | deny | Same as above. The one exception is in 2.3 |
| `clippy::panic` | deny | No explicit `panic!` in application code |
| `clippy::unreachable` | deny | `unreachable!` is a panic path |
| `clippy::todo`, `clippy::unimplemented` | deny | Placeholders do not ship |
| `clippy::indexing_slicing` | deny | `v[i]` panics on a bad index. Use `get` |
| `clippy::arithmetic_side_effects` | deny | `a + b` on integers can overflow. See 2.2 |
| `clippy::cast_possible_truncation`, `clippy::cast_sign_loss`, `clippy::cast_possible_wrap`, `clippy::cast_precision_loss` | deny | An `as` cast silently changes the value |
| `clippy::exit` | deny | Only `main` ends the process |
| `clippy::print_stdout`, `clippy::print_stderr` | deny | All output goes through `tracing` |
| `clippy::large_futures` | warn | Keeps task stacks small |
| `unused_must_use` | deny | A dropped `Result` hides an error |

The `[profile.release]` section sets `overflow-checks = true`. An overflow in release is a panic with a clear message instead of a wrong value. Section 2.2 makes such a panic unreachable in application code.

### 2.2 Rules for code that cannot panic

| Situation | Rule |
|---|---|
| Integer arithmetic | Use `checked_add`, `checked_sub`, `checked_mul`, `checked_div`. Return `Err` on `None`. Use `saturating_*` only for display counters where a clamp is correct |
| Integer conversion | Use `u32::try_from(x)?` and `i64::try_from(x)?`. Never `as` between integer types in application code |
| Float to integer | Not needed in the domain. Values are `rust_decimal::Decimal`. If a conversion is needed, use `Decimal::to_i64()` and handle `None` |
| Decimal arithmetic | Use `checked_add`, `checked_mul`, `checked_div` of `rust_decimal`. A limit check uses comparison only. Comparison cannot fail |
| Slice and `Vec` access | Use `get(i)`, `first()`, `last()`, `chunks`, iterators. Never `v[i]` |
| `Option` | Use `let Some(x) = opt else { return Err(...) };` or `opt.ok_or(Error::...)?` |
| `HashMap` | Use `get` and `entry`. Never `map[&key]` |
| String slicing | Use `char_indices`, `get(a..b)`, or `split`. Never `s[a..b]` |
| Division | Check the divisor before the operation or use `checked_div` |
| Recursion on type trees | Depth is bounded by the `TypeTree` validation (section 4.3). Resolution uses an explicit stack, not recursion |
| Regular expressions | Compiled once with `LazyLock`. The pattern is a literal and is tested. A compile failure is a test failure, not a runtime panic |
| `Mutex` lock | Use `lock().map_err(...)?` or `parking_lot::Mutex` whose `lock()` cannot return an error |
| Tokio `JoinHandle` | `handle.await` returns `Result<T, JoinError>`. Map `JoinError` to `AppError::Storage` or `WebError::Internal`. Never `unwrap` it |
| Blocking inside async | Never `block_on` inside an async context. CPU-heavy work (Argon2id hashing) runs in `spawn_blocking` |
| Time | `SystemTime::now().duration_since(UNIX_EPOCH)` returns `Result`. Use the `time` crate `OffsetDateTime::now_utc()` which does not fail |

### 2.3 Permitted exceptions

- Test code (`#[cfg(test)]` and `tests/`) may use `unwrap`, `expect`, and indexing. The lint is set to `allow` for test targets.
- `recipe-server::main` may use `expect` for a startup precondition such as "configuration file is readable". The message names the precondition. The process exits before it serves any request.
- No other exception. A pull request that adds `#[allow(clippy::unwrap_used)]` outside tests is rejected in review.

### 2.4 Panic safety net

A panic in application code is a bug, not an error. The system treats it as fail-fast:

1. `recipe-server::main` installs a panic hook with `std::panic::set_hook`. The hook writes one `tracing::error!` event with the panic message, the location, and the current `request_id` when a span is active.
2. The hook then calls `std::process::abort()`. The process stops. Docker restarts the container because of `restart: always` (section 7).
3. The code never calls `std::panic::catch_unwind`. A caught panic would let the process continue in a state the type system did not promise. A restart from a known state is safer.
4. The `[profile.release]` section sets `panic = "abort"`. This removes unwinding tables and makes the behavior in rule 2 the only behavior.
5. An open transaction at the time of the abort is rolled back by PostgreSQL. No partial write survives (R29).

The restart loop is a visible signal. An alert fires when the container restarts more than 3 times in 10 minutes (section 7.9).

## 3. Structured logging and tracing

### 3.1 Stack

- `tracing` for spans and events in all crates.
- `tracing-subscriber` with the JSON formatter. Output goes to stdout. Docker collects it.
- `tower-http::trace::TraceLayer` creates the request span.
- One line per event. One JSON object per line.

### 3.2 Span fields

| Span | Fields |
|---|---|
| `http.request` | `request_id` (UUID v7, generated by the server, returned in the `X-Request-Id` response header), `method`, `route` (the pattern, for example `/recipes/{ref}/versions/{n}`, not the concrete path), `user_id`, `role`, `status`, `latency_ms`, `error_code` when the result is an error |
| `use_case` | `name` (for example `ApproveVersion`), `recipe_ref`, `version`, `machine_code`, `outcome` (`ok` or the error code) |
| `db.statement` | `name` (a label such as `recipe_version.update_status`), `rows_affected`, `latency_ms`. Never the SQL text with bound values |
| `uow.transaction` | `isolation`, `attempt` (retry counter), `outcome` |

The `request_id` of the HTTP span is inherited by all child spans. A log line without a `request_id` comes from startup, shutdown, or a background job. Those lines carry a `job` field instead.

### 3.3 Level policy

| Level | Use |
|---|---|
| `ERROR` | The system could not do what was asked because of an internal failure: storage error, audit write failure, panic hook, startup failure. Each `ERROR` line is actionable |
| `WARN` | Degraded but working: serialization retry, slow query (> 1 s), pool near exhaustion, disk space below the warning threshold, failed login |
| `INFO` | One line per request (from `TraceLayer`), one line per use case outcome, startup and shutdown, migration applied, backup completed |
| `DEBUG` | SQL statement names and timings, cache of type trees, template rendering |
| `TRACE` | Not used in production. Available for local debugging |

A validation error from a user (HTTP 4xx) is `INFO`, not `WARN`. It is a normal outcome. A 5xx is `ERROR`.

Production runs at `INFO` for all crates and `WARN` for `sqlx` and `hyper`. The level is set by the `RUST_LOG` variable and can change without a rebuild.

### 3.4 Redaction

Never written to the log:

- passwords and password hashes
- session identifiers and cookie headers
- the `Authorization` header
- the full request body of the login form

The `Debug` implementation of `PasswordHash`, `SessionId`, and `LoginForm` prints `[redacted]`. The `TraceLayer` is configured to skip the `Cookie` and `Set-Cookie` headers.

Parameter values and process limits are not secret. The use case span may include them at `DEBUG`.

### 3.5 Retention

Docker uses the `json-file` log driver with `max-size: 50m` and `max-file: 20` per container. This keeps about 1 GB of logs per container. Older lines are discarded. A site that needs longer retention forwards stdout to its own collector. The application does not write log files.

### 3.6 Audit log versus event log

| | Audit log | Event log |
|---|---|---|
| Location | PostgreSQL table `audit_log` | Container stdout |
| Content | Who changed what, before and after values | How the system behaved: latencies, errors, retries |
| Written | In the same transaction as the change (ADR-0005) | Asynchronously, best effort |
| Retention | Forever. Part of the backup | 1 GB rolling window |
| Reader | Administrator through the UI | Operations staff through Docker tools |

A business question ("who approved version 7?") is answered by the audit log. An operations question ("why was the approval slow?") is answered by the event log. The `request_id` is present in both and links them.

## 4. Input validation

### 4.1 Three layers

| Layer | Where | What it rejects |
|---|---|---|
| (a) HTTP | `recipe-web` extractors | Wrong shape, wrong encoding, too large, unknown fields |
| (b) Domain | `recipe-domain` value object constructors | Values that violate a domain rule |
| (c) Database | `CHECK`, `UNIQUE`, FK, triggers (`data-model.md`) | Anything that passes (a) and (b) because of a bug |

A request that passes layer (a) is well-formed. A value that passes layer (b) is valid. Layer (c) exists so that a bug in (a) or (b) produces an error instead of corrupt data.

### 4.2 HTTP layer rules

- The body limit is 64 KiB for HTML forms and 1 MiB for the JSON body of a parameter value. The server returns `E-WEB-003` above the limit.
- All JSON types derive `serde::Deserialize` with `#[serde(deny_unknown_fields)]`.
- Form parsing uses `axum::Form` with `serde` structs. A missing required field is a 422 with the field name.
- A path parameter is parsed directly into the value object (`ReferenceNumber`, `MachineCode`, `UnitPath`). A bad path parameter is a 404, not a 500.
- The `Content-Type` of a request must match the route. A form route rejects JSON and a JSON route rejects a form.
- Text fields are trimmed and normalized to Unicode NFC before layer (b).
- The `row_version` of a draft is a hidden form field. A missing `row_version` on a mutating request is a 422.

### 4.3 Domain value objects

Each value object has one constructor `parse(input) -> Result<Self, Error>` and no public `new`. A value object instance is valid by construction.

| Value object | Grammar or rule |
|---|---|
| `MachineCode` | `^M[1-9][0-9]{0,6}$`, number `1..=9999999` |
| `UnitPath` | `^U([1-9]|[1-5][0-9]|6[0-4])(\.([1-9]|[1-5][0-9]|6[0-4])){0,5}$`, 1 to 6 segments, each `1..=64` |
| `ReferenceNumber` | `^[A-Z0-9][A-Z0-9_-]{0,49}$` after uppercasing |
| `ParameterTag` | `^[A-Z][A-Z0-9_]{0,63}$` |
| `Login` | `^[a-z0-9._-]{3,64}$` after lowercasing |
| `Text` (names) | Not empty after trim, at most 200 chars, no control characters |
| `Text` (descriptions, change notes) | At most 4000 chars, no control characters except newline |
| `LangCode` | BCP 47 primary tag plus optional region: `^[a-z]{2,3}(-[A-Z]{2})?$` |
| `S7Address` | `^DB([1-9][0-9]{0,4})\.DB([XBWD])([0-9]{1,5})(\.([0-7]))?$`, db `1..=65535`, offset `0..=65535`, bit present if and only if width is `X`. Width must match the type size |
| `TypeName` (UDT) | `^[A-Za-z_][A-Za-z0-9_]{0,63}$`, not a reserved primitive name |
| `ArrayDimension` | `lower <= upper`, `upper - lower < 65536` |
| `LeafPath` | `^(\.[A-Za-z_][A-Za-z0-9_]*|\[[0-9-]+(,[0-9-]+)*\])*$`, resolves against the `TypeTree` |
| `ProcessLimit` | `min <= max` when both set. Inside the type range. `max_length <= string_length` |
| `Password` (input only) | 12 to 128 chars. Hashed at once. The plain text is dropped after hashing |

### 4.4 Parameter value JSON

- The JSON is parsed by `serde_json` with the `arbitrary_precision` feature. A number stays a decimal string inside `serde_json::Number`. There is no `f64` round trip. `REAL` and `LREAL` values keep the digits the user typed.
- The domain converts each numeric leaf to `rust_decimal::Decimal` for range and limit checks. A number that does not fit in `Decimal` (more than 28 significant digits) is rejected with `E-RCP-005`.
- `REAL` and `LREAL` leaves are additionally checked to fit the IEEE 754 single or double range. The stored text is the decimal the user typed. The export writes the same text.
- The JSON parser enforces a depth limit of 32. A `TypeTree` deeper than 32 levels is rejected when the UDT is defined, so a valid value never reaches that limit.
- A value has at most 65 536 scalar leaves. The `TypeTree` resolution counts leaves and rejects a type above that count at UDT definition time.
- The shape check is exact: an array has exactly `(upper - lower + 1)` elements per dimension. A struct has exactly the member names. No extra key, no missing key.
- `STRING` values are checked for byte length against `string_length` in the target encoding (Latin-1 for `STRING`, UTF-16 units for `WSTRING`).
- `BOOL` accepts only JSON `true` and `false`. Not `0`, `1`, or `"true"`.
- Time types (`TIME`, `LTIME`, `S5TIME`, `DATE`, `TOD`, `DT`, `DTL`) are stored as ISO 8601 strings and parsed with the `time` crate. The domain checks the platform range (for example `S5TIME` up to `2h46m30s`).

## 5. Migrations and schema evolution

- Migrations are SQL files in `crates/recipe-db/migrations/` managed by `sqlx::migrate!`. The file name is `<YYYYMMDDHHMMSS>_<slug>.sql`.
- Migrations are forward-only. There is no `down` file. A rollback is a new forward migration or a restore from backup.
- A migration that shipped is never edited. `sqlx` stores a checksum of each applied migration in `_sqlx_migrations`. A changed checksum stops the migrator with an error.
- The server binary has a `migrate` subcommand: `recipe-server migrate`. The Docker Compose file runs it as a one-shot service `migrate` with `depends_on: db` (condition `service_healthy`). The `app` service depends on `migrate` (condition `service_completed_successfully`). The server itself does not run migrations.
- The migrator connects as the PostgreSQL role `recipe_migrator`, which owns the schema. The server connects as `recipe_app`, which has only the grants listed in `data-model.md`. A bug in the server cannot alter the schema.
- The migrator takes an advisory lock. Two migrators cannot run at the same time.
- A breaking change uses the expand-and-contract pattern across releases: release N adds the new column or table and writes both. Release N+1 reads the new one. Release N+2 removes the old one.
- The backup job runs a `pg_dump` immediately before the `migrate` service starts (section 7.7). The upgrade procedure in 7.10 depends on this dump.
- Seed data (primitive `plc_type` rows, the first administrator) is a migration. The first administrator password comes from an environment variable read once by the migration subcommand and is never written to a file.

## 6. Testing strategy

| Level | Scope | Tooling | Rule |
|---|---|---|---|
| Unit, domain | `recipe-domain` | `cargo test`, no I/O | Every invariant in `domain-model.md` section 3 has at least one passing and one failing test. Every error variant is produced by at least one test |
| Unit, use case | `recipe-app` | In-memory implementations of all ports | Every command in `domain-model.md` section 5 has tests for success, each listed error, and the audit entry it writes |
| Permission matrix | `recipe-app` | Generated test from the matrix in `api.md` section 3 | One test per (operation, role) pair. The test asserts `Ok` or `AppError::Forbidden`. A new operation without a matrix row fails the build |
| Property | `recipe-domain` | `proptest` | `UnitPath`, `S7Address`, `ReferenceNumber` round-trip through `parse` and `Display`. `ParameterValue::validate` against random `TypeTree`s up to depth 6 with random limits: a generated valid value passes, a mutated value fails |
| Integration, storage | `recipe-db` | `testcontainers` with `postgres:16`, or `DATABASE_URL` of a Compose database in CI | Every `CHECK`, `UNIQUE`, FK, partial index, and trigger in `data-model.md` has a test that violates it and asserts the SQLSTATE. Every transactional operation in `data-model.md` section 6 has a test that fails midway and asserts no partial write |
| Concurrency | `recipe-db` | Two connections | Two concurrent approvals of different drafts of one recipe: exactly one succeeds. Two edits with the same `row_version`: exactly one succeeds |
| HTTP | `recipe-web` | `tower::ServiceExt::oneshot` with in-memory ports | Every route: status code, error code in the body, `X-Request-Id` header, no secret in the body. Login, logout, session expiry |
| End to end | Compose stack | `playwright-cli` smoke suite | Create machine, create recipe, add parameter, approve, export, verify the audit page |
| Soak | Compose stack, nightly CI | Fixed request mix for 24 h at a constant rate | Memory of the `app` container stays under a ceiling (256 MiB). No `ERROR` line. No restart. p99 latency stays under 500 ms |
| Lint gate | Workspace | `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` | Runs on every pull request |

Coverage target: 80 % line coverage per crate, measured with `cargo llvm-cov`. The CI job fails below the target.

## 7. Operations for 24/7

### 7.1 Compose layout

| Service | Image | Role | Restart | Healthcheck |
|---|---|---|---|---|
| `proxy` | `caddy` | TLS termination, HTTP to HTTPS redirect (Assumption A18) | `always` | HTTP `GET /healthz` through the proxy |
| `app` | `recipe-server` | The application | `always` | `GET /readyz` every 10 s, 3 failures mark it unhealthy |
| `migrate` | `recipe-server` with `migrate` | One-shot schema migration | `no` | Exit code |
| `db` | `postgres:16` | Database | `always` | `pg_isready` every 10 s |
| `backup` | `postgres:16` with a cron script | Daily `pg_dump`. The same image runs `backup-now` for a one-shot dump | `always` | File age of the latest dump < 26 h |

Start order: `db` healthy, then `migrate` completed, then `app`, then `proxy`.

### 7.2 Health endpoints

| Endpoint | Checks | Use |
|---|---|---|
| `GET /healthz` | Process is up and the router answers | Liveness. Docker restarts the container on failure |
| `GET /readyz` | A pooled connection can run `SELECT 1` within 2 s. Free disk space on the data volume is above the hard threshold (7.8) | Readiness. The proxy returns 503 when not ready |

Both endpoints need no session. Both return JSON with `status` and `checks`. Neither returns version strings or hostnames.

### 7.3 Graceful shutdown

1. The process receives `SIGTERM` from Docker.
2. The listener stops accepting new connections. The readiness endpoint returns 503.
3. Requests in flight continue. Open transactions commit or roll back as their use case finishes.
4. After all requests finish, or after 30 s, the pool closes and the process exits with code 0.
5. Docker `stop_grace_period` is 40 s so that step 4 completes before `SIGKILL`.

### 7.4 Timeouts

| Where | Value | Mechanism |
|---|---|---|
| HTTP request total | 30 s | `tower-http::timeout::TimeoutLayer`. Returns 503 with `E-DB-002` style message |
| Request body read | 10 s | `hyper` header and body read timeouts |
| SQL statement | 10 s | `statement_timeout = 10s` set per connection by the pool `after_connect` hook |
| Idle in transaction | 15 s | `idle_in_transaction_session_timeout = 15s` per connection |
| Pool acquire | 5 s | `sqlx::PoolOptions::acquire_timeout` |
| Pool idle connection | 10 min | `idle_timeout` |
| Pool max lifetime | 30 min | `max_lifetime`. Avoids stale connections after a database restart |
| Argon2id hashing | 2 s budget | Parameters tuned to about 250 ms. Runs in `spawn_blocking` |
| Export generation | 10 s | Timeout around the use case |

### 7.5 Connection pool

Pool size is 10 connections for the application and 2 for health checks. PostgreSQL `max_connections` is 50. The pool tests a connection before use with `test_before_acquire = true`. A pool exhaustion wait above 1 s produces a `WARN` event.

### 7.6 PostgreSQL settings for data safety

| Setting | Value | Reason |
|---|---|---|
| `fsync` | `on` | A commit is on disk before the client sees it |
| `synchronous_commit` | `on` | Same |
| `full_page_writes` | `on` | Protects against torn pages after a power loss |
| `wal_level` | `replica` | Allows a future physical replica and `pg_basebackup`. No cost for this workload |
| `checkpoint_timeout` | `5min` | Bounded recovery time |
| `shared_buffers` | 25 % of container memory | Default guidance |
| `log_min_duration_statement` | `1000` | Slow statements appear in the database log |
| `default_transaction_isolation` | `read committed` | Default. The approval use case sets `SERIALIZABLE` explicitly (`data-model.md` section 6) |
| `timezone` | `UTC` | All timestamps are `TIMESTAMPTZ`. The UI converts to the plant time zone |

### 7.7 Backup and restore

- The `backup` service runs `pg_dump --format=custom` daily at 02:00 plant time to the `backups` volume. File name `recipe_<YYYYMMDD_HHMM>.dump`.
- Retention is 30 daily dumps. Older files are deleted after a successful new dump.
- A dump also runs before every `migrate` (section 5) with the name suffix `_premigrate`.
- The latest dump is copied to the external backup storage by a site procedure (Assumption A16).
- A restore test runs weekly: the script restores the latest dump into a scratch database, runs `SELECT count(*)` on every table, compares the counts with the source, and writes the result to the event log. A failed restore test is an `ERROR` event.
- Recovery point objective: 24 hours. Recovery time objective: 1 hour. A site that needs a shorter objective adds WAL archiving. It is out of scope.

### 7.8 Volumes, resources, disk full

| Volume | Content |
|---|---|
| `pgdata` | PostgreSQL data directory |
| `backups` | Dumps from 7.7 |
| `caddy` | TLS certificates |

Resource limits per container: `app` 512 MiB memory, 1 CPU. `db` 2 GiB memory, 2 CPU. `proxy` and `backup` 128 MiB each.

Disk space policy (Assumption A24):

- `/readyz` reports the free space of the `pgdata` volume, measured by the database through `pg_stat_file` on a sentinel file, exposed to the application by a small SQL function.
- Warning threshold: 15 % free. The application writes a `WARN` event every 10 minutes.
- Hard threshold: 5 % free. The application enters read-only mode. Every command returns 503 with code `E-WEB-006` and the message "The system is in read-only mode because disk space is low. Contact the administrator." Queries and exports continue to work. The UI shows a banner.
- The application leaves read-only mode without a restart when free space rises above 10 %.

### 7.9 Clock, logs, and alerts

- All containers use the host clock. The host runs NTP. A clock skew above 5 s between `app` and `db` (checked at startup by `SELECT now()`) produces a `WARN` event.
- Log rotation is the Docker driver from section 3.5.
- Alert conditions, exported through the `/healthz` and `/readyz` endpoints and the Docker restart counter: container restart more than 3 times in 10 minutes, `readyz` failing for more than 1 minute, backup file older than 26 hours, disk free below 15 %. The alert transport (email, plant monitoring system) is a site decision.

### 7.10 Upgrade procedure

1. Read the release notes. Note whether the migration is reversible.
2. Pull the new image. Do not stop the running stack yet.
3. Run `docker compose run --rm backup backup-now` to create a `_preupgrade` dump.
4. `docker compose up -d migrate`. Wait for exit code 0. On failure, stop here. The old `app` keeps running on the old schema when the migration follows the expand pattern.
5. `docker compose up -d app`. Docker replaces the container. Downtime is the graceful shutdown window plus startup, under 60 s.
6. Check `/readyz`, log in, open one recipe.
7. Rollback: `docker compose up -d app` with the previous image tag. If the migration was not reversible, stop `app`, restore the `_preupgrade` dump, then start the previous image.

## 8. Code conventions

- Code comments, identifiers, commit messages, and documentation are English (R42). Comments follow the ASD-STE100 structural rules in the same way as this document.
- `CONTRIBUTING.md` in the repository root repeats the rules of sections 2 and 8 and links to this document.
- `rustfmt` with the workspace `rustfmt.toml`. CI runs `cargo fmt --check`.
- CI runs `cargo clippy --all-targets --all-features -- -D warnings` with the workspace lints from section 2.1.
- `cargo deny check` runs in CI. It fails on a denied license, a yanked crate, or a security advisory. The allow list is in `deny.toml`.
- `cargo audit` runs nightly against the `Cargo.lock`.
- `Cargo.lock` is committed. Dependency updates happen once per month in one pull request, plus at once for a security advisory.
- Minimum supported Rust version is the current stable at release time, recorded in `rust-version` of each crate. The project targets edition 2024.
- No `unsafe`. No `static mut`. No global mutable state. Shared state is passed through axum `State` as `Arc<AppState>` with interior mutability only inside the connection pool and the read-only mode flag (`AtomicBool`).
- Every public type and function in a library crate has a doc comment. `cargo doc --no-deps` runs in CI with `-D warnings` for broken links.
- A pull request needs: green CI, one review, and an entry in `CHANGELOG.md` when behavior changes.
