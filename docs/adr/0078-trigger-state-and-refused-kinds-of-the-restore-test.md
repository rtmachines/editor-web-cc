# ADR-0078: Trigger state and refused kinds of the restore test

Status: Accepted. Amended by ADR-0084 (d) (a mitigation of the `DEFAULT` risk). Amended by ADR-0095 (g) (the words "so no code of the dump runs" apply only to a stop on a refused kind)
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0076 adds the triggers and the event triggers to `restore-code.expected`. The review of the script `restore-test` found three gaps:

1. ADR-0076 names a residual risk. `pg_get_triggerdef` does not show the enabled state of a trigger. A dump with `ALTER TABLE ... DISABLE TRIGGER` therefore gives the same hash. A disabled trigger such as `audit_log_hash_chain` turns off a protection after a restore into production.
2. The query `restore-code.sql` selects only the functions with `prokind` `f` and `w`. A procedure and an aggregate of the dump get no line. The name list of step 2 covers only the entries `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER`.
3. A materialized view runs its query during the restore of `post-data`. An operator calls a function. No check of ADR-0067 or ADR-0076 covers these objects.

## Decision drivers

- The project gives data safety priority over performance. A dump that holds unknown code never restores without an alarm (R31).
- A failed step writes one `ERROR` line and stops (R35).
- A dump is external input. The test checks each object of the dump against the release before the data load (R38).
- No new setting, no new row of `reliability.md` section 7.11, and no new error code.

## Considered options

### Option A: The enabled state in the line of a trigger, and a stop on four kinds of entries

- Pros: one changed definition and one more check of step 2. The release has none of the four kinds, so no release dump stops.
- Cons: every trigger line of `restore-code.expected` gets a new hash.

### Option B: Lines with a hash for procedures, aggregates, materialized views, and operators

- Pros: a later migration can add one of these objects without a new ADR.
- Cons: four more kinds in the query and in the storage test, with no object in the release. The query of a materialized view runs during the restore of `post-data`, before the comparison after `post-data`.

### Option C: No change

- Pros: no work.
- Cons: a disabled trigger and the four kinds stay without a check.

## Decision

The project uses **Option A**.

Trigger state (user decision):

- (a) The definition of a trigger in `restore-code.sql` is the text of `pg_get_triggerdef` and the value of `tgenabled`, with a line feed between them. This item amends ADR-0076 (a).
  - The query of an event trigger already holds `evtenabled` (ADR-0076 (a)). Its line does not change.
  - `dumpTrigger` of `pg_dump` 16 writes the `ALTER TABLE ... DISABLE TRIGGER`, `ENABLE REPLICA TRIGGER`, or `ENABLE ALWAYS TRIGGER` into the same entry `TRIGGER` in `post-data`.
  - The comparison after `post-data` in step 5 therefore finds a changed state before the row counts. This closes the residual risk of ADR-0076.
  - No migration and no line of `01-roles.sh` changes the state of a trigger. Every trigger of the release has the state `O`.

Refused kinds (user decision):

- (b) Step 2 stops the test on each entry of `pg_restore -l` of the kind `PROCEDURE`, `AGGREGATE`, `MATERIALIZED VIEW`, or `OPERATOR`. This item amends ADR-0076 (b).
  - The kinds are the texts of `.description` in `pg_dump.c` of `REL_16_STABLE`. `dumpFunc` uses `PROCEDURE` for a procedure. `dumpAgg` uses `AGGREGATE`, `dumpOpr` uses `OPERATOR`, and `dumpTableSchema` uses `MATERIALIZED VIEW`.
  - An entry matches a kind when the text after the two object ids starts with the kind and one space.
  - The stop comes before the restore of the globals file, so no code of the dump runs.
  - No file of `migrations/` and no line of `deploy/db/initdb/01-roles.sh` creates a procedure, an aggregate, a materialized view, or an operator.
  - A later migration with one of these kinds needs a new ADR.

Note (not a decision):

- `pg_dump.c` writes `OPERATOR CLASS`, `OPERATOR FAMILY`, and `MATERIALIZED VIEW DATA` as separate kinds. The match of (b) also stops the test on these kinds, because each one starts with a refused kind and one space.
  - The project creates none of them. A later ADR can name them as own kinds.

## Consequences

### Positive

- A disabled, a replica, or an always trigger in the dump stops the test before the row counts.
- A procedure, an aggregate, a materialized view, or an operator in the dump stops the test before the restore.

### Negative

- The storage test of ADR-0067 (j) fails until `restore-code.expected` holds the new trigger hashes.

### Risks and mitigations

- Risk: a dump holds an index expression, a `DEFAULT` expression, or a setting of a role or a database that calls code. Mitigation: none in this ADR. This ADR names the risk as a residual risk.

## Related

- Amends ADR-0076 (a) (the definition of a trigger) and (b) (the entries that step 2 refuses).
- ADR-0034 (the event trigger `audit_objects_dropped`), ADR-0067 (restore test), ADR-0069 (the init script of the storage tests).
- `docs/architecture/reliability.md` section 7.7 (steps 2 and 5 of the restore test)
- `data-model.md` sections 4 and 5.1 (the triggers and the event trigger of the release)
- Source `src/bin/pg_dump/pg_dump.c` of `REL_16_STABLE` (`dumpFunc`, `dumpAgg`, `dumpOpr`, `dumpTableSchema`, `dumpTrigger`).
- Baseline requirement: `operations` "Restore test".
- `overview.md` section 7: R31, R35, R38. Section 9: A16
