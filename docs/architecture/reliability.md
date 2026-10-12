# Reliability

Status: Accepted (design stage, revised 2026-10-12 after review)
Source of requirements: `general-draft.txt` (R29 to R38, R42, R43)
Related: `overview.md`, `domain-model.md`, `data-model.md`, `api.md`, ADR-0014, ADR-0015, ADR-0019, ADR-0020, ADR-0022

This document specifies how the system stays correct and available during continuous 24/7 operation. Each section names the rule and the mechanism that enforces it. Section 7.11 is the single source for every numeric setting. Other documents link to it. A value repeated elsewhere must equal 7.11.

## 1. Error handling

### 1.1 Error types per crate

Each library crate owns its error types, with two exceptions. `recipe-export` returns `ExportError` of `recipe-domain`. `recipe-db` maps into `StorageError` of `recipe-app`. The binary crate uses `anyhow`. Every variant has exactly one owner.

| Crate | Error type | Variants |
|---|---|---|
| `recipe-domain` | `PlantError`, `RecipeError`, `IdentityError`, `ExportError` | One `thiserror` enum per bounded context. Leaf errors. Every row with prefix `E-PLT`, `E-RCP`, `E-IDN` and a number below 030 in section 1.2, and every `E-RCP-050..069` row. `ExportError` lives in the module `export` and owns every `E-EXP` row: `NoApprovedVersion { reference }`, `VersionNotExportable { reference, number }`, `ContentHashMismatch { reference, number }`, `Serialize(#[from] serde_json::Error)` |
| `recipe-app` | `AppError`, `AuditError`, `StorageError` | `AppError`: `#[from] PlantError`, `#[from] RecipeError`, `#[from] IdentityError`, `#[from] ExportError`, plus `Forbidden { operation }`, `Unauthenticated`, `NotFound { entity }`, `ConcurrentEdit { reference, number, expected, actual }`, `Timeout { stage }`, `AuditWriteFailed`, `Storage(#[from] StorageError)`. `stage` is `Argon2Permit`, `Validation`, `Export`, or `Request`. `AuditError`: `ChainBroken { chain_seq }`, raised by the job `audit_chain_check`. `StorageError` is the error type of the storage ports: `Duplicate { constraint }`, `MissingReference { constraint }`, `Constraint { code, message }`, `Deadlock`, `StatementTimeout`, `PoolTimeout`, `Unavailable`, `Other { sqlstate, source }`. `source` is a boxed `std::error::Error`, so `recipe-app` holds no `sqlx` type |
| `recipe-db` | none public | Maps `sqlx::Error` into `StorageError` of `recipe-app` with the rule below. `recipe-db` depends on `recipe-app`, and `recipe-app` does not depend on `recipe-db` (`overview.md` 5) |
| `recipe-export` | none | No error type of its own. The JSON writer of the HTTP response returns `ExportError` of `recipe-domain` |
| `recipe-web` | `WebError` | `#[from] AppError`, plus `PayloadTooLarge { limit }`, `BadRequest { field, reason }`, `CsrfRejected`, `ReadOnlyMode`, `Internal { request_id }` |
| `recipe-server` | `anyhow::Error` | Everything, only in `main()`, configuration loading, subcommands, and wiring |

Rules:

- Every error variant carries the values that identify the failure (recipe reference, version number, tag, leaf path). It never carries a password, a password hash, a session token, or a cookie value.
- Every error type implements `Display` for the user and `Debug` for the log. `Debug` on a type that holds a secret redacts it.
- Every error type has a `code(&self) -> ErrorCode` method. The code is stable. The message template can change. The code cannot.
- `?` converts between layers through `From`. A `map_err` closure appears only when it adds context.
- `recipe-db` maps `sqlx::Error` to `StorageError` by SQLSTATE in this order: a `RAISE EXCEPTION` whose message starts with a trigger code from section 1.2 becomes `StorageError::Constraint { code, message }` with that code. `23505` becomes `Duplicate`. `23503` becomes `MissingReference`. `23514` without a trigger code becomes `Constraint { code: "E-DB-003", .. }`. `40P01` becomes `Deadlock`. `40001` becomes `Deadlock` when the one retry of the next rule also fails. `57014` becomes `StatementTimeout`. `55P03` becomes `Deadlock` (a lock wait that hit `lock_timeout`). A pool acquire timeout becomes `PoolTimeout`. A connection error becomes `Unavailable`. Any other code becomes `Other`.
- `SetUserRole` and `DeactivateUser` run in `SERIALIZABLE`. They retry once on `40001`, then return `E-DB-001`. Every other command runs in `READ COMMITTED` with no retry. `40P01` maps to `E-DB-001` and is logged at `ERROR`, because a deadlock is a bug in the lock order (ADR-0015).
- `recipe-app` maps `StorageError::Duplicate` to the domain duplicate error of the use case (`DuplicateReference`, `DuplicateMachineCode`, `DuplicateUnitPath`, `DuplicateLogin`, `DuplicateTag`, `DuplicateTypeName`) by the constraint name. A `Duplicate` with an unknown constraint name stays `AppError::Storage` with code `E-DB-006`. The same rule maps `StorageError::MissingReference` by the constraint name, for example `unit_parent_exists` to `ParentNotFound`. An unknown name gives `E-DB-007`.
- `AppError::Timeout { stage }` owns `E-WEB-009`. The Argon2 permit, the snapshot validation, and the export use it. `recipe-web` maps the request timeout of `TimeoutLayer` to `AppError::Timeout { stage: Request }`, so every timeout has one code.
- An error is logged exactly once. The log statement is in the `WebError` to HTTP mapping in `recipe-web`. A background job logs its errors once at its loop boundary with a `job` field. Lower layers add context and propagate. They do not log the error.
- The HTTP response contains the error code, the user message, and the `request_id`. It does not contain the `Debug` output, a stack trace, or an SQL fragment.

### 1.2 Error code registry

Format: `E-<CTX>-<nnn>`. `CTX` names the layer that reports the code: `PLT` (Plant Model), `RCP` (Recipe Management), `IDN` (Identity and Access), `AUD` (Audit), `EXP` (Export), `DB` (storage adapter), `WEB` (HTTP layer and cross-cutting application results). `nnn` is a three-digit number. Ranges: `E-PLT-001..029` and `E-RCP-001..029` for domain variants, `E-PLT-030..049` and `E-RCP-030..049` for database triggers, `E-RCP-050..069` for domain variants after `E-RCP-029`, `E-IDN-001..019`, `E-AUD-001..009`, `E-EXP-001..009`, `E-DB-001..009`, `E-WEB-001..019`. A number is never reused after a variant is deleted.

The registry lives in `crates/recipe-domain/src/error_codes.rs` as one constant table. A test in `recipe-server` collects the `code()` of every variant of every crate and checks that each code is unique and present in the table.

Message templates are English, ASD-STE100. `{…}` marks a value from the variant.

| Code | Owner::Variant { fields } | HTTP | User message |
|---|---|---|---|
| `E-PLT-001` | `PlantError::DuplicateMachineCode { code }` | 409 | Machine code {code} is already in use. Use a different code. |
| `E-PLT-002` | `PlantError::UnitDepthExceeded { path }` | 422 | Unit path {path} has more than 5 levels. Use at most 5 levels. |
| `E-PLT-003` | `PlantError::ParentNotFound { path }` | 422 | The parent unit of {path} does not exist. Create the parent unit first. |
| `E-PLT-004` | `PlantError::DuplicateUnitPath { path }` | 409 | Unit {path} already exists on this machine. |
| `E-PLT-005` | `PlantError::TypeCycle { type_name }` | 422 | Type {type_name} contains itself. Delete the circular member. |
| `E-PLT-006` | `PlantError::MemberTypeNotOnPlatform { type_name, member, platform }` | 422 | Member {member} of {type_name} uses a type that does not exist on {platform}. |
| `E-PLT-008` | `PlantError::LastMainUnit { code, path }` | 409 | Unit {path} is the last main unit of machine {code}. A machine needs one main unit. |
| `E-PLT-009` | `PlantError::UnitArchived { path }` | 409 | Unit {path} is archived. Select a live unit. |
| `E-PLT-010` | `PlantError::MachineArchived { code }` | 409 | Machine {code} is archived. |
| `E-PLT-011` | `PlantError::MachineHasDraft { code, reference }` | 409 | Machine {code} has a draft in recipe {reference}. Approve or discard the draft first. |
| `E-PLT-012` | `PlantError::UnitReferencedByDraft { path, reference }` | 409 | Unit {path} is used by the draft of recipe {reference}. Move or delete those parameters first. |
| `E-PLT-013` | `PlantError::DerivedTypeInvalid { type_expr, reason }` | 422 | The type {type_expr} is not valid: {reason}. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL. |
| `E-PLT-014` | `PlantError::UnitReferencedByApproved { path, reference }` | 409 | Unit {path} is used by the approved version of recipe {reference}. Retire that version first. |
| `E-PLT-015` | `PlantError::DuplicateTypeName { type_name }` | 409 | Type {type_name} already exists on this machine. Create a new revision instead. |
| `E-PLT-016` | `PlantError::StructHasNoMembers { type_name }` | 422 | Type {type_name} has no members. Add at least one member. |
| `E-PLT-017` | `PlantError::InvalidUnitPath { text }` | 422 | {text} is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2. |
| `E-PLT-018` | `PlantError::TypeTooLarge { type_name, limit }` | 422 | Type {type_name} is larger than the limit {limit}. Use fewer elements, fewer members, or fewer levels. |
| `E-PLT-030` | `StorageError::Constraint`, trigger function `machine_platform_frozen` (data-model.md 4.5) | 409 | The PLC platform of a machine with recipes cannot change. Reference: {request_id}. |
| `E-PLT-031` | `StorageError::Constraint`, trigger function `enforce_type_platform` (data-model.md 4.2) | 422 | The type does not exist on the platform of the version. Reference: {request_id}. |
| `E-PLT-032` | `StorageError::Constraint`, trigger function `enforce_type_platform` (data-model.md 4.2) | 422 | The type belongs to another machine. Reference: {request_id}. |
| `E-PLT-033` | `StorageError::Constraint`, trigger function `plc_type_immutable` (data-model.md 4.6) | 409 | A type definition cannot change. Create a new revision. Reference: {request_id}. |
| `E-PLT-034` | `StorageError::Constraint`, trigger function `plc_type_immutable` (data-model.md 4.6) | 409 | The members of a type cannot change after the type is created. Reference: {request_id}. |
| `E-PLT-035` | `StorageError::Constraint`, trigger function `plc_type_immutable` (data-model.md 4.6) | 422 | Only a struct type has members. Reference: {request_id}. |
| `E-PLT-036` | `StorageError::Constraint`, trigger function `plc_type_struct_has_member` (data-model.md 4.6) | 422 | A struct needs at least one member. Reference: {request_id}. |
| `E-PLT-037` | `StorageError::Constraint`, trigger function `freeze_identifiers` (data-model.md 4.8) | 409 | The machine code cannot change while a version that is not a draft uses it. Reference: {request_id}. |
| `E-PLT-038` | `StorageError::Constraint`, trigger function `freeze_identifiers` (data-model.md 4.8) | 409 | The unit path or the machine of a unit cannot change while a version uses the unit. Reference: {request_id}. |
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
| `E-RCP-012` | `RecipeError::RecipeDeleted { reference }` | 409 | Recipe {reference} is deleted. An administrator can restore it. (On export: only `/export/v1/recipes/{ref}/versions/{n}` returns this code. `/approved` of a deleted recipe returns 404 `E-EXP-001`.) |
| `E-RCP-013` | `RecipeError::TypeNotOnPlatform { tag, type_name, platform }` | 422 | Parameter {tag} uses type {type_name}. This type does not exist on {platform}. (Only an item inside `E-RCP-020 ValidationFailed`. The snapshot check reports it for a stored parameter whose type is no longer on the platform. A `TypeExpr` that is not on the platform is `E-PLT-013`.) |
| `E-RCP-014` | `RecipeError::DuplicateTag { tag, number }` | 409 | Parameter tag {tag} is already used in version {number}. |
| `E-RCP-015` | `RecipeError::ApproverEditedDraft { reference, number, login }` | 409 | User {login} created or edited draft {number} of recipe {reference}. A different user must approve it. |
| `E-RCP-016` | `RecipeError::LimitMissing { tag, leaf }` | 422 | {tag}{leaf} has no process limit. Set a limit before approval. |
| `E-RCP-017` | `RecipeError::LeafPathNotFound { tag, leaf }` | 422 | Path {leaf} does not exist in the type of {tag}. |
| `E-RCP-018` | `RecipeError::MinGreaterThanMax { tag, leaf, min, max }` | 422 | The minimum {min} of {tag}{leaf} is greater than the maximum {max}. |
| `E-RCP-019` | `RecipeError::LimitNotApplicable { tag, leaf }` | 422 | {tag}{leaf} is a BOOL. A BOOL has no process limit. |
| `E-RCP-020` | `RecipeError::ValidationFailed { items }` | 422 | {count} parameters are not valid. See the list. |
| `E-RCP-021` | `RecipeError::SizeBudgetExceeded { budget, limit, actual }` | 422 | The version exceeds the limit of {limit} {budget}. Current count: {actual}. |
| `E-RCP-022` | `RecipeError::S7AddressOverlap { tag, other_tag, address }` | 422 | Address {address} of {tag} overlaps with {other_tag}. |
| `E-RCP-023` | `RecipeError::S7AddressMisaligned { tag, address }` | 422 | Address {address} of {tag} must start at an even byte. |
| `E-RCP-024` | `RecipeError::TypeRevisionOutdated { tag, type_name, current, newest }` | 422 | {tag} uses revision {current} of {type_name}. The newest revision is {newest}. Rebind the parameter. |
| `E-RCP-025` | `RecipeError::RecipeNotDeleted { reference }` | 409 | Recipe {reference} is not deleted. |
| `E-RCP-026` | `RecipeError::RecipeHasRetiredVersion { reference }` | 409 | Recipe {reference} has a retired version. Only an administrator can delete it. |
| `E-RCP-027` | `RecipeError::InvalidLangCode { lang }` | 422 | {lang} is not a language code. Use a code such as pl or de-AT. |
| `E-RCP-028` | `RecipeError::RebindNeedsValue { tag, leaf }` | 422 | {tag}{leaf} is a new member. Give it a value. |
| `E-RCP-029` | `RecipeError::RebindDropsLimit { tag, leaf }` | 422 | {tag}{leaf} has a limit but no longer exists in the new revision. Delete the limit first. |
| `E-RCP-030` | `StorageError::Constraint`, trigger function `enforce_version_mutable` (data-model.md 4.1) | 409 | A parameter row cannot move to another version. Reference: {request_id}. |
| `E-RCP-031` | `StorageError::Constraint`, trigger function `enforce_version_mutable` (data-model.md 4.1) | 409 | The version is not a live draft. Reference: {request_id}. |
| `E-RCP-032` | `StorageError::Constraint`, trigger function `enforce_unit_machine` (data-model.md 4.3) | 422 | The unit does not belong to the machine of the recipe. Reference: {request_id}. |
| `E-RCP-033` | `StorageError::Constraint`, trigger function `recipe_version_transition` (data-model.md 4.7) | 409 | The version status or a frozen column of the version cannot change in this way. Reference: {request_id}. |
| `E-RCP-034` | `StorageError::Constraint`, trigger function `freeze_identifiers` (data-model.md 4.8) | 409 | The reference number or the machine of a recipe cannot change while a version uses it. Reference: {request_id}. |
| `E-RCP-035` | `StorageError::Constraint`, trigger function `version_size_budget` (data-model.md 4.10) | 422 | The version has more than 4 000 parameters. Reference: {request_id}. |
| `E-RCP-036` | `StorageError::Constraint`, trigger function `version_size_budget` (data-model.md 4.10) | 422 | The version has more than 32 768 limit rows. Reference: {request_id}. |
| `E-RCP-050` | `RecipeError::TranslationNotFound { reference, number, tag, lang }` | 404 | Parameter {tag} in version {number} of recipe {reference} has no translation for {lang}. |
| `E-RCP-051` | `RecipeError::ParameterNotFound { reference, number, key }` | 404 | Parameter {key} does not exist in version {number} of recipe {reference}. |
| `E-RCP-052` | `RecipeError::RebindTargetInvalid { type_name, reason }` | 422 | The rebind of {type_name} is not possible: {reason}. |
| `E-RCP-053` | `RecipeError::S7AddressNotAllowed { tag, platform }` | 422 | Parameter {tag} cannot have a PLC address on platform {platform}. Delete the address. |
| `E-IDN-001` | `IdentityError::DuplicateLogin { login }` | 409 | Login {login} is already in use. |
| `E-IDN-002` | `IdentityError::LastAdministrator { login }` | 409 | {login} is the last active administrator. Add another administrator first. |
| `E-IDN-003` | `IdentityError::InvalidCredentials` | 401 | Login or password is not correct. (An unknown login and an inactive user get the same message on purpose, A42.) |
| `E-IDN-004` | `IdentityError::UserInactive` | 401, as `E-IDN-003` | Not shown to the user. `Login` returns `E-IDN-003` with the same body and timing (A42). `UserInactive` is recorded as the `security_event` kind `login_rejected_inactive` and in the event log. |
| `E-IDN-005` | `IdentityError::LoginThrottled { retry_after }` | 429 | Too many failed logins from this address. Try again in {retry_after} seconds. |
| `E-IDN-006` | `IdentityError::PasswordPolicy { reason }` | 422 | The password is not accepted: {reason}. Use 12 to 128 characters that are not in the breached-password list. |
| `E-IDN-007` | `IdentityError::MustChangePassword` | 303 | Not an error page. The server redirects to `/me/password`. |
| `E-IDN-008` | `IdentityError::InvalidCurrentPassword` | 422 | The current password is not correct. |
| `E-IDN-009` | `IdentityError::UserAlreadyActive { login }` | 409 | User {login} is already active. |
| `E-AUD-001` | `AppError::AuditWriteFailed` | 500 | The change was not saved. The system could not record the audit entry. Contact the administrator. Reference: {request_id}. |
| `E-AUD-002` | `AuditError::ChainBroken { chain_seq }`, check of data-model.md 4.9 | 500 | Audit entry {chain_seq} does not match the hash chain. Contact the administrator. The nightly job `audit_chain_check` writes this code to the event log. The job `alerts` raises the alert. No user request returns it. |
| `E-EXP-001` | `ExportError::NoApprovedVersion { reference }` | 404 | Recipe {reference} has no approved version. |
| `E-EXP-002` | `ExportError::ContentHashMismatch { reference, number }` | 500 | Version {number} of recipe {reference} does not match its stored hash. The export is refused. Contact the administrator. Reference: {request_id}. |
| `E-EXP-003` | `ExportError::VersionNotExportable { reference, number }` | 409 | Version {number} of recipe {reference} is a draft or was discarded. Only approved and retired versions can be exported. |
| `E-EXP-004` | `ExportError::Serialize` | 500 | The server cannot create the export document. Contact the administrator. Reference: {request_id}. |
| `E-DB-001` | `StorageError::Deadlock` | 503 | Another operation holds a lock on the same data. Try again. Reference: {request_id}. |
| `E-DB-002` | `StorageError::StatementTimeout` | 503 | The operation took too long and was stopped. Try again. Reference: {request_id}. |
| `E-DB-003` | `StorageError::Constraint { code: "E-DB-003", .. }` | 409 | The change violates a database rule. Reference: {request_id}. |
| `E-DB-004` | `StorageError::PoolTimeout` | 503 | No database connection is free. Try again. Reference: {request_id}. |
| `E-DB-005` | `StorageError::Unavailable` | 503 | The database does not answer. Try again. Reference: {request_id}. |
| `E-DB-006` | `StorageError::Duplicate { constraint }`, unknown constraint name | 409 | The change conflicts with an existing entry. Reference: {request_id}. |
| `E-DB-007` | `StorageError::MissingReference { constraint }`, unknown constraint name | 409 | The change refers to an entry that does not exist. Reference: {request_id}. |
| `E-DB-008` | `StorageError::Other { sqlstate, source }` | 500 | An unexpected database error occurred. Contact the administrator. Reference: {request_id}. |
| `E-WEB-001` | `AppError::ConcurrentEdit { reference, number, expected, actual }` | 409 | Another user changed version {number} of recipe {reference}. Reload the page and apply your change again. |
| `E-WEB-002` | `AppError::Forbidden { operation }` | 403 | Your role does not permit the operation {operation}. |
| `E-WEB-003` | `WebError::PayloadTooLarge { limit }` | 413 | The request is larger than {limit}. |
| `E-WEB-004` | `AppError::Unauthenticated` | 401 | Log in to continue. |
| `E-WEB-005` | `AppError::NotFound { entity }` | 404 | {entity} does not exist. |
| `E-WEB-006` | `WebError::ReadOnlyMode` | 503 | The system is in read-only mode because disk space is low. Contact the administrator. |
| `E-WEB-007` | `WebError::CsrfRejected` | 403 | The form is no longer valid. Reload the page and try again. |
| `E-WEB-008` | `WebError::BadRequest { field, reason }` | 422 | Field {field} is not valid: {reason}. |
| `E-WEB-009` | `AppError::Timeout { stage }` | 503 | The request took too long and was stopped. Try again. Reference: {request_id}. |
| `E-WEB-010` | `WebError::Internal { request_id }` | 500 | An internal error occurred. Contact the administrator. Reference: {request_id}. |

A trigger code (`E-PLT-030..038`, `E-RCP-030..036`) reaches the user only when a domain check did not fire first. The HTTP layer returns the status of the row: 409 for a state rule, 422 for a shape rule. The event log gets an `ERROR` line, because the domain check should have caught the condition. This registry is the authority for every code. `data-model.md` section 4.11 maps each trigger code to its function.

Codes after `E-RCP-029`: `E-RCP-050 TranslationNotFound` (`RemoveTranslation`), `E-RCP-051 ParameterNotFound` (every draft command with `{key}` in the URL), `E-RCP-052 RebindTargetInvalid` (`RebindTypeRevision`), and `E-RCP-053 S7AddressNotAllowed` (`AddParameter`, `UpdateParameterDefinition`). `E-RCP-001..029` has no free number, so the block `E-RCP-050..069` holds them. The `{lang}` and `{key}` values of `E-RCP-050` and `E-RCP-051` come from the URL, so their status is 404. `api.md` 1 gives the rule. `E-RCP-051` and `E-RCP-050` are for a well-formed `{key}` or `{lang}` that the existing draft does not hold. `E-WEB-005` is for every other missing resource in the URL.

### 1.3 Reference shape

```rust
#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("reference number {reference} is already in use")]
    DuplicateReference { reference: ReferenceNumber },
    #[error("version {number} of recipe {reference} is {status}")]
    VersionNotEditable { reference: ReferenceNumber, number: u32, status: VersionStatus },
    #[error("user {login} created or edited draft {number} of recipe {reference}")]
    ApproverEditedDraft { reference: ReferenceNumber, number: u32, login: Login },
    // ... one variant per E-RCP row below 030 and in 050..069 in 1.2
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
| `clippy::expect_used` | deny | Same as above. No exception outside tests |
| `clippy::panic` | deny | No explicit `panic!` in application code |
| `clippy::unreachable` | deny | `unreachable!` is a panic path |
| `clippy::todo`, `clippy::unimplemented` | deny | Placeholders do not ship |
| `clippy::indexing_slicing` | deny | `v[i]` panics on a bad index. Use `get` |
| `clippy::arithmetic_side_effects` | deny | `a + b` on integers can overflow. See 2.2 |
| `clippy::cast_possible_truncation`, `clippy::cast_sign_loss`, `clippy::cast_possible_wrap`, `clippy::cast_precision_loss` | deny | An `as` cast silently changes the value |
| `clippy::exit` | deny | `main` ends the process with a return value. The watchdog function is the one documented exception (2.3) |
| `clippy::print_stdout`, `clippy::print_stderr` | deny | All output goes through `tracing` |
| `clippy::large_futures` | warn | Keeps task stacks small |
| `unused_must_use` | deny | A dropped `Result` hides an error |

The `[profile.release]` section sets `overflow-checks = true`. An overflow in release is a panic with a clear message instead of a wrong value. Section 2.2 makes such a panic unreachable in application code. Section 2.4 contains a panic that still happens.

### 2.2 Rules for code that cannot panic

| Situation | Rule |
|---|---|
| Integer arithmetic | Use `checked_add`, `checked_sub`, `checked_mul`, `checked_div`. Return `Err` on `None`. Use `saturating_*` only for counters that the UI shows, where a clamp is correct |
| Integer conversion | Use `u32::try_from(x)?` and `i64::try_from(x)?`. Never `as` between integer types in application code |
| Float to integer | Not needed in the domain. Values are `rust_decimal::Decimal`. If a conversion is needed, use `Decimal::to_i64()` and handle `None` |
| Decimal arithmetic | Use `checked_add`, `checked_mul`, `checked_div` of `rust_decimal`. A limit check uses comparison only. Comparison cannot fail |
| Slice and `Vec` access | Use `get(i)`, `first()`, `last()`, `chunks`, iterators. Never `v[i]` |
| `Option` | Use `let Some(x) = opt else { return Err(...) };` or `opt.ok_or(Error::...)?` |
| `HashMap` | Use `get` and `entry`. Never `map[&key]` |
| String slicing | Use `char_indices`, `get(a..b)`, or `split`. Never `s[a..b]` |
| Division | Check the divisor before the operation or use `checked_div` |
| Recursion on type trees | Depth is bounded by the `TypeTree` validation (section 4.4). Resolution uses an explicit stack, not recursion |
| Regular expressions | Compiled once with `LazyLock`. The pattern is a literal and is tested. A compile failure is a test failure, not a runtime panic |
| `Mutex` lock | Use `parking_lot::Mutex` whose `lock()` cannot return an error |
| Tokio `JoinHandle` | `handle.await` returns `Result<T, JoinError>`. Map `JoinError` to `AppError::Storage` or `WebError::Internal`. Never `unwrap` it |
| Blocking inside async | Never `block_on` inside an async context. CPU-heavy work (Argon2id hashing, snapshot validation) runs in `spawn_blocking` |
| Time | Use `time::OffsetDateTime::now_utc()`, which does not fail |

### 2.3 Permitted exceptions

- Test code (`#[cfg(test)]` and `tests/`) may use `unwrap`, `expect`, and indexing. The lint is set to `allow` for test targets.
- The watchdog function of 2.4 has the attribute `#[allow(clippy::exit)]`, because it must end a stalled process with `std::process::exit(70)`. It is the only function with this attribute. A comment on the attribute names this section.
- No other exception. `recipe-server::main` returns `anyhow::Result<()>`. A startup precondition that fails returns an error with context, and the process exits with code 1 after one `ERROR` line. A pull request that adds `#[allow(clippy::unwrap_used)]` outside tests is rejected in review.

### 2.4 Panic containment

A panic in application code is a bug, not an error. The system contains it and keeps serving (Assumption A41, R30):

1. `[profile.release]` keeps `panic = "unwind"`.
2. `recipe-server::main` installs a panic hook with `std::panic::set_hook`. The hook writes one `tracing::error!` event with the panic message, the location, and the current `request_id` when a span is active. The hook does not end the process.
3. `tower_http::catch_panic::CatchPanicLayer` is the outermost layer of the router. A panic inside a request becomes a `500 E-WEB-010` response with the `request_id`. The open transaction of that request is dropped and PostgreSQL rolls it back. No partial write survives (R29).
4. Every background job of the table below runs inside a supervisor loop. A panic in one iteration is caught by the task join, logged with the `job` field, and the job restarts after `job.restart_delay`.
5. Watchdog: an OS thread, started before the Tokio runtime, probes the internal health listener every `watchdog.probe_interval`. After `watchdog.failures` failed probes in `watchdog.window`, the thread writes one `ERROR` line and calls `std::process::exit(70)` (the exception of 2.3). Docker restarts the container. This covers a stalled runtime, which `restart: always` alone does not detect.
6. Application code never calls `std::panic::catch_unwind` itself. The two places that catch a panic are the layer in rule 3 and the supervisor in rule 4.

A panic is a visible signal. The job `alerts` raises an alert when more than `alerts.panic_count` panic events occur in `alerts.panic_window` (section 3.7).

Supervised background jobs. They run inside `app`. The intervals are keys of section 7.11.

| Job | Work | Interval |
|---|---|---|
| `cleanup` | Deletes expired `session` rows and old `login_attempt` rows. Calls `security_event_purge()` (`data-model.md` 4.12). Uses the port `MaintenanceStore` and writes no audit entry | `session.cleanup_interval`, `login.attempt_cleanup_interval`, `security_event.purge_interval` |
| `disk_check` | Measures free space and sets the read-only mode (7.8) | `health.disk_check_interval` |
| `audit_chain_check` | Recomputes the audit hash chain in batches of `audit.check_batch` rows (`data-model.md` 4.9). Writes a break through the port `SecurityEventSink`. After a completed check, it records `audit.last_check_seq` and `audit.last_check_at` in `app_setting` through the port `MaintenanceStore`, with no audit entry | `audit_chain.check_time` |
| `clock_check` | Compares the clocks of `app` and `db` (7.9) | `clock.check_interval` |
| `alerts` | Checks the alert conditions (3.7) | `alerts.interval` |

## 3. Structured logging and tracing

### 3.1 Stack

- `tracing` for spans and events in all crates.
- `tracing-subscriber` with the JSON formatter. Output goes to stdout. Docker collects it.
- `tower-http::trace::TraceLayer` creates the request span.
- One line per event. One JSON object per line.

### 3.2 Span fields

| Span | Fields |
|---|---|
| `http.request` | `request_id`: UUID v7, generated by the server, returned in the `X-Request-Id` response header. `method`. `route`: the pattern, for example `/recipes/{ref}/versions/{n}`, not the concrete path. `client_ip`: from the trusted proxy (section 7.11). `user_agent`: first `http.user_agent_max` characters. `session_ref`: first `log.session_ref_length` hex characters of the session id hash, the same value as `audit_log.session_ref`. `user_id`, `role`, `status`, `latency_ms`. `error_code` when the result is an error |
| `use_case` | `name` (for example `ApproveVersion`), `recipe_ref`, `version`, `machine_code`, `outcome` (`ok` or the error code) |
| `db.statement` | `name` (a label such as `recipe_version.update_status`), `rows_affected`, `latency_ms`. Never the SQL text with bound values |
| `uow.transaction` | `isolation`, `outcome` |
| `job` | `name`, `iteration`, `outcome` |

The `request_id` of the HTTP span is inherited by all child spans. A log line without a `request_id` comes from startup, shutdown, or a background job. Those lines carry a `job` field instead.

### 3.3 Level policy

| Level | Use |
|---|---|
| `ERROR` | The system could not do what was asked because of an internal failure. Examples: storage error, audit write failure, caught panic, deadlock, trigger code reached the user, startup failure, audit chain break. Each `ERROR` line is actionable |
| `WARN` | Degraded but working. Examples: slow query (above `db.slow_query`), pool near exhaustion, disk space below the warning threshold, login throttled, CSRF rejected, Forbidden on `UserManage`, clock skew |
| `INFO` | One line per request (from `TraceLayer`), one line per use case outcome, failed login, startup and shutdown, migration applied, backup completed |
| `DEBUG` | SQL statement names and timings, cache of type trees, template rendering |
| `TRACE` | Not used in production. Available for local debugging |

A validation error from a user (HTTP 4xx) is `INFO`, not `WARN`. It is a normal outcome. A 5xx is `ERROR`.

Production runs at `INFO` for all crates and `WARN` for `sqlx` and `hyper`. The level is set by the `RUST_LOG` variable and can change without a rebuild.

### 3.4 Redaction

Never written to the log:

- passwords, password hashes, and the one-time password of `CreateUser` and `ResetPassword`
- session tokens, session ids, the `Cookie` and `Set-Cookie` headers. The log holds `session_ref` (the first 16 hex characters of the session id hash, 3.2) instead of the session id
- the `X-CSRF-Token` header and the `csrf_token` and `csrf_pre` fields
- the `Authorization` header
- the full request body of the login and password forms
- `DATABASE_URL`, every `*_PASSWORD` and `*_FILE` secret, the keys `login_hmac_key` and `csrf_signing_key`, the bootstrap administrator password
- the `Debug` output of configuration structs

The `Debug` implementation of `PasswordHash`, `SessionToken`, `CsrfToken`, `LoginForm`, `PasswordForm`, and `Config` prints `[redacted]` for the secret fields. The `TraceLayer` is configured to skip the `Cookie`, `Set-Cookie`, `Authorization`, and `X-CSRF-Token` headers.

Parameter values and process limits are not secret. The use case span may include them at `DEBUG`.

### 3.5 Retention

Docker uses the `json-file` log driver with `max-size` and `max-file` from `log.max_size` and `log.max_file` (section 7.11). This keeps about 1 GB of logs per container. Older lines are discarded. A site that needs longer retention forwards stdout to its own collector. The application does not write log files.

### 3.6 Audit log, security events, event log

| | Audit log | Security events | Event log |
|---|---|---|---|
| Location | PostgreSQL table `audit_log` | PostgreSQL table `security_event` | Container stdout |
| Content | Who changed what, before and after values, `aggregate_type`, `aggregate_id`, `client_ip`, `session_ref`, hash chain. Setting changes (`SettingChanged`, 7.12) | Failed logins, throttling, CSRF rejections, Forbidden results, unlocks, audit chain breaks (A33). `client_ip` is NULL only for `audit_chain_broken` | How the system behaved: latencies, errors, panics |
| Written | In the same transaction as the change (ADR-0005) | In its own short transaction, also when the request fails | Asynchronously, best effort |
| Retention | Forever. Part of the backup | `security_event.retention` (section 7.11). Then the job `cleanup` deletes the rows through `security_event_purge()` (`data-model.md` 4.12) | 1 GB rolling window |
| Reader | Administrator (all), ProcessEngineer (recipe entries), every role (recipe history page), per A20 | Administrator | Operations staff through Docker tools |
| Tamper evidence | `entry_hash` chain, checked every night (ADR-0022) | Append-only for `recipe_app` | none |

A business question ("who approved version 7?") is answered by the audit log. A security question ("who tried to log in as admin at 03:00?") is answered by the security events. An operations question ("why was the approval slow?") is answered by the event log. The `request_id` is present in all three and links them.

### 3.7 Alerts

The supervised job `alerts` runs inside `app` every `alerts.interval` (section 7.11). It evaluates the conditions below. For each active condition it writes one `WARN` line with the field `alert` to the event log. When `alerts.webhook_url` is set, the job also sends one HTTP `POST` with the condition name to that URL. A failed webhook call is logged once and does not end the job. There is no alert cron in the `backup` container (ADR-0020).

| Condition | Source |
|---|---|
| More than `alerts.panic_count` panic events in `alerts.panic_window` | In-process counter of the panic hook (2.4) |
| `/readyz` not ready for more than `alerts.not_ready_for` | In-process readiness state |
| Latest dump older than `backup.max_age_alert` | Read-only mount of the `backups` volume |
| WAL archive older than `wal.max_age_alert` | Read-only mount of the `walarchive` volume |
| Free disk below `disk.warn_percent` on `pgdata`, `backups`, or `walarchive` | `statvfs` on the read-only mounts |
| More than `alerts.failed_login_count` failed logins in `alerts.failed_login_window` | `security_event` |
| Any `csrf_rejected`, or a `forbidden` on `UserManage`, in the last `alerts.security_window` | `security_event` |
| Audit chain check failed | `security_event` kind `audit_chain_broken`, written by the job `audit_chain_check` |
| No completed audit chain check in the last 24 h | `audit.last_check_at` in `app_setting`, written by the job `audit_chain_check` (`data-model.md` 4.9) |
| One or more `E-EXP-002` in the last `alerts.interval` | In-process error counter of the job `alerts`. `ExportVersion` increments it (`data-model.md` 6) |

A restart loop of `app` also ends this job. Docker marks the container unhealthy. The runbook names `docker compose ps` and the host monitoring as the check for a restart loop.

## 4. Input validation

### 4.1 Three layers

| Layer | Where | What it rejects |
|---|---|---|
| (a) HTTP | `recipe-web` extractors | Wrong shape, wrong encoding, too large, unknown fields |
| (b) Domain | `recipe-domain` value object constructors and snapshot validation | Values that violate a domain rule |
| (c) Database | `CHECK`, `UNIQUE`, FK, triggers (`data-model.md`) | Anything that passes (a) and (b) because of a bug |

A request that passes layer (a) is well-formed. A value that passes layer (b) is valid. Layer (c) exists so that a bug in (a) or (b) produces an error instead of corrupt data.

### 4.2 HTTP layer rules

- Body limits are in section 7.11. The server returns `E-WEB-003` above the limit. The parameter value field has its own limit, because a form with 16 384 leaves does not fit in the form limit.
- All JSON types derive `serde::Deserialize` with `#[serde(deny_unknown_fields)]`.
- Form parsing uses `axum::Form` with `serde` structs. A missing required field is `422 E-WEB-008` with the field name.
- A path parameter is parsed directly into the value object (`ReferenceNumber`, `MachineCode`, `UnitPath`, `ParameterKey`, `LangCode`). A bad path parameter never gives a 500. The rule has two cases (`api.md` 1):
  - A path value that is a domain value object with its own error code returns that code with 422. `LangCode` returns `E-RCP-027`. `UnitPath` returns `E-PLT-017` for a malformed text or segment and `E-PLT-002` for more than 5 segments. No path holds an S7 address.
  - A path value that names a resource returns `404 E-WEB-005` when the resource does not exist. This applies also when the text cannot name a resource, for example a bad `ReferenceNumber`. The exemption: a command on a draft returns `404 E-RCP-051` for a well-formed `{key}` and `404 E-RCP-050` for a well-formed `{lang}` that the existing draft does not hold (`api.md` 1).
- The `Content-Type` of a request must match the route. Every mutating route accepts only `application/x-www-form-urlencoded`. The `value` field inside the form is JSON text.
- HTMX 2 sends the parameters of a `DELETE` in the URL by default. The `htmx-config` meta tag sets `methodsThatUseUrlParams: ["get"]`, so `hx-delete` sends a form body like `hx-post` (`api.md` 9).
- Text fields are trimmed and normalized to Unicode NFC before layer (b).
- The `row_version` of a draft is a hidden form field. A missing `row_version` on a mutating draft request is `422 E-WEB-008`.
- The `next` field of `POST /lang` must be a relative path that starts with `/` and not with `//`. Anything else is replaced by `/recipes`.
- The `lang` field of `POST /lang` and the cookie `recipe_lang` accept only `pl` and `en`. The server ignores any other cookie value and uses `pl`.

### 4.3 Domain value objects

Each value object has one constructor `parse(input) -> Result<Self, Error>` and no public `new`. A value object instance is valid by construction. `domain-model.md` section 2 is the authority for every grammar. A row that names `domain-model.md` does not repeat the grammar.

| Value object | Grammar or rule |
|---|---|
| `MachineCode` | `^M[1-9][0-9]{0,6}$`, number `1..=9999999` |
| `UnitPath` | `^U([1-9]\|[1-5][0-9]\|6[0-4])(\.([1-9]\|[1-5][0-9]\|6[0-4])){0,4}$`, 1 to 5 segments, each `1..=64` (A6, A30). A malformed text or segment is `E-PLT-017`. More than 5 segments is `E-PLT-002` |
| `ReferenceNumber` | `^[A-Z0-9][A-Z0-9_-]{0,49}$` after uppercasing (A27) |
| `ParameterTag` | `^[A-Z][A-Z0-9_]{0,63}$` |
| `ParameterKey` | UUID, generated by the server (A40) |
| `Login` | `^[a-z0-9._-]{3,64}$` after lowercasing |
| `Text` (names: machine, unit, recipe) | Not empty after trim, at most 200 characters, no control characters |
| `Text` (description, change note) | Not empty after trim, at most 4 000 characters, no control characters except newline. `description_en` is required (A36) |
| `Text` (translation name, description) | Name: not empty, at most 200 characters. Description: 0 to 4 000 characters, may be empty. No control characters except newline in the description |
| `LangCode` | `^[a-z]{2,3}(-[A-Z]{2})?$`, not `en` (English is the base text, A17) |
| `S7Address` | Grammar, width rule, alignment rule, and overlap rule: `domain-model.md` 2.1. Platform rule: required on `S7_300`, optional on `S7_1200` and `S7_1500`, not allowed on `CodesysIec` and `TwinCat3` (A31) |
| `S7Layout` | `domain-model.md` 2.1: `BOOL` packing, even-byte alignment, struct and array size |
| `TypeName` (UDT) | `^[A-Za-z_][A-Za-z0-9_]{0,63}$`, not a reserved primitive name |
| `StringLength` | Range per platform: the `StringLength` table of `domain-model.md` 2.1 |
| `TypeExpr` | Grammar, rules, and errors: `domain-model.md` 2.1. Every failure of the reason table of `domain-model.md` 2.1 is `E-PLT-013`. A type above a size limit is `E-PLT-018` |
| `ArrayDimension` | `lower <= upper`, `upper - lower < 65536`, at most 3 dimensions (A38, `domain-model.md` 2.1) |
| `LeafPath` | Grammar: `domain-model.md` 2.1. The empty path is the parameter itself. `[*]` means every element of that array level. Resolves against the `TypeTree` |
| `ProcessLimit` | `min <= max` when both set. Inside the type range. `max_length <= string_length`. Not on a `BOOL` leaf |
| `Password` (input only) | 12 to 128 characters, not in the breached-password list. Hashed at once. The plain text is dropped after hashing |

### 4.4 Parameter value JSON

- The JSON is parsed by `serde_json` with the `arbitrary_precision` feature. A number stays a decimal string inside `serde_json::Number`. There is no `f64` round trip.
- The domain converts each numeric leaf to `rust_decimal::Decimal` for range and limit checks. A number that does not fit in `Decimal` (more than 28 significant digits) is rejected with `E-RCP-005`.
- Canonical form: the domain rewrites every numeric leaf to the canonical decimal text of `Decimal` (no exponent, no trailing zeros, `-0` becomes `0`) before storage. `1.5e2` is stored as `150`. `JSONB` keeps this canonical form. The export writes each numeric leaf and each limit as a JSON string that holds this canonical text (`api.md` 6.1). The content hash therefore does not depend on a JSON number format. Equality of two values is equality of canonical forms.
- `REAL` and `LREAL` leaves are additionally checked to fit the IEEE 754 single or double range.
- The JSON parser enforces the depth limit `json.max_depth` (section 7.11). `DefineUdt` and `EnsureDerivedType` reject a `TypeTree` deeper than this limit with `E-PLT-018`, so a valid value never reaches it.
- Size budgets (Assumption A26, section 7.11): at most 16 384 scalar leaves per parameter. Per version: at most 262 144 leaves, 32 768 limit rows, and 4 000 parameters. `TypeTree` resolution counts leaves. `DefineUdt` and `EnsureDerivedType` reject a type above the per-parameter budget or above `type.max_members` with `E-PLT-018`. `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `RebindTypeRevision`, and `CreateVersion` check the per-version budgets with `E-RCP-021`. Statement-level triggers on `parameter` and `parameter_limit` are the last line (`E-RCP-035`, `E-RCP-036`).
- The shape check is exact: an array has at most 3 dimensions (Assumption A38) and exactly `(upper - lower + 1)` elements per dimension. A struct has exactly the member names. No extra key, no missing key.
- `STRING` values are checked for byte length against `string_length` in the target encoding (Latin-1 for `STRING`, UTF-16 units for `WSTRING`).
- `BOOL` accepts only JSON `true` and `false`. Not `0`, `1`, or `"true"`.
- Time types (`TIME`, `LTIME`, `S5TIME`, `DATE`, `LDATE`, `TIME_OF_DAY`, `LTOD`, `DATE_AND_TIME`, `LDT`, `DTL`) are stored as ISO 8601 strings and parsed with the `time` crate. The domain checks the platform range (for example `S5TIME` up to `2h46m30s`). Type names are the names of the matrix in `data-model.md` section 7.
- Snapshot validation (`CreateVersion`, `ApproveVersion`) runs in `spawn_blocking` before the transaction opens. The transaction then locks the rows and re-checks `row_version`. It also repeats two checks that can change after the snapshot (`data-model.md` 6). `CreateVersion` checks the units again. `ApproveVersion` checks the limit count and the outdated types again. A validation that takes longer than `timeout.validation` returns `E-WEB-009`.

## 5. Migrations and schema evolution

- Migrations are SQL files in `migrations/` at the repository root, managed by `sqlx::migrate!`. The file name is `NNNN_description.sql`. The first release ships `0001_schema.sql`, `0002_triggers.sql`, `0003_grants.sql`, `0004_seed_plc_types.sql`.
- Migrations are forward-only. There is no `down` file. A rollback is a new forward migration or a restore from backup.
- A migration that shipped is never edited. `sqlx` stores a checksum of each applied migration in `_sqlx_migrations`. A changed checksum stops the migrator with an error.
- Roles and the database are not created by a migration. The script `deploy/db/initdb/01-roles.sh` runs once as the PostgreSQL superuser when the `db` volume is empty. It reads the Docker secrets and creates `recipe_migrator`, `recipe_app`, `audit_owner`, `recipe_backup`, `recipe_replicator`, and the database. It adds the `pg_hba.conf` line for the replication connection of `recipe_replicator` (`data-model.md` section 5.1).
- The server binary has a `migrate` subcommand: `recipe-server migrate`. The Compose service `migrate` runs it once (`restart: no`). It is the only service that holds the migrator secret. The `app` service never connects as `recipe_migrator`. At startup `app` checks that `current_user` is `recipe_app`, is not a superuser, and owns no table. It refuses to serve otherwise.
- The migrator takes an advisory lock. Two migrators cannot run at the same time.
- Every migration sets `lock_timeout` to `migrate.lock_timeout`. The migrator retries a migration that failed on `55P03` up to `migrate.retries` times with a pause of `migrate.retry_pause` (section 7.11). A migration that must build an index on a large table uses `CREATE INDEX CONCURRENTLY` in a file that starts with `-- no-transaction`. CI rejects a migration that rewrites a table or scans it under an exclusive lock (`ALTER TABLE ... TYPE`, `ADD COLUMN ... DEFAULT <volatile>`, `SET NOT NULL` without a valid `CHECK (... IS NOT NULL)` first). The rule applies to every table (`data-model.md` 8).
- CI starts a fresh cluster, runs `01-roles.sh`, and applies `0001` to `0004` as `recipe_migrator`. Then it runs the grants test of `data-model.md` 5.3. A migration that fails as `recipe_migrator` therefore fails the build.
- A breaking change uses the expand-and-contract pattern across releases: release N adds the new column or table and writes both. Release N+1 reads the new one. Release N+2 deletes the old one.
- Start order in Compose: `db` healthy → `backup-premigrate` (one-shot `pg_dump` with suffix `_premigrate`) completed → `migrate` completed → `app` → `proxy`. The `depends_on` conditions encode this chain (section 7.1). The upgrade procedure in 7.10 depends on the `_premigrate` dump.
- The first administrator is not a migration. `recipe-server create-admin` reads the login from an argument and the password from stdin or from a secret file. It does nothing when an active administrator exists. It sets `must_change_password`. The Compose file does not run it. The runbook runs it once after the first `migrate`.
- Primitive `plc_type` rows are seed data in `0004_seed_plc_types.sql`. Each row has a UUID that never changes. The insert uses `ON CONFLICT DO NOTHING`, so a re-run is idempotent (`data-model.md` 8).

## 6. Testing strategy

| Level | Scope | Tooling | Rule |
|---|---|---|---|
| Unit, domain | `recipe-domain` | `cargo test`, no I/O | Every invariant in `domain-model.md` section 3 has at least one passing and one failing test. Every domain error variant (below 030 and `E-RCP-050..069`) is produced by at least one test |
| Unit, use case | `recipe-app` | In-memory implementations of all ports | Every command in `domain-model.md` section 5 has tests for success, each listed error, and the audit entry it writes. `SetParameterValue` leaves limit and translation rows unchanged. `ReplaceLimits` leaves the value unchanged. Separation of duties: the creator and each editor of a draft get `E-RCP-015`. Limit completeness: a numeric leaf without a limit blocks approval with `E-RCP-016` |
| Permission matrix | `recipe-app` | Generated test from the matrix in `api.md` section 3 | One test per (operation, role) pair, for commands and for queries. The test asserts `Ok` or `AppError::Forbidden`. A new use case without a matrix row fails the build. `EnsureDerivedType` is an internal operation, not a use case, and has no matrix row. A demoted user with an open session gets `AppError::Unauthenticated` (`401 E-WEB-004`) on the next request, not `Forbidden`, because `SetUserRole` deletes the sessions of the user and increments `session_epoch` |
| Property | `recipe-domain` | `proptest` | `UnitPath`, `S7Address`, `ReferenceNumber`, `LeafPath` round-trip through `parse` and `Display`. `ParameterValue::validate` against random `TypeTree`s up to depth 6 with random limits: a generated valid value passes, a mutated value fails. `S7Layout`: random sets of parameters in one data block, overlap and misalignment are always detected, non-overlapping aligned sets always pass |
| Integration, storage | `recipe-db` | `testcontainers` with the pinned `postgres` image, or `DATABASE_URL` of a Compose database in CI | Every `CHECK`, `UNIQUE`, FK, partial index, and trigger in `data-model.md` has a test that violates it and asserts the SQLSTATE and the trigger code. As `recipe_app`, an update of a column outside the UPDATE grant of `recipe_version` (`number`, `recipe_id`, `based_on_version_id`) fails with SQLSTATE `42501` before any trigger runs. The test fixture runs the forbidden version transitions as the table owner `recipe_migrator`. These are `approved→draft`, `retired→approved`, `retired→draft`, and the undelete of a discarded draft. They also include a change of `number`, `recipe_id`, `based_on_version_id`, or `content_sha256`. Each one fails with `E-RCP-033`. The grants of `recipe_app` (`role_table_grants`, `role_column_grants`) equal a checked-in list. `RemoveParameter` on a parameter with limits and translations succeeds. A second `STRING(20)` on one machine succeeds. Every transactional operation in `data-model.md` section 6 has a test that fails midway and asserts no partial write |
| Concurrency | `recipe-db` | Two connections | Two approvals of the same draft with the same `row_version`: exactly one writes. The other returns `AlreadyApproved` and writes no audit entry. An approval with an older `row_version` gets `E-WEB-001`. Two parallel audit inserts get consecutive `chain_seq` values, and the chain check passes. A parameter insert during an approval waits and then fails with `E-RCP-031`. `DeleteRecipe` during `ApproveVersion`: the second one fails. Two edits with the same `row_version`: exactly one succeeds. Two concurrent failed logins: both count |
| HTTP | `recipe-web` | `tower::ServiceExt::oneshot` with in-memory ports | Every route: status code, error code in the body, `X-Request-Id` header, security headers, no secret in the body. Login, logout, session expiry, session epoch invalidation, CSRF rejection, pre-session CSRF on `/login` and `/lang`, `must_change_password` gate. An XSS payload corpus through every text field |
| Load | `recipe-web` with Compose database | `oha` | Login flood: 200 concurrent `POST /login` with random logins. The container stays under its memory limit, no request waits more than 5 seconds, and the Argon2 semaphore returns `503` under load. A version at the size budget: `CreateVersion`, `ApproveVersion`, and the version page each finish under 10 seconds |
| End to end | Compose stack | `playwright-cli` smoke suite | Create machine, create recipe, add parameter, set limits, approve by a second user, export, check the hash, check the audit page |
| Soak | Compose stack, nightly CI | The same request mix for 24 h at a constant rate, with a login flood every hour | Memory of the `app` container stays under 384 MiB (section 7.11). No `ERROR` line. No restart. p99 latency stays under 500 ms |
| Lint gate | Workspace | `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` | Runs on every pull request |

Coverage target: 80 % line coverage per crate, measured with `cargo llvm-cov`. The CI job fails below the target.

## 7. Operations for 24/7

### 7.1 Compose layout

| Service | Image | Role | Restart | Healthcheck |
|---|---|---|---|---|
| `proxy` | `caddy`, pinned | TLS termination, HTTP to HTTPS redirect, `404` for `/healthz*` and `/readyz*`. Mandatory. When another device of the plant handles TLS, Caddy still runs and re-encrypts to the upstream (A18) | `always` | `GET http://app:9000/healthz` |
| `backup-premigrate` | same as `backup` | One-shot `pg_dump` with suffix `_premigrate` | `no` | Exit code |
| `migrate` | `recipe-server` with `migrate` | One-shot schema migration. Holds the migrator secret | `no` | Exit code |
| `app` | `recipe-server` | The application and its supervised jobs, `alerts` included (2.4). Public listener on 8080 (`expose`, never `ports`). Internal listener on 9000 | `always` | `GET http://localhost:9000/readyz` every `health.check_interval`. `health.check_failures` failures mark it unhealthy |
| `db` | `postgres:16.<x>-bookworm`, pinned | Database. `POSTGRES_INITDB_ARGS="--data-checksums --lc-collate=C --lc-ctype=C.UTF-8"`. Port 5432 not published | `always` | `pg_isready` every `health.check_interval` |
| `backup` | same base image as `db` with cron | Daily `pg_dump`, `backup-now` one-shot command, retention. Connects as `recipe_backup` | `always` | Latest dump younger than `backup.max_age_alert`. The backup path is not on the filesystem of `pgdata` |
| `walarchive` | same base image as `db` | `pg_receivewal` and the weekly `pg_basebackup` to a bind mount on a second disk (A35, ADR-0020). Connects as `recipe_replicator` over the replication protocol | `always` | Latest WAL segment younger than `wal.max_age_alert` |

Seven services. Start order through `depends_on`: `db` (healthy) → `backup-premigrate` (completed successfully) → `migrate` (completed successfully) → `app` → `proxy`. `walarchive` and `backup` depend only on `db`.

Secrets: every password is a Docker secret mounted as a file. The application reads `*_FILE` variables only. There is no env-file option. The superuser password exists only in `db` and in the initdb script. The secret `replicator_password` (`secrets.replicator_password_file` of 7.11) exists only in `db` and in `walarchive`. The initdb script creates `recipe_replicator` from it (`data-model.md` 5.1).

Two more Docker secrets hold keys of 32 bytes. `login_hmac_key` is the HMAC-SHA256 key of `security_event.login_hmac`. `csrf_signing_key` signs the pre-session CSRF cookie (`api.md` 7.3). The keys `security.login_hmac_key_file` and `security.csrf_signing_key_file` of 7.11 name the files. A key file holds one key per line, the newest key first. The server writes and signs with the first key and accepts every key in the file. Rotation: the operator adds the new key as the first line. Both keys are accepted for `security.key_rotation_overlap`. Then the operator deletes the old line.

### 7.2 Health endpoints

| Endpoint | Checks | Use |
|---|---|---|
| `GET /healthz` | The process is up and the router answers | Liveness. The watchdog thread (section 2.4) and the proxy healthcheck use it |
| `GET /readyz` | A pooled connection can run `SELECT 1` within `health.db_check_timeout`. The schema version matches the binary | Readiness. Docker marks the container unhealthy after `health.check_failures` failures. The result is cached for `health.readiness_cache` |

Both endpoints live on the internal listener on port 9000. They need no session. They return `text/plain` with `ok` or `not ready`. They do not return version strings, hostnames, or check details. Disk space is not part of readiness. A low-disk condition changes the mode of the application (7.8), not its readiness.

### 7.3 Graceful shutdown

1. The process receives `SIGTERM` from Docker.
2. The listener stops accepting new connections. The readiness endpoint returns `not ready`.
3. Requests in flight continue. Open transactions commit or roll back as their use case finishes.
4. After all requests finish, or after `shutdown.drain`, the pool closes and the process exits with code 0.
5. Docker `stop_grace_period` is longer than `shutdown.drain` (section 7.11), so step 4 completes before `SIGKILL`.

### 7.4 Timeouts

The values are in section 7.11. This table gives the key and the mechanism.

| Where | Key (7.11) | Mechanism |
|---|---|---|
| HTTP request total | `http.request_timeout` | `tower-http::timeout::TimeoutLayer`. Returns `503 E-WEB-009` |
| Request body read | `http.body_read_timeout` | `hyper` header and body read timeouts |
| SQL statement | `db.statement_timeout` | `statement_timeout` set per connection by the pool `after_connect` hook |
| Lock wait | `db.lock_timeout` | `lock_timeout` per connection. A timeout becomes `E-DB-001` |
| Idle in transaction | `db.idle_in_transaction_timeout` | `idle_in_transaction_session_timeout` per connection. No use case waits inside a transaction, because validation runs before it |
| Pool acquire | `db.pool_acquire_timeout` | `sqlx::PoolOptions::acquire_timeout`. A timeout becomes `E-DB-004` |
| Pool idle connection | `db.pool_idle_timeout` | `idle_timeout` |
| Pool max lifetime | `db.pool_max_lifetime` | `max_lifetime`. Avoids stale connections after a database restart |
| Argon2id semaphore | `argon2.acquire_timeout` | Acquire timeout. A timeout returns `503 E-WEB-009` (`AppError::Timeout`) |
| Export generation | `timeout.export` | Timeout around the use case. Returns `503 E-WEB-009` |
| Snapshot validation | `timeout.validation` | Timeout around the `spawn_blocking` call. Returns `503 E-WEB-009` |

### 7.5 Connection pool

Pool size is `db.pool_size` connections for the application and `db.health_pool_size` for health checks. PostgreSQL `max_connections` is `db.max_connections` (section 7.11). The pool tests a connection before use with `test_before_acquire = true`. A pool exhaustion wait above `db.pool_wait_warn` produces a `WARN` event. The `Login` use case acquires its connection only after the Argon2 semaphore permit, so a login flood cannot hold the pool.

### 7.6 PostgreSQL settings for data safety

| Setting | Value | Reason |
|---|---|---|
| `fsync` | `on` | A commit is on disk before the client sees it |
| `synchronous_commit` | `on` | Same |
| `full_page_writes` | `on` | Protects against torn pages after a power loss |
| `wal_level` | `replica` | Required by `pg_receivewal` |
| `archive_mode` | `off` | `walarchive` streams with `pg_receivewal`. No archive command inside the container |
| `max_wal_senders` | `pg.max_wal_senders` (7.11) | One for `walarchive`, spare for a future replica |
| `wal_keep_size` | `pg.wal_keep_size` (7.11) | Bridges a short outage of `walarchive` |
| `checkpoint_timeout` | `pg.checkpoint_timeout` (7.11) | Bounded recovery time |
| `shared_buffers` | `pg.shared_buffers` (7.11) | Default guidance |
| `log_min_duration_statement` | `db.slow_query` (7.11) | Slow statements appear in the database log |
| `log_lock_waits` | `on` | Every lock wait above `deadlock_timeout` is logged |
| `default_transaction_isolation` | `read committed` | Every command except `SetUserRole` and `DeactivateUser` runs in it. Those two set `SERIALIZABLE` explicitly (ADR-0015, section 7.11) |
| `timezone` | `UTC` | All timestamps are `TIMESTAMPTZ`. The UI converts to the plant time zone |
| data checksums | on, set at `initdb` | Detects silent corruption. Cannot be enabled later without a rebuild |
| collation | `C` for `lc-collate`, `C.UTF-8` for `lc-ctype`, set at `initdb` | A glibc update cannot corrupt text indexes. The UI sorts names with `COLLATE "und-x-icu"` |

### 7.7 Backup and restore

- The `backup` service runs `pg_dump --format=custom --exclude-table-data=session --exclude-table-data=login_attempt` daily at 02:00 UTC to the `backups` volume. File name `recipe_<YYYYMMDD_HHMM>.dump`. The same run writes `globals_<YYYYMMDD_HHMM>.sql` with `pg_dumpall --globals-only`, because roles are not inside a `pg_dump`.
- Each dump has a manifest `<name>.sha256`. The restore script verifies the manifest before it restores.
- Retention is 30 daily dumps. Older files are deleted after a successful new dump.
- `walarchive` streams WAL with `pg_receivewal --synchronous` to a bind mount on a second disk. A weekly `pg_basebackup` goes to the same disk. Both connect as `recipe_replicator`, which the `pg_hba.conf` line of `data-model.md` 5.1 admits. Recovery point objective: minutes. Recovery time objective: 1 hour (ADR-0020). Without the second disk the recovery point objective is 24 hours (A35).
- The backup healthcheck fails when the backup path and `pgdata` are on the same filesystem.
- The latest dump and the WAL archive are copied to the external backup storage by a site procedure (ADR-0020). The dump contains password hashes and the audit log. The site procedure MUST protect the copies with the same access control as the database.
- A restore test runs each week and before each release. The script restores the latest dump into a scratch database. It counts the rows in each table and compares the counts with the source. It runs the audit chain check of `data-model.md` 4.9. It writes the result to the event log. A failed restore test is an `ERROR` event.
- After a restore into production, the operator truncates `session` and `login_attempt`. The operator then re-applies every user deactivation and password reset recorded after the dump time, from the off-site audit copy when one exists. Last, the operator runs the audit chain check.

### 7.8 Volumes, resources, disk full

| Volume | Content |
|---|---|
| `pgdata` | PostgreSQL data directory. Mounted read-only into `app` for the disk check |
| `backups` | Dumps from 7.7. Mounted read-only into `app` for the job `alerts` |
| `walarchive` | Bind mount on a second disk. WAL segments and base backups. Mounted read-only into `app` for the job `alerts` |
| `caddy` | TLS certificates |

Resource limits per container are the keys `container.*` of 7.11. The Argon2 semaphore keeps hashing memory at 4 × 19 MiB, which is 15 % of the `app` limit.

Disk space policy (Assumption A24):

- The job `disk_check` in `app` measures the free space of the `pgdata` volume with `statvfs` on its read-only mount every `health.disk_check_interval`.
- Warning threshold: `disk.warn_percent` free. The application writes a `WARN` event every `disk.warn_log_interval`, and the job `alerts` raises the condition.
- Hard threshold: `disk.readonly_percent` free. The application enters read-only mode. `Login`, `Logout`, every query, and every export continue. Every other command returns `503 E-WEB-006`. The only audit actions in read-only mode are `UserLoggedIn`, `UserLoggedOut`, and `VersionExported`. Writes to `session`, `login_attempt`, and `security_event` continue, and so do the supervised jobs. These writes are small, and the margin of `disk.readonly_percent` is reserved for them. The UI shows a banner.
- The application leaves read-only mode without a restart when free space rises above `disk.resume_percent`.
- PostgreSQL itself stops at 0 % free. The hard threshold exists so that the application stops first.

### 7.9 Clock, logs, and alerts

- All containers use the host clock. The host runs NTP. A clock skew above `clock.skew_warn` between `app` and `db` produces a `WARN` event. The job `clock_check` compares the clocks with `SELECT now()` at startup and every `clock.check_interval`.
- Log rotation is the Docker driver from section 3.5.
- The supervised job `alerts` in `app` produces the alerts. Section 3.7 lists the conditions.
- The job `audit_chain_check` records `audit.last_check_seq` and `audit.last_check_at` in `app_setting` after each completed check (`data-model.md` 4.9). The job `alerts` raises an alert when no check completed in the last 24 h (3.7).

### 7.10 Upgrade procedure

1. Read the release notes. Note whether the migration follows the expand pattern.
2. Pull the new images. Do not stop the running stack yet.
3. `docker compose up -d backup-premigrate migrate`. The `_premigrate` dump is written first. Wait for exit code 0 of `migrate`. On failure, stop here. The old `app` keeps running on the old schema when the migration follows the expand pattern.
4. `docker compose up -d app`. Docker replaces the container. Downtime is the graceful shutdown window plus startup, under 60 s.
5. Check `/readyz` on the internal listener, log in, open one recipe, export one version and check its hash.
6. Rollback: `docker compose up -d app` with the previous image tag. If the migration did not follow the expand pattern, stop `app`, restore the `_premigrate` dump, then start the previous image.
7. Major PostgreSQL upgrade (at least twice in 10 years): follow `docs/ops/runbook.md` section "Major upgrade". It uses `pg_dumpall --globals-only` plus `pg_dump -Fc` into a new `db` volume created with the same `initdb` arguments, then a restore test, then the switch. There is no `pg_upgrade` (ADR-0020). `docs/ops/runbook.md` is written in plan P4.1. Until then, this section is the runbook.

### 7.11 Settings

This table is the single source for every numeric setting. The configuration file uses the key in the first column. Other documents link here. A value repeated elsewhere must equal this table.

| Key | Value | Used by |
|---|---|---|
| `session.idle_timeout` | 30 min | api.md 7.2 |
| `session.absolute_timeout` | 12 h | api.md 7.2, data-model.md 3.6 |
| `session.cleanup_interval` | 10 min | 2.4, api.md 7.2, data-model.md 3.6 |
| `session.cookie_name` | `__Host-recipe_session`, Secure, HttpOnly, SameSite=Strict, Path=/ | api.md 7.1 |
| `session.token_bits` | 128. The database stores SHA-256 of the token | api.md 7.1 |
| `session.secure` | `true`. `false` refuses to start without the `dev` feature | api.md 7.1 |
| `login.delay_after_failures` | 3 failures per (login, client_ip) in 15 min | api.md 7.4, data-model.md 6 |
| `login.delay_formula` | `2^(n-3)` s, cap 60 s | api.md 7.4, data-model.md 6 |
| `login.ip_budget` | 100 failures per client_ip in 15 min, then 429 | api.md 7.4, data-model.md 6 |
| `login.attempt_retention` | 24 h | data-model.md 3.7 |
| `login.attempt_cleanup_interval` | 10 min | 2.4, data-model.md 3.7 |
| `argon2.m_cost` / `t_cost` / `p_cost` | 19 MiB / 2 / 1 | api.md 7.4 |
| `argon2.permits` / `argon2.acquire_timeout` | 4 / 2 s | 7.4, 7.5, api.md 7.4, data-model.md 6 |
| `password.min` / `password.max` | 12 / 128 characters, breached-password list | api.md 7.4 |
| `security.login_hmac_key_file` | `/run/secrets/login_hmac_key`. 32 bytes. HMAC-SHA256 key of `security_event.login_hmac` | 7.1, data-model.md 3.8 |
| `security.csrf_signing_key_file` | `/run/secrets/csrf_signing_key`. 32 bytes. Signing key of the pre-session CSRF cookie | 7.1, api.md 7.3 |
| `security.key_rotation_overlap` | 24 h. The old and the new key are both accepted | 7.1 |
| `secrets.replicator_password_file` | `/run/secrets/replicator_password`. Password of `recipe_replicator`, read by the initdb script and by `walarchive` | 7.1, 7.7, data-model.md 5.1 |
| `i18n.lang_cookie_max_age` | 365 days. Cookie `recipe_lang` | api.md 8 |
| `approval.separation_of_duties` | `true` | 7.12, api.md 3.2, ADR-0022 (A34) |
| `http.request_timeout` | 30 s | 7.4 |
| `http.body_read_timeout` | 10 s | 7.4 |
| `http.body_limit.form` | 256 KiB | api.md 7.6 |
| `http.body_limit.value` | 1 MiB | api.md 7.6 |
| `http.body_limit.translation` | 64 KiB | api.md 7.6 |
| `http.trusted_proxies` | CIDR list, default the Compose network | api.md 7.1 |
| `http.public_port` / `http.internal_port` | 8080 / 9000 | 7.1, 7.2 |
| `http.retry_after` | 5 s. The `Retry-After` header of a 503 response | api.md 5.1 |
| `http.hsts_max_age` | 31536000 s (365 days), with `includeSubDomains` | api.md 7.1, api.md 9 |
| `http.user_agent_max` | 256 characters | 3.2, data-model.md 1.3, data-model.md 3.8 |
| `log.session_ref_length` | 16 hex characters of the session id hash | 3.2, data-model.md 1.3, data-model.md 3.15 |
| `db.statement_timeout` | 10 s | 7.4 |
| `db.lock_timeout` | 2 s | 7.4 |
| `db.idle_in_transaction_timeout` | 15 s | 7.4 |
| `db.pool_acquire_timeout` | 5 s | 7.4 |
| `db.pool_size` / `db.health_pool_size` | 10 / 2 | 7.5 |
| `db.pool_idle_timeout` / `db.pool_max_lifetime` | 10 min / 30 min | 7.4 |
| `db.pool_wait_warn` | 1 s | 7.5 |
| `db.max_connections` | 50 (PostgreSQL `max_connections`) | 7.5 |
| `db.slow_query` | 1 s (PostgreSQL `log_min_duration_statement = 1000`) | 3.3, 7.6 |
| `timeout.export` | 10 s | 7.4 |
| `timeout.validation` | 10 s | 4.4, 7.4 |
| `migrate.lock_timeout` / `migrate.retries` / `migrate.retry_pause` | 2 s / 10 / 5 s | 5, data-model.md 8 |
| `text.name_max` | 200 | 4.3, data-model.md 1.3 |
| `text.description_max` / `text.change_note_max` | 4 000 / 4 000 | 4.3, data-model.md 1.3 |
| `text.unit_name_max` / `text.tag_max` | 200 / 64 | 4.3 |
| `json.max_depth` | 32 | 4.4 |
| `type.max_dimensions` | 3 (A38) | 4.3, 4.4, domain-model.md 2.1, data-model.md 3.3 |
| `type.max_members` | 1024 | domain-model.md 2.1, data-model.md 3.4 |
| `budget.leaves_per_parameter` | 16 384 | 4.4 (A26) |
| `budget.leaves_per_version` | 262 144 | 4.4 (A26) |
| `budget.limit_rows_per_version` | 32 768 | 4.4 (A26) |
| `budget.parameters_per_version` | 4 000 | 4.4 (A26) |
| `budget.value_bytes` | 1 MiB per parameter value | data-model.md 3.12 |
| `parameter.ordinal_max` | 100 000 | data-model.md 3.12, domain-model.md 2.2 |
| `setting.value_max` | 1 000 characters | data-model.md 3.17 |
| `page.parameters` | 100 rows | api.md 2.6 |
| `health.readiness_cache` | 5 s | 7.2 |
| `health.db_check_timeout` | 2 s | 7.2 |
| `health.check_interval` / `health.check_failures` | 10 s / 3 | 7.1, 7.2 |
| `health.disk_check_interval` | 30 s | 2.4, 7.8 |
| `disk.warn_percent` / `disk.readonly_percent` / `disk.resume_percent` | 15 / 5 / 10 | 3.7, 7.8, api.md 5.1, overview.md A24 |
| `disk.warn_log_interval` | 10 min | 7.8 |
| `watchdog.probe_interval` / `watchdog.failures` / `watchdog.window` | 10 s / 3 / 30 s | 2.4 |
| `job.restart_delay` | 10 s | 2.4 |
| `audit_chain.check_time` | 03:00 UTC, every night | 2.4, data-model.md 4.9 |
| `audit.check_batch` | 10 000 rows per statement of the audit chain check | 2.4, data-model.md 4.9 |
| `audit.last_check_seq` | State key in `app_setting`, not a configuration file key. The last `chain_seq` of the last completed audit chain check | 2.4, 7.9, 7.12, data-model.md 3.17, data-model.md 4.9 |
| `audit.last_check_at` | State key in `app_setting`, not a configuration file key. The end time of the last completed audit chain check. The job `alerts` raises an alert when it is older than 24 h | 2.4, 3.7, 7.9, 7.12, data-model.md 3.17, data-model.md 4.9 |
| `alerts.interval` | 60 s | 2.4, 3.7, data-model.md 6 |
| `alerts.webhook_url` | empty, which means no webhook | 3.7 |
| `alerts.panic_count` / `alerts.panic_window` | 3 / 10 min | 2.4, 3.7 |
| `alerts.not_ready_for` | 1 min | 3.7 |
| `alerts.failed_login_count` / `alerts.failed_login_window` | 50 / 15 min | 3.7 |
| `alerts.security_window` | 15 min | 3.7 |
| `backup.time` / `backup.retention` | 02:00 UTC / 30 days | 7.7 |
| `backup.max_age_alert` / `wal.max_age_alert` | 26 h / 15 min | 3.7, 7.1 |
| `security_event.retention` | 400 days. A constant in `security_event_purge()`. A change needs a migration | 3.6, data-model.md 4.12 |
| `security_event.purge_interval` | 24 h | 2.4, data-model.md 4.12 |
| `log.max_size` / `log.max_file` | 50m / 20 | 3.5 |
| `soak.memory_ceiling` / container limit | 384 MiB / 512 MiB | 6, 7.8 |
| `container.app_memory` / `container.app_cpus` | 512 MiB / 1 | 7.8 |
| `container.db_memory` / `container.db_cpus` | 2 GiB / 2 | 7.8 |
| `container.small_memory` | 128 MiB each for `proxy`, `backup`, and `walarchive` | 7.8 |
| `pg.max_wal_senders` | 3 | 7.6 |
| `pg.wal_keep_size` | 1 GB | 7.6 |
| `pg.checkpoint_timeout` | 5 min | 7.6 |
| `pg.shared_buffers` | 25 % of the `db` container memory | 7.6 |
| `shutdown.drain` / `stop_grace_period` | 30 s / 40 s | 7.3 |
| `clock.skew_warn` | 5 s | 7.9 |
| `clock.check_interval` | 1 h | 2.4, 7.9 |

Transaction isolation and retry are rules in the code, not settings. `SetUserRole` and `DeactivateUser` run in `SERIALIZABLE`. They retry once on `40001`, then return `E-DB-001`. Every other command runs in `READ COMMITTED` with no retry. `40P01` maps to `E-DB-001` and is logged at `ERROR` (section 1.1).

### 7.12 Stored settings

`app_setting` (`data-model.md` 3.17) stores the effective value of every key of section 7.11 that the server reads from its configuration file (ADR-0022). The server compares the values at startup, before the listeners open:

1. The server reads its configuration and checks every value.
2. In one transaction it reads `app_setting` and compares each configured value with the stored value. A missing row counts as a difference with the old value `null`.
3. For each difference it writes one audit entry `SettingChanged` with `aggregate_type = 'app_setting'`, `actor = None`, and the old and the new value. All entries of one start share one `request_id`. Then it updates or inserts the row.
4. When the transaction fails, the server writes one `ERROR` line and does not start.

The state keys `audit.last_check_seq` and `audit.last_check_at` are not configuration. The comparison skips them. The job `audit_chain_check` writes them with no audit entry (2.4).

A change of `approval.separation_of_duties` therefore always has an audit entry with the old and the new value (A34). The audit log keeps the entry without a time limit.

## 8. Code conventions

- Code comments, identifiers, commit messages, documentation, and specifications are English (R42). Comments follow the ASD-STE100 structural rules in the same way as this document.
- Each design pattern in the code names the problem it solves in the doc comment of the type that implements it (R43). A review rejects a pattern with no named problem.
- `CONTRIBUTING.md` in the repository root repeats the rules of sections 2 and 8 and links to this document.
- `ste-lint.py` (`.claude/skills/asd-ste100/scripts/ste-lint.py`) checks the structural ASD-STE100 rules of English text (R40). CI runs it on every Markdown file in `docs/`. One hard violation fails the job. Hard violations:
  - a semicolon in prose,
  - a sentence of more than 25 words,
  - two verbs from one synonym group in one file, for example `check` and `verify`,
  - a soft phrasal verb, a marketing adjective, or a nominalization,
  - a list item that ends with `and` or `or`.

  Passive voice and compound tenses are advisory. They do not fail the job.
- `rustfmt` with the workspace `rustfmt.toml`. CI runs `cargo fmt --check`.
- CI runs `cargo clippy --all-targets --all-features -- -D warnings` with the workspace lints from section 2.1.
- `cargo deny check` runs in CI. It fails on a denied license, a yanked crate, or a security advisory. The allow list is in `deny.toml`.
- `cargo audit` runs nightly against the `Cargo.lock`.
- `Cargo.lock` is committed. Dependency updates happen once per month in one pull request, plus at once for a security advisory.
- Minimum supported Rust version is the current stable at release time, recorded in `rust-version` of each crate. The project targets edition 2024.
- No `unsafe`. No `static mut`. No global mutable state. Shared state is passed through axum `State` as `Arc<AppState>` with interior mutability only inside the connection pool, the Argon2 semaphore, and the read-only mode flag (`AtomicBool`).
- Every public type and function in a library crate has a doc comment. `cargo doc --no-deps` runs in CI with `-D warnings` for broken links.
- A pull request needs: green CI, one review, and an entry in `CHANGELOG.md` when behavior changes.
