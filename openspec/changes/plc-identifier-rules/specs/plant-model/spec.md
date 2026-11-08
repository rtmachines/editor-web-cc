# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
