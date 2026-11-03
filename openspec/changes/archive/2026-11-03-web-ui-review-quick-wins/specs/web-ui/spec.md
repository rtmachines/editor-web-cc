## ADDED Requirements

### Requirement: Password reset in a danger disclosure
The user page SHALL show the form of `ResetPassword` inside a danger disclosure of `design-system.md` section 10.5. The `summary` SHALL name the action. The warning SHALL have the id `danger-notice-reset`. The button SHALL name the login and SHALL name that id in `aria-describedby`. The form SHALL keep `hx-sync="closest form:drop"`. The deactivate form SHALL stay outside a disclosure (R28, R39, A44, ADR-0110).

#### Scenario: Closed disclosure on the user page
- **GIVEN** the Administrator `akowal` with an open session and the active user `anowak`
- **WHEN** `akowal` sends `GET /users/anowak`
- **THEN** the form that sends `POST /users/anowak/reset-password` stands inside a `details` element with no attribute `open`
- **AND** its button names `anowak` and has `aria-describedby="danger-notice-reset"`
- **AND** the page holds one element with the id `danger-notice-reset`

### Requirement: User page state after a user command
This requirement applies to a success of `SetUserRole`, `DeactivateUser`, `ActivateUser`, or `UnlockLogin` from the user page. An HTMX answer SHALL replace the facts of the user and the action forms out of band. The new facts SHALL come from `GetUser` after the commit. The answer SHALL keep `#user-result` as its target and SHALL NOT set `HX-Refresh`. An answer to a request without `HX-Request` SHALL be the full result page and SHALL hold no out-of-band element (R28, R39, A22).

#### Scenario: Status after a deactivation
- **GIVEN** the Administrator `akowal` on the page of the active user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/deactivate` and gets `200`
- **THEN** the answer replaces the facts of `anowak` out of band with the state `Inactive`
- **AND** the answer replaces the action forms out of band with the activate form

#### Scenario: Failed attempts after an unlock
- **GIVEN** the Administrator `akowal` on the page of the user `anowak` with 4 failed attempts
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/unlock` and gets `200`
- **THEN** the answer replaces the facts of `anowak` out of band with 0 failed attempts
- **AND** the replaced action forms hold no unlock form

#### Scenario: Request without HTMX
- **GIVEN** the Administrator `akowal` on the page of the active user `anowak`
- **WHEN** `akowal` sends `POST /users/anowak/deactivate` without the header `HX-Request`
- **THEN** the answer is the full result page
- **AND** the answer holds no element with `hx-swap-oob`

### Requirement: User page state after a password reset
This requirement applies to a success of `ResetPassword` from the user page. An HTMX answer SHALL replace the facts of the user out of band. The facts SHALL come from `GetUser` before the command and SHALL show the forced password change from the role. The answer SHALL NOT replace the action forms and SHALL NOT replace the reset disclosure. The answer SHALL keep `#user-result` as its target and SHALL NOT set `HX-Refresh` (R28, R39, A22, ADR-0109).

#### Scenario: One-time password stays visible
- **GIVEN** the Administrator `akowal` on the page of the user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/reset-password` and gets `200`
- **THEN** `#user-result` shows the one-time password
- **AND** the answer replaces the facts of `anowak` out of band, and they show the forced password change
- **AND** the answer replaces no action form and no reset disclosure
- **AND** the answer has no header `HX-Refresh`

#### Scenario: Reset of an export client
- **GIVEN** the Administrator `akowal` on the page of the user `plc-line1` with the role `ExportClient`
- **WHEN** `akowal` sends an HTMX `POST /users/plc-line1/reset-password` and gets `200`
- **THEN** `#user-result` shows the one-time password
- **AND** the facts of `plc-line1` show no forced password change

#### Scenario: Reset when the read fails
- **GIVEN** the Administrator `akowal` and a store that fails the read of the user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/reset-password`
- **THEN** the answer is the error fragment of the read error
- **AND** the password of `anowak` does not change


### Requirement: Focus after a user command
After a deactivation or an activation, the focus SHALL stay on the status button. After an unlock, the next Tab SHALL reach the first focusable element of the new answer in `#user-result`. After a reset, the focus SHALL stay on the reset button (R28, A22).

#### Scenario: Focus after a deactivation
- **GIVEN** the Administrator `akowal` on the page of the active user `anowak`, with the focus on the deactivate button
- **WHEN** `akowal` activates the button and the swap ends
- **THEN** the focus stands on the element with the id `user-status-submit`

#### Scenario: Focus after an unlock
- **GIVEN** the Administrator `akowal` on the page of the user `anowak` with 4 failed attempts, with the focus on the unlock button
- **WHEN** `akowal` activates the button, the swap ends, and `akowal` presses Tab
- **THEN** the focus stands on the first link inside `#user-result`

#### Scenario: Focus after a reset
- **GIVEN** the Administrator `akowal` on the page of the user `anowak`, with the focus on the reset button
- **WHEN** `akowal` activates the button and the swap ends
- **THEN** the focus stands on the reset button
- **AND** the one-time password is visible in `#user-result`

### Requirement: Applicable actions of the user page
The user page SHALL show the deactivate form only for an active user and the activate form only for an inactive user. The two submit buttons SHALL have the same fixed id. The page SHALL show the unlock form only when the user has at least one failed attempt (R28, R39, A22).

#### Scenario: Active user without failed attempts
- **GIVEN** the Administrator `akowal` and the active user `anowak` with no failed attempt
- **WHEN** `akowal` sends `GET /users/anowak`
- **THEN** the page holds the deactivate form and no activate form
- **AND** the page holds no unlock form

#### Scenario: Inactive user with failed attempts
- **GIVEN** the Administrator `akowal` and the inactive user `anowak` with 4 failed attempts
- **WHEN** `akowal` sends `GET /users/anowak`
- **THEN** the page holds the activate form, no deactivate form, and the unlock form

### Requirement: Machine unit column of the parameter table
The parameter table of the version page SHALL show the unit path under the header of the key `parameters-col-unit`. The key SHALL give `Jednostka maszyny` in `pl` and `Machine unit` in `en`. A user then does not read the column as a unit of measure (R06, R10, R39).

#### Scenario: Header in Polish
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `pl`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the parameter table has the column header `Jednostka maszyny`

### Requirement: Parent link in the page header
Each parameter, version, unit, recipe, and machine page SHALL show a link to the parent page in its header. The link SHALL follow the `h1`. A page with a large state badge keeps the badge next to the `h1`. On that page, the link SHALL follow the row of the badge. The link SHALL stand before the first `dl` of the page. The link SHALL have the target and the text of the parent link at the end of the page. The link at the end SHALL stay (R39, A44).

#### Scenario: Parameter page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the element after the `h1` holds a link to `/recipes/R-100/versions/3`

#### Scenario: Version page with a state badge
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the row of the large state badge follows the `h1`
- **AND** the element after that row holds a link to `/recipes/R-100`

#### Scenario: Machine page
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the element after the row of the large state badge holds a link to `/machines`

### Requirement: Heading of a parameter page
The `h1` and the `title` of a parameter page SHALL name the tag, the reference number of the recipe, and the number of the version. The key `parameter-title` SHALL take the arguments `tag`, `reference`, and `number`. This applies to the page and to every answer that renders the form page of a parameter, such as the answer for `fields=1` (R16, R39).

#### Scenario: Heading in English
- **GIVEN** the ProcessEngineer `anowak` with the UI language `en` and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the `h1` holds `SPEED`, `R-100`, and `3`
- **AND** the `title` holds `SPEED`, `R-100`, and `3`

#### Scenario: Heading of the form page
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl` and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends the request for the form page of `SPEED` with `fields=1`
- **THEN** the `h1` and the `title` hold `SPEED`, `R-100`, and `3`
- **AND** the page shows no unresolved message key

### Requirement: Links of the type usage page
Each row of the type usage page SHALL link the reference number to the recipe page. It SHALL link the version number to the version page and the tag to the parameter page. Each link SHALL put the user data only into its `href` (R13, R19, R39).

#### Scenario: Row with links
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100` with the parameter `LINE` of the type `ST_Line@1`, which holds `ST_Zone@2`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone/usage?rev=2`
- **THEN** the row of `LINE` links `R-100` to `/recipes/R-100`
- **AND** the row links `3` to `/recipes/R-100/versions/3`
- **AND** the row links `LINE` to the parameter page of `LINE`

### Requirement: Language of translated text
Each stored text of a translation row and each input of a translation form SHALL have the attribute `lang` with the language code of its row. The label of the input SHALL keep the UI language (R10, A17, A44).

#### Scenario: German translation form
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`, the language `de` in the list of translation languages, and the parameter `SPEED` with a German translation
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the name input and the description input of the row `de` have `lang="de"`
- **AND** their labels have no attribute `lang`

### Requirement: Link to the save button after the leaf grid
A parameter form that shows the leaf grid SHALL show a link after the grid. The `href` of the link SHALL be `#` followed by the id of the save button of the form. The link SHALL have the class `target-link` and SHALL hold no `hx-*` attribute (R39, A44).

#### Scenario: Edit form of a parameter
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the edit form holds a link with `href="#parameter-save-submit"` after the leaf grid

#### Scenario: Jump on a narrow screen
- **GIVEN** the page of `SPEED` at a width of 320 CSS px with the end of the leaf grid in view
- **WHEN** the user activates the link after the leaf grid
- **THEN** the save button is in view
- **AND** the browser sends no request

### Requirement: Empty state of a filtered recipe list
A recipe list is filtered when `q`, `machine`, or `status` holds a non-empty value, or when `show_deleted` is on. When a filtered list has no matching row, the page SHALL show the sentence of `recipes-empty-filtered` in place of `recipes-empty`. The page SHALL also show a link to `/recipes` without a query, outside the empty table row. A list that is not filtered SHALL show `recipes-empty` and no such link (R24, R39).

#### Scenario: No match for a search text
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100`
- **WHEN** `mhoffman` sends `GET /recipes?q=ZZZ`
- **THEN** the page shows the sentence of `recipes-empty-filtered`
- **AND** the page holds a link to `/recipes`

#### Scenario: No recipe without a filter
- **GIVEN** the Operator `mhoffman` with an open session and no recipe
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the page shows the sentence of `recipes-empty`

#### Scenario: Submit of the filter form with empty values
- **GIVEN** the Operator `mhoffman` with an open session and no recipe
- **WHEN** `mhoffman` sends `GET /recipes?q=&machine=&status=`
- **THEN** the page shows the sentence of `recipes-empty`
- **AND** the page holds no link to `/recipes` in the empty state

### Requirement: Word of an empty text value
The key `parameters-value-empty` SHALL name an empty text, not a missing value. The key SHALL give `Pusty tekst` in `pl` and `Empty text` in `en` (R39, A43).

#### Scenario: Empty string in the parameter table
- **GIVEN** the Operator `mhoffman` with the UI language `en` and the approved version 2 of `R-100` with the parameter `NOTE : STRING(254)` and an empty value
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the row of `NOTE` shows `Empty text` in the column Value

### Requirement: Recipe link of a machine page
The page of a live machine SHALL show a link to `/recipes?machine={code}`. The page of an archived machine SHALL NOT show this link (R05, R24, R39).

#### Scenario: Live machine
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the page holds a link to `/recipes?machine=M7`

#### Scenario: Archived machine
- **GIVEN** the Administrator `akowal` with an open session and the archived machine `M8`
- **WHEN** `akowal` opens the page of `M8`
- **THEN** the page holds no link to `/recipes?machine=M8`

### Requirement: Pager of the user list
The user list SHALL use the page pager of `design-system.md` section 10.12. The pager SHALL be a `nav` element with the accessible name of `pager-label` and SHALL show the number of the current page. A page after the last page SHALL show the notice `list-page-after-end` and a link to the first page (R39, A44).

#### Scenario: Second page of users
- **GIVEN** the Administrator `akowal` with an open session and 150 users
- **WHEN** `akowal` sends `GET /users?page=2`
- **THEN** the page holds a `nav` element with the class `pager` and a link to the first page
- **AND** the pager shows the number 2 with `aria-current="page"`

#### Scenario: Page after the last page
- **GIVEN** the Administrator `akowal` with an open session and 150 users
- **WHEN** `akowal` sends `GET /users?page=99`
- **THEN** the page shows the notice `list-page-after-end`
- **AND** the page holds a link to the first page

### Requirement: Role select of the user forms
The role select of the set-role form and of the new-user form SHALL use the select field of `design-system.md` section 10.5. The field SHALL have its label, the required mark, and the error line wiring of that section. A role outside the list SHALL give `422` and an answer that shows the error line under the select, with `aria-invalid="true"` and `aria-describedby` on the select (R28, A44).

#### Scenario: New-user form
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/new`
- **THEN** the select `role` has a `label` and `aria-required="true"`

#### Scenario: Set-role form
- **GIVEN** the Administrator `akowal` with an open session and the user `mhoffman`
- **WHEN** `akowal` sends `GET /users/mhoffman`
- **THEN** the select `role` has a `label` and `aria-required="true"`
- **AND** the option of the current role of `mhoffman` is selected

#### Scenario: Role outside the list
- **GIVEN** the Administrator `akowal` with an open session and the user `mhoffman`
- **WHEN** `akowal` sends `POST /users/mhoffman/role` with a role that is not in the list
- **THEN** the answer has the status `422`
- **AND** the select `role` has `aria-invalid="true"` and `aria-describedby="role-error"`
- **AND** the element `role-error` holds the error code and one sentence

### Requirement: Focus on the login page
`GET /login` SHALL render the login input with the attribute `autofocus`. A login page with field errors SHALL NOT set `autofocus` on an input (R39, A44).

#### Scenario: Empty login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the login input has the attribute `autofocus`

#### Scenario: Login page after a failed login
- **GIVEN** a browser without a session
- **WHEN** a user sends `POST /login` without a password and without the header `HX-Request`
- **THEN** no input of the page has the attribute `autofocus`

### Requirement: State words of the type list
The state column of the type list SHALL show the word of `type-latest` for the latest revision of a struct. Every other revision SHALL show the word of `type-superseded`. Each word SHALL start with a capital letter (R13, R19).

#### Scenario: Two revisions of one struct
- **GIVEN** the Operator `mhoffman` with the UI language `en` and the struct `ST_Zone` with the revisions 1 and 2 on the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7/types`
- **THEN** the row `ST_Zone@2` shows `Latest`
- **AND** the row `ST_Zone@1` shows `Superseded`

### Requirement: Empty state of a unit page
A unit page without a live sub-unit SHALL show the sentence of `unit-children-empty` in the empty state element of `design-system.md` section 10.10 (R06, R39).

#### Scenario: Unit without sub-units
- **GIVEN** the Operator `mhoffman` with an open session and the unit `U1.2` of the machine `M7` with no sub-unit
- **WHEN** `mhoffman` sends `GET /machines/M7/units/U1.2`
- **THEN** the page shows the sentence of `unit-children-empty` in an element with the class `empty`

### Requirement: Warning of a delete disclosure
The delete disclosure of a parameter SHALL give its warning the id `danger-notice-parameter`. The delete disclosure of a translation SHALL give its warning the id `danger-notice-translation-` followed by the language code of the row. The delete button SHALL name that id in `aria-describedby` (R10, R39, A44, ADR-0110).

#### Scenario: Parameter page with two translations
- **GIVEN** the ProcessEngineer `anowak` and the parameter `SPEED` of the draft version 3 of `R-100` with the translations `de` and `fr`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the delete buttons name `danger-notice-parameter`, `danger-notice-translation-de`, and `danger-notice-translation-fr` in `aria-describedby`
- **AND** the page holds each of these ids once

### Requirement: Polish terms for a draft and a tag
The bundle `pl` SHALL name the state Draft `Wersja robocza` and SHALL name a parameter tag `Tag`. No value of `pl` SHALL hold a form of the word `szkic` or the word `znacznik`. The bundle `en` SHALL stay as it is (R10, A17, A44).

#### Scenario: Words of the Polish bundles
- **GIVEN** the bundles `pl` and `pl-approval`
- **WHEN** the server loads them
- **THEN** no value holds `szkic` or `znacznik` in any letter case
- **AND** the keys `field-tag`, `parameters-col-tag`, and `diff-field-tag` give `Tag`

#### Scenario: Field error for a tag
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`
- **WHEN** `anowak` sends a new parameter with an invalid tag
- **THEN** the error line names the field `Tag`

## MODIFIED Requirements

### Requirement: State shown by color, icon, and word
A version state, a user state, and the state "Archived" of a machine or a unit SHALL show a color, an icon, and a word. Each error, warning, information, and success message SHALL also show a color, an icon, and a word. No element SHALL show a state by its color alone (R39, A44).

#### Scenario: State of a draft version
- **GIVEN** the draft version 3 of the recipe `R-100` and the UI language `pl`
- **WHEN** a page shows the state of version 3
- **THEN** the page shows the icon `status-draft` and the word `Wersja robocza` next to it

#### Scenario: State of an inactive user
- **GIVEN** the Administrator `akowal` with an open session and the inactive user `anowak`
- **WHEN** `akowal` sends `GET /users` with the UI language `en`
- **THEN** the row of `anowak` shows the icon `user-inactive` and the word `Inactive`

#### Scenario: State of an archived unit
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the archived machine `M12` with the archived unit `U2`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M12`
- **THEN** the node of `U2` shows the icon `status-retired` and the word `Archived`

#### Scenario: Error fragment as one alert
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends an HTMX request for `GET /users` and gets `403 E-WEB-002`
- **THEN** the fragment is one element with `role="alert"`
- **AND** that element holds the icon `alert-error`, the code `E-WEB-002`, one sentence, and the reference

### Requirement: Texts only from the bundles
A template SHALL hold no literal word. Every UI text SHALL come from the `fluent` bundles `pl` and `en`. Each message key that a template uses SHALL exist in both bundles (R10, R40, A17).

#### Scenario: Template without a literal word
- **GIVEN** every template of `recipe-web`
- **WHEN** the text test strips the askama tags and the markup
- **THEN** no letter remains
- **AND** the attributes `aria-label`, `title`, `placeholder`, and `alt` hold no letter outside an askama expression

#### Scenario: Status word in both languages
- **GIVEN** the bundles `pl` and `en`
- **WHEN** the server loads them
- **THEN** the key `status-draft` gives `Wersja robocza` in `pl` and `Draft` in `en`

### Requirement: Notice of a danger action
The notice of a danger action SHALL have an id that is unique in the page and no role. The danger button SHALL name its notice in `aria-describedby`. A page MAY hold more than one danger action. The id `danger-notice` SHALL occur at most once in a page. Each other notice id SHALL start with `danger-notice-` and end with a stable suffix (R39, A44, ADR-0049, ADR-0110).

#### Scenario: Notice of the retire action
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the page holds one element with the id `danger-notice`
- **AND** the retire button has `aria-describedby="danger-notice"`

#### Scenario: Two danger actions on one page
- **GIVEN** the ProcessEngineer `anowak` and the parameter `SPEED` of the draft version 3 of `R-100` with the stored translation `de`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the page holds one element with the id `danger-notice-parameter` and one element with the id `danger-notice-translation-de`
- **AND** the parameter delete button has `aria-describedby="danger-notice-parameter"`
- **AND** the translation delete button has `aria-describedby="danger-notice-translation-de"`
- **AND** every `id` of the page occurs once
