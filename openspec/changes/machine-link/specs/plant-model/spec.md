## MODIFIED Requirements

### Requirement: Archive of a machine
The system SHALL archive a machine only when no recipe of the machine has a live draft. The machine SHALL also have no load request in the effective state `claimed` or `unknown`. Such a request SHALL give `409 E-LNK-003`. The recipes of an archived machine SHALL stay readable and exportable (R05, A14, A55).

#### Scenario: Machine with a live draft
- **GIVEN** the machine `M12` and its recipe `R-100` with a live draft
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** the system rejects the command with `E-PLT-011`
- **AND** `M12` stays live

#### Scenario: Machine without a live draft
- **GIVEN** the machine `M12` and its recipe `R-100` with an approved version and no draft
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** the system sets the archive time of `M12`
- **AND** the recipe `R-100` and its versions stay readable
- **AND** the audit log holds the action `MachineArchived`

#### Scenario: Machine with a claimed request
- **GIVEN** the machine `M12` without a live draft and a load request of `M12` in the state `claimed`
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** the system rejects the command with `E-LNK-003`
- **AND** `M12` stays live

#### Scenario: Machine with an unknown request
- **GIVEN** the machine `M12` without a live draft and a load request of `M12` in the stored state `claimed` after its result deadline
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** the system rejects the command with `E-LNK-003`
- **AND** `M12` stays live

## ADDED Requirements

### Requirement: Archive of a machine with a pending load request
`ArchiveMachine` SHALL end a `pending` load request of the machine in its commit. A live `pending` request, before its claim deadline, SHALL get the state `cancelled` and one `RecipeLoadCancelled` entry. The actor of that entry SHALL be the person who runs `ArchiveMachine`. An overdue `pending` request SHALL get the state `expired` and one `RecipeLoadExpired` entry (R05, R20, A55).

#### Scenario: Machine with a pending request
- **GIVEN** the machine `M12` without a live draft and a load request of `M12` in the state `pending` before its claim deadline
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** the system sets the archive time of `M12`
- **AND** the load request has the state `cancelled`
- **AND** the audit log holds one `RecipeLoadCancelled` entry and one `MachineArchived` entry

#### Scenario: Machine with an overdue pending request
- **GIVEN** the machine `M12` without a live draft and a load request of `M12` in the stored state `pending` after its claim deadline
- **WHEN** a ProcessEngineer runs `ArchiveMachine` on `M12`
- **THEN** the system sets the archive time of `M12`
- **AND** the load request has the state `expired`
- **AND** the audit log holds one `RecipeLoadExpired` entry and one `MachineArchived` entry
