# ADR-0169: A new version row is a draft

Status: Accepted
Date: 2026-11-14
Deciders: project owner

## Context

Migration `0003_grants.sql` lines 34 to 37 give `recipe_app` a table-level `INSERT` grant on `recipe_version`.
The trigger `recipe_version_b_transition` of `data-model.md` §4.7 fires only `BEFORE UPDATE` (`0002_triggers.sql` lines 219 to 221).
No trigger fires on an insert into `recipe_version`.

Thus a defect or a compromised `recipe_app` session can insert a version with `status = 'approved'`, a forged `content_sha256`, and any `row_version`.
The `CHECK recipe_version_status_fields` accepts such a row when its approval columns and its hash agree.

I4 of `domain-model.md` §3 covers a change of a version, not an insert. No normative rule is broken.
The adapter in `crates/recipe-db/src/transaction/version_writes.rs` line 25 names no `status`, `row_version`, or `deleted_at`. The path therefore needs a defect.

## Decision drivers

- Defense in depth for I4. The database refuses a version that skips the state machine of `domain-model.md` §4 (R17, R29).
- The rule holds for every role, not only for `recipe_app`.
- A refused insert gives a coded error, like every other trigger of `data-model.md` §4.11.
- The migration does not scan or rewrite a table (`reliability.md` §5).

## Considered options

### Option A: A column-level `INSERT` grant without `status`

- Pros: no new function and no new trigger.
- Cons: the grant binds only `recipe_app`. The column default then gives the status, but `row_version` and `deleted_at` stay open. A refused statement gives a privilege error with no code.

### Option B: A `BEFORE INSERT` trigger on `recipe_version`

- Pros: the rule holds for every role. The refusal has a code. The trigger has the shape of trigger 4.7.
- Cons: one more function and one more trigger. The tests that insert a row that is not a draft need a change.

## Decision

The decision selects Option B.

Trigger:

- (a) Migration `0016_recipe_version_insert.sql` adds the function `recipe_version_insert_draft()` and a `BEFORE INSERT` trigger on `recipe_version`.
- (b) The trigger has the name of its function, `recipe_version_insert_draft`, like `security_event_guard` in `0005_security_event_guard.sql`.
- (c) The trigger refuses a row with a status other than `draft`, a `row_version` other than 1, or a `deleted_at` that is not null.
- (d) Like trigger 4.7, the function raises an exception whose message starts with the new code `E-RCP-037`. The code has the status 409.
- (e) The `CHECK recipe_version_status_fields` then covers the approval columns and the hash of a new draft.
- (f) The migration is forward-only. It adds one function and one trigger. It does not scan or rewrite a table.

Documents:

- (g) `reliability.md` §1.2 gets the row of `E-RCP-037` before the code gets the variant.
- (h) `data-model.md` §4 gets the new section 4.14 and a row of §4.11. §8 lists migration 0016.
- (i) `data-model.md` §5.2 gets one sentence. The trigger limits the `INSERT` grant of `recipe_app` on `recipe_version` to a new draft.
- (j) `domain-model.md` §3 extends I4 to an insert.

Grants and the restore test:

- (k) The function keeps the default `EXECUTE` grant of `PUBLIC`, like `recipe_version_transition()`. `crates/recipe-db/tests/grants.expected` gets the line `PUBLIC:recipe_version_insert_draft()|EXECUTE`.
- (l) `deploy/pgtools/restore-code.expected` gets two lines in the form of ADR-0076 (a). Each line holds the kind, the name, the owner, and the hash.
- (m) The first line is `FUNCTION|public recipe_version_insert_draft()|recipe_migrator|<hash>`. The hash covers the definition of the function.
- (n) The second line is `TRIGGER|public recipe_version recipe_version_insert_draft|recipe_migrator|<hash>`. `restore-code.sql` collects it, because the trigger has `NOT tgisinternal`.
- (o) The owner of the trigger is the owner of its table, as ADR-0076 (a) states. The hash covers `pg_get_triggerdef` and `tgenabled` (ADR-0078).
- (p) Both names join the name list of step 2 (ADR-0076 (b)). Step 4 compares the `FUNCTION` line, and the step after the restore of `post-data` compares the `TRIGGER` line (ADR-0076 (c)).

Tests:

- (q) The trigger fires before each `CHECK`. These eight tests of `crates/recipe-db/tests/storage/constraints/recipe.rs` then get `E-RCP-037` and need a change.
  - `recipe_version_approved_by_fkey_rejects_an_unknown_approver`
  - `recipe_version_status_value_rejects_an_unknown_status`
  - `an_unknown_status_is_reported_by_status_fields_when_both_checks_exist`
  - `recipe_version_row_version_positive_rejects_a_row_version_of_zero`
  - `recipe_version_hash_format_rejects_a_hash_that_is_not_lowercase_hex`
  - `recipe_version_status_fields_rejects_an_approved_version_without_a_hash`
  - `recipe_version_status_fields_rejects_a_retired_version_without_a_retirement_time`
  - `recipe_version_deleted_only_draft_rejects_a_discarded_approved_version`
- (r) Such a test inserts a draft and moves it through the allowed transitions of trigger 4.7.
- (s) A test that must break a `CHECK` on insert disables the new trigger only inside its own transaction. That transaction rolls back.
- (t) The trigger gets a positive test and a negative test. The negative test asserts the SQLSTATE and the code `E-RCP-037` (`reliability.md` §6).

Restore and replay:

- (u) `pg_dump` writes each trigger into the section `post-data` (ADR-0076 (d)). A restore of a dump loads the rows before it creates the trigger, so the trigger does not fire.
- (v) WAL replay fires no trigger.

## Consequences

### Positive

The database refuses a new version that is not a draft, for every role.
A refused insert gives the code `E-RCP-037` and not a privilege error.
The restore test finds a missing or changed function or trigger of this rule.

### Negative

Eight constraint tests need a change.
A test that breaks a `CHECK` on insert must disable the trigger in a transaction that rolls back.
The restore list gets two more lines with their hashes.

### Risks and mitigations

Only the owner of `recipe_version` can disable the trigger. The owner is `recipe_migrator`, as the `TRIGGER` lines of `restore-code.expected` show.
The restore test detects a disabled trigger after a restore, because the hash covers `tgenabled` (ADR-0078).

## Related

- ADR-0015, lock order and one transaction per command.
- ADR-0076, name list of the restore test. ADR-0078, trigger state of the restore test.
- `data-model.md` §4.7, §4.11, §5.2. `domain-model.md` §3 (I4), §4.
- `reliability.md` §1.2, §5, §6.
- `overview.md` §7: R17, R29.
