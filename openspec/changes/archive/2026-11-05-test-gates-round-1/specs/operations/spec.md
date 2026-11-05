## ADDED Requirements

### Requirement: Coverage gate on production code
CI SHALL measure the line coverage of each crate on production code only. A file of a test module SHALL NOT count. An inline test module in `src/` SHALL fail the gate. A production file with a path that the default filter of the coverage tool drops SHALL fail the gate. A crate below 80 % SHALL fail the gate. A crate with a production file and no line in the report SHALL also fail the gate (R30, R34).

#### Scenario: Test helper file does not count
- **GIVEN** a crate with a test-only module `fixtures` in `src/`
- **WHEN** the coverage gate reads the report
- **THEN** the lines of the file of `fixtures` do not count for the crate

#### Scenario: Inline test module
- **GIVEN** a file in `src/` with a `#[cfg(test)]` module that has a body in the same file
- **WHEN** the coverage gate runs
- **THEN** the gate fails and names the file and the line

#### Scenario: Production file with a test file name
- **GIVEN** a production module `unit_tests` in `src/plant/` without a test-only attribute
- **WHEN** the coverage gate runs
- **THEN** the gate fails and names the file `unit_tests.rs`

#### Scenario: Production file in an examples directory
- **GIVEN** a production file `src/examples/mod.rs` without a test-only attribute
- **WHEN** the coverage gate runs
- **THEN** the gate fails and names the file `src/examples/mod.rs`

#### Scenario: Crate missing from the report
- **GIVEN** the crate `recipe-export` with production files in `src/`
- **AND** a coverage report that holds no file of `recipe-export`
- **WHEN** the coverage gate reads the report
- **THEN** the gate fails and names `recipe-export`

### Requirement: Coverage gate classifies a file by its declarations
The coverage gate SHALL count a file as test code only when every declaration that reaches it is test-gated. The gate SHALL compare the decoded value of a string literal in a `cfg` predicate (R30, R34).

#### Scenario: File with a test declaration and a production declaration
- **GIVEN** `src/lib.rs` with `#[cfg(test)] pub mod common;` and `src/main.rs` with `mod common;`
- **AND** a report with 10 covered lines in the other files and 90 uncovered lines in `common.rs`
- **WHEN** the coverage gate reads the report
- **THEN** the lines of `common.rs` count for the crate
- **AND** the gate fails with 10 %

#### Scenario: Raw string literal in a cfg predicate
- **GIVEN** a module `fixtures` with `#[cfg(feature = r"memory")]` in `src/`
- **WHEN** the coverage gate reads the report
- **THEN** the lines of the file of `fixtures` do not count for the crate

#### Scenario: Other string value in a cfg predicate
- **GIVEN** a module `fixtures` with `#[cfg(feature = "memory2")]` in `src/`
- **WHEN** the coverage gate reads the report
- **THEN** the lines of the file of `fixtures` count for the crate

### Requirement: Grant test covers PUBLIC
The grant test SHALL compare the rights of `PUBLIC` on the schema `public` and on its tables, sequences, columns, and functions with a checked-in list. A default right that no `REVOKE` removed SHALL count. A difference SHALL fail the test (R31, R38).

#### Scenario: New function without a revoke
- **GIVEN** a migration that creates a function and does not revoke `EXECUTE` from `PUBLIC`
- **WHEN** the grant test runs
- **THEN** the test fails and names the function with the right `EXECUTE`

#### Scenario: One right of each kind
- **GIVEN** a test database where `PUBLIC` gets `SELECT` on a table, `UPDATE` on a column, `USAGE` on a sequence, and `USAGE` on the schema `public`
- **WHEN** the grant test compares the rights with the list
- **THEN** the difference names each of the four rights

#### Scenario: Revoked function
- **GIVEN** the function `replication_state()` with `EXECUTE` revoked from `PUBLIC`
- **WHEN** the grant test runs
- **THEN** the test finds no right of `PUBLIC` on `replication_state()`

### Requirement: CI rejects a table-rewriting migration
CI SHALL check every file in `migrations/` on each pull request. A statement that changes a column type, adds a column with a volatile default, or sets `NOT NULL` without a current valid `CHECK (... IS NOT NULL)` SHALL fail the job. A `serial` or identity column SHALL count as a volatile default. A `DO` block SHALL also fail the job. Only a table that an unconditional `CREATE TABLE` in the same file creates SHALL be exempt. A listed exception with its ADR SHALL pass (R30, R31, R34).

#### Scenario: Change of a column type
- **GIVEN** a new migration with `ALTER TABLE audit_log ALTER COLUMN action TYPE TEXT`
- **WHEN** the CI job checks the migrations
- **THEN** the job fails and names the file and the statement

#### Scenario: Volatile default
- **GIVEN** a new migration that adds a column to `recipe` with the default `gen_random_uuid()`
- **WHEN** the CI job checks the migrations
- **THEN** the job fails

#### Scenario: Serial or identity column
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN s bigserial` or `ALTER TABLE recipe ADD COLUMN s bigint GENERATED ALWAYS AS IDENTITY`
- **THEN** the job fails

#### Scenario: Not null after a valid check
- **GIVEN** an earlier migration that adds `CHECK (note IS NOT NULL) NOT VALID` to `recipe` and then runs `VALIDATE CONSTRAINT` on it
- **WHEN** a new migration sets `NOT NULL` on `recipe.note`
- **THEN** the job passes

#### Scenario: Not null after a dropped check
- **GIVEN** a valid `CHECK (note IS NOT NULL)` on `recipe` that a later migration drops
- **WHEN** a new migration sets `NOT NULL` on `recipe.note`
- **THEN** the job fails

#### Scenario: DO block
- **GIVEN** a new migration with a `DO` block
- **AND** no listed exception for that file
- **WHEN** the CI job checks the migrations
- **THEN** the job fails and names the file and the line of the block

#### Scenario: Conditional create of an existing table
- **GIVEN** the table `recipe` from an earlier migration
- **AND** a new migration with `CREATE TABLE IF NOT EXISTS recipe` and then a change of a column type of `recipe`
- **WHEN** the CI job checks the migrations
- **THEN** the job fails

#### Scenario: Shipped migrations
- **GIVEN** the migrations `0001` to `0010`
- **WHEN** the CI job checks the migrations
- **THEN** the job passes

### Requirement: Rewrite check resolves bodies, tables, and check names
The rewrite check SHALL count a form inside the body of a function or a procedure. A temporary table SHALL NOT exempt a table of the schema `public`. A check with a name that the script cannot know for certain SHALL NOT count as a valid check (R30, R31, R34).

#### Scenario: Form inside a function body
- **GIVEN** the table `recipe` from an earlier migration
- **AND** a new migration with a function whose body changes a column type of `recipe`
- **WHEN** the CI job checks the migrations
- **THEN** the job fails, also when the file does not call the function

#### Scenario: Temporary table with the name of a table of public
- **GIVEN** the table `public.recipe` from an earlier migration
- **AND** a new migration with `CREATE TEMP TABLE recipe (note TEXT)` and then `ALTER TABLE public.recipe ALTER COLUMN note TYPE INTEGER USING 1`
- **WHEN** the CI job checks the migrations
- **THEN** the job fails

#### Scenario: Not null after an unnamed check
- **GIVEN** an earlier migration with `CREATE TABLE recipe (note TEXT CHECK (length(note) < 10))` and then `ALTER TABLE recipe ADD CHECK (note IS NOT NULL)`
- **AND** a new migration that drops the constraint `recipe_note_check1` and then sets `NOT NULL` on `recipe.note`
- **WHEN** the CI job checks the migrations
- **THEN** the job fails
