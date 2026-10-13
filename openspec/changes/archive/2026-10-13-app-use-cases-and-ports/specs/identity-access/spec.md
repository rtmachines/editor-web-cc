# Spec Delta: identity-access

## MODIFIED Requirements

### Requirement: Forbidden result event
The system SHALL write one security event of kind `forbidden` for a `Forbidden` result and then return `E-WEB-002`. When the system cannot write that event, it SHALL return the code of the storage error instead. The use case SHALL change no data in both cases (R28, R35, A33).

#### Scenario: Forbidden security event
- **GIVEN** the user `mhoffman` with the role `Operator`
- **WHEN** an Operator runs `RenameMachine` on `M12`
- **THEN** the system rejects the command with `E-WEB-002`
- **AND** the system writes a security event of kind `forbidden` for the user `mhoffman`

#### Scenario: Security event storage does not answer
- **GIVEN** the user `mhoffman` with the role `Operator`
- **AND** a database that does not answer the write of the security event
- **WHEN** an Operator runs `RenameMachine` on `M12`
- **THEN** the system rejects the command with `E-DB-005`
- **AND** the name of `M12` does not change
