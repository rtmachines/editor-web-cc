## ADDED Requirements

### Requirement: PLC connection record of a machine
The system SHALL keep at most one PLC connection record for each machine. The system SHALL store the record only. It MUST NOT open a network connection to a PLC (R03, A47).

#### Scenario: Machine without a record
- **GIVEN** the live machine `M12` without a PLC connection record
- **WHEN** a user with `RecipeView` opens the page of `M12`
- **THEN** the page shows that `M12` has no PLC connection

#### Scenario: First record of a machine
- **GIVEN** the live machine `M12` with the platform `S7_1500` and without a record
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M12` with the host `10.0.0.5`, the rack `0`, the slot `1`, and the data block number `100`
- **THEN** the system stores the record of `M12` with `row_version` 1
- **AND** the system sends no network traffic to `10.0.0.5`

### Requirement: Connection fields of the platform
The system SHALL accept a field only when the platform of the machine uses it. `S7_300`, `S7_1200`, and `S7_1500` use the host, the rack, the slot, and the data block number. `CodesysIec` and `TwinCat3` use the host, the port, the address, the mode, the login, and the password. The system SHALL treat an empty field text as no value (R15, A47).

#### Scenario: All fields of an S7 platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` with the host, the rack, the slot, and the data block number
- **THEN** the system stores the record

#### Scenario: Field of another platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` with all S7 fields and the port `502`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `port`
- **AND** the record of `M3` does not change

#### Scenario: Missing field of the platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` without the data block number
- **THEN** the system rejects the command with `422 E-PLT-023` for the field `db_number`

#### Scenario: Empty field of another platform
- **GIVEN** the live machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M3` with all S7 fields and an empty port field
- **THEN** the system stores the record

#### Scenario: Several bad fields
- **GIVEN** the live machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `OpcUa`, the rack `0`, and no login
- **THEN** the answer lists `E-PLT-022` for `rack` and `E-PLT-023` for `login` together

### Requirement: Platform of the connection record
The database SHALL keep the platform of a connection record equal to the platform of its machine. It SHALL refuse a record that holds a field of another platform (R15, A47, I21).

#### Scenario: Record with another platform
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** an `INSERT` statement as `recipe_app` writes a record of `M3` with the platform `TwinCat3`
- **THEN** the database rejects the statement with a foreign key violation

#### Scenario: Record with a field of another platform
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** an `INSERT` statement as `recipe_app` writes a record of `M3` with a port
- **THEN** the database rejects the statement with a check violation

#### Scenario: Record with a login and no password
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** an `INSERT` statement as `recipe_app` writes a record of `M8` with a login and without a ciphertext
- **THEN** the database rejects the statement with a check violation

#### Scenario: Direct change of the row version
- **GIVEN** the record of `M8`
- **WHEN** an `UPDATE` statement as `recipe_app` sets `row_version` of the record
- **THEN** the database rejects the statement with a permission error

### Requirement: PLC host
The system SHALL accept as the host an IPv4 address in dotted decimal form. It SHALL also accept a host name of RFC 1123 with at most 253 characters. It SHALL refuse every other text (R38, A47).

#### Scenario: IPv4 address
- **WHEN** a ProcessEngineer sends the host `192.168.1.20`
- **THEN** the system accepts the host

#### Scenario: Host name
- **WHEN** a ProcessEngineer sends the host `plc-line1.plant.local`
- **THEN** the system accepts the host

#### Scenario: Bad IPv4 address
- **WHEN** a ProcessEngineer sends the host `192.168.1.300`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `host`

#### Scenario: Bad host text
- **WHEN** a ProcessEngineer sends the host `plc_01:102` or `fe80::1`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `host`

#### Scenario: Ambiguous host text
- **WHEN** a ProcessEngineer sends the host `010.0.0.1`, `1.2.3`, or `plc.local.`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `host`

#### Scenario: Bad host label
- **WHEN** a ProcessEngineer sends a host with a label of 64 characters or a label that starts with a hyphen
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `host`

#### Scenario: Empty host
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer sends all S7 fields with an empty host field
- **THEN** the system rejects the command with `422 E-PLT-023` for the field `host`

### Requirement: Rack
The system SHALL accept a rack number from 0 to 7 on `S7_300`, `S7_1200`, and `S7_1500` (R38, A47).

#### Scenario: Rack in range
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer sends the rack `7`
- **THEN** the system accepts the rack

#### Scenario: Rack out of range
- **WHEN** a ProcessEngineer sends the rack `8`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `rack`

#### Scenario: Rack on a platform without a rack
- **GIVEN** the machine `M8` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the rack `0`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `rack`

### Requirement: Slot
The system SHALL accept a slot number from 0 to 31 on `S7_300`, `S7_1200`, and `S7_1500` (R38, A47).

#### Scenario: Slot in range
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer sends the slot `2`
- **THEN** the system accepts the slot

#### Scenario: Slot out of range
- **WHEN** a ProcessEngineer sends the slot `32`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `slot`

#### Scenario: Slot on a platform without a slot
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the slot `1`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `slot`

### Requirement: Data block number of the connection
The system SHALL accept a data block number from 1 to 16000 on `S7_300` and from 1 to 59999 on `S7_1200` and `S7_1500`. The number is the default data block for a parameter without `s7_address`. The record MUST NOT change the address of a parameter (R14, R38, A47).

#### Scenario: Recipe with several data blocks
- **GIVEN** the record of `M5` with the data block number `100`
- **AND** the approved version of `R-200` of `M5` with the addresses `DB1.DBW0` and `DB2.DBW0`
- **WHEN** the transfer client exports the version
- **THEN** the document holds the addresses `DB1.DBW0` and `DB2.DBW0` unchanged

#### Scenario: Highest number on S7-300
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer sends the data block number `16000`
- **THEN** the system accepts the number

#### Scenario: Number above the S7-300 limit
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer sends the data block number `16001`
- **THEN** the system rejects the command with `422 E-PLT-024`
- **AND** the message names the highest number `16000`

#### Scenario: Number above the S7-1500 limit
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer sends the data block number `60000`
- **THEN** the system rejects the command with `422 E-PLT-024`
- **AND** the message names the highest number `59999`

#### Scenario: Number outside the grammar
- **WHEN** a ProcessEngineer sends the data block number `0` or `65536`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `db_number`

#### Scenario: Data block number on an IEC platform
- **GIVEN** the machine `M8` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the data block number `10`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `db_number`

### Requirement: Port
The system SHALL accept a TCP port from 1 to 65535 on `CodesysIec` and `TwinCat3` (R38, A47).

#### Scenario: Port in range
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the port `48898`
- **THEN** the system accepts the port

#### Scenario: Port out of range
- **WHEN** a ProcessEngineer sends the port `0` or `65536`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `port`

#### Scenario: Port on an S7 platform
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer sends the port `102`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `port`

### Requirement: Communication mode
The system SHALL accept the modes `OpcUa`, `CodesysV3`, and `ModbusTcp` on `CodesysIec`. It SHALL accept the modes `Ads`, `OpcUa`, and `ModbusTcp` on `TwinCat3` (R38, A47).

#### Scenario: ADS on TwinCAT 3
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `Ads`
- **THEN** the system accepts the mode

#### Scenario: ADS on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the mode `Ads`
- **THEN** the system rejects the command with `422 E-PLT-025` for the field `comm_mode`

#### Scenario: Unknown mode
- **WHEN** a ProcessEngineer sends the mode `Profinet`
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `comm_mode`

#### Scenario: Mode on an S7 platform
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer sends the mode `OpcUa`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `comm_mode`

### Requirement: Connection address
The system SHALL accept as the address a text of 1 to 200 characters without a control character. With the mode `Ads`, the address MUST be an AMS Net ID. An AMS Net ID has six numbers from 0 to 255 with a dot between two numbers (R38, A47).

#### Scenario: AMS Net ID with ADS
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `Ads` and the address `192.168.1.10.1.1`
- **THEN** the system accepts the address

#### Scenario: Short AMS Net ID with ADS
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `Ads` and the address `192.168.1.10.1`
- **THEN** the system rejects the command with `422 E-PLT-026` for the field `address`

#### Scenario: Address text with OPC UA
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the mode `OpcUa` and the address `opc.tcp://10.0.0.9:4840`
- **THEN** the system accepts the address

#### Scenario: Address text too long
- **WHEN** a ProcessEngineer sends an address of 201 characters
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `address`

#### Scenario: Address on an S7 platform
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer sends the address `10.0.0.5.1.1`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `address`

### Requirement: Connection login
The system SHALL accept as the login a text of 1 to 64 characters without a control character on `CodesysIec` and `TwinCat3`. The modes `OpcUa` and `CodesysV3` MUST have a login. The mode `Ads` permits a login. The mode `ModbusTcp` SHALL refuse a login (R38, A47).

#### Scenario: OPC UA without a login
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the mode `OpcUa` and no login
- **THEN** the system rejects the command with `422 E-PLT-023` for the field `login`

#### Scenario: ADS without a login
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `Ads`, an AMS Net ID, and no login and no password
- **THEN** the system stores the record without a login and without a password

#### Scenario: Modbus TCP with a login
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the mode `ModbusTcp` and the login `adapter`
- **THEN** the system rejects the command with `422 E-PLT-028` for the field `login`

#### Scenario: Login in range
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the login `adapter`
- **THEN** the system accepts the login

#### Scenario: Login too long
- **WHEN** a ProcessEngineer sends a login of 65 characters
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `login`

#### Scenario: Login on an S7 platform
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer sends the login `adapter`
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `login`

### Requirement: Connection password
The system SHALL accept as the password a text of 1 to 128 characters on `CodesysIec` and `TwinCat3`. A record with a login MUST have a password, and a record without a login MUST NOT have one. An empty password field SHALL keep the stored password. A mode without credentials SHALL delete the stored password (R38, A47).

#### Scenario: ADS with a password and no login
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the mode `Ads`, a password, and no login
- **THEN** the system rejects the command with `422 E-PLT-023` for the field `login`

#### Scenario: Modbus TCP with a password
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer sends the mode `ModbusTcp` and a password
- **THEN** the system rejects the command with `422 E-PLT-028` for the field `password`

#### Scenario: Change to a mode without credentials
- **GIVEN** the record of `M9` with the mode `OpcUa`, a login, and a stored password
- **WHEN** a ProcessEngineer saves the record with the mode `ModbusTcp`, no login, and an empty password field
- **THEN** the system stores the record without a login and without a password
- **AND** the audit entry `PlcConnectionSet` holds `password_changed` with the value `true`

#### Scenario: S7 record without a password
- **GIVEN** the record of `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer saves the record with a new host and an empty password field
- **THEN** the system stores the record without a password

#### Scenario: New password
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends a password of 20 characters
- **THEN** the system stores the new password

#### Scenario: Empty field keeps the password
- **GIVEN** the record of `M8` with a stored password
- **WHEN** a ProcessEngineer sends the record with an empty password field
- **THEN** the stored password does not change

#### Scenario: Empty field without a stored password
- **GIVEN** the machine `M8` with the platform `TwinCat3` and without a record
- **WHEN** a ProcessEngineer sends the mode `OpcUa`, the login `adapter`, and an empty password field
- **THEN** the system rejects the command with `422 E-PLT-023` for the field `password`

#### Scenario: Password too long
- **WHEN** a ProcessEngineer sends a password of 129 characters
- **THEN** the system rejects the command with `422 E-WEB-008` for the field `password`

#### Scenario: Password on an S7 platform
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer sends a password
- **THEN** the system rejects the command with `422 E-PLT-022` for the field `password`

### Requirement: Encrypted connection password
The system SHALL store the PLC password only as AES-256-GCM ciphertext. The key MUST come from the Docker secret `plc_credential_key`. The file MUST hold exactly one line with the 32 bytes of the key as 64 lower-case hex characters. The server SHALL refuse to start without such a key (R31, A47).

#### Scenario: Stored password
- **GIVEN** a stored record with the password `S3cret-PLC`
- **WHEN** a query reads the row of the record from the database
- **THEN** no column holds the text `S3cret-PLC`

#### Scenario: Missing key file
- **GIVEN** the file of the secret `plc_credential_key` is in one of four bad states
- **AND** the bad states are a missing file, 62 or 66 hex characters, a character other than lower-case hex, and a second line
- **WHEN** the operator runs `recipe-server serve`
- **THEN** the server writes one `ERROR` line and exits with code 1

#### Scenario: Ciphertext of another machine
- **GIVEN** the records of `M8` and `M9`
- **WHEN** a statement copies the ciphertext of `M9` into the record of `M8`
- **THEN** a read of the password of `M8` fails with `500 E-WEB-014`

### Requirement: Secrecy of the connection password
The system MUST NOT show the PLC password. This covers HTML, a form field value, an audit entry, a log line, an error, and `Debug` output. Only the transfer route of the connection gives the password (R31, A47).

#### Scenario: Edit form
- **GIVEN** the record of `M8` with a stored password
- **WHEN** a ProcessEngineer opens the edit form of the connection of `M8`
- **THEN** the password field is empty
- **AND** the form shows the state of the stored password

#### Scenario: Refused save
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the password `S3cret-PLC` and the port `0`
- **THEN** the answer holds `E-WEB-008` for the field `port`
- **AND** the answer body and the log line do not hold `S3cret-PLC`

#### Scenario: Successful save
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer saves a valid record with the password `S3cret-PLC`
- **THEN** no answer body, log line, or audit entry holds `S3cret-PLC`

#### Scenario: Audit entry of a password change
- **WHEN** a ProcessEngineer changes the password of `M8`
- **THEN** the audit entry `PlcConnectionSet` holds `password_changed` with the value `true`
- **AND** the entry does not hold the password or its ciphertext

### Requirement: Permission to edit the connection
`SetPlcConnection` and its form query `GetPlcConnectionForm` SHALL check `MachineManage` as the first statement (R28, A9).

#### Scenario: Operator saves a connection
- **GIVEN** a user with the role Operator
- **WHEN** the user sends `POST /machines/M8/connection`
- **THEN** the system answers `403 E-WEB-002`
- **AND** the system writes one `security_event` row and no record

#### Scenario: Operator opens the edit form
- **GIVEN** a user with the role Operator
- **WHEN** the user opens `GET /machines/M8/connection/edit`
- **THEN** the system answers `403 E-WEB-002`

### Requirement: Connection edit in read-only mode
In read-only mode, `SetPlcConnection` SHALL return `503 E-WEB-006` and write nothing (A24).

#### Scenario: Save in read-only mode
- **GIVEN** the system is in read-only mode
- **WHEN** a ProcessEngineer runs `SetPlcConnection` on `M8`
- **THEN** the system answers `503 E-WEB-006`
- **AND** the record and the audit log do not change

### Requirement: Concurrent edit of the connection
`SetPlcConnection` SHALL compare the sent `row_version` with the `row_version` of the record. A machine without a record has the `row_version` 0. A mismatch SHALL refuse the command (R29, A47).

#### Scenario: Matching row version
- **GIVEN** the record of `M8` with `row_version` 4
- **WHEN** a ProcessEngineer sends a valid record with `row_version` 4
- **THEN** the system stores the record with `row_version` 5
- **AND** the answer sets `HX-Trigger: {"rowVersion": 5}`

#### Scenario: Stale row version
- **GIVEN** the record of `M8` with `row_version` 5
- **WHEN** a ProcessEngineer sends a valid record with `row_version` 4
- **THEN** the system answers `409 E-WEB-013`
- **AND** the record does not change

#### Scenario: Two first saves
- **GIVEN** the machine `M8` without a record
- **WHEN** two users send a first record with `row_version` 0 at the same time
- **THEN** the system stores one record
- **AND** the other request gets `409 E-WEB-013`

#### Scenario: Two saves of one row version
- **GIVEN** the record of `M8` with `row_version` 4
- **WHEN** two users send a valid record with `row_version` 4 at the same time
- **THEN** the system stores one record with `row_version` 5
- **AND** the other request gets `409 E-WEB-013`

#### Scenario: Save during an archive
- **GIVEN** the live machine `M8` with a record
- **WHEN** a ProcessEngineer saves the record while a second user archives `M8`
- **THEN** either the save commits before the archive, or the save gets `409 E-PLT-010`

### Requirement: Audit of a connection edit
Each successful `SetPlcConnection` SHALL write one audit entry `PlcConnectionSet` in its transaction. The entry holds the record before and after the change without the password (R20, A47).

#### Scenario: Change of the host
- **GIVEN** the record of `M3` with the host `10.0.0.5`
- **WHEN** a ProcessEngineer changes the host to `10.0.0.6`
- **THEN** the audit log holds one entry `PlcConnectionSet` with the aggregate of `M3`
- **AND** the before state holds `10.0.0.5` and the after state holds `10.0.0.6`

#### Scenario: Failed audit insert
- **GIVEN** the audit insert of `SetPlcConnection` fails
- **WHEN** a ProcessEngineer saves the record of `M3`
- **THEN** the transaction rolls back and the record does not change

### Requirement: Connection of an archived machine
`SetPlcConnection` and the transfer route of the connection SHALL refuse an archived machine. The page of an archived machine SHALL still show its record without the password (A14, A32, A47).

#### Scenario: Transfer read on an archived machine
- **GIVEN** the archived machine `M3` with a record and no live machine with the code `M3`
- **WHEN** the `ExportClient` account reads the connection of `M3` for the recipe `R-100` of `M3`
- **THEN** the system answers `409 E-PLT-010` without a field of the record

#### Scenario: Save on an archived machine
- **GIVEN** the archived machine `M3` with a record and no live machine with the code `M3`
- **WHEN** a ProcessEngineer sends a save of the connection of `M3`
- **THEN** the system answers `404 E-WEB-005`
- **AND** the record does not change

### Requirement: Transfer route of the connection
`GET /export/v1/machines/{code}/connection?recipe={ref}` SHALL give the record with the password as JSON to an `ExportClient` account with HTTP Basic. The recipe `{ref}` MUST belong to the live machine `{code}`. Each successful read SHALL commit one audit entry `PlcConnectionExported` without the password before the answer (A32, A45, A47).

#### Scenario: Read of a TwinCAT 3 record
- **GIVEN** the record of `M8` with the mode `Ads`, a login, and a stored password
- **AND** the recipe `R-300` of `M8`
- **WHEN** the `ExportClient` account `transfer01` sends the request for `M8` and `R-300` with HTTP Basic
- **THEN** the system answers `200 application/json` with the fields of `TwinCat3` and the password
- **AND** the answer sets `Cache-Control: no-store`
- **AND** the audit log holds one entry `PlcConnectionExported` with the actor `transfer01`

#### Scenario: Machine without a record
- **GIVEN** the live machine `M12` without a record and with the recipe `R-400`
- **WHEN** the `ExportClient` account sends the request for `M12` and `R-400`
- **THEN** the system answers `404 E-PLT-027`

#### Scenario: Missing recipe value
- **WHEN** the `ExportClient` account sends the request for `M8` without the query value `recipe`
- **THEN** the system answers `404 E-WEB-005` and gives no field of the record

#### Scenario: Recipe of a reused machine code
- **GIVEN** the recipe `R-100` of the archived machine `M8`
- **AND** a new live machine `M8` with a record
- **WHEN** the `ExportClient` account sends the request for `M8` and `R-100`
- **THEN** the system answers `404 E-PLT-029` and gives no field of the record

#### Scenario: Failed audit insert of a read
- **GIVEN** the audit insert of the read fails
- **WHEN** the `ExportClient` account reads the record of `M8` for `R-300`
- **THEN** the system answers `500` without the password
- **AND** the audit log holds no `PlcConnectionExported` entry

#### Scenario: Timeout of a read
- **GIVEN** the read of the record of `M8` runs longer than `timeout.export`
- **WHEN** the `ExportClient` account reads the record of `M8` for `R-300`
- **THEN** the system answers `503 E-WEB-009` without the password
- **AND** the audit log holds no `PlcConnectionExported` entry

#### Scenario: Person account
- **GIVEN** the ProcessEngineer account `anna`
- **WHEN** a request for `M8` carries the Basic credentials of `anna`
- **THEN** the system answers `401 E-IDN-003` and gives no field of the record

#### Scenario: HTMX request
- **WHEN** a request for `M8` carries the header `HX-Request: true`
- **THEN** the navigation guard answers `403 E-WEB-007` before the Basic check

#### Scenario: Read in read-only mode
- **GIVEN** the system is in read-only mode
- **WHEN** the `ExportClient` account reads the record of `M8` for `R-300`
- **THEN** the system answers `200` and writes the audit entry `PlcConnectionExported`

#### Scenario: Unreadable ciphertext
- **GIVEN** the key of `plc_credential_key` differs from the key of the stored ciphertext of `M8`
- **WHEN** the `ExportClient` account reads the record of `M8` for `R-300`
- **THEN** the system answers `500 E-WEB-014` without a password
- **AND** the system writes no audit entry

### Requirement: Connection outside the export documents
The export documents v1 and v2 SHALL NOT hold a field of the connection record. A change of the record SHALL NOT change `content_sha256` (R03, A47).

#### Scenario: Export after a connection change
- **GIVEN** the approved version 3 of the recipe `R-100` of the machine `M3`
- **WHEN** a ProcessEngineer changes the record of `M3` and the transfer client exports version 3
- **THEN** the export succeeds with the stored `content_sha256`
- **AND** the document holds no field of the record
