# ADR-0191: Error sets and statement order of the operation rows follow the code

Status: Accepted. Amends `domain-model.md` §4, §5.3, §5.4, and `data-model.md` §6
Date: 2026-11-17
Deciders: project owner

## Context

The semantic review of 2026-11-17 found two kinds of gaps in the operation rows. The review record lists them as DC-2 and RCA-01 (`docs/plan/review/2026-11-17-semantic-review.md`). Plan review R3 moved RCB-01 to this ADR. RCB-01 is a third gap, in the rules of `domain-model.md` §4.

The first kind is an error column that misses a code. `domain-model.md` §5 says: "The error column gives the complete error set of each command." `overview.md` R35 says: "Each command lists its complete error set."

- §5.3 gives no `E-WEB-005 NotFound` for `SetUserRole`, `DeactivateUser`, `ActivateUser`, `ResetPassword`, and `UnlockLogin`. `api.md` §2.8 gives `404 E-WEB-005` for the five routes. The code returns `AppError::NotFound` for an unknown login (`admin_change.rs` line 64, `activate_user.rs` line 62, `reset_password.rs` line 112, `unlock_login.rs` line 125, all in `crates/recipe-app/src/identity/`).
- The §5.4 row `ExportVersion` gives no `E-WEB-011 TaskFailed`. The row `ExportVersion` of `data-model.md` §6 names it for the rebuild in `spawn_blocking` (ADR-0029).
- The guard column of `data-model.md` §6 misses codes that the rows of `domain-model.md` §5.2 list:
  - `AddParameter` and `UpdateParameterDefinition`: `E-RCP-055 PrecisionExceeded` (ADR-0117), `E-RCP-056 S7AddressPastBlockEnd` (ADR-0116 (a)), `E-RCP-057 S7DataBlockOutOfRange` (ADR-0116 (c), ADR-0140).
  - `SetParameterValue` and `ReplaceLimits`: `E-RCP-022 S7AddressOverlap` (ADR-0149) and `E-RCP-055 PrecisionExceeded`.
  - `RebindTypeRevision`: `E-RCP-055`, `E-RCP-056`, and `E-RCP-057`.

The second kind is the position of the touch statement. The rows `AddParameter` and `UpdateParameterDefinition` of `data-model.md` §6 put the touch right after the `row_version` compare. The unit lock and the class-2 locks come after it. The row `RebindTypeRevision` puts the class-2 lock after the touch.

The code runs these steps in another order (`add_parameter.rs` line 128, `update_definition.rs` line 131, `rebind.rs` line 91, in `crates/recipe-app/src/recipe/`):

1. The version lock and the `row_version` compare.
2. The unit lock, the class-2 locks, and `EnsureDerivedType`.
3. The domain method of the command.
4. The touch and `INSERT draft_editor`.
5. The first parameter write.

The touch updates a row that the command already holds `FOR UPDATE`. It takes no new lock, so the order of the lock waits is the same in both texts.

In step 2, `EnsureDerivedType` can insert a new derived type before the touch. ADR-0137 (c) permits this insert for the three commands. The other five draft commands write nothing before the touch. In all eight draft commands, the touch runs after the domain method and before the first write of a `parameter`, `parameter_limit`, or `parameter_translation` row.

The third gap is the order of the approval checks. The rule "Idempotent approve" of `domain-model.md` §4 gives one order for all checks of `ApproveVersion`: the idempotent case, then the row version, then the transition. The accepted requirement "Order of the approval errors" of the `web-ui` spec states two steps:

1. The check of the draft before the transaction gives its own code, also with a stale `row_version`.
2. Under the locks, the `row_version` compare comes before the second check.

The code (`crates/recipe-app/src/recipe/approve_version/prepare.rs`) and the row `ApproveVersion` of `data-model.md` §6 follow the requirement. The owner decided on 2026-11-17 that the requirement and the code state the intended order. The sentence of §4 is too general.

Under the locks, the order of execution and the precedence of the answer differ. `approve_in_transaction` (`approve_version/write.rs`) runs the second check before `Recipe::approve()` compares the `row_version`. It returns the domain fault of the second check (`E-RCP-016`, `E-RCP-024`) only after `Recipe::approve()` gives `Approved`. A read fault of the type rows can return before the compare. The word "before" in the requirement therefore names the precedence of the answer, not the order of execution. This ADR keeps that wording as a known note and changes no spec and no code.

`Recipe::approve()` (`recipe-domain/src/recipe/root/approve.rs`) checks the deleted recipe (`E-RCP-012`) first, as the rules of `data-model.md` §6 require. Then it checks the idempotent case, the row version, and the transition.

## Decision drivers

- The normative text states the behavior of the shipped code when the code meets the requirements (R34).
- Each command lists its complete error set (R35).
- A command takes its locks in one fixed order (R29, ADR-0015).

## Considered options

### Option A: Change the text to the code

- Pros: no code change. The rows then state what the system does.
- Cons: the three rows get a longer lock column.

### Option B: Change the code to the text

- Pros: the rows stay.
- Cons: the touch then writes before the domain method of every failed command. The rollback undoes that write, so the change adds work and no safety. The error sets cannot change, because the codes come from the domain rules. For the approval order, a code change contradicts the accepted `web-ui` requirement and three `recipe-web` tests.

## Decision

The project uses **Option A**. One ADR holds the three gaps, because they share one decision: the operation rows and the rules of §4 follow the shipped code. No code and no migration change.

- (a) The rows `SetUserRole`, `DeactivateUser`, `ActivateUser`, `ResetPassword`, and `UnlockLogin` of `domain-model.md` §5.3 add `E-WEB-005 NotFound` (unknown login).
- (b) The row `ExportVersion` of `domain-model.md` §5.4 adds `E-WEB-011 TaskFailed` (export).
- (c) The guard column of the row `AddParameter` of `data-model.md` §6 adds `PrecisionExceeded` (`E-RCP-055`), `S7AddressPastBlockEnd` (`E-RCP-056`), and `S7DataBlockOutOfRange` (`E-RCP-057`). The row `UpdateParameterDefinition` keeps "Same as `AddParameter`".
- (d) The guard column of the rows `SetParameterValue` and `ReplaceLimits` adds `S7AddressOverlap` (`E-RCP-022`, ADR-0149) and `PrecisionExceeded` (`E-RCP-055`).
- (e) The guard column of the row `RebindTypeRevision` adds `PrecisionExceeded`, `S7AddressPastBlockEnd`, and `S7DataBlockOutOfRange`.
- (f) The rows `AddParameter` and `UpdateParameterDefinition` of `data-model.md` §6 move the touch. The lock column drops the touch after the `row_version` compare. The statement column moves `INSERT draft_editor` from its first place. The touch and `INSERT draft_editor` then come after `EnsureDerivedType` and the domain check, before `INSERT parameter` or `UPDATE parameter`.
- (g) The row `RebindTypeRevision` moves the touch in the same way. The lock column names its locks and drops the touch. The statement column puts `EnsureDerivedType` and the domain mapping first. The touch and `INSERT draft_editor` follow them, before the first `UPDATE parameter`.
- (h) The rule "Every draft command then runs ..." of `data-model.md` §6 gets two new sentences at its end. The new text is: "The touch and `INSERT draft_editor` run after the domain check and before the first write of a `parameter`, `parameter_limit`, or `parameter_translation` row. Only `EnsureDerivedType` can write before the touch: `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` can insert a new derived type first (ADR-0137 (c))."
- (i) The rule "Idempotent approve" of `domain-model.md` §4 follows the requirement "Order of the approval errors" of the `web-ui` spec and the row `ApproveVersion` of `data-model.md` §6. Its last sentence changes:
  - Old: "The order of the checks is the idempotent case, then the row version, then the transition (`api.md` section 4)."
  - New: "The check of the draft before the transaction gives its own code, also with a stale `row_version`. Under the locks, `Recipe::approve()` checks the deleted recipe (`E-RCP-012`), then the idempotent case, then the row version, then the transition (`api.md` section 4)."

## Consequences

### Positive

- The error column of each row gives the complete error set (R35).
- `domain-model.md` §4, the `web-ui` spec, and `data-model.md` §6 give one order of the approval checks.
- A reviewer who compares the statement order with `data-model.md` §6 finds the order of the code.

### Negative

- The three rows of items (f) and (g) differ from the other draft rows in the position of the touch.

### Risks and mitigations

- Risk: a later change adds a write of a parameter row before the touch. Then item (h) no longer states the code. Mitigation: the atomic tests of `crates/recipe-db/tests/storage/atomic/draft.rs` assert no partial write and no touch for a failed command.

## Related

- ADR-0015, ADR-0029, ADR-0116, ADR-0117, ADR-0137 (c) (the derived type before the touch), ADR-0140, ADR-0149, ADR-0168 (a correction of the text to the code)
- `domain-model.md` §4 and §5.2 to §5.4, `data-model.md` §6, `api.md` §2.8 and §4, `web-ui` spec requirement "Order of the approval errors"
- Change `semantic-review-fixes`, doc conflict DC-2 and findings RCA-01 and RCB-01 of the semantic review of 2026-11-17
- `overview.md` §7: R27, R29, R34, R35
