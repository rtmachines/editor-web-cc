# Spec Delta: audit

## ADDED Requirements

### Requirement: Full row with child rows
The full row of a parameter in the audit actions `ParameterAdded` and `ParameterRemoved` SHALL hold the parameter row, its limit rows, and its translation rows. The full row of a struct in the audit action `UdtDefined` SHALL hold the type row and its member rows. A command SHALL write no separate audit entry for these child rows (R20).

#### Scenario: Parameter added with limits
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `SPEED` and the limit `min 10, max 250` on the leaf path `""`
- **THEN** the system writes one audit entry with the action `ParameterAdded`
- **AND** the `after_state` holds the row of `SPEED` and its limit row with the leaf path `""`

#### Scenario: `RemoveParameter` with a translation
- **GIVEN** the parameter `SPEED` of the draft version 3 of `R-100` with a translation for `pl`
- **WHEN** a ProcessEngineer runs `RemoveParameter` on `SPEED`
- **THEN** the system writes one audit entry with the action `ParameterRemoved`
- **AND** the `before_state` holds the row of `SPEED` and its translation row for `pl`

#### Scenario: Struct defined with two members
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Zone` with the members `setpoint` of type `REAL` and `label` of type `STRING(16)`
- **THEN** the audit entry `UdtDefined` holds the type row of `ST_Zone@1`
- **AND** the `after_state` holds the member rows `setpoint` and `label` in their order
