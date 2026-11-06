# ADR-0136: More rewrite forms of the migration check

Status: Accepted. Amends ADR-0127 (p)
Date: 2026-11-06
Deciders: project owner

## Context

`data-model.md` §8 forbids a migration that rewrites a table or scans it under an exclusive lock. ADR-0127 (n) to (u) set the script `deploy/ci/migration_rewrite.py`. That script reports four forms: a column type change, a volatile default, `SET NOT NULL` without a valid check, and a `DO` block.

The Risks of ADR-0127 name the residual risk "other rewrite forms pass". Examples are `ADD COLUMN ... GENERATED ALWAYS AS ... STORED` and `ADD CONSTRAINT` without `NOT VALID`. ADR-0127 (p) keeps `GENERATED ... STORED` out of scope.

These facts shape the decision:

- `check_statement` reads `DO`, `ALTER TABLE`, `CREATE TABLE`, and `DROP TABLE`. It ignores every other statement.
- A constraint without `NOT VALID` scans the table. A `UNIQUE` or `PRIMARY KEY` constraint builds an index under an `ACCESS EXCLUSIVE` lock.
- `CREATE INDEX` without `CONCURRENTLY` blocks writes. A `CONCURRENTLY` index needs a file that starts with `-- no-transaction` and holds one statement.
- Such a file cannot run `SET ROLE audit_owner`. An index on an audit object is thus a runbook procedure. The next migration records the index with `CREATE INDEX IF NOT EXISTS` (`data-model.md` §5.1, `reliability.md` §7.10).
- PostgreSQL 16 accepts `USING INDEX` only for `PRIMARY KEY` and `UNIQUE`.
- `PRIMARY KEY USING INDEX` on a column without `NOT NULL` runs an implicit `SET NOT NULL`. That statement scans the table under `ACCESS EXCLUSIVE`.
- The script has no knowledge of column nullability.
- `ALTER DOMAIN ... VALIDATE CONSTRAINT` takes a `SHARE` lock on each table that uses the domain and scans that table. It blocks writes. For a table, `VALIDATE CONSTRAINT` does not block writes.
- The shipped files `0001` to `0010` hold one of the new forms. It is `CREATE INDEX security_event_occurred_at_idx` in `0006_operations.sql`. ADR-0069 adds that index.

The project owner decided on 2026-11-06:

- The script always rejects an inline `REFERENCES` in `ADD COLUMN`, for simplicity.
- The script rejects each `ADD PRIMARY KEY` on a table of an earlier file.
- A migration that records a runbook index needs an `EXEMPT` entry with an ADR.
- The three forms `SET ACCESS METHOD`, `ALTER DOMAIN`, and `LOCK TABLE` join the list.
- The script also rejects `ALTER DOMAIN ... VALIDATE CONSTRAINT` on a domain of an earlier file.

## Decision drivers

- Each form has one stable diagnostic name. An `EXEMPT` entry names the exact form that an ADR permits.
- The new forms follow the exemption rules of ADR-0127 (q) and (t).
- A gap of the script gives a false alarm, never a silent miss.
- The script stays small. It does not read PL/pgSQL and does not read the column list of `CREATE TABLE`.
- No new Cargo dependency and no new external binary in CI.

## Considered options

### Option A: One name for each form

- Pros: an `EXEMPT` entry names the exact form. The 14 names are as stable as the four current names.
- Cons: the list is longer.

### Option B: One name `table rewrite` for all forms

- Pros: one name.
- Cons: an `EXEMPT` entry cannot name the exact form that an ADR permits.

## Decision

The project uses **Option A**. The items below are the rules. This ADR amends ADR-0127 (p). Items (n) to (u) of ADR-0127 stay in force for the new forms, except where an item below says otherwise.

- (a) The script reports 14 more forms. It prints `file:line: <form>` with these names:

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

- (b) ADR-0127 (q) applies to each new form on a table. A table that an unconditional `CREATE TABLE` in the same file created gives an exemption. An unconditional `CREATE DOMAIN` in the same file exempts the domain in the same way. This also applies to `domain validate`.
  - `VACUUM FULL` and `CLUSTER` without a table name act on every table. They always count.
- (c) `check_statement` reads four more statements at the start of a statement:
  - `CREATE [UNIQUE] INDEX [CONCURRENTLY] [IF NOT EXISTS] [name] ON [ONLY] table`.
  - `VACUUM` and `CLUSTER`.
  - `ALTER DOMAIN`.
  - `LOCK [TABLE] [ONLY] name [, ...]`.
  - The script resolves each table name with the rules of ADR-0127 (o).
  - `body_forms` reads the same statements only at the start of a statement inside a function body or an `EXECUTE` string. A column with the name `cluster` or `vacuum` thus gives no form.
- (d) An inline `CHECK` in `ADD COLUMN` scans the table. An inline `UNIQUE` or `PRIMARY KEY` builds an index under `ACCESS EXCLUSIVE`. These clauses cannot take `NOT VALID` or `USING INDEX`.
  - An inline `REFERENCES` without a `DEFAULT` does not scan, because each value of the new column is `NULL`. The script still rejects it, by the decision of the owner.
  - One rule for all inline constraints keeps the parser small. The safe path adds the column first. Then it adds `ADD CONSTRAINT ... FOREIGN KEY ... NOT VALID`.
- (e) The script does not know if a column has `NOT NULL`. Each `ADD PRIMARY KEY` on a table of an earlier file thus counts, also with `USING INDEX`. `UNIQUE USING INDEX` passes.
  - Each table gets its key at creation, so this case is rare.
- (f) The constant `EXEMPT` gets the entry `("0006_operations.sql", "index without concurrently")` with ADR-0069. The dead-entry test of ADR-0127 (t) keeps the entry live.
  - A migration that records an index of the runbook procedure of `data-model.md` §5.1 also needs an entry. The entry names the ADR that adds the index.
  - Without the runbook step, `CREATE INDEX IF NOT EXISTS` builds the index under a `SHARE` lock. CI thus does not let it pass without an ADR.
- (g) The parser follows these details:
  - `NOT VALID` counts only after the closing parenthesis of the `CHECK` expression. Thus `CHECK (NOT valid)` is no proof.
  - The guard of the volatile default also returns no form for `ADD EXCLUDE`.
  - A statement with several `ALTER TABLE` actions reports each form once.
- (h) `GENERATED ... STORED` is no longer out of scope. Item (a) replaces that sentence of ADR-0127 (p). The four forms of ADR-0127 (p) stay.

## Consequences

### Positive

- A migration with one of the 14 forms fails CI on the pull request.
- Each new form has its own name, so an `EXEMPT` entry names the exact form.
- The shipped files `0001` to `0010` stay valid. Only `0006_operations.sql` needs the entry of item (f).

### Negative

- An exclusion constraint or a primary key on an existing table always fails. The author adds an `EXEMPT` entry with an ADR. The other path is a new table.
- Each recorded runbook index needs an ADR. The index of an audit object is rare. The ADR also documents why the index exists.
- The tests of `test_migration_rewrite.py` changed. The constant `CHECK` of the tests got `NOT VALID` and a `VALIDATE CONSTRAINT`. The old test of `GENERATED ... STORED` became a failing case.
- A domain check on a domain of an earlier file needs an `EXEMPT` entry with an ADR. The `VALIDATE CONSTRAINT` of a domain blocks writes.
- The documents `data-model.md` §5.1 and §8, `reliability.md` §5 and §7.10, and `docs/ops/runbook.md` change in the same change as this ADR.

### Risks and mitigations

- Risk: the splitter misreads a new statement form. Mitigation: one fixture for each form and each syntax variant. An unterminated quote still exits 2.
- Risk: `EXEMPT` keys on the pair of file and form. One entry for a runbook migration also permits each other index without `CONCURRENTLY` in that file. Mitigation: the review of the entry checks the whole file.
- Risk: `EXECUTE` reads only a plain string literal. `EXECUTE E'...'`, `EXECUTE format(...)`, and `EXECUTE (...)` give no form. Mitigation: the review of a migration checks each dynamic `EXECUTE`.
- Risk: a function body that a file creates after its table is exempt (ADR-0127 (q)). A later file can call that function. Mitigation: the review of a migration checks such a call.
- Risk: some statements are out of scope, so they pass. They are `REINDEX`, `ALTER INDEX ... SET TABLESPACE`, `ALTER MATERIALIZED VIEW ... SET TABLESPACE`, `REFRESH MATERIALIZED VIEW`, `ALTER TABLE ... ATTACH PARTITION`, and `ALTER TABLE ALL IN TABLESPACE`. The last one parses as a table with the name `all`. Mitigation: no migration moves a tablespace, and the review of a migration checks these statements.
- Risk: the tokens do not show a statement form, for example `VACUUM` with a quoted option list. Mitigation: the fixtures cover the forms of the PostgreSQL 16 grammar.
- Risk: the script does not check the lock that a `NOT VALID` foreign key takes on the referenced table. Mitigation: that lock is short and does not scan.
- Risk: an `EXECUTE` string that joins parts with `||`, for example `EXECUTE 'ALTER TABLE ' || quote_ident(n) || ' ...'`, makes the script exit 2 with the message "missing table name". Mitigation: the check fails closed, and the author writes the table name as a literal or adds an `EXEMPT` entry with an ADR.
- Risk: some rare names give a false alarm. Examples are a unique index with the name `tablespace` in `ADD UNIQUE USING INDEX tablespace`, a domain with the name `exclude` as a column type, and `VACUUM (FULL "false")`. Mitigation: a false alarm fails closed (see the decision drivers), and the author renames the object or adds an `EXEMPT` entry.

## Related

- ADR-0127 (p) (the four forms, amended), ADR-0127 (n) to (u) (the script and its exemptions)
- ADR-0069 (the index of `0006_operations.sql`)
- `data-model.md` §5.1 and §8, `reliability.md` §5 and §7.10
- Change `migration-rewrite-forms`, capability `operations`
