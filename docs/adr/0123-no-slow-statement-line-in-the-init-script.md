# ADR-0123: No slow-statement line in the init script

Status: Accepted. Amends ADR-0031
Date: 2026-11-04
Deciders: project owner

## Context

`deploy/db/initdb/01-roles.sh` creates the login roles with `CREATE ROLE ... PASSWORD`. psql puts the plain password from the secret file into the text of each statement.

ADR-0031 makes the script set two options in its session before the role statements:

- `log_statement = 'none'`, so the server writes no statement line,
- `log_min_error_statement = 'panic'`, so a failed statement writes no statement text.

The review of 2026-11-04 (finding D7-1) found a third path to the log:

1. The `command:` list of `db` in `deploy/compose.yaml` sets `log_min_duration_statement=1000` (`reliability.md` section 7.6).
2. The entrypoint of the postgres image passes these options to the temporary server that runs the init scripts. The option is therefore live while `01-roles.sh` runs.
3. A statement that runs for 1 s or more gets a line `duration: ... statement: CREATE ROLE ... PASSWORD '<plain>'`. A WAL flush stall on the first start can cause such a delay.
4. The json-file log of `db` keeps the line. A log collector can copy it off the host.

Sampling can also write statement text through `log_transaction_sample_rate`, or through `log_min_duration_sample` with `log_statement_sample_rate`. The init script must disable both sampling paths before each password statement.

`reliability.md` section 3.4 forbids a password in the log. The runbook steps that run `ALTER ROLE ... PASSWORD` already set `log_min_duration_statement = -1` (`docs/ops/runbook.md`, "Restore into production" step 6 and "Change a database password"). `deploy/ci/switch-steps.sh` does the same. Only the init script and the SQL block of `data-model.md` section 5.1 lack the setting.

## Decision drivers

- The log never holds a password (`reliability.md` section 3.4, R37).
- The init session prevents statement text in regular, error, duration, and sampling logs before each password statement.
- No change of a role attribute or a role membership, so ADR-0103 (m) needs no forward step.
- No new setting in `reliability.md` section 7.11.

## Considered options

### Option A: The script disables duration and sampling logs in its session

- Pros: three additional settings in the script and in `data-model.md` section 5.1. The runbook already disables duration logs.
- Cons: the settings act only on a new volume. An existing installation can already hold a line with a password.

### Option B: Remove `log_min_duration_statement` from the `command:` list of `db`

- Pros: no slow-statement line can hold any statement text.
- Cons: the database log loses the slow statements of the application. `reliability.md` section 7.6 requires them.

### Option C: Pass a password hash in place of the plain password

- Pros: the statement text holds no plain password.
- Cons: section 3.4 also forbids a password hash in the log. The script must compute a SCRAM verifier, which needs a new tool in the image.

## Decision

The project uses **Option A**. It amends ADR-0031.

- (a) `01-roles.sh` sets `log_min_duration_statement = -1` in its session, directly after `log_min_error_statement = 'panic'` and before the first `CREATE ROLE`. The next two statements set `log_transaction_sample_rate = 0` and `log_min_duration_sample = -1`.
- (b) The five `SET` statements come before the first statement that holds a password. `\connect` starts a new session without them. Thus no statement after a `\connect` can hold a password.
- (c) The SQL block of `data-model.md` section 5.1 gets the same three additional settings.
- (d) The runbook gets a step for an existing installation. The operator searches the log of `db` for the text `PASSWORD` followed by a quote or by `E` and a quote. psql writes the `E'...'` form for a value with a backslash. The operator rotates each password that a line shows, with the steps of "Change a database password".
- (e) A static test of `recipe-db` asserts the order of item (b). Each line with the token `PASSWORD :'`, matched with case, comes after the five `SET` lines and before the first `\connect`.
- (f) A storage test runs the script in its own container on a server with `log_min_duration_statement=0`. The container has its own label, so the cleanup of the storage harness does not remove it. The test waits for the final TCP login and for the roles of the script. It asserts at least one `duration:` line, so the option was live. It asserts that no line holds `PASSWORD '`, `PASSWORD E'`, or a test password of a login role.

## Consequences

### Positive

- A slow or sampled role statement on a new volume writes no password into the log.
- The init session uses all five options, even when the operator enables sampling on the server. The restore scripts send passwords and password verifiers only to temporary servers without the option `log_min_duration_statement`.

### Negative

- The fix acts only on a new volume. The operator must check an existing log by hand once.
- A slow role statement in the init script leaves no slow-statement line, so the log does not show the delay.

### Risks and mitigations

- Risk: a later edit adds a password statement after a `\connect`. Mitigation: the static test of item (e) fails on a password statement outside the session of the five options.

## Related

- ADR-0031 (secret files and the two options), ADR-0103 (m) (forward step of a role change), ADR-0033 (password steps of the runbook)
- `reliability.md` sections 3.4 and 7.6, `data-model.md` section 5.1
- `overview.md` section 7: R34, R37. Section 9: A1
