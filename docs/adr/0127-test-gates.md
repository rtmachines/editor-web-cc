# ADR-0127: Test gates on production code, PUBLIC rights, and rewrite forms

Status: Accepted
Date: 2026-11-04
Deciders: project owner

## Context

`reliability.md` §6 sets the test levels and the coverage target of 80 % line coverage per crate. `data-model.md` §5.3 sets the grant test. `data-model.md` §8 forbids a migration that rewrites a table or scans it under an exclusive lock.

The review of 2026-11-04 found four gaps in these gates:

- The coverage gate counts test code as crate code.
- The coverage gate passes a crate that is missing from the report.
- The grant test does not see a right of `PUBLIC`.
- No CI step rejects a migration that rewrites a table.

Each gap lets a defect pass CI (R30, R31, R34, R38).

These facts shape the decision:

- Since version 0.6.22, `cargo-llvm-cov` drops test files from the report. Version 0.6.24 drops each `tests/`, `examples/`, or `benches/` directory and each file `tests.rs`, `*_tests.rs`, or `*-tests.rs`.
- The job `coverage` installs `cargo-llvm-cov@0.6`, so the exact version can change without a change of `ci.yml`.
- The flag `--no-default-ignore-filename-regex` turns off that filter. It is stable only since version 0.7.0.
- `src/` holds more than 40 test-only file modules with other names, for example `fixtures`, `matrix`, `prop`, `world`, `testkit`, `xss_kit`, and `test_support`.
- `src/` holds 64 inline test modules in five crates.
- About 14 functions of `0001` and `0002` keep the default `EXECUTE` right of `PUBLIC`. Their `proacl` is `NULL`, so `aclexplode(proacl)` returns no row for them.
- `information_schema.role_table_grants`, `role_column_grants`, and `role_routine_grants` do not show `PUBLIC`.
- The branch `main` has no branch protection. A red CI job does not stop a merge.

## Decision drivers

- Each rule has one owner. The gate script owns the definition of test code. The rewrite script owns the forbidden forms.
- A gap of a gate gives a false alarm, never a silent miss (R30, R31).
- No new Cargo dependency and no new external binary in CI (R34).
- Each new check gets a unit test that fails before the fix.
- No change of production behavior.

## Considered options

### Test code in the coverage report

#### Option A: The gate finds test code from the source

- Pros: one owner for the definition of test code. It finds every test-only module, whatever its name.
- Cons: the gate must read the module forms of Rust correctly.

#### Option B: A fixed regex of test file names in `--ignore-filename-regex`

- Pros: a small change of the CI step.
- Cons: the regex misses the test-only modules with other names, for example `fixtures` and `world`. A second copy of the regex in `CONTRIBUTING.md` can drift.

#### Option C: `cargo-llvm-cov` 0.9 with `--no-default-ignore-filename-regex`

- Pros: the report holds every file of the workspace, and the gate classifies each one.
- Cons: it needs a new tool version and gives a larger report. The name rule of item (f) closes the same gap with a smaller change. The gate thus keeps the current measurement.

### Check of the rewrite forms

#### Option D: An own script in `deploy/ci/`

- Pros: it reads all files in name order and keeps the state of a check across files. It has about 200 lines and unit tests in the pattern of `test_migration_freeze.py`.
- Cons: the project maintains a small SQL splitter.

#### Option E: The linter `squawk`

- Pros: version 2.x accepts `SET NOT NULL` after a valid `CHECK` on PostgreSQL 12 and later.
- Cons: it checks one file at a time. The check of `0009` becomes valid only through `0010`. It brings an external binary with its own pin. The project must turn off many of its rules.

## Decision

The project uses **Option A** and **Option D**. The items below are the rules. This ADR changes `reliability.md` §6 and `data-model.md` §5.3 and §8. It does not change ADR-0069 (q). It adds the rights of `PUBLIC` to the grant test.

### Coverage gate

- (a) `tools/coverage-gate.py` reads every `.rs` file under `crates/<crate>/src/`. A `#[cfg(...)]` attribute is test-only when its predicate can hold only in a test build.
- (b) `test` is test-only. `feature = "memory"` is test-only, because `recipe-server` never enables it (`crates/recipe-app/Cargo.toml`).
  - The gate compares the decoded value of a string literal in a `cfg` predicate. Thus `"memory"`, `r"memory"`, and `r#"memory"#` are the same value.
  - The escapes of a normal string literal also decode before the comparison. `"memory2"` is a different value.
- (c) `any(...)` is test-only when each argument is test-only. `all(...)` is test-only when one argument is test-only. `not(...)` is never test-only.
- (d) A test-only attribute before `mod name;` marks the file of the module and every file in its directory as test code. Doc comments, blank lines, and other attributes can stand between the two lines.
  - A file counts as test code only when every declaration that reaches it is test-gated.
  - One production declaration makes the file production code. An example is `#[cfg(test)] pub mod common;` in `lib.rs` and `mod common;` in `main.rs`.
- (e) A test-only attribute before `mod name { ... }` in a production file is a violation. The gate prints the file and the line and exits 1.
- (f) A production file that the default filter of `cargo-llvm-cov` 0.6.24 drops is a violation. The gate prints the file and exits 1.
  - The gate copies the full filter of that version: `(^|/)(tests|examples|benches)/|/(tests\.rs|[0-9a-zA-Z_-]+[_-]tests\.rs)$`.
  - Thus the filter drops a `tests/`, `examples/`, or `benches/` directory, `tests.rs`, `*_tests.rs`, and `*-tests.rs`.
- (g) A `#[path]` attribute in `src/` stops the gate with exit 2, because the gate cannot resolve it.
- (h) The gate removes each test file from the report before it adds the lines. The target of 80 % per crate of `reliability.md` §6 stays.
- (i) A crate with at least one production file and a count of 0 lines fails with exit 1 and its name. The result "no code" stays only for a crate without a production file.

### Grant test

- (j) The grant test of `data-model.md` §5.3 gets four queries for the grantee `0` (`PUBLIC`) in the schema `public`:
  - The schema: `aclexplode(coalesce(nspacl, acldefault('n', nspowner)))` for `nspname = 'public'`.
  - Tables and sequences: `aclexplode(coalesce(relacl, acldefault(<kind>, relowner)))`.
  - Columns: `aclexplode(attacl)`. A column has no default right.
  - Functions and procedures: `aclexplode(coalesce(proacl, acldefault('f', proowner)))`.
- (k) `acldefault` makes a default right visible. A default right that no `REVOKE` removed thus counts as a right of `PUBLIC`.
- (l) Each `PUBLIC` line of `grants.expected` starts with `PUBLIC:`, for example `PUBLIC:set_updated_at()|EXECUTE`. The list stays one file.
- (m) A function `grant_difference` returns both sets of the comparison. A test gives `PUBLIC` one right of each kind: `EXECUTE` on a function, `SELECT` on a table, `UPDATE` on a column, `USAGE` on a sequence, and `USAGE` on the schema. For each right, `grant_difference` must name exactly that `PUBLIC:` line.

### Check of the rewrite forms

- (n) `deploy/ci/migration_rewrite.py` reads every file of `migrations/` in name order. It removes comments. It splits statements outside quotes and dollar quotes, also named dollar quotes such as `$fn$`. It keeps the start line of each statement.
- (o) The table identifier keeps its schema. An unqualified name means `public`, so `recipe` and `public.recipe` are the same table. `ONLY` and `IF EXISTS` do not change the form.
  - A temporary table has its own namespace. `CREATE TEMP TABLE`, `CREATE TEMPORARY TABLE`, and a `pg_temp.` name make a temporary table.
  - A temporary table never exempts a table of `public`.
  - After `CREATE TEMP TABLE recipe`, an unqualified `recipe` in the same file names the temporary table, as in PostgreSQL. `public.recipe` still names the table of `public`.
- (p) The script reports four forms:
  1. `ALTER [COLUMN] c [SET DATA] TYPE`.
  2. `ADD [COLUMN] ... DEFAULT <expr>`, where `<expr>` calls a function outside a short list of non-volatile built-ins, such as `now()`. An unknown function, for example `nextval()`, counts as volatile.
     - Form 2 also covers `ADD [COLUMN]` with the type `smallserial`, `serial`, `bigserial`, `serial2`, `serial4`, or `serial8`.
     - Form 2 also covers `ADD [COLUMN] ... GENERATED { ALWAYS | BY DEFAULT } AS IDENTITY`.
     - These two forms rewrite a table that holds rows, also without the word `DEFAULT`. `GENERATED ... STORED` stays out of scope.
  3. `ALTER [COLUMN] c SET NOT NULL` without a current valid `CHECK (c IS NOT NULL)` constraint on the same table and column.
  4. A `DO` block. The script does not read PL/pgSQL, so it cannot see a form inside the block, also not a form in `EXECUTE`.
- (q) The forms 1 to 3 do not count on a table that an unconditional `CREATE TABLE` in the same file created. `CREATE TABLE IF NOT EXISTS` gives no exemption, because the table can be old.
  - A form inside the body of `CREATE FUNCTION` or `CREATE PROCEDURE` counts like a form in a `DO` block. The same file can call the body with `SELECT`, `PERFORM`, or `CALL`, so the form can run during the migration.
  - The form counts also when the file does not call the function or the procedure.
- (r) The state of form 3 is valid across files:
  - A `CHECK (c IS NOT NULL)` gets the state "valid" when it has no `NOT VALID`, or when a later `VALIDATE CONSTRAINT` names it.
  - A check on another column or another table does not count.
  - A check with a name that the script cannot know for certain is no proof of NOT NULL. An unnamed `CHECK` can be such a check, because PostgreSQL picks its name. A later `SET NOT NULL` that needs this check is form 3.
  - A named `ADD CONSTRAINT <name> CHECK (c IS NOT NULL)` stays a proof.
  - `DROP CONSTRAINT`, `DROP COLUMN`, and `DROP TABLE` remove the state.
  - `RENAME CONSTRAINT`, `RENAME COLUMN`, and `RENAME TO` move the state to the new name.
  - The script reads all actions of one `ALTER TABLE` before it uses the state. Thus `SET NOT NULL, DROP CONSTRAINT x` in one statement fails.
- (s) The script splits an `ALTER TABLE` with several actions at the commas of depth 0. It exits 1 with `file:line: <form>` for each match. It exits 2 when it cannot read or split a file.
- (t) The constant `EXEMPT` maps a pair (file, form) to the ADR that permits it. A test asserts that each entry still matches a form, so no dead entry stays.
- (u) The script checks every file, not only new files. It thus needs no Git base. The shipped files `0001` to `0010` serve as negative fixtures.

### CI

- (v) The job `coverage` installs exactly `cargo-llvm-cov@0.6.24`. A new version then needs a change of `ci.yml`. The review of that change compares the default filter of the tool with the copy of item (f).
- (w) The job `coverage` runs `python3 -I -B tools/test_coverage_gate.py` before the measurement.
- (x) A new job `migration-rewrite` runs `test_migration_rewrite.py` and then `migration_rewrite.py migrations` on every pull request. The job `migration-freeze` stays unchanged.
- (y) After the merge, the branch protection of `main` requires each job of `ci.yml` and `playwright.yml` that runs on a pull request. The new job names exist on `main` only after the merge. The project owner approves the exact list before the `gh api` call.

## Consequences

### Positive

- The coverage figure of each crate counts production code only.
- A new inline test module or a production file with a path that the default filter drops fails CI.
- A crate that is missing from the report fails CI.
- A new function without a `REVOKE` of `EXECUTE` from `PUBLIC` fails the grant test.
- A migration with a rewrite form of `data-model.md` §8 or a `DO` block fails CI on the pull request.
- A red CI job stops a merge into `main`.

### Negative

- The 64 inline test modules move to their own files. No production code changes.
- The list `EXEMPT` is empty today. A shipped file can never change, because `sqlx` stops on a changed checksum. When a later rule catches a shipped file, an entry with an ADR keeps CI green.
- A needed `DO` block also needs an entry in `EXEMPT` with an ADR.
- The exact pin `cargo-llvm-cov@0.6.24` needs a manual change for each new version of the tool.
- An unknown function in a column default fails the job. The author adds the function to the list with a test.
- The branch protection of item (y) is a setting of GitHub. A revert of the merge commit does not remove it.
- A test-only `fn` or `impl` with `#[cfg(test)]` in a production file still counts. Such items are rare and small. The rules of items (d) and (e) name modules only.

### Risks and mitigations

- Risk: the source scan misreads a module form. Mitigation: the unit tests cover visibility, doc comments, stacked attributes, `mod.rs`, and nested directories. Item (g) stops a silent miss on `#[path]`.
- Risk: a new `cargo-llvm-cov` version changes the default filter. Mitigation: the exact pin of item (v) stops a silent change.
- Risk: the SQL splitter misreads a statement. Mitigation: exit 2 on an unterminated quote. The shipped files and one fixture per form and per syntax variant test the splitter.
- Risk: other rewrite forms pass, for example `ADD COLUMN ... GENERATED ALWAYS AS ... STORED` or `ADD CONSTRAINT` without `NOT VALID`. Mitigation: they are out of the scope of `data-model.md` §8 today. A later ADR can add them.
- Risk: a merge happens before the branch protection exists. Mitigation: the step of item (y) follows the merge directly. Until then, the reviewer reads the CI result by hand.

## Related

- ADR-0069 (q) (routine rows of the grant test), ADR-0084 (d) (migration rule of a generated column), ADR-0101 (storage tests start their own container)
- ADR-0048 (browser tests), ADR-0023 (STE lint scope and script location in CI)
- `reliability.md` §5 and §6, `data-model.md` §5.3 and §8
- `api.md` §9 (the security headers that the route matrix of the same change checks)
- Change `test-gates-round-1`, capability `operations`
- `overview.md` §7: R30, R31, R34, R38
