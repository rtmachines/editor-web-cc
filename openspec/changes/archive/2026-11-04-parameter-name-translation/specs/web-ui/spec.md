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

## ADDED Requirements

### Requirement: Language tag of the English base text
The English base text in a page of another language SHALL carry the language tag `en` (R39, A44).

#### Scenario: Language tag of the English base text
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the element that holds `Line speed` has the attribute `lang="en"`
- **AND** the marker `(en)` stands outside that element
