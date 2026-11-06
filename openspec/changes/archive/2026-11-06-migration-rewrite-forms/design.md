# Design: migration-rewrite-forms

## Context

See `proposal.md`, section Why. The normative sources are `data-model.md` §5.1 and §8, `reliability.md` §5 and §7.10, ADR-0069, and ADR-0127 (n) to (u).

Facts that shape the design:

- `deploy/ci/migration_rewrite.py` splits each file into statements of lowercase tokens. `check_statement` reads `DO`, `ALTER TABLE`, `CREATE TABLE`, `DROP TABLE`, and function bodies. It ignores every other statement.
- `body_forms` reads only `ALTER TABLE`, `DO`, and `EXECUTE` inside a body.
- `volatile_default` returns no form for `ADD CONSTRAINT`, `ADD CHECK`, `ADD FOREIGN`, `ADD UNIQUE`, and `ADD PRIMARY`. It has no guard for `ADD EXCLUDE`.
- The shipped files `0001` to `0010` hold one form of this change: `CREATE INDEX security_event_occurred_at_idx` in `0006_operations.sql` line 10. The table comes from `0001`. Each of the 41 indexes of `0001` is on a table that `0001` creates.
- `0009` adds two constraints with `NOT VALID`, and `0010` runs `VALIDATE CONSTRAINT` on them. Both files pass.
- A `CONCURRENTLY` index needs a file that starts with `-- no-transaction` and holds one statement. Such a file cannot run `SET ROLE audit_owner`, so an index on an audit object is a runbook procedure. The next migration records the index with `CREATE INDEX IF NOT EXISTS` (`data-model.md` §5.1, `reliability.md` §7.10).
- PostgreSQL 16 accepts `USING INDEX` only for `PRIMARY KEY` and `UNIQUE`. `PRIMARY KEY USING INDEX` on a column without `NOT NULL` runs an implicit `SET NOT NULL`, which scans the table under `ACCESS EXCLUSIVE`.
- The tests of `test_migration_rewrite.py` use the constant `CHECK`, which adds a check without `NOT VALID` to a table of an earlier file. The test `test_stored_generated_column_stays_out_of_scope` expects a pass. Both conflict with the new forms.

## Goals / Non-Goals

**Goals:**

- One diagnostic name for each new form, stable like the four current names.
- The same exemption rules as ADR-0127 (q) and (t) for the new forms.

**Non-Goals:**

- No parser of PL/pgSQL. A `DO` block stays a form of its own.
- No check of the lock that a `NOT VALID` foreign key takes on the referenced table. That lock is short and does not scan.
- No knowledge of column nullability. The check does not read the column list of `CREATE TABLE` for `NOT NULL`.

## Decisions

### D1: Names of the new forms

The script prints `file:line: <form>` with these names:

| Form | Statement |
|---|---|
| `stored generated column` | `ADD [COLUMN] ... GENERATED ALWAYS AS (...) STORED` |
| `constraint without not valid` | `ADD [CONSTRAINT n] CHECK` or `FOREIGN KEY` without `NOT VALID`. An inline `CHECK` or `REFERENCES` in `ADD [COLUMN]` |
| `index without concurrently` | `CREATE [UNIQUE] INDEX` without `CONCURRENTLY`, also with `IF NOT EXISTS` |
| `unique without using index` | `ADD [CONSTRAINT n] UNIQUE` without `USING INDEX`. An inline `UNIQUE` in `ADD [COLUMN]` |
| `primary key` | Each `ADD [CONSTRAINT n] PRIMARY KEY`, also with `USING INDEX`. An inline `PRIMARY KEY` in `ADD [COLUMN]` |
| `exclusion constraint` | Each `ADD [CONSTRAINT n] EXCLUDE` |
| `vacuum full` | `VACUUM` with the option `FULL`, also in the form `VACUUM (FULL, ...)`. `VACUUM (FULL false)` passes |
| `cluster` | `CLUSTER` |
| `set tablespace` | `ALTER TABLE ... SET TABLESPACE` |
| `set logged` | `ALTER TABLE ... SET LOGGED` or `SET UNLOGGED` |
| `set access method` | `ALTER TABLE ... SET ACCESS METHOD` |
| `domain constraint without not valid` | `ALTER DOMAIN ... ADD [CONSTRAINT n] CHECK` without `NOT VALID`, and `ALTER DOMAIN ... SET NOT NULL` |
| `domain validate` | `ALTER DOMAIN ... VALIDATE CONSTRAINT` |
| `lock table` | `LOCK [TABLE]` |

`VALIDATE CONSTRAINT` of a domain takes a `SHARE` lock on each table that uses the domain and scans that table in PostgreSQL 16. It thus blocks writes. `VALIDATE CONSTRAINT` of a table does not block writes.

Alternative: one name `table rewrite` for all forms. Rejected, because an `EXEMPT` entry must name the exact form that an ADR permits.

### D2: Exemption of an object that the same file creates

ADR-0127 (q) applies to each new form on a table. An unconditional `CREATE DOMAIN` in the same file exempts the domain in the same way. This covers `domain constraint without not valid` and `domain validate`. `VACUUM FULL` and `CLUSTER` without a table name act on every table, so they always count.

### D3: Statements outside `ALTER TABLE`

`check_statement` reads these statements at the start of a statement:

- `CREATE [UNIQUE] INDEX [CONCURRENTLY] [IF NOT EXISTS] [name] ON [ONLY] table`.
- `VACUUM` and `CLUSTER`.
- `ALTER DOMAIN`.
- `LOCK [TABLE] [ONLY] name [, ...]`.

It resolves each table name with the rules of ADR-0127 (o). `body_forms` reads the same statements only at the start of a statement inside a function body or an `EXECUTE` string. A column with the name `cluster` or `vacuum` thus gives no form.

### D4: Inline constraints of `ADD COLUMN`

An inline `CHECK` in `ADD COLUMN` scans the table. An inline `UNIQUE` or `PRIMARY KEY` builds an index under `ACCESS EXCLUSIVE`. These clauses cannot take `NOT VALID` or `USING INDEX`.

An inline `REFERENCES` without a `DEFAULT` does not scan, because each value of the new column is `NULL`. The check still rejects it, by the decision of the owner. One rule for all inline constraints keeps the parser small. The safe path adds the column first and then `ADD CONSTRAINT ... FOREIGN KEY ... NOT VALID`.

### D5: Primary key

The check does not know if a column has `NOT NULL`. So each `ADD PRIMARY KEY` on a table of an earlier file counts, also with `USING INDEX`. Each table gets its key at creation, so this case is rare. `UNIQUE USING INDEX` passes.

### D6: `EXEMPT` entries

`EXEMPT` gets the entry `("0006_operations.sql", "index without concurrently")` with ADR-0069. The dead-entry test of ADR-0127 (t) keeps the entry live.

A migration that records an index of the runbook procedure of `data-model.md` §5.1 also needs an entry. The entry names the ADR that adds the index. Without the runbook step, `CREATE INDEX IF NOT EXISTS` builds the index under a `SHARE` lock, so CI does not let it pass without an ADR.

### D7: Parser details

- `NOT VALID` counts only after the closing parenthesis of the `CHECK` expression, as in `add_checks`. Thus `CHECK (NOT valid)` is no proof.
- The guard of `volatile_default` also returns no form for `ADD EXCLUDE`.
- A statement with several `ALTER TABLE` actions reports each form once.

### D8: Documents

New ADR-0136 records D1 to D7. It amends ADR-0127 (p), which keeps `GENERATED ... STORED` out of scope. ADR-0127 gets the Status line "Accepted. Amended by ADR-0136".

- `data-model.md` §8 lists all forms.
- `reliability.md` §5 links to that list and does not repeat it (`.claude/rules/docs.md`, one rule in one document).
- `data-model.md` §5.1, `reliability.md` §7.10 item 7, and `docs/ops/runbook.md` add the `EXEMPT` entry to the recording migration.

## Risks / Trade-offs

- [`EXEMPT` keys on the file and the form] → One entry for a runbook-recording migration also permits each other index without `CONCURRENTLY` in that file. The review of the entry must check the file.
- [`EXECUTE` reads only a plain string literal] → `EXECUTE E'...'`, `EXECUTE format(...)`, and `EXECUTE (...)` give no form.
- [A file can create a function body after its table, and ADR-0127 (q) exempts that body] → A later file can call that function. The review of a migration must check such a call.
- [Some statements are out of scope] → These pass: `REINDEX`, `ALTER INDEX ... SET TABLESPACE`, `ALTER MATERIALIZED VIEW ... SET TABLESPACE`, `REFRESH MATERIALIZED VIEW`, `ALTER TABLE ... ATTACH PARTITION`, and `ALTER TABLE ALL IN TABLESPACE`.
- [An exclusion constraint or a primary key on an existing table always fails] → The author adds an `EXEMPT` entry with an ADR. The other path is a new table.
- [The splitter misreads a new statement form] → One fixture for each form and each syntax variant. An unterminated quote still exits 2.
- [Each recorded runbook index needs an ADR] → The index of an audit object is rare. The ADR also documents why the index exists.
- [`ALTER TABLE ALL IN TABLESPACE` parses as a table with the name `all`] → The statement passes by mistake. It is out of scope, and no migration moves a tablespace.
- [The tokens do not show a statement form, for example `VACUUM` with a quoted option list] → The statement passes by mistake. The fixtures cover the forms of the PostgreSQL 16 grammar.

## Migration Plan

The change touches CI scripts and documents only. No migration and no deploy step. A revert of the merge commit restores the old check.
