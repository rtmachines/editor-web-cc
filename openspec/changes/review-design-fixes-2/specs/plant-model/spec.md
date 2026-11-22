## MODIFIED Requirements

### Requirement: Size limits of a type
The system SHALL reject a type with more than 16 384 leaves or more than 32 levels. The system SHALL reject a struct with more than 1024 members. `DefineUdt` and `EnsureDerivedType` SHALL reject a type with a leaf path longer than `budget.leaf_path_bytes` (128 bytes) with `E-PLT-018`. A stored type with a longer leaf path SHALL stay readable (R13, A26, ADR-0211).

#### Scenario: Array above the size limit
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `ARRAY[1..20000] OF INT`
- **THEN** the system rejects the command with `E-PLT-018`
- **AND** the system creates no type

#### Scenario: Struct above the size limit
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Big` with 1025 members
- **THEN** the system rejects the command with `E-PLT-018`

#### Scenario: Leaf path above the length limit
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Deep` with a member path whose longest leaf path has 129 bytes
- **THEN** the system rejects the command with `E-PLT-018`
- **AND** the system creates no type

#### Scenario: Leaf path at the length limit
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Deep` with a member path whose longest leaf path has 128 bytes
- **THEN** the system creates `ST_Deep@1`

#### Scenario: Stored type with a longer leaf path
- **GIVEN** the stored struct `ST_Old@1` with a leaf path of 200 bytes from before ADR-0211, used by the parameter `OLD` of the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer opens the parameter `OLD`
- **THEN** the system shows the value of `OLD`

### Requirement: Immutable type rows
The system SHALL NOT change a stored type or the members of a stored type. A member insert SHALL pass only in the transaction that inserted its struct row. The trigger SHALL compare the transaction of the struct row with the current transaction, and its `created_at` with the start time of that transaction. A member insert without a visible struct row SHALL fail (R13, I22, ADR-0212).

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

#### Scenario: Member from a second transaction with the same start time
- **GIVEN** the committed struct `ST_Zone@2` and an open transaction B
- **AND** a superuser sets `created_at` of `ST_Zone@2` to the `now()` of B and commits
- **WHEN** B inserts a member of `ST_Zone@2`
- **THEN** the database rejects the statement with `E-PLT-034`

#### Scenario: Member without a visible struct row
- **GIVEN** a struct identifier that no row of `plc_type` visible to the transaction holds
- **WHEN** an `INSERT` statement adds a member with that struct identifier
- **THEN** the database rejects the statement with `E-PLT-035`

#### Scenario: Member in the transaction of the struct
- **GIVEN** a transaction that inserts the struct `ST_New@1`
- **WHEN** the same transaction inserts the members of `ST_New@1`
- **THEN** the database accepts the members
