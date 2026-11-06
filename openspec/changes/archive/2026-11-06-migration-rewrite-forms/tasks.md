# Tasks: migration-rewrite-forms

Each task names its agent. One task goes to one agent. The decisions D1 to D8 are in `design.md`.

## 1. ADR and documents

- [x] 1.1 (docs/adr) Write ADR-0136 with the decisions D1 to D7 of `design.md`. Copy the shape of ADR-0127. Then set the Status line of ADR-0127 to "Accepted. Amended by ADR-0136". Add the row of ADR-0136 to the index of `docs/adr/README.md` and change the row of ADR-0127. Add ADR-0136 to the decision map row of the test gates. Check: `ste-lint.py` reports 0 hard violations on the three files. Agent: Sonnet high.
- [x] 1.2 (docs/architecture) In `data-model.md` §8, list the 14 forms of D1 next to the current four and cite ADR-0136. In `reliability.md` §5, replace the list of forms with a link to `data-model.md` §8. In `data-model.md` §5.1 and `reliability.md` §7.10 item 7, add the `EXEMPT` entry of D6 to the migration that records a runbook index. Check: `ste-lint.py` on both files reports 0 hard violations. Agent: Sonnet high.
- [x] 1.3 (docs/ops) [mechanical] In `docs/ops/runbook.md`, next to "The next migration records the same index", add one sentence. The sentence names the `EXEMPT` entry with an ADR (D6, `data-model.md` §5.1). Check: `ste-lint.py docs/ops/runbook.md` reports 0 hard violations. Agent: Haiku.
- [x] 1.4 (CHANGELOG.md) [mechanical] Add one `### Changed` line under `[Unreleased]`. The line says that the CI check of the migrations rejects 14 more forms (ADR-0136). Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations. Agent: Haiku.

## 2. Rewrite check

- [x] 2.1 (deploy/ci) Change the old tests and add the failing new tests in `deploy/ci/test_migration_rewrite.py`. Check: `python3 -I -B deploy/ci/test_migration_rewrite.py` and `python3 -I -B deploy/ci/test_migration_rewrite_forms.py` fail on each new test and on the inverted tests, and pass on every other test. Agent: Codex gpt-6.1-sol high.
  - Old tests: change the constant `CHECK` to `... NOT VALID` and follow it with `ALTER TABLE recipe VALIDATE CONSTRAINT note_nn`. Change `test_stored_generated_column_stays_out_of_scope` to expect the form `stored generated column`. Change the test with an inline `CHECK` in `ADD COLUMN` to expect `constraint without not valid`. Read each test that adds a constraint or an index to a table of an earlier file. Make its fixture safe, or make its expectation match D1.
  - One fixture for each form of D1 on a table or domain of an earlier file. Each fixture fails with the name of D1.
  - The syntax variants: `ADD CHECK` and `ADD CONSTRAINT n CHECK`, `ADD FOREIGN KEY` and `ADD CONSTRAINT n FOREIGN KEY`, and inline `CHECK`, `REFERENCES`, `UNIQUE`, and `PRIMARY KEY` in `ADD COLUMN`. `CREATE UNIQUE INDEX`, `CREATE INDEX IF NOT EXISTS`, an index without a name, and `ON ONLY`. `VACUUM FULL t`, `VACUUM (FULL) t`, `VACUUM (VERBOSE, FULL) t`, `VACUUM FULL` without a table, and `CLUSTER` with and without a table. `SET LOGGED`, `SET UNLOGGED`, `SET ACCESS METHOD`, `LOCK recipe`, and `LOCK TABLE ONLY recipe IN SHARE MODE`. `ALTER DOMAIN ... VALIDATE CONSTRAINT` (form `domain validate`).
  - The passing forms: `CHECK ... NOT VALID`, `FOREIGN KEY ... NOT VALID`, `UNIQUE USING INDEX`, `CREATE INDEX CONCURRENTLY`, `VACUUM` without `FULL`, `VACUUM (ANALYZE) t`, `VACUUM (FULL false) t`, and `ALTER DOMAIN ... NOT VALID`. `ALTER DOMAIN ... VALIDATE CONSTRAINT` on a domain of an earlier file fails. `CHECK (NOT valid)` without `NOT VALID` after the expression fails.
  - A column with the name `cluster`, `vacuum`, or `lock` in a function body gives no form.
  - Each new form on a table or domain that an unconditional `CREATE TABLE` or `CREATE DOMAIN` in the same file created passes. The same after `CREATE TABLE IF NOT EXISTS` of a table of an earlier file fails. `VACUUM FULL` and `CLUSTER` without a table name fail also after a `CREATE TABLE`.
  - A new form inside a `$$` function body and inside an `EXECUTE` string of a body fails.
  - The shipped files `0001` to `0010` pass with the entry of D6 and fail without it on `0006_operations.sql:10`.
- [x] 2.2 (deploy/ci) Change `deploy/ci/migration_rewrite.py` to D1 to D7, with the 14 forms of D1 (also `domain validate`). Add the entry of D6 to `EXEMPT` and update the docstring. Check: `python3 -I -B deploy/ci/test_migration_rewrite.py` and `python3 -I -B deploy/ci/test_migration_rewrite_forms.py` pass, and `python3 -I -B deploy/ci/migration_rewrite.py migrations` exits 0. Agent: Codex gpt-6.1-sol high.

## 3. Verification

- [x] 3.1 (workspace) Run `python3 -I -B deploy/ci/test_migration_rewrite.py`, `python3 -I -B deploy/ci/test_migration_rewrite_forms.py`, `python3 -I -B deploy/ci/migration_rewrite.py migrations`, `openspec validate migration-rewrite-forms --strict`, and the `ste-lint.py` command of `CLAUDE.md`. Check: each command passes. Agent: orchestrator.
