# ADR-0096: Guard lock of the last Administrator

Status: Proposed
Date: 2026-10-25
Deciders: project owner

## Context

ADR-0015 and ADR-0035 run `SetUserRole` and `DeactivateUser` in `SERIALIZABLE`. A serialization failure (`40001`) gives one more run. The isolation keeps two concurrent demotions from leaving no active Administrator.

ADR-0035 names a known limit of this design:

1. Each request of the target touches its session row in a separate statement (ADR-0035 (k)).
2. The command deletes the same rows. A touch that commits after the snapshot of the command causes `40001`.
3. A target that sends requests without a pause can cause `40001` twice. The command then returns `E-DB-001`, and the target keeps its access.
4. ADR-0035 says that step P4.2 decides on a fix. The security audit of P4.2 confirms the limit.

`READ COMMITTED` alone does not keep the last-administrator rule. Two commands on two Administrators lock two different user rows. Each count then sees two active Administrators, and both commands commit.

`data-model.md` section 1.4 gives class 1 to `Login` T1 and class 2 to the derived types. It reserves class 3 for the audit head.

## Decision drivers

- An Administrator ends the access of a user with one command, also while that user sends requests (R28, A9).
- The system keeps one active Administrator (A9).
- The account change, the epoch increment, the session delete, and the audit entry commit together (R29).
- The lock order of ADR-0015 stays free of cycles (R31).

## Considered options

### Option A: `READ COMMITTED` and one advisory lock of a new class 4

- Pros: one statement orders exactly the commands that can reduce the number of active Administrators. No retry, no table, and no grant. A session touch never waits for this lock.
- Cons: a new class in `data-model.md` section 1.4. Two administrative commands run one after the other.

### Option B: `READ COMMITTED` and a row lock on every active Administrator

- Pros: no new lock class.
- Cons: the result depends on the second evaluation of a locked row under `READ COMMITTED`. The lock also holds the rows of users who are not the target, so their `Login` T2 waits.

### Option C: `SERIALIZABLE` with more runs

- Pros: no change of the lock model.
- Cons: a busy target can still use up every fixed number of runs.

### Option D: A fixed key in class 1 or class 2

- Pros: no new lock class.
- Cons: a fixed key can equal the `hashtext` value of a login or of a type name. Two unrelated commands then wait for each other.

## Decision

The project uses **Option A**.

- (a) `SetUserRole` and `DeactivateUser` run in one `READ COMMITTED` transaction. They have no retry.
- (b) The first statement of the transaction is `SELECT pg_advisory_xact_lock(4, 0)`. Class 4 is the guard of the last Administrator. Its only key is 0.
- (c) The command then locks the user row `FOR NO KEY UPDATE`, counts the active Administrators, and applies the last-administrator rule. The count is a new statement, so it sees each commit of an earlier holder of the lock.
- (d) The command then changes the row, increments `session_epoch`, deletes every session of the user, and commits one audit entry. These writes stay in the one transaction.
- (e) The lock order is: the class-4 lock, the `app_user` row, the `session` rows, and the row of `audit_chain_head`.
  - No other command takes class 4.
  - `Login` T2, `ChangeOwnPassword`, and `ResetPassword` also lock the `app_user` row before the `session` rows.
  - The session touch locks only its own session row. No cycle exists in the class-4 path.
  - `data-model.md` section 6 and `.claude/rules/sql.md` state this order as the lock order of the identity commands.
  - Known case: the `DELETE` of expired sessions of the maintenance job deletes many rows. It can meet `delete_all_of_user` of a command in the other row order. PostgreSQL then gives `40P01`, and the command returns `E-DB-001`. This pair exists before this ADR. The Administrator repeats the command.
- (f) The wait for the class-4 lock obeys `db.lock_timeout` of `reliability.md` section 7.11. A timeout (`55P03`) returns `E-DB-001`. A deadlock still returns `E-DB-001` and logs at `ERROR`.
- (g) The command succeeds in both orders of a touch.
  - A touch that commits before the delete loses its row with the delete.
  - A touch that comes after the delete waits for the commit. Then it finds no row, and the request gets `E-WEB-004`.
- (h) `bootstrap_administrator` of `create-admin` stays `SERIALIZABLE` with one more run (ADR-0035 (h)). It deletes no session row, so a touch cannot cause its failure.
- (i) `data-model.md` section 1.4 gets the row of class 4. Sections 4.9 and 6 and the documents that name the isolation of the two commands change in the same change.
- (j) This item amends ADR-0015: the isolation and the retry of `SetUserRole` and `DeactivateUser`. It also ends the known limit of ADR-0035 for these two commands. It ends the sentence of ADR-0022 (Risks) about the retry of a `SERIALIZABLE` command after a head update.
- (k) Race tests of `recipe-db` use the gate of `support::gate`. No test depends on a sleep. The races are:
  - a touch during `DeactivateUser`, and a touch during `SetUserRole`
  - a touch after the delete of the sessions
  - two Administrators who demote each other
  - three Administrators and two parallel deactivations, which both commit, one after the other
- (l) Unit tests of `recipe-app` use the memory adapter. They show that the guard lock is the first port call after `authorize`. They show that one `READ COMMITTED` run happens, and that a serialization fault gives no second run. A lock timeout at the guard lock gives `E-DB-001`, no write, and no audit entry.

## Consequences

### Positive

- A target that sends requests cannot stop its own deactivation or demotion.
- The last-administrator rule holds without `SERIALIZABLE`.
- Only `create-admin` keeps a retry.

### Negative

- One more advisory lock class.
- Two administrative commands on two users wait for each other for the length of one short transaction.

### Risks and mitigations

- Risk: a later command reduces the number of active Administrators without the class-4 lock. Mitigation: `data-model.md` section 1.4 names every command of class 4. The race test of two demotions fails without the lock.
- Risk: a long first command makes the second one reach `db.lock_timeout`. Mitigation: the transaction holds no wait for I/O (`reliability.md` section 7.4). The Administrator repeats the command after `E-DB-001`.
- Risk: an old `app` and a new `app` run at the same time. The old one runs `SERIALIZABLE` without the class-4 lock, so the two versions do not exclude each other. Mitigation: the upgrade replaces the one container of `app` with `up -d --no-deps app`. The runbook says: never run two `app` containers at the same time.

## Related

- Amends ADR-0015 (isolation and retry of `SetUserRole` and `DeactivateUser`).
- Amends ADR-0035 (the known limit of code review M7).
- Amends ADR-0022 (the retry sentence of its risks).
- `data-model.md` sections 1.4, 4.9, and 6.
- `domain-model.md` sections 5 and 5.3. `reliability.md` sections 1.1, 7.6, and 7.11. `api.md` section 3.2. `overview.md` section 7 (R31 and the KISS row).
- `crates/recipe-app/src/identity/admin_change.rs`, `crates/recipe-db/tests/storage/race/identity.rs`.
- Baseline requirements: `identity-access` "Last administrator", "Concurrent demotion of administrators", and "Session end on role change, deactivation, and reset".
- `domain-model.md` section 7 (term "guard lock of the last Administrator"). `docs/ops/runbook.md` (one `app` container).
- `overview.md` section 7: R28, R29, R31. Section 9: A9.
