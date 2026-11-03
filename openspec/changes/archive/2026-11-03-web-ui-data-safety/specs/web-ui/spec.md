## ADDED Requirements

### Requirement: Answer of a translation deletion
An HTMX success of `RemoveTranslation` SHALL replace only the row of the deleted tag. The new row SHALL show the tag without a stored translation. The answer SHALL NOT replace another row, so the unsaved input of the other rows SHALL stay. The answer SHALL set `HX-Trigger` with the new `row_version` (R10, R31, R39, A17, ADR-0111).

#### Scenario: Unsaved input of another row stays
- **GIVEN** the ProcessEngineer `anowak` on the page of `SPEED` of the draft version 3 of `R-100` with the stored translations `pl` and `de`
- **AND** `anowak` typed a new name into the row `pl` and did not save it
- **WHEN** `anowak` deletes the translation `de` and gets `200`
- **THEN** the answer holds the row with the id `translation-de` and no row with the id `translation-pl`
- **AND** the row `pl` of the page still shows the typed name

#### Scenario: Deleted tag of the list
- **GIVEN** `i18n.translation_langs` with `pl` and `de` and the parameter `SPEED` with the stored translation `de`
- **WHEN** `anowak` sends an HTMX `POST .../parameters/<key>/translations/de/delete` with the current `row_version`
- **THEN** the answer row `translation-de` holds an empty form that sends `POST .../translations/de`
- **AND** the answer row holds no delete form

#### Scenario: Deleted tag outside the list
- **GIVEN** `i18n.translation_langs` with `pl` and the parameter `LINE` with the stored translation `fr`
- **WHEN** `anowak` sends an HTMX `POST .../parameters/<key>/translations/fr/delete` with the current `row_version`
- **THEN** the answer row `translation-fr` shows the sentence of `translation-none` and holds no form

#### Scenario: New row version for the other rows
- **GIVEN** the draft version 3 of `R-100` with `row_version` 7
- **WHEN** `anowak` deletes the translation `de` and gets `200`
- **THEN** the answer has the header `HX-Trigger` with `rowVersion` 8
- **AND** the field `row_version` of the form `pl` holds 8 without a page load
- **AND** a later save of the row `pl` with `row_version` 8 gives `200`

#### Scenario: Deletion of the last stored translation
- **GIVEN** `i18n.translation_langs` with `pl` and `de` and the parameter `SPEED` with only the stored translation `pl`
- **WHEN** `anowak` deletes the translation `pl` and gets `200`
- **THEN** the answer row `translation-pl` holds an empty form and no delete form

#### Scenario: Error answer keeps every row
- **GIVEN** another session changed the draft version 3 of `R-100` after `anowak` loaded the page
- **WHEN** `anowak` deletes the translation `de`
- **THEN** the server returns `409 E-WEB-001` with `HX-Retarget: #error-panel`
- **AND** no translation row of the page changes

### Requirement: Focus after a translation deletion
The heading of the answer row of a translation deletion SHALL have `tabindex="-1"` and `autofocus`. After the swap the focus SHALL be on that heading (A44, ADR-0111).

#### Scenario: Focus on the heading of the row
- **GIVEN** `anowak` on the page of `SPEED` with the stored translation `de`
- **WHEN** `anowak` opens the delete disclosure of `de` and activates the delete button with the keyboard
- **THEN** the heading of the row `translation-de` has the focus

### Requirement: English base text after a save of the definition
An HTMX success of `UpdateParameterDefinition` SHALL also replace the English base text above the translation rows out of band. The answer SHALL NOT replace a translation row, so the unsaved input of the rows SHALL stay (R08, R10, R31, R39, ADR-0111).

#### Scenario: New English name above the translations
- **GIVEN** `anowak` on the page of `SPEED` of the draft version 3 of `R-100` with the stored translation `pl`
- **AND** `anowak` typed a new name into the row `pl` and did not save it
- **WHEN** `anowak` saves the edit form with the new `name_en` `Press speed` and gets `200`
- **THEN** the answer holds the block `translations-base` with `hx-swap-oob="true"` and the text `Press speed`
- **AND** the answer holds no element with the class `translation-row`
- **AND** the row `pl` of the page still shows the typed name

### Requirement: Multi-line text field
The fields `description_en`, the `description` of a translation row, and `change_note` SHALL be multi-line text fields with four rows and a maximum of 4 000 characters. Each field SHALL show a limit line with the maximum count and SHALL name its hint, its limit line, and its error line in `aria-describedby`. Every other text field SHALL stay a single-line input (R08, R10, R39, A44, ADR-0112).

#### Scenario: Description field of the edit form
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` opens the page of `SPEED` of the draft version 3 of `R-100`
- **THEN** the edit form holds a `textarea` with the id `description_en`, `rows="4"`, and `maxlength="4000"`
- **AND** its `aria-describedby` names `description_en-limit`
- **AND** the field `name_en` stays an `input` with `type="text"`

#### Scenario: Change note of a new version
- **GIVEN** `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/new`
- **THEN** the form holds a `textarea` with the id `change_note` and `maxlength="4000"`

#### Scenario: Stored text with a line end in the field
- **GIVEN** the parameter `SPEED` with the description `Line 1` LF `Line 2`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the value of the `textarea` `description_en` is `Line 1` LF `Line 2`

#### Scenario: Leading line end and markup in the field
- **GIVEN** the parameter `SPEED` with a description that starts with LF and holds `<b>`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the value of the `textarea` `description_en` starts with LF
- **AND** the page holds `&lt;b&gt;` and no element `b` inside the `textarea`

### Requirement: Line ends of a multi-line field
The server SHALL convert each CRLF of a multi-line text field to LF before it checks the text. A lone CR SHALL stay, and the check SHALL refuse it. The limit of 4 000 characters SHALL count each line end as one character. The limit SHALL count Unicode characters, not UTF-16 code units (R31, R38, ADR-0112).

#### Scenario: CRLF stored as LF
- **GIVEN** `anowak` on the page of `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` saves the edit form with the description `Line 1` CRLF `Line 2`
- **THEN** the server returns `200`
- **AND** the stored description of `SPEED` is `Line 1` LF `Line 2`

#### Scenario: Text at the limit with line ends
- **GIVEN** a change note of 3 990 letters and 10 line ends, sent as CRLF
- **WHEN** `anowak` sends `POST /recipes` for a new recipe with that change note
- **THEN** the server creates the recipe
- **AND** the stored change note of its version 1 has 4 000 characters

#### Scenario: Characters outside the Basic Multilingual Plane
- **GIVEN** a change note of 4 000 characters `𝄞` (U+1D11E)
- **WHEN** `anowak` sends `POST /recipes` for a new recipe with that change note
- **THEN** the server creates the recipe

#### Scenario: Lone CR refused
- **GIVEN** `anowak` on the page of `SPEED`
- **WHEN** `anowak` saves the edit form with the description `Line 1` CR `Line 2`
- **THEN** the server returns `422` with a field error of `description_en`
- **AND** the stored description does not change

### Requirement: Save keeps the line ends of a description
A save of the edit form of a parameter SHALL keep each line end of the stored `description_en`. This rule SHALL also apply when the user changes only a value (R08, R31, ADR-0112).

#### Scenario: Value save with a two-line description
- **GIVEN** the parameter `SPEED` with the description `Line 1` LF `Line 2`
- **WHEN** `anowak` changes only the value of `SPEED` and saves the edit form
- **THEN** the stored description of `SPEED` is `Line 1` LF `Line 2`

### Requirement: Multi-line text outside a form
A page SHALL show each line end of a description or a change note outside a form as a new line (R39, ADR-0112).

#### Scenario: Description in the parameter summary
- **GIVEN** the parameter `SPEED` with the description `Line 1` LF `Line 2`
- **WHEN** the Operator `mhoffman` opens the page of `SPEED`
- **THEN** the summary shows `Line 1` and `Line 2` on two lines

#### Scenario: Change note on the version page
- **GIVEN** the version 3 of `R-100` with the change note `Step 1` LF `Step 2`
- **WHEN** `mhoffman` opens the page of the version 3 of `R-100`
- **THEN** the page shows `Step 1` and `Step 2` on two lines

### Requirement: Controls of a blocked command in read-only mode
In read-only mode a page SHALL show no control of a command that the mode blocks. A control is a form that sends the command, or a link or button whose only target is the form of the command. The login form, the logout form, the language switch, and the export link SHALL stay. Every form with `method="get"`, every pager, and every navigation link SHALL also stay (R28, R31, R39, A24, ADR-0113).

#### Scenario: Parameter page of a draft
- **GIVEN** the application in read-only mode and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` opens the page of `SPEED` of the draft version 3 of `R-100`
- **THEN** the page holds the read-only grid of the value and the limits
- **AND** the page holds the logout form and the language switch
- **AND** the page holds no other form with `method="post"`
- **AND** the page shows the read-only banner

#### Scenario: Version page of a draft
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` opens the page of the draft version 3 of `R-100`
- **THEN** the page holds no link "Add a parameter", no link "Approve", no rebind form, and no danger disclosure
- **AND** the page holds the compare form

#### Scenario: Links to the form of a new version or revision
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` opens the page of `R-100` and the page of a UDT type
- **THEN** the recipe page holds no link to the form of a new version
- **AND** the type page holds no link to the form of a new revision

#### Scenario: Export link of an approved version
- **GIVEN** the application in read-only mode and the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` opens the page of the `Approved` version 2 of `R-100`
- **THEN** the page holds the export link

#### Scenario: Same page in normal mode
- **GIVEN** the application not in read-only mode and `anowak` with an open session
- **WHEN** `anowak` opens the page of `SPEED` of the draft version 3 of `R-100`
- **THEN** the page holds the edit form and the translation forms

### Requirement: Form route in read-only mode
In read-only mode a route that answers the form of a blocked command SHALL run its permission check first. It SHALL then answer `200` with its heading and the notice `readonly-form-unavailable` in place of the form. The notice SHALL use text and an icon, not color alone (R28, R39, A24, A44, ADR-0113).

#### Scenario: Form of a new recipe
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes/new`
- **THEN** the server returns `200` with the heading of the page and the notice `readonly-form-unavailable`
- **AND** the page holds no form that sends `POST /recipes`

#### Scenario: Fragment of the form of a new parameter
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` sends an HTMX `GET .../parameters/new` with `HX-Target: parameter-form`
- **THEN** the server returns `200` with the notice `readonly-form-unavailable`
- **AND** the answer holds no form that sends `POST .../parameters`

#### Scenario: Form route without the permission
- **GIVEN** the application in read-only mode and the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes/new`
- **THEN** the server returns `403 E-WEB-002`

#### Scenario: Forced password change in read-only mode
- **GIVEN** the application in read-only mode and the user `mhoffman` with `must_change_password` set to true
- **WHEN** `mhoffman` sends `GET /me/password`
- **THEN** the page shows the notice `readonly-form-unavailable` and the logout button
- **AND** the page holds no form that sends `POST /me/password`
