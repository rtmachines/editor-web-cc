# ADR-0144: Order of the class-2 locks by key value

Status: Accepted. Amends ADR-0017 (order of the keys of derived types)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a deadlock path in the class-2 advisory locks. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #6.

These facts shape the decision:

- ADR-0017 sorts the class-2 keys in the fixed order of the key kinds: the struct name, `STRING`, `WSTRING`, `ARRAY`. The lock takes the key value `hashtext(machine_id::text || ':' || name)`.
- The order of the kinds is not the order of the key values. A struct named `ARRAY` has the same key value as the key `ARRAY`.
- Thus `DefineUdt` of a struct `ARRAY` with a member `STRING(5)` locks `ARRAY` and then `STRING`. `AddParameter` with `ARRAY[1..2] OF STRING(5)` locks `STRING` and then `ARRAY`. Two such commands at one time can end with `40P01`.
- `hashtext` gives a 32-bit value. Two other key texts can also give one value, and the same inversion can occur.
- ADR-0143 refuses a new struct named `ARRAY`. A stored struct with that name can still exist, and the collision of two other texts stays.
- ADR-0015 uses the order of `data-model.md` §1.4 and does not define it.

The project owner decided on 2026-11-06 (fix plan, question Q-I, option (1)).

## Decision drivers

- No two commands wait on each other in opposite orders (R29, R31).
- The rule does not depend on the names that a user can choose.
- The lock set of ADR-0017 stays. Only the order changes.

## Considered options

### Option A: Sort by the key value in the database

- Pros: the order is the order of the values that the locks take. A collision of two texts gives one lock.
- Cons: one more statement before the locks.

### Option B: Compute `hashtext` in Rust and sort in the application

- Pros: no extra statement.
- Cons: the code copies an internal hash of PostgreSQL. A new major version of PostgreSQL can change it without notice.

### Option C: Rely on the keyword rule of ADR-0143

- Pros: no change of the lock code.
- Cons: stored structs and hash collisions of other texts keep the deadlock path.

### Option D: One class-2 key per machine

- Pros: one lock, so no order.
- Cons: every command that creates a derived type of a machine waits for every other such command of the machine.

## Decision

The project uses **Option A**.

- (a) The adapter of `lock_derived_types` computes the key value of each key text in one statement. The statement removes duplicate values and sorts the values in ascending order.
- (b) Then the adapter takes `pg_advisory_xact_lock(2, key)` for each value in that order, with one statement for each value.
- (c) Two key texts with one key value take one lock. The second text needs no lock of its own, because the lock is the value.
- (d) `recipe-app` collects the key texts as before and removes duplicate texts. It does not define the order of the locks. The memory adapter takes the keys in the order of the texts.
- (e) The key texts do not change. The text of a struct is its exact name (ADR-0143 (h)). The other texts are `STRING`, `WSTRING`, and `ARRAY`.
- (f) The place of the locks does not change. A command takes them after its row locks and before its first `plc_type` insert (ADR-0015, `data-model.md` §6).
- (g) This decision replaces the sentence of ADR-0017 that sorts the keys in the fixed order of the key kinds.

## Consequences

### Positive

- Two commands always take their class-2 locks in one order.
- A hash collision of two texts costs one lock and no deadlock.

### Negative

- Each command that takes class-2 locks runs one more statement.
- The order of the locks depends on the machine identifier. A test cannot name the order of two texts without the key values.

### Risks and mitigations

- Risk: a later change takes a class-2 lock outside the adapter. Mitigation: the rule in `.claude/rules/sql.md` names the order by key value, and the contract test of `recipe-db` pins it.

## Related

- ADR-0015 (locking model), ADR-0017 (identity of derived types), ADR-0143 (identifier rules)
- `data-model.md` §1.4, §6, `domain-model.md` §5.1
- `overview.md` §7: R29, R31
