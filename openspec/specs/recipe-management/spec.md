# recipe-management Specification

## Purpose
The recipe management capability describes the recipes of one machine, their versions, the editable draft, and the parameters with their process limits. It also describes approval, retirement, and the rebinding of struct revisions.

## Requirements

### Requirement: Recipe belongs to one machine
The system SHALL bind each recipe to exactly one machine. A recipe SHALL keep its machine for its whole life (R02, A10, A25).

#### Scenario: Create a recipe on a live machine
- **GIVEN** the live machine `M12` with the unit `U1`
- **WHEN** a ProcessEngineer runs `CreateRecipe` with the machine code `M12` and the reference number `R-100`
- **THEN** the system creates the recipe `R-100` on `M12`
- **AND** the system creates version 1 as an empty draft
- **AND** the audit log holds the actions `RecipeCreated` and `VersionCreated`

### Requirement: Recipe needs a live machine
The system SHALL create a recipe only on a live machine. The system SHALL reject an unknown machine code with `E-PLT-019` and an archived machine with `E-PLT-010` (R02, A14).

#### Scenario: Machine code not found
- **GIVEN** no machine with the code `M12`
- **WHEN** a ProcessEngineer runs `CreateRecipe` with the machine code `M12`
- **THEN** the system rejects the command with `E-PLT-019`
- **AND** the system creates no recipe

#### Scenario: Archived machine
- **GIVEN** the archived machine `M3`
- **WHEN** a ProcessEngineer runs `CreateRecipe` with the machine code `M3`
- **THEN** the system rejects the command with `E-PLT-010`
- **AND** the system creates no recipe

### Requirement: Reference number
The system SHALL parse a reference number that matches `[A-Z0-9][A-Z0-9_-]{0,49}` after it folds the text to upper case. The system SHALL keep each reference number unique and SHALL NOT reuse a reference number (R16, R38, A27, I11).

#### Scenario: Duplicate reference number in lower case
- **GIVEN** the recipe `R-100`
- **WHEN** a ProcessEngineer runs `CreateRecipe` with the reference number `r-100`
- **THEN** the system rejects the command with `E-RCP-001`
- **AND** the system creates no recipe

### Requirement: Reference number frozen while a version is not a draft
The database SHALL reject a change of the reference number while a version of the recipe that is not a draft exists (R16, A27, I23).

#### Scenario: Change of the reference number of an approved recipe
- **GIVEN** the recipe `R-100` with an approved version
- **WHEN** an `UPDATE` statement as the table owner `recipe_migrator` changes the reference number of `R-100` to `R-101`
- **THEN** the database rejects the statement with `E-RCP-034`

### Requirement: Commands on a deleted recipe
Every recipe management command except `CreateRecipe` and `RestoreRecipe` SHALL reject a deleted recipe with `E-RCP-012` (A14, I19).

#### Scenario: Rename a deleted recipe
- **GIVEN** the deleted recipe `R-100`
- **WHEN** a ProcessEngineer runs `RenameRecipe` on `R-100`
- **THEN** the system rejects the command with `E-RCP-012`
- **AND** the name of `R-100` does not change

#### Scenario: Add a parameter to a deleted recipe
- **GIVEN** the deleted recipe `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on `R-100`
- **THEN** the system rejects the command with `E-RCP-012`

### Requirement: Soft delete of a recipe
The system SHALL soft delete a recipe and discard its live draft in the same transaction. The system SHALL reject `DeleteRecipe` for a recipe with an approved version. A ProcessEngineer SHALL NOT delete a recipe with a retired version (R23, A13, A14, I14).

#### Scenario: Delete a recipe with a live draft
- **GIVEN** the recipe `R-100` with the draft version 3 and no approved version
- **WHEN** a ProcessEngineer runs `DeleteRecipe` on `R-100`
- **THEN** the system sets the deletion time of `R-100` and of draft version 3
- **AND** the audit log holds the actions `DraftDiscarded` and `RecipeDeleted`

#### Scenario: Delete a recipe with an approved version
- **GIVEN** the recipe `R-100` with the approved version 2
- **WHEN** a ProcessEngineer runs `DeleteRecipe` on `R-100`
- **THEN** the system rejects the command with `E-RCP-010`
- **AND** `R-100` stays live

#### Scenario: ProcessEngineer deletes a recipe with a retired version
- **GIVEN** the recipe `R-100` with the retired version 1 and no approved version
- **WHEN** a ProcessEngineer runs `DeleteRecipe` on `R-100`
- **THEN** the system rejects the command with `E-RCP-026`

#### Scenario: Administrator deletes a recipe with a retired version
- **GIVEN** the recipe `R-100` with the retired version 1 and no approved version
- **WHEN** an Administrator runs `DeleteRecipe` on `R-100`
- **THEN** the system sets the deletion time of `R-100`

### Requirement: Restore of a deleted recipe
The system SHALL restore a recipe only when the recipe has a deletion time and its machine is live (A14).

#### Scenario: Restore a live recipe
- **GIVEN** the live recipe `R-100`
- **WHEN** an Administrator runs `RestoreRecipe` on `R-100`
- **THEN** the system rejects the command with `E-RCP-025`

#### Scenario: Restore on an archived machine
- **GIVEN** the deleted recipe `R-100` on the archived machine `M3`
- **WHEN** an Administrator runs `RestoreRecipe` on `R-100`
- **THEN** the system rejects the command with `E-PLT-010`
- **AND** `R-100` stays deleted

### Requirement: Version status and number
The system SHALL give each version one status of `Draft`, `Approved`, or `Retired`. The system SHALL number the versions of a recipe from 1 and SHALL NOT reuse a number (R17).

#### Scenario: Number after a discarded draft
- **GIVEN** the recipe `R-100` with the versions 1 to 3, where a ProcessEngineer discarded draft version 3
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100`
- **THEN** the system creates version 4

### Requirement: Version copy keeps parameter keys
The system SHALL create a new draft as a copy of its base version and SHALL keep the parameter key of every parameter (A40).

#### Scenario: Copy an approved version
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 2
- **THEN** the system creates draft version 3 with the same parameter key of `SPEED`
- **AND** the audit log holds the action `VersionCreated`

### Requirement: Snapshot check
`CreateVersion` SHALL check the snapshot of its base version, and `ApproveVersion` SHALL check the snapshot of the draft. Both SHALL run the check before the transaction. A failed check SHALL return `E-RCP-020` with one item for each problem, and the command SHALL write nothing (R29, I19).

#### Scenario: Base version with an archived unit
- **GIVEN** the retired version 1 of `R-100` with a parameter on the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 1
- **THEN** the system rejects the command with `E-RCP-020` and lists the items
- **AND** the system creates no draft

#### Scenario: Type no longer on the platform
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED`
- **AND** after a catalog data migration, the type of `SPEED` is no longer on the platform of the machine
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 2
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list holds the item `E-RCP-013` for `SPEED`
- **AND** the system creates no draft

### Requirement: Second check of CreateVersion under the locks
Under the locks, `CreateVersion` SHALL check the units and the machine again. It SHALL return `E-RCP-020` for an archived unit and `E-PLT-010` for an archived machine (R29, I19).

#### Scenario: Machine archived after the snapshot check
- **GIVEN** the approved version 2 of `R-100` on the machine `M12`
- **AND** a `CreateVersion` on `R-100` whose snapshot check passed
- **WHEN** an `ArchiveMachine` on `M12` commits before `CreateVersion` locks the machine row
- **THEN** the system rejects `CreateVersion` with `E-PLT-010`
- **AND** the system creates no draft

### Requirement: Time limit of the snapshot check
A snapshot check that takes longer than 10 seconds SHALL return `E-WEB-009` (R30, I19).

#### Scenario: Snapshot check above the time limit
- **GIVEN** the draft version 3 of `R-100` whose snapshot check takes longer than 10 seconds
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-WEB-009`
- **AND** version 3 stays a draft

### Requirement: One live draft per recipe
The system SHALL allow at most one draft per recipe. The system SHALL reject a second draft with `E-RCP-011` (A7).

#### Scenario: Second draft
- **GIVEN** the recipe `R-100` with the draft version 3
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100`
- **THEN** the system rejects the command with `E-RCP-011`
- **AND** the system creates no version

### Requirement: Allowed version transitions
The database SHALL allow only the status changes from Draft to Approved and from Approved to Retired. It SHALL also allow a deletion time on a draft. It SHALL reject every other status change. The system SHALL reject a command that needs another transition. `ApproveVersion` and `DiscardDraft` SHALL return `E-RCP-007`, `RetireVersion` SHALL return `E-RCP-009`, and a draft command SHALL return `E-RCP-002` (R17, A12, A28, I3, I4).

#### Scenario: Retired version becomes approved
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an `UPDATE` statement changes the status of version 1 to `approved`
- **THEN** the database rejects the statement with `E-RCP-033`

#### Scenario: Approve a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an Administrator runs `ApproveVersion` on version 1
- **THEN** the system rejects the command with `E-RCP-007`
- **AND** version 1 stays retired

#### Scenario: Approve a discarded draft
- **GIVEN** the draft version 3 of `R-100`, which a ProcessEngineer discarded
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-007`

#### Scenario: Retire a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an Administrator runs `RetireVersion` on version 1
- **THEN** the system rejects the command with `E-RCP-009`

#### Scenario: Discard a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** a ProcessEngineer runs `DiscardDraft` on version 1
- **THEN** the system rejects the command with `E-RCP-007`

#### Scenario: Change a parameter of a discarded draft
- **GIVEN** the draft version 3 of `R-100` with the parameter `SPEED`, which a ProcessEngineer discarded
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` in version 3
- **THEN** the system rejects the command with `E-RCP-002`

### Requirement: Frozen columns of a version
The database SHALL NOT change `id`, `recipe_id`, `plc_platform`, `number`, `based_on_version_id`, `created_by`, or `created_at` of a version. After approval, it SHALL NOT change `approved_by`, `approved_at`, `change_note`, or `content_sha256`. It SHALL NOT change a discarded draft or a retired version (R19, I4).

#### Scenario: Change of the hash of an approved version
- **GIVEN** the approved version 2 of `R-100`
- **WHEN** an `UPDATE` statement as `recipe_app` changes `content_sha256` of version 2
- **THEN** the database rejects the statement with `E-RCP-033`

### Requirement: At most one approved version
The system SHALL keep at most one approved version per recipe. `ApproveVersion` SHALL retire the previous approved version in the same transaction (R18, R27, R29, I1, I2).

#### Scenario: Approve a new version
- **GIVEN** the approved version 2 of `R-100` and the draft version 3
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 with its current row version
- **THEN** the system sets the status of version 2 to `Retired`
- **AND** the system sets the status of version 3 to `Approved`
- **AND** the audit log holds the actions `VersionRetired` and `VersionApproved`

### Requirement: Retire an approved version
The system SHALL retire an approved version without a replacement. The system SHALL reject `RetireVersion` with `E-RCP-009` for a version that is not approved (A28).

#### Scenario: Retire an approved version
- **GIVEN** the approved version 2 of `R-100`
- **WHEN** an Administrator runs `RetireVersion` on version 2 with its current row version
- **THEN** the system sets the status of version 2 to `Retired`
- **AND** the recipe `R-100` has no approved version

#### Scenario: Retire a draft
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** an Administrator runs `RetireVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-009`

### Requirement: Discard a draft
The system SHALL discard a live draft by setting its deletion time. The system SHALL reject `DiscardDraft` with `E-RCP-007` for a version that is not a draft (A28).

#### Scenario: Discard a live draft
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `DiscardDraft` on version 3
- **THEN** the system sets the deletion time of version 3
- **AND** the audit log holds the action `DraftDiscarded`

#### Scenario: Discard an approved version
- **GIVEN** the approved version 2 of `R-100`
- **WHEN** a ProcessEngineer runs `DiscardDraft` on version 2
- **THEN** the system rejects the command with `E-RCP-007`

### Requirement: Parameter changes only in a live draft
The system SHALL change a parameter, a limit, or a translation only in a live draft. The system SHALL reject such a command on another version with `E-RCP-002` (R19, I3).

#### Scenario: Add a parameter to an approved version
- **GIVEN** the approved version 2 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 2 with the tag `SPEED`
- **THEN** the system rejects the command with `E-RCP-002`
- **AND** the system creates no parameter

#### Scenario: Change a parameter of an approved version
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED`
- **WHEN** an `UPDATE` statement changes the value of `SPEED` in version 2
- **THEN** the database rejects the statement with `E-RCP-031`

#### Scenario: Move a parameter to another version
- **GIVEN** the parameter `SPEED` in the draft version 3 of `R-100`
- **WHEN** an `UPDATE` statement as the table owner `recipe_migrator` sets the version of `SPEED` to version 2
- **THEN** the database rejects the statement with `E-RCP-030`

### Requirement: Row version check of version commands
Every draft command, `ApproveVersion`, `RetireVersion`, and `DiscardDraft` SHALL compare the expected `row_version` after the version lock. The system SHALL reject a mismatch with `E-WEB-001` before it writes (R29, I12).

#### Scenario: Stale row version
- **GIVEN** the draft version 3 of `R-100` with the row version 5 and the parameter `SPEED`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the expected row version 4
- **THEN** the system rejects the command with `E-WEB-001`
- **AND** the value of `SPEED` does not change

#### Scenario: Stale row version on retirement
- **GIVEN** the approved version 2 of `R-100` with the row version 6
- **WHEN** an Administrator runs `RetireVersion` on version 2 with the expected row version 5
- **THEN** the system rejects the command with `E-WEB-001`
- **AND** version 2 stays approved

### Requirement: Fixed lock order
A plant model command or a recipe management command SHALL take only the locks that it needs. It SHALL take them in the order `machine`, `recipe`, `recipe_version`, `unit`, and then the child rows. A statement that locks several rows of one table SHALL lock them in the order of `id` (R29, I19).

#### Scenario: Archive of a unit during a parameter add
- **GIVEN** the draft version 3 of `R-100` on the machine `M12` with the live unit `U1.2`
- **AND** a running `ArchiveUnit` on `U1.2` that holds the lock of the machine row of `M12`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the unit path `U1.2`
- **THEN** `AddParameter` waits for the machine row before it locks the recipe row
- **AND** after the archive commits, the system rejects `AddParameter` with `E-PLT-009`

### Requirement: Server generates the parameter key
The system SHALL generate the parameter key on the server when it adds a parameter. The key SHALL be unique inside its version (R08, A40).

#### Scenario: Add a parameter
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `SPEED`
- **THEN** the system creates the parameter `SPEED` with a new key
- **AND** no other parameter of version 3 has that key

### Requirement: Parameter tag and English texts
The system SHALL keep each parameter tag unique inside one version. A tag SHALL match `^[A-Z][A-Z0-9_]{0,63}$`. A parameter SHALL have a `name_en` of 1 to 200 characters and a `description_en` of 1 to 4 000 characters (R08, R38, A36).

#### Scenario: Duplicate tag
- **GIVEN** the draft version 3 of `R-100` with the parameter `SPEED`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `SPEED`
- **THEN** the system rejects the command with `E-RCP-014`
- **AND** the system creates no parameter

### Requirement: Unit of a parameter
The system SHALL attach a parameter only to a live unit of the machine of its recipe (R02, I15).

#### Scenario: Archived unit
- **GIVEN** the archived unit `U1.2` of `M12` and the draft version 3 of `R-100` on `M12`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the unit path `U1.2`
- **THEN** the system rejects the command with `E-PLT-009`

#### Scenario: Unit path not found
- **GIVEN** the draft version 3 of `R-100` on `M12` and no unit `U1.9` on `M12`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the unit path `U1.9`
- **THEN** the system rejects the command with `E-PLT-020`

#### Scenario: Unit of another machine
- **GIVEN** the parameter `SPEED` in the draft version 3 of `R-100` on `M12`
- **WHEN** an `UPDATE` statement sets the unit of `SPEED` to a unit of `M3`
- **THEN** the database rejects the statement with `E-RCP-032`

### Requirement: S7 address per platform
The system SHALL require an S7 address on `S7_300`. An address SHALL be optional on `S7_1200` and `S7_1500`. The system SHALL reject an address on `CodesysIec` and `TwinCat3` (R14, A31, I5).

#### Scenario: Missing address on S7_300
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` without an address
- **THEN** the system rejects the command with `E-RCP-003`

#### Scenario: Address on TwinCat3
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M12` with the address `DB1.DBW4`
- **THEN** the system rejects the command with `E-RCP-053`

### Requirement: S7 overlap and alignment
The system SHALL reject a parameter whose S7 address overlaps another parameter of the same version. The system SHALL reject an address of an element wider than one byte that does not start on an even byte (R14, A31, I20).

#### Scenario: Overlapping address
- **GIVEN** the parameter `SPEED` at `DB1.DBW4` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP` and the address `DB1.DBW4`
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Odd start byte
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the tag `SPEED`, the type `INT`, and the address `DB1.DBW3`
- **THEN** the system rejects the command with `E-RCP-023`

### Requirement: Value matches type and limits
The system SHALL accept a parameter value only when its shape matches its type tree. The system SHALL accept it only when the value lies inside the process limits (A11, I7).

#### Scenario: Value outside the process limit
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the parameter `SPEED` of type `INT` with the process limit `min` 0 and `max` 100
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value 150
- **THEN** the system rejects the command with `E-RCP-004`

#### Scenario: Value with the wrong shape
- **GIVEN** the parameter `SPEED` of type `INT`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with a JSON string
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Canonical decimal text of a value
The system SHALL store each numeric, duration, or instant leaf as canonical decimal text with no exponent and no trailing zeros. The value `-0` SHALL become `0`. A number with more than 28 significant digits SHALL give `E-RCP-005` (R38, I7).

#### Scenario: Number with an exponent
- **GIVEN** the parameter `RATE` of type `REAL` with the process limit `min` 0 and `max` 500 in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `1.5e2`
- **THEN** the system stores the value `150`

### Requirement: Time value as a count
The system SHALL store a `Duration` or `Instant` value as the integer count of the base unit of its type (A43, I7).

#### Scenario: Store a TIME value
- **GIVEN** a parameter of type `TIME` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on that parameter with the value `T#1d2h3m4s5ms`
- **THEN** the system stores the value `93784005`

### Requirement: Process limit on a matching leaf
The system SHALL attach a process limit only to a leaf of a matching class. A numeric, duration, or instant leaf SHALL take `min` and `max`. A string leaf SHALL take only `max_length`. A bool or char leaf SHALL take no limit. A limit SHALL stay inside the range of the data type (R09, A36).

#### Scenario: Limit on a BOOL leaf
- **GIVEN** the parameter `SPEED` with a BOOL leaf in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `SPEED` with a `min` value
- **THEN** the system rejects the command with `E-RCP-019`

#### Scenario: Leaf path not found
- **GIVEN** the parameter `SPEED` of a struct type in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `SPEED` with the leaf path `.setpoint`, which the type does not have
- **THEN** the system rejects the command with `E-RCP-017`

#### Scenario: Limit outside the type range
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `SPEED` with `max` 40000
- **THEN** the system rejects the command with `E-RCP-006`

#### Scenario: Minimum above maximum
- **GIVEN** the parameter `SPEED` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `SPEED` with `min` 10 and `max` 5
- **THEN** the system rejects the command with `E-RCP-018`

### Requirement: Value change keeps limits and translations
`SetParameterValue` SHALL change only the value of a parameter. It SHALL keep the limits and the translations of the parameter (R09, I7).

#### Scenario: New value
- **GIVEN** the parameter `SPEED` with the value 120, the process limit `min` 0 and `max` 200, and the translation `pl`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value 150
- **THEN** the value of `SPEED` is 150
- **AND** the process limit and the translation `pl` of `SPEED` do not change
- **AND** the audit log holds the action `ParameterValueSet`

### Requirement: Type change needs a new value
When `UpdateParameterDefinition` changes the type of a parameter, the request SHALL carry a new value and new limits. Without a type change, the value and the limits SHALL stay, unless the request carries a new value and new limits. Then the system SHALL replace them and check them in the same way as after a type change (R08, R09, R19, I7).

#### Scenario: Type change without a value
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `DINT` and no value
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Same type without a value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, a new name, and no value
- **THEN** the value and the limits of `SPEED` stay unchanged

#### Scenario: Same type with a new value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `130`, and the limit `min 10, max 250`
- **THEN** the value of `SPEED` is `130`
- **AND** the audit entry `ParameterDefinitionUpdated` holds the old and the new value

#### Scenario: Same type with a value outside the new limit
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `300`, and the limit `min 10, max 250`
- **THEN** the system rejects the command with `E-RCP-004`
- **AND** the parameter `SPEED` does not change

### Requirement: Approval requires process limits
The system SHALL reject `ApproveVersion` while a numeric, string, duration, or instant leaf has no process limit. The system SHALL run this check before the transaction and again after the locks (R09, A36, I17).

#### Scenario: Leaf without a limit
- **GIVEN** the draft version 3 of `R-100` with the parameter `SPEED` that has no process limit
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-016`
- **AND** version 3 stays a draft

### Requirement: Replacement of limits keeps the value
The system SHALL keep the value and the translations of a parameter when it replaces its limits. The system SHALL reject a replacement that puts the current value outside the new limit with `E-RCP-004` (R09, I7).

#### Scenario: Limit below the current value
- **GIVEN** the parameter `SPEED` with the value 120 and the process limit `min` 0 and `max` 200
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `SPEED` with `max` 100
- **THEN** the system rejects the command with `E-RCP-004`
- **AND** the process limit of `SPEED` stays `max` 200

### Requirement: One translation per language
The system SHALL keep at most one translation per language of a parameter. The system SHALL accept only a language code that matches `^[a-z]{2,3}(-[A-Z]{2})?$` and is not `en` (R10, A17).

#### Scenario: Replace a translation
- **GIVEN** the parameter `SPEED` with the translation `pl`
- **WHEN** a ProcessEngineer runs `UpsertTranslation` on `SPEED` with the language `pl` and a new name
- **THEN** the system keeps one translation of `SPEED` for `pl` with the new name

#### Scenario: Language code en
- **GIVEN** the parameter `SPEED` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpsertTranslation` on `SPEED` with the language `en`
- **THEN** the system rejects the command with `E-RCP-027`

#### Scenario: Delete a missing translation
- **GIVEN** the parameter `SPEED` with no translation for `de-AT`
- **WHEN** a ProcessEngineer runs `RemoveTranslation` on `SPEED` with the language `de-AT`
- **THEN** the system rejects the command with `E-RCP-050`

### Requirement: Delete a parameter with its children
The system SHALL delete a parameter together with its limits and translations (R08).

#### Scenario: Delete a parameter
- **GIVEN** the parameter `SPEED` with a process limit and a translation in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `RemoveParameter` on `SPEED`
- **THEN** the system deletes `SPEED`, its limit, and its translation

#### Scenario: Delete a missing parameter
- **GIVEN** the draft version 3 of `R-100` with no parameter `SPEED`
- **WHEN** a ProcessEngineer runs `RemoveParameter` on `SPEED`
- **THEN** the system rejects the command with `E-RCP-051`

### Requirement: Size budgets of a version
The system SHALL reject a command that exceeds a size budget of a version with `E-RCP-021`. The budgets of a version SHALL be 262 144 leaves, 32 768 limit rows, and 4 000 parameters (R30, A26, I18).

#### Scenario: Add a parameter above 4 000
- **GIVEN** the draft version 3 of `R-100` with 4 000 parameters
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3
- **THEN** the system rejects the command with `E-RCP-021`
- **AND** the system creates no parameter

#### Scenario: Database backstop for parameters
- **GIVEN** the draft version 3 of `R-100` with 4 000 parameters
- **WHEN** an `INSERT` statement adds a 4 001st parameter to version 3
- **THEN** the database rejects the statement with `E-RCP-035`

#### Scenario: Database backstop for limit rows
- **GIVEN** the draft version 3 of `R-100` with 32 768 limit rows
- **WHEN** an `INSERT` statement adds one more limit row to version 3
- **THEN** the database rejects the statement with `E-RCP-036`

### Requirement: Approval needs parameters
The system SHALL reject approval of a draft without parameters with `E-RCP-008` (R27).

#### Scenario: Empty draft
- **GIVEN** the draft version 3 of `R-100` with no parameter
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-008`
- **AND** version 3 stays a draft

### Requirement: Separation of duties at approval
The system SHALL record each user who runs a draft command in `draft_editor`. The system SHALL reject approval by a user who created the draft or who is in `draft_editor`, while `approval.separation_of_duties` is on (R27, A5, A34, I16).

#### Scenario: Creator approves the draft
- **GIVEN** the draft version 3 of `R-100` created by a ProcessEngineer
- **WHEN** the same ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-015`

#### Scenario: Editor approves the draft
- **GIVEN** the draft version 3 of `R-100` and an Administrator who ran `SetParameterValue` on it
- **WHEN** the same Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-015`

#### Scenario: Separation of duties off
- **GIVEN** the setting `approval.separation_of_duties` with the value `false`
- **AND** the valid draft version 3 of `R-100` created by a ProcessEngineer
- **WHEN** the same ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system sets the status of version 3 to `Approved`

### Requirement: Repeated approval
The system SHALL return `AlreadyApproved` when the version is already approved and its row version equals the expected row version plus one. The system SHALL write no audit entry for that result (R27, I1).

#### Scenario: Repeated approval
- **GIVEN** the approved version 3 of `R-100` with the row version 7
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 with the expected row version 6
- **THEN** the system returns `AlreadyApproved`
- **AND** the audit log gets no new entry

### Requirement: Outdated struct revision at approval
The system SHALL reject `ApproveVersion` with `E-RCP-024` for a draft with an outdated parameter, unless the request sets `allow_outdated`. The system SHALL run this check before the transaction and again after the locks (R19, A29).

#### Scenario: Approval without the flag
- **GIVEN** the draft version 3 of `R-100` with the parameter `SPEED` of type `ST_Zone@2` and the newer revision `ST_Zone@3`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 without `allow_outdated`
- **THEN** the system rejects the command with `E-RCP-024`
- **AND** version 3 stays a draft

#### Scenario: Approval with the flag
- **GIVEN** the draft version 3 of `R-100` with the parameter `SPEED` of type `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 with `allow_outdated` set
- **THEN** the system sets the status of version 3 to `Approved`

### Requirement: Content hash at approval
The system SHALL set `content_sha256` of a version once, when it approves the version. The system SHALL reject the approval with `E-EXP-004` when it cannot build the hashed body (R27, A2).

#### Scenario: Approve a valid draft
- **GIVEN** the valid draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system sets the status of version 3 to `Approved`
- **AND** the system sets `content_sha256` of version 3

#### Scenario: Hashed body cannot be built
- **GIVEN** the valid draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 and the serializer fails
- **THEN** the system rejects the command with `E-EXP-004`
- **AND** version 3 stays a draft

### Requirement: Rebind target is a newer revision
The system SHALL reject `RebindTypeRevision` with `E-RCP-052` when `to_type_id` is not a newer revision of the struct of `from_type_id` on the same machine. It SHALL also return `E-RCP-052` when no parameter of the draft uses `from_type_id` (R19, A29).

#### Scenario: Rebind to an older revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 2 and 3, and a draft that uses `ST_Zone@3`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@3` to `ST_Zone@2`
- **THEN** the system rejects the command with `E-RCP-052`

### Requirement: Rebind needs values and changes nothing on failure
The system SHALL return one list entry for each parameter that fails a rebind. The entry SHALL hold `E-RCP-028` for a new member without a value or `E-RCP-029` for a limit that the new revision drops. The system SHALL change no parameter when any entry exists (A29).

#### Scenario: New member without a value
- **GIVEN** the draft version 3 of `R-100` with the parameters `SPEED` and `TEMP` of type `ST_Zone@2`, and the revision 3 adds the member `temp_max`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@2` to `ST_Zone@3` without a value for `temp_max`
- **THEN** the system rejects the command with `E-RCP-028`
- **AND** the list holds one entry for each of `SPEED` and `TEMP`
- **AND** the type of `SPEED` stays `ST_Zone@2`

### Requirement: IEC literal of a time value
The system SHALL accept a `Duration` or `Instant` value only as an IEC literal of `domain-model.md` section 2.1. The prefix SHALL name the type of the leaf. The count SHALL be inside the range of the type on the platform of the machine. The system SHALL show each value as its canonical literal (R11, R12, R38, A43).

#### Scenario: Parts in the wrong order
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `T#2h1d`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Prefix of another type
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `LT#5s`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Negative TIME on a platform with an unsigned range
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `T#-1s`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Canonical literal of a stored value
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `T#90m`
- **THEN** the system stores the value `5400000`
- **AND** the version page shows the value `T#1h30m`

### Requirement: Leaf path of a parameter type
A leaf path SHALL name one scalar leaf of the type tree of a parameter, with the grammar of `domain-model.md` section 2.1. A leaf path with `[*]` SHALL name every element of one array level. An index segment SHALL hold one index for each dimension of its array (R09, A38).

#### Scenario: Limit on every element of an array
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `ramps` of type `ARRAY[1..3] OF INT`
- **AND** the process limit `min` 0 and `max` 100 on the leaf path `.ramps[*]`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ZONE_1` with the value `[10, 150, 30]` for `ramps`
- **THEN** the system rejects the command with `E-RCP-004`

#### Scenario: Index outside the array bounds
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `ramps` of type `ARRAY[1..3] OF INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `ZONE_1` with the leaf path `.ramps[4]`
- **THEN** the system rejects the command with `E-RCP-017`

#### Scenario: Index count different from the dimension count
- **GIVEN** the parameter `GRID` of type `ARRAY[0..1,1..2] OF INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `GRID` with the leaf path `[1]`
- **THEN** the system rejects the command with `E-RCP-017`

### Requirement: S7 layout size of a struct and an array
The system SHALL compute the S7 layout size of a struct or an array with the `S7Layout` rules of `domain-model.md` section 2.1. Up to 8 consecutive `BOOL` members SHALL share one byte. A member wider than 1 byte SHALL start on an even byte. A struct and an array SHALL end on an even byte (R14, A31, I20).

#### Scenario: Three BOOL members occupy two bytes
- **GIVEN** the machine `M3` with the platform `S7_300` and the struct `ST_Flags` with three `BOOL` members
- **AND** the parameter `FLAGS` of type `ST_Flags@1` at the address `DB1.DBB0` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `MODE`, the type `BYTE`, and the address `DB1.DBB1`
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Parameter after the end of a struct
- **GIVEN** the parameter `FLAGS` of type `ST_Flags@1` at the address `DB1.DBB0` in the draft version 3 of `R-100` on `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP`, the type `INT`, and the address `DB1.DBW2`
- **THEN** the system adds the parameter `TEMP`

#### Scenario: Padding before a member wider than one byte
- **GIVEN** the struct `ST_Mix` of `M3` with the members `flag` of type `BOOL`, `speed` of type `INT`, and `mode` of type `BYTE`
- **AND** the parameter `MIX` of type `ST_Mix@1` at the address `DB1.DBB0` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `EXTRA`, the type `BYTE`, and the address `DB1.DBB4`
- **THEN** the system rejects the command with `E-RCP-022`

### Requirement: S7 address grammar
The system SHALL accept an S7 address only with the grammar of `domain-model.md` section 2.1. The data block number SHALL be 1 to 65535. The offset SHALL be 0 to 65535. The bit SHALL be 0 to 7, and only the width `X` SHALL have a bit (R14, R38, A31).

#### Scenario: Data block zero
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB0.DBW0`
- **THEN** the system rejects the command with `E-WEB-008`

#### Scenario: Bit outside 0 to 7
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `BOOL` and the address `DB1.DBX0.8`
- **THEN** the system rejects the command with `E-WEB-008`

#### Scenario: Bit with a byte width
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB1.DBW0.1`
- **THEN** the system rejects the command with `E-WEB-008`

### Requirement: S7 address width matches the type
The width letter of an S7 address SHALL match the type of the parameter, with the width table of `domain-model.md` section 2.1 (R14, A31).

#### Scenario: Double word address of an INT
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB1.DBD0`
- **THEN** the system rejects the command with `E-RCP-054`

#### Scenario: Byte address of a BOOL
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `BOOL` and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-RCP-054`

#### Scenario: Double word address of a REAL
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the tag `RATE`, the type `REAL`, and the address `DB1.DBD4`
- **THEN** the system adds the parameter `RATE`

### Requirement: JSON form of a leaf value
The system SHALL accept a leaf value only in the JSON form of its leaf class. A `BOOL` leaf SHALL accept only `true` and `false`. A `CHAR` or `WCHAR` leaf SHALL accept one Unicode scalar inside the range of its type. A `STRING` leaf SHALL accept only Latin-1 characters, and the number of characters SHALL fit the type length. A `WSTRING` value SHALL fit the type length in UTF-16 units (R11, R12, R38).

#### Scenario: Number for a BOOL leaf
- **GIVEN** the parameter `ENABLE` of type `BOOL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ENABLE` with the JSON number `1`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Two characters for a CHAR leaf
- **GIVEN** the parameter `SEP` of type `CHAR` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SEP` with the JSON string `"ab"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Character outside Latin-1 in a STRING leaf
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Łódź"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: STRING value longer than its type
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"ABCDE"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: WSTRING length in UTF-16 units
- **GIVEN** the parameter `NOTE` of type `WSTRING(2)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"a😀"`, which has 3 UTF-16 units
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Value inside the type range
The system SHALL check each leaf against the range of its data type before it checks the process limit. A value that the data type cannot hold SHALL give `E-RCP-005`. A number with a fraction is outside the range of an integer type (R11, R12, R38, I7).

#### Scenario: INT value above the type range
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value `40000`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Negative value of an unsigned type
- **GIVEN** the parameter `COUNT` of type `USINT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `COUNT` with the value `-1`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Fraction for an integer type
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value `1.5`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Change note of a version
Each version SHALL have a change note from the user, also version 1. A change note SHALL have 1 to 4 000 characters and no control character except a line end. A line end SHALL be LF or CRLF. A change note of spaces only SHALL count as empty. The system SHALL reject any other change note with `E-WEB-008` (R17, R38).

#### Scenario: Change note of spaces only
- **GIVEN** the approved version 2 of `R-100` and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` with a change note of three spaces
- **THEN** the system rejects the command with `E-WEB-008`

#### Scenario: Empty change note
- **GIVEN** the approved version 2 of `R-100` and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` with an empty change note
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the system creates no version

#### Scenario: Change note with two lines from a browser form
- **GIVEN** the approved version 2 of `R-100` and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` with a change note of two lines separated by CRLF
- **THEN** the system creates draft version 3 with that change note

### Requirement: Base of a new version
`CreateVersion` SHALL accept any version of the recipe as its base, also a discarded draft. Without a base, the new draft SHALL have no parameters (R17, A28).

#### Scenario: Discarded draft as the base
- **GIVEN** the recipe `R-100` with the approved version 2 and the discarded draft version 3 with the parameter `TEMP`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 3
- **THEN** the system creates draft version 4 with the parameter `TEMP`

#### Scenario: No base
- **GIVEN** the recipe `R-100` with the approved version 2 and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` without a base
- **THEN** the system creates draft version 3 without parameters

### Requirement: Recipe name
A recipe SHALL have a name of 1 to 200 characters with no control character. A name of spaces only SHALL count as empty. The system SHALL reject any other name with `E-WEB-008` (R16, R38).

#### Scenario: Name of 201 characters
- **GIVEN** the recipe `R-100` with the name `Line A`
- **WHEN** a ProcessEngineer runs `RenameRecipe` on `R-100` with a name of 201 characters
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the name of `R-100` stays `Line A`

### Requirement: Sort order of a parameter
Each parameter SHALL have a sort order from 0 to 100 000. Two parameters of one unit SHALL be able to share a sort order. The system SHALL reject a sort order outside the range with `E-WEB-008` (R38, R39).

#### Scenario: Sort order above the range
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the sort order 100 001
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the system creates no parameter

#### Scenario: Two parameters with one sort order
- **GIVEN** the parameter `SPEED` with the sort order 5 on the unit `U1` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP`, the unit `U1`, and the sort order 5
- **THEN** the system adds the parameter `TEMP`

### Requirement: One limit for each leaf path
A parameter SHALL have at most one process limit for each leaf path. The system SHALL reject a limit list with two rows for one leaf path with `E-WEB-008`. The system SHALL also reject a `[*]` row and an index row for the same array level with `E-WEB-008` (R09, A36).

#### Scenario: Two rows for one leaf path
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `setpoint` of type `INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `ZONE_1` with two rows for the leaf path `.setpoint`
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the limits of `ZONE_1` do not change

#### Scenario: All elements and one element of one array level
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `ramps` of type `ARRAY[1..3] OF INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `ZONE_1` with rows for the leaf paths `.ramps[*]` and `.ramps[2]`
- **THEN** the system rejects the command with `E-WEB-008`

### Requirement: Outdated parameter
A parameter SHALL be outdated when a type in its resolved type tree has a newer revision. The tree SHALL hold the parameter type, every member type, and every element type at any depth. A successful `CreateVersion` SHALL list the outdated parameters of the new draft (R19, A29).

#### Scenario: Array of an old struct revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 2 and 3
- **AND** the parameter `ZONES` of type `ARRAY[1..4] OF ST_Zone@2` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 without `allow_outdated`
- **THEN** the system rejects the command with `E-RCP-024`

#### Scenario: Member of an old struct revision
- **GIVEN** the struct `ST_Line@1` of `M12` with the member `zone` of type `ST_Zone@2`, and the newer revision `ST_Zone@3`
- **AND** the parameter `LINE` of type `ST_Line@1` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 without `allow_outdated`
- **THEN** the system rejects the command with `E-RCP-024`

#### Scenario: New draft lists the outdated parameters
- **GIVEN** the approved version 2 of `R-100` with the parameters `SPEED` of type `INT` and `ZONE_1` of type `ST_Zone@2`, and the newer revision `ST_Zone@3`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 2
- **THEN** the system creates draft version 3
- **AND** the result lists `ZONE_1` and does not list `SPEED`

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
