# ADR-0095: Role grammar of the globals file in the restore test

Status: Proposed
Date: 2026-10-25
Deciders: project owner

## Context

The security audit of step P4.2 found a path around the code checks of the restore test:

1. `check_globals` in `deploy/pgtools/bin/restore-test` refuses every psql meta-command. It does not refuse plain SQL between `\restrict` and `\unrestrict`.
2. Step 3 runs the globals file with `psql` as the superuser of the scratch cluster. The comparison of step 4 comes after this step.
3. The manifest lies on the backup disk. A writer of that disk can replace a set and compute a new manifest.
4. A line such as `COPY (SELECT 1) TO PROGRAM '<command>';` then starts a shell as uid 999 in the service `backup`.
5. That shell can read the secret `backup_password` and can reach `db` over the network `backend`.

The first risk of ADR-0067 says that step 4 stops a changed definition "before that code runs". ADR-0078 (b) says that "no code of the dump runs". The globals file runs before these checks.

ADR-0084 accepts the code of a generated column in `pre-data`. Its attacker holds the migrator credential. ADR-0084 does not cover the globals file.

The globals file of the release holds only the role statements of `01-roles.sh` (ADR-0033). `pg_dumpall` 16 writes each of these statements on one line.

## Decision drivers

- No SQL of a backup set runs before a check of that SQL (R31, R38).
- The restore test still restores the real globals file, so it proves the roles of the set (ADR-0033, A16).
- The gate stays a shell check. It needs no new tool, no new secret, and no new service.

## Considered options

### Option A: A closed grammar of role statements in step 1

- Pros: the gate refuses every statement outside a short list before any tool runs. The test still restores the real file. The stack test of CI runs the gate on a real file of `pg_dumpall` 16.
- Cons: a new role statement in `01-roles.sh` needs a change of the grammar.

### Option B: An exact list of the expected lines in a file of the image

- Pros: the strictest check of the content.
- Cons: the file must equal the output of `pg_dumpall` byte for byte. A storage test cannot build that output without the tool. A new option of a role changes the file.

### Option C: Create the roles from a fixed script and skip the globals file

- Pros: no SQL of the set runs in step 3.
- Cons: the test no longer proves that the globals file of the set restores.

### Option D: An HMAC of the manifest with a key outside the backup disk

- Pros: it also stops a crafted entry of the dump file.
- Cons: a new secret, a change of `backup-once`, and a procedure for a new key. The user decision limits this step to the globals file.

## Decision

The project uses **Option A**.

- (a) Step 1 reads each line of the globals file. It applies three checks to each line in this order:
  1. A line with a backslash is the restrict pair or a psql meta-command. This branch comes before the grammar, so a meta-command keeps its `ERROR` line.
  2. A line with a control character other than a tab stops the test. This check also runs between `\restrict` and `\unrestrict`, before the comment check.
  3. Between the pair, a line is a comment, a blank line, or one whole statement of the grammar of (b).
  - Any other line stops the test before any PostgreSQL tool runs.
  - Reason for check 2: PostgreSQL ends a comment at a carriage return (CR). Text after a CR in a comment line therefore runs as SQL.
- (b) The grammar has five forms. `<role>` matches `^[a-z_][a-z0-9_]*$`, the role grammar of the script. Each statement ends with `;` and fills the whole line. The keywords are upper case.
  1. `SET default_transaction_read_only = off;`, `SET client_encoding = 'UTF8';`, or `SET standard_conforming_strings = on;`.
  2. `CREATE ROLE <role>;`.
  3. `ALTER ROLE <role> WITH` and one or more of the attributes `[NO]SUPERUSER`, `[NO]INHERIT`, `[NO]CREATEROLE`, `[NO]CREATEDB`, `[NO]LOGIN`, `[NO]REPLICATION`, and `[NO]BYPASSRLS`.
  4. `ALTER ROLE <role> SET <name> TO '<value>';`. `<name>` is `log_error_verbosity` or `log_parameter_max_length`. `<value>` matches `^[a-z0-9]+$`.
  5. `GRANT <granted> TO <role>`, an optional `WITH` list, and an optional `GRANTED BY <role>`. `<granted>` is `audit_owner`, `pg_read_all_data`, or `pg_read_all_stats`, the granted roles of `01-roles.sh`. The list holds `ADMIN OPTION`, `INHERIT TRUE`, `INHERIT FALSE`, `SET TRUE`, or `SET FALSE`, separated by `, `.
- (c) A line outside the grammar gives the `ERROR` line "the globals file holds a statement outside the role grammar". The line names the line number. It holds no text of the file.
- (d) The grammar refuses `PASSWORD`, `CONNECTION LIMIT`, `VALID UNTIL`, `COPY`, `DO`, a function, a tablespace, and every other statement.
  - A value with a quote or a backslash cannot match.
  - Two statements on one line, text after the `;`, and one statement on two lines cannot match.
  - A quoted identifier, a lower-case keyword, and a role name with a hyphen cannot match.
- (e) A new role statement in `01-roles.sh` needs a change of the grammar in the same change. The stack test of CI runs `restore-test` on a real set (ADR-0070). A missing form therefore fails CI before the release.
- (f) This item amends the first risk of ADR-0067. Step 1 refuses each statement of the globals file outside the grammar of (b). Step 3 runs the globals file only after this check.
- (g) This item amends ADR-0078 (b). The words "so no code of the dump runs" apply to the stop on a refused kind. Without a refused kind, step 3 runs the globals file after the check of (a).

Known limit:

- (h) This ADR does not stop a crafted entry of the dump file. `pg_restore` runs the definition text of each entry of the dump.
  - Step 2 compares only the names of the entries `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER`.
  - A writer of the backup disk can put SQL into the definition of an entry of another kind, for example `TABLE`.
  - That SQL runs as the scratch superuser in `pre-data`, before the comparison of step 4.
  - The project owner decides on this path in a later ADR. Option D of this ADR is one candidate.
  - A second candidate: `pg_restore --file` writes the pre-data and post-data script without a run. A comparison with a file that CI generates then needs no key.
  - This ADR therefore closes the path of the globals file only. A crafted set can still run SQL through another entry.

Grammar file:

- (i) One file holds the grammar: `deploy/pgtools/lib/globals-grammar.sh`.
  - The file holds the regular expressions of (b) and the function `check_globals_file`. The function returns a fault and the line number. It does not stop the shell.
  - `restore-test` sources the file. The `Dockerfile` of `recipe-pgtools` copies it into the image.
  - A storage test of `recipe-db` pipes the real output of `pg_dumpall --globals-only --no-role-passwords` through the same function. No second copy of the grammar exists.

## Consequences

### Positive

- The globals file of a set runs only after a check of each statement.
- The restore test still proves the roles and the logins of the set.
- The documents state the real order of the checks and of the SQL of the globals file.

### Negative

- A new role statement needs a change of the grammar and of its tests.
- The known limit of (h) stays open until a later ADR.

### Risks and mitigations

- Risk: `pg_dumpall` of a later minor release writes a form outside the grammar. Mitigation: the stack test of CI runs the real restore test, and the image pins the PostgreSQL release (ADR-0064).
- Risk: an allowed form gives the attacker a new path. Mitigation: no allowed form runs a program, reads a file, or loads a library. The setting names of form 4 and the granted roles of form 5 are closed lists.
- Risk: an operator adds a role by hand with a setting outside the grammar, for example `CONNECTION LIMIT`. The weekly restore test then stops. Mitigation: the `ERROR` line names the line number, and the runbook has an entry for "statement outside the role grammar".
- Risk: a comment hides SQL behind a carriage return. Mitigation: check 2 of (a) runs before the comment check, also between the pair.

## Related

- Amends the first risk of ADR-0067 and ADR-0078 (b).
- ADR-0033 (globals dump), ADR-0066 (sets on the backup disk), ADR-0070 (stack test), ADR-0084 (generated column in `pre-data`).
- `docs/architecture/reliability.md` section 7.7 (steps 1 and 3 of the restore test).
- `deploy/pgtools/bin/restore-test` (`check_globals`, `restore_globals`), `deploy/pgtools/lib/globals-grammar.sh`.
- `docs/ops/runbook.md` (entry "statement outside the role grammar").
- `deploy/db/initdb/01-roles.sh` (the role statements of the release).
- Baseline requirement: `operations` "Restore test".
- `overview.md` section 7: R31, R38. Section 9: A16.
