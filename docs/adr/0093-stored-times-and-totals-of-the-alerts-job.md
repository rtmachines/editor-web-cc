# ADR-0093: Stored times and counter totals of the job `alerts`

Status: Accepted
Date: 2026-10-25
Deciders: project owner

## Context

Step P4.1 builds the job `alerts` (`crates/recipe-app/src/jobs/alerts.rs`). The review of that code found two gaps in the documents:

1. The job compares stored times with `database_now` of its single snapshot (ADR-0060 (e)). No document says what the job does with a stored time that is later than `database_now`.
   - The stored times are the time of the newest complete dump, `audit.last_check_at`, the `occurred_at` of the newest `audit_chain_broken` row, and the first start of ADR-0060 (x).
   - `reliability.md` section 3.7 defines `backup_age` and `audit_check_age` with "older than". A time in the future is not older, so the literal rule makes the condition inactive.
   - The code makes the condition active when the stored time is later than `database_now` by more than `alerts.interval`.
2. Two documents disagree on the condition `export_hash_mismatch`.
   - `reliability.md` section 3.7 says "One or more `E-EXP-002` in the last `alerts.interval`".
   - ADR-0073 (g) says that the condition is active when the export total is above the total of the last run. The code follows ADR-0073 (g).

Facts of the clock:

- All containers use the host clock (`reliability.md` section 7.9). A step of the host clock moves `database_now` and the clock of each writer together.
- A clock that is ahead can write a stored time in the future. After the clock comes back, that time stays later than `database_now`.
- ADR-0088 accepts a delay of the due times of `cleanup` after a backward step. That job removes rows. It does not report a fault.

## Decision drivers

- An alert stays active while the fault exists (R37). A clock fault never hides a missing dump or a missing audit check.
- The project gives data safety priority over performance (R31). An extra alert is the safe direction.
- Each rule lives in one document. Two documents never disagree on one condition.
- No new error code, no new number in section 7.11, no new configuration key, and no new alert condition.

## Considered options for a stored time in the future

### Option A: A time ahead by more than `alerts.interval` is untrusted, and its condition stays active

- Pros: a stored time in the future cannot silence `backup_age` or `audit_check_age`. A skew of at most one `alerts.interval` gives no false alert.
- Cons: after a backward step of more than `alerts.interval`, a recent dump or check can give a false alert. `alerts.interval` gets a second meaning.

### Option B: The literal rule "older than" of section 3.7

- Pros: the text of section 3.7 stays as it is. No setting gets a second meaning.
- Cons: a future time gives the age 0 for any distance. The condition stays inactive until the clock passes the stored time and the maximum age. During that time a missing dump gives no alert.

The user chose **Option A** in session 9 of step P4.1.

## Considered options for `export_hash_mismatch`

### Option A: The total since the previous run (ADR-0073 (g))

- Pros: the job counts each `E-EXP-002` once. A late run or a run that stops early loses no result.
- Cons: the text of section 3.7 changes.

### Option B: The time window of section 3.7

- Pros: the text of section 3.7 stays as it is.
- Cons: the in-process counter holds no time of each result, so the job cannot count a window. A run that starts late or stops early can lose a result.

The user chose **Option A** in session 9 of step P4.1.

## Decision

- (a) The job `alerts` takes its time of the run from `database_now` of its single snapshot. For each stored time it computes an age in this way:
  - A stored time at or before `database_now` has the age `database_now` minus the stored time.
  - A stored time later than `database_now` by at most `alerts.interval` has the age 0. The border is inclusive, so the job trusts a time exactly `alerts.interval` ahead.
  - A stored time later than `database_now` by more than `alerts.interval` is untrusted. Its condition stays active.
  - An untrusted dump time makes `backup_age` active. An untrusted `audit.last_check_at` makes `audit_check_age` active.
  - An untrusted `occurred_at` of the newest `audit_chain_broken` row makes `audit_chain_broken` active.
  - An untrusted first start gives no start grace (ADR-0060 (y)).
  - The tests hold the border with the default of 60 s. The job trusts a time 60 s ahead and does not trust a time 61 s ahead.
  - `alerts.interval` keeps its value of section 7.11. It is also the largest skew that the job trusts.
- (b) The condition `export_hash_mismatch` is active when the total of the `E-EXP-002` counter is higher than the total of the previous run (ADR-0073 (g)).
  - The first run after the start of the process compares with 0.
  - The job stores the total only at the end of a run. A run with a failed snapshot therefore keeps the previous total (ADR-0089 (d)).
  - The condition text of section 3.7 changes to "since the previous run".

The rules live in `reliability.md` section 3.7.

## Consequences

### Positive

- A stored time in the future cannot hide a missing dump, a missing audit check, or a broken chain.
- Section 3.7 and ADR-0073 (g) agree on `export_hash_mismatch`.
- No new number, no new key, and no new code.

### Negative

- After a backward step of more than `alerts.interval`, the job can give a false alert of `backup_age` or `audit_check_age`. The alert lasts until the next dump or check, or until the clock comes within `alerts.interval` of the stored time.
- `alerts.interval` has two meanings: the time between two runs and the largest trusted skew.

### Risks and mitigations

- Risk: an operator increases `alerts.interval`, and the trusted skew increases too. Mitigation: `alerts.interval` is not a configuration key of ADR-0062, so its value is always 60 s.
- Risk: the times in the process are outside this rule. The panic slots and the readiness state of section 3.7 hold no stored time of the database. Mitigation: the job lowers a panic slot that is later than `database_now` to that time. `clock_check` writes a `WARN` line for a skew between `app` and `db`.
- Risk: a panic stops a run after the webhook call and before the job stores the totals. Mitigation: the next run counts the same results again. This can give an extra alert but never hides one (ADR-0073 Risks).

## Related

- ADR-0073 (a), (c), and (g) keep their text. Item (b) applies ADR-0073 (g) to section 3.7, so ADR-0073 gets no note.
- ADR-0075 (the name `export_hash_mismatch`). Item (b) amends the condition text of ADR-0075.
- ADR-0060 (e), (x), and (y) keep their text. An untrusted first start is not a first start in the sense of (y), so ADR-0060 gets no note.
- ADR-0088 (Risks: the due times of `cleanup` after a backward step), ADR-0089 (d) (a failed snapshot).
- `docs/architecture/reliability.md` sections 3.7, 7.9, and 7.11. `docs/architecture/data-model.md` section 6 (`ExportVersion`).
- `crates/recipe-app/src/jobs/alerts.rs` (`Moment`, `AlertSettings::interval`, `store_totals`).
- Baseline requirements: operations "Alert conditions" and "Alerts on a new installation".
- `overview.md` section 7: R31, R37.
