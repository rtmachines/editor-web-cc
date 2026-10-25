# ADR-0076: Name list of the restore test

Status: Accepted. Amended by ADR-0078 (enabled state of a trigger, refused kinds of entries)
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 writes the script `restore-test` and the query `deploy/pgtools/restore-code.sql`. ADR-0067 leaves one list open:

1. Step 2 of ADR-0067 compares the entries `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER` of `pg_restore -l` with "a list that the release builds". No document gives the file or the format of that list.
2. ADR-0067 (i) limits `restore-code.expected` to functions, `CHECK` constraints, and generated columns. The file holds no trigger and no event trigger.
3. Step 4 of ADR-0067 compares the file after the restore of `pre-data`. `pg_dump` puts each trigger and each event trigger into `post-data`, so step 4 cannot see them.

A trigger runs code of the dump on each write after a restore into production. The event trigger `audit_objects_dropped` protects the audit objects (ADR-0034). A changed trigger therefore needs the same check as a changed function.

## Decision drivers

- The project gives data safety priority over performance. A dump that holds unknown code never restores without an alarm (R31).
- A failed step writes one `ERROR` line and stops (R35).
- A dump is external input. The test checks each object of the dump against the release before the data load (R38).
- Each list has one normative place and one storage test, so the list cannot drift from `migrations/`.
- No new setting, no new row of `reliability.md` section 7.11, and no new error code.

## Considered options

### Option A: Triggers and event triggers in `restore-code.expected`, and the list of step 2 from that file

- Pros: one query, one file, and one storage test hold all code of the dump. The hash also finds a changed trigger, not only an unknown name.
- Cons: the file holds two kinds that step 4 cannot compare. The script needs a second comparison after `post-data`.

### Option B: A new file `restore-entries.expected` with a second storage test

- Pros: each file has one purpose.
- Cons: two files and two storage tests hold overlapping names. The names of the functions are in both files and can drift. The list holds no hash, so a changed trigger passes.

### Option C: No name list, only the code check of ADR-0067 (i)

- Pros: the smallest change.
- Cons: no check covers a trigger or an event trigger. A planted trigger runs on the first write after a restore into production.

## Decision

The project uses **Option A**.

Lines of the file:

- (a) `restore-code.sql` also builds one line for each trigger and each event trigger outside `pg_catalog` and `information_schema`. This item amends ADR-0067 (i).
  - The query selects each trigger with `NOT tgisinternal`. The query of `pg_dump` 16 uses the same filter.
  - A line holds the kind, the name, the owner, and the SHA-256 of the definition, in the form of ADR-0067 (i).
  - The definition of a trigger is the text of `pg_get_triggerdef`. The hash therefore also covers the `WHEN` clause.
  - The owner of a trigger is the owner of its table. `pg_dump` writes the same owner into the entry of a trigger.
  - The definition of an event trigger holds `evtevent`, `evttags`, `evtfoid` as `regprocedure`, and `evtenabled` of `pg_event_trigger`.

Name list of step 2:

- (b) The list of step 2 is the set of the names of the lines of kind function, trigger, and event trigger of `restore-code.expected`. This item amends ADR-0067 step 2.
  - The storage test of ADR-0067 (j) compares that file with the harness database after `01-roles.sh` and every migration. This file is therefore the list that the release builds.
  - The name of a line identifies the object as the entry of `pg_restore -l` does: the schema and the tag. The tag of a trigger holds the table and the trigger name.
  - An entry `FUNCTION`, `TRIGGER`, or `EVENT TRIGGER` of `pg_restore -l` whose name is not in the list stops the test.

Comparisons by section:

- (c) Step 4 compares only the kinds that the section `pre-data` holds: function, `CHECK` constraint, and generated column. This item amends ADR-0067 step 4.
  - After the restore of `post-data` in step 5, the script compares the lines of every kind with the file.
  - A missing, extra, or changed line stops the test before the row counts of step 6.

Facts of PostgreSQL 16:

- (d) The documentation of `pg_dump` and the source of `pg_dump` 16 give these sections. The rule of (c) agrees with each fact.
  - The text of the option `--section` puts indexes, triggers, rules, and constraints into `post-data`. The exception is a `CHECK` constraint without `NOT VALID`. It stays in `pre-data`.
  - The same text puts every other data definition item into `pre-data`. A function is therefore in `pre-data`.
  - `dumpTableSchema` writes a generated column as `GENERATED ALWAYS AS (...) STORED` inside `CREATE TABLE`. A generated column is therefore in `pre-data`.
  - `dumpConstraint` writes a `CHECK` constraint that it dumps separately, for example a constraint `NOT VALID`, as an entry `CHECK CONSTRAINT` in `post-data`. Step 4 then finds a missing line, as ADR-0067 (k) says.
  - `dumpTrigger` writes each trigger, also a constraint trigger, as an entry `TRIGGER` in `post-data`.
  - The documentation names no section for an event trigger. `dumpEventTrigger` writes it as an entry `EVENT TRIGGER` in `post-data`.
  - `pg_dump_sort.c` restores the triggers "after all data modifications" and the event triggers "next-to-last".
  - The section `post-data` of this project writes no row. The checks of the foreign keys read rows and run no trigger of the project.
  - The only event trigger fires on `sql_drop`. `post-data` holds no `DROP` statement, because the script uses no `--clean`. No trigger and no event trigger of this project therefore runs during the restore of `post-data`.

## Consequences

### Positive

- One file and one storage test hold every name and every hash of the code of the dump.
- The test finds an unknown trigger in step 2, before the restore. It finds a changed trigger after step 5, before the row counts.
- A new migration with a trigger changes the file, and the storage test fails CI until the file holds the new line.

### Negative

- The script compares the file twice: the kinds of `pre-data` in step 4, and every kind after step 5.
- A changed trigger stops the test only after the data load. No trigger exists during the data load, so this order runs no code of the trigger.

### Risks and mitigations

- Risk: a later `pg_dump` moves a kind to another section. Mitigation: a missing line in step 4 or an extra line after step 5 stops the test. The test then stops before the row counts.
- Risk: `pg_get_triggerdef` does not show the enabled state of a trigger. A dump with `ALTER TABLE ... DISABLE TRIGGER` gives the same hash. Mitigation: none in this ADR. This ADR names the risk as a residual risk.

## Related

- Amends ADR-0067 (i) (the trigger lines), step 2 (the name list), and step 4 (the kinds of `pre-data`).
- ADR-0034 (the event trigger `audit_objects_dropped`), ADR-0066 (file sets), ADR-0069 (the init script of the storage tests).
- `docs/architecture/reliability.md` section 7.7 (steps 2, 4, and 5 of the restore test)
- `data-model.md` sections 4 and 5.1 (the triggers and the event trigger of the release)
- PostgreSQL 16 documentation of `pg_dump` (option `--section`). Source `src/bin/pg_dump/pg_dump.c` and `pg_dump_sort.c` of `REL_16_STABLE`.
- Baseline requirement: `operations` "Restore test".
- `overview.md` section 7: R31, R35, R38. Section 9: A16
