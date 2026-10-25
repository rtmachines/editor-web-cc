# ADR-0079: Row of the startup settings comparison

Status: Accepted
Date: 2026-10-24
Deciders: project owner

## Context

`data-model.md` section 6 gives the isolation, the locks, the statements, the audit action, and the errors of each operation. It has rows for the internal operations `EnsureDerivedType` and `cleanup`. It has no row for `CompareSettings`, the startup settings comparison of `reliability.md` section 7.12.

Step P4.1 writes the port `SettingRepository` and its two adapters. The adapters make these choices, and no document records them:

1. `lock_rows` locks every configuration row of `app_setting` with `FOR NO KEY UPDATE` in the order of `id`.
2. `upsert` writes one row with `INSERT ... ON CONFLICT (key) DO UPDATE` and sets `updated_by` to `NULL`.
3. `lock_rows` skips the three state keys. A fix of the same step also makes `upsert` refuse a state key before any write.

The project rule of the SQL tests asks for a test that fails midway in each operation of section 6. Without a row, that rule does not name `CompareSettings`.

Two first starts of `serve` at the same time can both find a configuration key without a row. Section 7.12 does not say what the second start then writes.

## Decision drivers

- Every setting change has an audit entry, also in read-only mode (A24, A34, ADR-0061 (e)).
- The rows and their entries commit together or not at all (R29, ADR-0015).
- A statement that locks several rows of one table orders them by `id` (`data-model.md` section 6).
- The state keys belong to the port `MaintenanceStore` and get no audit entry (`data-model.md` section 3.17).
- No new setting, no new row of `reliability.md` section 7.11, and no new error code.

## Considered options

### Option A: A row in section 6 with the lock and the statements of the adapters

- Pros: section 6 names every operation that writes rows. Each fact has one normative place. The rule of the midway test then covers `CompareSettings`.
- Cons: section 6 gets a row for an operation that no user starts.

### Option B: No row. The lock and the statements stay details of the adapters

- Pros: no document changes.
- Cons: a later change of an adapter can drop the lock order or write a state key, and no document shows the fault. Section 7.12 stays the only source, and it names no lock.

## Decision

The project uses **Option A**.

The row of section 6:

- (a) `data-model.md` section 6 gets the row `CompareSettings` after the row `cleanup`. The row holds these facts:
  - Isolation: `READ COMMITTED`, one transaction in step 7 of the start order of ADR-0061.
  - Lock: `SELECT id, key, value, to_jsonb(s) FROM app_setting s WHERE key <> ALL($state_keys) ORDER BY id FOR NO KEY UPDATE`. The operation takes no other row lock.
  - The audit insert takes the lock of `audit_chain_head` last, as in every command. `app_setting` is outside the lock chain of ADR-0015, so no cycle can occur.
  - Statements: the operation compares each configured key with its locked row. A missing row is a difference with the old value `null`. A locked row without a configured key stays unchanged.
  - For each difference it runs `INSERT INTO app_setting (key, value) VALUES ($k, $v) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_by = NULL`.
  - The trigger `app_setting_set_updated_at` sets `updated_at` on the update path.
  - Audit: one `SettingChanged` entry per difference with `actor = None` and the full row before and after (section 10). All entries of one start share one `request_id`.
  - With no difference, the operation writes no row and no entry. The commit path needs at least one entry (`domain-model.md` section 5), so the transaction ends without a commit. The rollback releases the locks.
  - Errors: each `StorageError` of the transaction, and `AuditWriteFailed` (`E-AUD-001`). The server then writes one `ERROR` line and does not start (`reliability.md` section 7.12 step 4, ADR-0061 (b)).
- (b) The operation also runs in read-only mode (ADR-0061 (e)).
- (c) The operation never writes a state key. `lock_rows` skips the three state keys. `upsert` refuses a state key with `E-DB-008` before any write. Only the jobs write the state keys, through `MaintenanceStore`.

Accepted risk:

- (d) Two first starts of `serve` at the same time can both find no row of a key. Then both run the insert.
  - The unique constraint `app_setting_key_unique` makes the second insert wait for the first transaction. After the first commit, the second insert takes the update path.
  - The `SettingChanged` entry of the second start then has no old value, although a row exists at its commit.
  - Design D9 of the change `operations-compose` runs one `app` container. The case therefore needs a second server on the same database. The project accepts this risk.

Test of a failure midway:

- (e) The rule of the midway test covers the new row. In the change `operations-compose`, two tasks hold the tests.
  - Task 3.2 holds the storage test of design D14 row T8. A failed statement after two upserts writes nothing. A failed audit insert also keeps every upsert out.
  - Task 3.9 holds the use case test of design D14 row T15, "a failed commit".

## Consequences

### Positive

- Section 6 names the lock, the statements, and the errors of every operation that writes rows at the start.
- A review can compare each adapter with one row.
- The midway test rule covers the comparison.

### Negative

- The comparison locks every configuration row, also when no value changed. Only the comparison writes these rows, so no other transaction waits for the lock.
- Section 6 gets a third internal operation next to `EnsureDerivedType` and `cleanup`.

### Risks and mitigations

- Risk: two parallel first starts give one entry with no old value (d). Mitigation: one `app` container. The audit log still holds the new value of each key.
- Risk: a later key of `serve` uses the name of a state key. Mitigation: `upsert` refuses the key, and the start stops with one `ERROR` line.

## Related

- This ADR amends no ADR.
- ADR-0015 (lock order), ADR-0022 (settings), ADR-0060 (t) (internal operations), ADR-0061 (step 7, (b), (e)), ADR-0062 (a), ADR-0063 (c), ADR-0073 (i)
- `docs/architecture/data-model.md` sections 3.17, 6, and 10
- `docs/architecture/reliability.md` section 7.12. `docs/architecture/domain-model.md` section 5 (commit paths)
- Baseline requirements: `operations` "Startup settings comparison" and "Failed startup settings comparison"
- `overview.md` section 7: R20, R29, R31, R35. Section 9: A24, A34
