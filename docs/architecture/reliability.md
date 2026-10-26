# Reliability

Status: Accepted (design stage, revised 2026-10-12 after review)
Source of requirements: `general-draft.txt` (R29 to R38, R42, R43)
Related: `overview.md`, `domain-model.md`, `data-model.md`, `api.md`, ADR-0014, ADR-0015, ADR-0019, ADR-0020, ADR-0022, ADR-0057 to ADR-0071

This document specifies how the system stays correct and available during continuous 24/7 operation. Each section names the rule and the mechanism that enforces it. Section 7.11 is the single source for every numeric setting. Other documents link to it. A value repeated elsewhere must equal 7.11.

## 1. Error handling

### 1.1 Error types per crate

Each library crate owns its error types, with two exceptions. `recipe-export` returns `ExportError` of `recipe-domain`. `recipe-db` maps into `StorageError` of `recipe-app`. The binary crate uses `anyhow`. Every variant has exactly one owner.

| Crate | Error type | Variants |
|---|---|---|
| `recipe-domain` | `PlantError`, `RecipeError`, `IdentityError`, `ExportError` | One `thiserror` enum per bounded context. Leaf errors. Every row with prefix `E-PLT`, `E-RCP`, `E-IDN` and a number below 030 in section 1.2, and every `E-RCP-050..069` row. `ExportError` lives in the module `export` and owns every `E-EXP` row: `NoApprovedVersion { reference }`, `VersionNotExportable { reference, number }`, `ContentHashMismatch { reference, number }`, `Serialize(#[from] serde_json::Error)`. Two carriers own no code (ADR-0027): `RecipeErrors` holds one or more `RecipeError` values, and `DraftError` holds a `RecipeError`, a `PlantError`, or a `RecipeErrors`. `ValueFaults` holds the faults of one value and owns no code. A draft function turns it into `RecipeErrors` (ADR-0041). `LeafTextsError` is a parse reason of `LeafTexts` without a code (ADR-0025, ADR-0039) |
| `recipe-app` | `AppError`, `AuditError`, `StorageError` | `AppError`: `#[from] PlantError`, `#[from] RecipeError`, `#[from] RecipeErrors`, `#[from] IdentityError`, `#[from] ExportError`, plus `Forbidden { operation }`, `Unauthenticated`, `NotFound { entity }`, `ConcurrentEdit { reference, number, expected, actual }`, `Timeout { stage }`, `TaskFailed { stage }`, `AuditWriteFailed`, `Storage(#[from] StorageError)`, `Audit(#[from] AuditError)` (ADR-0089 (a)). `stage` of `Timeout` is `Argon2Permit`, `Validation`, `Export`, `Request`, or `Diff` (ADR-0050). `stage` of `TaskFailed` is `PasswordHash` or `Validation`, the two kinds of `spawn_blocking` work (2.2), `Export` (ADR-0029), `Token`, a failed read of random bytes from the operating system (ADR-0035), or `Diff` (ADR-0050). `AuditError`: `ChainBroken { chain_seq }`, raised by the job `audit_chain_check`. The job returns it as `AppError::Audit`, and `AppError::code()` gives its code `E-AUD-002`. `StorageError` is the error type of the storage ports: `Duplicate { constraint }`, `MissingReference { constraint }`, `Constraint { code, message }`, `Deadlock { cause }`, `StatementTimeout`, `PoolTimeout`, `Unavailable`, `Other { sqlstate, source }`. `cause` of `Deadlock` is `Deadlock`, `LockTimeout`, or `Serialization` (ADR-0035). `source` is a boxed `std::error::Error`, so `recipe-app` holds no `sqlx` type. `WebhookFault` is the reason of a failed call of the port `AlertWebhook` and owns no code, like `ValueFaults` (ADR-0025, ADR-0060 (r), ADR-0063 (n)) |
| `recipe-db` | none public | Maps `sqlx::Error` into `StorageError` of `recipe-app` with the rule below. `recipe-db` depends on `recipe-app`, and `recipe-app` does not depend on `recipe-db` (`overview.md` 5) |
| `recipe-export` | none | No error type of its own. The JSON writer of the HTTP response returns `ExportError` of `recipe-domain` |
| `recipe-web` | `WebError` | `#[from] AppError`, plus `PayloadTooLarge { limit }`, `BadRequest { field, reason }`, `BadRow { row, field, reason }` (ADR-0037), `BadQuery { field, reason }` (a query value with no input on the page, ADR-0050 (e)), `CsrfRejected`, `ReadOnlyMode`, `Internal { request_id }`. Two carriers own no code. `Fields { lines, form }` holds a non-empty list of field errors, and `code()` returns the code of the first line (ADR-0041). `Conflict { error, view }` holds `AppError::ConcurrentEdit` and the data of the last change, and `code()` returns `E-WEB-001` of the inner error (ADR-0043). `WebError` has no `#[from] ExportError`. An `ExportError` of the JSON writer becomes `AppError` first and reaches `WebError` only through `AppError` |
| `recipe-server` | `anyhow::Error` | Everything, only in `main()`, configuration loading, subcommands, and wiring |

Rules:

- Every error variant carries the values that identify the failure (recipe reference, version number, tag, leaf path). It never carries a password, a password hash, a session token, or a cookie value.
- Every error type implements `Display` for the user message and for the log line. `Debug` on a type that holds a secret redacts it. The log never uses `Debug`.
- Every error type has a `code(&self) -> ErrorCode` method. The code is stable. The message template can change. The code cannot.
- A value object without its own row in section 1.2 returns a reason from `parse`. A reason is a field type, not an error type, and has no `code()`. The caller wraps it into its coded error (ADR-0025). `WebhookFault`, the reason of a failed call of the port `AlertWebhook`, is a reason of the same kind (ADR-0060 (r)). `DiskFault` of the port `DiskSpace` and `BackupFault` of the port `BackupDirectory` are reasons of the same kind too (ADR-0073 (b)).
- `?` converts between layers through `From`. A `map_err` closure appears only when it adds context.
- `recipe-db` maps `sqlx::Error` to `StorageError` by SQLSTATE in this order: a `RAISE EXCEPTION` whose message starts with a trigger code from section 1.2 becomes `StorageError::Constraint { code, message }` with that code. The trigger codes are `E-PLT-030..049`, `E-RCP-030..049`, and `E-AUD-003`. `23505` becomes `Duplicate`. `23503` becomes `MissingReference`. `23514` without a trigger code becomes `Constraint { code: "E-DB-003", .. }`. `40P01` becomes `Deadlock { cause: Deadlock }`. `40001` becomes `Deadlock { cause: Serialization }`. `57014` becomes `StatementTimeout`. `55P03` becomes `Deadlock { cause: LockTimeout }` (a lock wait that hit `lock_timeout`). A pool acquire timeout becomes `PoolTimeout`. A connection error becomes `Unavailable`. Any other code becomes `Other`.
- `SetUserRole` and `DeactivateUser` run in `READ COMMITTED` with no retry. They take the class-4 guard lock of the last Administrator first (ADR-0096). Only the first-administrator operation of `create-admin` runs in `SERIALIZABLE`. It runs once more on `Deadlock { cause: Serialization }`, also when it comes from the audit insert or from the commit. A second serialization failure returns `E-DB-001` (ADR-0035). Every other command runs in `READ COMMITTED` with no retry. `40P01` maps to `E-DB-001` and is logged at `ERROR`, because a deadlock is a bug in the lock order (ADR-0015).
- `recipe-app` maps `StorageError::Duplicate` to the domain duplicate error of the use case (`DuplicateReference`, `DuplicateMachineCode`, `DuplicateUnitPath`, `DuplicateLogin`, `DuplicateTag`, `DuplicateTypeName`, `DuplicateMemberName`, `DraftAlreadyExists`) by the constraint name. A `Duplicate` with an unknown constraint name stays `AppError::Storage` with code `E-DB-006`. The same rule maps `StorageError::MissingReference` by the constraint name, for example `unit_parent_exists` to `ParentNotFound`. An unknown name gives `E-DB-007`.
- `AppError::Timeout { stage }` owns `E-WEB-009`. The Argon2 permit, the snapshot validation, the export, and the diff of two versions (ADR-0050) use it. The request timeout layer of `recipe-web` returns `AppError::Timeout { stage: Request }` (ADR-0058 (g)), so every timeout has one code.
- A body that does not arrive within the body deadline of 7.4 also gives `AppError::Timeout { stage: Request }` (`E-WEB-009`). Every other error of the body read stays `E-WEB-003` (ADR-0058 (j)).
- `AppError::TaskFailed { stage }` owns `E-WEB-011`. The use case maps the `JoinError` of a `spawn_blocking` task to it (2.4). `E-WEB-010` stays the code of a panic that one of the two panic layers of 2.4 rule 3 catches.
- An error is logged exactly once. The log statement is in the `WebError` to HTTP mapping in `recipe-web`. Lower layers add context and propagate. They do not log the error.
- The backstop panic layer of 2.4 rule 3 writes the line of its own `E-WEB-010` answer, because `render_errors` does not run for it (ADR-0057 (g)).
- A caught panic gives two `ERROR` lines: the event of the panic hook and the line of `E-WEB-010`. These are two events, so the rule above stays true (ADR-0057 (f)).
- A background job logs a handled fault once at `WARN` and continues, for example a `WebhookFault`. It returns `Err` only for an error of level `ERROR` of 3.3. The supervisor logs that `Err` once inside the span `job` of 3.2 (ADR-0060 (p), (q), ADR-0088 (a)). The one exception is the failed snapshot read of the job `alerts` in 3.7. The supervisor logs it at `WARN` (ADR-0089 (d)).
- The log line of an error holds its code and the `Display` text of each error in the chain. It never holds the `Debug` output of an error, because the `Debug` of a database error holds the row values of the detail (ADR-0035).
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
| `E-PLT-014` | `PlantError::UnitReferencedByApproved { path, reference }` | 409 | Unit {path} is used by the approved version of recipe {reference}. An Administrator can retire the version. |
| `E-PLT-015` | `PlantError::DuplicateTypeName { type_name }` | 409 | Type {type_name} already exists on this machine. Create a new revision instead. |
| `E-PLT-016` | `PlantError::StructHasNoMembers { type_name }` | 422 | Type {type_name} has no members. Add at least one member. |
| `E-PLT-017` | `PlantError::InvalidUnitPath { text }` | 422 | {text} is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2. |
| `E-PLT-018` | `PlantError::TypeTooLarge { type_name, limit }` | 422 | Type {type_name} is larger than the limit {limit}. Use fewer elements, fewer members, or fewer levels. |
| `E-PLT-019` | `PlantError::MachineNotFound { code }` | 404 | Machine {code} does not exist. Select an existing machine. |
| `E-PLT-020` | `PlantError::UnitNotFound { path }` | 404 | Unit {path} does not exist on this machine. Select an existing unit. |
| `E-PLT-021` | `PlantError::DuplicateMemberName { type_name, member }` | 422 | Type {type_name} has two members with the name {member}. Give each member a different name. |
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
| `E-RCP-010` | `RecipeError::RecipeHasApprovedVersion { reference }` | 409 | Recipe {reference} has an approved version. An Administrator can retire the version. |
| `E-RCP-011` | `RecipeError::DraftAlreadyExists { reference, number }` | 409 | Recipe {reference} already has draft version {number}. Edit or discard that draft. |
| `E-RCP-012` | `RecipeError::RecipeDeleted { reference }` | 409 | Recipe {reference} is deleted. An administrator can restore it. (On export: only `/export/v1/recipes/{ref}/versions/{n}` returns this code. `/approved` of a deleted recipe returns 404 `E-EXP-001`.) |
| `E-RCP-013` | `RecipeError::TypeNotOnPlatform { tag, type_name, platform }` | 422 | Parameter {tag} uses type {type_name}. This type does not exist on {platform}. (Only an item inside `E-RCP-020 ValidationFailed`. The snapshot check reports it for a stored parameter whose type is no longer on the platform. A `TypeExpr` that is not on the platform is `E-PLT-013`, or `E-PLT-006` for a member of `DefineUdt`.) |
| `E-RCP-014` | `RecipeError::DuplicateTag { tag, number }` | 409 | Parameter tag {tag} is already used in version {number}. |
| `E-RCP-015` | `RecipeError::ApproverEditedDraft { reference, number, login }` | 409 | User {login} created or edited draft {number} of recipe {reference}. A different user must approve it. |
| `E-RCP-016` | `RecipeError::LimitMissing { tag, leaf }` | 422 | {tag}{leaf} has no process limit. Set a limit before approval. |
| `E-RCP-017` | `RecipeError::LeafPathNotFound { tag, leaf }` | 422 | Path {leaf} does not exist in the type of {tag}. |
| `E-RCP-018` | `RecipeError::MinGreaterThanMax { tag, leaf, min, max }` | 422 | The minimum {min} of {tag}{leaf} is greater than the maximum {max}. |
| `E-RCP-019` | `RecipeError::LimitNotApplicable { tag, leaf }` | 422 | {tag}{leaf} does not accept this limit. A BOOL, CHAR, or WCHAR leaf has no limit. A string leaf has only a maximum length. |
| `E-RCP-020` | `RecipeError::ValidationFailed { items }` | 422 | {count} parameters are not valid. See the list. |
| `E-RCP-021` | `RecipeError::SizeBudgetExceeded { budget, limit, actual }` | 422 | The version exceeds the limit of {limit} {budget}. Current count: {actual}. |
| `E-RCP-022` | `RecipeError::S7AddressOverlap { tag, other_tag, address }` | 422 | Address {address} of {tag} overlaps with {other_tag}. |
| `E-RCP-023` | `RecipeError::S7AddressMisaligned { tag, address }` | 422 | Address {address} of {tag} must start at an even byte. |
| `E-RCP-024` | `RecipeError::TypeRevisionOutdated { tag, type_name, current, newest }` | 422 | {tag} uses revision {current} of {type_name}. The newest revision is {newest}. Rebind the parameter. |
| `E-RCP-025` | `RecipeError::RecipeNotDeleted { reference }` | 409 | Recipe {reference} is not deleted. |
| `E-RCP-026` | `RecipeError::RecipeHasRetiredVersion { reference }` | 409 | Recipe {reference} has a retired version. Only an administrator can delete it. |
| `E-RCP-027` | `RecipeError::InvalidLangCode { lang }` | 422 | {lang} is not a language code. Use a code such as pl or de-AT. |
| `E-RCP-028` | `RecipeError::RebindNeedsValue { tag, leaf }` | 422 | {tag}{leaf} is a new member. Give it a value. |
| `E-RCP-029` | `RecipeError::RebindDropsLimit { tag, leaf }` | 422 | {tag}{leaf} has a limit, but the new revision has no such member or gives the member another type. Delete the limit first. After the rebind, set the limit again. |
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
| `E-RCP-054` | `RecipeError::S7AddressWidthMismatch { tag, address, width }` | 422 | Address {address} of {tag} does not match the type of {tag}. Use the width {width}. |
| `E-IDN-001` | `IdentityError::DuplicateLogin { login }` | 409 | Login {login} is already in use. |
| `E-IDN-002` | `IdentityError::LastAdministrator { login }` | 409 | {login} is the last active administrator. Add another administrator first. |
| `E-IDN-003` | `IdentityError::InvalidCredentials` | 401 | Login or password is not correct. (An unknown login and an inactive user get the same message on purpose, A42.) |
| `E-IDN-004` | `IdentityError::UserInactive` | 401, as `E-IDN-003` | Not shown to the user. `Login` returns `E-IDN-003` with the same body and timing (A42). `UserInactive` is recorded as the `security_event` kind `login_rejected_inactive` and in the event log. |
| `E-IDN-005` | `IdentityError::LoginThrottled { retry_after }` | 429 | Too many failed logins from this address. Try again in {retry_after} seconds. |
| `E-IDN-006` | `IdentityError::PasswordPolicy { reason }` | 422 | The password is not accepted: {reason}. Use 12 to 128 characters that are not in the breached-password list. |
| `E-IDN-007` | `IdentityError::MustChangePassword` | 303 | Not an error page. The server redirects to `/me/password`. |
| `E-IDN-008` | `IdentityError::InvalidCurrentPassword` | 422 | The current password is not correct. |
| `E-IDN-009` | `IdentityError::UserAlreadyActive { login }` | 409 | User {login} is already active. |
| `E-IDN-010` | `IdentityError::UserAlreadyInactive { login }` | 409 | User {login} is already inactive. |
| `E-AUD-001` | `AppError::AuditWriteFailed` | 500 | The change was not saved. The system could not record the audit entry. Contact the administrator. Reference: {request_id}. |
| `E-AUD-002` | `AuditError::ChainBroken { chain_seq }`, check of data-model.md 4.9 | 500 | Audit entry {chain_seq} does not match the hash chain. Contact the administrator. The job `audit_chain_check` returns this code after a nightly run or a catch-up run. The supervisor writes it to the event log (ADR-0089 (a)). The job `alerts` raises the alert. No user request returns it. |
| `E-AUD-003` | `StorageError::Constraint { code: "E-AUD-003", .. }`, trigger `security_event_guard` of data-model.md 5.4 | 500 | A security event cannot change. Contact the administrator. Reference: {request_id}. |
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
| `E-WEB-006` | `WebError::ReadOnlyMode` | 503 | The system is in read-only mode because the free disk space is low or unknown. Contact the administrator. |
| `E-WEB-007` | `WebError::CsrfRejected` | 403 | The form is no longer valid. Reload the page and try again. |
| `E-WEB-008` | `WebError::BadRequest { field, reason }`, `WebError::BadQuery { field, reason }` (ADR-0050 (e)), `WebError::BadRow { row, field, reason }` | 422 | Field {field} is not valid: {reason}. |
| `E-WEB-009` | `AppError::Timeout { stage }` | 503 | The request took too long and was stopped. Try again. Reference: {request_id}. |
| `E-WEB-010` | `WebError::Internal { request_id }` | 500 | An internal error occurred. Contact the administrator. Reference: {request_id}. |
| `E-WEB-011` | `AppError::TaskFailed { stage }` | 500 | The operation {stage} did not complete. Try again. Reference: {request_id}. |

A trigger code (`E-PLT-030..038`, `E-RCP-030..036`, `E-AUD-003`) reaches the user only when a domain check did not fire first. No command of the application changes a security event, so `E-AUD-003` always marks a defect. The HTTP layer returns the status of the row: 409 for a state rule, 422 for a shape rule. The event log gets an `ERROR` line, because the domain check should have caught the condition. This registry is the authority for every code. `data-model.md` section 4.11 maps each trigger code to its function.

Codes after `E-RCP-029`: `E-RCP-050 TranslationNotFound` (`RemoveTranslation`), `E-RCP-051 ParameterNotFound` (every draft command with `{key}` in the URL), `E-RCP-052 RebindTargetInvalid` (`RebindTypeRevision`), `E-RCP-053 S7AddressNotAllowed` (`AddParameter`, `UpdateParameterDefinition`), and `E-RCP-054 S7AddressWidthMismatch` (`AddParameter`, `UpdateParameterDefinition`, ADR-0024). `E-RCP-001..029` has no free number, so the block `E-RCP-050..069` holds them. The `{lang}` and `{key}` values of `E-RCP-050` and `E-RCP-051` come from the URL, so their status is 404. `api.md` 1 gives the rule. `E-RCP-051` and `E-RCP-050` are for a well-formed `{key}` or `{lang}` that the existing draft does not hold. `E-WEB-005` is for every other missing resource in the URL.

`E-PLT-019 MachineNotFound` and `E-PLT-020 UnitNotFound` are for a request field that names a machine or a unit that does not exist. Their status is 404 (`api.md` 1). `E-WEB-005` is for URL values only. `E-PLT-019` belongs to `CreateRecipe` (`machine_code`). `E-PLT-020` belongs to `AddParameter` and `UpdateParameterDefinition` (`unit_path`).

### 1.3 Reference shape

```rust
#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("Reference number {reference} is already in use.")]
    DuplicateReference { reference: ReferenceNumber },
    #[error("Version {number} of recipe {reference} is {status}. Create a new draft to make changes.")]
    VersionNotEditable { reference: ReferenceNumber, number: u32, status: VersionStatus },
    #[error("User {login} created or edited draft {number} of recipe {reference}. A different user must approve it.")]
    ApproverEditedDraft { reference: ReferenceNumber, number: u32, login: Login },
    // ... one variant per E-RCP row below 030 and in 050..069 in 1.2
}

impl RecipeError {
    pub fn code(&self) -> ErrorCode {
        match self { .. }
    }
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
| `clippy::print_stdout`, `clippy::print_stderr` | deny | All output of the code goes through `tracing`. Only the std `Termination` of `main` writes outside `tracing` (3.1, ADR-0080 (c)) |
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
| Regular expressions | The pattern is a literal in a `static` of type `LazyLock<Result<Regex, regex::Error>>`. The first use compiles it. The caller maps an `Err` to its own error type with `map_err`, never with `expect`. A unit test compiles every pattern, so a bad pattern fails the build |
| `Mutex` lock | Use `parking_lot::Mutex` whose `lock()` cannot return an error |
| Tokio `JoinHandle` | `handle.await` returns `Result<T, JoinError>`. The use case maps the `JoinError` of a `spawn_blocking` task to `AppError::TaskFailed { stage }` (`E-WEB-011`). Never `unwrap` it |
| Blocking inside async | Never `block_on` inside an async context. CPU-heavy work (Argon2id hashing, snapshot validation, export rebuild, diff of two versions) runs in `spawn_blocking` |
| Time | Use `time::OffsetDateTime::now_utc()`, which does not fail |

### 2.3 Permitted exceptions

- Test code (`#[cfg(test)]` and `tests/`) may use `unwrap`, `expect`, and indexing. The lint is set to `allow` for test targets.
- The watchdog function of 2.4 has the attribute `#[allow(clippy::exit)]`, because it must end a stalled process with `std::process::exit(70)`. It is the only function with this attribute. A comment on the attribute names this section.
- No other exception. `recipe-server::main` returns `anyhow::Result<()>`. A startup precondition that fails returns an error with context, and the process exits with code 1 after one `ERROR` line from `main`. A panic hook line is not counted (ADR-0061 (j)). A pull request that adds `#[allow(clippy::unwrap_used)]` outside tests is rejected in review.

### 2.4 Panic containment

A panic in application code is a bug, not an error. The system contains it and keeps serving (Assumption A41, R30):

1. `[profile.release]` keeps `panic = "unwind"`. The CI job `musl-unwind` proves the unwind on the musl target of the image (ADR-0064 (m)).
2. `recipe-server` installs a panic hook with `std::panic::set_hook` in step 1 of the start (ADR-0061). The hook writes one `tracing::error!` event with the panic message, the location, and the current `request_id` when a span is active. It increments the panic counter of 3.7. The hook does not end the process.
3. The router has two instances of `tower_http::catch_panic::CatchPanicLayer` (ADR-0057):
   - The inner layer sits directly inside `render_errors`. A panic inside a request becomes the `500 E-WEB-010` page, HTMX fragment, or JSON body of `api.md` 5 with the `request_id`.
   - The backstop sits directly inside the layer `request_id` and catches a panic of the outer layers. `api.md` 5.1 gives its plain text answer, the one exception to `api.md` 5.1 to 5.3 and 9 (ADR-0057 (d), (e), (j)).
   - The open transaction of the request is dropped and PostgreSQL rolls it back. No partial write survives (R29).
4. Every background job of the table below runs inside a supervisor loop. A panic in one iteration is caught by the task join, logged with the `job` field, and the job restarts after `job.restart_delay`.
5. Watchdog: an OS thread, started before the Tokio runtime, probes `/healthz` on the internal listener every `watchdog.probe_interval`. The probe is the probe of the subcommand `healthcheck` with the budget `watchdog.probe_timeout` (ADR-0059 (d), (e)). After `watchdog.failures` failed probes in `watchdog.window`, the thread writes one `ERROR` line and calls `std::process::exit(70)` (the exception of 2.3). Docker restarts the container. This covers a stalled runtime, which `restart: always` alone does not detect. The watchdog counts no failed probe before the internal listener opens, and it stops at the signal (ADR-0061 (g), ADR-0058 (l)).
6. Application code never calls `std::panic::catch_unwind` itself. Six places catch a panic:
   - the inner layer and the backstop of rule 3,
   - the supervisor in rule 4,
   - the join of a `spawn_blocking` task,
   - the `JoinSet` of the connection tasks of the accept loop (ADR-0058 (f)). That panic ends only its connection.
   - the `JoinSet` of the webhook calls of one run of the job `alerts` (ADR-0092). That panic gives `WebhookFault::Connection` for its own call only. The other calls keep their results, and the job does not restart.
7. A panic inside a `spawn_blocking` task (Argon2id hashing, snapshot validation, export rebuild, diff of two versions) reaches the use case as a `JoinError`. The use case maps it to `AppError::TaskFailed { stage }`, which returns `500 E-WEB-011`. The panic hook of rule 2 logs the panic.

A panic is a visible signal. The job `alerts` raises an alert when more than `alerts.panic_count` panic events occur in `alerts.panic_window` (section 3.7).

Supervised background jobs. They run inside `app`. The intervals are keys of section 7.11.

| Job | Work | Interval |
|---|---|---|
| `cleanup` | Deletes expired `session` rows every `session.cleanup_interval`. Deletes old `login_attempt` rows every `login.attempt_cleanup_interval`, in batches of `login_attempt.purge_batch` rows (`data-model.md` 6). Calls `security_event_purge()` once per batch of `security_event.purge_batch` rows when its last purge is older than `security_event.purge_interval` (`data-model.md` 4.12). A failed batch keeps the batches before it, and the next run continues. One loop runs the job at the shorter of the first two intervals, and the job keeps a due time for each cadence (ADR-0088 (c)). Uses the port `MaintenanceStore` and writes no audit entry | `session.cleanup_interval`, `login.attempt_cleanup_interval`, `security_event.purge_interval` |
| `disk_check` | Measures free space and sets the read-only mode (7.8). Runs once at the start before the listeners open (ADR-0061 step 6). The supervisor runs that first run with `run_at_start` (ADR-0089 (b)). On a system without `statvfs` it writes one `WARN` line per process. A failed measurement on a system with `statvfs` turns the read-only mode on (ADR-0073 (d)) | `health.disk_check_interval` |
| `audit_chain_check` | Recomputes the audit hash chain up to the head that it reads first (`data-model.md` 4.9). A batch ends at `audit.check_batch` rows or `audit.check_batch_bytes`. Each batch runs in its own short transaction under `audit.check_statement_timeout`. Writes a break through the port `SecurityEventSink`. After a completed check, it records `audit.last_check_seq` and `audit.last_check_at` in `app_setting` through the port `MaintenanceStore`, with no audit entry. The catch-up of 7.9 adds one run after a start | `audit.check_time`, `audit.catchup_delay` |
| `heartbeat` | Writes the state key `heartbeat.last_at` in `app_setting` through the port `MaintenanceStore`, with no audit entry. The write keeps the WAL stream active (7.7) | `heartbeat.interval` |
| `clock_check` | Compares the clocks of `app` and `db` (7.9) | `clock.check_interval` |
| `alerts` | Checks the alert conditions (3.7) | `alerts.interval` |

Rules of the jobs (ADR-0060):

- `recipe-app` holds one iteration of each job, `run_once`, behind its ports. The supervisor passes the `request_id` of the run and its `RunKind` (ADR-0088 (b), (d)). `recipe-server` holds the supervisor, the schedules, the watchdog, the panic hook, and the wiring.
- The supervisor runs one `tokio::spawn` per iteration and stops on a `CancellationToken`. It starts the next run one interval after the end of the last run, so a late run gives no burst.
- Each run gets its own `request_id`. Only the rows that the run writes hold it (3.2).
- The six jobs and `CompareSettings` of 7.12 are internal operations, like `EnsureDerivedType`. They have no permission and no row of the permission matrix.

## 3. Structured logging and tracing

### 3.1 Stack

- `tracing` for spans and events in all crates.
- `tracing-subscriber` with the JSON formatter. Output goes to stdout. Docker collects it.
- `tower-http::trace::TraceLayer` creates the request span.
- One line per event. One JSON object per line.
- `RUST_LOG` goes through `EnvFilter`. A `RUST_LOG` without a directive gives the default `info,sqlx=warn,hyper=warn` of 3.3. A bad directive stops the start, and a fallback logger writes the one `ERROR` line (ADR-0061 (c), (d), ADR-0080 (a)).
- After the JSON `ERROR` line of a failed start or subcommand, the std `Termination` of `main` writes `Error: ` and the error chain to stderr. That text is not a log line and holds no secret (1.1, ADR-0080 (c)).

### 3.2 Span fields

| Span | Fields |
|---|---|
| `http.request` | `request_id`: UUID v7, generated by the server, returned in the `X-Request-Id` response header. `method`. `route`: the pattern, for example `/recipes/{ref}/versions/{n}`, not the concrete path. `client_ip`: from the trusted proxy (section 7.11). `user_agent`: first `http.user_agent_max` characters. `session_ref`: first `log.session_ref_length` hex characters of the session id hash, the same value as `audit_log.session_ref`. `user_id`, `role`, `status`, `latency_ms`. `error_code` when the result is an error |
| `use_case` | `name` (for example `ApproveVersion`), `recipe_ref`, `version`, `machine_code`, `outcome` (`ok` or the error code) |
| `db.statement` | `name` (a label such as `recipe_version.update_status`), `rows_affected`, `latency_ms`. Never the SQL text with bound values |
| `uow.transaction` | `isolation`, `outcome` |
| `job` | `job` (the name of the job, for example `cleanup`), `iteration`, `outcome` (`ok`, the code of the `Err`, or `panic`) |

The `request_id` of the HTTP span is inherited by all child spans. A log line without a `request_id` comes from startup, shutdown, a background job, or the backstop panic layer. A line of a job carries the field `job` of the span `job` instead.

- The span field has the name `job` and not `name`. The JSON formatter of `tracing-subscriber` writes its own key `name` for the name of each span (ADR-0088 (a)).
- The supervisor opens the span `job` around each run. Every event of the run gets the field `job`, also the `WARN` lines of the job and the line of the panic hook (ADR-0088 (a)).
- The first run of `disk_check` in step 6 of the start also runs inside the span, with `iteration` 1. The loop of the job continues with 2 (ADR-0089 (b)).

- Each run of a job has its own `request_id`. Only the rows of the run hold it, for example the row `audit_chain_broken` of `security_event` (ADR-0060 (s)).
- The backstop panic layer of 2.4 rule 3 gets only the panic payload, so its line holds no `request_id`. The line of the panic hook holds it while the request span is active (ADR-0057 (g)).

### 3.3 Level policy

| Level | Use |
|---|---|
| `ERROR` | The system could not do what was asked because of an internal failure. Examples: storage error, audit write failure, caught panic, deadlock, trigger code reached the user, startup failure, audit chain break. Each `ERROR` line is actionable |
| `WARN` | Degraded but working. Examples: slow query (above `db.slow_query`), pool near exhaustion, disk space below the warning threshold, login throttled, CSRF rejected, Forbidden on `UserManage`, clock skew. Also a failed webhook call and an active alert condition |
| `INFO` | One line per request (from `TraceLayer`), one line per use case outcome, failed login, startup and shutdown, migration applied, backup completed |
| `DEBUG` | SQL statement names and timings, cache of type trees, template rendering |
| `TRACE` | Not used in production. Available for local debugging |

A validation error from a user (HTTP 4xx) is `INFO`, not `WARN`. It is a normal outcome. A 5xx is `ERROR`. A failed snapshot read of the job `alerts` is a storage error at `WARN` (ADR-0089 (d)). The one exception is the health router of 7.2: its finish line has the level `DEBUG` for every status (ADR-0059 (l)).

Production runs at `INFO` for all crates and `WARN` for `sqlx` and `hyper`. The level is set by the `RUST_LOG` variable and can change without a rebuild.

A `RUST_LOG` has no directive when the variable is missing or empty, or when it holds only spaces and commas. Such a text gives the default `info,sqlx=warn,hyper=warn`. The server trims each comma piece and drops the empty pieces before the parse (ADR-0080 (a)).

A string field value in a directive needs double quotes, for example `[span{field="text"}]=debug`. The filter compares the `Debug` text of the value as literal text, and that text holds the quotes (ADR-0061 (c), ADR-0080 (b)).

### 3.4 Redaction

Never written to the log:

- passwords, password hashes, and the one-time password of `CreateUser` and `ResetPassword`
- session tokens, session ids, the `Cookie` and `Set-Cookie` headers. The log holds `session_ref` (the first 16 hex characters of the session id hash, 3.2) instead of the session id
- the `X-CSRF-Token` header and the `csrf_token` and `csrf_pre` fields
- the `Authorization` header
- the full request body of the login and password forms
- `DATABASE_URL`, every `*_PASSWORD` and `*_FILE` secret, the keys `login_hmac_key` and `csrf_signing_key`, the bootstrap administrator password
- the `Debug` output of configuration structs
- the webhook URL of `alerts.webhook_url_file`, and every part of it: no host, no path, and no query (ADR-0063 (f))

The `Debug` implementation of `PasswordHash`, `SessionToken`, `CsrfToken`, `LoginForm`, `PasswordForm`, and `Config` prints `[redacted]` for the secret fields. The `Display` and `Debug` text of `WebhookFault` and of the type that holds the webhook URL hold no part of the URL (ADR-0063 (g)). The `TraceLayer` is configured to skip the `Cookie`, `Set-Cookie`, `Authorization`, and `X-CSRF-Token` headers.

Parameter values and process limits are not secret. The use case span may include them at `DEBUG`.

### 3.5 Retention

Docker uses the `json-file` log driver with `max-size` and `max-file` from `log.max_size` and `log.max_file` (section 7.11). This keeps about 1 GB of logs per container. Older lines are discarded. One logging anchor of `compose.yaml` serves the seven services. It sets `mode` to `log.mode` and `max-buffer-size` to `log.max_buffer_size`, so a slow log driver cannot block `app`. A full buffer drops lines, which agrees with the best effort of the event log in 3.6 (ADR-0070 (s), (t)). A site that needs longer retention forwards stdout to its own collector. The application does not write log files.

### 3.6 Audit log, security events, event log

| | Audit log | Security events | Event log |
|---|---|---|---|
| Location | PostgreSQL table `audit_log` | PostgreSQL table `security_event` | Container stdout |
| Content | Who changed what, before and after values, `aggregate_type`, `aggregate_id`, `client_ip`, `session_ref`, hash chain. Setting changes (`SettingChanged`, 7.12) | Failed logins, throttling, CSRF rejections, Forbidden results, unlocks, audit chain breaks (A33). `client_ip` is NULL only for `audit_chain_broken` | How the system behaved: latencies, errors, panics |
| Written | In the same transaction as the change (ADR-0005) | In its own short transaction, also when the request fails | Asynchronously, best effort |
| Retention | Forever. Part of the backup | `security_event.retention` (section 7.11). Then the job `cleanup` deletes the rows through `security_event_purge()` (`data-model.md` 4.12) | 1 GB rolling window |
| Reader | Administrator (all), ProcessEngineer (recipe entries), every role (recipe history page), per A20 | Administrator | Operations staff through Docker tools |
| Tamper evidence | `entry_hash` chain, checked every night (ADR-0022) | Append-only for `recipe_app`, except `count` of an aggregated row (`data-model.md` 3.8) | none |

A business question ("who approved version 7?") is answered by the audit log. A security question ("who tried to log in as admin at 03:00?") is answered by the security events. An operations question ("why was the approval slow?") is answered by the event log. The `request_id` is present in all three and links them.

### 3.7 Alerts

The supervised job `alerts` runs inside `app` every `alerts.interval` (section 7.11). It evaluates the conditions below. It reads the database values of one run in one `READ ONLY` transaction (ADR-0060 (e)). When that read fails, the run returns the error and evaluates no condition. The supervisor logs the error once with the level of 1.1, and the next run follows after `alerts.interval` (ADR-0089 (d)). For each active condition it writes one `WARN` line to the event log. The field `alert` of the line holds the name of the condition (ADR-0075 (c)). The exception is `low_disk`: it writes one line for each low or unreadable volume, with the field `volume` (ADR-0075 (e)). There is no alert cron in the `backup` container (ADR-0020).

| Name | Condition | Source |
|---|---|---|
| `panics` | More than `alerts.panic_count` panic events in `alerts.panic_window` | In-process counter of the panic hook (2.4) |
| `not_ready` | `/readyz` not ready for more than `alerts.not_ready_for` | In-process readiness state |
| `backup_age` | Latest dump older than `backup.max_age_alert` | The newest complete set on the read-only mount of the `backups` volume. A set is complete when its manifest exists (7.7). An unreadable directory is an error, not "no set" (ADR-0060 (f)). It makes the condition active, also during the start grace. The job writes one `WARN` line of the `BackupFault` (ADR-0073 (c)) |
| `replication` | `recipe_replicator` not connected, or its lag `pg_current_wal_lsn() - sent_lsn` at or above `wal.max_lag_bytes`. A row without `sent_lsn` gives no lag, and the condition is active too (ADR-0090 (a)) | The row of `recipe_replicator` with `application_name = 'walarchive'` in `pg_stat_replication`. `recipe_app` reads it through the function `replication_state()` of `data-model.md` 5.5 (ADR-0068 (p), ADR-0069 (j)) |
| `low_disk` | Free disk below `disk.warn_percent` on `pgdata`, `backups`, or `walarchive` | `statvfs` on the read-only mounts. A `DiskFault::Unreadable` on a mount makes the condition active for that volume, and the job writes one `WARN` line of the fault. `DiskFault::Unsupported` keeps the condition inactive (ADR-0073 (e), (f)) |
| `failed_logins` | More than `alerts.failed_login_count` failed logins in `alerts.failed_login_window` | Sum of `count` of the `login_failed` rows in `security_event` |
| `security_events` | Any `csrf_rejected`, or a `forbidden` on `UserManage`, in the last `alerts.security_window` | `security_event` |
| `audit_chain_broken` | Audit chain check failed | `security_event` kind `audit_chain_broken`, written by the job `audit_chain_check`. The condition is active while the newest row of this kind is younger than `audit.max_check_age`. A chain that stays broken gives a new row at each nightly check (ADR-0073 (a)) |
| `audit_check_age` | `audit.last_check_at` older than `audit.max_check_age` | `audit.last_check_at` in `app_setting`, written by the job `audit_chain_check` (`data-model.md` 4.9) |
| `export_hash_mismatch` | One or more `E-EXP-002` since the previous run. The first run compares with 0. A run with a failed snapshot keeps the previous total (ADR-0073 (g), ADR-0093 (b)) | In-process error counter of the job `alerts`. `ExportVersion` increments it (`data-model.md` 6) |

The column "Name" gives the stable name of each condition. ADR-0075 (b) gives the rules of a name.

Webhook (ADR-0063):

- The secret file of `alerts.webhook_url_file` holds the URL. An empty key or an empty file means no webhook. The start then writes one `INFO` line that names the key.
- A missing file at a set path stops the start. A URL that does not parse, a scheme other than `https`, a carriage return, or a second line also stops the start. The `ERROR` line names the key only (ADR-0063 (a), (b)).
- The client trusts the webpki roots, or only the certificates of `alerts.webhook_ca_file` when the key is set. It follows no redirect (ADR-0063 (j), (k)).
- The job sends one HTTPS `POST` with the body `{"alert":"<name>"}` for each active condition. `<name>` is the name of the condition in the table above (ADR-0075 (c)). The calls of one run go out at the same time under one `alerts.webhook_timeout`.
- A connection fault, a status outside `2xx`, a timeout, or a TLS fault is a `WebhookFault`. The job writes one `WARN` line without the URL and continues.
- `app` reaches the webhook host through the network `egress` (ADR-0065 (n)).

Start grace (ADR-0060 (x), (y)): the first start of the installation is the oldest `created_at` in `app_setting`. During `backup.max_age_alert` after the first start, a missing dump raises no alert. During `audit.max_check_age` after the first start, a missing `audit.last_check_at` raises no alert. A read fault of the backup directory raises the alert also during the grace (ADR-0073 (c)).

Clock trust (ADR-0093 (a)): a stored time later than `database_now` of the snapshot by more than `alerts.interval` is untrusted, and its condition stays active.

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

- Body limits are in section 7.11. The server returns `E-WEB-003` above the limit. The value fields of a parameter have their own limit, because a form with 16 384 leaves does not fit in the form limit.
- All JSON types derive `serde::Deserialize` with `#[serde(deny_unknown_fields)]`.
- Form parsing uses `axum::Form` with `serde` structs. A missing required field is `422 E-WEB-008` with the field name. A form with indexed fields or value fields is read as pairs of key and value, and the reader rejects an unknown key (ADR-0037, ADR-0039).
- A path parameter is parsed directly into the value object (`ReferenceNumber`, `MachineCode`, `UnitPath`, `ParameterKey`, `LangCode`). A bad path parameter never gives a 500. The rule has two cases (`api.md` 1):
  - A path value that is a domain value object with its own error code returns that code with 422. `LangCode` returns `E-RCP-027`. `UnitPath` returns `E-PLT-017` for a malformed text or segment and `E-PLT-002` for more than 5 segments. No path holds an S7 address.
  - A path value that names a resource returns `404 E-WEB-005` when the resource does not exist. This applies also when the text cannot name a resource, for example a bad `ReferenceNumber`. The exemption: a command on a draft returns `404 E-RCP-051` for a well-formed `{key}` and `404 E-RCP-050` for a well-formed `{lang}` that the existing draft does not hold (`api.md` 1).
- A request field that names a machine or a unit that does not exist returns `404 E-PLT-019` or `404 E-PLT-020` (`api.md` 1).
- The `Content-Type` of a request must match the route. Every mutating route accepts only `application/x-www-form-urlencoded`. A parameter value travels as one field per leaf (ADR-0039).
- Every mutating route uses `POST` (ADR-0040). The `htmx-config` meta tag keeps `methodsThatUseUrlParams: ["get"]` (`api.md` 9).
- Text fields are trimmed and normalized to Unicode NFC before layer (b). Exception: the text of a `String` or `Char` leaf of a value is kept as sent (ADR-0039).
- The `row_version` of a draft is a hidden form field. A missing `row_version` on a mutating draft request is `422 E-WEB-008`.
- The `next` field of `POST /lang` must be a relative path that starts with `/` and not with `//`. Anything else is replaced by `/recipes`.
- The `lang` field of `POST /lang` and the cookie `recipe_lang` accept only `pl` and `en`. The server ignores any other cookie value and uses `pl`.

### 4.3 Domain value objects

Each value object has one constructor `parse(input) -> Result<Self, Error>` and no public `new`. `Error` is the error type of its row in section 1.2, or a reason when no row exists (section 1.1, ADR-0025). A value object instance is valid by construction. `domain-model.md` section 2 is the authority for every grammar. A row that names `domain-model.md` does not repeat the grammar.

| Value object | Grammar or rule |
|---|---|
| `MachineCode` | `^M[1-9][0-9]{0,6}$`, number `1..=9999999` |
| `UnitPath` | `^U([1-9]\|[1-5][0-9]\|6[0-4])(\.([1-9]\|[1-5][0-9]\|6[0-4])){0,4}$`, 1 to 5 segments, each `1..=64` (A6, A30). A malformed text or segment is `E-PLT-017`. More than 5 segments is `E-PLT-002` |
| `ReferenceNumber` | `^[A-Z0-9][A-Z0-9_-]{0,49}$` after uppercasing (A27) |
| `ParameterTag` | `^[A-Z][A-Z0-9_]{0,63}$` |
| `ParameterKey` | UUID, generated by the server (A40) |
| `Login` | `^[a-z0-9._-]{3,64}$` after lowercasing |
| `Text` (names: machine, unit, recipe) | Not empty after trim, at most 200 characters, no control characters |
| `Text` (description, change note) | Not empty after trim, at most 4 000 characters, no control characters except a line end. A line end is LF or CRLF (ADR-0028). `description_en` is required (A36) |
| `Text` (translation name, description) | Name: not empty, at most 200 characters. Description: 0 to 4 000 characters, may be empty. No control characters except a line end (LF or CRLF) in the description |
| `LangCode` | `^[a-z]{2,3}(-[A-Z]{2})?$`, not `en` (English is the base text, A17) |
| `S7Address` | Grammar, width rule, alignment rule, and overlap rule: `domain-model.md` 2.1. Platform rule: required on `S7_300`, optional on `S7_1200` and `S7_1500`, not allowed on `CodesysIec` and `TwinCat3` (A31) |
| `S7Layout` | `domain-model.md` 2.1: `BOOL` packing, even-byte alignment, struct and array size |
| `TypeName` (UDT) | `^[A-Za-z_][A-Za-z0-9_]{0,63}$`, not a reserved primitive name |
| `StringLength` | Range per platform: the `StringLength` table of `domain-model.md` 2.1 |
| `TypeExpr` | Grammar, rules, and errors: `domain-model.md` 2.1. Every failure of the reason table of `domain-model.md` 2.1 is `E-PLT-013`. The exception is a member of `DefineUdt` whose type is not on the platform: `E-PLT-006`. A type above a size limit is `E-PLT-018` |
| `ArrayDimension` | `lower <= upper`, `upper - lower < 65536`, at most 3 dimensions (A38, `domain-model.md` 2.1) |
| `LeafPath` | Grammar: `domain-model.md` 2.1. The empty path is the parameter itself. `[*]` means every element of that array level. Resolves against the `TypeTree` |
| `ProcessLimit` | `min <= max` when both set. Inside the type range. `max_length <= string_length`. The limit fields follow the `LeafClass` of the leaf. Not on a `BOOL`, `CHAR`, or `WCHAR` leaf (`domain-model.md` 2.2) |
| `IecLiteral` | Grammar, prefixes, and base units: `domain-model.md` 2.1. A text that does not parse, or a count outside the range, is `E-RCP-005` for a value and `E-RCP-006` for a limit |
| `Password` (input only) | 12 to 128 characters, not in the breached-password list. Hashed at once. The plain text is dropped after hashing |

### 4.4 Parameter value JSON

- A form carries the text of each leaf (ADR-0039). `recipe-domain` builds the JSON form of each leaf from its text and its `LeafClass` (`domain-model.md` 2.1). Then the rules of this section apply.
- The JSON is parsed by `serde_json` with the `arbitrary_precision` feature. A number stays a decimal string inside `serde_json::Number`. There is no `f64` round trip.
- The domain converts each numeric, duration, or instant leaf to `rust_decimal::Decimal` for range and limit checks. A number that does not fit in `Decimal` (more than 28 significant digits) is rejected with `E-RCP-005`.
- Canonical form: before storage the domain rewrites every numeric, duration, and instant leaf to the canonical decimal text of `Decimal`. This text has no exponent and no trailing zeros, and `-0` becomes `0`. `1.5e2` is stored as `150`. `JSONB` keeps this canonical form. The export writes each of these leaves and each limit as a JSON string that holds this canonical text (`api.md` 6.1). The content hash therefore does not depend on a JSON number format. Equality of two values is equality of canonical forms.
- `REAL` and `LREAL` leaves are additionally checked to fit the IEEE 754 single or double range. The range of `Decimal` lies inside both ranges, and a unit test proves it (ADR-0028).
- A number with a fraction is outside the range of an integer type and gives `E-RCP-005` (ADR-0028).
- The JSON parser enforces the depth limit `json.max_depth` (section 7.11). `DefineUdt` and `EnsureDerivedType` reject a `TypeTree` deeper than this limit with `E-PLT-018`, so a valid value never reaches it.
- Size budgets (Assumption A26, section 7.11): at most 16 384 scalar leaves per parameter. Per version: at most 262 144 leaves, 32 768 limit rows, and 4 000 parameters. `TypeTree` resolution counts leaves. `DefineUdt` and `EnsureDerivedType` reject a type above the per-parameter budget or above `type.max_members` with `E-PLT-018`. `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `RebindTypeRevision`, and `CreateVersion` check the per-version budgets with `E-RCP-021`. Statement-level triggers on `parameter` and `parameter_limit` are the last line (`E-RCP-035`, `E-RCP-036`).
- The shape check is exact: an array has at most 3 dimensions (Assumption A38) and exactly `(upper - lower + 1)` elements per dimension. A struct has exactly the member names. No extra key, no missing key.
- `STRING` values are checked for byte length against `string_length` in the target encoding (Latin-1 for `STRING`, UTF-16 units for `WSTRING`). A `STRING` character outside Latin-1 gives `E-RCP-005` (ADR-0028).
- `BOOL` accepts only JSON `true` and `false`. Not `0`, `1`, or `"true"`.
- `CHAR` and `WCHAR` accept a JSON string with exactly one Unicode scalar inside the range of `domain-model.md` 2.1.
- Time types (`TIME`, `LTIME`, `S5TIME`, `DATE`, `LDATE`, `TIME_OF_DAY`, `LTOD`, `DATE_AND_TIME`, `LDT`, `DTL`) have the class `duration` or `instant` (`domain-model.md` 2.1). A request carries an IEC literal, for example `T#1d2h3m4s5ms` or `DT#2026-10-12-12:30:00`. The parser `IecLiteral` of `recipe-domain` converts it to an integer count of the base unit. The domain stores, hashes, and limits this count as canonical decimal text. The base-unit table of `domain-model.md` 2.1 gives the range of each count. Type names are the names of the matrix in `data-model.md` section 7.
- Snapshot validation (`CreateVersion`, `ApproveVersion`) runs in `spawn_blocking` before the transaction opens. The transaction then locks the rows and re-checks `row_version`. It also repeats two checks that can change after the snapshot (`data-model.md` 6). `CreateVersion` checks the units again. `ApproveVersion` checks the limit count and the outdated types again. A validation that takes longer than `timeout.validation` returns `E-WEB-009`.

## 5. Migrations and schema evolution

- Migrations are SQL files in `migrations/` at the repository root, managed by `sqlx::migrate!`. The file name is `NNNN_description.sql`. The first release ships `0001_schema.sql`, `0002_triggers.sql`, `0003_grants.sql`, `0004_seed_plc_types.sql`, `0005_security_event_guard.sql`, and `0006_operations.sql` (`data-model.md` 8). The release of ADR-0098 adds `0007_audit_definer_operators.sql` and `0008_audit_log_insert_columns.sql`.
- Migrations are forward-only. There is no `down` file. A rollback is a new forward migration or a restore from backup.
- A migration that shipped is never edited. `sqlx` stores a checksum of each applied migration in `_sqlx_migrations`. A changed checksum stops the migrator with an error.
- Roles and the database are not created by a migration. The script `deploy/db/initdb/01-roles.sh` runs once as the PostgreSQL superuser when the `db` volume is empty. It reads the Docker secrets and creates `recipe_migrator`, `recipe_app`, `audit_owner`, `recipe_backup`, `recipe_replicator`, and the database. It grants `pg_read_all_stats` to `audit_owner` and not to `recipe_app` (ADR-0069 (f)).
- `01-roles.sh` writes the whole `pg_hba.conf` in place of the catch-all line of the image. `data-model.md` 5.1 gives its client rules (ADR-0069 (r) to (t)).
- `recipe-db` builds its connect options without the pgpass file of libpq. Code sets every field, and the TLS mode is `ssl_mode = disable` inside the Compose network (ADR-0069 (a)).
- `serve`, `migrate`, and `create-admin` refuse to start when a variable whose name starts with `PG` is set. The refusal writes one `ERROR` line with the name of the variable and not its value. The exit code is 1 (ADR-0069 (b), (c)).
- The server binary has a `migrate` subcommand: `recipe-server migrate`. The Compose service `migrate` runs it once (`restart: no`). It is the only service that holds the migrator secret. The `app` service never connects as `recipe_migrator`. At startup `app` checks that `current_user` is `recipe_app`, is not a superuser, and owns no table. It refuses to serve otherwise.
- The migrator takes an advisory lock. Two migrators cannot run at the same time.
- The migrator connects within `migrate.connect_timeout`. Otherwise it writes one `ERROR` line and exits with code 1 (ADR-0069 (d)).
- The migrator sets `lock_timeout` to `migrate.lock_timeout` at session level on its connection, before `sqlx::migrate!` runs. The migrator retries a migration that failed on `55P03` up to `migrate.retries` times with a pause of `migrate.retry_pause` (section 7.11). A migration that must build an index on a large table uses `CREATE INDEX CONCURRENTLY` in a file that starts with `-- no-transaction`. Such a file holds one statement only, so it cannot run `SET ROLE audit_owner`. A concurrent index on an audit object is therefore a runbook procedure, not a migration (7.10, `data-model.md` 5.1). CI rejects a migration that rewrites a table or scans it under an exclusive lock (`ALTER TABLE ... TYPE`, `ADD COLUMN ... DEFAULT <volatile>`, `SET NOT NULL` without a valid `CHECK (... IS NOT NULL)` first). The rule applies to every table (`data-model.md` 8).
- CI starts a fresh cluster, runs `01-roles.sh`, and applies every migration as `recipe_migrator`. Then it runs the grants test of `data-model.md` 5.3. A migration that fails as `recipe_migrator` therefore fails the build.
- A breaking change uses the expand-and-contract pattern across releases: release N adds the new column or table and writes both. Release N+1 reads the new one. Release N+2 deletes the old one.
- Start order in Compose: `db` healthy → `backup-premigrate` (one-shot `pg_dump` with suffix `_premigrate`) completed → `migrate` completed → `app` → `proxy`. The `depends_on` conditions encode this chain (section 7.1). The upgrade procedure in 7.10 depends on the `_premigrate` dump.
- The first administrator is not a migration. `recipe-server create-admin` reads the login from an argument and the password from stdin or from a secret file. It does nothing when an active administrator exists. It sets `must_change_password`. The Compose file does not run it. The runbook runs it once after the first `migrate`. CI runs `create-admin --password-file` (ADR-0062 (q)).
- Primitive `plc_type` rows are seed data in `0004_seed_plc_types.sql`. Each row has a UUID that never changes. The insert uses `ON CONFLICT DO NOTHING`, and `sqlx` never re-runs an applied migration (`data-model.md` 8).

## 6. Testing strategy

| Level | Scope | Tooling | Rule |
|---|---|---|---|
| Unit, domain | `recipe-domain` | `cargo test`, no I/O | Every invariant in `domain-model.md` section 3 has at least one passing and one failing test. Every domain error variant (below 030 and `E-RCP-050..069`) is produced by at least one test |
| Unit, use case | `recipe-app` | In-memory implementations of all ports | Every command in `domain-model.md` section 5 has tests for success, each listed error, and the audit entry it writes. `SetParameterValue` leaves limit and translation rows unchanged. `ReplaceLimits` leaves the value unchanged. Separation of duties: the creator and each editor of a draft get `E-RCP-015`. Limit completeness: a numeric leaf without a limit blocks approval with `E-RCP-016` |
| Permission matrix | `recipe-app` | Generated test from the matrix in `api.md` section 3 | One test per (operation, role) pair, for commands and for queries. The form and preview queries of `domain-model.md` 5.5 are part of it, with the permission of the command that they prepare. The test asserts `Ok` or `AppError::Forbidden`. A new use case without a matrix row fails the build. `EnsureDerivedType`, the six supervised jobs of 2.4, and `CompareSettings` of 7.12 are internal operations, not use cases, and have no matrix row (ADR-0060 (t)). A demoted user with an open session gets `AppError::Unauthenticated` (`401 E-WEB-004`) on the next request, not `Forbidden`, because `SetUserRole` deletes the sessions of the user and increments `session_epoch` |
| Property | `recipe-domain` | `proptest` | `UnitPath`, `S7Address`, `ReferenceNumber`, `LeafPath`, `IecLiteral` round-trip through `parse` and `Display`. `ParameterValue::parse` and `ParameterValue::check` against random `TypeTree`s up to depth 6 with random limits: a generated valid value passes, a mutated value fails. `S7Layout`: random sets of parameters in one data block, overlap and misalignment are always detected, non-overlapping aligned sets always pass |
| Integration, storage | `recipe-db` | `testcontainers` with the pinned `postgres` image (ADR-0101) | Every `CHECK`, `UNIQUE`, FK, partial index, and trigger in `data-model.md` has a test that violates it and asserts the SQLSTATE and the trigger code. As `recipe_app`, an update of a column outside the UPDATE grant of `recipe_version` (`number`, `recipe_id`, `based_on_version_id`) fails with SQLSTATE `42501` before any trigger runs. The test fixture runs the forbidden version transitions as the table owner `recipe_migrator`. These are `approved→draft`, `retired→approved`, `retired→draft`, and the undelete of a discarded draft. They also include a change of `number`, `recipe_id`, `based_on_version_id`, or `content_sha256` on an approved row. Each one fails with `E-RCP-033`. The grants of `recipe_app` (`role_table_grants`, `role_column_grants`, `role_routine_grants`) equal a checked-in list (ADR-0069 (q)). `RemoveParameter` on a parameter with limits and translations succeeds. A second `STRING(20)` on one machine succeeds. Every transactional operation in `data-model.md` section 6 has a test that fails midway and asserts no partial write |
| Concurrency | `recipe-db` | Two connections | Two approvals of the same draft with the same `row_version`: exactly one writes. The other returns `AlreadyApproved` and writes no audit entry. An approval with an older `row_version` gets `E-WEB-001`. Two parallel audit inserts get consecutive `chain_seq` values, and the chain check passes. A parameter insert during an approval waits and then fails with `E-RCP-031`. `DeleteRecipe` during `ApproveVersion`: the second one fails. Two edits with the same `row_version`: exactly one succeeds. Two concurrent failed logins: both count. `ArchiveUnit` against `AddParameter` with a new derived type: one waits on the machine row, and no `40P01` occurs. `ArchiveMachine` against `CreateVersion`: the same result |
| HTTP | `recipe-web` | `tower::ServiceExt::oneshot` with in-memory ports | Every route: status code, error code in the body, `X-Request-Id` header, security headers, no secret in the body. Login, logout, session expiry, session epoch invalidation, CSRF rejection, pre-session CSRF on `/login` and `/lang`, `must_change_password` gate. An XSS payload corpus through every text field. A panic on a page, on an HTMX request, and on an export route (ADR-0057). The answer of the backstop, with `X-Request-Id` and without the other headers of `api.md` 9. A normal answer after the panic |
| Load | `recipe-web` with Compose database | `oha` | Login flood: 200 concurrent `POST /login` with random logins. The container stays under its memory limit, no request waits more than 5 seconds, and the Argon2 semaphore returns `503` under load. A version at the size budget: `CreateVersion`, `ApproveVersion`, and the version page each finish under 10 seconds |
| Browser regression | `recipe-web` with `examples/dev_server.rs` and its in-memory store | Playwright Test in `e2e/` (ADR-0048) | The browser-only checks of ADR-0048 (c). They include no sideways scroll at 320 CSS px, the keyboard path through the leaf grid, and the focus after a swap. They also include WCAG 2.4.11 and the CSP checks of each page. CI runs the suite on every push to `main` and on every pull request |
| Stack | Compose stack built from zero in the CI job `compose` on `ubuntu-24.04`, on `workflow_dispatch` and on a push to `main` (ADR-0094) | Docker Compose, the scripts of `deploy/ci/`, the test scripts of `deploy/pgtools/tests/` | `/readyz` answers `ok` within 60 s. Then the job runs the checks of ADR-0070 (y). They cover the first administrator, the login, the health path variants, and the uid of `app`. They also cover the backup, the restore test, the base backup, the recovery from the WAL archive, and the stop with exit code 0. A last check requires that the backup healthcheck fails when the backup path and `pgdata` share one filesystem. A separate job `musl-unwind` runs the panic tests of `recipe-web` on the musl target (ADR-0070 (z)) |
| End to end | Compose stack | Playwright Test smoke suite in `e2e/` (ADR-0048) | Create machine, create recipe, add parameter, set limits, approve by a second user, export, check the hash, check the audit page |
| Soak | Compose stack, nightly CI | The same request mix for 24 h at a constant rate, with a login flood every hour | Memory of the `app` container stays under 384 MiB (section 7.11). No `ERROR` line. No restart. p99 latency stays under 500 ms. For the musl image, a value above the memory ceiling or the p99 bound applies the fallback rule of ADR-0064 (o) and (p) |
| Lint gate | Workspace | `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check` | Runs on every pull request |

Coverage target: 80 % line coverage per crate, measured with `cargo llvm-cov`. The CI job fails below the target.

## 7. Operations for 24/7

### 7.1 Compose layout

| Service | Image | Role | Restart | Healthcheck |
|---|---|---|---|---|
| `proxy` | `caddy`, pinned by digest | TLS termination, HTTP to HTTPS redirect, `404` for `/healthz*` and `/readyz*`. Mandatory. Every TLS connection of a client ends at Caddy, also from a plant device in front of it. Caddy sends plain HTTP to `app:8080` (A18, ADR-0065) | `always` | `GET http://app:9000/healthz` |
| `backup-premigrate` | same as `backup` | One-shot `pg_dump` with suffix `_premigrate` | `no` | Exit code |
| `migrate` | `recipe-server` with `migrate` | One-shot schema migration. Holds the migrator secret | `no` | Exit code |
| `app` | `recipe-server` | The application and its supervised jobs, `alerts` included (2.4). Public listener on `http.public_port` (`expose`, never `ports`). Internal listener on `http.internal_port` | `always` | `recipe-server healthcheck --path /readyz` every `health.check_interval` (ADR-0059). `health.check_failures` failures mark it unhealthy |
| `db` | `postgres:16.<x>-bookworm`, pinned by digest | Database. `POSTGRES_INITDB_ARGS="--data-checksums --lc-collate=C --lc-ctype=C.UTF-8"`. Port 5432 not published. Runs as `user: 999:999` with the `command:` list of 7.6, `db.stop_grace_period`, and `db.shm_size` | `always` | `pg_isready -h 127.0.0.1 -p 5432 -U pg_isready -d recipe` every `health.check_interval`. `db` is healthy only after its init (ADR-0070 (q)) |
| `backup` | `recipe-pgtools` | Daily `pg_dump`, the weekly restore test, the `backup-now` command, and the retention (7.7). supercronic runs the schedule (ADR-0066 (j)). Connects as `recipe_backup` | `always` | `backup-health`: the backup path is not on the filesystem of `pgdata`, the marker file exists, and a complete set is younger than `backup.max_age_alert`. `pgdata` is mounted read-only into `backup` for this check |
| `walarchive` | `recipe-pgtools` | `pg_receivewal` and the weekly `pg_basebackup` to a bind-backed volume on a second disk (A35, ADR-0020, ADR-0068). Connects as `recipe_replicator` over the replication protocol | `always` | `walarchive-health`: the `pg_receivewal` process runs. The job `alerts` checks the replication lag against `wal.max_lag_bytes` (3.7) |

Seven services. Start order through `depends_on`: `db` (healthy) → `backup-premigrate` (completed successfully) → `migrate` (completed successfully) → `app` → `proxy`. `walarchive` and `backup` depend only on `db`.

Images (ADR-0064):

- The project builds `recipe-server:<version>` and `recipe-pgtools:<version>`. `compose.yaml` names them with `image:` and `${RECIPE_VERSION}`. Each external image has a pinned digest.
- `recipe-server` runs on `gcr.io/distroless/static-debian13:nonroot` as the user `65532:65532`. The binary is built for `x86_64-unknown-linux-musl`. The image holds no shell.
- `recipe-pgtools` builds on the pinned image of `db` and adds supercronic and the scripts of `deploy/pgtools/bin/`. It runs as uid 999 (ADR-0066).
- A release moves the images as files of `docker save`. The plant loads them with `docker load` (7.10).

Networks and capabilities (ADR-0065, ADR-0070):

- The network `backend` has `internal: true`, the subnet `${COMPOSE_SUBNET}`, and `ip_range: ${COMPOSE_IP_RANGE}`. It holds the seven services. `proxy` always has the address `${PROXY_ADDRESS}` on it.
- The network `edge` holds `proxy` and the published ports. The network `egress` holds `app` only, for the webhook of 3.7. It publishes no port.
- `db` is only on `backend`. No service except `proxy` publishes a port.
- Every service gets `cap_drop: [ALL]` and `security_opt: [no-new-privileges:true]`. `proxy` gets `cap_add: [NET_BIND_SERVICE]` only.
- `app` and `proxy` get `read_only: true`. `proxy` mounts the volume `caddy` on `/data`. `backup`, `backup-premigrate`, and `walarchive` get `tmpfs: /run/pgpass`.
- `proxy` keeps an idle upstream connection for `proxy.upstream_keepalive`, below `http.body_read_timeout` of `app` (ADR-0065 (i)).

Healthchecks: each healthcheck uses `health.check_interval`, `health.check_failures`, `health.check_timeout`, `health.start_period`, and `health.start_interval` (ADR-0070 (n), (o)).

Secrets: every password is a Docker secret mounted as a file. The application reads `*_FILE` variables only. There is no env-file option. The file `deploy/.env` holds only the eight values of the Compose interpolation and no password. The application never reads it (ADR-0062 (i), (j)). The superuser password exists only in `db` and in the initdb script. Its file is `secrets.postgres_password_file` (ADR-0069 (e)). The secret `replicator_password` (`secrets.replicator_password_file` of 7.11) exists only in `db` and in `walarchive`. The initdb script creates `recipe_replicator` from it (`data-model.md` 5.1).

Secret files on the host (ADR-0062 (l) to (o), ADR-0065 (b)):

- The secret files live in a directory outside the repository. The directory belongs to `root` and has the mode `0700`. `${RECIPE_SECRETS_DIR}` names it, and `compose.yaml` names each file as `${RECIPE_SECRETS_DIR:?}/<name>`.
- Each file with one reader has the mode `0400` and the uid of its reader.
- `app_password` and `migrator_password` have two readers, uid 999 and uid 65532. Each of them gets a POSIX ACL for both uids with the mask `r--`. Its mode then shows `0440`. The owning group gets no right.
- `tls_cert` and `tls_key` hold the certificate and the key of the plant PKI. They belong to uid 0, the user of the Caddy image, with the mode `0400`. In development and in CI, `Caddyfile.dev` uses `tls internal` and needs no file.
- The secret `alerts_webhook_url` holds the URL of the webhook (3.7, ADR-0063).
- A secret file holds one line (ADR-0031).

Two more Docker secrets hold keys of 32 bytes. `login_hmac_key` is the HMAC-SHA256 key of `security_event.login_hmac`. `csrf_signing_key` signs the pre-session CSRF cookie (`api.md` 7.3). The keys `security.login_hmac_key_file` and `security.csrf_signing_key_file` of 7.11 name the files. A key file holds one key per line, the newest key first. Each line holds the 32 bytes of a key as 64 lower-case hex characters (ADR-0035). The server writes and signs with the first key and accepts every key in the file. Rotation: the operator adds the new key as the first line. Both keys are accepted for `security.key_rotation_overlap`. Then the operator deletes the old line.

### 7.2 Health endpoints

| Endpoint | Checks | Use |
|---|---|---|
| `GET /healthz` | The process is up and the router answers | Liveness. The watchdog thread (section 2.4) and the proxy healthcheck use it |
| `GET /readyz` | A connection of the health pool can run `SELECT 1` within `health.db_check_timeout`. Every migration of the binary is in `_sqlx_migrations` with the same checksum, and no failed row exists. A database with a newer migration stays ready (ADR-0059 (h), (i)) | Readiness. Docker marks the container unhealthy after `health.check_failures` failures. The result is cached for `health.readiness_cache` |

Both endpoints live on the internal listener on `http.internal_port`. They need no session. They return `text/plain` with `ok` and the status 200, or `not ready` and the status 503 (ADR-0059 (g)). They do not return version strings, hostnames, or check details. Readiness turns on after the first successful check after the start, and it turns off at the signal (ADR-0059 (j)).

- The internal listener serves a separate health router. The public router answers `404` for `/healthz` and `/readyz` (ADR-0059 (k)).
- The health router has the layer `request_id`, the security headers of `api.md` 9, and `TraceLayer`. It has no layer `client_address`, no session layer, no CSRF layer, and no `CatchPanicLayer` (ADR-0059 (l), (m)).
- The subcommand `recipe-server healthcheck [--path <path>]` sends `GET` to `127.0.0.1` on `http.internal_port`. The default path is `/readyz`. It exits with code 0 for the status 200 and with code 1 otherwise.
- The subcommand uses `std` only and the budget `watchdog.probe_timeout`. It reads `RECIPE_HTTP__INTERNAL_PORT` and no other value (ADR-0059 (b), (c), (e)). Disk space is not part of readiness. A low-disk condition changes the mode of the application (7.8), not its readiness.

### 7.3 Graceful shutdown

ADR-0058 (l) to (p) holds the stop. ADR-0087 amends the drain of the internal listener and the abort. ADR-0088 (e) adds the cut count of the jobs to the `WARN` line of step 6. ADR-0089 (c) writes that line only for a cut. ADR-0090 (b) holds a pool close that reaches its limit. ADR-0061 (f) holds a signal during the start.

1. The process receives `SIGTERM` from Docker.
2. Readiness turns off, and the watchdog stops. The public listener stops accepting new connections. The supervised jobs get the cancel.
3. Requests in flight and open job iterations continue together under `shutdown.drain`. Open transactions commit or roll back as their use case finishes. The internal listener still serves `/healthz` with `ok` and `/readyz` with `not ready`.
4. After `shutdown.drain`, the server aborts the open connections and does not wait for them. Their transactions roll back (ADR-0087 (d)). The supervisor aborts the open job iterations in the same way (ADR-0060 (j)).
5. Both pools close within `shutdown.pool_close`. The pool close waits for the connections of the aborted tasks. A pool whose close reaches the limit gets one `WARN` line that names the pool, and the stop continues (ADR-0090 (b), 1.1). Then the internal listener stops with the drain `shutdown.internal_drain` of 0 s. It cuts its connections at once (ADR-0087 (a)).
6. The runtime stops within `shutdown.runtime_stop`. The process exits with code 0, also after a cut drain or a pool close that reached its limit. When at least one count is above 0, one `WARN` line gives three counts (ADR-0088 (e), ADR-0089 (c)):
   - `cut_public`: the cut connections of the public listener,
   - `cut_internal`: the cut connections of the internal listener,
   - `cut_jobs`: the cut job iterations.

   When all three counts are 0, the stop writes no such line. A full stop writes one `INFO` line, "the server stopped", through the step 1 logger (ADR-0061), and an early or failed start writes none.
7. The Compose setting `stop_grace_period` of the service `app` is `app.stop_grace_period`.

Two sum rules hold for the values of 7.11. A test reads both from 7.11 (ADR-0058 (p), ADR-0070 (v)):

- `shutdown.drain + shutdown.pool_close + shutdown.internal_drain + shutdown.runtime_stop < app.stop_grace_period`, so step 6 completes before `SIGKILL` (ADR-0087 (e)).
- `http.request_timeout <= shutdown.drain`.

### 7.4 Timeouts

The values are in section 7.11. This table gives the key and the mechanism.

| Where | Key (7.11) | Mechanism |
|---|---|---|
| HTTP request total | `http.request_timeout` | A `from_fn` layer wraps `next.run` in `tokio::time::timeout`. It returns `AppError::Timeout { stage: Request }`, and `render_errors` gives `503 E-WEB-009` with `Retry-After` (ADR-0058 (g), (h)). A request on an export path runs without this limit and keeps its own limits (ADR-0072) |
| Request headers | `http.body_read_timeout` | `header_read_timeout` of `hyper` `http1` in the accept loop of `recipe-server`, on both listeners. An idle keep-alive connection closes after the same time (ADR-0058 (a), (c), (k)) |
| Request body read | `http.body_read_timeout` | `tower_http::timeout::RequestBodyDeadlineLayer`, one deadline for the whole body. A late body gives `503 E-WEB-009` (ADR-0058 (i), (j)) |
| SQL statement | `db.statement_timeout` | `statement_timeout` set per connection by the pool `after_connect` hook |
| Lock wait | `db.lock_timeout` | `lock_timeout` per connection. A timeout becomes `E-DB-001` |
| Idle in transaction | `db.idle_in_transaction_timeout` | `idle_in_transaction_session_timeout` per connection. Between two statements of a transaction, no use case waits for I/O or for a lock that it does not already hold. Validation runs before the transaction |
| Pool acquire | `db.pool_acquire_timeout` | `sqlx::PoolOptions::acquire_timeout`. A timeout becomes `E-DB-004` |
| Pool idle connection | `db.pool_idle_timeout` | `idle_timeout` |
| Pool max lifetime | `db.pool_max_lifetime` | `max_lifetime`. Avoids stale connections after a database restart |
| Argon2id semaphore | `argon2.acquire_timeout` | Acquire timeout. A timeout returns `503 E-WEB-009` (`AppError::Timeout`) |
| Export generation | `timeout.export` | Timeout around the wait for an `export.permits` permit, the read, the locks, the rebuild in `spawn_blocking`, the hash compare, and the writer. Returns `503 E-WEB-009`. The use case takes the permit before its transaction. The blocking task holds the permit to its end (ADR-0056). The commit runs after this limit, so a timeout never hides a committed `VersionExported` entry (ADR-0029) |
| Snapshot validation | `timeout.validation` | Timeout around the `spawn_blocking` call. Returns `503 E-WEB-009` |
| Diff of two versions | `timeout.validation` | Timeout around the wait for a `diff.permits` permit and the `spawn_blocking` call. Returns `503 E-WEB-009`. The query takes the permit before its `REPEATABLE READ` transaction and holds it to the end of the render (ADR-0050) |

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
| `max_wal_senders` | `pg.max_wal_senders` (7.11) | One for `pg_receivewal`, one for the weekly `pg_basebackup --wal-method=fetch`, one spare for a future replica (ADR-0068 (h)) |
| `wal_keep_size` | `pg.wal_keep_size` (7.11) | Bridges a short outage of `walarchive`. Keeps the WAL of a running base backup (ADR-0068 (h)) |
| `checkpoint_timeout` | `pg.checkpoint_timeout` (7.11) | Bounded recovery time |
| `shared_buffers` | `pg.shared_buffers` (7.11) | Default guidance |
| `log_min_duration_statement` | `db.slow_query` (7.11) | Slow statements appear in the database log |
| `log_lock_waits` | `on` | Every lock wait above `deadlock_timeout` is logged |
| `default_transaction_isolation` | `read committed` | Every command runs in it, including `SetUserRole` and `DeactivateUser` (ADR-0096). Only the first-administrator operation of `create-admin` sets `SERIALIZABLE` explicitly (ADR-0035, section 7.11). The draft queries of ADR-0042 set `REPEATABLE READ` explicitly |
| `timezone` | `UTC` | All timestamps are `TIMESTAMPTZ`. The UI converts to the plant time zone |
| data checksums | on, set at `initdb` | Detects silent corruption. Cannot be enabled later without a rebuild |
| collation | `C` for `lc-collate`, `C.UTF-8` for `lc-ctype`, set at `initdb` | A glibc update cannot corrupt text indexes. The UI sorts names with `COLLATE "und-x-icu"` |

The `command:` list of `db` in `compose.yaml` sets the server settings of this table and `max_connections` from `db.max_connections` (ADR-0070 (l)). `db` sets no `synchronous_standby_names`, so a commit never waits for the archive (ADR-0068 (b)).

### 7.7 Backup and restore

Disks (ADR-0066 (a) to (d)):

- `backups` and `walarchive` are named volumes with `driver: local` and a bind to `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR` of `deploy/.env`. Each path is on a second disk.
- The operator creates one marker file at the top of each disk (ADR-0074 (a)). The backup disk gets `.backup-marker`, and the WAL archive disk gets `.walarchive-marker`. Each script of `backup` and `walarchive` checks the marker first. Without it, the script writes one `ERROR` line and writes no file.
- The backup healthcheck `backup-health` fails when the backup path and `pgdata` are on the same filesystem (`stat -c %d`). It also fails without the marker or without a complete set younger than `backup.max_age_alert`. A `_premigrate` set counts. `pgdata` is mounted read-only into `backup` for this check.

Dump (ADR-0066 (e) to (l), ADR-0074 (b), (c)):

- The `backup` service runs `pg_dump --format=custom --exclude-table-data=session --exclude-table-data=login_attempt` daily at `backup.time` to the `backups` volume. The same run writes the globals file with `pg_dumpall --globals-only --no-role-passwords -l recipe`, because roles are not inside a `pg_dump`. The file holds no password. A restore sets the role passwords from the secret files (ADR-0033).
- One run writes one set: `recipe_<YYYYMMDD_HHMM><suffix>.dump`, `globals_<YYYYMMDD_HHMM><suffix>.sql`, and `recipe_<YYYYMMDD_HHMM><suffix>.counts`. `<suffix>` is empty for the daily run, `_premigrate` for `backup-premigrate`, and `_now` for `backup-now`.
- The manifest `recipe_<YYYYMMDD_HHMM><suffix>.dump.sha256` covers the three files. The script writes it last. A set without a manifest is not complete. The restore script checks the manifest before it restores.
- A run writes each file of a new set first under the temp name `.tmp-<final name>` in the same directory. It publishes the file with a hard link (`ln -T` without `-f`) and then deletes the temp name. A publish never replaces a file (ADR-0074 (b)).
- A temp name never counts as a set. The temp files of a killed run stay until the operator deletes them.
- A reader finds a manifest only by the anchored grammar `^recipe_[0-9]{8}_[0-9]{4}(_premigrate|_now)?\.dump\.sha256$` of the whole name. `[0-9]` stands for the ASCII digits only (ADR-0074 (c)).
- The retention, `backup-health`, `restore-test`, and the adapter of `BackupDirectory` use this grammar. A name that does not match is not a set, and the retention never deletes it.
- The counts file holds the row count of each table in the snapshot of the dump. It holds no row for `session` and `login_attempt`, because the dump excludes their data.
- A psql session opens `BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY` and exports the snapshot. It counts the rows in the same transaction and keeps it open until `pg_dump --snapshot` ends.
- `flock` stops a second run while one run writes. The script writes the pgpass file in the tmpfs `/run/pgpass` from `/run/secrets/backup_password`.
- Retention: after a successful new set, the script deletes every set older than `backup.retention`. The rule covers the daily sets, `_premigrate`, and `_now`. The newest daily set and the newest `_premigrate` set always stay.
- A failed run deletes no file of another set. It deletes only its own temp names and the files that it published without a manifest (ADR-0074 (b)). A delete uses `rm -- "${path:?}"` after a check of the path prefix. The marker file and the scratch directory of the restore test always stay.

Rules of every script of `recipe-pgtools` (ADR-0066 (m) to (o)):

- `umask 077` is the first command. Files get `0600`, and directories get `0700`. Only the top directory of `backups` gets `0755`, so uid 65532 in `app` can list the names. The top directory of `walarchive` stays `0700`.
- A script writes one JSON object per line to stdout.
- A script escapes `\` and `:` in the pgpass file. It keeps each password out of the argument list and out of shell variables. No script enables `xtrace`.

WAL archive (ADR-0068, ADR-0077):

- `walarchive` streams WAL with `pg_receivewal --synchronous` and the connection value `application_name=walarchive` to its volume on a second disk. It uses no replication slot. `pg_receivewal` and `pg_basebackup` connect as `recipe_replicator`, which the `pg_hba.conf` line of `data-model.md` 5.1 admits.
- Recovery point objective: minutes. Recovery time objective: 1 hour (ADR-0020). Without the second disk the recovery point objective is 24 hours (A35).
- An outage longer than `pg.wal_keep_size` leaves a gap. The runbook then moves the old segments aside and starts the stream again. After `pg_stat_replication` shows the stream, it takes a new base backup and checks the WAL of that base backup in the archive (ADR-0068 (e), ADR-0099).
- `basebackup-once` runs at `basebackup.time`. It runs `pg_basebackup --wal-method=fetch -Fp --checkpoint=fast --manifest-checksums=SHA256`, and then `pg_verifybackup` checks the new base backup.
- After a passed check, the script keeps the newest `basebackup.retention` base backups. Then `pg_archivecleanup` deletes the WAL segments before the lowest start segment of all kept base backups (ADR-0077 (f)).
- The start segment comes from the line `START WAL LOCATION` of the `backup_label`. The comparison uses the 16 hex characters after the timeline, as `pg_archivecleanup` does. A failed `pg_verifybackup` deletes no base backup and no WAL segment.
- Layout of the WAL archive disk (ADR-0077 (d)). The WAL segments lie at the top of `/wal`. Each base backup is the directory `/wal/base/base_<YYYYMMDD_HHMMSS>`.
- A reader finds a base backup only by the anchored grammar `^base_[0-9]{8}_[0-9]{6}$` of the whole name, with `LC_ALL=C`. A link or a file with this name is not a base backup.
- A run of `basebackup-once` writes the base backup first under the temp name `.tmp-base_<YYYYMMDD_HHMMSS>` and renames it after the check. The marker file, the lock file `/wal/.lock`, and the directory `/wal/.pitr-scratch` always stay.
- The directory `.tmp-base_*` of a killed run stays in `/wal/base`. The operator deletes it when no `basebackup-once` runs (ADR-0077 (h)).
- A recovery uses `restore_command = 'cp /wal/%f %p || cp /wal/%f.partial %p'`. The fallback replays the transactions of the partial last segment.
- The CI script `pitr-check` proves a recovery from the archive (ADR-0068 (m) to (o), ADR-0077). CI runs `basebackup-once`, then the command `ChangeOwnPassword` as the marker entry, then `pitr-check`.
- Before `pitr-check`, CI waits until `pg_stat_replication` shows the flush of the marker commit by `walarchive`. CI gives `chain_seq` and `entry_hash` of the marker entry to `pitr-check` (ADR-0077 (a) to (c)).
- `pitr-check` copies the newest base backup into `/wal/.pitr-scratch` under `/wal/.lock` and recovers a scratch cluster from the copy and the archive. It opens no connection to `db` and mounts no secret.
- The copy needs free space on the WAL archive disk for the newest base backup and the WAL of the recovery. After the recovery, `pitr-check` finds the marker entry and runs the audit chain check (ADR-0077 (e)).
- `pitr-check` runs in a temporary container of `recipe-pgtools` with `container.backup_memory` and `container.small_cpus`. Its cluster uses the `scratch.*` settings of 7.11 (ADR-0068 (o), ADR-0081 (a)).
- WAL health is the replication lag. The job `alerts` reads the row of `recipe_replicator` with `application_name = 'walarchive'` through the function `replication_state()` (`data-model.md` 5.5, ADR-0069 (j)). It raises the alert `replication` when its condition of 3.7 is active.
- The job `heartbeat` writes `heartbeat.last_at` in `app_setting` every `heartbeat.interval`. The WAL stream therefore never stays idle for long, also on a quiet night.

Copies and restore:
- The latest dump and the WAL archive are copied to the external backup storage by a site procedure (ADR-0020). The dump contains password hashes and the audit log. The site procedure MUST protect the copies with the same access control as the database.
- A restore test runs each week at `restore_test.time`, before each release, and in steps 5 and 6 of 7.10 (ADR-0067, ADR-0083). The service `backup` runs `restore-test` through supercronic. Before a release, the operator runs `docker compose exec backup restore-test`.
- The image `recipe-pgtools` holds the `restore-code.expected` of its release. `backup` and `walarchive` therefore run the image of the release whose migrations the schema of `db` holds (ADR-0083 (a)).
- After a rollback with the expand pattern, `app` runs the previous image, and `backup` and `walarchive` keep the new image (ADR-0083 (c)).
- The script uses a private scratch cluster on the backup disk. It runs `initdb` with the arguments of 7.1 and with `--auth-local=peer --auth-host=scram-sha-256`. The scratch server listens on `127.0.0.1` only and uses the `scratch.*` settings of 7.11.
- The script runs these steps (ADR-0067):
  1. It selects the newest complete set and checks its manifest with `sha256sum -c --strict`. It then checks each line of the globals file against the role grammar of five statement forms (ADR-0095 (a), (b)). A line outside the grammar stops the test before any PostgreSQL tool runs.
  2. It compares the names of the entries `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER` of `pg_restore -l`. It compares them with the names of the lines of these kinds in `deploy/pgtools/restore-code.expected`. A storage test compares that file with a database after `01-roles.sh` and every migration. An unknown entry stops the test (ADR-0076 (b)). An entry of the kind `PROCEDURE`, `AGGREGATE`, `MATERIALIZED VIEW`, or `OPERATOR` also stops the test (ADR-0078 (b)).
  3. It restores the globals file only after step 1 accepts it (ADR-0095 (f)). It then sets a random password for each login role.
  4. It restores the section `pre-data` of the dump into the scratch cluster. It compares the SHA-256 of each function, `CHECK` constraint, and generated column with the lines of these kinds in `restore-code.expected`. A difference stops the test before the data load (ADR-0067 (h) to (l), ADR-0076 (c)). The restore of `pre-data` can run the code of a generated column before this comparison, as the scratch superuser. That code can write files as uid 999, hide its change from the comparison, or keep the restore from ending. ADR-0084 accepts this residual risk.
  5. It restores the sections `data` and `post-data`. Then it compares the lines of every kind with `restore-code.expected`. This includes each trigger with its enabled state and each event trigger. A difference stops the test before step 6 (ADR-0076 (a), (c), ADR-0078 (a)).
  6. It compares the row count of each table of the counts file with the restored table.
  7. It runs the audit chain check of `data-model.md` 4.9.
  8. Each login role logs in over `127.0.0.1` with its random password. A login with a wrong password must fail.
  9. It writes one JSON line with the result to the event log. A failed step is an `ERROR` event and ends the script with a code other than 0.
- A `trap` deletes the scratch cluster on each exit of the script.
- After a restore into production, the operator truncates `session` and `login_attempt`. The operator then re-applies every user deactivation and password reset recorded after the dump time, from the off-site audit copy when one exists. Last, the operator waits for the next completed run of the job `audit_chain_check` and reads its result (ADR-0100).

### 7.8 Volumes, resources, disk full

| Volume | Content |
|---|---|
| `pgdata` | PostgreSQL data directory. Mounted read-only into `app` for the disk check and into `backup` for the filesystem check of 7.7 |
| `backups` | Dumps from 7.7. A named volume with a bind to a second disk (ADR-0066 (a)). Mounted read-only into `app` for the job `alerts` |
| `walarchive` | A named volume with a bind to a second disk. WAL segments and base backups. Mounted read-only into `app` for the job `alerts` |
| `caddy` | TLS state of Caddy, mounted on `/data` of `proxy`. The Caddyfile sets `persist_config off` (ADR-0070 (d)) |

Resource limits per container are the keys `container.*` of 7.11 (ADR-0070 (f) to (m), ADR-0081):

- Each service gets `mem_limit` equal to `memswap_limit`, so the limit is hard and has no swap. Each service gets `cpus`.
- `app` has `container.app_memory` and `container.app_cpus`. `db` has `container.db_memory` and `container.db_cpus`.
- `backup` has `container.backup_memory`. `proxy` and `walarchive` have `container.small_memory`. `migrate` and `backup-premigrate` have `container.oneshot_memory`.
- `proxy`, `backup`, `walarchive`, `migrate`, and `backup-premigrate` have `container.small_cpus`. The temporary container of `pitr-check` also has `container.small_cpus` (ADR-0081 (a)).
- `app` runs `runtime.worker_threads` Tokio workers, so a long render does not stop `/healthz` on one CPU (ADR-0061 (h)).
- ADR-0070 (m) gives the minimum host and the versions of Docker Engine and Compose.

The Argon2 semaphore keeps hashing memory at 4 × 19 MiB, which is 15 % of the `app` limit.

Disk space policy (Assumption A24):

- The job `disk_check` in `app` measures the free space of the `pgdata` volume with `statvfs` on its read-only mount every `health.disk_check_interval`. Its first run is in step 6 of the start, before the listeners open, so no command writes before the mode is set (ADR-0061).
- Warning threshold: `disk.warn_percent` free. The application writes a `WARN` event every `disk.warn_log_interval`, and the job `alerts` raises the condition.
- Hard threshold: `disk.readonly_percent` free. The application enters read-only mode. `Login`, `Logout`, every query, and every export continue. Every other command returns `503 E-WEB-006`. The only audit actions in read-only mode are `UserLoggedIn`, `UserLoggedOut`, `VersionExported`, and the `SettingChanged` entries of the startup comparison of 7.12 (ADR-0061 (e)).
- In read-only mode `Login` keeps T1, which inserts the `login_attempt` row. A failed or throttled login also writes its `security_event` row, with the aggregation of `data-model.md` 3.8. The login throttle therefore works in read-only mode (`data-model.md` 6).
- Other writes to `session` and `security_event` continue, and so do the supervised jobs. These writes and the `Login` writes are small, and the margin of `disk.readonly_percent` is reserved for them. The UI shows the read-only banner. `design-system.md` 10.1 gives its place and its pages (ADR-0071).
- A failed measurement of `pgdata` (`DiskFault::Unreadable`) also turns the read-only mode on. This rule also holds for the first run in step 6 of the start, and the start continues (ADR-0073 (d)).
- After a failed measurement the job writes one `WARN` line of the fault at most every `disk.warn_log_interval`.
- On a system without `statvfs` (`DiskFault::Unsupported`) the job writes one `WARN` line per process and does not change the mode. Only a development system on Windows has no `statvfs` (ADR-0073 (e)).
- The application leaves read-only mode without a restart when free space rises above `disk.resume_percent`. Only a measurement that succeeds can end the mode (ADR-0073 (d)).
- PostgreSQL itself stops at 0 % free. The hard threshold exists so that the application stops first.

### 7.9 Clock, logs, and alerts

- All containers use the host clock. The host runs NTP. A clock skew above `clock.skew_warn` between `app` and `db` produces a `WARN` event. The job `clock_check` compares the clocks with `SELECT now()` at startup and every `clock.check_interval`.
- Log rotation is the Docker driver from section 3.5.
- The supervised job `alerts` in `app` produces the alerts. Section 3.7 lists the conditions.
- The job `audit_chain_check` records `audit.last_check_seq` and `audit.last_check_at` in `app_setting` after each completed check (`data-model.md` 4.9). The job `alerts` raises an alert when `audit.last_check_at` is older than `audit.max_check_age` (3.7). The margin above the nightly interval prevents a false alert after a slow check.
- `data-model.md` 4.9 gives the catch-up check after a start, with `audit.catchup_age` and `audit.catchup_delay` (ADR-0060 (v), (w)).
- 3.7 gives the start grace of a new installation.

### 7.10 Upgrade procedure

1. Read the release notes. Note whether the migration follows the expand pattern.
2. Load the new image files with `docker load`. Compare the SHA-256 of each file before the load and the image ID after it (ADR-0064 (j), (k)). Do not stop the running stack yet. Set `RECIPE_VERSION` in `deploy/.env` to the new release tag.
3. The old image checks the newest set with `docker compose exec backup restore-test`. Then five commands run in this order (ADR-0097 (h), (h4)):
   1. `docker compose up -d backup-premigrate`. It writes the rollback set `_premigrate`. Wait for exit code 0.
   2. `docker compose up -d --no-deps migrate`. Without `--no-deps`, Compose runs `backup-premigrate` a second time, and that dump fails in the same UTC minute (ADR-0082). Wait for exit code 0 of `migrate`.
   3. On failure of command 1 or 2, stop here. Set the previous tag in `deploy/.env` again. The old `app` keeps running on the old schema when the migration follows the expand pattern. A release with two or more migration files can fail after the first file, so use a forward correction or the `_premigrate` restore (ADR-0083).
   4. The guard step: the superuser replaces `audit_guard.audit_objects_dropped()` with `CREATE OR REPLACE FUNCTION` of the new `01-roles.sh`. This step is the only place that replaces the function on an older volume.
   5. A query computes the SHA-256 of `pg_get_functiondef` of the function, with `search_path = pg_catalog`. It must equal field 4 of the line `FUNCTION|audit_guard audit_objects_dropped()` of the new `deploy/pgtools/restore-code.expected`. No `backup-now` runs before the hash matches. The release notes name commands 4 and 5 as a "manual superuser step".
   - A failure of command 4 or 5 leaves the new schema and the old guard. The operator runs commands 4 and 5 again (`docs/ops/runbook.md`, section "Upgrade").
4. `docker compose up -d --no-deps app`. Docker replaces the container. Compose starts no one-shot service again, because step 3 ran them (ADR-0082 (a)). Downtime is the graceful shutdown window plus startup, under 60 s.
5. Check `/readyz` on the internal listener, log in, open one recipe, export one version and check its hash. Then run these three commands while the new `app` serves (ADR-0083 (b)):
   1. `docker compose up -d --no-deps backup walarchive`. Both services get the new `recipe-pgtools` image.
   2. `docker compose exec backup backup-now`. The new `_now` set holds the new schema.
   3. `docker compose exec backup restore-test`. Check that the test passes and names the `_now` set.
   - If the test names a `_premigrate` set of the same UTC minute, run commands 2 and 3 again in a later minute.
6. Rollback: `RECIPE_VERSION=<previous tag> docker compose up -d --no-deps app`. Without `--no-deps`, the previous `migrate` runs against the newer schema and fails, and `app` does not start (ADR-0082 (b)). `deploy/.env` keeps the new tag, because the schema stays newer (ADR-0083 (c)). If the migration did not follow the expand pattern, stop `app` and restore the `_premigrate` dump. Then set the previous tag in `deploy/.env` and run `docker compose up -d --no-deps app` (ADR-0083 (d)). Last, run the three commands of step 5 with the tag of `deploy/.env`. `backup` and `walarchive` then run the image of the schema (7.7).
7. Concurrent index on an audit object (`data-model.md` 5.1). The operator connects as `recipe_migrator` and runs these statements in one `psql` session: `GRANT CREATE ON SCHEMA public TO audit_owner`, `SET ROLE audit_owner`, `CREATE INDEX CONCURRENTLY`, `RESET ROLE`, `REVOKE CREATE ON SCHEMA public FROM audit_owner`. The next migration records the index with `CREATE INDEX IF NOT EXISTS`.
8. Major PostgreSQL upgrade (at least twice in 10 years): follow `docs/ops/runbook.md` section "Major upgrade". It uses `pg_dumpall --globals-only` plus `pg_dump -Fc` into a new `db` volume created with the same `initdb` arguments, then a restore test, then the switch. There is no `pg_upgrade` (ADR-0020).

### 7.11 Settings

This table is the single source for every numeric setting. A key of the first column is a configuration key only when ADR-0062 or ADR-0031 lists it. Every other value of this table that `serve` uses is a code default (ADR-0062 (a), (f)). Other documents link here. A value repeated elsewhere must equal this table.

| Key | Value | Used by |
|---|---|---|
| `session.idle_timeout` | 30 min | 7.12, api.md 7.2 |
| `session.absolute_timeout` | 12 h | 7.12, api.md 7.2, data-model.md 3.6 |
| `session.cleanup_interval` | 10 min | 2.4, api.md 7.2, data-model.md 3.6 |
| `session.cookie_name` | `__Host-recipe_session`, Secure, HttpOnly, SameSite=Strict, Path=/ | api.md 7.1 |
| `session.token_bits` | 128. The database stores SHA-256 of the token | api.md 7.1 |
| `session.secure` | `true`. `false` refuses to start without the `dev` feature | 7.12, api.md 7.1 |
| `login.delay_after_failures` | 3 failures per (login, client_ip) in 15 min | api.md 7.4, data-model.md 6 |
| `login.delay_formula` | `2^(n-3)` s, cap 60 s | api.md 7.4, data-model.md 6 |
| `login.ip_budget` | 100 failures per client_ip in 15 min, then 429 | api.md 7.4, data-model.md 6 |
| `login.attempt_retention` | 24 h | data-model.md 3.7 |
| `login.attempt_cleanup_interval` | 10 min | 2.4, data-model.md 3.7 |
| `login_attempt.purge_batch` | 5 000 rows per statement of the job `cleanup` | 2.4, data-model.md 3.7, data-model.md 6 |
| `argon2.m_cost` / `t_cost` / `p_cost` | 19 MiB / 2 / 1 | api.md 7.4 |
| `argon2.permits` / `argon2.acquire_timeout` | 4 / 2 s | 7.4, 7.5, api.md 7.4, data-model.md 6 |
| `password.min` / `password.max` | 12 / 128 characters, breached-password list | api.md 7.4 |
| `security.login_hmac_key_file` | `/run/secrets/login_hmac_key`. 32 bytes. HMAC-SHA256 key of `security_event.login_hmac` | 7.1, 7.12, data-model.md 3.8 |
| `security.csrf_signing_key_file` | `/run/secrets/csrf_signing_key`. 32 bytes. Signing key of the pre-session CSRF cookie | 7.1, 7.12, api.md 7.3 |
| `security.key_rotation_overlap` | 24 h. The old and the new key are both accepted | 7.1 |
| `secrets.replicator_password_file` | `/run/secrets/replicator_password`. Password of `recipe_replicator`, read by the initdb script and by `walarchive` | 7.1, 7.7, data-model.md 5.1 |
| `secrets.migrator_password_file` | `/run/secrets/migrator_password`. Password of `recipe_migrator`, read by the initdb script and by `recipe-server migrate` (ADR-0031) | 5, 7.1, data-model.md 5.1 |
| `secrets.app_password_file` | `/run/secrets/app_password`. Password of `recipe_app`, read by the initdb script and by `recipe-server serve` (ADR-0031, ADR-0062) | 7.1, 7.12, data-model.md 5.1 |
| `secrets.backup_password_file` | `/run/secrets/backup_password`. Password of `recipe_backup`, read by the initdb script (ADR-0031) | 7.1, 7.7, data-model.md 5.1 |
| `secrets.postgres_password_file` | `/run/secrets/postgres_password`. Password of the superuser. Read by the image of `db` only (ADR-0069) | 7.1 |
| `i18n.lang_cookie_max_age` | 365 days. Cookie `recipe_lang` | api.md 8 |
| `approval.separation_of_duties` | `true` | 7.12, api.md 3.2, ADR-0022 (A34) |
| `http.request_timeout` | 30 s | 7.3, 7.4 |
| `http.body_read_timeout` | 10 s | 7.1, 7.4 |
| `http.accept_error_pause` | 1 s. The pause of the accept loop after a fault of `accept` that is not a connection fault, for example too many open files. A code default (ADR-0085) | ADR-0058 (a) |
| `http.body_limit.form` | 256 KiB | api.md 7.6 |
| `http.body_limit.value` | 4 MiB | api.md 7.6, ADR-0039 |
| `http.body_limit.translation` | 64 KiB | api.md 7.6 |
| `http.trusted_proxies` | CIDR list, default `172.28.0.2/32`, the address of `proxy` (ADR-0065 (j)) | 7.1, 7.12, api.md 7.1 |
| `http.public_port` / `http.internal_port` | 8080 / 9000. Configuration keys, recorded in `app_setting` (ADR-0062 (e)) | 7.1, 7.2, 7.12 |
| `proxy.upstream_keepalive` | 5 s. The idle time of an upstream connection of Caddy, below `http.body_read_timeout` (ADR-0065 (i)) | 7.1 |
| `http.retry_after` | 5 s. The `Retry-After` header of a 503 response | api.md 5.1 |
| `http.hsts_max_age` | 31536000 s (365 days), with `includeSubDomains` | api.md 7.1, api.md 9 |
| `static.cache_max_age` | 1 day. `Cache-Control: public, max-age` of `GET /static/*` | api.md 2.1, api.md 9 |
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
| `db.host` / `db.port` / `db.name` | `db` / 5432 / `recipe`. The Compose service and the database of `data-model.md` 5.1 (ADR-0031) | 5, 7.1, 7.12 |
| `db.migrator_user` | `recipe_migrator`. The role of `recipe-server migrate` (ADR-0031) | 5, data-model.md 5.1 |
| `db.app_user` | `recipe_app`. The role of `recipe-server create-admin` and of the application (ADR-0035) | 7.1, 7.12, data-model.md 5.1 |
| `db.stop_grace_period` / `db.shm_size` | 60 s / 256 MiB. `stop_grace_period` and `shm_size` of the service `db` (ADR-0070 (k)) | 7.1 |
| `db.slow_query` | 1 s (PostgreSQL `log_min_duration_statement = 1000`) | 3.3, 7.6 |
| `timeout.export` | 10 s | 7.4 |
| `timeout.validation` | 10 s | 4.4, 7.4 |
| `migrate.lock_timeout` / `migrate.retries` / `migrate.retry_pause` | 2 s / 10 / 5 s | 5, data-model.md 8 |
| `migrate.connect_timeout` | 10 s. The connect timeout of `recipe-server migrate` (ADR-0069 (d)) | 5 |
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
| `page.users` | 100 rows of `ListUsers` (ADR-0035) | api.md 2.8 |
| `page.machines` | 100 rows of `ListMachines` (ADR-0037) | api.md 2.2 |
| `page.type_usage` | 100 rows of `GetTypeUsage` (ADR-0037) | api.md 2.3 |
| `page.recipes` | 100 rows of `ListRecipes` and `ListDeletedRecipes` (ADR-0043) | api.md 2.4 |
| `page.versions` | 100 rows of the version list of `GetRecipe` and `ListVersions` (ADR-0043) | api.md 2.4, api.md 2.5 |
| `page.diff` | 50 parameters of one page of `DiffVersions` and `GetApprovalPreview`, over all its sections (ADR-0050) | api.md 2.5 |
| `diff.leaf_rows` | 1 000 rows of one parameter in one section of a diff (ADR-0050) | api.md 2.5 |
| `diff.permits` | 2 diffs that run at the same time in `spawn_blocking` (ADR-0050) | 7.4 |
| `export.permits` | 2 exports that run the read, the rebuild, the hash compare, and the writer at the same time (ADR-0056) | 7.4 |
| `page.audit` | 100 rows of `ListAuditEntries` (ADR-0051) | api.md 2.9 |
| `page.history` | 100 rows of `GetRecipeHistory` (ADR-0051) | api.md 2.9 |
| `page.security_events` | 100 rows of `ListSecurityEvents` (ADR-0051) | api.md 2.9 |
| `form.array_chunk` | 100 elements per group of an array in the leaf grid (ADR-0039) | design-system.md 10.8 |
| `form.rebind_leaves` | 16 384 new member leaves in the rebind forms of one version page (ADR-0044) | api.md 2.5 |
| `i18n.translation_langs` | `pl`. The list of language tags of the translation rows (ADR-0012, ADR-0043) | 7.12, api.md 2.6 |
| `audit.unlock_rows` | 100. The newest deleted attempts in the row image of `LoginUnlocked` (ADR-0035) | data-model.md 10 |
| `health.readiness_cache` | 5 s | 7.2 |
| `health.db_check_timeout` | 2 s | 7.2 |
| `health.check_interval` / `health.check_failures` | 10 s / 3 | 7.1, 7.2 |
| `health.check_timeout` / `health.start_period` / `health.start_interval` | 5 s / 30 s / 1 s. `timeout`, `start_period`, and `start_interval` of each Compose healthcheck (ADR-0070 (o)) | 7.1 |
| `health.disk_check_interval` | 30 s | 2.4, 7.8 |
| `disk.warn_percent` / `disk.readonly_percent` / `disk.resume_percent` | 15 / 5 / 10 | 3.7, 7.8, api.md 5.1, overview.md A24 |
| `disk.warn_log_interval` | 10 min | 7.8 |
| `watchdog.probe_interval` / `watchdog.failures` / `watchdog.window` | 10 s / 3 / 30 s | 2.4 |
| `watchdog.probe_timeout` | 3 s. The budget of the watchdog probe and of the subcommand `healthcheck`, above `health.db_check_timeout` (ADR-0059 (e)) | 2.4, 7.2 |
| `runtime.worker_threads` | 2 Tokio workers of `serve` (ADR-0061 (h)) | 7.8 |
| `job.restart_delay` | 10 s | 2.4 |
| `audit.check_time` | 03:00 UTC, every night | 2.4, data-model.md 4.9 |
| `audit.check_batch` | 10 000 rows per statement of the audit chain check | 2.4, data-model.md 4.9 |
| `audit.check_batch_bytes` | 8 MiB of `before_state` and `after_state` per statement of the audit chain check | 2.4, data-model.md 4.9 |
| `audit.check_statement_timeout` | 60 s. `statement_timeout` of the head read and of one batch of the audit chain check (ADR-0086 (a)) | 2.4, data-model.md 4.9 |
| `audit.max_check_age` | 26 h. The job `alerts` raises an alert when `audit.last_check_at` is older | 3.7, 7.9, data-model.md 4.9 |
| `audit.catchup_delay` / `audit.catchup_age` | 10 min / 24 h. The delay of the catch-up check after a start, and the age of `audit.last_check_at` that starts it (ADR-0060 (v)) | 2.4, 7.9, data-model.md 4.9 |
| `audit.last_check_seq` | State key in `app_setting`, not a configuration file key. The end of a clean prefix: the last `chain_seq` up to which a completed audit chain check read the chain without a break. The value never decreases. A head below it is a break. The job writes it before `audit.last_check_at` (ADR-0086 (e)) | 2.4, 7.9, 7.12, data-model.md 3.17, data-model.md 4.9 |
| `audit.last_check_at` | State key in `app_setting`, not a configuration file key. The end time of the last completed audit chain check. The job `alerts` compares it with `audit.max_check_age` | 2.4, 3.7, 7.9, 7.12, data-model.md 3.17, data-model.md 4.9 |
| `heartbeat.interval` | 5 min | 2.4, 7.7, data-model.md 3.17 |
| `heartbeat.last_at` | State key in `app_setting`, not a configuration file key. The time of the last write of the job `heartbeat` | 2.4, 7.7, 7.12, data-model.md 3.17 |
| `alerts.interval` | 60 s | 2.4, 3.7 |
| `alerts.webhook_url` | The fingerprint `sha256:<64 hex>` of the URL in the file of `alerts.webhook_url_file`, or empty. A derived key of `app_setting`, not a configuration key (ADR-0063 (c)) | 3.7, 7.12, data-model.md 3.17 |
| `alerts.webhook_url_file` | `/run/secrets/alerts_webhook_url`. Optional. An empty key or an empty file means no webhook. A missing file at a set path stops the start (ADR-0063 (a)) | 3.7, 7.1, 7.12 |
| `alerts.webhook_ca_file` | empty, which means the webpki roots. A set file replaces the webpki roots with its certificates (ADR-0063 (j)) | 3.7, 7.12 |
| `alerts.webhook_timeout` | 10 s for all `POST` requests of one run of `alerts` (ADR-0063 (m)) | 3.7 |
| `alerts.panic_count` / `alerts.panic_window` | 3 / 10 min | 2.4, 3.7 |
| `alerts.not_ready_for` | 1 min | 3.7 |
| `alerts.failed_login_count` / `alerts.failed_login_window` | 50 / 15 min | 3.7 |
| `alerts.security_window` | 15 min | 3.7 |
| `backup.time` / `backup.retention` | 02:00 UTC / 30 days. The retention covers every set (ADR-0066 (k)) | 7.7 |
| `backup.max_age_alert` | 26 h | 3.7, 7.1, 7.7 |
| `restore_test.time` | Sunday 05:00 UTC (ADR-0067 (a)) | 7.7 |
| `basebackup.time` / `basebackup.retention` | Sunday 03:30 UTC / 4 base backups (ADR-0068 (f), (i)) | 7.7 |
| `scratch.shared_buffers` / `scratch.maintenance_work_mem` / `scratch.max_connections` | 64 MB / 64 MB / 10. The scratch clusters of `restore-test` and `pitr-check` (ADR-0067 (e)) | 7.7 |
| `scratch.max_parallel_workers_per_gather` / `scratch.max_parallel_maintenance_workers` | 0 / 0. The scratch clusters of `restore-test` and `pitr-check` (ADR-0067 (e)) | 7.7 |
| `scratch.restore_fsync` / `scratch.restore_wal_level` / `scratch.restore_max_wal_senders` | `off` / `minimal` / 0, for the cluster of `restore-test` only (ADR-0067 (f)) | 7.7 |
| `scratch.pitr_hot_standby` | `off`, for the cluster of `pitr-check` only. The lower `scratch.max_connections` then does not stop the recovery (ADR-0068 (o)) | 7.7 |
| `scratch.port` / `scratch.stop_timeout` | 5433 / 60 s. The port and the `pg_ctl stop` timeout of the scratch servers of `restore-test` and `pitr-check` (ADR-0077 (g)) | 7.7 |
| `scratch.restore_start_timeout` / `scratch.pitr_start_timeout` | 60 s / 600 s. The `pg_ctl start` timeout of the cluster of `restore-test`, and of the cluster of `pitr-check` with its recovery, inside the recovery time objective of 1 h (ADR-0077 (g)) | 7.7 |
| `restore_test.detail_length` | 160 characters of a tool message in an `ERROR` line of `restore-test` (ADR-0077 (g)) | 7.7 |
| `pitr_check.log_tail` | 20 lines of the server log on stderr after a failed start of the scratch cluster of `pitr-check` (ADR-0077 (g)) | 7.7 |
| `pitr_check.ready_poll` | 1 s. The interval of the connection check while the scratch cluster of `pitr-check` finishes its recovery. The checks stay inside `scratch.pitr_start_timeout` | 7.7 |
| `wal.max_lag_bytes` | 16 MiB. Replication lag `pg_current_wal_lsn() - sent_lsn` of `recipe_replicator` | 3.7, 7.1, 7.7 |
| `security_event.retention` | 400 days. A constant in `security_event_purge()`. A change needs a migration | 3.6, data-model.md 4.12 |
| `security_event.purge_interval` | 24 h | 2.4, data-model.md 4.12 |
| `security_event.purge_batch` | 5 000 rows per statement. A constant in `security_event_purge()`. A change needs a migration | 2.4, data-model.md 4.12 |
| `log.max_size` / `log.max_file` | 50m / 20 | 3.5 |
| `log.mode` / `log.max_buffer_size` | `non-blocking` / 4m. The delivery mode of the logging anchor (ADR-0070 (t)) | 3.5 |
| `soak.memory_ceiling` / container limit | 384 MiB / 512 MiB | 6, 7.8 |
| `container.app_memory` / `container.app_cpus` | 512 MiB / 1 | 7.8 |
| `container.db_memory` / `container.db_cpus` | 2 GiB / 2 | 7.8 |
| `container.small_memory` | 128 MiB each for `proxy` and `walarchive` (ADR-0070 (i)) | 7.8 |
| `container.small_cpus` | 0.5 each for `proxy`, `backup`, `walarchive`, `migrate`, `backup-premigrate`, and the temporary container of `pitr-check` (ADR-0070 (j), ADR-0081 (a)) | 7.7, 7.8 |
| `container.backup_memory` | 512 MiB for `backup` and for the temporary container of `pitr-check` (ADR-0067 (b)) | 7.7, 7.8 |
| `container.oneshot_memory` | 256 MiB each for `migrate` and `backup-premigrate` (ADR-0070 (h)) | 7.8 |
| `pg.max_wal_senders` | 3 | 7.6 |
| `pg.wal_keep_size` | 1 GB | 7.6 |
| `pg.checkpoint_timeout` | 5 min | 7.6 |
| `pg.shared_buffers` | 25 % of the `db` container memory | 7.6 |
| `shutdown.drain` / `app.stop_grace_period` | 30 s / 40 s | 7.3 |
| `shutdown.pool_close` / `shutdown.runtime_stop` | 5 s / 3 s. The close of both pools and the stop of the runtime after the drain (ADR-0058 (q)) | 7.3 |
| `shutdown.internal_drain` | 0 s. The drain of the internal listener after the pool close. The listener cuts its open connections at once. A code default (ADR-0087) | 7.3 |
| `clock.skew_warn` | 5 s | 7.9 |
| `clock.check_interval` | 1 h | 2.4, 7.9 |

Transaction isolation and retry are rules in the code, not settings. `SetUserRole` and `DeactivateUser` run in `READ COMMITTED` with no retry. They take the class-4 guard lock of the last Administrator first (ADR-0096). Only the first-administrator operation of `create-admin` runs in `SERIALIZABLE`. It retries once on `40001`, then returns `E-DB-001`. Every other command runs in `READ COMMITTED` with no retry. `40P01` maps to `E-DB-001` and is logged at `ERROR` (section 1.1). A query runs in `READ COMMITTED READ ONLY`. The draft queries of ADR-0042 and the queries `GetApprovalPreview` and `DiffVersions` of ADR-0050 run in `REPEATABLE READ READ ONLY`. A query never retries.

### 7.12 Stored settings

`app_setting` (`data-model.md` 3.17) stores the effective value of each of the 18 keys that `serve` reads and records (ADR-0022, ADR-0062 (a)). The keys of `migrate` and `create-admin` stay out of `app_setting` (ADR-0062 (c)). For the derived key `alerts.webhook_url`, the stored value is the fingerprint of the URL and not the effective value (ADR-0063 (c), (d)).

The internal operation `CompareSettings` compares the values at startup, before the listeners open. It runs in step 7 of the start order of ADR-0061, also in read-only mode (ADR-0061 (e)):

1. The server reads its configuration and checks every value.
2. In one transaction it reads `app_setting` and compares each configured value with the stored value. A missing row counts as a difference with the old value `null`. `data-model.md` section 6 gives the locks and the statements (ADR-0079).
3. For each difference it writes one audit entry `SettingChanged` with `aggregate_type = 'app_setting'`, `actor = None`, and the old and the new value. All entries of one start share one `request_id`. Then it updates or inserts the row.
4. When the transaction fails, the server writes one `ERROR` line and does not start.

The state keys `audit.last_check_seq`, `audit.last_check_at`, and `heartbeat.last_at` are not configuration. The comparison skips them. The jobs `audit_chain_check` and `heartbeat` write them with no audit entry (2.4).

A change of `approval.separation_of_duties` therefore always has an audit entry with the old and the new value (A34). The audit log keeps the entry without a time limit.

## 8. Code conventions

- Code comments, identifiers, commit messages, documentation, and specifications are English (R42). Comments follow the ASD-STE100 structural rules in the same way as this document.
- Each design pattern in the code names the problem it solves in the doc comment of the type that implements it (R43). A review rejects a pattern with no named problem.
- `CONTRIBUTING.md` in the repository root repeats the rules of sections 2 and 8 and links to this document.
- `ste-lint.py` checks the structural ASD-STE100 rules of English text (R40). CI runs the copy `tools/ste-lint/ste-lint.py` on the Markdown files that ADR-0023 names. One hard violation fails the job. Hard violations:
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
- No `unsafe`. No `static mut`. No global mutable state. Shared state is passed through axum `State` as `Arc<AppState>`. Interior mutability exists only in these places:
  - the connection pool and the Argon2 semaphore,
  - the read-only mode flag (`AtomicBool`),
  - the atomic counters of the job `alerts` (3.7): the panic events of the panic hook and the `E-EXP-002` results,
  - the readiness state of 3.7, as an atomic that the health router and the job `alerts` share (ADR-0091 (c)),
  - the readiness cache of 7.2, as a `tokio::sync::Mutex` in the state of the health router. It holds one probe result. A request holds the lock across the probe call, so one probe call runs at a time (ADR-0091 (a)),
  - the flag `draining` of the health router (`AtomicBool`). `begin_shutdown()` sets it once, and no code clears it. A late probe result does not make the readiness state ready (ADR-0091 (b)),
  - the flag `gate_open` of the health router (`AtomicBool`). The start of `serve` opens it once, after the public listener is bound. No code closes it (ADR-0091 (e)),
  - the cancel tokens of the supervisor of the jobs and the flags of the watchdog handle (`arm`, `disarm`, `stop`),
  - the open-run counter of the supervisor of the jobs (`Arc<AtomicUsize>`, field `open`). A guard in the task of each run adds one when the run starts. It subtracts one when the run ends or panics, or when the drain aborts it. Many tasks write it, so it is atomic. The guard uses `AcqRel`. `drain` reads it once with `Acquire`, before `abort_all`. That value is the number of runs that the drain cut (ADR-0060 (aa)),
  - the run state of a job between runs, as atomics in the job (ADR-0060 (aa)). It holds the start of the last security event purge of `cleanup` and the due times of its two other cadences (ADR-0088 (c)). It also holds the time of the last low-disk `WARN` line of `disk_check`, the time of its last read-fault `WARN` line, and its `Unsupported` flag (ADR-0073 (d)). For `alerts`, it holds the two counter totals of the last run and the times of the newest panics in `alerts.panic_count` + 1 slots (ADR-0073 (g)).
- Every public type and function in a library crate has a doc comment. `cargo doc --no-deps` runs in CI with `-D warnings` for broken links.
- A pull request needs: green CI, one review, and an entry in `CHANGELOG.md` when behavior changes.
