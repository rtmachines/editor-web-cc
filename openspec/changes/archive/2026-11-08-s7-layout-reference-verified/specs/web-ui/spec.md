## MODIFIED Requirements

### Requirement: Notice of an approved version that is not loadable
The version page of an Approved version SHALL check every value, limit, and S7 address with the current rules. The address check SHALL include the overlap check of I20. When a part breaks a rule, the export area SHALL show a notice with the code of each broken rule. The export link SHALL name the notice in `aria-describedby` (R12, R28, R39, ADR-0149).

#### Scenario: Approved version with an affected value
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in `DUE`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows a notice with `E-RCP-005`
- **AND** the export link names the notice in `aria-describedby`

#### Scenario: Approved version without an affected part
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED : INT` and the value `1200`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows no notice
- **AND** the export link has no `aria-describedby`

#### Scenario: Approved version with an overlap after a change of the layout rules
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored parameters `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`, approved before ADR-0147
- **AND** `ST_Bytes@1` has the members `a` of type `BYTE`, `b` of type `CHAR`, `c` of type `SINT`, and `d` of type `USINT`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows a notice with `E-RCP-022`
- **AND** the export link names the notice in `aria-describedby`
