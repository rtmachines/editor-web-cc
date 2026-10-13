# Design: app-use-cases-and-ports

## Context

See `proposal.md`, section Why. `recipe-domain` holds the modules `plant`, `recipe`, `identity` (`UserId`, `Login`), `text`, `timestamp`, and `error_codes` of P2.1 and P2.2. `recipe-app` is an empty crate. This change reuses the domain API of ADR-0028. It does not copy these types: `Recipe`, `Draft`, `VersionSnapshot`, `RebindPlan`, `TypeTrees`, `PlatformTypes`, `LiveUnits`, `Machine`, `PlcTypeRecord`, `TypeExpr`, `Optimistic`, `RecipeErrors`, and `DraftError`.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Commands, errors, commit paths, queries | `domain-model.md` sections 5, 5.1 to 5.5 |
| Statements and lock sequence of each command | `data-model.md` section 6 |
| Advisory locks of derived types | `data-model.md` section 1.4, `domain-model.md` section 5.1 |
| Audit entry, state rule, aggregate of each entity | `domain-model.md` section 2.4, `data-model.md` sections 9.2 and 10 |
| Security event | `domain-model.md` section 2.4, `data-model.md` section 3.8 |
| Permission enum, matrix, role table | `api.md` sections 3.1 to 3.3 |
| Row version protocol, idempotent approve | `api.md` section 4, ADR-0008, ADR-0028 |
| Export document, hashed body, array orders | `api.md` section 6, ADR-0019 |
| Error types, error codes, constraint mapping | `reliability.md` sections 1.1 and 1.2 |
| `spawn_blocking`, `JoinError`, timeouts | `reliability.md` sections 2.2, 2.4, 4.4, 7.4 |
| Test levels, permission matrix test | `reliability.md` section 6 |
| Settings | `reliability.md` section 7.11 |
| Crate dependencies | `overview.md` section 5 |
| Permission check first, forbidden event | ADR-0013, ADR-0022 |
| Lock order, commit paths, validation before the transaction | ADR-0015 |
| Aggregates, root and draft repositories | ADR-0016 |
| Derived types, rebind | ADR-0017, ADR-0018 |
| Delete rules | ADR-0021 |
| Error lists, `DraftError` | ADR-0027 |
| Version header, approval token | ADR-0028 |
| Decisions D3, D9, D12, D17, D18, D21 of this change | ADR-0029 (this change) |

## Goals / Non-Goals

**Goals:**

- The permission check is the first statement of every use case. A use case without an operation in the catalog does not compile.
- Every command that changes data commits through `UnitOfWork::commit(NonEmpty<AuditEntry>)`. A command without an audit entry does not compile (I13).
- Every command takes its locks in the order of ADR-0015 and runs the domain method on the locked state. The in-memory adapter records the order, and a test checks it.
- No use case waits for I/O inside a transaction. The snapshot check runs in `spawn_blocking` before the transaction (`reliability.md` section 7.4).
- No path can panic. `JoinError` gives `E-WEB-011`, and an elapsed limit gives `E-WEB-009`.

**Non-Goals:**

- No SQL, no `sqlx`, and no `axum`. P2.4 implements the ports.
- No query use case and no read model of a page (D1).
- No configuration file. `recipe-server` reads `reliability.md` section 7.11 later and passes `AppSettings`.

## Decisions

### D1. Scope (open points 2 and 7)

The full scope of the plan has about 95 tasks. This change has about 60 tasks:

- The foundation: the errors, the ports, the authorization, the blocking work, and the in-memory adapters.
- The 7 Plant Model commands, `EnsureDerivedType`, the 16 Recipe Management commands, and `ExportVersion`.
- The catalog of every operation, also the 9 Identity and Access commands and all queries. The permission matrix test therefore covers every row of `api.md` section 3.3 now.

P2.5 adds the Identity and Access commands, their ports, and the identity queries. It already plans Argon2, the login throttle, and the session table. P3.2, P3.3, and P3.4 add the other queries with the pages that show them. A read model needs the page that uses it. `MaintenanceStore` goes to P4.1 with the supervised jobs.

Alternative: all 33 commands and all queries in this change. Rejected, because the change then has about 95 tasks and the identity ports need types of P2.5.

### D2. Module layout

- `recipe-domain::identity`: add `Permission`, `Role`, and `Role::permissions` (`api.md` section 3.1).
- `recipe-domain::audit`: `AuditAction`, `AuditEntry`, `AggregateType`, `SessionRef`, `SecurityEvent`, `SecurityEventKind`, `UserAgent`, and `LoginHmac`. The module imports no entity of another context (`domain-model.md` section 1).
- `recipe-domain::export`: `ExportError`, `Sha256`, and `HashedBody` (D15).
- `recipe-domain::recipe`: add `VersionSnapshot::parameters` for the export of a stored version (D15).
- `recipe-domain::error_codes`: the constants of `E-WEB-001`, `-002`, `-004`, `-005`, `-009`, `-011`, `E-AUD-001`, `-002`, `E-DB-001` to `-008`, and `E-EXP-001` to `-004` (D4 adds the trigger codes).
- `recipe-app`: `error`, `ports`, `auth`, `blocking`, `audit`, `plant`, `recipe`, `export`, and `memory`.
- The module `memory` holds the in-memory adapters under `#[cfg(any(test, feature = "memory"))]`. Unit tests compile it without a flag. The HTTP tests of `recipe-web` enable the feature (`reliability.md` section 6).
- The module `memory` is not test code. It follows every lint of `reliability.md` section 2. `recipe-server` never wires it.

### D3. Dependencies of `recipe-app`

`recipe-app` depends on these crates:

- `recipe-domain`, `thiserror`, `async-trait`, and `tracing`.
- `tokio` with the features `rt` and `time`. The tests add `macros` and `test-util`.
- `serde_json` for the audit states and `uuid` with the feature `v4` for new identifiers.
- `parking_lot` and `time`, only with the feature `memory`. The in-memory adapters lock their state through `&self` (`.claude/rules/rust.md`), and the in-memory clock uses `time`.

`thiserror` is the rule of every library crate (R33). `spawn_blocking` and the limit of `timeout.validation` need `tokio` (`reliability.md` sections 2.2 and 4.4). `overview.md` section 5 names three crates as examples. ADR-0029 adds the full list to that row. The forbidden list stays: `sqlx`, `axum`, `recipe-export`, and `recipe-db`.

Alternative: a port for blocking work, implemented in `recipe-server`. Rejected, because `reliability.md` section 2.2 gives the mapping of `JoinError` to the use case.

### D4. Errors (open point 4)

- `AppError` has the variants of `reliability.md` section 1.1. `#[from] IdentityError` follows in P2.5 with `IdentityError`. `code()` gives the code of the inner error or of the variant.
- `From<DraftError>` gives the inner `RecipeError`, `PlantError`, or `RecipeErrors` (ADR-0027). `RecipeErrors` has its own `#[from]`. `SnapshotItem` stays inside `RecipeError::ValidationFailed`.
- `Optimistic::Conflict { expected, actual }` becomes `ConcurrentEdit { reference, number, expected, actual }`. One function holds this mapping.
- `NotFound { entity }` takes the enum `Missing` with one variant for each URL value of `api.md` section 1. Its `Display` gives the text of `{entity}`.
- A user message that ends with `Reference: {request_id}.` ends before that sentence in `Display`. `recipe-web` adds the reference, because only the request knows it (`api.md` section 5.2).
- `StorageError::Constraint` carries an `ErrorCode`. The registry therefore gets the trigger codes `E-PLT-030` to `E-PLT-038`, `E-RCP-030` to `E-RCP-036`, and `E-DB-003`. `ErrorCode::find` gives the code of a text, for the trigger messages of P2.4.
- One table holds the constraint names of `reliability.md` section 1.1 and `data-model.md` section 6. A use case maps `StorageError::Duplicate` and `MissingReference` with this table. An unknown name stays `AppError::Storage` with `E-DB-006` or `E-DB-007`. P2.4 uses the same constants.
- A mapped duplicate takes its values from the command. The domain check on the locked state fires first (D10), so the database rule is the second line.

### D5. Ports and transactions

- `UnitOfWork::begin` opens one transaction and returns a `Transaction`. Its sub-ports are `PlantRepository`, `UnitLookup`, `TypeCatalog`, `RecipeRepository` (root), and `DraftRepository` (ADR-0016).
- `UnitOfWork::commit(tx, entries)` is the only commit path of ADR-0015. It is an inherent method of `dyn UnitOfWork`, so an adapter cannot replace it.
- `commit` calls `Transaction::append_audit` first. Its `StorageError` becomes the source of `AuditWriteFailed` (`E-AUD-001`), and `commit` logs nothing. Then it calls `Transaction::finish`, whose error stays `AppError::Storage`. `reliability.md` section 1.1 therefore keeps three error types.
- `finish` takes the token `AuditWritten`. Only `UnitOfWork::commit` can build the token, so a use case cannot commit without entries (I13).
- Drop without commit is a rollback.
- A lock method takes the SQL lock and returns the locked state: the `Recipe` root, the `Machine`, the units, or the `StoredDraft`. It also returns the row image before the change (D9). A lock method names its SQL lock, for example `lock_machine_shared`.
- A write method names its intent, for example `approve_version` (`domain-model.md` section 2.2). It returns the row image after the change.
- `touch_draft` writes the touch statement and the `draft_editor` row and returns the new row version. The response header `HX-Trigger` needs it (`api.md` section 4).
- A read before the transaction uses `Isolation::ReadOnly` and takes no lock. It only resolves the URL values and reads the input of a snapshot check. The use case drops that transaction before the blocking work, so no connection waits for CPU work.
- `SecurityEventSink` and `Clock` are separate ports. Every port is `Send + Sync`, and every async port uses `async-trait`.

### D6. Current user and request data

`CurrentUser` holds the `UserId`, the `Login`, and the `Role`. P2.5 builds it per request from the session and `app_user` (ADR-0014). `RequestMeta` holds the request id, the client address, the user agent, and the `SessionRef`. Every `AuditEntry` and `SecurityEvent` takes these values from `RequestMeta`.

### D7. Operation catalog and permission check

- `Operation` has one variant for each use case and route of `api.md` section 3.3. `ExportVersion` has two variants, one for `/approved` and one for `/versions/{n}`. `ListRecipes` with `show_deleted` is one more variant. The catalog has 68 variants.
- `Operation::requirement` gives a permission, any one of two permissions, "authenticated", or "no session". The match has no wildcard, so a new variant without a row does not compile.
- Each use case declares its operation. `authorize` is its first statement. A `Forbidden` result writes one `SecurityEvent` of kind `forbidden` and then returns `E-WEB-002`. No transaction of the command opens.
- `Operation::runs_in_read_only` is the classification of A24. `recipe-web` returns `E-WEB-006` with it, because `WebError::ReadOnlyMode` owns the code (`reliability.md` section 1.2).
- The matrix test holds one row for each pair of operation and role, copied from `api.md` section 3.3. A second test reads column 1 of `api.md` section 3.3 and compares each operation name in backticks with the catalog. An explicit list excludes the other names in backticks, for example `Approved`, `Retired`, and `show_deleted`.

### D8. Blocking work

- `Blocking` holds the limits `timeout.validation` and `timeout.export` and runs a closure in `spawn_blocking` inside `tokio::time::timeout`.
- An elapsed limit gives `Timeout { stage }`. A `JoinError` gives `TaskFailed { stage }`.
- With the feature `memory`, `Blocking::elapsed` and `Blocking::panicked` are test doubles. The use case tests of `E-WEB-009` and `E-WEB-011` are therefore deterministic.
- `AppSettings` holds the `Blocking` limits and the `ApprovalPolicy`.

### D9. Audit entries

- `AuditEntry` has one constructor for each state kind of the state rule: row, diff, and state object. The constructor rejects an entry without a state, so `audit_log_has_state` cannot fail.
- The lock method gives the row image before the change, and the write method gives the image after it. PostgreSQL 16 has no `RETURNING OLD` (A15). P2.4 builds each image with `to_jsonb`, without `password_hash`. The image keys are the column names of the table in `data-model.md` section 3, so the in-memory images and the stored images agree.
- The image of a parameter holds its limit rows and its translation rows. The image of a struct holds its member rows (delta requirement "Full row with child rows").
- `recipe-app` builds the five diffs from `ParameterChange`:
  - A value diff uses leaf paths as keys (`data-model.md` section 9.2).
  - A definition diff uses column names.
  - A limit diff uses leaf paths, and a translation diff uses the language code.
  - A rebind diff uses `plc_type_id`, the value leaf paths, and the limit leaf paths.
- `VersionApproved` adds `allow_outdated` to the row image. `VersionExported` holds the version id, the schema version, and `content_sha256`.

### D10. Pipeline of a command

1. `authorize`.
2. Read without a lock and resolve the URL values. A missing value gives `E-WEB-005`.
3. `CreateVersion` and `ApproveVersion` only: read the snapshot, then check it with `Blocking`.
4. Open the transaction. Take the locks in the order of `data-model.md` section 6. Each lock returns the locked state.
5. Call the domain method on the locked state. A draft command loads the `Draft` from its locked row and compares its row version first (I12).
6. Write through the intent methods. A draft command also calls `touch_draft`.
7. Commit with the entries of the command.

The state of step 2 never reaches a domain method of a command. For example, an approval that commits between step 2 and step 4 makes `DeleteRecipe` return `E-RCP-010`. A draft command changes the database only after the domain method succeeds.

### D11. `EnsureDerivedType` and the advisory keys

- `DerivedTypeLock` has the values struct name, `STRING`, `WSTRING`, and `ARRAY`, ordered as `data-model.md` section 1.4 gives. A command parses every `TypeExpr` first and collects its keys.
- `RebindTypeRevision` collects `ARRAY` from `RebindPlan::array_shapes` (open point 6).
- `EnsureDerivedType` is a function of `recipe-app::plant`, not a use case. It takes a `TypeExpr` or an `ArrayShape`. It resolves the innermost element first through `TypeCatalog` and takes no lock.
- An `ArrayShape` names its element type and its nested dimensions. `EnsureDerivedType` creates one array row for each level, the innermost level first.
- A new row gives one `DerivedTypeCreated` entry. `PlcType::try_from` checks the row before the insert.
- `DefineUdt` maps a member type that is not on the platform from `E-PLT-013` to `E-PLT-006` with the member name.

### D12. `ApproveVersion` and the approval token

`ApprovalReady` borrows the `Draft` (ADR-0028). A borrowed token cannot leave `spawn_blocking`, so the check cannot run there. ADR-0029 changes the token: `Draft::check_for_approval` consumes the `Draft`, and `ApprovalReady` owns it. While the token lives, no command can change the draft. `ApprovalReady::draft` gives read access. The token loses `PartialEq` and `Eq`, because `Draft` has neither.

The flow follows `data-model.md` section 6:

1. `authorize`, then `approval_replay` and `check_approvable` on the root.
2. Read the draft. When it is no longer a live draft, read the root again and repeat step 1. A request that lost a race therefore gets `AlreadyApproved`, not `E-RCP-002` (`api.md` section 4).
3. With `Blocking`: `check_for_approval`, then the hashed body and its SHA-256 (D15).
4. In the transaction: lock the recipe and the version. Run `check_limits` and `check_outdated` with the type rows of the transaction.
5. `Recipe::approve` consumes the token. `AlreadyApproved` drops the transaction and writes no entry.
6. Write `VersionRetired` when the approval retired a version. Then write `VersionApproved`.

`Recipe::approve` compares the row version of the token with the locked header. A match proves that the locked draft equals the checked draft. The second `check_limits` therefore gives the "limit count" check of `data-model.md` section 6, and ADR-0029 records this. The second `check_outdated` reads new revisions.

`serde_json` cannot fail on the hashed body, so no seam produces `E-EXP-004` in a use case. Tasks 3.5 and 4.2 test its mapping.

Alternatives: run `check_for_approval` a second time in the transaction to get a token, or run the whole approval inside the transaction. Rejected, because both run the snapshot check under the locks (ADR-0015).

### D13. `CreateVersion`

1. Check the root on a copy with `Recipe::create_version` (`E-RCP-012`, `E-RCP-011`) and the machine (`E-PLT-010`). A deleted recipe therefore gives `E-RCP-012`, not an item of `E-RCP-020`.
2. Run `VersionSnapshot::check` with `Blocking`.
3. In the transaction, lock the units of the base version in `id` order and check `archived_at` again. `Recipe::create_version` on the locked root gives the number.

The response lists the outdated tags.

### D14. `RebindTypeRevision` (open point 6)

The order follows `data-model.md` section 6 and section 1.4:

1. Lock the machine, the recipe, and the version. Load the `Draft` and compare the row version.
2. Read the type rows and call `plan_rebind`.
3. Take the key `ARRAY` when `array_shapes` is not empty.
4. Call `EnsureDerivedType` for each shape. Resolve the trees of `RebindPlan::parameters` with the new rows.
5. Call `apply_rebind`. Write each moved parameter. Commit `DerivedTypeCreated` and one `TypeRevisionRebound` for each parameter.

### D15. Export module (open point 1)

- `ApproveVersion` needs `content_sha256`. The baseline requirement "Content hash at approval" and the constraint of `data-model.md` section 3.10 require it. `ExportVersion` needs the same hash. The module `export` of `recipe-domain` therefore comes in this change (`overview.md` section 5).
- `HashedBody` holds the fields of `api.md` section 6.1 with "In hash" yes, in the orders of section 6.3. It takes the header values, the parameters as `&[&Parameter]`, the unit paths, and the `TypeTrees`. A `Draft` and a `VersionSnapshot` both give the parameters.
- `HashedBody::sha256` writes the RFC 8785 form with `serde_json_canonicalizer` and hashes it with `sha2`.
- `ExportVersion` checks its permission by route, then locks the version `FOR SHARE`, rebuilds the body in `spawn_blocking`, and compares the hash (`E-EXP-002`). `timeout.export` limits these steps, and the commit runs after it (ADR-0029). It returns `ExportedVersion`: the body and the fields outside the hash.
- P3.5 keeps the JSON Schema, the HTTP writer, the golden files, and the routes.

### D16. `DeleteRecipe` and the role

`recipe-app` maps `Role` to `RecipeDeleter`. `Operator` has no `RecipeDelete`, so `authorize` stops it first. The map still returns `Forbidden` for `Operator`, and it does not panic.

### D17. Definition update without a type change (open point 5)

`Draft::set_definition` accepts `Retype` also without a type change. The use case passes it on. The value and the limits change, and the diff shows both (delta requirement "Type change needs a new value"). ADR-0029 adds this case to `api.md` section 2.6, `domain-model.md` section 5.2, and `data-model.md` section 6.

Alternatives: reject the request with a new code, or drop the value. Rejected. A new code needs a registry row for a request that the form never sends. A dropped value loses input without a message.

### D18. Forbidden event that cannot be written

When `SecurityEventSink::record` fails, `authorize` returns the storage error. A forbidden result is then not invisible (delta requirement "Forbidden result event"). ADR-0029 records the rule.

### D19. Identifiers and time

The use case creates the identifier of each new row with UUID version 4. `Clock::now` gives the `Timestamp` of a domain method, for example `Recipe::delete`.

### D20. Tests

- Each use case has a success test and a test of its audit entries. It has one test for each error of its row in `domain-model.md` section 5.
- A fixture uses the platform of its machine. A draft of an `S7_300` machine uses `S7_300` type rows.
- A generic test runs each command with a role that has no permission. It finds one `forbidden` event and no transaction. The export of `/approved` has no such role.
- The matrix test and the read-only test cover every operation of the catalog.
- Race tests change the in-memory state between the read of step 2 and the locks of step 4 (D10, D12).

### D21. Archive of an archived unit

`Machine::archive_unit` returns an empty list when the unit is already archived. The commit needs one entry, so the use case returns `E-PLT-009` (delta requirement "Archive of an archived unit"). ADR-0029 adds the code to the row `ArchiveUnit` of `domain-model.md` section 5.1 and of `api.md` section 2.2.

### Signatures

```rust
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError>;
}
impl dyn UnitOfWork {
    pub async fn commit(&self, tx: Box<dyn Transaction>, entries: NonEmpty<AuditEntry>) -> Result<(), AppError>;
}
#[async_trait]
pub trait Transaction: Send {
    fn plant(&mut self) -> &mut dyn PlantRepository;   // also units(), types(), recipes()
    fn drafts(&mut self) -> &mut dyn DraftRepository;
    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError>;
    async fn finish(self: Box<Self>, written: AuditWritten) -> Result<(), StorageError>;
}
pub async fn authorize(op: Operation, user: &CurrentUser, meta: &RequestMeta,
                       sink: &dyn SecurityEventSink) -> Result<(), AppError>;
impl Draft { pub fn check_for_approval(self, units: &LiveUnits, trees: &TypeTrees,
                                       allow_outdated: bool) -> Result<ApprovalReady, DraftError>; }
```

## Risks / Trade-offs

- [The ports exist before the sqlx adapters] → Each method follows one statement group of `data-model.md` section 6. P2.4 can change a method in its own change.
- [The in-memory adapter has no PostgreSQL locks or triggers] → It records the lock order only. P2.4 runs the integration and concurrency tests of `reliability.md` section 6.
- [`check_limits` runs under the locks] → `data-model.md` section 6 requires the second check. The budgets limit its cost, and the load test of `reliability.md` section 6 measures it.
- [ADR-0029 changes the approval token of P2.2] → The change is small. The `compile_fail` example of the token moves from a borrow error to a move error.
- [`ExportVersion` hashes the body in async code] → `timeout.export` cannot stop a running hash, and the hash occupies one worker. `reliability.md` section 1.1 has no `TaskFailed` stage for the export. The budgets of A26 limit the size, and the load test of P4.2 measures it.
- [The matrix test reads Markdown] → It reads only column 1 of the table of `api.md` section 3.3, with an explicit list of excluded names.
- [About 60 tasks] → Subagents work in sequence in one crate, three to four tasks for each agent.

## Migration Plan

No deployment and no migration. P2.4 implements the ports with `sqlx`. A rollback before P2.4 is a revert of the commits.

## Open Questions

- The read ports of the queries come with P2.5, P3.2, P3.3, and P3.4.
