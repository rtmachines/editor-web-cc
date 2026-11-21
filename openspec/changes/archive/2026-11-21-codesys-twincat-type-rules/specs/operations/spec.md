## ADDED Requirements

### Requirement: STRING encoding of a stored machine after the upgrade
The upgrade SHALL give each stored machine the `STRING` encoding `Latin1`. The upgrade SHALL not change a stored value, a stored `content_sha256`, or a stored `scope_sha256`. ADR-0202 SHALL add no byte to a document of a `Latin1` machine. Only ADR-0201 (i) SHALL change `loadable` of a stored version. The migrations SHALL not rewrite the table `machine` and SHALL not scan it under an exclusive lock (R31, ADR-0157, ADR-0201, ADR-0202).

#### Scenario: Upgrade of a database with a CODESYS machine
- **GIVEN** a database of the previous release with the machine `M9` on `CodesysIec` and the approved version 2 of `R-100` on `M9`
- **AND** no stored value of version 2 breaks the `WSTRING` rule of ADR-0201
- **WHEN** the operator runs `recipe-server migrate` of this release
- **THEN** `M9` has the encoding `Latin1`
- **AND** the v1 document and the v2 document of each unit of version 2 have the bytes of the previous release (ADR-0202 (j2))

#### Scenario: Upgrade with a stored WSTRING outside the Basic Multilingual Plane
- **GIVEN** a database of the previous release with the approved version 2 of `R-100` on the `TwinCat3` machine `M8`
- **AND** version 2 holds the `WSTRING(10)` value `"A😀"`
- **WHEN** the operator runs `recipe-server migrate` of this release
- **THEN** the documents of version 2 hold `loadable` with the value `false` (ADR-0201 (i))
- **AND** the hashed bodies, `content_sha256`, and `scope_sha256` of version 2 do not change

#### Scenario: Previous release after a rollback
- **GIVEN** a database after the migrations of this release
- **WHEN** the previous release runs `CreateMachine` for a `CodesysIec` machine
- **THEN** the system creates the machine with the encoding `Latin1`

### Requirement: Ops query parts of the CODESYS and TwinCAT rules
The ops queries of `deploy/db/` SHALL list the stored rows that break a rule of this change before an upgrade. `check-platform-ranges.sql` SHALL hold the part `iec_wide_text`. `check-identifier-rules.sql` SHALL hold the parts `iec_reserved_struct_name`, `iec_keyword_member_name`, and `codesys_one_member_struct` (R31, ADR-0201, ADR-0203, ADR-0204).

#### Scenario: Stored WSTRING outside the Basic Multilingual Plane
- **GIVEN** a stored `WSTRING` value `"A😀"` on a `TwinCat3` machine
- **WHEN** the operator runs `check-platform-ranges.sql`
- **THEN** the part `iec_wide_text` lists the value

#### Scenario: Stored reserved struct name and keyword member
- **GIVEN** a stored struct `VERSION@1` on a `TwinCat3` machine and a stored member `END_IF` on a `CodesysIec` machine
- **WHEN** the operator runs `check-identifier-rules.sql`
- **THEN** the part `iec_reserved_struct_name` lists the struct
- **AND** the part `iec_keyword_member_name` lists the member

#### Scenario: Stored one-member struct on CODESYS
- **GIVEN** a stored struct with one member on a `CodesysIec` machine and a stored struct with one member on a `TwinCat3` machine
- **WHEN** the operator runs `check-identifier-rules.sql`
- **THEN** the part `codesys_one_member_struct` lists only the struct of the `CodesysIec` machine

### Requirement: Rollback on the newer schema after the upgrade
The runbook SHALL permit the previous image on the newer schema only with no `Utf8` machine and no audit entry `MachineStringEncodingSet`. The runbook SHALL name the query that counts both. Otherwise the operator SHALL roll forward or restore the `_premigrate` set. That restore loses every write since the dump (R31, ADR-0202).

#### Scenario: Rollback with no change of an encoding
- **GIVEN** a database after the migrations of this release with no `Utf8` machine and no entry `MachineStringEncodingSet`
- **WHEN** the operator runs the query of the runbook section "Roll back on the newer schema"
- **THEN** the query gives `utf8_machines` 0 and `encoding_entries` 0
- **AND** the runbook permits the previous image on the newer schema

#### Scenario: Rollback with a Utf8 machine
- **GIVEN** a `CodesysIec` machine with the encoding `Utf8` and the stored `STRING` value `"é"`
- **WHEN** the operator runs the query of the runbook section "Roll back on the newer schema"
- **THEN** the query gives `utf8_machines` 1
- **AND** the runbook tells the operator to roll forward or to restore the `_premigrate` set

#### Scenario: Rollback after a change back to Latin1
- **GIVEN** a machine whose encoding changed from `Latin1` to `Utf8` and back to `Latin1`
- **WHEN** the operator runs the query of the runbook section "Roll back on the newer schema"
- **THEN** the query gives `utf8_machines` 0 and `encoding_entries` 2
- **AND** the runbook does not permit the previous image on the newer schema

### Requirement: Checks before a plant adds a CODESYS machine
The runbook SHALL name the checks of A51 and A52 before a plant adds a `CodesysIec` machine. The editor SHALL not check the compiler or the CODESYS version (R31, A51, A52, ADR-0199, ADR-0200).

#### Scenario: CODESYS version below V3.5 SP16
- **GIVEN** a controller project with CODESYS V3.5 SP15
- **WHEN** the operator follows the runbook section "Check the CODESYS version and the LREAL size before you add a CodesysIec machine"
- **THEN** the runbook tells the operator to update the project before the plant adds the machine

#### Scenario: LREAL of four bytes
- **GIVEN** a CODESYS target that compiles `LREAL` with a symbol size of 4 bytes
- **WHEN** the operator follows the same runbook section
- **THEN** the runbook tells the operator not to add the machine
