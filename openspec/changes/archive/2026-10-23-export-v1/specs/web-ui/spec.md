## ADDED Requirements

### Requirement: Redirects on the export routes
An export request without a valid session SHALL get `303` to `/login` and SHALL end the session cookie. An export request of a session with `must_change_password` SHALL get `303` to `/me/password`. An HTMX header SHALL NOT change these answers (R28, R39).

#### Scenario: Expired session
- **GIVEN** the Operator `mhoffman` with an expired session
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the server returns `303` with the location `/login`

#### Scenario: Forced password change
- **GIVEN** the ProcessEngineer `anowak` with a session that has `must_change_password`
- **WHEN** `anowak` sends `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the server returns `303` with the location `/me/password`

### Requirement: Export link of a version page
The page of an `Approved` version SHALL show a link to `/export/v1/recipes/{ref}/approved` to each role with `VersionExport`. The page of a `Retired` version SHALL show a link to `/export/v1/recipes/{ref}/versions/{n}` only to a role with `VersionExportHistory`, with a notice on the prefix `RETIRED_`. A `Draft`, a discarded draft, and a version of a deleted recipe SHALL show no export link. The link SHALL stay in read-only mode (R24, R28, A24).

#### Scenario: Operator reads an approved version
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows a link to `/export/v1/recipes/R-100/approved`

#### Scenario: Operator reads a retired version
- **GIVEN** the Operator `mhoffman` with an open session and the retired version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows no export link

#### Scenario: ProcessEngineer reads a retired version
- **GIVEN** the ProcessEngineer `anowak` with an open session and the retired version 2 of `R-100`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows a link to `/export/v1/recipes/R-100/versions/2`
- **AND** the link names the notice on the prefix `RETIRED_` in `aria-describedby`

#### Scenario: Draft version
- **GIVEN** the Administrator `admin` with an open session and the draft version 4 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/4`
- **THEN** the page shows no export link

#### Scenario: Discarded draft
- **GIVEN** the Administrator `admin` with an open session and the discarded draft version 4 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/4`
- **THEN** the page shows no export link

#### Scenario: Read-only mode
- **GIVEN** the application in read-only mode, the Operator `mhoffman` with an open session, and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows a link to `/export/v1/recipes/R-100/approved`

#### Scenario: Version of a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the retired version 2 of the deleted recipe `R-700`
- **WHEN** `admin` sends `GET /recipes/R-700/versions/2`
- **THEN** the page shows no export link

### Requirement: Export link without HTMX
The export link SHALL be a plain link with the literal `hx-boost="false"`. No ancestor of the link SHALL have `hx-boost="true"`. The link SHALL have no `download` attribute and no `target` attribute. Its visible text SHALL name the version and the file type (R39, A44).

#### Scenario: Attributes of the export link
- **GIVEN** the ProcessEngineer `anowak` with an open session in the UI language `pl` and the approved version 3 of `R-100`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the export link has `hx-boost="false"` and no `download` and no `target` attribute
- **AND** no ancestor of the link has `hx-boost="true"`
- **AND** the link text is `Eksportuj wersję 3 do pliku JSON`

### Requirement: Navigation guard of the export routes
A request on an export route with a `Sec-Fetch-Site` header other than `same-origin` or `none`, or with `HX-Request: true`, SHALL get `403` with the code `E-WEB-007` as a JSON body. The system SHALL write one `csrf_rejected` security event, SHALL NOT run `ExportVersion`, and SHALL write no `VersionExported` entry. A request without `Sec-Fetch-Site` SHALL pass the guard (R20, A8).

#### Scenario: Same-site request
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** a page of a sibling host sends `GET /export/v1/recipes/R-100/approved` with `Sec-Fetch-Site: same-site` and the session cookie of `mhoffman`
- **THEN** the server returns `403` with a JSON error body and the code `E-WEB-007`
- **AND** the system writes one `csrf_rejected` security event
- **AND** the audit log holds no new `VersionExported` entry

#### Scenario: Bookmark
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` opens `GET /export/v1/recipes/R-100/approved` from a bookmark with `Sec-Fetch-Site: none`
- **THEN** the server returns `200` with the export document

#### Scenario: Request without a site header
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved` without a `Sec-Fetch-Site` header
- **THEN** the server returns `200` with the export document

## MODIFIED Requirements

### Requirement: JSON error on export routes
Every answer of an export route with an error code SHALL be a JSON body. Its fields SHALL be `code`, `message`, and `request_id` in this order. `message` SHALL be the sentence of the code in the UI language. The answer SHALL hold no `HX-*` header (R10, R36).

#### Scenario: Recipe without an approved version
- **GIVEN** the recipe `R-100` with no approved version
- **WHEN** a user sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the server returns `404` with a JSON error body
- **AND** the field `code` of the body holds `E-EXP-001`

#### Scenario: Message in the UI language
- **GIVEN** the Operator `mhoffman` with the UI language `pl` and the recipe `R-100` with no approved version
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the field `message` of the body holds the Polish sentence of `E-EXP-001`

#### Scenario: Operator on the versions route
- **GIVEN** the Operator `mhoffman` with an open session and the retired version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the server returns `403` with a JSON error body and the code `E-WEB-002`
- **AND** the system writes one `forbidden` security event

#### Scenario: HTMX header on an export route
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100` with no approved version
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved` with the header `HX-Request: true`
- **THEN** the server returns `403` with a JSON error body and the code `E-WEB-007`
- **AND** the answer has no header `HX-Retarget`, `HX-Reswap`, or `HX-Push-Url`
- **AND** the system writes one `csrf_rejected` security event
