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
The system SHALL create a machine with exactly one PLC platform and with the main unit `U1` in one transaction. The PLC platform SHALL be `CodesysIec`, `TwinCat3`, `S7_1200`, `S7_1500`, or `S7_300` (R06, R15).

#### Scenario: Create a machine
- **GIVEN** no live machine `M7`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M7`, the platform `S7_300`, and the main unit name `Infeed`
- **THEN** the system creates the machine `M7` with the platform `S7_300`
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
Every type that a parameter or a struct member uses SHALL exist on the PLC platform of the machine (R11, R12, I6).

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
The system SHALL create a struct only with at least one member. A new struct SHALL NOT use the name of an existing struct of the same machine (R13, I22).

#### Scenario: Struct without members
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Empty` with no member
- **THEN** the system rejects the command with `E-PLT-016`

#### Scenario: Name of an existing struct
- **GIVEN** the machine `M12` with the struct `ST_Zone`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new struct with the name `ST_Zone`
- **THEN** the system rejects the command with `E-PLT-015`

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
A struct SHALL NOT contain itself, directly or through its members (R13, I8).

#### Scenario: Circular member
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Loop` with a member whose type tree contains `ST_Loop`
- **THEN** the system rejects the command with `E-PLT-005`
- **AND** the system creates no type

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
The length of a `STRING(n)` or a `WSTRING(n)` SHALL be inside the range of the `StringLength` table of `domain-model.md` section 2.1 for the platform of the machine. A `STRING` or a `WSTRING` without a length SHALL mean the seed type of that platform (R11, R12, R13).

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

### Requirement: Unique member names of a struct
The system SHALL reject a struct with two members of the same name (R13, I22).

#### Scenario: Two members with one name
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Pair` with the members `a` of type `INT` and `a` of type `REAL`
- **THEN** the system rejects the command with `E-PLT-021`
- **AND** the system creates no type

### Requirement: Order of the derived type locks
A command that can insert a type row SHALL take the advisory locks of derived types in a fixed order. The order is the struct name, `STRING`, `WSTRING`, and `ARRAY`. The command SHALL take every lock that its type expressions need before its first insert of a type row. `EnsureDerivedType` SHALL take no lock of its own (R29, R31).

#### Scenario: Array of sized strings in a new parameter
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..4] OF STRING(16)`
- **THEN** the command takes the lock `STRING` of `M12` and then the lock `ARRAY` of `M12`
- **AND** the command takes both locks before it inserts the type `STRING(16)`

#### Scenario: Struct with an array member
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Zone` with the member `temps` of type `ARRAY[1..2] OF REAL`
- **THEN** the command takes the lock `ST_Zone` of `M12` and then the lock `ARRAY` of `M12`
- **AND** the command takes both locks before it inserts a type row

#### Scenario: Seed string without a length
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `STRING`
- **THEN** the command uses the seed row `STRING(80)`
- **AND** the command takes no advisory lock of derived types

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
