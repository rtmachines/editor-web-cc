## ADDED Requirements

### Requirement: LDATE and LDT range on CodesysIec and TwinCat3
On `CodesysIec` and `TwinCat3`, the system SHALL accept an `LDT` value only from 1970-01-01-00:00:00 to 2262-04-11-23:47:16.854775807. The count of nanoseconds SHALL be 0 to 9 223 372 036 854 775 807. An `LDATE` value SHALL be in the same range, and its last day SHALL be 2262-04-11. A value outside the range SHALL give `E-RCP-005`. A limit outside the range SHALL give `E-RCP-006` (R03, R12, R38, A43).

#### Scenario: Last LDT instant on TwinCat3
- **GIVEN** the parameter `STAMP` of type `LDT` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `STAMP` with the value `LDT#2262-04-11-23:47:16.854775807`
- **THEN** the system stores the value `9223372036854775807`

#### Scenario: LDT after the last instant on CodesysIec
- **GIVEN** the parameter `STAMP` of type `LDT` in the draft version 3 of `R-100` on the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `STAMP` with the value `LDT#2262-04-11-23:47:16.854775808`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the value of `STAMP` and the row version of the draft do not change

#### Scenario: Last LDATE day on TwinCat3
- **GIVEN** the parameter `DUE` of type `LDATE` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `LD#2262-04-11`
- **THEN** the system stores the value `9223286400000000000`

#### Scenario: LDATE after the last day on CodesysIec
- **GIVEN** the parameter `DUE` of type `LDATE` in the draft version 3 of `R-100` on the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `LD#2262-04-12`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: LDT limit after the last instant
- **GIVEN** the parameter `STAMP` of type `LDT` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `STAMP` with the `max` value `LDT#2262-04-11-23:47:16.854775808`
- **THEN** the system rejects the command with `E-RCP-006`

#### Scenario: Stored LDT after the range at approval
- **GIVEN** the draft version 3 of `R-100` on the platform `TwinCat3` with the stored `LDT` value `10413792000000000000` in the parameter `STAMP`, which is `LDT#2300-01-01-00:00:00`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-005` for `STAMP`

#### Scenario: New value next to a stored LDT limit of the earlier rule set
- **GIVEN** the draft version 3 of `R-100` on the platform `TwinCat3` with the stored `LDT` limit `max` `10413792000000000000` in the parameter `STAMP`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `STAMP` with the value `LDT#2026-10-12-12:30:00`
- **THEN** the system stores the value `1791808200000000000`

## MODIFIED Requirements

### Requirement: Data block number per S7 platform
On `S7_300`, the data block number of an S7 address SHALL be at most 16 000. On `S7_1200` and `S7_1500`, it SHALL be at most 59 999. A higher number SHALL give `E-RCP-057`, and the message SHALL name the highest number of the platform. The system SHALL apply the rule to the same parameters as the end rule. A stored address of another parameter SHALL not refuse a command (R03, R14, R38, A31).

#### Scenario: Highest data block on S7_300
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB16000.DBW0`
- **THEN** the system adds the parameter

#### Scenario: Data block above 16000 on S7_300
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB16001.DBW0`
- **THEN** the system rejects the command with `E-RCP-057`

#### Scenario: Data block above 16000 on S7_1500
- **GIVEN** the machine `M7` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M7` with the type `INT` and the address `DB16001.DBW0`
- **THEN** the system adds the parameter

#### Scenario: Highest data block on S7_1500
- **GIVEN** the machine `M7` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M7` with the type `INT` and the address `DB59999.DBW0`
- **THEN** the system adds the parameter

#### Scenario: Data block above 59999 on S7_1500
- **GIVEN** the machine `M7` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M7` with the type `INT` and the address `DB60000.DBW0`
- **THEN** the system rejects the command with `E-RCP-057`
- **AND** the message names the highest data block number `59999`

#### Scenario: Last data block of the grammar on S7_1200
- **GIVEN** the machine `M9` with the platform `S7_1200`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M9` with the type `INT` and the address `DB65535.DBW0`
- **THEN** the system rejects the command with `E-RCP-057`

#### Scenario: Stored data block above 59999 at approval
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored parameter `SPEED` of type `INT` at `DB60000.DBW0`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-057`

#### Scenario: New parameter next to a stored data block above 59999
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored parameter `SPEED` of type `INT` at `DB60000.DBW0`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `FEED`, the type `INT`, and the address `DB2.DBW0`
- **THEN** the system adds the parameter

#### Scenario: Approval after the fix of a stored data block above 59999
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored parameter `SPEED` of type `INT` at `DB60000.DBW0` and no other fault
- **AND** a ProcessEngineer ran `UpdateParameterDefinition` on `SPEED` with the address `DB59999.DBW0`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system approves version 3

#### Scenario: Rebind of a parameter in a data block above 59999
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the parameter `ZONE_1` of type `ST_Zone@2` at `DB60000.DBB0` and the revision `ST_Zone@3`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@2` to `ST_Zone@3`
- **THEN** the system rejects the command with `E-RCP-057`
- **AND** the type of `ZONE_1` stays `ST_Zone@2`

### Requirement: New draft from a base that breaks a newer rule
`CreateVersion` SHALL check its base with the earlier rule set of ADR-0120. It SHALL copy a base whose value and limit faults all come from ADR-0114, ADR-0117, or ADR-0139. It SHALL copy a stored S7 address that breaks ADR-0116 or ADR-0140. `CreateVersion` SHALL refuse every other fault of the base with `E-RCP-020` and create no draft (R12, R38).

#### Scenario: New draft from an affected approved version
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in `DUE` and the stored `REAL` limit `max` `100.00001` in `RATE`
- **WHEN** a ProcessEngineer runs `CreateVersion` with the base version 2
- **THEN** the system creates the draft version 3 with the value `72700` and the limit `max` `100.00001`

#### Scenario: New draft from a base with an affected address
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the parameter `SPEED` of type `DINT` at `DB1.DBD65534`
- **WHEN** a ProcessEngineer runs `CreateVersion` with the base version 2
- **THEN** the system creates the draft version 3 with `SPEED` at `DB1.DBD65534`

#### Scenario: Approval of the copied draft before the fix
- **GIVEN** the draft version 3 of `R-100` that `CreateVersion` copied from a base with the stored `DATE` value `72700` in `DUE`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-005` for `DUE`

#### Scenario: New draft from a base with a DATE after the earlier range
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72900` in `DUE`
- **WHEN** a ProcessEngineer runs `CreateVersion` with the base version 2
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the system creates no draft

#### Scenario: New draft from a base with an LDT after the current range
- **GIVEN** the approved version 2 of `R-100` on the platform `TwinCat3` with the stored `LDT` value `10413792000000000000` in `STAMP`
- **WHEN** a ProcessEngineer runs `CreateVersion` with the base version 2
- **THEN** the system creates the draft version 3 with the value `10413792000000000000`

#### Scenario: New draft from a base with a data block above 59999
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the parameter `SPEED` of type `INT` at `DB60000.DBW0`
- **WHEN** a ProcessEngineer runs `CreateVersion` with the base version 2
- **THEN** the system creates the draft version 3 with `SPEED` at `DB60000.DBW0`

## RENAMED Requirements

- FROM: `### Requirement: Data block number on S7_300`
- TO: `### Requirement: Data block number per S7 platform`
