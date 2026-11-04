# Review: PLC data types on S7-300, S7-1200, and S7-1500

Date: 2026-11-03. Reviewer role: Siemens S7 domain reviewer. This review is read-only. It changes no code and no document.

Result: 15 findings. 1 CRITICAL, 5 HIGH, 5 MEDIUM, 4 LOW. Six findings (F01 to F06) are conflicts between a normative document and a Siemens source. Section 4.1 lists them as open questions for the owner.

## 1. Scope and sources

### 1.1 Scope

- Platforms: `S7_300`, `S7_1200`, `S7_1500` of `PlcPlatform`.
- Out of scope: `CodesysIec`, `TwinCat3`, PLC communication, and the IEC derived types of A39.
- Section 5 lists the types that a recipe cannot hold.

### 1.2 Normative documents read

| Document | Sections |
|---|---|
| `docs/architecture/data-model.md` | §7 (primitive matrix and notes), §9.1 (JSONB example) |
| `docs/architecture/domain-model.md` | §2.1 (`PlcType`, `StringLength`, `TypeExpr`, `S7Address`, `S7Layout`, `LeafClass`, base units, `IecLiteral`), §3 (I6) |
| `docs/architecture/overview.md` | §9 rows A3, A31, A36, A38, A39, A43 |
| `docs/architecture/reliability.md` | §4.3, §4.4, §6 |
| `docs/architecture/api.md` | §6.1, §6.3 |
| ADRs | ADR-0006, ADR-0017, ADR-0022 (A31 row), ADR-0024, ADR-0026, ADR-0028, ADR-0045. The decision map of `docs/adr/README.md` marks none of them as superseded |

### 1.3 Code read

| Layer | Files |
|---|---|
| Seed | `migrations/0004_seed_plc_types.sql`. No later migration changes `plc_type` rows |
| Plant model | `crates/recipe-domain/src/plant/primitive.rs`, `platform.rs`, `string_length.rs`, `array_dimension.rs`, `s7_address.rs`, `s7_width.rs`, `s7_layout.rs`, `iec_literal/range.rs`, `iec_literal/canonical.rs`, `iec_literal/instant.rs`, `iec_literal/duration.rs` |
| Value | `crates/recipe-domain/src/recipe/value/leaf.rs`, `numeric.rs`, `decimal.rs`, `texts.rs`, `check.rs`, `crates/recipe-domain/src/recipe/limit/parse.rs`, `crates/recipe-domain/src/recipe/draft/approval.rs`, `crates/recipe-domain/src/recipe/draft/s7.rs` |
| Storage | `Cargo.toml` (`serde_json` with `arbitrary_precision`), `crates/recipe-db/tests/storage/seed.rs` |
| Web | `crates/recipe-web/templates/components/grid.html`, `crates/recipe-web/src/pages/parameters/grid/walk.rs` |
| Export | `crates/recipe-export/schema/export-v1.schema.json`, `crates/recipe-export/tests/golden.rs`, `crates/recipe-export/tests/schema.rs` |

### 1.4 Siemens sources

| Id | Source |
|---|---|
| S1 | Siemens, "SIMATIC S7-300/S7-1200/S7-1200 G2/S7-1500 Comparison list for programming languages", Reference manual, Edition 02/2025, A5E33285102-AK, SIOS entry ID 109986777, section "Overview of data types", pages 9 to 14. <https://cache.industry.siemens.com/dl/files/777/109986777/att_1322640/v1/s7_1500_compare_table_en.pdf> |
| S2 | Siemens, "Programming Guideline for S7-1200/1500", SIOS entry ID 81318674, V1.6, 12/2018. Sections 2.6 (Tables 2-5, 2-6), 2.7.1 (Table 2-7), 2.8.1 (Tables 2-9, 2-10), 2.8.2, 2.8.3 (Table 2-12). <https://cache.industry.siemens.com/dl/files/040/90885040/att_970576/v1/81318674_Programming_guideline_DOC_v16_en.pdf> |

Limits of the evidence:

- S1 marks the CPU families with icons. The text extraction lost the icons. The review uses S1 for sizes and ranges. It uses S1 for availability only where a text marker such as "1500" is next to the type.
- S2 states "only for S7-1500" for `LINT`, `ULINT`, and `LWORD`. It states "only S7-1500" for `LTIME` and `LTOD`.
- No Siemens source in reach states the even-byte rules of non-optimized blocks. This review marks those rules "unverified". It does not report them as wrong.

## 2. Coverage matrix

Columns 1 to 8 are the points of the review brief:

1. Availability.
2. Size and layout.
3. Address.
4. Range and literal grammar.
5. Time encoding.
6. Process limit.
7. Round trip.
8. Tests.

Cell values:

- `OK`: the code, the documents, and Siemens agree. For an absent type, `OK` in column 1 means that all three say "not available".
- `n/a`: the point does not apply.
- `Fnn`: a finding of section 3.
- `Qnn`: an unverified question of section 4.2.

Rules that hold for every row:

- Column 3. `S7Width::expected_for` gives the width of the matrix (`s7_width.rs:25`). `check_platform_rule` requires an address on `S7_300` only (`recipe/draft/s7.rs:72`). F04 applies to every addressed parameter. F05 applies to every `S7_300` row.
- Column 6. `needs_limit` requires a limit for `Numeric`, `String`, `Duration`, and `Instant` leaves (`recipe/draft/approval.rs:109`). `BOOL`, `CHAR`, and `WCHAR` get none (A36).
- Column 7. A form sends text. `leaf_json` builds JSON (`texts.rs:138`). `convert` writes canonical text (`leaf.rs:37`). JSONB keeps it with `arbitrary_precision`. The export writes each number and count as a JSON string (`export-v1.schema.json:145`). The review found no path that changes a value.
- Column 8. `recipe-db/tests/storage/seed.rs:324` compares every seed row with the matrix. This test covers the platform rejection of each absent primitive.

### 2.1 Bit strings and integers

| Type | Platform | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| `BOOL` | S7_300 | OK | OK | F04 F05 | OK | n/a | OK (none) | OK | OK |
| `BOOL` | S7_1200 | OK | OK | F04 | OK | n/a | OK (none) | OK | OK |
| `BOOL` | S7_1500 | OK | OK | F04 | OK | n/a | OK (none) | OK | OK |
| `BYTE` | S7_300 | OK | OK | F04 F05 | F12 | n/a | OK | OK | F10 |
| `BYTE` | S7_1200 | OK | OK | F04 | F12 | n/a | OK | OK | F10 |
| `BYTE` | S7_1500 | OK | OK | F04 | F12 | n/a | OK | OK | F10 |
| `WORD` | S7_300 | OK | OK | F04 F05 | F12 | n/a | OK | OK | F10 |
| `WORD` | S7_1200 | OK | OK | F04 | F12 | n/a | OK | OK | F10 |
| `WORD` | S7_1500 | OK | OK | F04 | F12 | n/a | OK | OK | F10 |
| `DWORD` | S7_300 | OK | OK | F04 F05 | F12 | n/a | OK | OK | F10 |
| `DWORD` | S7_1200 | OK | OK | F04 | F12 | n/a | OK | OK | F10 |
| `DWORD` | S7_1500 | OK | OK | F04 | F12 | n/a | OK | OK | F10 |
| `LWORD` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LWORD` | S7_1200 | OK, F14 | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LWORD` | S7_1500 | OK | OK | F04 | F12 | n/a | OK | OK | F10 F11 |
| `SINT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `SINT` | S7_1200 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `SINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `USINT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `USINT` | S7_1200 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `USINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `INT` | S7_300 | OK | OK | F04 F05 | OK | n/a | OK | OK | OK |
| `INT` | S7_1200 | OK | OK | F04 | OK | n/a | OK | OK | OK |
| `INT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | OK |
| `UINT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `UINT` | S7_1200 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `UINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `DINT` | S7_300 | OK | OK | F04 F05 | OK | n/a | OK | OK | F10 |
| `DINT` | S7_1200 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `DINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `UDINT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `UDINT` | S7_1200 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `UDINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 |
| `LINT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LINT` | S7_1200 | OK, F14 | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 F11 |
| `ULINT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `ULINT` | S7_1200 | OK, F14 | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `ULINT` | S7_1500 | OK | OK | F04 | OK | n/a | OK | OK | F10 F11 |

### 2.2 Floating point

| Type | Platform | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| `REAL` | S7_300 | OK | OK | F04 F05 | F06 F07 | n/a | OK | OK | OK |
| `REAL` | S7_1200 | OK | OK | F04 | F06 F07 | n/a | OK | OK | OK |
| `REAL` | S7_1500 | OK | OK | F04 | F06 F07 | n/a | OK | OK | OK |
| `LREAL` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LREAL` | S7_1200 | OK | OK | F04 | F06 | n/a | OK | OK | F10 |
| `LREAL` | S7_1500 | OK | OK | F04 | F06 | n/a | OK | OK | F10 |

### 2.3 Characters and strings

| Type | Platform | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| `CHAR` | S7_300 | OK | OK | F04 F05 | Q06 | n/a | OK (none) | OK | F10 |
| `CHAR` | S7_1200 | OK | OK | F04 | Q06 | n/a | OK (none) | OK | F10 |
| `CHAR` | S7_1500 | OK | OK | F04 | Q06 | n/a | OK (none) | OK | F10 |
| `WCHAR` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `WCHAR` | S7_1200 | OK | OK | F04 | OK | n/a | OK (none) | OK | F10 |
| `WCHAR` | S7_1500 | OK | OK | F04 | OK | n/a | OK (none) | OK | F10 |
| `STRING` (default 254) | S7_300 | OK | OK | F04 F05 | Q06 | n/a | OK | OK | OK |
| `STRING` (default 254) | S7_1200 | OK | OK | F04 | Q06 | n/a | OK | OK | OK |
| `STRING` (default 254) | S7_1500 | OK | OK | F04 | Q06 | n/a | OK | OK | OK |
| `STRING(n)` | S7_300 | OK | OK, F09 | F04 F05 | OK, Q06 | n/a | OK | OK | OK |
| `STRING(n)` | S7_1200 | OK | OK, F09 | F04 | OK, Q06 | n/a | OK | OK | OK |
| `STRING(n)` | S7_1500 | OK | OK, F09 | F04 | OK, Q06 | n/a | OK | OK | OK |
| `WSTRING` (default 254) | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `WSTRING` (default 254) | S7_1200 | OK | OK | F04 | Q07 | n/a | OK | OK | F10 |
| `WSTRING` (default 254) | S7_1500 | OK | OK | F04 | Q07 | n/a | OK | OK | F10 |
| `WSTRING(n)` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `WSTRING(n)` | S7_1200 | OK | F09 | F04 | OK, Q07 | n/a | OK | OK | F09 F10 |
| `WSTRING(n)` | S7_1500 | OK | F09 | F04 | OK, Q07 | n/a | OK | OK | F09 F10 |

### 2.4 Time and date

| Type | Platform | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| `S5TIME` | S7_300 | OK | OK | F04 F05 | OK | OK | OK | OK | F10 |
| `S5TIME` | S7_1200 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `S5TIME` | S7_1500 | OK | OK | F04 | OK | OK | OK | OK | F10 |
| `TIME` | S7_300 | OK | OK | F04 F05 | OK | OK | OK | OK | OK |
| `TIME` | S7_1200 | OK | OK | F04 | OK | OK | OK | OK | OK |
| `TIME` | S7_1500 | OK | OK | F04 | OK | OK | OK | OK | OK |
| `LTIME` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LTIME` | S7_1200 | OK, F14 | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LTIME` | S7_1500 | OK | OK | F04 | OK | OK | OK | OK | F10 F11 |
| `DATE` | S7_300 | OK | OK | F04 F05 | OK | OK | OK | OK | F10 |
| `DATE` | S7_1200 | OK | OK | F04 | OK | F01 | OK | OK | F10 |
| `DATE` | S7_1500 | OK | OK | F04 | OK | F01 | OK | OK | F10 |
| `LDATE` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LDATE` | S7_1200 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LDATE` | S7_1500 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `TIME_OF_DAY` (TOD) | S7_300 | OK | OK | F04 F05 | OK | OK | OK | OK | F10 |
| `TIME_OF_DAY` (TOD) | S7_1200 | OK | OK | F04 | OK | OK | OK | OK | F10 |
| `TIME_OF_DAY` (TOD) | S7_1500 | OK | OK | F04 | OK | OK | OK | OK | F10 |
| `LTOD` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LTOD` | S7_1200 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LTOD` | S7_1500 | OK | OK | F04 | OK | OK | OK | OK | F10 |
| `DATE_AND_TIME` (DT) | S7_300 | OK | OK | F04 F05 | F02 | F02 | OK | OK | F10 |
| `DATE_AND_TIME` (DT) | S7_1200 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `DATE_AND_TIME` (DT) | S7_1500 | OK | OK | F04 | F02 | F02 | OK | OK | F10 |
| `LDT` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LDT` | S7_1200 | OK (unverified) | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `LDT` | S7_1500 | OK | OK | F04 | OK | OK | OK | OK | F10 F11 |
| `DTL` | S7_300 | OK | n/a | n/a | n/a | n/a | n/a | n/a | OK |
| `DTL` | S7_1200 | OK | OK | F04 | OK | F03 | OK | OK | F10 |
| `DTL` | S7_1500 | OK | OK | F04 | OK | F03 | OK | OK | F10 |

Notes on the time rows:

- `S5TIME`: `check_s5time_step` rejects a count between two steps and a count above 9 990 000 ms (`iec_literal/canonical.rs:34`). This matches S1 ("0 ms ... 2 h 46 m 30 s 0 ms").
- `TIME`, `LTIME`, `TIME_OF_DAY`, `LTOD`, and `LDT` on S7: the ranges of `iec_literal/range.rs:11` to `:40` equal S1.
- `range.rs` tests both ends and one step outside each end, per platform. `assert_absent` tests each absent cell (`range.rs:148`).

### 2.5 Composite types

| Type | Platform | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| `ARRAY` (1 to 3 dims, A38) | S7_300 | OK | OK, F09 | F04 F05 | Q08 | n/a | OK (`[*]`) | OK | F08 F09 |
| `ARRAY` (1 to 3 dims, A38) | S7_1200 | OK | OK, F09 | F04 | OK | n/a | OK (`[*]`) | OK | F08 F09 |
| `ARRAY` (1 to 3 dims, A38) | S7_1500 | OK | OK, F09 | F04 | OK | n/a | OK (`[*]`) | OK | F08 F09 |
| `ARRAY OF BOOL` | S7_300 | OK | OK, F09 | F04 F05 | OK | n/a | OK (none) | OK | F09 |
| `ARRAY OF BOOL` | S7_1200 | OK | OK, F09 | F04 | OK | n/a | OK (none) | OK | F09 |
| `ARRAY OF BOOL` | S7_1500 | OK | OK, F09 | F04 | OK | n/a | OK (none) | OK | F09 |
| UDT / `STRUCT` | S7_300 | OK | OK | F04 F05 | OK | n/a | OK per leaf | OK | F08 |
| UDT / `STRUCT` | S7_1200 | OK | OK | F04 | OK | n/a | OK per leaf | OK | F08 |
| UDT / `STRUCT` | S7_1500 | OK | OK | F04 | OK | n/a | OK per leaf | OK | F08 |
| Nested array in a group (ADR-0045) | S7_300 | Q09 | F09 | F04 F05 | OK | n/a | OK | OK | F09 |
| Nested array in a group (ADR-0045) | S7_1200 | Q09 | F09 | F04 | OK | n/a | OK | OK | F09 |
| Nested array in a group (ADR-0045) | S7_1500 | Q09 | F09 | F04 | OK | n/a | OK | OK | F09 |

Notes on the composite rows:

- ADR-0045 changes the page layout only. It does not change the type, the value, or the export.
- `S7Layout` packs `BOOL` members 8 per byte and starts every wider member, struct, and array on an even byte (`s7_layout.rs:232` to `:300`). This matches `domain-model.md` §2.1. Siemens confirmation is open (F08).

### 2.6 Types that the docs or the seed hold and the brief list lacks

None. The 29 primitives of `Primitive::ALL` (`primitive.rs`) equal the 29 rows of `data-model.md` §7. The seed has 31 rows because `STRING` and `WSTRING` have one extra row each for CODESYS and TwinCAT 3.

## 3. Findings

The findings are in the order of severity.

### F01 — CRITICAL — doc-vs-Siemens — `DATE` upper end on S7-1200 and S7-1500

- Evidence. ADR-0026 and the base-unit table of `domain-model.md` §2.1 give `DATE` on `S7_1200` and `S7_1500` the range 1990-01-01 to 2169-06-06. `iec_literal/range.rs:23` to `:24` holds `7_305..=72_840`.
- Siemens. S1 ("Overview of data types", page 12) gives one `DATE` range for every CPU family: "01.01.1990 ... 31.12.2168".
- Effect. The domain accepts the dates 2169-01-01 to 2169-06-06 on S7-1200 and S7-1500. When S1 is correct, such a value reaches the export outside the PLC range.
- Action. Open question Q01. When the owner confirms S1, a new ADR changes the row of ADR-0026. Then `range.rs:24` and its test follow.

### F02 — HIGH — doc-vs-Siemens — `DATE_AND_TIME` has no milliseconds

- Evidence. A43 and `domain-model.md` §2.1 count `DATE_AND_TIME` in seconds. The `IecLiteral` rules say "`DATE_AND_TIME` has no fraction". `iec_literal/instant.rs:119` sets the fraction limit 0.
- Siemens. S1 (page 12) gives the range "01.01.1990-0:0:0 ... 31.12.2089-23:59:59.999" and the example `DT#2008-10-25-8:12:34.567`. The type carries milliseconds.
- Effect. The domain rejects a valid S7 value with milliseconds, for example `DT#2026-10-12-12:30:00.500`. ADR-0026 also sets the upper end at 23:59:59, not 23:59:59.999.
- Action. Open question Q02. A change needs a new ADR, because it changes the base unit of A43 and the export unit of `api.md` §6.1.

### F03 — HIGH — doc-vs-Siemens — `DTL` upper end

- Evidence. `domain-model.md` §2.1 gives `DTL` the range 1970-01-01 to 2262-04-11. `iec_literal/range.rs:41` to `:42` holds `0..=9_223_372_036_854_775_807` ns.
- Siemens. S1 (page 12) gives `DTL` the range "01.01.1970-00:00:00.0 ... 31.12.2554-23:59:59.999999999".
- Effect. The domain rejects valid `DTL` values from 2262-04-12 to 2554-12-31. The stored decimal text can hold these counts (`reliability.md` §4.4). The practical effect is small.
- Action. Open question Q03. A change needs a new ADR and a new row of the base-unit table.

### F04 — HIGH — doc-vs-Siemens — an address can end after 64 KB

- Evidence. `domain-model.md` §2.1 limits the offset to `0..=65535`. No document limits the end of the parameter. `occupy` (`recipe/draft/s7.rs:119`) computes `offset + layout size` and checks only the overlap. `S7Address::parse` checks only the offset (`s7_address.rs:141`).
- Siemens. S2 §2.7.1 Table 2-7 gives a data block a maximum size of 64 kB on S7-300, S7-1200, and S7-1500. A non-optimized block has this limit. S2 §2.7.1 says only an optimized S7-1500 block can be larger, and an optimized block has no offset (S2 §2.6, Table 2-5).
- Effect. The domain accepts `DB1.DBB65534` for a `STRING(254)`. The parameter ends after the last byte of the data block. The export carries an address that the PLC cannot hold.
- Action. Open question Q04. A new ADR adds the rule "offset plus layout size is at most 65536". Then `check_tree_rules` (`recipe/draft/s7.rs:90`) and a negative test follow.

### F05 — HIGH — doc-vs-Siemens — data block number on S7-300

- Evidence. `domain-model.md` §2.1 allows `db` `1..=65535` on every platform. `parse_db` follows it (`s7_address.rs:132`).
- Siemens. S2 §2.7.1 Table 2-7 gives a maximum number of 16 000 data blocks for S7-300/400. The value comes from the text extraction of the table. The real limit also depends on the CPU.
- Effect. The domain accepts `DB20000.DBW0` on `S7_300`. No S7-300 CPU can hold this block, when the table value is correct.
- Action. Open question Q04. The owner decides on a platform limit or a configured limit per machine.

### F06 — HIGH — doc-vs-Siemens — `REAL` and `LREAL` range is the range of `Decimal`

- Evidence. `numeric.rs:25` gives `REAL` and `LREAL` the range `Decimal::MIN..=Decimal::MAX`, about ±7.9e28. `decimal.rs:6` and `:9` limit a value to 28 significant digits and scale 28. `reliability.md` §4.4 and ADR-0028 record this decision.
- Siemens. S1 (page 11) gives `REAL` the range ±1.175495e-38 to ±3.402823e+38 and `LREAL` the range ±2.2250738585072014e-308 to ±1.7976931348623158e+308.
- Effect. The domain rejects valid values above 7.9e28 and below 1e-28 in magnitude. A recipe rarely needs them, so the practical effect is small. The rubric of this review still classifies a rejected valid value as HIGH.
- Action. Open question Q05. The owner confirms the restriction in ADR-0028, or a new ADR widens it.

### F07 — MEDIUM — doc-vs-Siemens — `REAL` precision is not checked

- Evidence. `reliability.md` §4.4 checks the range of `REAL`, not its precision. `numeric` (`leaf.rs:129`) accepts up to 28 significant digits for a `REAL` leaf.
- Siemens. S2 §2.8.1 Table 2-10 gives `REAL` "precision 7 places after the comma" and `LREAL` 15 places.
- Effect. The export keeps the decimal text unchanged. The transfer client stores the nearest `REAL`, for example `16777217` becomes `16777216`. A value at a limit can then be outside the limit by one unit of the last place.
- Action. The owner decides if the domain rejects a decimal that `REAL` cannot hold exactly, or shows a warning. A decision needs a new ADR.

### F08 — MEDIUM — missing-test — no Siemens reference table for `S7Layout`

- Evidence. ADR-0017 requires a property test that compares the computed sizes with a reference table from the Siemens manual. The doc comment at `s7_layout.rs:28` to `:31` says the table "is not in the repository yet".
- Siemens. The review found no Siemens source in reach for the even-byte rules. The rules are unverified.
- Effect. Nothing proves that the layout of a struct or an array equals the layout of TIA Portal. A wrong layout gives an address overlap in the PLC (R14, R31).
- Action. Add the reference table from a TIA Portal export of non-optimized DBs. Add the property test of ADR-0017.

### F09 — MEDIUM — missing-test — layout cases are incomplete

- Evidence. The cases of `s7_layout.rs:400` to `:478` cover `BOOL`, `BYTE`, `INT`, `CHAR`, `DTL`, `LREAL`, `STRING(3)`, structs, a 1-D `BOOL` array, and a 2-D `INT` array.
- Missing cases:
  - A `WSTRING(n)` root and a `WSTRING(n)` member.
  - A 2-D and a 3-D `ARRAY OF BOOL`. The code packs all bits of all dimensions in one run (`s7_layout.rs:286`).
  - An array whose element is an array (`TypeExpr` allows it, `domain-model.md` §2.1).
  - A `STRING(n)` member with odd `n` before a `BYTE` member.
- Action. Add one golden case for each item. Take the expected sizes from TIA Portal (F08).

### F10 — MEDIUM — missing-test — export golden tests cover few types

- Evidence. `crates/recipe-export/tests/golden.rs` uses only `INT`, `REAL`, and `STRING`. The export tests of `recipe-domain/src/export` add `BOOL` and `TIME`.
- Missing types: `BYTE`, `WORD`, `DWORD`, `LWORD`, `SINT`, `USINT`, `UINT`, `DINT`, `UDINT`, `LINT`, `ULINT`, `LREAL`, `CHAR`, `WCHAR`, `WSTRING`, `S5TIME`, `LTIME`, `DATE`, `TIME_OF_DAY`, `LTOD`, `DATE_AND_TIME`, `LDT`, and `DTL`.
- Effect. No test proves the export text of these leaves, for example a `CHAR` above U+007F or a `WSTRING` with a surrogate pair.
- Action. Add one golden document per S7 platform with one leaf of each available type and its limit.

### F11 — MEDIUM — missing-test — no JSONB round trip of 64-bit extremes

- Evidence. The review found no `recipe-db` test that writes and reads a parameter value at the end of a 64-bit range.
- Values to test: `ULINT` and `LWORD` 18446744073709551615, `LINT` −9223372036854775808, `LTIME` −9223372036854775808 ns, `LDT` 9223372036854775807 ns.
- Effect. The workspace enables `arbitrary_precision` (`Cargo.toml:62`). A future change of a feature flag or of the `sqlx` decoder could round these values through `f64` without a failed test.
- Action. Add one `testcontainers` test that writes these values and compares the canonical text after the read.

### F12 — LOW — doc-vs-Siemens — bit strings accept decimal text only

- Evidence. `leaf_json` accepts a JSON number only (`texts.rs:138`). `numeric.rs:18` to `:24` gives `BYTE`, `WORD`, `DWORD`, and `LWORD` unsigned ranges only.
- Siemens. S1 (pages 9 and 10) writes bit strings as `2#0011_1010` or `16#6B0F`. It also gives each bit string a signed range, for example `WORD` "0 ... 65 535 or -32 768 ... 32 767".
- Effect. A user cannot type `16#FF` or `-1` for a `WORD`. Each bit pattern still has one decimal form, so no value is lost.
- Action. The owner decides if the form accepts the `16#` and `2#` forms. A change needs a new ADR, because ADR-0039 defines the leaf text.

### F13 — LOW — wording — some excluded types have no document

- Evidence. `data-model.md` §7 excludes `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, and `ANY` by name. A3 makes the matrix the closed list of types.
- No document names `VARIANT`, `BLOCK_FB`, `BLOCK_FC`, `BLOCK_DB`, `BLOCK_SDB`, `HW_*`, `IEC_TIMER`, `IEC_COUNTER`, `TIMER`, `COUNTER`, `DB_ANY`, or `REF_TO`.
- Effect. A3 excludes them, so no wrong data is possible. A reader can still ask if they are forgotten.
- Action. Add these names to the exclusion note of `data-model.md` §7 in the next change of that document.

### F14 — LOW — doc-vs-Siemens — S7-1200 G2 is not a platform

- Evidence. `PlcPlatform` has `S7_1200` only (`platform.rs`). `data-model.md` §7 gives `S7_1200` no `LWORD`, `LINT`, `ULINT`, or `LTIME`.
- Siemens. S1 (02/2025) adds the family "S7-1200 G2" as a separate column. S2 (12/2018) gives the 64-bit integer types "only for S7-1500". The review could not read the G2 column marks of S1.
- Effect. A machine with an S7-1200 G2 CPU uses `S7_1200` and cannot use the 64-bit types, when G2 supports them.
- Action. Open question for the owner, not urgent. A new platform needs a new ADR and a seed migration.

### F15 — LOW — code-vs-doc — the export schema has no pattern for `s7_address`

- Evidence. `export-v1.schema.json` defines `s7_address` as `"type": ["string", "null"]` with no pattern. `domain-model.md` §2.1 gives the grammar of `S7Address`.
- Effect. The schema accepts a malformed address. The serializer writes only parsed addresses, so the export holds no malformed address today.
- Action. Add the `S7Address` grammar as a `pattern` in a future schema version. A change of the v1 schema needs the rule of `api.md` §6.1.

## 4. Open questions for the owner

### 4.1 Conflicts between a normative document and a Siemens source

| Id | Finding | Document | Siemens | Question |
|---|---|---|---|---|
| Q01 | F01 | ADR-0026, `domain-model.md` §2.1: `DATE` on S7-1200 and S7-1500 ends at 2169-06-06 | S1 page 12: `DATE` ends at 31.12.2168 | Which end applies? The TIA Portal help for your CPU and firmware can decide |
| Q02 | F02 | A43, `domain-model.md` §2.1: `DATE_AND_TIME` counts seconds and has no fraction | S1 page 12: range ends at 23:59:59.999, example `DT#2008-10-25-8:12:34.567` | Must a recipe hold milliseconds of `DATE_AND_TIME`? |
| Q03 | F03 | `domain-model.md` §2.1: `DTL` ends at 2262-04-11 | S1 page 12: `DTL` ends at 31.12.2554 | Is the narrower range a decision or an error? |
| Q04 | F04, F05 | `domain-model.md` §2.1: offset `0..=65535`, `db` `1..=65535`, no end rule | S2 Table 2-7: DB size 64 kB, S7-300/400 at most 16 000 DBs | Add an end rule and a DB number limit per platform? |
| Q05 | F06 | `reliability.md` §4.4, ADR-0028: `REAL` range is the `Decimal` range | S1 page 11: IEEE 754 range | Keep the restriction? |

### 4.2 Questions without a Siemens source

The review could not find a Siemens source for these questions. Each claim is unverified.

| Id | Topic | Current behavior | Question |
|---|---|---|---|
| Q06 | Character set of `CHAR` and `STRING` | The domain accepts U+0000 to U+00FF and calls it Latin-1 (`leaf.rs:10`, ADR-0028). S1 says "ASCII character set" | Does the transfer client write U+0080 to U+00FF as one byte each? TIA Portal can use a Windows code page, where 0x80 is `€` (unverified) |
| Q07 | `WSTRING` and characters outside the Basic Multilingual Plane | The domain accepts a surrogate pair and counts it as 2 units (`leaf/tests.rs:59`) | Does the S7 `WSTRING` accept UTF-16 surrogate pairs, or UCS-2 only (unverified)? |
| Q08 | Array bounds on S7-300 | `ArrayDimension` accepts `i32` bounds on every platform (`array_dimension.rs:48`) | Does S7-300 limit an array index to the `INT` range −32768 to 32767 (unverified)? |
| Q09 | Array of an array | `domain-model.md` §2.1 allows "also another array" as an element | Does TIA Portal accept `ARRAY[..] OF ARRAY[..]` in a DB of each S7 family (unverified)? |
| Q10 | Even-byte rules of non-optimized DBs | `S7Layout` follows `domain-model.md` §2.1 | F08 needs a TIA Portal reference before these rules count as verified |

## 5. Excluded types

| Type | Document that excludes it |
|---|---|
| IEC `ENUM`, subrange, alias | A39, `domain-model.md` §2.1 rules of `PlcType`, `data-model.md` §7 notes |
| `POINTER`, `REFERENCE`, `ANY` | `data-model.md` §7 notes |
| `VARIANT` | No document by name. A3 excludes it, because the matrix is the closed list (F13) |
| `BLOCK_FB`, `BLOCK_FC`, `BLOCK_DB`, `BLOCK_SDB` | No document by name. A3 excludes them (F13) |
| `HW_*` hardware types | No document by name. A3 excludes them (F13) |
| `IEC_TIMER`, `IEC_COUNTER` instances, S7-300 `TIMER` and `COUNTER` | No document by name. A3 excludes them (F13) |
| `DB_ANY`, `REF_TO` | No document by name. A3 excludes them (F13) |
| `LDATE` on S7 | `domain-model.md` §2.1 base-unit table: "`LDATE` does not exist on S7 platforms". This is an availability row, not a scope exclusion |
