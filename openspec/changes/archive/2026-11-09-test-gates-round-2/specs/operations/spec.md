## ADDED Requirements

### Requirement: CI rejects an added column of a constrained domain
CI SHALL fail a migration that adds a column to a table of an earlier file with a constrained domain type. A domain with a `CHECK` or a `NOT NULL` is constrained. A domain over such a domain SHALL count. The form SHALL count also without a default. A domain that the same file creates SHALL NOT give an exemption. A table that the same file creates and a listed exception SHALL pass (R30, R31, R34).

#### Scenario: Domain with a check in the same file
- **GIVEN** the table `recipe` from an earlier migration
- **WHEN** a new migration runs `CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0)` and then `ALTER TABLE recipe ADD COLUMN weight pos_int`
- **THEN** the job fails and names the file, the line, and the form `add column of a constrained domain`

#### Scenario: Constrained domain from an earlier migration
- **GIVEN** the table `recipe` and the domain `code_text AS TEXT NOT NULL` from an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN code code_text`
- **THEN** the job fails

#### Scenario: Domain over a constrained domain
- **GIVEN** the table `recipe` and the domain `pos_int AS INTEGER CHECK (VALUE > 0)` from an earlier migration
- **WHEN** a new migration runs `CREATE DOMAIN weight_int AS pos_int` and then `ALTER TABLE recipe ADD COLUMN weight weight_int`
- **THEN** the job fails

#### Scenario: Constraint that a later statement adds to a domain
- **GIVEN** the table `recipe` and the domain `label AS TEXT` from an earlier migration
- **AND** a later migration that runs `ALTER DOMAIN label ADD CONSTRAINT c CHECK (VALUE <> '') NOT VALID`
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN tag label`
- **THEN** the job fails

#### Scenario: Domain without a constraint
- **GIVEN** the table `recipe` and the domain `label AS TEXT` from an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN tag label`
- **THEN** the job passes

#### Scenario: Table that the same file creates
- **GIVEN** a new migration with an unconditional `CREATE TABLE note_tag (id UUID)` and `CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0)`
- **WHEN** the same file runs `ALTER TABLE note_tag ADD COLUMN weight pos_int`
- **THEN** the job passes

#### Scenario: Form inside a function body
- **GIVEN** the table `recipe` and the domain `pos_int AS INTEGER CHECK (VALUE > 0)` from an earlier migration
- **WHEN** a new migration creates a function whose body runs `ALTER TABLE recipe ADD COLUMN weight pos_int`
- **THEN** the job fails, also when the file does not call the function

#### Scenario: Shipped migrations
- **GIVEN** the migrations `0001` to `0011` and the listed exceptions of `0006_operations.sql` and `0011_plc_type_rules.sql`
- **WHEN** the CI job checks the migrations
- **THEN** the job passes

### Requirement: CI rejects an added column of a domain with a volatile default
CI SHALL fail an added column without its own default when the domain type of the column has a volatile default. The rule SHALL apply to a table of an earlier file. A domain without its own default over such a domain SHALL count. A table that the same file creates and a listed exception SHALL pass (R30, R31, R34).

#### Scenario: Domain with a volatile default
- **GIVEN** the table `recipe` and the domain `row_key AS UUID DEFAULT gen_random_uuid()` from an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN key row_key`
- **THEN** the job fails and names the file, the line, and the form `volatile default`

#### Scenario: Domain with a default that is not volatile
- **GIVEN** the table `recipe` and the domain `count_int AS INTEGER DEFAULT 0` from an earlier migration
- **WHEN** a new migration runs `ALTER TABLE recipe ADD COLUMN amount count_int`
- **THEN** the job passes

## MODIFIED Requirements

### Requirement: End-to-end smoke on the Compose stack
The job `compose` SHALL run a browser smoke suite through Caddy after its stack checks and before the stop of `app`. The suite SHALL create a machine, a recipe, and a parameter, and set limits. A second user who did not edit the draft SHALL approve it. The suite SHALL then export the version and check its content hash and its content. Then the suite SHALL check the audit page (R04, R08, R09, R20, R27, A34).

#### Scenario: Flow of the smoke suite
- **GIVEN** the Compose stack of the job `compose` after its stack checks
- **WHEN** the smoke suite runs the flow through Caddy
- **THEN** the second user approves the draft, and the version gets the status `Approved`
- **AND** the export returns `200` with `loadable` set to `true`
- **AND** the SHA-256 of the RFC 8785 form of the hashed body equals `content_sha256`
- **AND** the document holds the machine code and the reference number of the run and the version number `1`
- **AND** the document holds one parameter `SPEED` with the type `INT` and the value `50`
- **AND** that parameter holds one limit with the minimum `0` and the maximum `100`
- **AND** the audit page shows the entry of the approval

#### Scenario: Export that lost its parameter
- **GIVEN** an export document with a correct `content_sha256` and no parameter `SPEED`
- **WHEN** the content check of the smoke suite reads the document
- **THEN** the check fails and names the missing parameter
- **AND** the check also fails when a limit or the value differs from the value that the suite set

#### Scenario: Approval by the editor of the draft
- **GIVEN** a draft that the first user edited in the smoke suite
- **WHEN** the first user sends the approval of the draft
- **THEN** the approval fails with `E-RCP-015`
- **AND** the draft keeps the status `Draft`

#### Scenario: Failed smoke step
- **GIVEN** a smoke run in which one step fails
- **WHEN** the job `compose` reads the result
- **THEN** the job fails, and the trace of the failed test is a CI artifact
