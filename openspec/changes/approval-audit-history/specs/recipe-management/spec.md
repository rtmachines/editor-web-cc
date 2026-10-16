## ADDED Requirements

### Requirement: Diff of two versions
`DiffVersions` SHALL match the parameters of two versions of one recipe by `key`. A key only in the new version SHALL give an added parameter. A key only in the old version SHALL give a parameter of the old version only. A key in both versions SHALL give a changed parameter when its definition, a leaf value, a limit row, or a translation differs. The result SHALL hold no parameter without a difference (R27, R39, A40, ADR-0050).

#### Scenario: Parameter added in the new version
- **GIVEN** the version 1 of `R-100` without `HOLD` and the version 2 with `HOLD`
- **WHEN** a ProcessEngineer runs `DiffVersions` with the new version 2 and the old version 1
- **THEN** the result holds `HOLD` in the section of the added parameters

#### Scenario: Parameter without a change
- **GIVEN** the versions 1 and 2 of `R-100` with the same `SPEED` in both
- **WHEN** a ProcessEngineer runs `DiffVersions` on the versions 2 and 1
- **THEN** the result holds no row of `SPEED`
- **AND** the summary counts `SPEED` among the parameters without a change

### Requirement: Limit rows in a diff
The diff SHALL compare the limit rows by their stored leaf path, with `min`, `max`, and `max_length`. A `[*]` row SHALL be one row. A change from a `[*]` row to element rows SHALL show the `[*]` row on the old side only. It SHALL show each element row on the new side only. The limits section SHALL also hold the limit rows of an added parameter. It SHALL also hold the limit rows of a parameter of the old version only (R09, R27, ADR-0022, ADR-0050).

#### Scenario: Limit of every element
- **GIVEN** `SPEED_TABLE` of type `ARRAY[1..8] OF INT` with the row `[*]` `min` 0 in version 1 and `min` 10 in version 2
- **WHEN** a ProcessEngineer runs `DiffVersions` on the versions 2 and 1
- **THEN** the limits section holds one row of `SPEED_TABLE` with the path `[*]`, the bound `min`, the old text `0`, and the new text `10`

#### Scenario: Change of the limit mode
- **GIVEN** `SPEED_TABLE` with the row `[*]` in version 1 and the rows `[1]` to `[8]` in version 2
- **WHEN** a ProcessEngineer runs `DiffVersions` on the versions 2 and 1
- **THEN** the limits section holds the row `[*]` with an empty new side
- **AND** it holds the rows `[1]` to `[8]` with an empty old side

### Requirement: Limit rows after a type change
When the type of a parameter changes, the limits section SHALL hold every limit row of both sides. This SHALL include a row with an equal stored bound. A new type can give a stored bound a new meaning (R09, R27, A43, ADR-0022, ADR-0050).

#### Scenario: Type change keeps the stored bound
- **GIVEN** `HOLD` of type `TIME` with the stored `max` 1000 in version 1 and of type `LTIME` with the same stored `max` in version 2
- **WHEN** a ProcessEngineer runs `DiffVersions` on the versions 2 and 1
- **THEN** the limits section holds the row of `HOLD` with the old text `T#1s` and the new text `LT#1us`

### Requirement: Values in a diff
The diff SHALL compare a value per leaf with the texts of the value form. A text is canonical decimal text, or an IEC literal for a duration or an instant. When the type changes, the diff SHALL show every leaf of both sides, also a leaf with an equal text. A leaf of one side only SHALL show with an empty other side. Each value row SHALL also hold the limit of the new version for that leaf (R13, A43, ADR-0039, ADR-0050).

#### Scenario: Time value
- **GIVEN** `HOLD` of type `TIME` with the value `T#5s` in version 1 and `T#7s` in version 2
- **WHEN** a ProcessEngineer runs `DiffVersions` on the versions 2 and 1
- **THEN** the values section holds a row of `HOLD` with the old text `T#5s` and the new text `T#7s`

#### Scenario: Value with a decimal comma in the UI language
- **GIVEN** `LINE_A` with `.zone.setpoint` 85.5 in version 1 and 90 in version 2
- **WHEN** a ProcessEngineer with the UI language `pl` runs `DiffVersions` on the versions 2 and 1
- **THEN** the row holds the texts `85.5` and `90`

### Requirement: One read state for the preview and the diff
`GetApprovalPreview` and `DiffVersions` SHALL read every row of both versions in one read state of the database. The `row_version` of the preview SHALL belong to the same state as its diff and its blockers (R31, I12, ADR-0050).

#### Scenario: Command between two reads of the preview
- **GIVEN** the draft version 2 of `R-100` with `row_version` 7
- **WHEN** `GetApprovalPreview` reads the draft and a draft command commits before the next read of the query
- **THEN** the result holds `row_version` 7 and the diff of `row_version` 7

### Requirement: Approval checks of the preview
`GetApprovalPreview` SHALL check its permission first. It SHALL run the state check, the separation of duties for the current user, and `check_for_approval` with `allow_outdated` set to true. The separation of duties SHALL run before `check_for_approval`, so the result can hold `E-RCP-015` with the other blockers. It SHALL write nothing. The separation of duties SHALL use the same domain function as `ApproveVersion` (R27, R28, A34, A36, ADR-0049).

#### Scenario: Operator asks for the preview
- **GIVEN** the Operator `mhoffman` and the draft version 2 of `R-100`
- **WHEN** `mhoffman` runs `GetApprovalPreview` on version 2
- **THEN** the system returns `Forbidden` and reads no port

#### Scenario: Preview of an editor
- **GIVEN** the draft version 2 of `R-100`, which `anowak` edited, and separation of duties on
- **WHEN** `anowak` runs `GetApprovalPreview` on version 2
- **THEN** the result holds the blocker `E-RCP-015`
- **AND** the system writes no audit entry and no row

#### Scenario: Preview of an editor with a missing limit
- **GIVEN** the draft version 2 of `R-100`, which `anowak` edited, with a numeric leaf of `HOLD` without a limit
- **WHEN** `anowak` runs `GetApprovalPreview` on version 2
- **THEN** the result holds the blocker `E-RCP-015` and a blocker `E-RCP-016` of `HOLD`

#### Scenario: Preview of a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** a ProcessEngineer runs `GetApprovalPreview` on version 1
- **THEN** the system returns `E-RCP-007`

### Requirement: Bounds of a diff
A diff page SHALL load the parameter pairs of one page only. One page SHALL hold at most `page.diff` parameters over all its sections. Without `kind`, the sections SHALL fill the page in their order up to that bound. A section SHALL hold at most `diff.leaf_rows` rows of one parameter (R30, A26, ADR-0050).

#### Scenario: Page without a section
- **GIVEN** two versions of `R-100` with 40 parameters of changed limits and 30 parameters of changed values
- **WHEN** a ProcessEngineer runs `DiffVersions` on the two versions without `kind`
- **THEN** the page holds the 40 parameters of the limits section and 10 parameters of the values section
- **AND** the values section links to its own pages

### Requirement: Permits of a diff
At most `diff.permits` diffs SHALL run at the same time. `DiffVersions` and `GetApprovalPreview` SHALL take the permit before the first read and hold it to the end of the render. A diff that does not end within `timeout.validation` SHALL return `E-WEB-009` (R30, A26, ADR-0050).

#### Scenario: Busy diffs
- **GIVEN** `diff.permits` diffs that hold a permit
- **WHEN** another diff waits longer than `timeout.validation`
- **THEN** the system returns `E-WEB-009`

### Requirement: Number of the retired version
`ApproveVersion` SHALL return the number of the version that the approval retires, when one exists (R27, ADR-0049).

#### Scenario: Approval retires the old version
- **GIVEN** the approved version 1 and the draft version 2 of `R-100`
- **WHEN** an Administrator runs `ApproveVersion` on version 2
- **THEN** the result names version 1 as the retired version
