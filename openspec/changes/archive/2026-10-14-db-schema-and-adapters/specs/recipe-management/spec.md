# Spec Delta: recipe-management

## ADDED Requirements

### Requirement: Guard of a version status write
`DiscardDraft`, `RetireVersion`, and `ApproveVersion` SHALL write a status change only to a version row that the same transaction locked. The write SHALL name the row version that the lock read. A status write that matches no row SHALL stop the command with `E-DB-008` and SHALL write nothing (R17, R29, I12).

#### Scenario: Approval of a locked draft
- **GIVEN** the draft version 3 of `R-100` with the row version 5
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 with the expected row version 5
- **THEN** the system approves version 3
- **AND** the row version of version 3 is 6

#### Scenario: Status write without the lock
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a transaction writes the discard of version 3 without a lock of version 3
- **THEN** the system rejects the write with `E-DB-008`
- **AND** version 3 stays a live draft
