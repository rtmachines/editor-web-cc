# Spec Delta: recipe-management

## MODIFIED Requirements

### Requirement: Type change needs a new value
When `UpdateParameterDefinition` changes the type of a parameter, the request SHALL carry a new value and new limits. Without a type change, the value and the limits SHALL stay, unless the request carries a new value and new limits. Then the system SHALL replace them and check them in the same way as after a type change (R08, R09, R19, I7).

#### Scenario: Type change without a value
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `DINT` and no value
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Same type without a value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, a new name, and no value
- **THEN** the value and the limits of `SPEED` stay unchanged

#### Scenario: Same type with a new value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `130`, and the limit `min 10, max 250`
- **THEN** the value of `SPEED` is `130`
- **AND** the audit entry `ParameterDefinitionUpdated` holds the old and the new value

#### Scenario: Same type with a value outside the new limit
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `300`, and the limit `min 10, max 250`
- **THEN** the system rejects the command with `E-RCP-004`
- **AND** the parameter `SPEED` does not change
