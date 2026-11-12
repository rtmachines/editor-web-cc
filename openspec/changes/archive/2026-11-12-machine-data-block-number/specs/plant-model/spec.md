## ADDED Requirements

### Requirement: Data block number of a machine
Each live machine with the platform `S7_300`, `S7_1200`, or `S7_1500` SHALL have exactly one data block number. An archived S7 machine from before this change MAY have no number. A machine with the platform `CodesysIec` or `TwinCat3` SHALL have no data block number. The number is the default data block for a parameter without `s7_address`. The number MUST NOT change the address of a parameter (R14, R15, A14, A48).

#### Scenario: Recipe with several data blocks
- **GIVEN** the machine `M5` with the data block number `100` and a PLC connection record
- **AND** the approved version of `R-200` of `M5` with the addresses `DB1.DBW0` and `DB2.DBW0`
- **WHEN** the transfer client exports the version
- **THEN** the document holds the addresses `DB1.DBW0` and `DB2.DBW0` unchanged

#### Scenario: Database refuses an S7 machine without a number
- **GIVEN** the role `recipe_app`
- **WHEN** an `INSERT` statement adds a machine with the platform `S7_1500` and no `db_number`
- **THEN** the database rejects the statement with SQLSTATE `23514`

#### Scenario: Database refuses a number on an IEC machine
- **GIVEN** the role `recipe_app`
- **WHEN** an `INSERT` statement adds a machine with the platform `TwinCat3` and the `db_number` `10`
- **THEN** the database rejects the statement with SQLSTATE `23514`

#### Scenario: Page of an archived S7 machine without a number
- **GIVEN** the archived machine `M4` with the platform `S7_300` and no data block number
- **WHEN** a user with `RecipeView` opens the page of `M4`
- **THEN** the page shows that `M4` has no data block number

### Requirement: Data block number at machine creation
`CreateMachine` SHALL require a data block number on `S7_300`, `S7_1200`, and `S7_1500`. It SHALL refuse a data block number on `CodesysIec` and `TwinCat3`. The system SHALL treat an empty field text as no value (R15, R38, A48).

#### Scenario: Create an S7-1500 machine with a number
- **GIVEN** no live machine `M7`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M7`, the platform `S7_1500`, the data block number `100`, and the main unit name `Infeed`
- **THEN** the system creates the machine `M7` with the data block number `100`
- **AND** the entry `MachineCreated` holds the data block number `100`

#### Scenario: S7 machine without a number
- **GIVEN** no live machine `M7`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M7`, the platform `S7_300`, and an empty field `db_number`
- **THEN** the system rejects the command with `422 E-PLT-050` for the field `db_number`
- **AND** the system creates no machine

#### Scenario: IEC machine with a number
- **GIVEN** no live machine `M8`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M8`, the platform `CodesysIec`, and the data block number `10`
- **THEN** the system rejects the command with `422 E-PLT-051` for the field `db_number`

#### Scenario: IEC machine with an empty field
- **GIVEN** no live machine `M8`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M8`, the platform `TwinCat3`, and an empty field `db_number`
- **THEN** the system creates the machine `M8` without a data block number

### Requirement: Data block limit of the platform
The system SHALL accept a data block number from 1 to 16000 on `S7_300` and from 1 to 59999 on `S7_1200` and `S7_1500`. `CreateMachine` and `SetMachineDataBlock` SHALL apply the same limits (R14, R38, A48).

#### Scenario: Highest number on S7-300
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M3` with the number `16000`
- **THEN** the system stores the number `16000`

#### Scenario: Number above the S7-300 limit
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M3` with the number `16001`
- **THEN** the system rejects the command with `422 E-PLT-024`
- **AND** the message names the highest number `16000`

#### Scenario: Number above the S7-1500 limit at creation
- **GIVEN** no live machine `M5`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M5`, the platform `S7_1500`, and the number `60000`
- **THEN** the system rejects the command with `422 E-PLT-024`
- **AND** the message names the highest number `59999`

#### Scenario: Number outside the grammar
- **WHEN** a ProcessEngineer sends the data block number `0` or `65536`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `db_number`

### Requirement: Change of the data block number
`SetMachineDataBlock` SHALL set the data block number of a live machine on `S7_300`, `S7_1200`, or `S7_1500`. Each successful run SHALL write one audit entry `MachineDataBlockSet` with the machine before and after the change in its transaction (R20, R29, A48).

#### Scenario: New number of an S7 machine
- **GIVEN** the live machine `M5` with the platform `S7_1500` and the data block number `100`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M5` with the number `200`
- **THEN** the machine `M5` has the data block number `200`
- **AND** the audit log holds one entry `MachineDataBlockSet` with `100` before and `200` after

#### Scenario: Empty number
- **GIVEN** the live machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M5` with an empty field `db_number`
- **THEN** the system rejects the command with `422 E-PLT-050`
- **AND** the number of `M5` does not change

#### Scenario: Number on an IEC machine
- **GIVEN** the live machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M8` with the number `10`
- **THEN** the system rejects the command with `422 E-PLT-051`
- **AND** the audit log does not change

#### Scenario: Number of an archived machine
- **GIVEN** the archived machine `M3` and no live machine with the code `M3`
- **WHEN** a ProcessEngineer sends `POST /machines/M3/data-block`
- **THEN** the system answers `404 E-WEB-005`
- **AND** the number of `M3` does not change

#### Scenario: Failed audit insert
- **GIVEN** the audit insert of `SetMachineDataBlock` fails
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M5` with the number `200`
- **THEN** the number of `M5` does not change

### Requirement: Permission to set the data block number
`SetMachineDataBlock` SHALL check `MachineManage` as the first statement (R28, A9).

#### Scenario: Operator sets a number
- **GIVEN** a user with the role Operator
- **WHEN** the user sends `POST /machines/M5/data-block`
- **THEN** the system answers `403 E-WEB-002`
- **AND** the system writes one `security_event` row and does not change the machine

### Requirement: Data block number in read-only mode
In read-only mode, `SetMachineDataBlock` SHALL return `503 E-WEB-006` and write nothing (A24).

#### Scenario: Set in read-only mode
- **GIVEN** the system is in read-only mode
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M5`
- **THEN** the system answers `503 E-WEB-006`
- **AND** the machine and the audit log do not change

### Requirement: Data block number in the connection document
The connection document of an S7 machine SHALL hold the data block number of the machine in the field `connection.db_number` (A47, A48).

#### Scenario: Read after a change of the number
- **GIVEN** the machine `M5` with a PLC connection record and the recipe `R-200`
- **AND** a ProcessEngineer ran `SetMachineDataBlock` on `M5` with the number `200`
- **WHEN** the `ExportClient` account reads the connection of `M5` for `R-200`
- **THEN** the field `connection.db_number` of the answer is `200`

### Requirement: Copy of the data block number in the connection record
Until a later change removes the column `plc_connection.db_number`, each command SHALL keep that column equal to the data block number of the machine. The previous release reads that column after a rollback. A number that the previous release saves in that column SHALL also become the data block number of the machine (R31, A48).

#### Scenario: Copy after a change of the number
- **GIVEN** the machine `M5` with a PLC connection record and the data block number `100`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M5` with the number `200`
- **THEN** the column `plc_connection.db_number` of `M5` is `200`

#### Scenario: Copy in a new connection record
- **GIVEN** the machine `M5` with the data block number `100` and no PLC connection record
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M5` with the host, the rack, and the slot
- **THEN** the column `plc_connection.db_number` of the new record is `100`

#### Scenario: Current number keeps the connection record
- **GIVEN** the machine `M5` with a PLC connection record at `row_version` `3` and the data block number `100`
- **WHEN** a ProcessEngineer runs `SetMachineDataBlock` on `M5` with the number `100`
- **THEN** the PLC connection record of `M5` stays at `row_version` `3`
- **AND** the audit log holds one new entry `MachineDataBlockSet`

#### Scenario: The previous release saves a new data block number
- **GIVEN** the machine `M5` with the platform `S7_1500`, a PLC connection record, and the data block number `100`
- **WHEN** the previous release saves the PLC connection of `M5` with the data block number `200`
- **THEN** the machine `M5` has the data block number `200`
- **AND** the connection document of `M5` holds `200` in the field `connection.db_number`

### Requirement: Migration of the data block number
The migration SHALL copy the data block number of each PLC connection record into its machine. When a live S7 machine has no PLC connection record, the migration SHALL stop before it changes the schema. The message SHALL name the code of each such machine (R31, A48).

#### Scenario: Copy of a stored number
- **GIVEN** the machine `M5` with the platform `S7_1500` and a PLC connection record with the data block number `100`
- **WHEN** the migrator applies the migrations of this change
- **THEN** the machine `M5` has the data block number `100`

#### Scenario: S7 machine without a connection record
- **GIVEN** the live machine `M12` with the platform `S7_1500` and no PLC connection record
- **WHEN** the migrator applies the migrations of this change
- **THEN** the migration fails with a message that names `M12`
- **AND** the database holds no column `machine.db_number`

## MODIFIED Requirements

### Requirement: Machine with one PLC platform and a first unit
The system SHALL create a machine with exactly one PLC platform and with the main unit `U1` in one transaction. The PLC platform SHALL be `CodesysIec`, `TwinCat3`, `S7_1200`, `S7_1500`, or `S7_300`. A machine on `S7_300`, `S7_1200`, or `S7_1500` SHALL get its data block number in the same transaction (R06, R15, A48).

#### Scenario: Create a machine
- **GIVEN** no live machine `M7`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M7`, the platform `S7_300`, the data block number `10`, and the main unit name `Infeed`
- **THEN** the system creates the machine `M7` with the platform `S7_300` and the data block number `10`
- **AND** the system creates the unit `U1` with the name `Infeed`
- **AND** the audit log holds the actions `MachineCreated` and `UnitAdded`

### Requirement: PLC connection record of a machine
The system SHALL keep at most one PLC connection record for each machine. The system SHALL store the record only. It MUST NOT open a network connection to a PLC (R03, A47).

#### Scenario: Machine without a record
- **GIVEN** the live machine `M12` without a PLC connection record
- **WHEN** a user with `RecipeView` opens the page of `M12`
- **THEN** the page shows that `M12` has no PLC connection

#### Scenario: First record of a machine
- **GIVEN** the live machine `M12` with the platform `S7_1500`, the data block number `100`, and no record
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M12` with the host `10.0.0.5`, the rack `0`, and the slot `1`
- **THEN** the system stores the record of `M12` with `row_version` 1
- **AND** the system sends no network traffic to `10.0.0.5`

### Requirement: Connection fields of the platform
The system SHALL accept a field only when the platform of the machine uses it. `S7_300`, `S7_1200`, and `S7_1500` use the host, the rack, and the slot. `CodesysIec` and `TwinCat3` use the host, the port, the address, the mode, the login, and the password. The system SHALL treat an empty field text as no value (R15, A47).

#### Scenario: All fields of an S7 platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` with the host, the rack, and the slot
- **THEN** the system stores the record

#### Scenario: Field of another platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` with all S7 fields and the port `502`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `port`
- **AND** the record of `M3` does not change

#### Scenario: Missing field of the platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` without the slot
- **THEN** the system rejects the command with `422 E-PLT-023` for the field `slot`

#### Scenario: Data block number in a connection request
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer sends `POST /machines/M3/connection` with all S7 fields and the field `db_number`
- **THEN** the system rejects the request with one line `422 E-WEB-008`
- **AND** the record of `M3` does not change

#### Scenario: Empty field of another platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` with all S7 fields and an empty port field
- **THEN** the system stores the record

#### Scenario: Several bad fields
- **GIVEN** the live machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `OpcUa`, the rack `0`, and no login
- **THEN** the answer lists `E-PLT-022` for `rack` and `E-PLT-023` for `login` together

## REMOVED Requirements

### Requirement: Data block number of the connection
**Reason**: The data block number moves from the PLC connection record to the machine (ADR-0156).
**Migration**: The requirements "Data block number of a machine" and "Data block limit of the platform" hold the rules. `SetMachineDataBlock` changes the number.
