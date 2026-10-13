# recipe-management

## Purpose

The recipe management capability describes the recipes of one machine, their versions, the editable draft, and the parameters with their process limits. It also describes approval, retirement, and the rebinding of struct revisions.

## ADDED Requirements

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
The database SHALL allow only the status changes from Draft to Approved and from Approved to Retired. It SHALL also allow a deletion time on a draft. It SHALL reject every other status change (R17, A12, I4).

#### Scenario: Retired version becomes approved
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an `UPDATE` statement changes the status of version 1 to `approved`
- **THEN** the database rejects the statement with `E-RCP-033`

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
When `UpdateParameterDefinition` changes the type of a parameter, the request SHALL carry a new value and new limits. Without a type change, the value and the limits SHALL stay (R08, R19, I7).

#### Scenario: Type change without a value
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `DINT` and no value
- **THEN** the system rejects the command with `E-RCP-005`

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
