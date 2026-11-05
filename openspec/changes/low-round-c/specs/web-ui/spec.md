## ADDED Requirements

### Requirement: Field answer of the own password form
`POST /me/password` SHALL check the current password and the new password before it answers. Its answer SHALL list each bad field at once, with a line under each bad field. A wrong current password (`E-IDN-008`) SHALL give a line under the field `current_password` and SHALL mark that input with `aria-invalid="true"`. `POST /login` SHALL keep one line per answer (R36, A44).

#### Scenario: Two bad fields of the own password form
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a current password of 129 characters and a new password of 3 characters
- **THEN** the server returns `422` with the fragment `field-errors.html`
- **AND** the fragment holds one line for `current_password` and one line for `password`
- **AND** the line for `password` holds the code `E-IDN-006`

#### Scenario: New password on the breach list
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a wrong current password and a new password of the breach list
- **THEN** the server returns `422` with one line `E-IDN-006` under `password`
- **AND** the server writes no login attempt row

#### Scenario: Wrong current password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a wrong current password and a valid new password
- **THEN** the server returns `422 E-IDN-008`
- **AND** the line of `E-IDN-008` links to `#current_password`
- **AND** the input `current_password` has `aria-invalid="true"`

#### Scenario: Two empty fields of the login form
- **GIVEN** a browser with a valid pre-session token
- **WHEN** a user sends `POST /login` with an empty `login` and an empty `password`
- **THEN** the server rejects the request with `E-WEB-008`
- **AND** the answer holds one line
