# Proposal

## Why

The security audit of step P4.2 found seven weak points (F1 to F7 in `target/p42-agents/security-findings.md`). Three of them change behavior that the specs state. The change satisfies R20, R28, R29, R31, and R38.

The change uses A9 and A16.

## What Changes

- F1. The restore test checks each line of the globals file against a closed role grammar before any PostgreSQL tool runs. The control-character check also runs inside the restrict pair. One grammar file serves the gate and its tests (ADR-0095). The change narrows F1 and does not close it (ADR-0095 (h)).
- F2. `SetUserRole` and `DeactivateUser` run in one `READ COMMITTED` transaction after a class-4 advisory lock. They have no retry. A target that sends requests can no longer stop the command (ADR-0096).
- F3. The drop guard of the audit objects compares names without the schema. A rename of `public` no longer opens a path to drop an audit object (ADR-0097). An older volume gets the new guard during the upgrade, after `migrate` and before the first new backup.
- F4. `ChangeOwnPassword` and `Logout` call `authorize` as the first statement. A source test of the identity use cases keeps the rule.
- F5. CLAUDE.md, `.claude/rules/sql.md`, `AGENTS.md`, and `data-model.md` section 6 name `ChangeOwnPassword` as the second command with two transactions (ADR-0035 (b)).
- F6. `recipe_app` inserts only eleven columns of `audit_log`. The database sets `id`, `occurred_at`, and the three chain columns (ADR-0098).
- F7. A forward migration qualifies the operators and types of `audit_hash_chain()` and `security_event_purge()`. The code review adds `security_event_guard()`, because `IS DISTINCT FROM` also uses a search-path lookup. It keeps the grouping of each expression and the attributes of both functions (ADR-0098).
- ADR-0015, ADR-0022, ADR-0034, ADR-0035, ADR-0067, and ADR-0078 get "Amended by" notes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity-access`: the isolation and the retry of `SetUserRole` and `DeactivateUser`, the wait for the guard lock, and their result while the target sends requests.
- `audit`: the columns that the database sets in a new audit entry, and the drop guard after a schema rename.
- `operations`: the role grammar of the globals file in the restore test, and the guard function of an older volume during the upgrade.

## Impact

- Crates: `recipe-app` (identity use cases, unit tests, memory adapter, source test), `recipe-domain` (one doc comment), and `recipe-db` (guard lock, race tests, grant and storage tests).
- Migrations: `0007_audit_definer_operators.sql` and `0008_audit_log_insert_columns.sql`.
- Deploy: `deploy/db/initdb/01-roles.sh`, `deploy/pgtools/bin/restore-test`, `deploy/pgtools/lib/globals-grammar.sh`, `deploy/pgtools/Dockerfile`, `deploy/pgtools/restore-code.expected`, the restore test cases, and a CI job that upgrades an older volume.
- Documents: four new ADRs, `data-model.md`, `domain-model.md`, `reliability.md`, `overview.md`, `api.md`, `docs/ops/runbook.md`, and `CHANGELOG.md`.
- Instruction files: CLAUDE.md, `.claude/rules/sql.md`, and `AGENTS.md`. `design.md` gives each new line, and the project owner approves the exact text.
- No new error code. No new setting. No new dependency.

## Out of scope

- A crafted entry of the dump file that runs SQL in `pre-data` (ADR-0095 (h)). The project owner decides on it in a later ADR.
- The code of a generated column in `pre-data`. ADR-0084 accepts this risk.
- The per-login throttle limit of ADR-0035 (code review M9).
- The soak test, the coverage gate, the accessibility audit, and the Playwright suites of step P4.2.
- `create-admin`: it stays `SERIALIZABLE` with one more run.
