# ADR-0086: Bounds and anchor of the audit chain check

Status: Accepted. Amended by a note: item (h) changes ADR-0073 for the anchor of an empty chain table. ADR-0073 has a matching note. Amended by ADR-0100 (the audit chain check after a restore: wait for the next run of the job)
Date: 2026-10-24
Deciders: project owner

## Context

Task 4.4 wrote the reads of the nightly audit chain check of `data-model.md` section 4.9. The review of these reads found four gaps in section 4.9. No two documents disagree.

1. The check reads only the rows inside `(0, last_seq]`. A superuser can disable the triggers and insert a row with a `chain_seq` of 0 or below, or above the head. No batch reads that row, so the check never finds it.
2. `AuditError::ChainBroken { chain_seq }` needs a value. Section 4.9 gives no value for a mismatch between the end of the check and the head.
3. After a break, the job records the last `chain_seq` before the break in `audit.last_check_seq`. The stored value can thus decrease.
4. A superuser can delete the last rows and set the head back to the last remaining row. The remaining rows form a sound chain, and the check finds no break.

Two more facts apply:

- A clean batch does not prove that the rows up to the head exist. A deleted last row gives a clean batch. Section 4.9 gives the end check to the job, but the port does not say so.
- `audit_log.chain_seq` has no CHECK constraint on its sign. The setting `session_replication_role = replica` disables triggers, but it does not disable CHECK constraints.

## Decision drivers

- Every change of the audit log is tamper-evident (R20, ADR-0022).
- A break gives `E-AUD-002`, a `security_event` row, and the alert (ADR-0073 (a)). A break never shows only as a storage fault (`E-DB-008`).
- A head shape that the constraints of `audit_chain_head` permit is never a storage fault. Only a superuser or `audit_owner` can write a head that the trigger did not write.
- No migration, no new error code, and no new configuration key.
- `audit.check_statement_timeout` bounds each statement of the check (`reliability.md` section 7.11).
- The port of task 3.1 gets the smallest change that carries the new values.

## Considered options for gap 1

### Option A: A read of the minimum and the maximum in the statement of step 1

- Pros: no migration. The unique index `audit_log_chain_seq_unique` gives both values with two short index reads. The same read also finds a row above the head.
- Cons: the check finds such a row only at its next run.

### Option B: A forward migration that adds `CHECK (chain_seq >= 1)`

- Pros: the database refuses a row with a `chain_seq` of 0 or below at once.
- Cons: a new migration on the largest table, with `NOT VALID` and a separate validation. The CHECK does not find a row above the head. A superuser can drop the CHECK.

The user chose **Option A** on 2026-10-24.

## Decision

- (a) Step 1 of the check reads three values in one statement:
  - `last_seq` and `last_hash` of `audit_chain_head`, the head of this run.
  - `min(chain_seq)` and `max(chain_seq)` of `audit_log`, the bounds. An empty table gives NULL for both.
  - The text of `audit.last_check_seq` in `app_setting`, the anchor. A missing row gives NULL.
  - The statement runs in its own short `READ ONLY` transaction with `SET LOCAL statement_timeout` = `audit.check_statement_timeout`.
  - `data-model.md` section 4.9 gives the SQL.
- (b) A missing anchor row gives the anchor 0. An anchor text in a form other than the form of ADR-0073 (i) is a `StorageError` (`E-DB-008`).
- (c) Step 1 applies these rules in this order. `H` is `last_seq` and `A` is the anchor. The first rule that applies is the break, and it names the `chain_seq` of `E-AUD-002`:
  1. The minimum is below 1. The break names the minimum.
  2. The maximum is above `H`. The break names the maximum.
  3. `H` is below `A`. The break names `H` + 1.
  4. `H` is above 0, and the table has no row or a minimum above 1. The break names 1.
  5. The maximum is below `H`. The break names the maximum + 1.
  - An empty table with `H` = 0 has no break.
  - After a break in step 1, the job reads no batch.
- (d) A clean batch does not prove that the rows up to the head exist. After the last batch, the job compares its end with the head of step 1:
  - The end is below `H`. The break names the last `chain_seq` of the check + 1. With no checked row, that value is 1.
  - The end equals `H`, but the hash differs from `last_hash`. The break names `H`.
  - A head with `last_seq` above 0 and no `last_hash` is a break of the chain, not a storage fault. Every row has an `entry_hash`, so the end check finds another hash at `H` and names `H`.
  - A break inside a batch names `first_break` of that batch, as before.
- (e) The anchor marks the end of a clean prefix: the last `chain_seq` up to which a completed check read the chain without a break. The job records `audit.last_check_seq` after each completed check with these values:
  - After a check without a break: `H`.
  - After a break inside a batch: the larger value of `A` and `last_clean` of that batch.
  - `last_clean` is the `chain_seq` of the row before the first break in the batch. When the first row of the batch breaks, `last_clean` is the `chain_seq` of the `after` position of the batch. The batch statement returns `last_clean` next to `first_break`.
  - After a break at the end check with an end below `H`: the larger value of `A` and the end. With no checked row, the end is 0.
  - After a break at the end check with an end equal to `H`: the larger value of `A` and `H` - 1.
  - After a break in step 1: `A`.
  - Example: the rows 1, 2, and 100 with the head 100 and the anchor 0. The batch names 100 and returns `last_clean` 2. The job records 2, not 99.
  - The value thus never decreases.
  - The job writes `audit.last_check_seq` before `audit.last_check_at`. A completed check is thus never announced before its anchor persists.
  - The value is an anchor, not a start position. Each check still starts at the first row (ADR-0073 (a)), so no check skips a broken row.
  - A head that a superuser set back below the anchor gives a break at each nightly check. The alert of ADR-0073 (a) thus stays active.
- (f) The anchor rule agrees with a restore and with the first run:
  - The daily dump holds `audit_log`, `audit_chain_head`, and `app_setting` from one snapshot (`reliability.md` section 7.7). A recovery to a point in time also gives one consistent state.
  - A check never raises the anchor above the head that it read. A restored anchor is above the restored head only when the source chain had the same break. The restored chain then gives the same break at its first check.
  - On the first run the anchor is 0, so rule 3 never applies.
  - The restore test and `pitr-check` read the whole table without a range. Their gap condition finds a row at 0 or below, and their head comparison finds a row above the head. They need no change.
- (g) The code of the rules:
  - `AuditChainReads::head()` returns a new port type `ChainHead`. Its fields are `position` (a `ChainPosition`), `bounds` (an `Option<ChainBounds>` with `min` and `max`), and `anchor` (a `ChainSeq`).
  - The adapter returns the bounds as it reads them, also a value of 0 or below. It returns the head as it reads it, also a head with `last_seq` above 0 and no hash.
  - The adapter refuses only a head that the constraints of `audit_chain_head` do not permit: a negative `last_seq` or a hash that is not 32 bytes long. It also refuses a bad anchor text and a pair of bounds with one NULL value, which one statement cannot give.
  - `BatchOutcome::Broken` carries `first_break` and `last_clean`. `data-model.md` section 4.9 gives the output column `last_clean` of the batch statement.
  - `ports/state_key.rs` gets `parse_chain_seq_text`, the reader of the `chain_seq` text of `StateValue::to_text`.
  - The doc of `check_batch` states the first sentence of item (d).
  - The job `audit_chain_check` of task 3.6 applies items (c), (d), and (e). No adapter applies these rules.
  - The memory adapter computes the bounds from its rows and reads the anchor from its state keys. A new method `plant_row` adds a row with a given `chain_seq` and does not move the head.
  - The memory adapter gives the results of the SQL adapter. It reads the rows in `chain_seq` order, refuses a second row with a taken `chain_seq`, and applies the gap rule of the batch statement. It permits a head with `last_seq` above 0 and no hash.
- (h) This ADR amends the audit chain check of ADR-0022. It does not change ADR-0073. Item (a) of ADR-0073 stays: each check starts at the first row and stops at the first break.

## Consequences

### Positive

- The check finds a row outside `(0, last_seq]` within one night.
- The check finds a head below the highest `chain_seq` that a completed check read without a break.
- Each break names one `chain_seq` by a fixed rule. The tests and the operator see the same value.
- No migration, no new error code, and no new configuration key.

### Negative

- A superuser can delete only rows above the anchor and set the head back to a value at or above the anchor. The check does not find this change. The anchor protects only the rows up to the last completed check.
- A superuser who also lowers `audit.last_check_seq` hides a rewound head.
- A full rewrite of the chain with recomputed hashes and a new head stays undetectable without an off-host copy. Off-host shipping is a site option (ADR-0022).
- The port of task 3.1 and the memory adapter of task 3.10 change after their close.
- The statement of step 1 also reads `app_setting`.

### Risks and mitigations

- Risk: a bloated index makes the read of the minimum or the maximum slow. Mitigation: `audit.check_statement_timeout` bounds the statement. The job returns `E-DB-002`, and the supervisor logs it once. The check is then not completed, so the condition of `audit.last_check_at` raises the alert after `audit.max_check_age`.
- Risk: a restore from another source brings back `audit_log` without `app_setting`, and the old anchor gives a false break. Mitigation: the daily dump holds both tables from one snapshot. The operator runs the audit chain check after each restore (`reliability.md` section 7.7).

## Related

- Amends ADR-0022 (the audit chain check: bounds, anchor, and the values of a break).
- ADR-0060 (ports of the jobs), ADR-0073 (a) and (i) (alert window, text form of the state keys)
- `docs/architecture/data-model.md` sections 3.17 and 4.9
- `docs/architecture/reliability.md` sections 1.2 (`E-AUD-002`) and 7.11 (`audit.check_statement_timeout`, `audit.last_check_seq`)
- `crates/recipe-app/src/ports/audit_chain.rs`, `crates/recipe-app/src/ports/state_key.rs`
- Baseline requirements: audit "Nightly audit chain check" and "Chain break"
- `overview.md` section 7: R20
