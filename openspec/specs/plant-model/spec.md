# plant-model Specification

## Purpose
The plant model describes the machines of one production line, their unit trees, their PLC platforms, and the PLC data types that recipe parameters use.

## Requirements

### Requirement: Machine code
The system SHALL identify a machine by a machine code from `M1` to `M9999999`. A machine code SHALL be unique among the live machines (R05, A32, I11).

#### Scenario: Code of a live machine
- **GIVEN** the live machine `M12`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M12`
- **THEN** the system rejects the command with `E-PLT-001`
- **AND** the system creates no machine and no unit

#### Scenario: Code of an archived machine
- **GIVEN** the archived machine `M12` and no live machine `M12`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M12`
- **THEN** the system creates a new live machine `M12`

### Requirement: Machine with one PLC platform and a first unit
The system SHALL create a machine with exactly one PLC platform and with the main unit `U1` in one transaction. The PLC platform SHALL be `CodesysIec`, `TwinCat3`, `S7_1200`, `S7_1500`, or `S7_300`. A machine on `S7_300`, `S7_1200`, or `S7_1500` SHALL get its data block number in the same transaction (R06, R15, A48).

#### Scenario: Create a machine
- **GIVEN** no live machine `M7`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M7`, the platform `S7_300`, the data block number `10`, and the main unit name `Infeed`
- **THEN** the system creates the machine `M7` with the platform `S7_300` and the data block number `10`
- **AND** the system creates the unit `U1` with the name `Infeed`
- **AND** the audit log holds the actions `MachineCreated` and `UnitAdded`

### Requirement: Frozen PLC platform
The system SHALL NOT change the PLC platform of a machine that has a recipe. A new controller SHALL be a new machine (R15, A32, I21).

#### Scenario: Platform change after the first recipe
- **GIVEN** the machine `M12` with the platform `S7_300` and one recipe
- **WHEN** an `UPDATE` statement as the table owner `recipe_migrator` changes the platform of `M12` to `S7_1500`
- **THEN** the database rejects the statement with `E-PLT-030`

### Requirement: Rename of a live machine
The system SHALL rename a machine only while the machine is live (A14).

#### Scenario: Rename an archived machine
- **GIVEN** the archived machine `M3`
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M3`
- **THEN** the system rejects the command with `E-PLT-010`
- **AND** the name of `M3` does not change

### Requirement: Archive of a machine
The system SHALL archive a machine only when no recipe of the machine has a live draft. The recipes of an archived machine SHALL stay readable and exportable (R05, A14).

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

### Requirement: Unit path
The system SHALL accept a unit path `U<s1>[.<s2>[.<s3>[.<s4>[.<s5>]]]]` with 1 to 5 segments. Each segment SHALL be a number from 1 to 64 (R06, R07, R38, A6, A30, I9).

#### Scenario: Sixth level
- **GIVEN** the machine `M12` with the live unit `U1.2.4.5.6`
- **WHEN** a ProcessEngineer runs `AddUnit` with the parent path `U1.2.4.5.6` and the segment `1`
- **THEN** the system rejects the command with `E-PLT-002`
- **AND** the system creates no unit

#### Scenario: Segment outside 1 to 64
- **GIVEN** the machine `M12` with the live unit `U1`
- **WHEN** a ProcessEngineer runs `AddUnit` with the parent path `U1` and the segment `65`
- **THEN** the system rejects the command with `E-PLT-017`

### Requirement: Live parent unit
The system SHALL add a unit only on a live machine. A unit below the main level SHALL have a live parent unit in the same machine (R07, A14, I10).

#### Scenario: Parent does not exist
- **GIVEN** the machine `M12` with the unit `U1` and no unit `U1.3`
- **WHEN** a ProcessEngineer runs `AddUnit` with the parent path `U1.3` and the segment `2`
- **THEN** the system rejects the command with `E-PLT-003`

#### Scenario: Archived parent
- **GIVEN** the machine `M12` with the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `AddUnit` with the parent path `U1.2` and the segment `1`
- **THEN** the system rejects the command with `E-PLT-009`

#### Scenario: Archived machine
- **GIVEN** the archived machine `M3`
- **WHEN** a ProcessEngineer runs `AddUnit` on `M3` with an empty parent path and the segment `2`
- **THEN** the system rejects the command with `E-PLT-010`

### Requirement: Unique unit path in a machine
The system SHALL keep each unit path unique inside one machine (R06).

#### Scenario: Duplicate unit path
- **GIVEN** the machine `M12` with the units `U1` and `U1.2`
- **WHEN** a ProcessEngineer runs `AddUnit` with the parent path `U1` and the segment `2`
- **THEN** the system rejects the command with `E-PLT-004`

### Requirement: Rename of a live unit
The system SHALL rename a unit only while the unit is live (R06, A14).

#### Scenario: Rename an archived unit
- **GIVEN** the machine `M12` with the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `RenameUnit` on `U1.2`
- **THEN** the system rejects the command with `E-PLT-009`

### Requirement: Archive of a unit and its descendants
The system SHALL archive a unit and all its descendants in one transaction. The system SHALL NOT delete a unit (R06, R29, A14).

#### Scenario: Archive a unit with a child
- **GIVEN** the machine `M12` with the units `U1`, `U1.2`, and `U1.2.1`
- **AND** no parameter on `U1.2` or `U1.2.1`
- **WHEN** a ProcessEngineer runs `ArchiveUnit` on `U1.2`
- **THEN** the system sets the archive time of `U1.2` and `U1.2.1`
- **AND** the audit log holds one action `UnitArchived` for each of the two units

### Requirement: Archive of a unit in use
The system SHALL NOT archive a unit while a live draft or an approved version has a parameter on the unit or on a descendant. The command SHALL then archive no unit (R06, R29, I19).

#### Scenario: Unit in a live draft
- **GIVEN** the units `U1.2` and `U1.2.1` of the machine `M12`
- **AND** a live draft of the recipe `R-100` with a parameter on `U1.2.1`
- **WHEN** a ProcessEngineer runs `ArchiveUnit` on `U1.2`
- **THEN** the system rejects the command with `E-PLT-012`
- **AND** `U1.2` and `U1.2.1` stay live

#### Scenario: Unit in an approved version
- **GIVEN** the approved version of the recipe `R-100` with a parameter on the unit `U1.2` of `M12`
- **WHEN** a ProcessEngineer runs `ArchiveUnit` on `U1.2`
- **THEN** the system rejects the command with `E-PLT-014`

### Requirement: Last main unit
The system SHALL keep at least one live main unit in every live machine (R06).

#### Scenario: Archive the last main unit
- **GIVEN** the machine `M12` with `U1` as its only live main unit
- **WHEN** a ProcessEngineer runs `ArchiveUnit` on `U1`
- **THEN** the system rejects the command with `E-PLT-008`

### Requirement: Primitive type catalog
The system SHALL provide the primitive types of the matrix in `data-model.md` section 7 for each PLC platform. The system SHALL NOT support `ENUM`, subrange, or alias types (R11, R12, A3, A39).

#### Scenario: Primitive type from the catalog
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `INT`
- **THEN** the system uses the global primitive `INT` of the catalog
- **AND** the system creates no type

### Requirement: Type on the platform of the machine
Every type that a parameter or a struct member uses SHALL exist on the PLC platform of the machine. A type text that a command writes SHALL also follow the rule of ADR-0148. On `S7_1200` and `S7_1500`, the element of an array is not an array. A stored type SHALL keep the earlier rule set of ADR-0120, so a stored array of an array stays valid (R11, R12, I6, ADR-0148).

#### Scenario: Parameter type not on the platform
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `WSTRING(10)` on a draft of a recipe of `M3`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: Member type not on the platform
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Label` with a member of the type `WSTRING(10)`
- **THEN** the system rejects the command with `E-PLT-006`

#### Scenario: Database backstop
- **GIVEN** a draft of a recipe of the machine `M3` with the platform `S7_300`
- **WHEN** an `INSERT` statement adds a parameter with a type that does not exist on `S7_300`
- **THEN** the database rejects the statement with `E-PLT-031`

#### Scenario: Type of another machine
- **GIVEN** the machines `M3` and `M4` with the platform `S7_300`
- **AND** the type `STRING(20)` of `M4` and a draft of a recipe of `M3`
- **WHEN** an `INSERT` statement adds a parameter of the type `STRING(20)` of `M4` to the draft of `M3`
- **THEN** the database rejects the statement with `E-PLT-032`

### Requirement: Type expression
The system SHALL accept a type in a request only as a type expression of `domain-model.md` section 2.1. An array SHALL have 1 to 3 dimensions (R13, R38, A38).

#### Scenario: Array with four dimensions
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `ARRAY[1..2,1..2,1..2,1..2] OF INT`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: Unknown struct revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 to 3
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `ST_Zone@9`
- **THEN** the system rejects the command with `E-PLT-013`

### Requirement: Derived types on demand
The system SHALL create a `STRING(n)`, `WSTRING(n)`, or array type of a machine at its first use. After that, the system SHALL use the existing type. A new derived type SHALL exist only on the platform of its machine (R13, I6).

#### Scenario: First use of a sized string
- **GIVEN** the machine `M12` with the platform `TwinCat3` and no type `STRING(20)`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(20)`
- **THEN** the system creates the type `STRING(20)` for `M12` with the platform `TwinCat3` only
- **AND** the audit log holds the action `DerivedTypeCreated`

#### Scenario: Second use of a sized string
- **GIVEN** the machine `M12` with the type `STRING(20)`
- **WHEN** a ProcessEngineer runs `AddParameter` for a second parameter with the type `STRING(20)`
- **THEN** the system uses the existing type `STRING(20)`
- **AND** the system writes no action `DerivedTypeCreated`

### Requirement: Size limits of a type
The system SHALL reject a type with more than 16 384 leaves or more than 32 levels. The system SHALL reject a struct with more than 1024 members (R13, A26).

#### Scenario: Array above the size limit
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `ARRAY[1..20000] OF INT`
- **THEN** the system rejects the command with `E-PLT-018`
- **AND** the system creates no type

#### Scenario: Struct above the size limit
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Big` with 1025 members
- **THEN** the system rejects the command with `E-PLT-018`

### Requirement: Struct definition
The system SHALL create a struct only with at least one member. A new struct SHALL NOT use the name of an existing struct of the same machine. The compare of the two names SHALL ignore the case of the letters (R13, I22, ADR-0143).

#### Scenario: Struct without members
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Empty` with no member
- **THEN** the system rejects the command with `E-PLT-016`

#### Scenario: Name of an existing struct
- **GIVEN** the machine `M12` with the struct `ST_Zone`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new struct with the name `ST_Zone`
- **THEN** the system rejects the command with `E-PLT-015`

#### Scenario: Name of an existing struct in another case
- **GIVEN** the machine `M12` with the struct `ST_Zone`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new struct with the name `st_zone`
- **THEN** the system rejects the command with `E-PLT-015`
- **AND** the system creates no type

#### Scenario: Two names in two cases at one time
- **GIVEN** the machine `M12` with no struct `ST_Zone`
- **WHEN** two ProcessEngineers run `DefineUdt` for the new structs `ST_Zone` and `st_zone` at one time
- **THEN** one command creates its struct
- **AND** the other command fails with `E-PLT-015`

#### Scenario: Struct row in another case in the database
- **GIVEN** the stored struct `ST_Zone@1` of the machine `M12`
- **WHEN** an `INSERT` statement as `recipe_app` adds the struct row `st_zone` with the revision 1 on `M12`
- **THEN** the database rejects the statement on the unique index `plc_type_struct_name_ci_unique`

#### Scenario: One name on two machines
- **GIVEN** the machine `M12` with the struct `ST_Zone` and the machine `M7` with no struct
- **WHEN** a ProcessEngineer runs `DefineUdt` on `M7` for a new struct with the name `st_zone`
- **THEN** the system creates `st_zone@1` on `M7`

#### Scenario: Struct row without a member at commit
- **GIVEN** a transaction that inserts the struct row `ST_Empty` and no member row
- **WHEN** the transaction commits
- **THEN** the database rejects the commit with `E-PLT-036`

### Requirement: Struct revision
A change of a struct SHALL create a new revision with the next revision number. A parameter that uses the old revision SHALL keep it (R19, A29, I22).

#### Scenario: New revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revision 2
- **AND** a draft with a parameter of the type `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ST_Zone`
- **THEN** the system creates the revision 3 of `ST_Zone`, which supersedes the revision 2
- **AND** the parameter keeps the type `ST_Zone@2`
- **AND** the system marks the parameter as outdated
- **AND** the audit log holds the action `UdtDefined`

### Requirement: Struct without a cycle
A struct SHALL NOT contain itself, directly or through its members. Two revisions of one struct SHALL count as one type for this rule (R13, I8, ADR-0038).

#### Scenario: Circular member
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Loop` with a member whose type tree contains `ST_Loop`
- **THEN** the system rejects the command with `E-PLT-005`
- **AND** the system creates no type

#### Scenario: Earlier revision of the same struct
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ST_Zone` with a member of the type `ST_Zone@1`
- **THEN** the system rejects the command with `E-PLT-005` for `ST_Zone`
- **AND** the system creates no type

#### Scenario: One struct on two paths
- **GIVEN** the machine `M12` with the struct `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Pair` with the members `a` and `b`, both of the type `ST_Zone@2`
- **THEN** the system creates `ST_Pair@1`

### Requirement: Immutable type rows
The system SHALL NOT change a stored type or the members of a stored type (R13, I22).

#### Scenario: Change of a stored type
- **GIVEN** the stored struct `ST_Zone@2`
- **WHEN** an `UPDATE` statement as the table owner `recipe_migrator` changes the row of `ST_Zone@2`
- **THEN** the database rejects the statement with `E-PLT-033`

#### Scenario: New member of a stored type
- **GIVEN** the stored struct `ST_Zone@2`
- **WHEN** an `INSERT` statement adds a member to `ST_Zone@2`
- **THEN** the database rejects the statement with `E-PLT-034`

#### Scenario: Member of a primitive type
- **GIVEN** the primitive type `INT` of the catalog
- **WHEN** an `INSERT` statement adds a member to `INT`
- **THEN** the database rejects the statement with `E-PLT-035`

### Requirement: Frozen machine code and unit path
The system SHALL NOT change a machine code while a version that is not a draft uses it. The same rule SHALL apply to the path and the machine of a unit (R05, R07, I23).

#### Scenario: Machine code in an approved version
- **GIVEN** the machine `M12` and an approved version of a recipe of `M12`
- **WHEN** an `UPDATE` statement as the table owner `recipe_migrator` changes the code of `M12`
- **THEN** the database rejects the statement with `E-PLT-037`

#### Scenario: Unit path in an approved version
- **GIVEN** the unit `U1.2` of `M12` and an approved version with a parameter on `U1.2`
- **WHEN** an `UPDATE` statement as the table owner `recipe_migrator` changes the path of `U1.2`
- **THEN** the database rejects the statement with `E-PLT-038`

### Requirement: String length per platform
The length of a `STRING(n)` or a `WSTRING(n)` SHALL be inside the range of the `StringLength` table of `domain-model.md` section 2.1 for the platform of the machine. On `CodesysIec` and `TwinCat3`, the range SHALL stay 1 to 255, also when the controller accepts a longer string. A `STRING` or a `WSTRING` without a length SHALL mean the seed type of that platform (R11, R12, R13).
#### Scenario: STRING length above the S7 range
- **GIVEN** the machine `M12` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(255)`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** the system creates no type

#### Scenario: STRING length inside the TwinCat3 range
- **GIVEN** the machine `M7` with the platform `TwinCat3` and no type `STRING(255)`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(255)`
- **THEN** the system creates the type `STRING(255)` for `M7`

#### Scenario: WSTRING length above the TwinCat3 range
- **GIVEN** the machine `M7` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `WSTRING(256)`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: STRING without a length
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING`
- **THEN** the system uses the seed type `STRING(254)` of `S7_300`
- **AND** the system writes no action `DerivedTypeCreated`

#### Scenario: STRING length above the CodesysIec range
- **GIVEN** the machine `M12` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(256)`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** the system creates no type

#### Scenario: WSTRING length inside the CodesysIec range
- **GIVEN** the machine `M12` with the platform `CodesysIec` and no type `WSTRING(255)`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `WSTRING(255)`
- **THEN** the system creates the type `WSTRING(255)` for `M12`

### Requirement: Unique member names of a struct
The system SHALL reject a struct with two members of the same name. The compare of two member names SHALL ignore the case of the letters (R13, I22, ADR-0143).

#### Scenario: Two members with one name
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Pair` with the members `a` of type `INT` and `a` of type `REAL`
- **THEN** the system rejects the command with `E-PLT-021`
- **AND** the system creates no type

#### Scenario: Two member names in two cases
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Drive` with the members `speed` of type `INT` and `Speed` of type `REAL`
- **THEN** the system rejects the command with `E-PLT-021` for the member `Speed`
- **AND** the system creates no type

#### Scenario: Member rows in two cases in the database
- **GIVEN** a transaction that inserts the struct row `ST_Drive` on the machine `M12`
- **WHEN** an `INSERT` statement as `recipe_app` adds the members `speed` and `Speed` to `ST_Drive`
- **THEN** the database rejects the statement on the unique index `plc_type_member_name_ci_unique`

### Requirement: Order of the derived type locks
A command that can insert a type row SHALL take the advisory locks of derived types in the ascending order of their key values. It SHALL take each key value once. The command SHALL take every lock that its type expressions need before its first insert of a type row. `EnsureDerivedType` SHALL take no lock of its own (R29, R31, ADR-0144).

#### Scenario: Array of sized strings in a new parameter
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..4] OF STRING(16)`
- **THEN** the command takes the locks `STRING` and `ARRAY` of `M12` in the ascending order of their key values
- **AND** the command takes both locks before it inserts the type `STRING(16)`

#### Scenario: Struct with an array member
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Zone` with the member `temps` of type `ARRAY[1..2] OF REAL`
- **THEN** the command takes the locks `ST_Zone` and `ARRAY` of `M12` in the ascending order of their key values
- **AND** the command takes both locks before it inserts a type row

#### Scenario: Seed string without a length
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `STRING`
- **THEN** the command uses the seed row `STRING(80)`
- **AND** the command takes no advisory lock of derived types

#### Scenario: Stored struct ARRAY next to an array of sized strings
- **GIVEN** the machine `M12` with the stored struct `ARRAY@1` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ARRAY` with a member of type `STRING(5)`
- **AND** another ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF STRING(5)` at the same time
- **THEN** both commands take the locks of `M12` in one order
- **AND** no command fails with `40P01`
- **AND** `DefineUdt` fails with `E-PLT-013` and the reason "reserved name", because `ARRAY` is a reserved name

#### Scenario: Two key texts with one key value
- **GIVEN** two key texts of the machine `M12` with one key value
- **WHEN** a command needs the keys of both texts
- **THEN** the command takes one lock for that key value

### Requirement: Archive of an archived unit
The system SHALL reject `ArchiveUnit` with `E-PLT-009` when the unit is already archived. The command SHALL then change no unit and write no audit entry (R06, R29).

#### Scenario: Unit archived before
- **GIVEN** the machine `M12` with the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `ArchiveUnit` on `U1.2`
- **THEN** the system rejects the command with `E-PLT-009`
- **AND** the archive time of `U1.2` does not change

### Requirement: Stable identifiers of the seed types
Each primitive row of the matrix in `data-model.md` section 7 SHALL have one identifier that never changes. A second run of the migrator SHALL NOT add, change, or delete a seed row (R11, R12, A3).

#### Scenario: Seed rows after the first migration
- **GIVEN** the database `recipe` after the first run of `recipe-server migrate`
- **WHEN** a user reads the global primitive rows
- **THEN** the system has 31 global primitive rows: one row for each other type of the matrix, and two rows each for `STRING` and `WSTRING`
- **AND** each row has the identifier of the seed file

#### Scenario: Second run of the migrator
- **GIVEN** the database `recipe` with the seed rows
- **WHEN** the operator runs `recipe-server migrate` again
- **THEN** the seed rows keep their identifiers
- **AND** the number of global primitive rows stays the same

### Requirement: Machine list
The query `ListMachines` SHALL return the live machines in the order of the code number, `page.machines` rows per page. A search text SHALL match a part of the code or of the name, with no difference between upper and lower case. With `show_archived`, the list SHALL also hold the archived machines, each with its identifier (R05, R24, A32).

#### Scenario: Order of the code number
- **GIVEN** the live machines `M10`, `M2`, and `M1`
- **WHEN** the Operator `mhoffman` runs `ListMachines` with no search text
- **THEN** the list holds `M1`, `M2`, and `M10` in this order

#### Scenario: Search text with another case
- **GIVEN** the live machine `M4` with the name `Piec hartowniczy` and the live machine `M5` with the name `Prasa`
- **WHEN** a ProcessEngineer runs `ListMachines` with the search text `PIEC`
- **THEN** the list holds `M4` and not `M5`

#### Scenario: Archived machine
- **GIVEN** the archived machine `M12` and the live machine `M12` that replaces it
- **WHEN** a ProcessEngineer runs `ListMachines` with `show_archived`
- **THEN** the list holds both machines `M12`
- **AND** the row of the archived machine holds its identifier and its archive time

#### Scenario: Second page
- **GIVEN** 101 live machines and `page.machines` set to 100
- **WHEN** a ProcessEngineer runs `ListMachines` for the second page
- **THEN** the list holds one machine and no next page

#### Scenario: Page after the end
- **GIVEN** 3 live machines and `page.machines` set to 100
- **WHEN** a ProcessEngineer runs `ListMachines` for the third page
- **THEN** the list holds no machine and no next page

### Requirement: Archive message
The query `ListMachines` SHALL carry a machine code as an archive message only when an archived machine has that code and no live machine has it. The query `GetMachine` SHALL carry a unit path as an archive message only when the machine holds that unit archived (R05, R06, R39).

#### Scenario: Message of an archived machine
- **GIVEN** the archived machine `M7` and no live machine `M7`
- **WHEN** a ProcessEngineer runs `ListMachines` with the archive message `M7`
- **THEN** the result carries the archive message `M7`

#### Scenario: Code of a live machine
- **GIVEN** the live machine `M7`
- **WHEN** a ProcessEngineer runs `ListMachines` with the archive message `M7`
- **THEN** the result carries no archive message

#### Scenario: Path of a live unit
- **GIVEN** the live machine `M7` with the live unit `U1.2`
- **WHEN** a ProcessEngineer runs `GetMachine` for `M7` with the archive message `U1.2`
- **THEN** the result carries no archive message

### Requirement: Unit tree of a machine
The queries `GetMachine` and `GetUnitTree` SHALL return the machine with the units of its tree. For a live machine, the tree SHALL hold the live units. For an archived machine that `GetMachine` finds by its identifier, the tree SHALL hold every unit of the machine (R06, R07, A30, A32).

#### Scenario: Tree of five levels
- **GIVEN** the live machine `M7` with the live units `U1`, `U1.2`, `U1.2.4`, `U1.2.4.5`, and `U1.2.4.5.6`
- **WHEN** the Operator `mhoffman` runs `GetUnitTree` for `M7`
- **THEN** the tree holds the five units, each under its parent

#### Scenario: Archived unit of a live machine
- **GIVEN** the live machine `M7` with the live unit `U1` and the archived unit `U2`
- **WHEN** a ProcessEngineer runs `GetMachine` for the code `M7`
- **THEN** the tree holds `U1` and not `U2`

#### Scenario: Archived machine by identifier
- **GIVEN** the archived machine `M12` with the unit `U1` and the archived unit `U2`
- **WHEN** a ProcessEngineer runs `GetMachine` with the identifier of the archived `M12`
- **THEN** the tree holds `U1` and `U2`
- **AND** `U2` carries its archive time

#### Scenario: Unknown machine code
- **GIVEN** no live machine `M99`
- **WHEN** a ProcessEngineer runs `GetUnitTree` for `M99`
- **THEN** the system rejects the query with `E-WEB-005`

### Requirement: Proposal of a new unit
The query `GetUnitForm` SHALL offer as a parent each live unit of depth 1 to 4. It SHALL propose the lowest segment from 1 to 64 that no unit under the parent uses, live or archived. A parent that is not a live unit SHALL give `E-WEB-005`. A live parent of depth 5 SHALL give `E-PLT-002` (R06, R07, A30).

#### Scenario: Segment after an archived unit
- **GIVEN** the live machine `M7` with the live unit `U1.1` and the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7` with the parent `U1`
- **THEN** the form proposes the segment `3`

#### Scenario: All segments in use
- **GIVEN** the live machine `M7` with 64 units under `U1`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7` with the parent `U1`
- **THEN** the form proposes no segment

#### Scenario: Parent of depth 5
- **GIVEN** the live machine `M7` with the live unit `U1.2.4.5.6`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7` with the parent `U1.2.4.5.6`
- **THEN** the system rejects the query with `E-PLT-002`

#### Scenario: List of parents
- **GIVEN** the live machine `M7` with the live unit `U1.2.4.5.6`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7`
- **THEN** the list of parents does not hold `U1.2.4.5.6`

#### Scenario: Form without the permission
- **GIVEN** the live machine `M7`
- **WHEN** the Operator `mhoffman` runs `GetUnitForm` for `M7`
- **THEN** the system rejects the query with `E-WEB-002`

### Requirement: Type list of a machine
The query `ListTypes` SHALL return one row per struct revision of the machine. The rows SHALL come in the byte order of the struct name. Inside one name, the latest revision SHALL come first and SHALL carry a mark (R13, R19).

#### Scenario: Two revisions of one struct
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2 and the struct `ST_Axis` in the revision 1
- **WHEN** the Operator `mhoffman` runs `ListTypes` for `M12`
- **THEN** the rows are `ST_Axis@1`, `ST_Zone@2`, and `ST_Zone@1` in this order
- **AND** the rows `ST_Axis@1` and `ST_Zone@2` carry the mark of the latest revision

### Requirement: Type tree of a revision
The query `GetTypeTree` SHALL return the resolved type tree of the latest revision of a struct, or of the revision that the request names. A struct or a revision that the machine does not hold SHALL give `E-WEB-005` (R13, R19).

#### Scenario: Earlier revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2
- **WHEN** the Operator `mhoffman` runs `GetTypeTree` for `ST_Zone` with the revision 1
- **THEN** the system returns the type tree of `ST_Zone@1`
- **AND** the result lists the revisions 2 and 1

#### Scenario: Unknown revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revision 2
- **WHEN** the Operator `mhoffman` runs `GetTypeTree` for `ST_Zone` with the revision 9
- **THEN** the system rejects the query with `E-WEB-005`

### Requirement: Form of a new revision
The query `GetUdtForm` with the name of a struct SHALL return the members of the latest revision of that struct. Each member SHALL carry its name and its type as a type expression that parses to the same type. The type proposals of that form SHALL hold no revision of that struct (R13, R19, ADR-0038).

#### Scenario: Members of the latest revision
- **GIVEN** the machine `M12` with the struct `ST_Zone@2` and its members `temp_max` of the type `REAL` and `names` of the type `ARRAY[1..4] OF STRING(16)`
- **WHEN** a ProcessEngineer runs `GetUdtForm` for `M12` with the name `ST_Zone`
- **THEN** the form holds `temp_max` with `REAL` and `names` with `ARRAY[1..4] OF STRING(16)` in this order

#### Scenario: Proposals without the revised struct
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2
- **WHEN** a ProcessEngineer runs `GetUdtForm` for `M12` with the name `ST_Zone`
- **THEN** the type proposals hold no revision of `ST_Zone`

#### Scenario: Unknown struct
- **GIVEN** the machine `M12` without the struct `ST_Axis`
- **WHEN** a ProcessEngineer runs `GetUdtForm` for `M12` with the name `ST_Axis`
- **THEN** the system rejects the query with `E-WEB-005`

### Requirement: Type usage
The query `GetTypeUsage` SHALL return each parameter whose type uses the struct revision, directly or through another type, once. It SHALL list the versions `Draft`, `Approved`, and `Retired` of the recipes that are not deleted, `page.type_usage` rows per page. A row SHALL name the recipe, the version, its state, the tag, the key of the parameter, and the type of the parameter (R19, A29).

#### Scenario: Use through another struct
- **GIVEN** the machine `M12` with the struct `ST_Line@1` that has a member of the type `ARRAY[1..4] OF ST_Zone@2`
- **AND** the draft version 3 of the recipe `R-100` with the parameter `LINE` of the type `ST_Line@1`
- **WHEN** the Operator `mhoffman` runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds the row `R-100`, version 3, `Draft`, `LINE`, `ST_Line@1`
- **AND** the row holds the key of the parameter `LINE`

#### Scenario: Two members of one struct
- **GIVEN** the struct `ST_Pair@1` with the members `a` and `b`, both of the type `ST_Zone@2`
- **AND** the draft version 1 of the recipe `R-400` with the parameter `PAIR` of the type `ST_Pair@1`
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds one row for the parameter `PAIR`

#### Scenario: Deleted recipe and discarded draft
- **GIVEN** the deleted recipe `R-200` with a parameter of the type `ST_Zone@2`
- **AND** a discarded draft of the recipe `R-300` with a parameter of the type `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds no row of `R-200` and no row of the discarded draft

#### Scenario: Revision without use
- **GIVEN** the struct `ST_Zone@1` that no parameter uses
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 1
- **THEN** the list is empty

### Requirement: Reserved struct names
The system SHALL reject a new struct name that equals a reserved word of ADR-0143 in any case. The reserved words SHALL be the 182 keywords of ADR-0143 (d) and the 29 primitive names (R11, R12, R13, R38).

#### Scenario: Primitive name in another case
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer sends the form of a new struct with the name `Int` or `real`
- **THEN** the system rejects the form with `E-WEB-008` for the field `type_name`
- **AND** the system creates no type

#### Scenario: IEC keyword as a struct name
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer sends the form of a new struct with the name `ARRAY`, `OF`, `STRUCT`, or `end_struct`
- **THEN** the system rejects the form with `E-WEB-008` for the field `type_name`

#### Scenario: New revision of a stored struct with a reserved name
- **GIVEN** the machine `M12` with the struct `Int@1`, stored before ADR-0143
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `Int`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the system creates no type

#### Scenario: Keyword as a member name
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Text` with the member `REAL` of type `INT`
- **THEN** the system creates `ST_Text@1`

### Requirement: Stored struct from before the identifier rules
The system SHALL read a stored struct whose name is a reserved word of ADR-0143. A stored parameter type SHALL keep such a struct. A type text that a command writes SHALL NOT name such a struct. The lookup of a struct SHALL use the exact stored spelling (R19, ADR-0120, ADR-0143).

#### Scenario: Stored struct with a primitive name
- **GIVEN** the machine `M12` with the struct `Int@1`, stored before ADR-0143
- **WHEN** a ProcessEngineer opens the page of the type `Int`
- **THEN** the system shows `Int@1` and its members

#### Scenario: Parameter of a stored struct with a primitive name
- **GIVEN** the draft version 3 of `R-100` on `M12` with the parameter `ZONE` of type `Int@1`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `ZONE` with the type `Int@1` and a new name
- **THEN** the system stores the new name
- **AND** `ZONE` keeps the type `Int@1`

#### Scenario: New parameter of a stored struct with a primitive name
- **GIVEN** the machine `M12` with the struct `Int@1`, stored before ADR-0143, and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `Int@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the system adds no parameter

#### Scenario: Changed type to a stored struct with a primitive name
- **GIVEN** the draft version 3 of `R-100` on `M12` with the parameter `SPEED` of type `INT` and the stored struct `Int@1`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `Int@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** `SPEED` keeps the type `INT`

#### Scenario: Struct reference in another case
- **GIVEN** the machine `M12` with the struct `ST_Zone@1` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `st_zone@1`
- **THEN** the system rejects the command with `E-PLT-013`

### Requirement: Lower subscript of stored array bounds
The database SHALL accept an array type row only when its two bound arrays start at the subscript 1 (R13, A38, ADR-0145).

#### Scenario: Lower bounds at the subscript 0
- **GIVEN** the function `dims_valid`
- **WHEN** a statement calls `dims_valid('[0:0]={5}', '{0}')` and `dims_valid('[0:0]={0}', '{65536}')`
- **THEN** both calls give `false`

#### Scenario: Upper bounds at the subscript 0
- **GIVEN** the function `dims_valid`
- **WHEN** a statement calls `dims_valid('{1}', '[0:0]={8}')`
- **THEN** the call gives `false`

#### Scenario: Insert of an array row at the subscript 0
- **GIVEN** the machine `M12` and the primitive type `INT`
- **WHEN** an `INSERT` statement adds an array row of `INT` on `M12` with the bounds `'[0:0]={5}'` and `'{0}'`
- **THEN** the database rejects the statement on the constraint `plc_type_array_shape`

#### Scenario: Bounds at the subscript 1
- **GIVEN** the function `dims_valid`
- **WHEN** a statement calls `dims_valid('{1}', '{8}')`
- **THEN** the call gives `true`

### Requirement: Check of the stored type rows before the new rules
The migration that adds the rules of ADR-0143 and ADR-0145 SHALL stop and change nothing when the stored rows break the new rules. An array row with another start subscript breaks the new rules. Two struct names or two member names that differ only in case also break them (R31, ADR-0143, ADR-0145).

#### Scenario: Stored struct names in two cases
- **GIVEN** the structs `st_zone@1` and `ST_Zone@1` of the machine `M12`, stored before ADR-0143
- **WHEN** the migrator applies the migration `0011`
- **THEN** the migration `0011` stops with a message that names the number of each kind of row
- **AND** the function `dims_valid` does not change
- **AND** the database holds neither `plc_type_struct_name_ci_unique` nor `plc_type_member_name_ci_unique`

#### Scenario: Stored rows that follow the new rules
- **GIVEN** stored type rows that follow the rules of ADR-0143 and ADR-0145
- **WHEN** the migrator applies the migration `0011`
- **THEN** the database holds the new `dims_valid` and the indexes `plc_type_struct_name_ci_unique` and `plc_type_member_name_ci_unique`

### Requirement: No array of an array on S7-1200 and S7-1500
On a machine with `S7_1200` or `S7_1500`, the system SHALL reject a written type text that holds an array as the element of an array. The type of `AddParameter`, a changed type of `UpdateParameterDefinition`, and a member type of `DefineUdt` are such texts. An array with two or three dimensions in one prefix SHALL stay valid (R12, R13, R38, A38, ADR-0148).

#### Scenario: Parameter of an array of an array
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "the type does not exist on the platform of the machine"
- **AND** the system creates no type

#### Scenario: Stored array type in a new parameter
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **AND** the array type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` of `M5`, stored before ADR-0148
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** version 3 holds no new parameter

#### Scenario: Member of an array of an array
- **GIVEN** the machine `M6` with the platform `S7_1200`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Grid` with the member `cells` of type `ARRAY[1..3] OF ARRAY[1..3] OF BOOL`
- **THEN** the system rejects the command with `E-PLT-006` for the member `cells`
- **AND** the system creates no type

#### Scenario: New revision of a stored struct with an array of an array
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Grid@1` with the member `cells` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`, stored before ADR-0148
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Grid` with the members of `ST_Grid@1` and the new member `mode` of type `BYTE`
- **THEN** the system rejects the command with `E-PLT-006` for the member `cells`
- **AND** the system creates no revision `ST_Grid@2`

#### Scenario: Changed type to an array of an array
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `SPEED` of type `INT`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `ARRAY[1..2] OF ARRAY[1..2] OF INT`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** `SPEED` keeps the type `INT`

#### Scenario: Array with two dimensions
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2,1..3] OF BYTE`
- **THEN** the system adds the parameter

#### Scenario: Array of a struct with an array member
- **GIVEN** the machine `M5` with the struct `ST_Zone@1` and its member `ramps` of type `ARRAY[1..3] OF INT`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M5` with the type `ARRAY[1..2] OF ST_Zone@1`
- **THEN** the system adds the parameter

#### Scenario: Array of an array on TwinCAT 3
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M12` with the type `ARRAY[1..2] OF ARRAY[1..3] OF INT`
- **THEN** the system adds the parameter

### Requirement: Stored array of an array on S7-1200 and S7-1500
The system SHALL read a stored array type on `S7_1200` or `S7_1500` whose element is an array. A stored parameter type SHALL keep such a type while a command does not change the type. `RebindTypeRevision` SHALL keep the array levels of such a parameter type (R19, R31, ADR-0120, ADR-0148).

#### Scenario: Type page of a stored array of an array
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Grid@1` with the member `cells` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`, stored before ADR-0148
- **WHEN** a ProcessEngineer opens the page of the type `ST_Grid`
- **THEN** the system shows `ST_Grid@1` and its member `cells`

#### Scenario: Parameter keeps a stored array of an array
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `CELLS` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`, stored before ADR-0148
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `CELLS` with the same type and a new name
- **THEN** the system stores the new name
- **AND** `CELLS` keeps its type

#### Scenario: Rebind of a stored array of an array
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `ZONES` of type `ARRAY[1..2] OF ARRAY[1..2] OF ST_Zone@1`, stored before ADR-0148
- **AND** the struct `ST_Zone@2` with the members of `ST_Zone@1`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3 from `ST_Zone@1` to `ST_Zone@2`
- **THEN** `ZONES` gets the type `ARRAY[1..2] OF ARRAY[1..2] OF ST_Zone@2`

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
Until a later change deletes the column `plc_connection.db_number`, each command SHALL keep that column equal to the data block number of the machine. The previous release reads that column after a rollback. A number that the previous release saves in that column SHALL also become the data block number of the machine (R31, A48).

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
