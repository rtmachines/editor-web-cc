## MODIFIED Requirements

### Requirement: Type usage
The query `GetTypeUsage` SHALL return each parameter whose type uses the struct revision, directly or through another type, once. It SHALL list the versions `Draft`, `Approved`, and `Retired` of the recipes that are not deleted, `page.type_usage` rows per page. A row SHALL name the recipe, the version, its state, the tag, the key of the parameter, and the type of the parameter (R19, A29).

#### Scenario: Use through another struct
- **GIVEN** the machine `M12` with the struct `ST_Line@1` that has a member of the type `ARRAY[1..4] OF ST_Zone@2`
- **AND** the draft version 3 of the recipe `R-100` with the parameter `LINE` of the type `ST_Line@1`
- **WHEN** the Operator `mhoffman` runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds the row `R-100`, version 3, `Draft`, `LINE`, `ST_Line@1`
- **AND** the row holds the key of the parameter `LINE`

#### Scenario: Two members of one struct
- **GIVEN** the struct `ST_Pair@1` with the members `a` and `b`, both of the type `ST_Zone@2`
- **AND** the draft version 1 of the recipe `R-400` with the parameter `PAIR` of the type `ST_Pair@1`
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds one row for the parameter `PAIR`

#### Scenario: Deleted recipe and discarded draft
- **GIVEN** the deleted recipe `R-200` with a parameter of the type `ST_Zone@2`
- **AND** a discarded draft of the recipe `R-300` with a parameter of the type `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds no row of `R-200` and no row of the discarded draft

#### Scenario: Revision without use
- **GIVEN** the struct `ST_Zone@1` that no parameter uses
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 1
- **THEN** the list is empty
