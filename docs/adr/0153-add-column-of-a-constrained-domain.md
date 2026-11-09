# ADR-0153: Add column of a constrained domain in the migration check

Status: Accepted. Amends ADR-0136 (a) (the table of forms) and (b) (the exemptions), and ADR-0127 (p) (the form volatile default)
Date: 2026-11-09
Deciders: project owner

## Context

`data-model.md` §8 forbids a migration that rewrites a table or scans it under an exclusive lock. ADR-0127 (n) to (u) and ADR-0136 set the script `deploy/ci/migration_rewrite.py`. The script reports 18 forms.

The full repo review of 2026-11-08 found that the script passes `ADD COLUMN` with a domain type that has a constraint (finding D4-1). These facts shape the decision:

- The review probe used PostgreSQL 16.15 and a table of 1000 rows. `ADD COLUMN` of a plain `INTEGER` kept the file node of the table.
- `ADD COLUMN` of a domain with a `CHECK` and no `DEFAULT` changed the file node. PostgreSQL thus rewrote the table under an `ACCESS EXCLUSIVE` lock.
- PostgreSQL 16 tests each domain constraint against the value of the new column in each row. A `NOT NULL` of a domain is a domain constraint, like a `CHECK`.
- A domain over a domain also has the constraints of its base domain.
- PostgreSQL 16 uses the default of the domain when the added column has no `DEFAULT` of its own. The function `build_column_default` falls back to the type default.
- A volatile domain default thus also rewrites the table. A domain without its own `DEFAULT` copies the default of its base domain.
- An array of a domain is not a domain. PostgreSQL 16 tests no domain constraint and uses no domain default for an array column.
- `volatile_default` and `added_forms` never read the type of an added column.
- The script keeps a set `domains` for each file. The set holds the names of the domains that the file creates. It holds no constraints, and the script clears it at the start of each file.
- No shipped migration from `0001` to `0011` creates a domain.

## Decision drivers

- Each form has one stable diagnostic name (ADR-0136).
- A gap of the script gives a false alarm, never a silent miss (ADR-0136).
- The cost of a rewrite comes from the rows of the table. The age of the domain does not change it.
- The script stays small. It reads no catalog and needs no database.
- No new dependency and no new external binary in CI.

## Considered options

### Option A: A new form, and a set of constrained domains over all files

- Pros: the probe of the review fails CI. A domain without a constraint passes. The diagnostic names the exact form.
- Cons: the script keeps one more set across the files.

### Option B: A known passing form in the Risks of ADR-0136

- Pros: no code.
- Cons: a migration can rewrite a large table, and CI passes. This breaks `data-model.md` §8.

### Option C: Reject each `ADD COLUMN` with any domain type

- Pros: the script needs no constraint state.
- Cons: a domain without a constraint gives a false alarm. Each such migration needs an `EXEMPT` entry with an ADR.

## Decision

The project uses **Option A**. The items below are the rules. This ADR amends ADR-0136 (a) and (b), and ADR-0127 (p) for item (i). The other items of ADR-0136 and ADR-0127 (n) to (u) stay in force for the new form.

- (a) The script reports one more form with the name `add column of a constrained domain`. It prints `file:line: add column of a constrained domain`.
  - The statement is `ALTER TABLE ... ADD [COLUMN] [IF NOT EXISTS] name type`, where `type` names a constrained domain.
  - The form counts with and without a `DEFAULT`, and with and without an array suffix.
  - The form counts in addition to the other forms of the same action, for example `volatile default`.
- (b) A domain is constrained when one of these conditions is true:
  - Its `CREATE DOMAIN` holds a `CHECK` or a `NOT NULL` outside parentheses. A `NULL`, a `DEFAULT`, and a `COLLATE` are no constraints. A `NOT NULL` after `IS` is part of an expression.
  - Its base type is a constrained domain.
  - An earlier statement ran `ALTER DOMAIN ... ADD [CONSTRAINT n] CHECK` on it, also with `NOT VALID`.
  - An earlier statement ran `ALTER DOMAIN ... SET NOT NULL` on it.
- (c) The script keeps the set of constrained domains over all files, as it keeps the tables.
  - `ALTER DOMAIN ... RENAME TO` and `ALTER DOMAIN ... SET SCHEMA` move the domain in the set.
  - `ALTER TYPE ... RENAME TO` and `ALTER TYPE ... SET SCHEMA` also work on a domain. They move the domain in the set in the same way.
  - `DROP DOMAIN` removes the domain from the set.
  - `ALTER DOMAIN ... DROP CONSTRAINT` and `DROP NOT NULL` keep the domain in the set. The script does not count the constraints of a domain. A false alarm can follow, and the check fails closed.
- (d) The exemption of ADR-0127 (q) and of the first sentence of ADR-0136 (b) applies to the new form. A table that an unconditional `CREATE TABLE` in the same file created is exempt.
  - A domain that an unconditional `CREATE DOMAIN` in the same file created gives no exemption for the new form. The rows of the table decide the cost, not the age of the domain.
  - The domain exemption of ADR-0136 (b) stays for the forms `domain constraint without not valid` and `domain validate`.
- (e) The script resolves a domain name with the rules of ADR-0127 (o). An unqualified name resolves to `public`. A quoted name keeps its case.
- (f) The form also counts inside a function body and inside an `EXECUTE` string, as ADR-0127 and ADR-0136 (c) say.
  - The statements of one body share one copy of the sets of items (c) and (i). The copy starts from the sets of the file.
  - A domain that a body creates thus counts for a later statement of the same body. A body statement never changes the sets of the file.
- (g) An `EXEMPT` entry with an ADR permits the form for one file, as for each other form.
- (h) `ALTER COLUMN ... TYPE` with a domain stays the form `column type change` of ADR-0127 (p).
- (i) The script reports the form `volatile default` of ADR-0127 (p) also for a domain with a volatile default.
  - The statement is `ALTER TABLE ... ADD [COLUMN] [IF NOT EXISTS] name type` without a `DEFAULT` of its own. `type` names a domain with a volatile default.
  - A domain has a volatile default when its `CREATE DOMAIN` holds a `DEFAULT` that the rule of ADR-0127 (p) calls volatile.
  - A domain without its own `DEFAULT` has a volatile default when its base domain has one.
  - `ALTER DOMAIN ... SET DEFAULT` with a volatile expression gives the domain a volatile default.
  - `ALTER DOMAIN ... DROP DEFAULT` and a `SET DEFAULT` that is not volatile keep the domain in the set, as item (c) keeps a constraint. The check fails closed.
  - The script keeps this set over all files. Items (c), (d), (e), and (f) apply to it in the same way.
  - A column with its own `DEFAULT` follows the rule of the column default of ADR-0127 (p). A domain default that is not volatile gives no form.

## Consequences

### Positive

- The migration of the review probe fails CI on the pull request.
- An added column of a domain with a volatile default fails CI, also without a column `DEFAULT`.
- A domain without a constraint passes, so a plain name alias needs no `EXEMPT` entry.
- The shipped files `0001` to `0011` stay valid, because no file creates a domain.

### Negative

- A column of a constrained domain on an existing table always fails. The safe path uses the base type. A table `CHECK ... NOT VALID` and a later `VALIDATE CONSTRAINT` then hold the rule.
- A domain that lost its constraints still counts. Its migration needs an `EXEMPT` entry with an ADR, or a new domain without a constraint.
- `data-model.md` §8 changes in the same change as this ADR.

### Risks and mitigations

- Risk: a later major version of PostgreSQL may add the column without a rewrite in some cases. Mitigation: the form then gives a false alarm only. A new ADR can narrow the rule after a probe.
- Risk: a domain that no migration created gives no form. Examples are a domain of `information_schema` and a domain of an extension. Mitigation: the review of a migration checks each column type outside `public`.
- Risk: PostgreSQL resolves an unqualified type name in `pg_catalog` before `public`. A domain with the name of a built-in type thus gives a false alarm. Mitigation: the check fails closed, and the author renames the domain.
- Risk: an array of a constrained domain, or of a domain with a volatile default, gives a false alarm. PostgreSQL 16 does not rewrite the table for it. Mitigation: the check fails closed. The author uses the base type or an `EXEMPT` entry with an ADR.
- Risk: the splitter misreads a new type form. Mitigation: one fixture for each condition of items (b) and (i) and each syntax variant of item (a).

## Related

- ADR-0136 (a) and (b) (the table of forms and the exemptions, amended).
- ADR-0127 (p) (the form volatile default, amended).
- ADR-0127 (n) to (u) (the script and its exemptions).
- `data-model.md` §8, `reliability.md` §5
- Finding D4-1 of `docs/plan/review/2026-11-08-full-repo-review.md`
- Change `test-gates-round-2`, capability `operations`
