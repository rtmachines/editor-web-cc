## ADDED Requirements

### Requirement: DATE range on S7 platforms
On `S7_300`, `S7_1200`, and `S7_1500`, the system SHALL accept a `DATE` value only from 1990-01-01 to 2168-12-31. The count of days since 1970-01-01 SHALL be 7 305 to 72 683. A value outside this range SHALL give `E-RCP-005`. A limit outside this range SHALL give `E-RCP-006` (R12, R38, A43).

#### Scenario: Last day of the DATE range on S7_1500
- **GIVEN** the parameter `DUE` of type `DATE` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2168-12-31`
- **THEN** the system stores the value `72683`

#### Scenario: First day after the DATE range on S7_1200
- **GIVEN** the parameter `DUE` of type `DATE` in the draft version 3 of `R-100` on the platform `S7_1200`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2169-01-01`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: DATE limit after the range on S7_1500
- **GIVEN** the parameter `DUE` of type `DATE` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` with the `max` value `D#2169-06-06`
- **THEN** the system rejects the command with `E-RCP-006`

#### Scenario: Stored DATE after the range at approval
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-005` for `DUE`

### Requirement: Ranges narrower than the vendor range
The system SHALL keep three ranges that are narrower than the Siemens range. A `DATE_AND_TIME` value SHALL have whole seconds. A `DTL` value SHALL end at 2262-04-11-23:47:16.854775807. A `REAL` or `LREAL` value SHALL stay inside the range of a 96-bit decimal with 28 significant digits. A value outside these ranges SHALL give `E-RCP-005` (R12, R38, A43).

#### Scenario: DATE_AND_TIME with milliseconds
- **GIVEN** the parameter `START` of type `DATE_AND_TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `START` with the value `DT#2026-10-12-12:30:00.500`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Last DTL instant
- **GIVEN** the parameter `STAMP` of type `DTL` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `STAMP` with the value `DTL#2262-04-11-23:47:16.854775807`
- **THEN** the system stores the value `9223372036854775807`

#### Scenario: DTL after the last instant
- **GIVEN** the parameter `STAMP` of type `DTL` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `STAMP` with the value `DTL#2262-04-11-23:47:16.854775808`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: REAL above the decimal range
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1e29`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Significant digits of a REAL or LREAL number
On every platform, a `REAL` number SHALL have at most 7 significant digits, and an `LREAL` number at most 15. The rule SHALL apply to a value leaf and to the `min` and `max` of a limit. The system SHALL count the digits from the first to the last digit that is not zero. More digits SHALL give `E-RCP-055`. The system SHALL not round a number (R12, R38).

#### Scenario: REAL value with 8 significant digits
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `16777217`
- **THEN** the system rejects the command with `E-RCP-055`

#### Scenario: REAL value with 7 significant digits
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1234.567`
- **THEN** the system stores the value `1234.567`

#### Scenario: Zeros at the end are not significant
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1000000000`
- **THEN** the system stores the value `1000000000`

#### Scenario: LREAL value with 16 significant digits
- **GIVEN** the parameter `GAIN` of type `LREAL` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `GAIN` with the JSON number `0.1234567890123456`
- **THEN** the system rejects the command with `E-RCP-055`

#### Scenario: REAL limit with 8 significant digits
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `RATE` with the `max` value `100.00001`
- **THEN** the system rejects the command with `E-RCP-055`

#### Scenario: Stored REAL value with 8 digits at approval
- **GIVEN** the draft version 3 of `R-100` with the stored `REAL` value `16777217` in the parameter `RATE`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-055` for `RATE`

### Requirement: End of an S7 address
On every platform with an S7 address, the offset plus the S7 layout size of the type SHALL be at most 65 536 bytes. A parameter that ends after the last byte of the data block SHALL give `E-RCP-056`. `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` SHALL apply the rule to the parameters that they write. `ApproveVersion` SHALL apply it to every parameter (R14, R38, A31).

#### Scenario: STRING that ends at the last byte
- **GIVEN** the machine `M7` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M7` with the type `STRING(254)` and the address `DB1.DBB65280`
- **THEN** the system adds the parameter

#### Scenario: STRING that ends after the last byte
- **GIVEN** the machine `M7` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M7` with the type `STRING(254)` and the address `DB1.DBB65282`
- **THEN** the system rejects the command with `E-RCP-056`

#### Scenario: DINT at the last word
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `DINT` and the address `DB1.DBD65534`
- **THEN** the system rejects the command with `E-RCP-056`

#### Scenario: BOOL at the last bit
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `BOOL` and the address `DB1.DBX65535.7`
- **THEN** the system adds the parameter

#### Scenario: Stored address after the end at approval
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBD65534`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-056`

### Requirement: Data block number on S7_300
On `S7_300`, the data block number of an S7 address SHALL be at most 16 000. A higher number SHALL give `E-RCP-057`. On `S7_1200` and `S7_1500`, the data block number SHALL stay 1 to 65 535. The system SHALL apply the rule to the same parameters as the end rule (R14, R38, A31).

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

### Requirement: Draft commands on stored data that breaks a newer rule
A draft command SHALL check the value or the limit that it writes with the current rules. It SHALL check a stored value or limit that it reads and does not write with the earlier rule set of ADR-0120. The process-limit check between the two parts SHALL still run. A draft command SHALL apply the S7 address rules only to the parameters that it writes (R12, R14, R38).

#### Scenario: Command on another parameter
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on the parameter `RATE` of type `REAL` with the JSON number `12.5`
- **THEN** the system stores the value `12.5`

#### Scenario: Fix of the value before the limit
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored value `72700` and the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2168-12-31`
- **THEN** the system stores the value `72683`

#### Scenario: Fix of the limit after the value
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored value `72683` and the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` with the `max` value `D#2168-12-31`
- **THEN** the system stores the limit `max` `72683`

#### Scenario: Fix of the limit before the value
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored value `72700` and the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` without a `max` value
- **THEN** the system stores the limits of `DUE` without `max`

#### Scenario: New value that breaks the current range
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2169-01-17`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: New value outside an affected stored limit
- **GIVEN** the draft version 3 of `R-100` with the stored `REAL` limit `max` `100.00001` in the parameter `RATE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `200`
- **THEN** the system rejects the command with `E-RCP-004`

#### Scenario: New parameter next to an affected address
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBD65534`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `FEED`, the type `INT`, and the address `DB2.DBW0`
- **THEN** the system adds the parameter

### Requirement: New draft from a base that breaks a newer rule
`CreateVersion` SHALL check its base with the earlier rule set of ADR-0120. It SHALL copy a base whose value and limit faults all come from ADR-0114 or ADR-0117. It SHALL copy a stored S7 address that breaks ADR-0116. `CreateVersion` SHALL refuse every other fault of the base with `E-RCP-020` and create no draft (R12, R38).

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
