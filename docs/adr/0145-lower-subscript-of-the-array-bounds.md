# ADR-0145: Lower subscript of the array bounds

Status: Accepted. Amends ADR-0017 (the function `dims_valid`)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a gap in the CHECK of an array row. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #5.

These facts shape the decision:

- `plc_type.dim_lower` and `plc_type.dim_upper` are `INTEGER[]`. A PostgreSQL array can start at any subscript, for example `'[0:0]={5}'`.
- `dims_valid` walks the subscripts of `lower_b` and reads `upper_b` at the same subscript. When the two arrays start at different subscripts, it compares a bound with `NULL`. Then the test of the dimension passes.
- Thus `dims_valid('[0:0]={5}', '{0}')` and `dims_valid('[0:0]={0}', '{65536}')` give `true`. `dims_valid('{1}', '[0:0]={8}')` also gives `true`, because the upper array starts at 0. The CHECK `plc_type_array_shape` accepts these rows.
- Array equality of PostgreSQL also compares the subscripts. Thus `plc_type_array_unique` sees `'[0:0]={1}'` and `'{1}'` as two values, and one shape can get two rows.
- The domain writes each array from a list, and the driver sends it with the subscript 1. The driver drops the subscripts on a read. Thus only a statement outside the application can store such a row.

## Decision drivers

- The database refuses a row that breaks a rule of the domain (R13, R31).
- One shape has one row (ADR-0017).
- A rule of the database must not depend on the client that writes the row.

## Considered options

### Option A: `dims_valid` requires the subscript 1 at the start of both arrays

- Pros: one function changes. The CHECK and the unique index then see one form of each shape.
- Cons: one migration replaces the function.

### Option B: `dims_valid` walks the subscripts of each array on its own

- Pros: any start subscript passes.
- Cons: `plc_type_array_unique` still sees two values for one shape.

### Option C: The domain normalizes the arrays on a read

- Pros: no migration.
- Cons: the database keeps rows that break the rule, and the unique index stays open.

## Decision

The project uses **Option A**.

- (a) `dims_valid` also requires `array_lower(lower_b, 1) = 1` and `array_lower(upper_b, 1) = 1`. Otherwise it gives `false`. The NULL-safe form of ADR-0017 stays.
- (b) Thus `plc_type_array_shape` refuses an array row whose bounds start at another subscript. `data-model.md` §3.3 holds the new DDL.
- (c) Migration `0011_plc_type_rules.sql` replaces the function with `CREATE OR REPLACE FUNCTION`. PostgreSQL does not check the stored rows again after the replace. Thus the migration first counts the stored array rows that break item (a), and it stops when the count is not 0.
- (d) The replace keeps the grant `EXECUTE` of `PUBLIC` on the function. Thus `grants.expected` does not change. The line of `dims_valid` in `deploy/pgtools/restore-code.expected` gets the new hash.
- (e) The domain and `recipe-db` do not change. They already write and read every array with the subscript 1.

## Consequences

### Positive

- Every stored array row has one form, and `plc_type_array_unique` holds one row for each shape.

### Negative

- The upgrade stops at migration `0011` when the stored data holds such a row. A forward correction then needs a new decision.

### Risks and mitigations

- Risk: a manual statement stored such a row before the upgrade. Mitigation: the ops query of ADR-0143 (k) lists the row before the upgrade. The owner checks the query result before the merge.

## Related

- ADR-0017 (identity of derived types), ADR-0143 (identifier rules and the ops query), ADR-0144 (order of the class-2 locks)
- `data-model.md` §3.3, §8, `reliability.md` §5
- `overview.md` §7: R13, R31. §9: A38
