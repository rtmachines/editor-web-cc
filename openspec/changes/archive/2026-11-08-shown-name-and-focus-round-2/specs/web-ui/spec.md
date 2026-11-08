## MODIFIED Requirements

### Requirement: Translation fallback
A parameter name or description that a page shows without a label SHALL show the translation for the UI language when a translation exists. Otherwise the page SHALL show the English base text with the marker `(en)` (R10, A17). With the UI language `en`, the page SHALL show the English base text without the marker (ADR-0126). A labeled field of the English base text SHALL show the English base text in each UI language (ADR-0126 (f)).

#### Scenario: Missing Polish translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the page shows `Line speed (en)`

#### Scenario: Stored Polish translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and the stored `pl` translation with the name `Prędkość linii`
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the row of `SPEED` shows `Prędkość linii` without the marker `(en)`
- **AND** the row of `SPEED` does not show `Line speed`

#### Scenario: English UI language
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and the stored `pl` translation with the name `Prędkość linii`
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `en`
- **THEN** the row of `SPEED` shows `Line speed` without the marker `(en)`

#### Scenario: Row of a new parameter
- **GIVEN** a live `Draft`, a user with `ParameterEdit`, and the cookie `recipe_lang` set to `pl`
- **WHEN** the user adds the parameter `FEED` with the English name `Feed rate` through an HTMX request
- **THEN** the row fragment shows `Feed rate (en)`

#### Scenario: Added parameter with a Polish translation in the diff
- **GIVEN** the retired version 2 and the approved version 3 of `R-100`
- **AND** version 3 adds the parameter `FEED` with the English name `Feed rate` and the stored `pl` translation with the name `Posuw`
- **WHEN** the Operator `mhoffman` sends `GET /recipes/R-100/versions/3/diff/2` with the cookie `recipe_lang` set to `pl`
- **THEN** the row of `FEED` in the added section shows `Posuw` without the marker `(en)`
- **AND** the row of `FEED` in the added section does not show `Feed rate`

#### Scenario: Added parameter without a translation in the approval preview
- **GIVEN** the approved version 2 and the draft version 3 of `R-100`
- **AND** the draft adds the parameter `FEED` with the English name `Feed rate` and no `pl` translation
- **WHEN** the Administrator `admin` sends `GET /recipes/R-100/versions/3/approve` with the cookie `recipe_lang` set to `pl`
- **THEN** the row of `FEED` in the added section shows `Feed rate (en)`
- **AND** the element that holds `Feed rate` has the attribute `lang="en"`
- **AND** the marker `(en)` stands outside that element

#### Scenario: Removed parameter on an English page
- **GIVEN** the retired version 2 of `R-100` with the parameter `SPEED`, the English name `Line speed`, and the stored `pl` translation with the name `Prędkość linii`
- **AND** the approved version 3 of `R-100` without `SPEED`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3/diff/2` with the cookie `recipe_lang` set to `en`
- **THEN** the row of `SPEED` in the removed section shows `Line speed` without the marker `(en)`
- **AND** the cell of that name has no `lang` attribute

### Requirement: Language tag of the English base text
The English base text in a page of another language SHALL carry the language tag `en` (R39, A44).

#### Scenario: Language tag of the English base text
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the element that holds `Line speed` has the attribute `lang="en"`
- **AND** the marker `(en)` stands outside that element

#### Scenario: English base text inputs of the parameter form
- **GIVEN** the live draft version 3 of `R-100` with the parameter `SPEED` and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` opens the edit form of `SPEED` with the cookie `recipe_lang` set to `pl`
- **THEN** the input `name_en` has the attribute `lang="en"`
- **AND** the text area `description_en` has the attribute `lang="en"`
- **AND** the labels of the two fields carry no `lang` attribute

## ADDED Requirements

### Requirement: Language switch after a pushed address
An HTMX content answer to a `POST` that pushes a path through `HX-Push-Url` SHALL also set the field `next` out of band. The field `next` of the language switch SHALL hold the pushed path. The approval, the retire, and the removal of a parameter SHALL follow this rule (R10, R39, A17, ADR-0049 (e)).

#### Scenario: Language switch after an HTMX approval
- **GIVEN** the draft version 2 of `R-500` and the Administrator `admin` with an open session
- **WHEN** `admin` sends an HTMX approval of version 2 with its `row_version` and `HX-Target: content`
- **THEN** the answer sets `HX-Push-Url` to `/recipes/R-500/versions/2`
- **AND** the answer holds one field `next` with the value `/recipes/R-500/versions/2` and `hx-swap-oob="true"`

#### Scenario: Language switch after an HTMX parameter removal
- **GIVEN** the live draft version 3 of `R-100` with the parameter `SPEED` and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` removes `SPEED` through an HTMX request with `HX-Target: content`
- **THEN** the answer sets `HX-Push-Url` to `/recipes/R-100/versions/3`
- **AND** the answer holds one field `next` with the value `/recipes/R-100/versions/3` and `hx-swap-oob="true"`

#### Scenario: Approval without HTMX
- **GIVEN** the draft version 2 of `R-500` and the Administrator `admin` with an open session
- **WHEN** `admin` sends the approval of version 2 without the header `HX-Request`
- **THEN** the server returns the full version page
- **AND** the page holds no language switch

### Requirement: Submit button of an out-of-band form
Each form whose start tag carries the attribute `hx-swap-oob` SHALL give each of its submit buttons a fixed `id`. Then HTMX keeps the focus on the activated button after the swap (R39, A44, ADR-0041 (e)).

#### Scenario: Refused change of the own password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a new password of 3 characters
- **THEN** the server returns `422` and swaps `#password-form` out of band
- **AND** the submit button of `#password-form` has the attribute `id="password-submit"`

#### Scenario: Every out-of-band form of the templates
- **GIVEN** the templates of `recipe-web`
- **WHEN** the template test reads each form whose start tag has the attribute `hx-swap-oob`
- **THEN** each submit button of that form has a fixed `id`
- **AND** each submit button of a macro that the form calls has a fixed `id`
