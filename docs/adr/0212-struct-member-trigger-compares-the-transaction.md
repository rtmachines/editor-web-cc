# ADR-0212: The struct-member trigger compares the transaction

Status: Accepted. Amends ADR-0006 (the trigger of immutable revisions) and `data-model.md` §4.6 and §8
Date: 2026-11-22
Deciders: project owner

## Context

ADR-0006 makes each `plc_type` row and its members immutable. The trigger function `plc_type_immutable` of `data-model.md` §4.6 enforces this rule (I22).

A member insert must run in the transaction that created the struct row. The function in `migrations/0002_triggers.sql`, lines 155 to 163, compares `created_at` of the struct row with `now()`. `data-model.md` §4.6 says that this test is true only inside the creating transaction.

That claim is not exact. `now()` is the start time of the transaction, and two transactions can start in the same microsecond. Transaction A inserts a struct and commits. Transaction B starts at the same time and inserts a member of that struct. The test is true in B, so the committed revision changes (`docs/plan/review/2026-11-21-full-repo-review.md` §4.3).

The application cannot cause this case. Its only member insert runs on the connection of the struct insert, directly after it (`crates/recipe-db/src/transaction/type_rows.rs`, function `insert_struct`). The trigger is a backstop for a writer outside the application.

The function has a second gap. When no struct row is visible, the `SELECT` gives NULL values. Then `v_kind <> 'struct'` and `v_created <> now()` are NULL, and the function accepts the insert.

## Decision drivers

- An approved version keeps the exact type structure of its approval (R13, R19, I22).
- A trigger is the last check. It must refuse each insert that it cannot prove valid (`reliability.md` §1.2, the trigger codes).
- The change must keep every stored row readable and must not rewrite a table (`reliability.md` §5).
- The change must need no new grant and no new error code.

## Considered options

### Option A: Compare `xmin` and keep the test of `created_at`

- Pros: `xmin` identifies the transaction that inserted the row. Two open transactions never share one id. The test of `created_at` refuses an old row whose 32-bit `xmin` matches the current id after a wraparound.
- Cons: a struct insert under a savepoint has the `xmin` of the subtransaction. The trigger then refuses its members.

### Option B: Compare `xmin` only

- Pros: one test, and the text of the function stays short.
- Cons: `xmin` has 32 bits, and a frozen row keeps its raw `xmin`. After a wraparound, an old struct can match the current id, and the trigger accepts a member.

### Option C: Keep the function and change the text of `data-model.md` §4.6

- Pros: no migration.
- Cons: the backstop stays open for two transactions with one start time. The gap of a struct row that is not visible also stays open.

## Decision

The decision selects Option A. Migration `0019_struct_member_transaction.sql` replaces the function with `CREATE OR REPLACE FUNCTION plc_type_immutable()`. The function keeps the refusal of `UPDATE` and `DELETE` with `E-PLT-033`.

- (a) A member insert passes only when both tests are true. The `xmin` of the struct row equals the 32-bit id of the current top-level transaction (`pg_current_xact_id()`, PostgreSQL 16). The `created_at` of the struct row equals `now()`.
- (b) The `xmin` test refuses a member from a second transaction with the same start time. The `created_at` test refuses an old struct whose `xmin` matches the current id after a wraparound.
- (c) After the `SELECT` of the struct row, an `IF NOT FOUND` branch refuses the insert with `E-PLT-035`.
- (d) Each compare uses `IS DISTINCT FROM`, so a NULL value refuses the insert. A struct test that fails gives `E-PLT-034`. A kind other than `struct` gives `E-PLT-035`.
- (e) The struct insert and its member inserts run in the top-level transaction. No savepoint and no PL/pgSQL `EXCEPTION` block encloses the struct insert.
- (f) The function keeps its owner, the grant `EXECUTE` to `PUBLIC`, and `SET search_path = pg_catalog, public, pg_temp`. `recipe_app` reads `xmin` through its table grant `SELECT` on `plc_type` (`crates/recipe-db/tests/grants.expected`).
- (g) `data-model.md` §4.6 states both tests and holds the SQL text of the new function. The migration list of `data-model.md` §8 gets the row of `0019`. `deploy/pgtools/restore-code.expected` gets the new `FUNCTION` hash of `plc_type_immutable`.

No new error code, no new grant, and no code change of the application follow. The rows `E-PLT-034` and `E-PLT-035` of `reliability.md` §1.2 stay as they are.

## Consequences

### Positive

- The claim of `data-model.md` §4.6 becomes exact. Only the creating transaction can add a member to a struct.
- A member insert without a visible struct row gets a refusal and a trigger code.
- The migration replaces one function. It scans no table and takes no exclusive lock on a table.

### Negative

- The function has two tests of one fact. A reader must know why both are necessary.
- The hash line of `plc_type_immutable` in `restore-code.expected` must change in the same release as the migration.

### Risks and mitigations

- Risk: a later change puts the struct insert under a savepoint or inside an `EXCEPTION` block. Then the trigger refuses the members of each new struct. Mitigation: item (e) states the rule. A case of the `recipe-db` storage suite inserts a struct and its members through the real adapter.
- Risk: the `IF NOT FOUND` branch is hard to reach. The foreign key needs a visible parent at its later check, and no route knows the id of an uncommitted struct. Mitigation: the branch refuses, so a defect of the branch cannot accept a row.

## Related

- ADR-0006 (immutable UDT revisions), ADR-0017 (derived types)
- `data-model.md` §4.6, §4.11, §5, §8
- `reliability.md` §1.2, §5, §6
- `domain-model.md` §3: I22
- `docs/plan/review/2026-11-21-full-repo-review.md` §4.3
- `overview.md` §7: R13, R19
