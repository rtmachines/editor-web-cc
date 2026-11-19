## MODIFIED Requirements

### Requirement: Diff state of five actions
The system SHALL store in `before_state` and `after_state` only the changed fields of the actions `ParameterDefinitionUpdated`, `ParameterValueSet`, `LimitsReplaced`, `TranslationUpserted`, and `TypeRevisionRebound`. One exception applies. When `ParameterDefinitionUpdated` replaces a type without a tree, the system SHALL store `before_state.value` and `after_state.value`, also for equal sides (ADR-0195) (R20).

#### Scenario: Value of a parameter
- **GIVEN** the draft of the recipe `R-100` with a parameter
- **WHEN** a ProcessEngineer runs `SetParameterValue` on that parameter with a new value
- **THEN** the audit entry holds the action `ParameterValueSet`
- **AND** the `before_state` and the `after_state` hold only the changed fields

#### Scenario: New type for a parameter whose type has no tree
- **GIVEN** the draft of the recipe `R-100` with the parameter `ZONE` of a stored struct type that contains itself, and the stored value `0`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `ZONE` with the type `INT` and the value `0`
- **THEN** the audit entry holds the action `ParameterDefinitionUpdated`
- **AND** `before_state.value` holds the stored value `0` under the key `""`
- **AND** `after_state.value` holds the value `0` of the leaf of `INT`
