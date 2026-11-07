## MODIFIED Requirements

### Requirement: Canonical decimal text of a value
The system SHALL read the JSON number of a numeric leaf or of a numeric limit as its exact decimal value. The system SHALL store each numeric, duration, or instant leaf as canonical decimal text with no exponent and no trailing zeros. The value `-0` SHALL become `0`. A zero SHALL give `0` with each exponent. Trailing zeros of the digits SHALL not count. A number with more than 28 significant digits or more than 28 fraction digits SHALL give `E-RCP-005` for a value and `E-RCP-006` for a limit. A number outside the range of `Decimal`, such as `1e29`, SHALL also give these codes (R09, R38, I7).

#### Scenario: Number with an exponent
- **GIVEN** the parameter `RATE` of type `REAL` with the process limit `min` 0 and `max` 500 in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1.5e2`
- **THEN** the system stores the value `150`

#### Scenario: Exponent below minus 28 with trailing zeros
- **GIVEN** the parameter `RATE` of type `REAL` without a process limit in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `150e-29`
- **THEN** the system stores the value `0.0000000000000000000000000015`

#### Scenario: Exponent minus 30 with two trailing zeros
- **GIVEN** the parameter `RATE` of type `REAL` without a process limit in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `100e-30`
- **THEN** the system stores the value `0.0000000000000000000000000001`

#### Scenario: Fraction of 29 zeros
- **GIVEN** the parameter `SPEED` of type `INT` without a process limit in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the JSON number `1.` followed by 29 zeros
- **THEN** the system stores the value `1`

#### Scenario: Zero with an exponent above the range of a 64-bit integer
- **GIVEN** the parameter `RATE` of type `REAL` without a process limit in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `0e99999999999999999999`
- **THEN** the system stores the value `0`

#### Scenario: Number with 29 fraction digits
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1e-29`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Number outside the range of Decimal
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1e29`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Limit bound with an exponent below minus 28
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `RATE` with the `min` value `150e-29`
- **THEN** the system stores the limit `min` `0.0000000000000000000000000015`

#### Scenario: Limit bound with 29 fraction digits
- **GIVEN** the parameter `RATE` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `RATE` with the `min` value `1e-29`
- **THEN** the system rejects the command with `E-RCP-006`

## ADDED Requirements

### Requirement: Count rules of a stored time limit
The system SHALL check each bound of a stored limit of a `Duration` or `Instant` leaf with the count rules of a stored value of that leaf. An `S5TIME` bound SHALL be one of its steps. An `LDATE` bound SHALL be a whole number of days. A bound that breaks one of these rules SHALL also fail the earlier rule set of ADR-0120 (R09, R12, A43).

#### Scenario: New draft from a base with an S5TIME limit between two steps
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored `S5TIME` limit `max` `15` in the parameter `DELAY`
- **WHEN** a ProcessEngineer runs `CreateVersion` with the base version 2
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-006` for `DELAY`
- **AND** the system creates no draft

#### Scenario: Approval of a draft with an S5TIME limit between two steps
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `S5TIME` limit `max` `15` in the parameter `DELAY`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-006` for `DELAY`

#### Scenario: Read of a version with an LDATE limit inside a day
- **GIVEN** the approved version 2 of `R-100` on the platform `CodesysIec` with the stored `LDATE` limit `max` `1` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `GetParameter` on `DUE` of version 2
- **THEN** the system returns `500 E-DB-008`

### Requirement: Literal of a time bound in a message
An error message of a value check or of a limit check SHALL show a `Duration` or `Instant` value and its bounds as canonical IEC literals. This SHALL also apply to a count that only the earlier rule set of ADR-0120 accepts, in each rule set of the check (R36, A43).

#### Scenario: Stored DATE value above a new limit
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` with the `max` value `D#2168-12-31`
- **THEN** the system rejects the command with `E-RCP-004`
- **AND** the message shows the value `D#2169-01-17` and the limit `D#1990-01-01` to `D#2168-12-31`

#### Scenario: New DATE value below a stored limit of the earlier rule set
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` limit `min` `72700` and `max` `72800` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2168-12-31`
- **THEN** the system rejects the command with `E-RCP-004`
- **AND** the message shows the value `D#2168-12-31` and the limit `D#2169-01-17` to `D#2169-04-27`

### Requirement: Ends of the integer type ranges
The system SHALL accept each end of the range of each of the 12 integer types as a value and as a limit bound. A number one step outside an end SHALL give `E-RCP-005` for a value and `E-RCP-006` for a limit (R11, R12, R38).

#### Scenario: Lowest LINT value
- **GIVEN** the parameter `OFFSET` of type `LINT` without a process limit in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `OFFSET` with the JSON number `-9223372036854775808`
- **THEN** the system stores the value `-9223372036854775808`

#### Scenario: ULINT value above the type range
- **GIVEN** the parameter `TOTAL` of type `ULINT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `TOTAL` with the JSON number `18446744073709551616`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Highest ULINT limit
- **GIVEN** the parameter `TOTAL` of type `ULINT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `TOTAL` with the `max` value `18446744073709551615`
- **THEN** the system stores the limit `max` `18446744073709551615`

#### Scenario: SINT limit below the type range
- **GIVEN** the parameter `TRIM` of type `SINT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `TRIM` with the `min` value `-129`
- **THEN** the system rejects the command with `E-RCP-006`

### Requirement: S5TIME steps at each tier edge
The system SHALL accept an `S5TIME` count only when it is a multiple of the step of its tier in the base-unit table of `domain-model.md` §2.1. Each tier edge SHALL belong to the lower tier. A count between two steps SHALL give `E-RCP-005` for a value (R12, A43).

#### Scenario: Last count of the 10 ms tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#9s990ms`
- **THEN** the system stores the value `9990`

#### Scenario: First step of the 100 ms tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#10s`
- **THEN** the system stores the value `10000`

#### Scenario: Last count of the 100 ms tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#1m39s900ms`
- **THEN** the system stores the value `99900`

#### Scenario: Last count of the 1 s tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#16m39s`
- **THEN** the system stores the value `999000`

#### Scenario: 10 ms step inside the 100 ms tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#10s10ms`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: 100 ms step inside the 1 s tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#1m40s100ms`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: 1 s step inside the 10 s tier
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the value `S5T#16m41s`
- **THEN** the system rejects the command with `E-RCP-005`
