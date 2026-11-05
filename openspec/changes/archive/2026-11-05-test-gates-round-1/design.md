# Design: test-gates-round-1

## Context

See `proposal.md` for the five findings. The normative sources are `reliability.md` §5 and §6, `data-model.md` §5.3 and §8, `api.md` §9, and ADR-0069 (q).

Facts that shape the design:

- Since version 0.6.22, `cargo-llvm-cov` drops test files from the report. Version 0.6.24 drops each `tests/`, `examples/`, or `benches/` directory and each file `tests.rs`, `*_tests.rs`, or `*-tests.rs`. The job `coverage` installs `cargo-llvm-cov@0.6`, so the exact version can change.
- The flag `--no-default-ignore-filename-regex` turns off that filter. It is stable only since version 0.7.0.
- `src/` holds more than 40 test-only file modules with other names, for example `fixtures`, `matrix`, `prop`, `world`, `testkit`, `xss_kit`, and `test_support`. A fixed list of name patterns misses most of them.
- `src/` holds 64 inline test modules in five crates. `recipe-domain` holds 50 of them.
- An estimate from the report of 2026-11-04 08:41 gives 95 % to 98 % production coverage per crate after the test files are removed. The figure of 72 % in the review was too low.
- About 14 functions of `0001` and `0002` keep the default `EXECUTE` right of `PUBLIC`. Their `proacl` is `NULL`, so `aclexplode(proacl)` returns no row for them.
- `0003` revokes all rights on the schema `public` from `PUBLIC`. No test checks that state today.
- `deploy/ci/check.sh` runs the Python tests of `deploy/ci/` only in the Compose jobs. These jobs do not run on a pull request.
- The branch `main` has no branch protection. A red CI job does not stop a merge today.
- The route matrix also calls `GET /static/*`. These replies carry `Cache-Control: public, max-age` with `static.cache_max_age` (`api.md` §9).
- `routes/matrix/tests.rs` builds replies without the common headers. Four of its tests use `#[should_panic(expected = ...)]` with a general text.

## Goals / Non-Goals

**Goals:**

- One owner for each rule: the gate script owns the definition of test code, and the rewrite script owns the forbidden forms.
- Each new check gets a unit test that fails before the fix.
- No change of production behavior.

**Non-Goals:**

- No new Cargo dependency and no new external binary in CI.
- No check of rewrite forms beyond the three forms of `data-model.md` §8 and the `DO` block.

## Decisions

### D1. The gate finds test code from the source, not from name patterns

`tools/coverage-gate.py` reads every `.rs` file under `crates/<crate>/src/`. A `#[cfg(...)]` attribute is test-only when its predicate can hold only in a test build.

- `test` is test-only. `feature = "memory"` is test-only, because `recipe-server` never enables it (`crates/recipe-app/Cargo.toml`).
- The gate compares the decoded value of a string literal in a `cfg` predicate. Thus `"memory"`, `r"memory"`, and `r#"memory"#` are the same value. The escapes of a normal string literal also decode before the comparison.
- `any(...)` is test-only when each argument is test-only. `all(...)` is test-only when one argument is test-only. `not(...)` is never test-only.
- A test-only attribute before `mod name;` marks the file of the module and every file in its directory as test code. Doc comments, blank lines, and other attributes can stand between the two lines.
- A file counts as test code only when every declaration that reaches it is test-gated. One production declaration, for example `mod common;` in `main.rs`, makes it production code.
- A test-only attribute before `mod name { ... }` in a production file is a violation. The gate prints the file and the line and exits 1.
- A production file that the default filter of `cargo-llvm-cov` 0.6.24 drops is a violation. The gate prints the file and exits 1. The gate copies the full filter of that version: a `tests/`, `examples/`, or `benches/` directory, `tests.rs`, `*_tests.rs`, and `*-tests.rs` (ADR-0127 (f)).
- A `#[path]` attribute in `src/` stops the gate with exit 2, because the gate cannot resolve it. No file in `src/` uses one today.
- The gate drops each test file from the report before it adds the lines.

Alternative: an `--ignore-filename-regex` in CI, as the fix plan proposed. The regex of the plan misses the modules listed in Context, and a second copy in `CONTRIBUTING.md` can drift.

Alternative: an update to `cargo-llvm-cov` 0.9 with `--no-default-ignore-filename-regex`. The gate then classifies every file of the workspace. That needs a new tool version and a larger report. The name rule closes the same gap with a smaller change, so the gate keeps the current measurement.

### D2. A crate with production files and no reported line fails

A crate with at least one production file and a count of 0 lines fails with exit 1 and its name (D6-7). The result "no code" stays only for a crate without a production file.

### D3. Inline test modules move to files

Each inline module moves to its own file without a change of content, except one level less of indentation. `foo.rs` gets `foo/tests.rs`. `lib.rs`, `main.rs`, and `mod.rs` get `tests.rs` in their own directory. The module keeps its name, so `database.rs` gets `database/probe_text_tests.rs`. Inline modules in a test file, for example `style_tests/css.rs`, stay, because the gate ignores that file.

### D4. The route matrix asserts the headers on every cell

`pages/test_helpers.rs` gets one table `SECURITY_HEADERS` of names and values from `api.md` §9. `health/tests.rs` uses that table and drops its own list. `assert_want` calls a new `assert_common_headers(reply, path, what)` before the `match`. It checks a version 7 `X-Request-Id` and each header of the table. The check covers every `Want` arm, also the redirects and the errors.

- `Cache-Control` must be `no-store`. For a path under `/static/`, it must be `public, max-age=<static.cache_max_age>` from the settings of the fixture.
- The value of `Strict-Transport-Security` comes from the test settings of the fixture, not from a literal.
- A cell that fails shows a real defect. The apply stops for that cell, and a new task fixes the defect.

`routes/matrix/tests.rs` changes in the same task. Its factory `reply()` adds every header of the table and a version 7 `X-Request-Id`. Each `#[should_panic]` test gets an `expected` text that only its own assertion prints. Two new negative tests remove one header and use a version 4 id.

### D5. The grant test reads the rights of PUBLIC from pg_catalog

`checks.rs` gets four queries for the grantee `0` (`PUBLIC`) in the schema `public`.

- The schema: `aclexplode(coalesce(nspacl, acldefault('n', nspowner)))` for `nspname = 'public'`.
- Tables and sequences: `aclexplode(coalesce(relacl, acldefault(<kind>, relowner)))`.
- Columns: `aclexplode(attacl)`. A column has no default right.
- Functions and procedures: `aclexplode(coalesce(proacl, acldefault('f', proowner)))`.

`acldefault` makes a default right visible. `information_schema.role_*_grants` does not show `PUBLIC`, so it cannot do this.

Each `PUBLIC` line of `grants.expected` starts with `PUBLIC:`, for example `PUBLIC:set_updated_at()|EXECUTE`. The file stays one file, as `.claude/rules/sql.md` requires. The comparison gets a function `grant_difference` that returns both sets. The failing test uses that function and does not need `#[should_panic]`.

The test gives `PUBLIC` one right of each kind: `EXECUTE` on a function, `SELECT` on a table, `UPDATE` on a column, `USAGE` on a sequence, and `USAGE` on the schema. For each right, `grant_difference` must name exactly that `PUBLIC:` line. A query that finds nothing then fails the test.

### D6. An own script rejects the rewrite forms

`deploy/ci/migration_rewrite.py` reads every file of `migrations/` in name order. It removes comments and splits statements outside quotes and dollar quotes, also named dollar quotes such as `$fn$`. It keeps the start line of each statement. `ONLY` and `IF EXISTS` do not change the form.

The table identifier keeps its schema. An unqualified name means `public`, so `recipe` and `public.recipe` are the same table. A temporary table (`TEMP`, `TEMPORARY`, or a `pg_temp.` name) has its own namespace. It never exempts a table of `public`. After `CREATE TEMP TABLE recipe`, an unqualified `recipe` in the same file names the temporary table, as in PostgreSQL.

The script reports these forms:

1. `ALTER [COLUMN] c [SET DATA] TYPE`.
2. `ADD [COLUMN] ... DEFAULT <expr>`, where `<expr>` calls a function outside a short list of non-volatile built-ins, such as `now()`. An unknown function, for example `nextval()`, counts as volatile. Form 2 also covers `ADD [COLUMN]` with a `serial` type (`smallserial`, `serial`, `bigserial`, `serial2`, `serial4`, `serial8`) and `ADD [COLUMN] ... GENERATED { ALWAYS | BY DEFAULT } AS IDENTITY`.
3. `ALTER [COLUMN] c SET NOT NULL` without a current valid `CHECK (c IS NOT NULL)` constraint on the same table and column.
4. A `DO` block. The script does not read PL/pgSQL, so it cannot see a form inside the block, also not a form in `EXECUTE`.

The forms 1 to 3 do not count on a table that an unconditional `CREATE TABLE` in the same file created. `CREATE TABLE IF NOT EXISTS` gives no exemption, because the table can be old. A form inside the body of `CREATE FUNCTION` or `CREATE PROCEDURE` counts like a form in a `DO` block. The same file can call the body with `SELECT`, `PERFORM`, or `CALL`. The form counts also when the file does not call it.

The state of form 3 is valid across files:

- A `CHECK (c IS NOT NULL)` gets the state "valid" when it has no `NOT VALID`, or when a later `VALIDATE CONSTRAINT` names it. A check on another column or another table does not count.
- A check with a name that the script cannot know for certain is no proof of NOT NULL. An unnamed `CHECK` can be such a check, because PostgreSQL picks its name. A named `ADD CONSTRAINT <name> CHECK (c IS NOT NULL)` stays a proof.
- `DROP CONSTRAINT`, `DROP COLUMN`, and `DROP TABLE` remove the state. `RENAME CONSTRAINT`, `RENAME COLUMN`, and `RENAME TO` move it to the new name.
- The script reads all actions of one `ALTER TABLE` before it uses the state. Thus `SET NOT NULL, DROP CONSTRAINT x` in one statement fails, because PostgreSQL cannot use the dropped check.

The script splits an `ALTER TABLE` with several actions at the commas of depth 0. It exits 1 with `file:line: <form>` for each match and 2 when it cannot read or split a file.

The constant `EXEMPT` maps a pair (file, form) to the ADR that permits it. It is empty today. A shipped file can never change, because `sqlx` stops on a changed checksum. When a later rule catches a shipped file, an entry with an ADR keeps CI green. A test asserts that each entry still matches a form, so no dead entry stays.

Alternative: the linter `squawk`. Version 2.x accepts `SET NOT NULL` after a valid `CHECK` on PostgreSQL 12 and later. But it checks one file at a time, and the project uses a check in `0009` and its validation in `0010`. It also brings an external binary with its own pin and many rules that the project must turn off. The own script has about 200 lines and unit tests in the pattern of `test_migration_freeze.py`.

The script checks every file, not only new files. It then needs no Git base, and the shipped files `0001` to `0010` serve as negative fixtures.

### D7. CI runs the new checks on every pull request

- The job `coverage` installs exactly `cargo-llvm-cov@0.6.24`. A new version then needs a change of `ci.yml`, and the review of that change compares the default filter with the copy in D1.
- The job `coverage` runs `python3 -I -B tools/test_coverage_gate.py` before the measurement.
- A new job `migration-rewrite` runs `test_migration_rewrite.py` and then `migration_rewrite.py migrations`. The job `migration-freeze` stays unchanged.
- After the merge, the branch protection of `main` requires each job of `ci.yml` and `playwright.yml` that runs on a pull request. The new job names exist on `main` only after the merge. The owner approves the exact list before the `gh api` call.

### D8. ADR-0127 records D1, D2, D5, and D6

The documents follow the ADR in the same change:

- `reliability.md` §6: the coverage line and the row "Integration, storage".
- `data-model.md` §5.3: the `PUBLIC` rows. `data-model.md` §8: the name of the check script, the `DO` rule, and the list of exceptions.
- `CONTRIBUTING.md`: the coverage section.

## Risks / Trade-offs

- [The source scan of D1 misreads a module form] → The unit tests cover visibility, doc comments, stacked attributes, `mod.rs`, and nested directories. Exit 2 on `#[path]` stops a silent miss.
- [A new `cargo-llvm-cov` version changes the default filter] → The exact pin of D7 stops a silent change. The review of the pin change compares the filter with D1.
- [A crate falls under 80 % after the change] → The estimate gives at least 95 %. Task 7.1 measures it. A crate below the target stops the apply for an owner decision.
- [A test-only `fn` or `impl` with `#[cfg(test)]` in a production file still counts] → Such items are rare and small. The rule of Q-D names modules only.
- [The SQL splitter of D6 misreads a statement] → Exit 2 on an unterminated quote. The shipped files and one fixture per form and per syntax variant test it.
- [The list of non-volatile functions is too short] → An unknown function fails the job. The error is a false alarm and never a miss. The author can add a function to the list with a test.
- [A needed `DO` block fails the job] → An entry in `EXEMPT` with an ADR permits it. No shipped file uses `DO` today.
- [Other rewrite forms pass, for example `ADD COLUMN ... GENERATED ALWAYS AS ... STORED` or `ADD CONSTRAINT` without `NOT VALID`] → They are out of scope of `data-model.md` §8 today. A later ADR can add them.
- [The header check of D4 finds a defect in many cells] → The apply stops. Each defect becomes its own task.
- [A merge happens before the branch protection exists] → Task 7.3 follows the merge directly. Until then, the reviewer checks the CI result by hand.

## Migration Plan

The change touches tests, CI, and documents only. No release step and no data step exist. A revert of the merge commit restores the old gates. The branch protection is a setting of GitHub and stays after a revert.
