## MODIFIED Requirements

### Requirement: Audit entry in the change transaction
The system SHALL write at least one audit entry in the same transaction as the change of every command. `domain-model.md` section 5 names the cases without an entry. The status report of an adapter is not a command and SHALL write no entry. A repeated load request, claim, result, cancel, or release that changes nothing SHALL write no entry. The system SHALL NOT commit a change without its audit entry (R20, I13, A55).

#### Scenario: Rename of a live machine
- **GIVEN** the live machine `M12`
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M12` with a new name
- **THEN** the system commits the new name and one audit entry in one transaction
- **AND** the audit entry holds the action `MachineRenamed`

#### Scenario: Repeated claim
- **GIVEN** a load request of `M12` in the state `claimed` by `plc-m12`
- **WHEN** `plc-m12` claims the request again
- **THEN** the system commits no change
- **AND** the audit log holds no new entry

### Requirement: Aggregate type of an audit entry
The system SHALL write the aggregate type `machine` for a change of a machine, a unit, a PLC type, or a load request. The system SHALL write the aggregate type `recipe` for a change of a recipe, a version, or a parameter (R20, A55).

#### Scenario: Unit added to a machine
- **GIVEN** the live machine `M12` with the unit `U1`
- **WHEN** a ProcessEngineer runs `AddUnit` on `M12` with the parent path `U1` and the segment `2`
- **THEN** the audit entry holds the aggregate type `machine` and the aggregate id of `M12`

#### Scenario: Claim of a load request
- **GIVEN** a `pending` load request of `M12` before its claim deadline
- **WHEN** `plc-m12` claims it
- **THEN** the audit entry holds the aggregate type `machine`, the aggregate id of `M12`, and the entity type `load_request`
