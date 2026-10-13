# Tasks: domain-recipe

Each code task starts with a failing test. Run it, see it fail, and keep the output. Then write the code. After each code task, run the clippy, test, and fmt commands of `openspec/config.yaml`, section `operations.apply.guidance`. A `[mechanical]` task goes to a Haiku subagent. Sonnet does every other code task. `target/p22-brief.md` is the common brief of the subagents.

The ste-lint command is the last command of the section "Local checks" of `CONTRIBUTING.md`.

Each draft command task also tests that the `Draft` stays unchanged after each error of the command (design, Goals).

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0027 for the error lists and `DraftError` (design D14). Add `RecipeErrors` and `DraftError` to the row `recipe-domain` and `#[from] RecipeErrors` to the row `recipe-app` of `reliability.md` section 1.1. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.2 (docs/adr) Write ADR-0028 for D3 to D6, D8, D10, D12, D15, and the base of `CreateVersion`. Change `domain-model.md` sections 2.2, 3, 4, and 6, `reliability.md` sections 4.3 and 6, and the status lines of ADR-0008 and ADR-0016. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.3 (CHANGELOG.md) Add ADR-0027, ADR-0028, and the recipe domain types under `[Unreleased]`. Check with the ste-lint command.

## 2. Identifiers, texts, and error carriers

- [x] 2.1 (recipe-domain) [mechanical] Add `RecipeId`, `RecipeVersionId`, `ParameterId`, and `ParameterKey` (D1). First write failing tests for the conversion from `Uuid` and the `Display` form. `ParameterKey::parse` of a text that is not a UUID gives a reason. Check with `cargo test -p recipe-domain recipe::ids`.
- [x] 2.2 (recipe-domain) [mechanical] Add the module `identity` with `UserId` and `Login` (D2). First write failing tests from the row `Login` of `reliability.md` section 4.3. `Admin.User` folds to `admin.user`. 3 and 64 characters pass. 2 and 65 characters and `a b c` fail. Add a proptest for the round trip of `Login`. Check with `cargo test -p recipe-domain identity`.
- [x] 2.3 (recipe-domain) [mechanical] Add `LongText` (D15). First write failing tests from the row "Text (description, change note)" of `reliability.md` section 4.3. 4 000 characters, LF, and CRLF pass. 4 001 characters, a blank text, a lone CR, and a tab fail. Check with `cargo test -p recipe-domain text`.
- [x] 2.4 (recipe-domain) [mechanical] Add `ParameterTag` and `Ordinal`. First write failing tests. A tag of 64 characters passes. A tag of 65 characters, `speed`, and `1A` fail. The sort orders 0 and 100 000 pass, and 100 001 fails. Add a proptest for the round trip of the tag. Check with `cargo test -p recipe-domain recipe::tag`.
- [x] 2.5 (recipe-domain) Add `RecipeError` with its first variant `E-RCP-027`, `LangCode`, and `Translation` (D14, D15). First write failing tests from A17. `pl` and `de-AT` pass. `en`, `EN`, `de-at`, and `d` give `E-RCP-027`. Also test `code()`, `REGISTRY`, and `Display`. Check with `cargo test -p recipe-domain lang_code`.
- [x] 2.6 (recipe-domain) [mechanical] Add `RecipeErrors` and `DraftError` (D14). First write failing tests. `RecipeErrors` cannot be empty, and `code()` gives the code of the first entry. `DraftError::code` gives the code of its inner error. Check with `cargo test -p recipe-domain recipe::error`.

## 3. Version state machine and recipe root

- [x] 3.1 (recipe-domain) [mechanical] Add `VersionStatus`, `VersionState`, `Transition`, `ForbiddenTransition`, and `VersionState::apply` (D3). First write one table test with all 16 pairs of state and transition from `domain-model.md` section 4. Four pairs give their target, and twelve pairs are forbidden. Also test `VersionState::status` and the SQL names of `VersionStatus`. Check with `cargo test -p recipe-domain recipe::state`.
- [x] 3.2 (recipe-domain) Add `Optimistic`, `VersionHeader`, `Recipe`, `StoredRecipe`, `StoredRecipeReason`, `TryFrom<StoredRecipe>`, `Recipe::create`, `version`, `rename`, and the variant `E-RCP-012` (D4). First write failing tests for two approved headers, two live drafts, a duplicate number, and the number 0. Test `rename` of a deleted recipe. Check with `cargo test -p recipe-domain recipe::root`.
- [x] 3.3 (recipe-domain) Add `RecipeDeleter`, `Recipe::delete`, `restore`, and the variants `E-RCP-010`, `E-RCP-025`, and `E-RCP-026`. First write failing tests from the delete rules of ADR-0021, one test for each row and deleter. Test that `delete` discards the live draft. Check with `cargo test -p recipe-domain recipe::root::delete`.
- [x] 3.4 (recipe-domain) Add `Recipe::create_version` and the variants `E-RCP-011` and `E-RCP-021` (D4). First write failing tests. A live draft gives `E-RCP-011`. After a discarded draft 3, the next number is 4. After the number 2 147 483 647, the next number gives `E-RCP-021`, not a panic. Check with `cargo test -p recipe-domain recipe::root::create_version`.
- [x] 3.5 (recipe-domain) Add `Recipe::retire`, `discard`, and the variants `E-RCP-007` and `E-RCP-009` (D4). First write failing tests from A28 and the delta requirement "Allowed version transitions". An old row version gives `Optimistic::Conflict` before the transition check. Check with `cargo test -p recipe-domain recipe::root::retire`.

## 4. Read values of the Plant Model

- [x] 4.1 (recipe-domain) Add `TypeTree::platform` and `TypeTree::leaves` (D7). First write failing tests. A tree keeps the platform of `PlatformTypes`. `leaves` gives `.ramps[1]`, `.ramps[2]`, and `.ramps[3]` for the member `ramps: ARRAY[1..3] OF INT`, and the empty path for a primitive. It walks a tree of 16 384 leaves without recursion. Check with `cargo test -p recipe-domain type_tree::leaves`.
- [x] 4.2 (recipe-domain) Add the names of dropped rows to `PlatformTypes` and add `TypeTrees::resolve` (D7). First write failing tests. A missing row keeps its `PlantError`. A dropped row keeps its name. When `ST_Zone@3` exists, `TypeTrees` marks an array of `ST_Zone@2` and a struct with a member of type `ST_Zone@2` as outdated. Check with `cargo test -p recipe-domain plant::type_trees`.
- [x] 4.3 (recipe-domain) Add `LiveUnits`, `LiveUnit`, and `Machine::live_units` (D6). First write failing tests. `require` returns `E-PLT-020` for an unknown path and `E-PLT-009` for an archived unit. A `LiveUnit` knows its machine. `LiveUnits` reports an archived machine. Check with `cargo test -p recipe-domain plant::live_units`.

## 5. Parameter values

- [x] 5.1 (recipe-domain) [mechanical] Add `rust_decimal` and the range of each numeric type of `data-model.md` section 7. First write failing tests for both ends of each range and one value outside each end. Add the test of D8 that `Decimal::MAX` and `Decimal::MIN` fit the IEEE 754 ranges of `REAL` and `LREAL`. Check with `cargo test -p recipe-domain value::numeric`.
- [x] 5.2 (recipe-domain) Add `serde_json` with `arbitrary_precision`, `ParameterValue::parse` for struct, array, and numeric leaves, and `ValueFault` (D8). First write failing tests for the exact shape and the depth limit. Cover a missing member, an extra key, a wrong element count, `1.5e2`, `-0`, 29 significant digits, and `1.5` for `INT`. Test that the value records the root type of its tree. Check with `cargo test -p recipe-domain value::parse`.
- [x] 5.3 (recipe-domain) Add the string, char, bool, and time leaves to `ParameterValue::parse`. First write failing tests from the delta requirement "JSON form of a leaf value" and the baseline requirement "IEC literal of a time value". Cover `"Łódź"` for `STRING(10)`, `"a😀"` for `WSTRING(2)`, `1` for `BOOL`, and `T#90m` for `TIME`. Check with `cargo test -p recipe-domain value::leaf`.
- [x] 5.4 (recipe-domain) Add `StoredValue`, `TryFrom<(&StoredValue, &TypeTree)>`, and the canonical JSON output (D8). First write failing tests. The output of `parse` survives the stored form. A stored time leaf is a number. Use the JSONB example of `data-model.md` section 9.1. Check with `cargo test -p recipe-domain value::stored`.

## 6. Process limits

- [x] 6.1 (recipe-domain) [mechanical] Add `LimitInput`, `LimitList::parse`, and `LimitListReason` (D9). First write failing tests from the delta requirement "One limit for each leaf path". Cover two rows for `.setpoint`, `.ramps[*]` with `.ramps[2]`, and a row without a field. Check with `cargo test -p recipe-domain limit::list`.
- [x] 6.2 (recipe-domain) Add `ProcessLimit::parse`, `StoredLimit`, and `LimitFault` (D9). First write failing tests from the baseline requirement "Process limit on a matching leaf" and the class table of `domain-model.md` section 2.2. Cover `max_length` on `INT`, `min` on `BOOL`, `max` 40000 on `INT`, `min > max`, `max_length` 0, and a time bound `T#5s`. Check with `cargo test -p recipe-domain limit::parse`.
- [x] 6.3 (recipe-domain) Add `ParameterValue::check` (I7, D8, D9). First write failing tests. A `[*]` limit applies to each element. A string longer than `max_length` fails. Two limits for one leaf and a tree of another root type fail. The fault of a time value shows its canonical literal. A limit with only `max` shows the type minimum as `min`. Check with `cargo test -p recipe-domain value::check`.
- [x] 6.4 (recipe-domain) Add a proptest of `parse` and `check` with the strategy `type_trees` up to depth 6 (`reliability.md` section 6). Make the strategy visible to the module `recipe` in test builds only. Generate a valid value and random limits for each tree. A generated value passes. A value with one changed leaf type, one extra key, or one missing element fails. Check with `cargo test -p recipe-domain value::prop`.

## 7. Draft

- [x] 7.1 (recipe-domain) Add `Draft`, `Parameter`, `StoredDraft`, `ParameterChange`, `Draft::load`, `parameter`, and the variants `E-RCP-002`, `E-RCP-014`, and `E-RCP-051` (D6). First write failing tests. A discarded, approved, or retired version gives `E-RCP-002`. A duplicate tag gives `E-RCP-014`. An unknown key gives `E-RCP-051`. A parameter whose type does not resolve stays in the draft. Check with `cargo test -p recipe-domain draft::load`.
- [x] 7.2 (recipe-domain) Add `Draft::check_s7_layout` with the rules of D10 and the variants `E-RCP-003`, `E-RCP-022`, `E-RCP-023`, `E-RCP-053`, and `E-RCP-054`. First write failing tests from the baseline requirements of the S7 rules. Use "S7 address per platform", "S7 overlap and alignment", "S7 layout size of a struct and an array", and "S7 address width matches the type". Check with `cargo test -p recipe-domain draft::s7`.
- [x] 7.3 (recipe-domain) Add a proptest of the S7 rules (`reliability.md` section 6). Generate random parameter sets in one data block. An overlap and a misaligned start are always detected. A set without overlap and with aligned starts always passes. Check with `cargo test -p recipe-domain draft::s7::prop`.
- [x] 7.4 (recipe-domain) [mechanical] Add the budget function of D11 with the variant `E-RCP-021` of task 3.4. First write failing tests from the rows `budget.*` of `reliability.md` section 7.11. 4 000 parameters, 262 144 leaves, and 32 768 limit rows pass. One more gives `E-RCP-021`. Check with `cargo test -p recipe-domain draft::budget`.
- [x] 7.5 (recipe-domain) Add `NewParameter` and the first part of `Draft::add_parameter`: tag, S7 platform rule, S7 width, unit, and type tree (D6). First write failing tests. A duplicate tag gives `E-RCP-014`. A `LiveUnit` of another machine gives `E-PLT-020`. A `TypeTrees` of another machine gives `E-PLT-013`. Check with `cargo test -p recipe-domain draft::add::definition`.
- [x] 7.6 (recipe-domain) Add the second part of `Draft::add_parameter`: limits, value, budgets, alignment, and overlap. Add the variants `E-RCP-004`, `E-RCP-005`, `E-RCP-006`, `E-RCP-017`, `E-RCP-018`, and `E-RCP-019`. First write failing tests for the order of the checks of D6 with two errors at once. Check with `cargo test -p recipe-domain draft::add::content`.
- [x] 7.7 (recipe-domain) Add `Draft::set_definition`. First write failing tests from the baseline requirement "Type change needs a new value". A type change without a value gives `E-RCP-005`. Without a type change, the value and the limits stay. A parameter whose type did not resolve can get a new type. Check with `cargo test -p recipe-domain draft::definition`.
- [x] 7.8 (recipe-domain) Add `Draft::set_value` and `replace_limits`. First write failing tests from the baseline requirements "Value change keeps limits and translations" and "Replacement of limits keeps the value". Check with `cargo test -p recipe-domain draft::value`.
- [x] 7.9 (recipe-domain) Add `upsert_translation`, `remove_translation`, `remove_parameter`, and the variant `E-RCP-050`. First write failing tests from the baseline requirements "One translation per language" and "Delete a parameter with its children". Cover a translation name of 201 characters and a description of 4 001 characters. Check with `cargo test -p recipe-domain draft::translation`.

## 8. Snapshot and approval

- [x] 8.1 (recipe-domain) Add `VersionSnapshot`, `VersionSnapshot::check`, `SnapshotItem`, and the variants `E-RCP-013` and `E-RCP-020` (D12). First write failing tests from the baseline requirement "Snapshot check" and the delta requirement "Outdated parameter". Cover an archived unit, a type not on the platform with its name, and a stored value out of range. Also cover an archived machine, a deleted recipe, and the list of outdated tags. Check with `cargo test -p recipe-domain snapshot`.
- [x] 8.2 (recipe-domain) Add `ApprovalReady`, `Draft::check_for_approval`, `check_limits`, `check_outdated`, and the variants `E-RCP-008`, `E-RCP-016`, and `E-RCP-024` (D5, D12). First write failing tests for I17 with a `[*]` limit, one `E-RCP-016` entry for each leaf, A29 with and without `allow_outdated`, and an empty draft. Check with `cargo test -p recipe-domain draft::approval`.
- [x] 8.3 (recipe-domain) Add `ApprovalPolicy`, `Approver`, `ApprovalOutcome`, `Recipe::approve`, `approval_replay`, and the variant `E-RCP-015` (D5). First write failing tests for I1, I2, I16, A34, and the idempotent case of `api.md` section 4. Cover the creator, an editor, the policy off, a token with an old row version, and the order replay, row version, transition. Check with `cargo test -p recipe-domain recipe::root::approve`.

## 9. Rebind

- [x] 9.1 (recipe-domain) Add `Draft::plan_rebind` with the target check, `RebindTargetReason`, and the variant `E-RCP-052` (D13). First write failing tests from the baseline requirement "Rebind target is a newer revision". Cover an older revision, another struct name, and a source that only a struct member uses. Check with `cargo test -p recipe-domain rebind::target`.
- [x] 9.2 (recipe-domain) Add `TypeTree::rebind` (D13). First write failing tests. It replaces the struct of a parameter and of an array element chain, keeps the dimensions, and keeps a struct member of the source revision. A target above the leaf budget gives `E-PLT-018`. Check with `cargo test -p recipe-domain type_tree::rebind`.
- [x] 9.3 (recipe-domain) Add the value mapping and the variant `E-RCP-028` (ADR-0018). First write failing tests. A member with the same name and type keeps its subtree, also an array member. A new leaf takes its value from the map. A missing entry gives `E-RCP-028`, and a bad entry gives `E-RCP-005`. An array of the struct maps each element. Check with `cargo test -p recipe-domain rebind::value`.
- [x] 9.4 (recipe-domain) Add the limit mapping and the variant `E-RCP-029`. First write failing tests. A limit on a kept member stays, also `.ramps[*]`. A limit on a member that the target lacks or changes gives `E-RCP-029`. Two failing parameters give two entries in `RecipeErrors`. Check with `cargo test -p recipe-domain rebind::limit`.
- [x] 9.5 (recipe-domain) Add `Draft::apply_rebind`. First write failing tests. The draft changes only with a complete plan. A new tree that differs from the plan fails. A plan above a budget gives `E-RCP-021`. The plan lists each new array shape once. Check with `cargo test -p recipe-domain rebind::apply`.

## 10. Integration checks

- [x] 10.1 (recipe-domain) [mechanical] Add the registry test of D14. It compares the 34 `E-RCP` domain rows of `reliability.md` section 1.2 with the variants and `REGISTRY`. Add `E-RCP-001` with a test of `code()` and `Display`. Check with `cargo test -p recipe-domain registry_matches_reliability`.
- [x] 10.2 (openspec/changes/domain-recipe) Check the change with `openspec validate domain-recipe --strict`. Run `ste-lint.py` on the proposal, the design, the tasks, and the delta specs.

## Workflow follow-up

- Run every command of the section "Local checks" of `CONTRIBUTING.md`, including the coverage gate.
- Archive the change with the skill `openspec-archive-change` after the code review.
- P2.3 maps `RecipeErrors`, `DraftError`, and `Optimistic::Conflict`, wires `EnsureDerivedType` to the rebind plan, and resolves the open questions of `design.md`.
