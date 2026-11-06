# Spec Delta

## ADDED Requirements

### Requirement: CI rejects a migration that locks or rewrites an existing table
CI SHALL fail a migration that adds a stored generated column, a primary key, or an exclusion constraint. A `CHECK`, foreign key, or domain constraint without `NOT VALID` SHALL fail. So SHALL a unique constraint without `USING INDEX` and an index without `CONCURRENTLY`. `VACUUM FULL`, `CLUSTER`, `LOCK TABLE`, `VALIDATE CONSTRAINT` of a domain, and a change of tablespace, logging, or access method SHALL fail. An object that the same file creates and a listed exception SHALL pass (R30, R31, R34).

#### Scenario: Stored generated column
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN n INTEGER GENERATED ALWAYS AS (1) STORED`
- **THEN** the job fails and names the file and the statement

#### Scenario: Check or foreign key without NOT VALID
- **GIVEN** the tables `recipe` and `machine` from an earlier migration
- **WHEN** a new migration adds `CHECK (note <> '')` to `recipe` without `NOT VALID`, or adds a column with an inline `REFERENCES machine (id)`
- **THEN** the job fails

#### Scenario: Check with NOT VALID and a later validation
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration adds `CHECK (note <> '') NOT VALID` to `recipe` and a later migration runs `VALIDATE CONSTRAINT` on it
- **THEN** the job passes

#### Scenario: Index without CONCURRENTLY
- **GIVEN** the table `audit_log` from an earlier migration
- **WHEN** a new migration runs `CREATE INDEX audit_log_x ON audit_log (occurred_at)`
- **THEN** the job fails
- **AND** the job also fails for `CREATE INDEX IF NOT EXISTS` without a listed exception

#### Scenario: Index with CONCURRENTLY
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration that starts with `-- no-transaction` runs `CREATE INDEX CONCURRENTLY recipe_x ON recipe (name)`
- **THEN** the job passes

#### Scenario: Unique constraint with and without USING INDEX
- **GIVEN** the table `recipe` and an index that `CREATE UNIQUE INDEX CONCURRENTLY recipe_x` built in an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD CONSTRAINT recipe_u UNIQUE (name)`
- **THEN** the job fails
- **AND** the job passes for `ALTER TABLE recipe ADD CONSTRAINT recipe_u UNIQUE USING INDEX recipe_x`

#### Scenario: Primary key and exclusion constraint
- **GIVEN** the table `recipe` and an index that `CREATE UNIQUE INDEX CONCURRENTLY recipe_x` built in an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD PRIMARY KEY USING INDEX recipe_x` or adds an exclusion constraint to `recipe`
- **THEN** the job fails for each statement

#### Scenario: Statements that rewrite or lock a whole table
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration runs `VACUUM FULL recipe`, `CLUSTER recipe`, `LOCK TABLE recipe`, or `ALTER TABLE recipe` with `SET TABLESPACE t`, `SET LOGGED`, `SET UNLOGGED`, or `SET ACCESS METHOD heap`
- **THEN** the job fails for each statement

#### Scenario: VACUUM FULL or CLUSTER without a table name
- **GIVEN** any set of earlier migrations
- **WHEN** a new migration runs `VACUUM FULL` or `CLUSTER` without a table name
- **THEN** the job fails, also when the same file creates a table

#### Scenario: Domain constraint
- **GIVEN** the domain `short_text` from an earlier migration
- **WHEN** a new migration runs `ALTER DOMAIN short_text ADD CONSTRAINT c CHECK (length(VALUE) < 10)` or `ALTER DOMAIN short_text SET NOT NULL`
- **THEN** the job fails for each statement
- **AND** the job passes for `ALTER DOMAIN short_text ADD CONSTRAINT c CHECK (length(VALUE) < 10) NOT VALID`
- **AND** the job fails for `ALTER DOMAIN short_text VALIDATE CONSTRAINT c`, because the form is `domain validate`

#### Scenario: Table that the same file creates
- **GIVEN** a new migration with an unconditional `CREATE TABLE note_tag (id UUID)`
- **WHEN** the same file adds a stored generated column, a `CHECK` without `NOT VALID`, a primary key, and an index without `CONCURRENTLY` on `note_tag`
- **THEN** the job passes

#### Scenario: Form inside a function body
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration creates a function whose body runs `CREATE INDEX recipe_x ON recipe (name)`
- **THEN** the job fails, also when the file does not call the function

#### Scenario: Shipped migrations with the listed index
- **GIVEN** the migrations `0001` to `0010`
- **AND** the listed exception for the index of `0006_operations.sql` with ADR-0069
- **WHEN** the CI job checks the migrations
- **THEN** the job passes
