# Design: domain-recipe

## Context

See `proposal.md`, section Why. `recipe-domain` holds the module `plant` of P2.1 and `ReferenceNumber`. The module `plant` gives `TypeTree`, `PlatformTypes`, `LeafPath::resolve`, `S7Address`, `S7Width`, `S7Layout`, `IecLiteral`, `TimeType`, `Text`, and the proptest strategy `type_trees`. This change reuses them and does not copy them.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Recipe, version, parameter, limit, value, approval policy | `domain-model.md` section 2.2 |
| Leaf classes, base units, IEC literal, S7 rules, `TypeTrees` | `domain-model.md` section 2.1 |
| Invariants I1 to I7, I12, I15 to I20 | `domain-model.md` section 3 |
| Version state machine, outdated rule | `domain-model.md` section 4 |
| Commands, errors, rebind rules, `ValidationFailed` | `domain-model.md` section 5.2, `data-model.md` section 6 |
| Row version protocol | `api.md` section 4, ADR-0008 |
| Error codes and message templates | `reliability.md` sections 1.1, 1.2, 1.3 |
| Value objects, value JSON | `reliability.md` sections 4.3, 4.4 |
| Budgets, text limits, `json.max_depth` | `reliability.md` section 7.11, `data-model.md` sections 1.3, 3.12, 3.13 |
| Limits per leaf, `[*]` | ADR-0007, ADR-0022 |
| Aggregates | ADR-0009, ADR-0016 |
| Rebind | ADR-0018 |
| Rules of `DeleteRecipe` and `RestoreRecipe` | ADR-0021 |
| Separation of duties | ADR-0022 |
| Width code `E-RCP-054` | ADR-0024 |
| Parse reasons | ADR-0025 |
| Error lists, `DraftError` | ADR-0027 (this change) |
| Version header, approval token, clarifications | ADR-0028 (this change) |

## Goals / Non-Goals

**Goals:**

- No public function builds an invalid `VersionHeader` set, an invalid `Draft`, or an invalid `ParameterValue`.
- Only a draft that passed `check_for_approval` can become `Approved`.
- A draft command checks the whole result first. It changes the `Draft` only when every check passes.
- Every public function returns `Result` or `Option`. No path can panic.

**Non-Goals:**

- No audit entry, no `draft_editor` row, and no lock order. These belong to P2.3.
- No module `export` and no `content_sha256`. The plan of P2.3 decides the order of P2.3 and P3.5.
- No configuration of the values of `reliability.md` section 7.11. The values are constants, as in P2.1.

## Decisions

### D1. Module layout

- `recipe`: `ids`, `error`, `state`, `optimistic`, `root`, `approval`, `draft`, `parameter`, `tag`, `lang_code`, `limit`, `value`, `snapshot`, `rebind`, `budget`.
- `identity`: `UserId` and `Login` (D2).
- `plant`: add `TypeTrees`, `LiveUnits`, `LiveUnit`, `TypeTree::platform`, `TypeTree::leaves`, and `TypeTree::rebind` (D7, D13).
- `text`: add `LongText` (D15).

Alternative: a module `recipe::types` for `TypeTrees`. Rejected, because `domain-model.md` section 2.1 puts `TypeTrees` in the Plant Model, and `overview.md` section 4 names `LiveUnits` a read value of that context.

### D2. Identity value types (open point 2)

`E-RCP-015` carries `login: Login` (`reliability.md` section 1.3). `domain-model.md` section 1 permits a value object of another context in an error variant. The change therefore adds `UserId` and `Login` to the module `identity`, as P2.1 added `ReferenceNumber`. `Login::parse` returns `LoginReason` (ADR-0025). `Role` stays in P2.5, because `Recipe::delete` takes `RecipeDeleter` (D4).

### D3. State machine as one function (`domain-model.md` section 4)

`VersionState` has four values: `LiveDraft`, `Discarded`, `Approved`, and `Retired`. `VersionHeader` stores a `VersionState`, so a header cannot be approved and discarded at once (ADR-0028). `VersionState::status` gives the `VersionStatus` of the SQL column. `Transition` has four values: `Approve`, `Retire`, `Discard`, and `Edit`.

`VersionState::apply` is the one shared function of ADR-0016. It accepts four pairs. The other twelve pairs return `ForbiddenTransition`. The caller maps it: `approve` and `discard` to `E-RCP-007`, `retire` to `E-RCP-009`, and `Draft::load` to `E-RCP-002`.

### D4. Recipe root and version commands

`Recipe` has the fields of `domain-model.md` section 2.2. `TryFrom<StoredRecipe>` checks unique numbers from 1, at most one `Approved` (I1), and at most one live draft (A7). A failure gives `StoredRecipeReason`, and P2.4 maps it to a storage error.

- `retire` and `discard` take the version number of the URL. An unknown number gives the error of the forbidden transition with that number. The use case first calls `Recipe::version` and maps `None` to `E-WEB-005`.
- `approve`, `retire`, and `discard` take the expected row version (ADR-0008). They return `Optimistic<T>`. `Optimistic::Conflict` holds the expected and the actual value, and P2.3 maps it to `E-WEB-001`.
- The order follows `api.md` section 4: the idempotent case of `approve` first, then the row version, then the transition. The scenario "approved version with an old row version" therefore gives `E-WEB-001`.
- `delete` takes `RecipeDeleter` with the two values `Administrator` and `ProcessEngineer` (ADR-0021). It discards the live draft in the same call.
- `create_version` takes the new identifier. It gives the number `max + 1` with `checked_add` and counts discarded drafts. Any version can be the base, also a discarded draft (ADR-0028). The SQL statements copy the parameters (`data-model.md` section 6).
- A version number fits the SQL type `INTEGER` (`data-model.md` section 1.2). `TryFrom<StoredRecipe>` rejects a larger number. A next number above the range gives `E-RCP-021` with the budget "version numbers".
- The root holds no timestamps of a version. `approve`, `retire`, and `discard` take no time, because the SQL statements write `now()` (`data-model.md` section 6). `delete` and `restore` change `deleted_at` of the root and take the time.

### D5. Approval (I1, I2, I16, I17, A29, A34)

1. Before the transaction, the use case calls `Recipe::check_approvable` (`E-RCP-007`) and then `Draft::check_for_approval`. Success returns the token `ApprovalReady`. Only this function creates the token. The token borrows the `Draft`, so no draft command can change it while the token lives.
2. The token holds the version identifier, the number, the row version, the creator, the editors, and `allow_outdated`.
3. Under the locks, the use case calls `check_limits` and `check_outdated` again (`data-model.md` section 6).
4. `Recipe::approve` consumes the token. A token with another row version than the header gives `Optimistic::Conflict`.
5. `approve` checks separation of duties against the creator and the editors of the token. `E-RCP-015` names the login of `Approver`, which is the approving user.
6. `Recipe::approval_replay` gives `AlreadyApproved` before the transaction, because an approved version has no draft for a token. `approve` gives it inside the transaction.

`approve` receives no hash. `ApprovalOutcome` holds identifiers only (open point 4, ADR-0019).

### D6. Draft aggregate

`Draft::load` takes the stored version, its parameters, the creator, and the editors. It returns `E-RCP-002` for a version that is not a live draft (I3). It trusts the stored values and limits, as `reliability.md` section 7.4 puts validation before the transaction. A duplicate tag or key gives `E-RCP-014`.

- A loaded parameter keeps its value as `StoredValue` and its limits as `StoredLimit` rows. A command parses only the parameter that it changes.
- A parameter whose type no longer resolves stays in the draft. `RemoveParameter` and `UpdateParameterDefinition` can still change it. The snapshot check reports it (`E-RCP-013`).
- Each command builds the changed parameter first. Then it checks the draft in this order:
  1. The tag, the unit, the type tree, the S7 platform rule, the S7 width, and the S7 alignment.
  2. The limits, the value shape and range, and the value against the limits.
  3. The budgets and the S7 overlap.
- The command replaces the parameter only after every check passes. It returns `ParameterChange` with the old and the new parameter for the audit diff of P2.3.
- `NewParameter` takes a `LiveUnit`. Only `LiveUnits::require` creates one (I15). A `LiveUnit` or a `TypeTrees` of another machine gives `E-PLT-020` or `E-PLT-013` through `DraftError` (ADR-0027).
- The use case compares the row version of a draft command with `Draft::row_version`, after `Draft::load` (I12, `data-model.md` section 6).

### D7. Type trees of a draft

- `TypeTree::resolve` copies the platform of `PlatformTypes`, so `ParameterValue::check` keeps the signature of I7.
- `TypeTree::leaves` walks the tree with an explicit stack. It gives each scalar leaf with its concrete `LeafPath`. I17, the `[*]` limits, and the rebind use it (J1).
- `TypeTrees::resolve` takes the type identifiers of a draft and `PlatformTypes`. It keeps one result for each identifier: a tree, or the `PlantError` of the resolution. It also keeps the newest revision of each struct name.
- `PlatformTypes` keeps the name of each row that it drops. `E-RCP-013` then names the type and not only its identifier.
- A struct node with a newer revision makes the parameter outdated (A29). `E-RCP-024` names the first outdated parameter in tag order.

### D8. Parameter value (I7, `reliability.md` section 4.4)

`ParameterValue::parse` reads the request form. `TryFrom<(&StoredValue, &TypeTree)>` reads the stored form, where a time leaf is a number. Both parse with `serde_json` (`arbitrary_precision`) and walk the tree with an explicit stack.

- A value records the root type of its tree. `check` with another tree gives a fault.
- A numeric leaf becomes a `Decimal`. A text with an exponent uses `Decimal::from_scientific`. More than 28 significant digits gives a fault.
- A table in code holds the range of each numeric type. A fraction for an integer type is outside the type range (ADR-0028).
- The range of `Decimal` lies inside the IEEE 754 ranges of `REAL` and `LREAL`. A unit test proves this with `Decimal::MAX` and `Decimal::MIN` (ADR-0028).
- A `STRING` character outside Latin-1 has no byte in the target encoding, so it gives a fault (ADR-0028).
- The canonical text comes from `Decimal::normalize`, and `-0` becomes `0`.
- A time leaf uses `IecLiteral::parse` with `TimeType::of(primitive, tree.platform())`.

`parse` and `check` return `ValueFault` with the leaf path and the expected form. `ValueFault` has no `code()`. `Draft` adds the tag and wraps it into `E-RCP-005` or `E-RCP-004` (ADR-0025).

The fields of `E-RCP-004` are texts for the message. A missing bound shows the bound of the type. A string leaf shows its length in the unit of its type, `0` as `min`, and `max_length` as `max`. A time leaf shows the canonical literal.

### D9. Process limits (ADR-0007, ADR-0022)

`LimitList::parse` takes the rows of a request. These cases give `LimitListReason`, and `recipe-web` wraps it into `E-WEB-008` (ADR-0025):

- Two rows for one leaf path.
- A `[*]` row and an index row at one array level. "One array level" means the same segments before the position of `[*]`, the literal text of ADR-0007.
- A row without a field.

`ProcessLimit::parse` resolves the leaf path against the tree and returns `LimitFault`. The draft wraps it: no leaf to `E-RCP-017`, a field of another class to `E-RCP-019`, and a bound outside the type range to `E-RCP-006`. `min > max` gives `E-RCP-018`. A time bound is an IEC literal in a request and a count in storage. A `max_length` outside `1..=string_length` gives `E-RCP-006`.

`ParameterValue::check` takes `&[ProcessLimit]` and rejects two limits for one leaf, so a slice that skips `LimitList` cannot pass.

### D10. S7 rules

- I5: `E-RCP-003` and `E-RCP-053` by the platform of the draft (A31).
- Width: `S7Width::expected_for(tree)` against `S7Address::width` gives `E-RCP-054` (ADR-0024).
- Alignment: `S7Layout::start` of `EvenByte` needs an even offset (`E-RCP-023`).
- Overlap: a `BOOL` occupies one bit. Every other parameter occupies the bits of its layout bytes. A sorted sweep per data block finds the first overlap (`E-RCP-022`). The arithmetic uses `checked_*`.
- `Draft::check_s7_layout(&TypeTrees)` runs these rules for the whole draft (I20).

### D11. Budgets (I18)

The constants equal `reliability.md` section 7.11. The leaf count is the sum of `TypeTree::leaf_count`. The limit count is the sum of the rows. Each command that can grow a count calls one budget function and returns `E-RCP-021`.

### D12. Snapshot and approval checks (I17, I19)

`VersionSnapshot` holds the stored parameters of any version. `VersionSnapshot::check(&Recipe, &LiveUnits, &TypeTrees)` parses every value and limit. `CreateVersion` uses it for its base. A failure returns `E-RCP-020` with every item. Success returns the tags of the outdated parameters.

An item holds the tag and the `PlantError` or `RecipeError` of the problem. `SnapshotItem::code` returns the code of that error, so an item adds no second owner of a code. The items are `E-PLT-009`, `E-RCP-004`, `E-RCP-005`, `E-RCP-013`, `E-PLT-010`, and `E-RCP-012`. `E-RCP-005` names a stored value that no longer fits its type, for example after a range change (ADR-0028). A stored limit that no longer fits gives its limit code, and an unknown unit gives `E-PLT-020` (ADR-0028). `{count}` of `E-RCP-020` counts the parameters with an item, not the items.

`check_for_approval` and `VersionSnapshot::check` first check that `TypeTrees` and `LiveUnits` belong to the machine and the platform of the draft. A type that `TypeTrees` does not hold gives `E-PLT-013` in every whole-draft check.

`Draft::check_for_approval` runs the same check on the draft. Then it checks `E-RCP-008`, `E-RCP-016`, the S7 rules, and `E-RCP-024` without `allow_outdated`. It returns `DraftError`. Its list holds one `E-RCP-016` entry for each leaf without a limit (ADR-0027).

### D13. Rebind (ADR-0018, open point 3)

The mapping is pure, so it belongs to this change. `reliability.md` section 6 needs a domain test for `E-RCP-028`, `E-RCP-029`, and `E-RCP-052`.

1. A parameter uses the source revision when its type is that struct, or an array whose element chain ends in it. A struct member never counts (`domain-model.md` section 5.2).
2. `Draft::plan_rebind` checks the target. Another struct name, an older revision, a target that the machine does not hold, or no user gives `E-RCP-052` with `RebindTargetReason`.
3. `TypeTree::rebind` builds the target tree. It replaces the struct node along the element chain and keeps the array dimensions. A tree above a size limit gives `E-PLT-018`.
4. The mapping goes member by member. A member with the same name and the same type identifier keeps its subtree, its values, and its limits. Every other member is new.
5. A new scalar leaf takes its value from `new_member_values` by its concrete leaf path. A missing entry gives `E-RCP-028`. A bad value gives `E-RCP-005`.
6. A limit whose leaf is in a removed or new member gives `E-RCP-029`.
7. `plan_rebind` collects every entry of every parameter into the `RecipeErrors` of a `DraftError` before it returns.
8. The plan lists the array shapes that `EnsureDerivedType` must create (P2.3).
9. `Draft::apply_rebind` takes the plan and the trees of the new types. It checks that each tree equals the planned tree, checks the budgets and the S7 overlap (`E-RCP-022`, ADR-0028), and then changes the draft.

### D14. Errors (ADR-0027)

- `RecipeError` uses `thiserror`. Each `#[error]` text equals the user message of its row in `reliability.md` section 1.2.
- Each variant gets a constant in `ErrorCode`, a row in `REGISTRY`, and an arm in `code()`. The registry test of P2.1 D4 also reads the `E-RCP` domain rows.
- `RecipeErrors` holds one or more `RecipeError` values. `code()` returns the code of the first entry.
- `DraftError` is a sum of `RecipeError`, `PlantError`, and `RecipeErrors`. It owns no code.
- `LangCode::parse` returns `E-RCP-027`, because the code has its own row. `ParameterTag`, `ParameterKey`, `LongText`, `Ordinal`, and `Login` return a reason.

### D15. Texts

- `LongText` holds `description_en` and `change_note`: 1 to 4 000 characters, not empty after trim.
- A line end is LF or CRLF, because a browser form sends CRLF. A lone CR and every other control character give a reason (ADR-0028). The text keeps its input.
- A translation description is `Option<LongText>`, because it can be empty. A translation name is a `Text`.

### Signatures

```rust
pub enum VersionState { LiveDraft, Discarded, Approved, Retired }
impl VersionState { pub fn apply(self, step: Transition) -> Result<Self, ForbiddenTransition>; }
pub enum Optimistic<T> { Applied(T), Conflict { expected: i64, actual: i64 } }
impl Recipe {
    pub fn approval_replay(&self, number: u32, expected_row_version: i64) -> Option<ApprovalOutcome>;
    pub fn approve(&mut self, ready: ApprovalReady, expected_row_version: i64, approver: &Approver,
                   policy: &ApprovalPolicy) -> Result<Optimistic<ApprovalOutcome>, RecipeError>;
    pub fn retire(&mut self, number: u32, expected_row_version: i64)
        -> Result<Optimistic<RecipeVersionId>, RecipeError>;
}
impl ParameterValue {
    pub fn parse(json: &str, tree: &TypeTree) -> Result<Self, ValueFault>;
    pub fn check(&self, tree: &TypeTree, limits: &[ProcessLimit]) -> Result<(), ValueFault>;
}
impl Draft {
    pub fn load(stored: StoredDraft) -> Result<Self, RecipeError>;
    pub fn add_parameter(&mut self, new: NewParameter, trees: &TypeTrees) -> Result<ParameterChange, DraftError>;
    pub fn check_for_approval(&self, units: &LiveUnits, trees: &TypeTrees, allow_outdated: bool)
        -> Result<ApprovalReady, DraftError>;
}
impl LiveUnits { pub fn require(&self, path: &UnitPath) -> Result<LiveUnit, PlantError>; }
```

## Decisions of the user (2026-10-13)

- Q-A, error lists: the domain type `RecipeErrors`. ADR-0027 adds `#[from] RecipeErrors` to `AppError` for P2.3.
- Q-B, listings of `domain-model.md`: one ADR-0028 records every change of D3 to D6, D10, D12, and D15, and updates the listings.
- Q-C, name of the value check: `parse` and `check`. ADR-0028 changes the name in `reliability.md` section 6.
- Base of `CreateVersion`: any version, also a discarded draft. ADR-0028 records it.

## Risks / Trade-offs

- [`arbitrary_precision` applies to every crate of the workspace] → `reliability.md` section 4.4 requires the feature. P3.5 tests the JSON Schema crate with it. A domain test reads a number through `serde_json::Number::as_str`.
- [The structural tree of D13 keeps old array identifiers] → `apply_rebind` compares it with the trees of the new types and takes their identifiers.
- [A discarded draft shows the status `draft` in `E-RCP-002`] → `VersionStatus` has three values (`domain-model.md` section 4). The message stays true.
- [The literal reading of "one array level" in D9 rejects `.zones[*].a` with `.zones[2].b`] → This follows ADR-0007. A later ADR can narrow it.
- [`Draft::load` trusts the stored rows] → The database constraints of `data-model.md` sections 3.12 and 3.13 hold the shape. The snapshot check parses every row before an approval.

## Open Questions

- P2.3 decides if `UpdateParameterDefinition` with an unchanged type and a sent value gives `E-WEB-008`. `Draft::set_definition` accepts an optional retype and requires it when the type changes.
- P2.3 needs `content_sha256` to approve, because `recipe_version_status_fields` requires it (`data-model.md` section 3.10). The plan puts the module `export` in P3.5. The proposal of P2.3 resolves the order.
