## ADDED Requirements

### Requirement: Permission first in each recipe query
Each query of recipes, versions, and parameters SHALL check its permission as its first statement. `ListRecipes`, `GetRecipe`, `ListVersions`, `GetVersion`, `ListParameters`, `GetParameter`, `GetLimits`, and `GetTranslations` SHALL need `RecipeView`. `ListDeletedRecipes` SHALL need `RecipeRestore`. Each form query SHALL need the permission of its command (R24, R28).

#### Scenario: Operator asks for a parameter form
- **GIVEN** the Operator `mhoffman` and the draft version 3 of `R-100`
- **WHEN** `mhoffman` runs `GetParameterForm` on version 3
- **THEN** the system returns `Forbidden` and reads no port

### Requirement: Recipe list query
`ListRecipes` SHALL return at most `page.recipes` recipes of one page. It SHALL keep the recipes that are not deleted. It SHALL match `q` with a part of the reference number or the name with no difference of case. It SHALL order the rows by the bytes of the reference number (R16, R24, A19).

#### Scenario: Deleted recipe hidden
- **GIVEN** the recipes `R-100` and the deleted recipe `R-300`
- **WHEN** a ProcessEngineer runs `ListRecipes` with no filter
- **THEN** the result holds `R-100` and does not hold `R-300`

#### Scenario: Deleted recipes for the Administrator
- **GIVEN** the recipes `R-100` and the deleted recipe `R-300`
- **WHEN** an Administrator runs `ListDeletedRecipes`
- **THEN** the result holds `R-100` and `R-300`

### Requirement: Read of a deleted recipe
`GetRecipe`, `ListVersions`, `GetVersion`, and the parameter queries SHALL return the data of a deleted recipe only to a user with `RecipeRestore`. They SHALL return `E-RCP-012` to every other user (A14, R24).

#### Scenario: ProcessEngineer reads a deleted recipe
- **GIVEN** the deleted recipe `R-300`
- **WHEN** a ProcessEngineer runs `GetRecipe` on `R-300`
- **THEN** the system returns `E-RCP-012`

### Requirement: Form only for a live draft
`GetParameterForm` SHALL return the form only for a live `Draft`. For a version in another state, it SHALL return `E-RCP-002`. The check of a deleted recipe SHALL come first (R17, R25).

#### Scenario: ProcessEngineer asks for the form of an approved version
- **GIVEN** the ProcessEngineer `jsmith` and the approved version 2 of `R-100`
- **WHEN** `jsmith` runs `GetParameterForm` on version 2
- **THEN** the system returns `E-RCP-002` and gives no form

### Requirement: One read state for a draft query
`GetVersion`, `ListParameters`, `GetParameter`, `GetParameterForm`, `GetLimits`, and `GetTranslations` SHALL read every row in one read state of the database. The `row_version` of the result SHALL belong to the same state as its parameter data (R29, R31, I12).

#### Scenario: Command between two reads
- **GIVEN** the draft version 3 of `R-100` with `row_version` 7
- **WHEN** `GetVersion` reads the version and a draft command commits before the next read of the query
- **THEN** the result holds `row_version` 7 and the parameter data of `row_version` 7

### Requirement: Last change of a version
`GetVersion` SHALL return the time of the last change of the version and the login of the user of that change. The time SHALL be the `updated_at` of the version. The user SHALL be the actor of the audit entry of the recipe with that time (R20, R36).

#### Scenario: Last editor
- **GIVEN** the draft version 3 of `R-100`, which `anowak` changed last with `SetParameterValue`
- **WHEN** a ProcessEngineer runs `GetVersion` on version 3
- **THEN** the result names `anowak` and the `updated_at` of version 3

### Requirement: Value from the texts of its leaves
The system SHALL build a parameter value from one text per leaf of its type tree. A number SHALL use decimal text with a dot, and a bool SHALL use `true` or `false`. A string or a char SHALL use the string, and a time SHALL use an IEC literal. A missing leaf or an unknown leaf path SHALL give `E-RCP-005` (A11, A43, R38).

#### Scenario: Partial value
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT`
- **WHEN** a ProcessEngineer runs `SetParameterValue` with texts for the leaves `[1]` to `[7]`
- **THEN** the system rejects the command with `E-RCP-005` for the leaf `[8]`

#### Scenario: Decimal comma
- **GIVEN** the parameter `SETPOINT` of type `REAL`
- **WHEN** a ProcessEngineer runs `SetParameterValue` with the text `1,5`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Time literal
- **GIVEN** the parameter `HOLD` of type `TIME`
- **WHEN** a ProcessEngineer runs `SetParameterValue` with the text `T#1m30s`
- **THEN** the system stores the count `90000`

### Requirement: Texts of a value for a page
The system SHALL write the text of each leaf of a stored value for a page. A number SHALL use the canonical decimal text. A time SHALL use the canonical IEC literal. The texts SHALL build the same value again (A43).

#### Scenario: Round trip of a struct value
- **GIVEN** a stored value of type `ST_Zone@2` with the setpoint `182.5` and the hold count `90000`
- **WHEN** the system writes the texts of the leaves
- **THEN** the texts are `182.5` and `T#1m30s`
- **AND** the texts build a value equal to the stored value

### Requirement: Every fault of a value and of its limits
`AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, and `ReplaceLimits` SHALL check every leaf and every limit row. They SHALL return every fault of one parameter in one list. The faults of the leaves SHALL come first in preorder, and the faults of the limit rows SHALL follow in their order (R36, A36).

#### Scenario: Two leaves out of the limit
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` with the limit 0 to 3000 on `[*]`
- **WHEN** a ProcessEngineer runs `SetParameterValue` with 3500 in `[3]` and 4000 in `[6]`
- **THEN** the system rejects the command with a list of two `E-RCP-004` entries for `[3]` and `[6]`
- **AND** the value of `SPEED_TABLE` does not change

### Requirement: Texts of the new members of a rebind
`RebindTypeRevision` SHALL take the text of each new member leaf. The class of the leaf in the new revision SHALL convert the text as for a value (A29, A43).

#### Scenario: Number of a new member
- **GIVEN** the parameter `ZONE_1` of type `ST_Zone@2` and the revision `ST_Zone@3` with the new member `ramp` of type `REAL`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` with the text `12.5` for `ZONE_1` and `.ramp`
- **THEN** the system stores the value `12.5` for `.ramp` of `ZONE_1`
