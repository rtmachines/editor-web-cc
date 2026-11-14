# ADR-0168: Text of the data model follows the schema and the code

Status: Accepted. Amends `data-model.md` §1.1, §1.3, and §6
Date: 2026-11-14
Deciders: project owner

## Context

The review of 2026-11-13 found three places where `data-model.md` does not state the shipped code or the shipped DDL. Each finding has the severity LOW.

The first place is §6. `CreateUser`, `ResetPassword`, and `CreateRecipe` read in a read-only transaction before the command transaction. The rows of these commands in §6 do not name that read. The row `SetMachineDataBlock` names the same pattern with the words "Before the transaction".

The second place is the exception table of §1.1. `plc_type_member` has the primary key `(struct_type_id, ordinal)` and no `id`. `draft_editor` has the primary key `(recipe_version_id, user_id)`, no `id`, and `first_edit_at` instead of `created_at`. The table has no row for these two tables.

The third place is §1.3. The text says that `login_attempt.login` accepts 1 to 64 characters with no control character. The `CHECK` of §3.7 tests only the length. Only the value object refuses a control character.

`CLAUDE.md` ("Source of truth and change rule") lets a change of `docs/architecture/` go only with an ADR. ADR-0126 (h) recorded a correction of the text to the code in the same way.

## Decision drivers

- The normative text must state the behavior of the shipped code and the shipped DDL (R34).
- A command runs in one transaction, and validation runs before it (R29, `reliability.md` §7.4).
- The text must say which layer checks an external input (R38).
- A correction of the text needs no migration and no code change when the code is correct.

## Considered options

### Option A: Change the text to the code and the DDL

- Pros: no migration and no code change. The text then states what the system does.
- Cons: the reader must know that the read before the transaction is not part of the command transaction.

### Option B: Change the code and the DDL to the text

- Pros: the text stays.
- Cons: one transaction for the three commands puts an Argon2 hash or a field error inside the transaction. A new `CHECK` on `login_attempt.login` repeats the check of the value object. New `id` columns change two tables that need no `id`.

## Decision

The decision selects Option A. One ADR holds the three corrections, because they share one decision. No migration and no code change follow.

- (a) F-15: the rows `CreateUser`, `ResetPassword`, and `CreateRecipe` of `data-model.md` §6 name the read before the transaction. They use the form of the row `SetMachineDataBlock`.
- (b) `CreateUser` reads the login in a read-only transaction. A known login gives `DuplicateLogin` (`E-IDN-001`) before the hash. The unique constraint `app_user_login_unique` still finds the race. The code is in `crates/recipe-app/src/identity/create_user.rs`, lines 84 and 98.
- (c) `ResetPassword` reads the login in a read-only transaction. An unknown login gives `NotFound` (`E-WEB-005`) before the hash. The code is in `crates/recipe-app/src/identity/reset_password.rs`, lines 65 and 82.
- (d) `CreateRecipe` resolves the machine of `machine_code` in a read-only transaction. A missing machine gives `MachineNotFound` (`E-PLT-019`). The command transaction locks the machine `FOR SHARE` and requires it live under that lock. The code is in `crates/recipe-app/src/recipe/create_recipe.rs`, lines 57 and 96.
- (e) Each read of items (b) to (d) holds no lock into the command transaction. The command transaction or a constraint checks the same fact again.
- (f) F-17: the exception table of `data-model.md` §1.1 gets a row for `plc_type_member`. The row says: composite primary key `(struct_type_id, ordinal)`, no `id`. The DDL is in `migrations/0001_schema.sql`, lines 176 to 188.
- (g) F-17: the exception table also gets a row for `draft_editor`. The row says: composite primary key `(recipe_version_id, user_id)`, no `id`, and `first_edit_at` instead of `created_at`. The DDL is in `migrations/0001_schema.sql`, lines 378 to 383.
- (h) Neither table gets `updated_at`. `recipe_app` has no `UPDATE` grant on either table (`crates/recipe-db/tests/grants.expected`).
- (i) F-18: `data-model.md` §1.3 says that the value object `TypedLogin` refuses a control character. The database checks only the length of `login_attempt.login`.
- (j) The value object is in `crates/recipe-domain/src/identity/throttle.rs`, lines 61 to 70. The `CHECK` is `login_attempt_login_length` in `migrations/0001_schema.sql`, line 247.

## Consequences

### Positive

- `data-model.md` states the shipped transactions, keys, and checks.
- No migration and no code change are necessary.

### Negative

- §6 has three more rows with a read before the transaction. A reader must keep that read apart from the command transaction.

### Risks and mitigations

- Risk: a later change moves a check out of the value object `TypedLogin`. Then no layer refuses a control character in `login_attempt.login`. Mitigation: the unit tests of `TypedLogin` test the control character.

## Related

- ADR-0035 (the typed login and the throttle), ADR-0126 (h) (a correction of the text to the code)
- `data-model.md` §1.1, §1.3, §3.4, §3.7, §3.11, §6
- `reliability.md` §4.3, §7.4
- Change `review-design-fixes`, findings F-15, F-17, and F-18 of the review of 2026-11-13
- `overview.md` §7: R29, R34, R38
