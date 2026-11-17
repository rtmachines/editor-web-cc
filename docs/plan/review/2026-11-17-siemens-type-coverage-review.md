# Review: Siemens data types of the reference file against the recipe editor

Owner decisions of 2026-11-17: the OpenSpec change `siemens-type-rules` records them.

Date: 2026-11-17. Reviewer role: Siemens S7 domain reviewer and Rust reviewer. This review is read-only. It changes no code, no document, no migration, and not the Siemens file.

Result: 278 inventory rows. 31 rows are SUPPORTED, 247 rows are EXCLUDED-BY-DECISION, and 0 rows are GAP. 6 findings: 0 CRITICAL, 0 HIGH, 3 MEDIUM, 3 LOW. 12 owner questions. Four of them (OQ01 to OQ04) become CRITICAL when the owner accepts the Siemens claim, because the editor then accepts a value or a shape that the PLC type cannot hold.

Baseline of 2026-11-03: 10 items fixed, 1 obsolete (F03), 4 open (F12 to F15). Section 5 gives the detail.

## 1. Scope and sources

### 1.1 Scope

- Platforms: `S7_300`, `S7_1200`, `S7_1500` of `PlcPlatform`.
- Out of scope: `CodesysIec`, `TwinCat3`, PLC communication.
- Source under review: `sources/Siemens_PLC_Data_Types_Detailed_EN.md` (not normative, 1928 lines), §2 to §10 (lines 53 to 1820). §11 (lines 1821 to 1928) is the reference for layout (dimension c).
- The Siemens file is not normative. A conflict between it and a normative document is an owner question (section 4), not a code finding.

### 1.2 Method

Four Opus agents reviewed one section group each. Codex (`gpt-6-astra`, effort high) reviewed all of §2 to §10 in parallel as a second review. The orchestrator merged the results, removed duplicates, and checked the MEDIUM findings in the source.

| Group | Siemens sections | Rows | Baseline items | Reviewer |
|---|---|---|---|---|
| N: numeric and system types | §2 (lines 53 to 256), §8 (lines 995 to 1252) | 91 | F06, F07, F08, F11, F12, F14 | Opus |
| T: time, date, and fail-safe types | §3 (lines 257 to 401), §10 (lines 1699 to 1820) | 18 | F01, F02, F03 | Opus |
| S: characters, strings, pointers, parameters, instances | §4 (lines 402 to 491), §6 (lines 621 to 866), §7 (lines 867 to 994) | 20 | F04, F05, F15 | Opus |
| A: arrays, structures, UDTs, communication and technology types | §5 (lines 492 to 620), §9 (lines 1253 to 1698) | 149 | F09, F10, F13 | Opus |
| X: second review | §2 to §10 | 296 (finer split, same categories) | F01 to F15 | Codex `gpt-6-astra` high |

Codex split the §8, §4, and §10 rows more finely (100, 4, and 4 rows against 76, 5, and 9). It put each row in the same category as the Opus groups. The matrix of section 2 uses the Opus inventory.

### 1.3 Normative documents read

| Document | Sections |
|---|---|
| `docs/architecture/data-model.md` | §7 (primitive matrix and notes, lines 2248 to 2297) |
| `docs/architecture/domain-model.md` | §2.1 (`PlcType`, `TypeExpr`, `StringLength`, `S7Address`, `S7Layout`, `LeafClass`, base units, `IecLiteral`), §3 (I6), §5 (rebind) |
| `docs/architecture/overview.md` | §7 R03, R11, R12, R13, R14. §9 A3, A38, A39, A43 |
| `docs/architecture/api.md` | §6.1 (export representation and base units) |
| `docs/architecture/reliability.md` | §4.3, §4.4 |
| ADRs | 0006, 0017, 0018, 0022, 0024, 0026, 0028, 0045, 0114, 0115, 0116, 0117, 0120, 0137, 0139, 0140, 0143, 0145, 0146, 0147, 0148, 0149, 0156. The decision map of `docs/adr/README.md` marks none of them as superseded. The review followed every "Amended by" note |

### 1.4 Code read

| Layer | Files |
|---|---|
| Seed and rules | `migrations/0004_seed_plc_types.sql`, `migrations/0011_plc_type_rules.sql`, the `plc_type` parts of `0001_schema.sql` and `0002_triggers.sql` |
| Plant model | `crates/recipe-domain/src/plant/`: `primitive.rs`, `platform.rs`, `type_expr.rs`, `iec_literal/` (`range.rs`, `canonical.rs`, `instant.rs`, `duration.rs`), `s7_layout.rs` and `s7_layout/reference.rs`, `s7_width.rs`, `s7_address.rs`, `data_block_number.rs`, `string_length.rs`, `array_dimension.rs`, `limits.rs`, `type_name.rs`, `type_tree.rs` |
| Value and draft | `crates/recipe-domain/src/recipe/value/` (`leaf.rs`, `numeric.rs`, `texts.rs`), `recipe/limit/`, `recipe/draft/s7.rs` |
| Export | `crates/recipe-export/schema/export-v1.schema.json`, `export-v2.schema.json`, `tests/golden_types.rs`, `tests/schema/type_rules.rs` |

### 1.5 Commands run

All commands ran on the branch `docs/full-repo-review-2026-11-13` at commit `8f20b97`. Every run passed.

| Group | Command | Result |
|---|---|---|
| N | `cargo test -p recipe-domain --lib -- numeric adr_0117 integer_types s7_width decimal_extremes` | 51 passed, 0 failed |
| N | `cargo test -p recipe-domain --lib -- texts_tests s7_layout s7_address` | 64 passed, 0 failed |
| N, A | `cargo test -p recipe-export --test golden_types` | 7 passed, 0 failed |
| T | `cargo test -p recipe-domain iec_literal` | 141 passed, 0 failed |
| T | `cargo test -p recipe-domain s5time` | 22 passed, 0 failed |
| T | `cargo test -p recipe-domain type_name` | 19 passed, 0 failed |
| T | `cargo test -p recipe-export golden` | 15 passed, 0 failed (four test binaries) |
| S | `cargo test -q -p recipe-domain --lib string_length` | 24 passed, 0 failed |
| S | `cargo test -q -p recipe-domain --lib value::leaf` | 43 passed, 0 failed |
| S | `cargo test -q -p recipe-domain --lib s7_address` | 12 passed, 0 failed |
| S | `cargo test -q -p recipe-domain --lib data_block_number` | 15 passed, 0 failed |
| S | `cargo test -q -p recipe-domain --lib s7_width` | 4 passed, 0 failed |
| A | `cargo test -p recipe-domain --lib s7_layout` | 25 passed, 0 failed |
| A | `cargo test -p recipe-domain --lib type_expr` | 32 passed, 0 failed |
| A | `cargo test -p recipe-domain --lib array_dimension` | 19 passed, 0 failed |
| A | `cargo test -p recipe-domain --lib s7_members` | 9 passed, 0 failed |
| X | none. Codex ran in a read-only sandbox and executed no `cargo` command | 0 executed |

### 1.6 Siemens sources outside the reference file

- Groups N, S, and A searched the web for official Siemens manuals. They found no official page. They mark their claims "unverified".
- Codex cites official pages of the TIA Portal help (`docs.tia.siemens.cloud`, V20 and V21) for `WCHAR`, `WSTRING`, `ARRAY`, `STRUCT`, and `DATE`. It also cites the S7-1200 manual update (SIOS 89851659) for the `DTL` end.
- The orchestrator tried to read the four TIA help pages again. The pages load their text with JavaScript, and the fetch returned no body text. These claims are therefore marked "official source cited by Codex, not read again".

## 2. Coverage matrix

Cell values:

- Platform column: the S7-300, S7-1200, and S7-1500 marks. Group N writes "Siemens mark / editor mark". "G2" is S7-1200 G2.
- Verdict: `OK`, a finding id `Rnn` of section 3, or an owner question id `OQnn` of section 4.
- Group: the review group of section 1.2. Section 2.2 holds its dimension detail.

### 2.1 Inventory

The count is 278 rows: §2 15, §3 9, §4 5, §5 5, §6 13, §7 2, §8 76, §9 144, §10 9.

§2.16 (BCD) is a note about an encoding and not a type. It gets no row. The same holds for §2.0 (reading guide), the `LDATE` paragraph of §3 (line 277), the `AT` mechanism of §7.6 (line 974), and the field tables of §8.3 and §9.1. Each group states its counting rules below the matrix.

| # | Siemens § | Type (aliases) | Category | `plc_type` name | S7-300 / S7-1200 / S7-1500 | Document cite | Code path | Verdict | Group |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 2.1 | `BOOL` | SUPPORTED | `BOOL` | Y/Y, Y/Y, Y/Y | `data-model.md:2256` | `0004_seed_plc_types.sql:7` | OK | N |
| 2 | 2.2 | `BYTE` | SUPPORTED | `BYTE` | Y/Y, Y/Y, Y/Y | `data-model.md:2257` | `0004_seed_plc_types.sql:8` | R03 (OQ09) | N |
| 3 | 2.3 | `WORD` | SUPPORTED | `WORD` | Y/Y, Y/Y, Y/Y | `data-model.md:2258` | `0004_seed_plc_types.sql:9` | R03 (OQ09) | N |
| 4 | 2.4 | `DWORD` | SUPPORTED | `DWORD` | Y/Y, Y/Y, Y/Y | `data-model.md:2259` | `0004_seed_plc_types.sql:10` | R03 (OQ09) | N |
| 5 | 2.5 | `LWORD` | SUPPORTED | `LWORD` | N/N, N/N (G2 Y), Y/Y | `data-model.md:2260` | `0004_seed_plc_types.sql:11` | R03 (OQ06, OQ09) | N |
| 6 | 2.6 | `SINT` | SUPPORTED | `SINT` | N/N, Y/Y, Y/Y | `data-model.md:2261` | `0004_seed_plc_types.sql:12` | OK | N |
| 7 | 2.7 | `USINT` | SUPPORTED | `USINT` | N/N, Y/Y, Y/Y | `data-model.md:2262` | `0004_seed_plc_types.sql:13` | OK | N |
| 8 | 2.8 | `INT` | SUPPORTED | `INT` | Y/Y, Y/Y, Y/Y | `data-model.md:2263` | `0004_seed_plc_types.sql:14` | OK | N |
| 9 | 2.9 | `UINT` | SUPPORTED | `UINT` | N/N, Y/Y, Y/Y | `data-model.md:2264` | `0004_seed_plc_types.sql:15` | OK | N |
| 10 | 2.10 | `DINT` | SUPPORTED | `DINT` | Y/Y, Y/Y, Y/Y | `data-model.md:2265` | `0004_seed_plc_types.sql:16` | OK | N |
| 11 | 2.11 | `UDINT` | SUPPORTED | `UDINT` | N/N, Y/Y, Y/Y | `data-model.md:2266` | `0004_seed_plc_types.sql:17` | OK | N |
| 12 | 2.12 | `LINT` | SUPPORTED | `LINT` | N/N, N/N (G2 Y), Y/Y | `data-model.md:2267` | `0004_seed_plc_types.sql:18` | OK (OQ06) | N |
| 13 | 2.13 | `ULINT` | SUPPORTED | `ULINT` | N/N, N/N (G2 Y), Y/Y | `data-model.md:2268` | `0004_seed_plc_types.sql:19` | OK (OQ06) | N |
| 14 | 2.14 | `REAL` | SUPPORTED | `REAL` | Y/Y, Y/Y, Y/Y | `data-model.md:2269`, ADR-0115, ADR-0117 | `0004_seed_plc_types.sql:20` | OK (OQ08) | N |
| 15 | 2.15 | `LREAL` | SUPPORTED | `LREAL` | N/N, Y/Y, Y/Y | `data-model.md:2270`, ADR-0115, ADR-0117 | `0004_seed_plc_types.sql:21` | OK (OQ06) | N |
| 16 | 3.1 | `S5TIME` | SUPPORTED | `S5TIME` | yes / no / yes | `data-model.md:2284`, `domain-model.md:245` | `iec_literal/canonical.rs:34`, `range.rs:20` | OK | T |
| 17 | 3.2, 10.1 | `TIME` | SUPPORTED | `TIME` | yes / yes / yes | `data-model.md:2275`, `domain-model.md:243` | `range.rs:14`, `range.rs:79` | OK | T |
| 18 | 3.3 | `LTIME` | SUPPORTED | `LTIME` | no / no / yes | `data-model.md:2276`, `domain-model.md:244` | `range.rs:18`, `range.rs:81` | OK (S7-1200 G2: baseline F14, other group) | T |
| 19 | 3.4 | `DATE` | SUPPORTED | `DATE` | yes / yes / yes | `data-model.md:2277`, `domain-model.md:246`, ADR-0114 | `range.rs:24`, `range.rs:85` | OK, OQ10 | T |
| 20 | 3.5 | `TIME_OF_DAY` (`TOD`) | SUPPORTED | `TIME_OF_DAY` | yes / yes / yes | `data-model.md:2279`, `domain-model.md:248` | `range.rs:38`, `range.rs:88` | OK | T |
| 21 | 3.6 | `LTIME_OF_DAY` (`LTOD`) | SUPPORTED | `LTOD` | no / no / yes | `data-model.md:2280`, `domain-model.md:249` | `range.rs:40`, `range.rs:90` | OK (G2: F14) | T |
| 22 | 3.7 | `DATE_AND_TIME` (`DT`) | SUPPORTED | `DATE_AND_TIME` | yes / no / yes | `data-model.md:2281`, `domain-model.md:250`, ADR-0115 (a) | `range.rs:42`, `range.rs:92`, `instant.rs:119` | OK (deliberate whole seconds) | T |
| 23 | 3.8 | `DATE_AND_LTIME` (`LDT`) | SUPPORTED | `LDT` | no / no / yes | `data-model.md:2282`, `domain-model.md:251` | `range.rs:54`, `range.rs:95` | R02, OQ11 (G2: F14) | T |
| 24 | 3.9 | `DTL` | SUPPORTED | `DTL` | no / yes / yes | `data-model.md:2283`, `domain-model.md:252`, ADR-0115 (b) | `range.rs:56`, `range.rs:96` | R06 | T |
| 25 | 4.1 | `CHAR` | SUPPORTED | `CHAR` | yes / yes / yes | `data-model.md:2273` | `migrations/0004_seed_plc_types.sql:24`, `recipe/value/leaf.rs:135` | OK, OQ05 | S |
| 26 | 4.2 | `WCHAR` | SUPPORTED | `WCHAR` | no / yes / yes | `data-model.md:2274` | `0004_seed_plc_types.sql:25`, `leaf.rs:14` | OQ04, OQ06 | S |
| 27 | 4.3 | `STRING` (`STRING[n]`) | SUPPORTED | `STRING` | yes / yes / yes | `data-model.md:2271`, `domain-model.md:106-112` | `0004_seed_plc_types.sql:22`, `plant/string_length.rs:100` | OQ05, OQ07 | S |
| 28 | 4.4 | `WSTRING` (`WSTRING[n]`) | SUPPORTED | `WSTRING` | no / yes / yes | `data-model.md:2272`, `domain-model.md:106-112` | `0004_seed_plc_types.sql:23`, `string_length.rs:103`, `leaf.rs:126` | OQ04, OQ06, OQ07, R05 | S |
| 29 | 4.5 | Formal string parameter (FB/FC interface `STRING`, `WSTRING`, FC return) | EXCLUDED-BY-DECISION (implicit) | none | n/a | `overview.md:284` A39, `domain-model.md:191` (address grammar has only `DB`) | none | R04 | S |
| 30 | §5.1, §5.2 (lines 498, 506 to 537) | `ARRAY[lo..hi, …] OF T` (fixed array) | SUPPORTED | `kind = Array` row of the machine, created by `EnsureDerivedType` | yes / yes / yes | `domain-model.md:92`, `:136` to `:138`, `:150`, `:219`, `:220`, A38, ADR-0145, ADR-0147, ADR-0148 | `array_dimension.rs`, `type_expr.rs`, `s7_layout.rs:304` | OQ01, OQ02 | A |
| 31 | §5.1, §5.3 (lines 499, 539 to 557) | `ARRAY[*] OF T` (open array parameter) | EXCLUDED-BY-DECISION | none | no / no / no | grammar `bound := -?[0-9]{1,10}` `domain-model.md:138`. A3 `overview.md:248` | `type_expr.rs:199` (`parse_bound`) | R04 | A |
| 32 | §5.1, §5.4 (lines 500, 559 to 588) | `STRUCT … END_STRUCT` (anonymous structure) | SUPPORTED | `kind = Struct` row (named). An anonymous structure maps to a named struct | yes / yes / yes | R13 `overview.md:176`, `domain-model.md:218`, `:220` | `s7_layout.rs:260` | OQ03 | A |
| 33 | §5.1, §5.5 (lines 501, 590 to 600) | PLC data type, UDT | SUPPORTED | `kind = Struct` row with `name` and `revision` | yes / yes / yes | `domain-model.md:86`, `:135`, `:119`, ADR-0143, ADR-0146 | `s7_layout.rs:260`, `type_expr.rs:253` | OQ03 | A |
| 34 | §5.1, §5.6 (lines 502, 602 to 619) | Named value type (NVT) | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, A39 `overview.md:284`, `data-model.md:2296` | `primitive.rs:118` | R04 | A |
| 35 | 6.2 | `POINTER` | EXCLUDED-BY-DECISION | none | n/a | `data-model.md:2296` | `plant/type_name.rs:14` (reserved word) | OK | S |
| 36 | 6.3 | `ANY` | EXCLUDED-BY-DECISION | none | n/a | `data-model.md:2296` | `type_name.rs:14` (reserved word) | OK | S |
| 37 | 6.4 | `VARIANT` | EXCLUDED-BY-DECISION (implicit) | none | n/a | `overview.md:248` A3, `overview.md:284` A39 | none | R04 | S |
| 38 | 6.5 | `REF_TO T` | EXCLUDED-BY-DECISION | none | n/a | `data-model.md:2296` (CODESYS spelling `REFERENCE`), A39 | `type_name.rs:14` (reserved word) | R04 (spelling only) | S |
| 39 | 6.6 | `TIMER` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 40 | 6.6 | `COUNTER` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 41 | 6.6 | `BLOCK_FC` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 42 | 6.6 | `BLOCK_FB` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 43 | 6.6 | `BLOCK_DB` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 44 | 6.6 | `BLOCK_SDB` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 45 | 6.7 | `VOID` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 46 | 6.7 | `PARAMETER` | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 47 | 6.7, 7.1 | `INSTANCE` (instance parameter, "Instance passed as a parameter") | EXCLUDED-BY-DECISION (implicit) | none | n/a | A3, A39 | none | R04 | S |
| 48 | 7.1 | FB name as a multi-instance type | EXCLUDED-BY-DECISION (implicit) | none | n/a | A39 ("A parameter uses a primitive, an array, or a UDT") | none | R04 | S |
| 49 | 7.1 | `ARRAY[...] OF <FB type>` (multi-instance array) | EXCLUDED-BY-DECISION (implicit) | none | n/a | A39 | none | R04 | S |
| 50 | 8.1 | `IEC_TIMER` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3 (`overview.md:248`), not named in `data-model.md:2296` | none | R04 | N |
| 51 | 8.1 | `IEC_LTIMER` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 52 | 8.1 | `TP_TIME` | EXCLUDED-BY-DECISION | — | SFB/N, as `IEC_TIMER`/N, Y/N | A3, not named | none | R04 | N |
| 53 | 8.1 | `TON_TIME` | EXCLUDED-BY-DECISION | — | SFB/N, as `IEC_TIMER`/N, Y/N | A3, not named | none | R04 | N |
| 54 | 8.1 | `TOF_TIME` | EXCLUDED-BY-DECISION | — | SFB/N, as `IEC_TIMER`/N, Y/N | A3, not named | none | R04 | N |
| 55 | 8.1 | `TONR_TIME` | EXCLUDED-BY-DECISION | — | —/N, as `IEC_TIMER`/N, Y/N | A3, not named | none | R04 | N |
| 56 | 8.1 | `TP_LTIME` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 57 | 8.1 | `TON_LTIME` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 58 | 8.1 | `TOF_LTIME` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 59 | 8.1 | `TONR_LTIME` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 60 | 8.2 | `IEC_SCOUNTER` | EXCLUDED-BY-DECISION | — | SFB/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 61 | 8.2 | `IEC_USCOUNTER` | EXCLUDED-BY-DECISION | — | SFB/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 62 | 8.2 | `IEC_COUNTER` | EXCLUDED-BY-DECISION | — | SFB/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 63 | 8.2 | `IEC_UCOUNTER` | EXCLUDED-BY-DECISION | — | SFB/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 64 | 8.2 | `IEC_DCOUNTER` | EXCLUDED-BY-DECISION | — | SFB/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 65 | 8.2 | `IEC_UDCOUNTER` | EXCLUDED-BY-DECISION | — | SFB/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 66 | 8.2 | `IEC_LCOUNTER` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 67 | 8.2 | `IEC_ULCOUNTER` | EXCLUDED-BY-DECISION | — | N/N, N/N (G2 Y), Y/N | A3, not named | none | R04 | N |
| 68 | 8.2 | `CTU_*` (`CTU_SINT` … `CTU_ULINT`) | EXCLUDED-BY-DECISION | — | SFB/N, Y except 64-bit/N, Y/N | A3, not named | none | R04 | N |
| 69 | 8.2 | `CTD_*` (`CTD_SINT` … `CTD_ULINT`) | EXCLUDED-BY-DECISION | — | SFB/N, Y except 64-bit/N, Y/N | A3, not named | none | R04 | N |
| 70 | 8.2 | `CTUD_*` (`CTUD_SINT` … `CTUD_ULINT`) | EXCLUDED-BY-DECISION | — | SFB/N, Y except 64-bit/N, Y/N | A3, not named | none | R04 | N |
| 71 | 8.3 | `ERROR_STRUCT` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 72 | 8.3 | `NREF` | EXCLUDED-BY-DECISION | — | —/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 73 | 8.3 | `CREF` | EXCLUDED-BY-DECISION | — | —/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 74 | 8.3 | `VREF` | EXCLUDED-BY-DECISION | — | —/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 75 | 8.3 | `SSL_HEADER` | EXCLUDED-BY-DECISION | — | Y/N, N/N, N/N | A3, not named | none | R04 | N |
| 76 | 8.3 | `CONDITIONS` | EXCLUDED-BY-DECISION | — | N/N, Y (G2 N)/N, N/N | A3, not named | none | R04 | N |
| 77 | 8.3 | `TADDR_Param` | EXCLUDED-BY-DECISION | — | —/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 78 | 8.3 | `TCON_Param` | EXCLUDED-BY-DECISION | — | —/N, Y/N, conditional/N | A3, not named | none | R04 | N |
| 79 | 8.3 | `HSC_Period` | EXCLUDED-BY-DECISION | — | N/N, Y (G2 N)/N, N/N | A3, not named | none | R04 | N |
| 80 | 8.3 | `AssocValues` | EXCLUDED-BY-DECISION | — | —/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 81 | 8.3 | `ResolvedSymbol` | EXCLUDED-BY-DECISION | — | N/N, N (G2 unverified)/N, FW 3.0/N | A3, not named | none | R04 | N |
| 82 | 8.3 | `TimeTransformationRule` | EXCLUDED-BY-DECISION | — | —/N, by instruction version/N, FW 1.7/N | A3, not named | none | R04 | N |
| 83 | 8.4 | `HW_ANY` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 84 | 8.4 | `HW_DEVICE` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 85 | 8.4 | `HW_DPMASTER` | EXCLUDED-BY-DECISION | — | N/N, N (G2 Y)/N, Y/N | A3, not named | none | R04 | N |
| 86 | 8.4 | `HW_DPSLAVE` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 87 | 8.4 | `HW_IO` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 88 | 8.4 | `HW_IOSYSTEM` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 89 | 8.4 | `HW_SUBMODULE` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 90 | 8.4 | `HW_MODULE` | EXCLUDED-BY-DECISION | — | N/N, N (G2 Y)/N, Y/N | A3, not named | none | R04 | N |
| 91 | 8.4 | `HW_INTERFACE` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 92 | 8.4 | `HW_IEPORT` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 93 | 8.4 | `HW_HSC` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 94 | 8.4 | `HW_PWM` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 95 | 8.4 | `HW_PTO` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 96 | 8.4 | `PORT` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 97 | 8.5 | `AOM_IDENT` | EXCLUDED-BY-DECISION | — | —/N, —/N, —/N | A3, not named | none | R04 | N |
| 98 | 8.5 | `EVENT_ANY` | EXCLUDED-BY-DECISION | — | —/N, —/N, —/N | A3, not named | none | R04 | N |
| 99 | 8.5 | `EVENT_ATT` | EXCLUDED-BY-DECISION | — | —/N, —/N, —/N | A3, not named | none | R04 | N |
| 100 | 8.5 | `EVENT_HWINT` | EXCLUDED-BY-DECISION | — | —/N, —/N, —/N | A3, not named | none | R04 | N |
| 101 | 8.6 | `OB_ANY` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 102 | 8.6 | `OB_DELAY` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 103 | 8.6 | `OB_TOD` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 104 | 8.6 | `OB_CYCLIC` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 105 | 8.6 | `OB_ATT` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 106 | 8.6 | `OB_PCYCLE` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 107 | 8.6 | `OB_HWINT` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 108 | 8.6 | `OB_DIAG` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 109 | 8.6 | `OB_TIMEERROR` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 110 | 8.6 | `OB_STARTUP` | EXCLUDED-BY-DECISION | — | N/N, CPU-dependent/N, CPU-dependent/N | A3, not named | none | R04 | N |
| 111 | 8.7 | `CONN_ANY` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 112 | 8.7 | `CONN_PRG` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 113 | 8.7 | `CONN_OUC` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 114 | 8.7 | `CONN_R_ID` | EXCLUDED-BY-DECISION | — | N/N, N/N, Y/N | A3, not named | none | R04 | N |
| 115 | 8.7 | `DB_ANY` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 116 | 8.7 | `DB_WWW` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 117 | 8.7 | `DB_DYN` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 118 | 8.7 | `REMOTE` | EXCLUDED-BY-DECISION | — | N/N, system operand/N, Y/N | A3. Base type `ANY` is named in `data-model.md:2296`, `REMOTE` is not | none | R04 | N |
| 119 | 8.7 | `RTM` | EXCLUDED-BY-DECISION | — | N/N, Y/N, Y/N | A3, not named | none | R04 | N |
| 120 | 8.7 | `PIP` | EXCLUDED-BY-DECISION | — | N/N, N/N, Y/N | A3, not named | none | R04 | N |
| 121 | 8.8 | `C_ALARM_S` | EXCLUDED-BY-DECISION | — | Y/N, N/N, N/N | A3, not named | none | R04 | N |
| 122 | 8.8 | `C_ALARM` (`C_ALARM_8`, `C_ALARM_8P`) | EXCLUDED-BY-DECISION | — | N (S7-400)/N, N/N, N/N | A3, not named | none | R04 | N |
| 123 | 8.8 | `C_NOTIFY` (`C_NOTIFY_8P`) | EXCLUDED-BY-DECISION | — | N (S7-400)/N, N/N, N/N | A3, not named | none | R04 | N |
| 124 | 8.8 | `C_AR_SEND` | EXCLUDED-BY-DECISION | — | N (S7-400)/N, N/N, N/N | A3, not named | none | R04 | N |
| 125 | 8.8 | `C_ALARM_SD` | EXCLUDED-BY-DECISION | — | N (S7-410H)/N, N/N, N/N | A3, not named | none | R04 | N |
| 126 | §9.1 (line 1265) | `TCON_Param` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 127 | §9.1 (line 1266) | `TCON_IP_v4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 128 | §9.1 (line 1267) | `TCON_QDN` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 129 | §9.1 (line 1268) | `TCON_IP_V4_SEC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 130 | §9.1 (line 1269) | `TCON_QDN_SEC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 131 | §9.1 (line 1270) | `TCON_IP_RFC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 132 | §9.1 (line 1271) | `TCON_ISOnative` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 133 | §9.1 (line 1272) | `TCON_PHONE` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 134 | §9.1 (line 1273) | `TCON_FDL` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 135 | §9.1 (line 1274) | `TCON_Configured` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 136 | §9.1 (line 1275) | `TCON_DNS` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 137 | §9.1 (line 1276) | `TCON_TCPTermination` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 138 | §9.1 (line 1277) | `TCON_IPMulticastTTL` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 139 | §9.1 (line 1278) | `TSelector` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 140 | §9.2 (line 1341) | `TADDR_Param` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 141 | §9.2 (line 1342) | `TADDR_SEND_QDN` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 142 | §9.2 (line 1343) | `TADDR_RCV_IP` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 143 | §9.3 (line 1363) | `IF_CONF_HEADER` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 144 | §9.3 (line 1364) | `IF_CONF_V4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 145 | §9.3 (line 1365) | `IF_CONF_NOS` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 146 | §9.3 (line 1366) | `IF_CONF_NTP` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 147 | §9.3 (line 1367) | `IF_CONF_V6` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 148 | §9.3 (line 1368) | `IF_CONF_MAC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 149 | §9.3 (line 1369) | `IF_CONF_DNS` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 150 | §9.3 (line 1370) | `IP_V4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 151 | §9.3 (line 1371) | `IP_V6` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 152 | §9.3.1 (line 1394) | `Conf_Hostname` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 153 | §9.3.1 (line 1395) | `Conf_Domainname` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 154 | §9.3.1 (line 1396) | `Conf_ClientId` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 155 | §9.3.1 (line 1397) | `Conf_ClientId_Opaque` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 156 | §9.3.1 (line 1398) | `Conf_DNS` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 157 | §9.3.1 (line 1399) | `Conf_DNS_IPV4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 158 | §9.3.1 (line 1400) | `Conf_IPSuitev4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 159 | §9.3.1 (line 1401) | `Conf_IPSuitev4_IPV4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 160 | §9.3.1 (line 1402) | `Conf_NTP` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 161 | §9.4 (line 1412) | `TMAIL_V4` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 162 | §9.4 (line 1413) | `TMAIL_V6` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 163 | §9.4 (line 1414) | `TMAIL_FQDN` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 164 | §9.4 (line 1415) | `TMAIL_V4_SEC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 165 | §9.4 (line 1416) | `TMAIL_V6_SEC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 166 | §9.4 (line 1417) | `TMAIL_QDN_SEC` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 167 | §9.4 (line 1418) | `TMail_FileReference` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 168 | §9.5 (line 1428) | `OPC_UA_SessionConnectInfo` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 169 | §9.5 (line 1429) | `OPC_UA_NodeId` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 170 | §9.5 (line 1430) | `OPC_UA_NodeAdditionalInfo` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 171 | §9.5 (line 1431) | `OPC_UA_NodeAdditionalInfoExt` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 172 | §9.5 (line 1432) | `OPC_UA_BrowsePath` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 173 | §9.5 (line 1433) | `OPC_UA_RelativePath` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 174 | §9.5 (line 1434) | `OPC_UA_RelativePathElement` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 175 | §9.5 (line 1435) | `OPC_UA_QualifiedName` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 176 | §9.5.1 (line 1445) | `OPC_UA_LocalizedTextEncodingMask` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 177 | §9.5.1 (line 1446) | `OPC_UA_ByteStringActualLength` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 178 | §9.6 (line 1454) | `DIS` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 179 | §9.6 (line 1455) | `DNN` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 180 | §9.6 (line 1456) | `GEOADDR` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 181 | §9.6 (line 1457) | `IM0_Data` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 182 | §9.6 (line 1458) | `AlarmData` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 183 | §9.7 (line 1505) | `SI_classic` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 184 | §9.7 (line 1506) | `SI_none` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 185 | §9.7 (line 1507) | `SI_ProgramCycle` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 186 | §9.7 (line 1508) | `SI_TimeOfDay` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 187 | §9.7 (line 1509) | `SI_Delay` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 188 | §9.7 (line 1510) | `SI_Cyclic` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 189 | §9.7 (line 1511) | `SI_HWInterrupt` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 190 | §9.7 (line 1512) | `SI_HWInterrupt_Extended` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 191 | §9.7 (line 1513) | `SI_Submodule` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 192 | §9.7 (line 1514) | `SI_SynchCycle` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 193 | §9.7 (line 1515) | `SI_IORedundancyError` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 194 | §9.7 (line 1516) | `SI_CPURedundancyError` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 195 | §9.7 (line 1517) | `SI_TimeError` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 196 | §9.7 (line 1518) | `SI_DiagnosticInterrupt` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 197 | §9.7 (line 1519) | `SI_PlugPullModule` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 198 | §9.7 (line 1520) | `SI_AccessError` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 199 | §9.7 (line 1521) | `SI_StationFailure` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 200 | §9.7 (line 1522) | `SI_Servo` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 201 | §9.7 (line 1523) | `SI_PreOrPostServo` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 202 | §9.7 (line 1524) | `SI_Ipo` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 203 | §9.7 (line 1525) | `SI_PreInterpolator` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 204 | §9.7 (line 1526) | `SI_Startup` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 205 | §9.7 (line 1527) | `SI_ProgIOAccessError` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 206 | §9.7 (line 1528) | `SI_CPURedundancyError_V2` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 207 | §9.7 (line 1529) | `SI_IORedundancyError_V2` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 208 | §9.8 (line 1546) | `TO_Object` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 209 | §9.8 (line 1547) | `TO_Axis` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 210 | §9.8 (line 1548) | `TO_SpeedAxis` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 211 | §9.8 (line 1549) | `TO_PositioningAxis` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 212 | §9.8 (line 1550) | `TO_SynchronousAxis` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 213 | §9.8 (line 1551) | `TO_ExternalEncoder` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 214 | §9.8 (line 1552) | `TO_OutputCam` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 215 | §9.8 (line 1553) | `TO_CamTrack` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 216 | §9.8 (line 1554) | `TO_MeasuringInput` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 217 | §9.8 (line 1555) | `TO_CamBase` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 218 | §9.8 (line 1556) | `TO_Cam` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 219 | §9.8 (line 1557) | `TO_Cam_10k` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 220 | §9.8 (line 1558) | `TO_Cam_600Seg` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 221 | §9.8 (line 1559) | `TO_Cam_6kSeg` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 222 | §9.8 (line 1560) | `TO_Kinematics` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 223 | §9.8 (line 1561) | `TO_LeadingAxisProxy` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 224 | §9.8 (line 1562) | `TO_Interpreter` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 225 | §9.8 (line 1563) | `TO_InterpreterProgram` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 226 | §9.8 (line 1564) | `TO_InterpreterMapping` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 227 | §9.8 (line 1565) | `TO_CommandTable` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 228 | §9.9 (line 1586) | `PD_TELx` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 229 | §9.9 (line 1587) | `DX_TEL_SyncOp` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 230 | §9.9 (line 1588) | `PD_STW1_611Umode` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 231 | §9.9 (line 1589) | `PD_STW2_611Umode` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 232 | §9.9 (line 1590) | `PD_ZSW1_611Umode` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 233 | §9.9 (line 1591) | `PD_ZSW2_611Umode` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 234 | §9.10 (line 1601) | `HSC_Count` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 235 | §9.10 (line 1602) | `HSC_Period` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 236 | §9.10 (line 1603) | `HSC_Frequency` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 237 | §9.10 (line 1604) | `High_Speed_Counter` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 238 | §9.10 (line 1605) | `SSI_Absolute_Encoder` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 239 | §9.11 (line 1624) | `PID_Compact` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 240 | §9.11 (line 1625) | `PID_3Step` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 241 | §9.11 (line 1626) | `PID_Temp` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 242 | §9.11 (line 1627) | `CONT_C` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 243 | §9.11 (line 1628) | `CONT_S` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 244 | §9.11 (line 1629) | `TCONT_CP` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 245 | §9.11 (line 1630) | `TCONT_S` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 246 | §9.11 (line 1631) | `PULSEGEN` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 247 | §9.11 (line 1632) | `Polyline` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 248 | §9.11 (line 1633) | `SplitRange` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 249 | §9.11 (line 1634) | `RampFunction` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 250 | §9.11 (line 1635) | `RampSoak` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 251 | §9.11 (line 1636) | `Filter_PT1` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 252 | §9.11 (line 1637) | `Filter_PT2` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 253 | §9.11 (line 1638) | `Filter_DT1` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 254 | §9.11 (line 1639) | `Filter_Universal` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 255 | §9.12 (line 1672) | `TO_Struct_TransformationParameter_V1` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 256 | §9.12 (line 1673) | `TO_Struct_TransformationParameter_V2` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 257 | §9.12 (line 1674) | `TO_Struct_Kinematics_Frame` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 258 | §9.12 (line 1675) | `TO_Struct_Kinematics_KinematicsFrame` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 259 | §9.12 (line 1676) | `TO_Struct_Ipr_Position` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 260 | §9.12 (line 1677) | `TO_Struct_Ipr_AxPosition` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 261 | §9.12 (line 1678) | `TO_Struct_Ipr_JtPosition` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 262 | §9.12 (line 1679) | `TO_Struct_Ipr_Frame` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 263 | §9.13 (line 1689) | `TO_IDENT` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 264 | §9.13 (line 1690) | `TO_TAGLAYOUT` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 265 | §9.13 (line 1691) | `TO_TagFields` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 266 | §9.13 (line 1692) | `TO_BasicPos` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 267 | §9.13 (line 1693) | `TO_Ident` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 268 | §9.13 (line 1694) | `TUN_EC`, `TUN_ES`, `PID_CP`, `PID_ES` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 269 | §9.13 (line 1695) | `AXIS_REF` | EXCLUDED-BY-DECISION | none | no / no / no | A3 `overview.md:248`, `data-model.md:2296`, A39 `overview.md:284` | `primitive.rs:118`, `domain-model.md:132` | R04 | A |
| 270 | 10.1 | F-program `BOOL` | EXCLUDED-BY-DECISION (implicit: A3, R03) | none for F (standard `BOOL` is seeded) | n/a | `overview.md:248`, `overview.md:166` | `platform.rs:7-17` | R04 | T |
| 271 | 10.1, 10.5 | F-program `INT` | EXCLUDED-BY-DECISION (implicit: A3, R03) | none for F (standard `INT` is seeded) | n/a | `overview.md:248` | `platform.rs:7-17` | R04 | T |
| 272 | 10.1, 10.5 | F-program `WORD` | EXCLUDED-BY-DECISION (implicit: A3, R03) | none for F (standard `WORD` is seeded) | n/a | `overview.md:248` | `platform.rs:7-17` | R04 | T |
| 273 | 10.1, 10.5 | F-program `DINT` | EXCLUDED-BY-DECISION (implicit: A3, R03) | none for F (standard `DINT` is seeded) | n/a | `overview.md:248` | `platform.rs:7-17` | R04 | T |
| 274 | 10.1 | F-program `DWORD` | EXCLUDED-BY-DECISION (implicit: A3, R03) | none for F (standard `DWORD` is seeded) | n/a | `overview.md:248` | `platform.rs:7-17` | R04 | T |
| 275 | 10.2 | F-UDT (F-compliant PLC data type) | EXCLUDED-BY-DECISION (implicit: A3, A39 allows a plain UDT only) | none | n/a | `overview.md:284` | `platform.rs:7-17` | R04 | T |
| 276 | 10.3 | Fixed-size F array (`ARRAY[0..N] OF INT` or `DINT`) | EXCLUDED-BY-DECISION (implicit: A3) | none for F | n/a | `overview.md:248` | `platform.rs:7-17` | R04 | T |
| 277 | 10.4 | Open-array F parameter `ARRAY[*]` | EXCLUDED-BY-DECISION (implicit: a bound is an integer, `domain-model.md:147`) | none | n/a | `domain-model.md:92`, `domain-model.md:147` | `array_dimension.rs` | R04 | T |
| 278 | 10.5 | F-program named value type (NVT) | EXCLUDED-BY-DECISION (implicit: A39 excludes `ENUM` and alias) | none | n/a | `overview.md:284`, `data-model.md:2296` | none | R04 | T |

#### Counting rules of each group

Group N:

The count is 91 rows: 15 rows in §2 and 76 rows in §8.

- §2.16 `BCD16` and `BCD32` are encodings, not types (line 248). They get no row. §2.0 is a reading guide and gets no row.
- §8.2: the second table (lines 1050-1059) has the count value types as row keys. These types already have a row in §2. The instance names `CTU_*`, `CTD_*`, and `CTUD_*` are three family rows with eight aliases each.
- §8.3.1 to §8.3.4 hold field tables of `ERROR_STRUCT`, `SSL_HEADER`, `AssocValues`, `TimeTransformationRule`, and `CONDITIONS`. `START`, `START.SEQ`, `END`, and `END.SEQ` are member groups, not types. They get no row.
- §8.8: the families `C_ALARM` / `C_ALARM_8` / `C_ALARM_8P` and `C_NOTIFY` / `C_NOTIFY_8P` are one row each.

Platform column: Siemens mark / editor mark. "G2" is S7-1200 G2. "—" means the Siemens file states no mark.

Group T:

Count: 18 rows. §3 gives 9 rows. §10 gives 9 rows. `TIME` of §10.1 is the row of §3.2. The base types of §10.5 (`INT`, `WORD`, `DINT`) are the rows of §10.1.


The availability column of a §10 row is "n/a", because no `PlcPlatform` value stands for an F-CPU or the fail-safe program.

Group S:

Count: **20 rows**.

Constructs without a row:

- §6.2.2 four-byte address pointer. Line 645 calls it "a related addressing format, not a four-byte declaration". It is a form of `POINTER`.
- §6.3.1 and §6.3.2 forms of `ANY`. They are forms of the `ANY` row.
- `AT` (line 974). The Siemens file says "It is a declaration mechanism, not a data type".
- `DB_ANY` and NVT appear in §7.5 only as columns and notes. Their type rows are in §5.6 and line 1224, in other groups.
- `ARRAY[*]` appears only as a column. It belongs to the array group.


"Implicit" means that A3 (the matrix is the list) and A39 (a parameter uses a primitive, an array, or a UDT) close the list, but no rule names the type.

Group A:

Count: 149 rows. §5 gives 5 rows. §9 gives 144 rows.

Counting rules:

- §5: one row per row of the catalog table at lines 496 to 502. The headings §5.2 to §5.6 repeat these names.
- §9: one row per distinct type name in the first column of a type table. The layout tables repeat names and add no row.
- The field rows at lines 1290 to 1305 and 1326 to 1331 are members, not types. They get no row.
- Line 1694 holds four names in one table row. It counts as one family row.
- `TO_IDENT` (line 1689) and `TO_Ident` (line 1693) are two table rows with two spellings. Each gets a row.
- §9 has no note that is not a type, such as the BCD note of §2.16.

Columns "S7-300 / S7-1200 / S7-1500" give the support of the recipe editor, not the support of Siemens.

### 2.2 Dimension detail of the SUPPORTED rows

Each SUPPORTED row covers the dimensions a to h:

- a. name and aliases
- b. availability per platform
- c. size, S7 width letter, alignment, padding, standard and optimized access
- d. value range and limits
- e. literal grammar
- f. canonical JSONB storage and export representation
- g. string length and encoding, array bounds and nesting, struct and UDT names
- h. tests

The ids in this section are the ids of the group results. Section 3.1 maps them to the report ids.

### Group N

Shared evidence for the 15 SUPPORTED rows:

- Availability positive test: `crates/recipe-db/tests/storage/seed.rs:365` `each_seed_row_equals_its_row_of_the_matrix`. Negative test: `crates/recipe-app/src/plant/define_udt/tests.rs:362` `a_member_type_not_on_the_platform_gives_plt_006_with_the_member` (uses `WSTRING` on `S7_300`, the data path is the same for every seed row).
- Width: `crates/recipe-domain/src/plant/s7_width.rs:25-33`, test `crates/recipe-domain/src/plant/s7_width/tests.rs:24` `s7_width_of_each_primitive_follows_the_data_model_column`. Negative tests: `crates/recipe-domain/src/recipe/draft/s7/tests.rs:142` (`INT` at `DBD`), `:162` (`BOOL` at `DBB`).
- Alignment: `domain-model.md:203`. Positive test `crates/recipe-domain/src/recipe/draft/s7/tests.rs:205` (`BYTE` and `BOOL` at an odd byte), `crates/recipe-domain/src/recipe/draft/s7/layout_tests.rs:118` (`SINT` at an odd byte). Negative test `crates/recipe-domain/src/recipe/draft/s7/tests.rs:183` (`INT` at an odd byte).
- Integer ranges: `crates/recipe-domain/src/recipe/value/numeric.rs:17-24`. Tests `crates/recipe-domain/src/recipe/value/leaf/tests.rs:349` (ends and next values for all 12 integer and bit-string types, table `test_support.rs:125`), `numeric/tests.rs:36`, `:45`.
- Canonical text and export: `reliability.md:485`, `api.md:508`, `leaf.rs:191-223`, schema pattern `export-v1.schema.json:122`. Golden export per platform: `crates/recipe-export/tests/golden/types_S7_{300,1200,1500}.json` hold one leaf of each available numeric type. JSONB round trip of 64-bit ends: `crates/recipe-db/tests/storage/contract/draft/insert.rs:102-108`.
- Siemens §11.4 (line 1874) "BYTE and CHAR fields start at a byte boundary; other ordinary supported components follow their standard word-alignment rules" agrees with `domain-model.md:218` and ADR-0147 (a).

| Type | a name | b platform | c size, width, alignment | d range | e literal | f storage, export | g string, array | h tests |
|---|---|---|---|---|---|---|---|---|
| `BOOL` | OK | OK (`data-model.md:2256`) | OK: X, bit packing (`domain-model.md:197`, :218). Siemens optimized 1 B rule (line 100) n/a, an address implies standard access (ADR-0116 line 13) | OK: `true`/`false` only (`reliability.md:492`) | n/a: JSON boolean, no IEC literal | OK: JSON boolean in golden files | n/a | OK: `leaf/tests.rs:80` `bool_values_pass`, `:72` `number_and_text_for_a_bool_give_a_fault` |
| `BYTE` | OK | OK | OK: 1 B, `B`, any byte | OK: 0..255 | N02, NQ04: decimal only | OK: `"255"` in all three golden files | n/a | OK range. N02 for the literal form |
| `WORD` | OK | OK | OK: 2 B, `W`, even byte | OK: 0..65535 | N02, NQ04 | OK: `"65535"` | n/a | OK range. N02 |
| `DWORD` | OK | OK | OK: 4 B, `D`, even byte | OK: 0..4294967295 | N02, NQ04 | OK: `"4294967295"` | n/a | OK range. N02 |
| `LWORD` | OK | NQ01 (G2) | OK: 8 B, `B`, even byte | OK: 0..2^64-1 | N02, NQ04 | OK: `"18446744073709551615"` in `types_S7_1500.json`, JSONB round trip `insert.rs:104` | n/a | OK range. N02 |
| `SINT` | OK | OK | OK: 1 B, `B`, any byte as a parameter, even byte as a struct member (ADR-0147 (a), (c)) | OK: -128..127 | OK: decimal text (`domain-model.md:233`) | OK: `"-128"` | n/a | OK: `leaf/tests.rs:349`, `layout_tests.rs:118` |
| `USINT` | OK | OK | OK: as `SINT` | OK: 0..255 | OK | OK: `"255"` | n/a | OK: `leaf/tests.rs:349`, `texts_tests.rs:105` |
| `INT` | OK | OK | OK: 2 B, `W`, even byte | OK | OK | OK: `"-32768"` | n/a | OK: `texts_tests.rs:105`, `:119`, `draft/s7/tests.rs:183` |
| `UINT` | OK | OK | OK: 2 B, `W` | OK | OK | OK: `"65535"` | n/a | OK: `leaf/tests.rs:349` |
| `DINT` | OK | OK | OK: 4 B, `D` | OK | OK | OK: `"-2147483648"` | n/a | OK: `leaf/tests.rs:349` |
| `UDINT` | OK | OK | OK: 4 B, `D` | OK | OK | OK: `"4294967295"` | n/a | OK: `leaf/tests.rs:349` |
| `LINT` | OK | NQ01 (G2) | OK: 8 B, `B` | OK: i64 | OK | OK: `"-9223372036854775808"`, `insert.rs:105` | n/a | OK: `leaf/tests.rs:349` |
| `ULINT` | OK | NQ01 (G2) | OK: 8 B, `B` | OK: u64 | OK | OK: `"18446744073709551615"`, `insert.rs:103` | n/a | OK: `leaf/tests.rs:349` |
| `REAL` | OK | OK | OK: 4 B, `D`, even byte | OK by decision: `Decimal` range (ADR-0115 (c)), 7 significant digits (ADR-0117 (a)). NQ02 for 6 versus 7 digits | OK: JSON number grammar with exponent (`leaf/tests.rs:317`). `REAL#1.0` form: NQ04 | OK: `"150"` in all golden files. No IEEE special value can enter (`decimal.rs:22-25`) | n/a | OK: `leaf/tests.rs:243`, `:262`, `:203` (`1e-29`), `decimal/tests.rs:126` (`1e29`), `numeric/tests.rs:103` |
| `LREAL` | OK | OK. NQ03 for an address on classic S7-1200 | OK: 8 B, `B`, even byte | OK by decision: ADR-0115 (c), 15 digits (ADR-0117 (a)) | OK as `REAL` | OK: `"-0.125"` in the S7-1200 and S7-1500 golden files. Absent on S7-300 | n/a | OK: `leaf/tests.rs:249`, `:262`, `:291` |

### Group T

#### S5TIME (§3.1)

- a. OK. One name `S5TIME`. Prefix `S5T#` (`mod.rs:147`).
- b. OK. Siemens line 263 gives S7-300 yes, classic S7-1200 no, G2 no, S7-1500 yes. The seed (`0004_seed_plc_types.sql:35`) and `range.rs:82` agree. Test `s5time_range_exists_on_s7_1500_and_s7_300_only` (`range/tests.rs:118`) asserts the absence on `S7_1200`.
- c. OK. 2 bytes, width `W` (`data-model.md:2284`, `primitive.rs:236`, `s7_width/tests.rs:24`). Alignment: even byte, as every 2-byte element (`domain-model.md:203`).
- d. OK. Range `0..=9_990_000` ms equals "0 through 2 h 46 min 30 s" (line 281). The step rule of `canonical.rs:38-43` (10, 100, 1000, 10000 ms per band) accepts a count only when one time base holds it with `N <= 999`. This is the exact set of S5TIME values of lines 291-296.
- e. OK. Units `h` to `ms`, no sign (`duration.rs:23`, `duration.rs:119`). Tests: `parse_accepts_the_s5time_literals_at_every_tier_edge` and `parse_refuses_the_s5time_literals_between_steps_above_an_edge` (`canonical/tests.rs:203`, `:211`), `sign_on_s5time_is_sign_error` (`duration/tests.rs:98`).
- f. OK. Count of milliseconds (A43, `overview.md:288`). Export in milliseconds (`api.md:516`). Golden `types_S7_1500.json` holds `"1000"`.
- g. n/a.
- h. OK. Positive `s5time_maximum_is_valid` (`canonical/tests.rs:120`). Negative `s5time_12345_ms_is_a_step_error` (`canonical/tests.rs:107`), `s5time_above_maximum_is_out_of_range` (`:125`). Property test `stored_count_survives_display_and_parse` (`prop.rs:92`) draws every band.

#### TIME (§3.2, §10.1)

- a. OK. `TIME`, prefix `T#`.
- b. OK. All three S7 platforms (line 264, seed line 26).
- c. OK. 4 bytes, width `D` (`s7_width/tests.rs:24`).
- d. OK. `-2_147_483_648..=2_147_483_647` ms (`range.rs:14`) equals line 308.
- e. OK. Units `d` to `ms`, sign `-` (`duration.rs:14-20`). Canonical form writes every non-zero part.
- f. OK. Milliseconds (`api.md:516`). Golden value `"-5000"` on all three S7 platforms.
- g. n/a.
- h. OK. Positive and negative ends: `time_range_per_platform` (`range/tests.rs:96`). Sign: `negative_time_depends_on_the_platform` (`range/tests.rs:414`).

#### LTIME (§3.3)

- a. OK. `LTIME`, prefix `LT#`.
- b. OK for S7-300 and classic S7-1200 (line 265). S7-1200 G2 supports `LTIME`. Baseline F14 (other group) holds this point.
- c. OK. 8 bytes, width `B`.
- d. OK. Signed 64-bit ns (`range.rs:18`) equals line 314.
- e. OK. Every unit `d` to `ns` (`duration.rs:26-34`).
- f. OK. Nanoseconds (`api.md:517`). Golden `"90000000000"` on `S7_1500`.
- g. n/a.
- h. OK. `ltime_range_per_platform` (`range/tests.rs:102`) tests both ends, one step outside each end, and the absence on `S7_1200` and `S7_300`.

#### DATE (§3.4)

- a. OK. `DATE`, prefix `D#`.
- b. OK. All three S7 platforms (line 266).
- c. OK. 2 bytes, width `W`.
- d. OK with owner question TQ01. Domain: 1990-01-01 to 2168-12-31 on every S7 platform (ADR-0114, `range.rs:24`). Siemens line 324 gives 2169-06-06 for current TIA V21. Siemens line 326 gives 2168-12-31 for older documents. The domain range is inside both ranges.
- e. OK. `D#YYYY-MM-DD`, no fraction (`instant.rs:146`).
- f. OK. Days since 1970-01-01 (A43, `api.md:518`). The PLC counts from 1990-01-01 (line 322). The consumer subtracts 7305. `api.md:512` says that the type name gives the unit.
- g. n/a.
- h. OK. `date_range_per_platform` (`range/tests.rs:144`), `date_literals_at_the_s7_lower_end` (`:424`), `adr_0114_current_set_ends_date_at_72_683_on_s7_1200_and_s7_1500` (`:163`).

#### TIME_OF_DAY / TOD (§3.5)

- a. OK. Type name `TIME_OF_DAY`. `TOD#` is literal syntax only (`data-model.md:2288`). `TOD` is a reserved struct name (`type_name.rs:176`).
- b. OK. All three S7 platforms.
- c. OK. 4 bytes, width `D`.
- d. OK. `0..=86_399_999` (`range.rs:38`) equals line 330.
- e. OK. At most 3 fraction digits (`instant.rs:117`). Test `four_fraction_digits_on_tod_are_a_fraction_error` (`instant/tests.rs:135`).
- f. OK. Milliseconds since midnight (`api.md:520`). Golden `"45000500"`.
- g. n/a.
- h. OK. `time_of_day_range_is_the_same_on_every_platform` (`range/tests.rs:298`), `hour_twenty_four_is_a_calendar_error` (`instant/tests.rs:111`).

#### LTIME_OF_DAY / LTOD (§3.6)

- a. OK. Type name `LTOD`. `LTIME_OF_DAY` is a reserved struct name (`type_name.rs:114`).
- b. OK for S7-300 and classic S7-1200. G2: F14.
- c. OK. 8 bytes, width `B`.
- d. OK. `0..=86_399_999_999_999` (`range.rs:40`) equals line 338.
- e. OK. Up to 9 fraction digits (`instant.rs:118`). Test `ten_fraction_digits_on_ltod_are_a_grammar_error` (`instant/tests.rs:151`).
- f. OK. Nanoseconds since midnight (`api.md:521`). Golden `"45000000000001"`.
- g. n/a.
- h. OK. `ltod_range_is_below_one_day_where_it_exists` (`range/tests.rs:316`) tests both ends, one step outside, and the absence on `S7_1200` and `S7_300`.

#### DATE_AND_TIME / DT (§3.7)

- a. OK. Type name `DATE_AND_TIME`. `DT` is a reserved struct name (`type_name.rs:55`).
- b. OK. S7-300 and S7-1500 yes, S7-1200 classic and G2 no (line 269, seed line 32).
- c. OK. 8 bytes, width `B`, even start.
- d. OK by decision. Siemens line 346 gives 1 ms resolution and the end 2089-12-31 23:59:59.999. ADR-0115 (a) keeps whole seconds and the end 23:59:59 as a deliberate restriction. The domain range is inside the Siemens range.
- e. OK. No fraction (`instant.rs:119`). The weekday nibble (line 361) is derived. The literal does not hold it.
- f. OK. Seconds since 1970-01-01 (A43, `api.md:522`). Golden `"1791808200"`.
- g. n/a.
- h. OK. `date_and_time_range_per_platform` (`range/tests.rs:332`), `date_and_time_literals_at_the_s7_upper_end` (`:434`), `adr_0115_date_and_time_with_milliseconds_is_refused` (`:447`).

#### DATE_AND_LTIME / LDT (§3.8)

- a. TQ02. Type name `LDT`. Siemens line 371 gives the second name `DATE_AND_LTIME`. No document and no reserved-name list holds it.
- b. OK for S7-300 and classic S7-1200. G2: F14.
- c. OK. 8 bytes, width `B`.
- d. OK. `0..=2^63-1` ns (`range.rs:54`) equals line 371 and line 373.
- e. OK. Up to 9 fraction digits.
- f. OK. Nanoseconds since 1970-01-01 (`api.md:519`). Golden `"1791808200000000001"`.
- g. n/a.
- h. T01. Positive `ldt_range_per_platform` (`range/tests.rs:381-383`). Negative test above the upper end on `S7_1500` is missing.

#### DTL (§3.9)

- a. OK. `DTL`, prefix `DTL#`.
- b. OK. S7-300 no, S7-1200 yes, S7-1500 yes (line 379, seed line 34).
- c. OK. 12 bytes, width `B`, even start. The layout reference row `(130, Primitive::Dtl)` (`s7_layout/reference.rs:114`). The field layout of lines 383-392 is the business of the transfer system. The editor stores one count.
- d. OK in code. `0..=2^63-1` ns (`range.rs:56`) equals the current Siemens range (line 381, line 398). T02 covers the text of ADR-0115.
- e. OK. Up to 9 fraction digits. The weekday is derived (line 394). The literal does not hold it.
- f. OK. Nanoseconds since 1970-01-01 (`api.md:519`). Golden `"1791808200500000000"` on `S7_1200` and `S7_1500`.
- g. n/a.
- h. OK. `adr_0115_last_dtl_instant_is_accepted` (`range/tests.rs:470`), `adr_0115_dtl_next_nanosecond_after_the_last_instant_is_refused` (`:485`), `dtl_range_per_platform` (`:392`).

### Group S

#### CHAR (§4.1)

- a. OK. One name `CHAR`, no alias (`data-model.md:2273`, `primitive.rs:183`).
- b. OK. S7-300, S7-1200, S7-1500 yes (Siemens line 408, `data-model.md:2273`, `0004_seed_plc_types.sql:24`).
- c. OK. 1 B, width `B`, start at any byte (Siemens lines 417, 1874; `domain-model.md:203,218`; `primitive.rs:235`; `s7_layout.rs:237`; `s7_width/tests.rs:43`; `s7_layout/tests.rs:185` "CHAR root" 1 byte, class `Byte`).
- d. OK with a recorded narrowing. Range `U+0001..=U+00FF` (`domain-model.md:237`, ADR-0137 (a) line 54, `leaf.rs:11,146`). Siemens raw range is `16#00..16#FF` (line 419). ADR-0137 line 52 records the exclusion of `U+0000` as a deliberate narrowing. Code page: SQ02.
- e. n/a. A `Char` leaf is a JSON string, not an `IecLiteral` (`domain-model.md:229`).
- f. OK. Stored and exported as the JSON string (`domain-model.md:229`, `api.md:508`). A43 does not apply.
- g. SQ02 (encoding Latin-1 against the Siemens code page).
- h. OK. Positive and negative: `recipe/value/leaf/tests.rs:87` `char_takes_exactly_one_scalar_up_to_latin_1` (accepts "é", refuses "Ł", "ab", ""). NUL: `leaf/tests.rs:103` `char_and_wchar_refuse_u0000`. Property generator draws `U+0001..=U+00FF` (`prop/generate.rs:347-352`).

#### WCHAR (§4.2)

- a. OK. One name `WCHAR`.
- b. S7-300 no: OK (Siemens line 431, `data-model.md:2274`). S7-1500 yes: OK. S7-1200 yes: SQ03 (Siemens says "Conditional" for classic S7-1200).
- c. OK. 2 B, width `W`, even start (Siemens lines 425, 1874; `data-model.md:2274`; `primitive.rs:236`; `s7_layout.rs:238`; `s7_width/tests.rs:44`).
- d. SQ01. The editor accepts every scalar `U+0001..=U+FFFF` (`leaf.rs:14,146`). Siemens documents `16#0000..16#D7FF` (line 427).
- e. n/a (JSON string).
- f. OK. JSON string (`domain-model.md:229`).
- g. SQ01 (repertoire).
- h. OK. `leaf/tests.rs:96` `wchar_takes_one_scalar_up_to_the_basic_plane` (accepts "Ł", refuses "😀"). `leaf/tests.rs:103` refuses `U+0000`.

#### STRING / STRING[n] (§4.3)

- a. SQ04. The editor writes `STRING(n)` (`domain-model.md:134`). Siemens writes `STRING[n]` (lines 410, 433).
- b. OK. All three S7 platforms (Siemens line 450, `data-model.md:2271`, `string_length.rs:100`).
- c. OK. Size `n + 2` (Siemens line 435, `data-model.md:2271`, `string_length.rs:87`, test `string_length/tests.rs:136` `size_of_string_20_is_22_bytes`). Width `B` for every length (ADR-0139 (i), `data-model.md:2252`, `s7_width.rs:28`, test `s7_width/tests.rs:64`). Even start (Siemens line 450 "begins on a word boundary" in standard blocks, `domain-model.md:203`, `s7_layout.rs:236`, test `s7_layout/tests.rs:188`). Array stride rounds to even (`s7_layout.rs:310`, test `s7_layout/tests.rs:146` "4 strings of 3" gives 24 bytes). The editor models standard access only (ADR-0147).
- d. OK. Maximum 254 (Siemens line 435, `domain-model.md:111-112`, `string_length.rs:100-102`). Default 254 (Siemens line 435, `0004_seed_plc_types.sql:22`, `string_length.rs:74-75`). Minimum 1 (`string_length.rs:54`). Process limit `max_length` is `1..=string_length` (`limit/parse.rs:147-150`).
- e. n/a (JSON string).
- f. OK. Stored as the string. Export carries `types[].string_length` (`api.md:496`) and the value as a JSON string (`api.md:508`).
- g. Length unit OK: characters, one Latin-1 byte each (ADR-0137 (h) line 61, `leaf.rs:128`). No-NUL rule: deliberate narrowing (ADR-0137 (a), line 54). Siemens line 448 says `00` is a stored character code, so the narrowing is real and recorded (ADR-0137 lines 76-77). Code page: SQ02.
- h. OK. Length: `string_length/tests.rs:30` (254 passes, 255 fails), `:47` (0 fails). Value: `leaf/tests.rs:31` and `:24` (Latin-1 passes, "Ł" fails), `:41` and `:36` (length), `:115` (NUL). End rule: `draft/s7/tests.rs:566` and `:578`.

#### WSTRING / WSTRING[n] (§4.4)

- a. SQ04 (`WSTRING(n)` against `WSTRING[n]`).
- b. S7-300 no: OK (Siemens line 471, `string_length.rs:104`, test `string_length/tests.rs:90`). S7-1500 yes: OK. S7-1200 yes: SQ03.
- c. OK. Size `2n + 4` (Siemens line 456, `string_length.rs:88`, test `string_length/tests.rs:142` `size_of_wstring_80_is_164_bytes`). Width `B` (`s7_width.rs:28`, `s7_width/tests.rs:42`). Even start (`s7_layout.rs:236`).
- d. OK. Maximum 16 382 (Siemens line 456, `domain-model.md:112`, `string_length.rs:103`, test `string_length/tests.rs:78`). Default 254 (Siemens line 456, `0004_seed_plc_types.sql:23`). The editor does not take the 65 534 value of line 471, which Siemens itself rejects.
- e. n/a (JSON string).
- f. OK. JSON string (`api.md:508`).
- g. Length unit OK: UTF-16 code units (ADR-0137 (i) line 62, `leaf.rs:126`). Siemens line 467: "The two header values count character units, not bytes". Repertoire: SQ01. Code comment: S02.
- h. OK. `leaf/tests.rs:56` `wstring_counts_utf16_units` (negative), `:62` `wstring_with_one_surrogate_pair_fits_two_units` (positive), `:123` (NUL). `string_length/tests.rs:78` (16 382 passes, 16 383 fails).

#### S7 address and data block number (ADR-0116, ADR-0140, ADR-0156)

- Offset `0..=65535` and bit `0..=7` (`domain-model.md:191`, `s7_address.rs:141-157`) agree with Siemens lines 660 and 668.
- End rule: offset plus layout size is at most 65 536 (`domain-model.md:205`, `draft/s7.rs:159-183`). Tests: `draft/s7/tests.rs:566` (passes), `:578` (`E-RCP-056`).
- Data block limit 16 000 on `S7_300` and 59 999 on `S7_1200` and `S7_1500` (`domain-model.md:207`, `platform.rs:72-78`, `draft/s7.rs:189-205`). Tests: `draft/s7/tests.rs:443`, `:457`, `:485`, `:495`. The Siemens file gives no limit per CPU. It says only "Actual usable ranges depend on the CPU" (line 668). No conflict.
- Machine data block (ADR-0156 (a), (b)): `data_block_number.rs:41-56`. Tests: `plant/machine/data_block_tests.rs:45`, `:57`, `:108`. Code agrees with the ADR.
- Grammar rejects `DB0` (`s7_address.rs:131-137`, `[1-9]` first digit). OK.

### Group A

#### Row 1: fixed `ARRAY[lo..hi, …] OF T`

- a. Name: OK. The grammar `array := "ARRAY[" dim ("," dim){0,2} "] OF " type_expr` (`domain-model.md:136`) uses the Siemens form of line 498.
- b. Availability: OK on all three platforms (Siemens line 498). Dimensions: A38 allows 1 to 3 (`overview.md:283`, `limits.rs:14`). Siemens allows up to 6 (line 515). The project rule is stricter, so no wrong type is possible.
- b. Array of an array: OK on `S7_1200` and `S7_1500`. `type_expr.rs:91` refuses it (ADR-0148 (a)), as Siemens line 515 requires. On `S7_300`, see AQ01.
- c. Layout: OK for the standard layout. `s7_layout.rs:304` to `:331` gives the five sizes of the Siemens table at lines 527 to 531: 2, 4, 4, 6, and 12 bytes.
- c. `BOOL` lines: OK. Each line of the last index starts at the next byte (ADR-0147 (d), `s7_layout.rs:324`). This agrees with Siemens line 523.
- c. Three dimensions of `BOOL`: Siemens line 523 supports the interim reading of ADR-0147 (g). The Siemens file gives no example, so reference row A03 stays `Unverified` (`reference.rs:415`). No new finding.
- c. Array of structs: OK. The stride is the padded struct size (`s7_layout.rs:309` to `:311`, Siemens lines 521 and 594).
- c. Optimized access: n/a. The type page states that its offsets apply to a data block with standard access (ADR-0108 (h), `0108-version-page-and-type-page-additions.md:64`).
- d. Bound range: see AQ02. Bounds are `i32` with `upper - lower < 65536` (`domain-model.md:92`, `array_dimension.rs:48` to `:57`). Siemens line 512 gives −32,768 to 32,767 for standard access.
- d. Lower subscript: OK. A bound can be negative or 0 (Siemens line 508). ADR-0145 (a) fixes only the PostgreSQL subscript of `dim_lower` and `dim_upper`, not the PLC bound.
- e. Literal: n/a for `IecLiteral`. The type text round trips: `type_expr_examples_round_trip` (`type_expr/tests.rs:21`) and the property test `type_expr_generated_text_round_trips` (`type_expr/tests.rs:355`).
- f. JSONB and export: OK. A value has one JSON level per dimension, so the last index changes fastest, as in Siemens line 519. The export writes `dimensions[]` as `{lower, upper}` (`api.md:498`). Test: `a_two_dimensional_array_of_a_struct_keeps_the_member_and_element_order` (`export/parameters/tests.rs:344`).
- g. Bounds, lower subscript, nesting: AQ01 and AQ02. ADR-0045 changes the page layout only, so it is n/a for Siemens.
- h. Tests. Positive: `span_of_65535_passes_with_65536_elements` (`array_dimension/tests.rs:24`), `type_expr_array_of_array_passes_on_the_other_platforms` (`type_expr/tests.rs:393`), `s7_layout_golden_struct_and_array_sizes` (`s7_layout/tests.rs:193`). Negative: `span_of_65536_fails_with_span` (`array_dimension/tests.rs:16`), `four_dimensions_fail_with_count` (`array_dimension/tests.rs:83`), `type_expr_array_of_array_is_not_on_s7_1200_and_s7_1500` (`type_expr/tests.rs:373`). No test for the bound range of standard access, because no rule exists (AQ02).

#### Row 3: `STRUCT` (anonymous structure)

- a. Name: OK. The editor has no anonymous structure. A user defines a named struct. Siemens lines 588 and 594 give one layout rule for both forms.
- b. Availability: OK on all three platforms (Siemens line 500). Nesting limit: see AQ03.
- c. Layout: OK. `struct_layout` (`s7_layout.rs:260` to `:272`) places members in declaration order and rounds the size to an even byte. This agrees with Siemens lines 577 to 588 and 1874.
- c. The Siemens example of lines 579 to 586 puts a `DINT` at byte 2 and gives 8 bytes. Test `s7_members_golden_struct_with_two_bools` (`s7_layout/tests.rs:280`) pins a `REAL` at byte 2 and a size of 8 bytes.
- c. A `SINT` or `USINT` member starts on an even byte (ADR-0147 (a), `s7_layout.rs:247` to `:255`). Siemens §5.4 gives no rule for it. No new finding.
- d. Value range: n/a.
- e. Literal: n/a.
- f. JSONB and export: OK. The export lists `members[]` by ordinal (`api.md:499`). Test: `the_serialized_value_of_zone1_holds_setpoint_before_label` (`export/parameters/tests.rs:270`).
- g. Member names: OK. Siemens §5.4 gives no name rule. ADR-0143 (b) makes member names unique without case. Nesting: AQ03.
- h. Tests. Positive: `s7_members_golden_nested_struct_and_array` (`s7_layout/tests.rs:314`). Negative: `member_names_that_differ_only_in_case_are_plt_021_under_both_rule_sets` (`plc_type/tests.rs:405`).

#### Row 4: PLC data type (UDT)

- a. Name: OK. The type text `udt_name "@" revision` (`domain-model.md:135`) names a revision. The export gives `name` and `revision` in two fields (`api.md:493`, `:494`).
- b. Availability: OK on all three platforms (Siemens line 501). Nesting limit: see AQ03.
- c. Layout: OK, as row 3. The stride of an array of UDTs holds the final padding (Siemens line 594, reference row A11 at `reference.rs:460`).
- d. Value range: n/a.
- e. Literal: n/a.
- f. JSONB and export: OK, as row 3.
- g. Names and revisions: OK. Siemens line 596 says the name identifies the type. ADR-0146 (e) gives two struct names, or two revisions, two shapes. A rebind therefore never treats two structs as one type.
- h. Tests. Positive: `a_rebind_across_two_rows_of_one_shape_keeps_the_value_and_the_limit` (`rebind/kept_member_tests.rs:118`). Negative: `type_expr_reserved_struct_name_in_any_case_is_reserved_name` (`type_expr/tests.rs:182`), `a_limit_on_a_dropped_member_gives_rcp_029` (`rebind/tests.rs:161`).

## 3. Findings

### 3.1 Map of the group ids

| Report id | Severity | Group ids | Baseline |
|---|---|---|---|
| R01 | MEDIUM | X02 | F15 |
| R02 | MEDIUM | T01 | none |
| R03 | MEDIUM | N02 | F12 (test part) |
| R04 | LOW | N01, T03, S01, A01, A02, X01 | F13 |
| R05 | LOW | S02 | none |
| R06 | LOW | T02 | F03 (follow-up) |

Owner questions: AQ01 and XQ04 are OQ01. AQ02 and XQ03 are OQ02. AQ03 and XQ05 are OQ03. SQ01, XQ01, and XQ02 are OQ04. SQ02 and XQ08 are OQ05. NQ01, NQ03, SQ03, and XQ07 are OQ06. SQ04 is OQ07. NQ02 is OQ08. NQ04 is OQ09. TQ01 and XQ06 are OQ10. TQ02 is OQ11. XQ09 is OQ12.

The findings are in the order of severity. No finding is CRITICAL or HIGH.

### R01 — MEDIUM — code-vs-doc, missing check — dimensions c and h — the export schema accepts a malformed `s7_address`

The orchestrator read `export-v1.schema.json:306` again. The schema gives `"s7_address": { "type": ["string", "null"] }` and no `pattern`. Group S reports the same state for `export-v2.schema.json:324`.

**The export schema accepts malformed S7 address strings.**

This is baseline F15. The current rubric assigns MEDIUM because the schema lacks a check.

**Siemens evidence**

`sources/Siemens_PLC_Data_Types_Detailed_EN.md:1841` states:

> “The number after the point is a bit position from 0 to 7, not a decimal fraction.”

**Normative evidence**

`docs/architecture/domain-model.md:191` states:

> “`bit` is `0..=7`.”

The same line defines the DBX, DBB, DBW, and DBD grammar.

**Code evidence**

`crates/recipe-export/schema/export-v1.schema.json:306` contains:

> `"s7_address": { "type": ["string", "null"] }`

The schema supplies no address pattern.

`crates/recipe-export/tests/schema/type_rules.rs:91` tests:

> `fn s7_address_follows_the_platform_rule_of_a31()`

That test checks platform nullability. It does not check address grammar.

**Failure scenario**

A document containing `DB1.DBX0.8` can satisfy this schema field.

The domain parser rejects that address. The exporter therefore does not currently generate this malformed value.

**Proposed fix**

Add address grammar validation under the project’s schema-evolution policy. Add positive DBX/DBB/DBW/DBD cases and negative malformed-address cases.

Do not silently change the compatibility contract of a published schema version.

### R02 — MEDIUM — missing-test — dimension h — no test refuses an `LDT` count above 2^63-1 on `S7_1500`

The orchestrator read `iec_literal/range/tests.rs:381` to `:387` again. The `S7_1500` block tests `0`, `I63_MAX`, and `-1` only. The literal test `adr_0139_the_literal_after_the_last_ldt_and_ldate_is_out_of_range` (line 553) loops over `IEC` only.

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:373` "The valid range leaves `b63` zero. A pattern with `b63 = 1` must not be treated as a supported pre-1970 timestamp or an extended unsigned date range."
- Document: `docs/architecture/domain-model.md:251` "`CodesysIec`, `TwinCat3`, and `S7_1500`: 0 to 2^63-1, which is 1970-01-01-00:00:00 to 2262-04-11-23:47:16.854775807 (ADR-0139)."
- Code: `crates/recipe-domain/src/plant/iec_literal/range.rs:95` `(Primitive::Ldt, S7_1500) => Some(LDT_S7),` with `range.rs:54` `const LDT_S7: RangeInclusive<i128> = 0..=9_223_372_036_854_775_807;`.
- Test: `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:381-387` tests `0`, `I63_MAX`, and `-1` on `S7_1500`. It does not test `I63_MAX + 1`. The literal tests at `range/tests.rs:556-571` loop over `IEC` only.
- Failure scenario: a later change gives `LDT_S7` the earlier unsigned range, or routes `S7_1500` to `LDT_UNSIGNED_EARLIER`. No test fails. The domain then accepts an `LDT` with `b63 = 1`, and the export holds a value that the PLC cannot hold (CRITICAL class).
- Proposed fix: in `ldt_range_per_platform`, add `assert_eq!(try_count(p, PlcPlatform::S7_1500, I63_MAX + 1), Err(IecLiteralReason::OutOfRange))`. Also assert it under `RuleSet::Earlier`, and parse `LDT#2262-04-11-23:47:16.854775808` on `S7_1500`.

### R03 — MEDIUM — missing-test — dimension e — no test pins the refusal of based and typed literals on a bit-string leaf

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:104` "`BYTE` is an 8-bit string with 256 possible patterns, `16#00` through `16#FF`." Line 110 writes `WORD` values as `16#0000` through `16#FFFF`. Line 216 writes `REAL#1.0`.
- Document: `docs/architecture/domain-model.md:233` "The text of a `Numeric` leaf is decimal text with a dot."
- Code: `crates/recipe-domain/src/recipe/value/texts.rs:150` `if !is_json_number(text) { return Err(expected()); }`. Test `crates/recipe-domain/src/recipe/value/texts_tests.rs:108` checks only `["", "12,5", "abc", "+5"]` on `INT` and `USINT`. Line 95 checks `"0x10"` on `REAL`. `grep -rn '16#\|2#1' crates --include=*.rs` finds no test.
- Failure scenario: the refusal of `16#FF`, `2#1010`, `8#377`, and `WORD#16#FF` is correct today, but no test names it. Baseline F12 is still an open owner question. A later change of `is_json_number` or a partial answer to F12 can accept a based form for one class and not for the others. No test then fails.
- Proposed fix: add one `recipe-domain` test that sends `16#FF`, `2#1111_1111`, `8#377`, `BYTE#16#FF`, and `1_000` to `BYTE`, `WORD`, `DWORD`, and `LWORD` leaves and expects `Expected::Integer`. Add `REAL#1.0` on a `REAL` leaf with `Expected::Decimal`. Name F12 in the doc comment.

### R04 — LOW — doc gap — dimensions a and b — the excluded native types have no named exclusion

This is the open baseline item F13. `data-model.md:2296` names `POINTER`, `REFERENCE`, `ANY`, and the IEC `ENUM`, subrange, and alias. A3 (`overview.md:248`) is a closed list, so it excludes every other name only implicitly. Five group results and Codex X01 report the same gap for different families. The fix is one change of the `data-model.md` §7 note. It names each excluded family: parameter types, instances, system types and identifiers, communication and technology types, open arrays, NVTs, and F constructs.

#### Source N01 (§8 system types)

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:997` "System data types, or SDTs, have a structure defined by the system." Line 1009 lists `IEC_TIMER`. Line 1167 lists `HW_ANY`. Line 1224 lists `DB_ANY`.
- Document: `docs/architecture/data-model.md:2296` "Types that are not in scope: IEC `ENUM`, subrange, and alias (Assumption A39), and `POINTER`, `REFERENCE`, `ANY`." `docs/architecture/overview.md:248` A3 makes the matrix the list of supported types, but names no excluded system type.
- Code: `crates/recipe-db/tests/storage/seed.rs:360` `assert_eq!(SEED.len(), 31);`. The seed has no row for any of the 76 names. `docs/adr/0143-iec-identifier-rules-of-struct-and-member-names.md:55` reserves only keywords and primitive names, so a user struct can have the name `IEC_TIMER` or `HW_ANY`.
- Failure scenario: no wrong data is possible, because A3 is a closed list. A reader cannot tell whether the 76 names are a decision or an omission. A user can define a UDT named `IEC_TIMER`. Siemens line 997 says such a struct does not satisfy the system interface. Whether TIA Portal refuses that UDT name is unverified.
- Relation: this finding repeats baseline F13 (open, not assigned to this group) for the §8 names.
- Proposed fix: extend the exclusion note of `data-model.md` §7 with the families of Siemens §8: IEC timer and counter state types and instances, system structures (`ERROR_STRUCT`, `NREF`, `CREF`, `VREF`, `SSL_HEADER`, `CONDITIONS`, `TADDR_Param`, `TCON_Param`, `HSC_Period`, `AssocValues`, `ResolvedSymbol`, `TimeTransformationRule`), `HW_*`, `EVENT_*`, `AOM_IDENT`, `OB_*`, `CONN_*`, `DB_ANY`, `DB_WWW`, `DB_DYN`, `REMOTE`, `RTM`, `PIP`, and `C_ALARM*` / `C_NOTIFY*` / `C_AR_SEND`. The owner can also decide to add these names to the reserved words of ADR-0143 (c). That change needs a new ADR.

#### Source T03 (§10 fail-safe types)

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:1701` "The fail-safe program has its own restricted type set. Support for a type in the standard program does not make it valid in an F-FB, F-FC or F-DB."
- Document: `docs/architecture/data-model.md:2296` "Types that are not in scope: IEC `ENUM`, subrange, and alias (Assumption A39), and `POINTER`, `REFERENCE`, `ANY`." `docs/architecture/overview.md:248` (A3) gives the matrix as the type list. Neither text names an F-CPU, an F-DB, an F-UDT, an F array, an F open array, or an F named value type.
- Code: `crates/recipe-domain/src/plant/platform.rs:7-17` `pub enum PlcPlatform { CodesysIec, TwinCat3, S7_1200, S7_1500, S7_300 }`. No variant or flag marks a fail-safe data block.
- Failure scenario: a user defines an `INT` parameter at the address of an F-DB, or a struct that copies an F-UDT. The editor accepts it, because it does not know the F rules (line 1735: no `ARRAY` in an F-UDT, line 1712: `DWORD` only on S7-300 F). The transfer system then tries to write fail-safe data. A reader of the docs cannot tell if this case is out of scope.
- Proposed fix: add one Assumption to `overview.md` §9 and a note to `data-model.md` §7. Text: "The editor models the standard program only. A recipe parameter never targets an F-DB. F-UDTs, F arrays, F open arrays, and F named value types are not in scope."

#### Source S01 (§6 and §7 parameter, block, and instance types)

- Dimension: classification (b).
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:629` "`VARIANT` | 0 in the type table ... Reference to data with type information." Also line 631 (`TIMER`), line 637 (`VOID`), line 639 (`INSTANCE`), line 877 ("FB name as a multi-instance type").
- Document: `docs/architecture/data-model.md:2296` "Types that are not in scope: IEC `ENUM`, subrange, and alias (Assumption A39), and `POINTER`, `REFERENCE`, `ANY`." `overview.md:284` A39 "A parameter uses a primitive, an array, or a UDT".
- Code: `crates/recipe-domain/src/plant/type_name.rs:14` `RESERVED_KEYWORDS` holds `POINTER`, `ANY`, `REF_TO`, `REFERENCE`, but not `VARIANT`, `VOID`, `TIMER`, `COUNTER`, `BLOCK_FC`, `BLOCK_FB`, `BLOCK_DB`, `BLOCK_SDB`, `PARAMETER`, `INSTANCE` (grep count 0 each). `plant/primitive.rs:8` `enum Primitive` has no variant for these types.
- Failure scenario: a reader cannot tell if `VARIANT`, `TIMER`, or an FB instance is out of scope by decision or by omission. Also, a user can define a struct named `TIMER` or `VARIANT`. ADR-0143 line 282 records this risk for Siemens SCL keywords ("vendor point R4"). Whether TIA refuses such a UDT name is unverified.
- Proposed fix: extend the note of `data-model.md` §7 through an ADR. Name the Siemens types `VARIANT`, `REF_TO`, `TIMER`, `COUNTER`, `BLOCK_FC`, `BLOCK_FB`, `BLOCK_DB`, `BLOCK_SDB`, `VOID`, `PARAMETER`, `INSTANCE`, multi-instances, and formal block parameters. Consider these names for vendor point R4 of ADR-0143.

#### Source A01 (`ARRAY[*]` and NVT)

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:499`: "`ARRAY[*] OF T` | … | No | Conditional … | Formal array parameter whose bounds are supplied by the actual argument."
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:502`: "Named value type / NVT | 1, 2, 4 or 8 … | No | No | No | Conditional: TIA V19 or later, within software units".
- Document: `docs/architecture/data-model.md:2296`: "Types that are not in scope: IEC `ENUM`, subrange, and alias (Assumption A39), and `POINTER`, `REFERENCE`, `ANY`."
- Document: `docs/architecture/overview.md:284`: "A39 | IEC derived types (ENUM, subrange, alias) are not supported. A parameter uses a primitive, an array, or a UDT".
- Code: `crates/recipe-domain/src/plant/type_expr.rs:199`: "/// Parse a bound `-?[0-9]{1,10}`." A `*` bound is a grammar error. `crates/recipe-domain/src/plant/primitive.rs:118`: "pub const ALL: [Self; 29]" holds no NVT.
- Failure scenario: none for data. A3 and the grammar exclude both names. A reader cannot tell if the NVT of S7-1500 is excluded by A39 or forgotten. An NVT is not an IEC `ENUM`.
- Proposed fix: add `ARRAY[*]` and the NVT of S7-1500 to the note at `data-model.md:2296`. Do it in the same edit as baseline F13.

#### Source A02 (§9 communication and technology types)

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:1255`: "Specialized types belong to particular instructions, protocols, or technology objects."
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:1257`: "Technology-object data, FB-instance data, object identifiers, and parameters carrying an object reference have different memory meanings."
- Document: `docs/architecture/overview.md:248`: "A3 | The list of supported data types per platform is the matrix in `data-model.md` section 7."
- Document: `docs/architecture/data-model.md:2296` names only `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, and `ANY`.
- Code: `crates/recipe-domain/src/plant/primitive.rs:118`: "pub const ALL: [Self; 29]". The grammar `type_expr := primitive | string | udt | array` (`domain-model.md:132`) has no system data type.
- Failure scenario: none for data. A3 is a closed list, so the 144 names of §9 cannot be a parameter type. A user can still define a UDT with the same members, for example six `LREAL` for `TO_Struct_Ipr_Position`.
- Proposed fix: add one sentence to the note at `data-model.md:2296`. It names the families: system data types of instructions (`TCON_*`, `TADDR_*`, `IF_CONF_*`, `Conf_*`, `TMAIL_*`, `OPC_UA_*`, `DIS`, `DNN`, `GEOADDR`, `IM0_Data`, `AlarmData`, `SI_*`, `HSC_*`, `PD_*`, `DX_*`, `TO_Struct_*`), technology objects (`TO_*`, `High_Speed_Counter`, `SSI_Absolute_Encoder`, `AXIS_REF`), and FB instance types (`PID_*`, `CONT_*`, `TCONT_*`, `PULSEGEN`, `TUN_*`, filter, ramp, and spline families). Merge it with F13.

### R05 — LOW — code-vs-doc — dimension g — doc comments give the wrong unit of a `WSTRING` length

- Dimension: g.
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:467` "The two header values count **character units, not bytes**."
- Document: `docs/architecture/domain-model.md:106` "A `WSTRING` length counts UTF-16 code units (ADR-0137)." ADR-0137 line 62: "A character outside the Basic Multilingual Plane counts 2."
- Code: `crates/recipe-domain/src/plant/string_length.rs:13` "/// `WSTRING`, two bytes per character." and `string_length.rs:16` "/// Number of characters of a string type. The range depends on the platform." The behavior is correct: `recipe/value/leaf.rs:126` `text.encode_utf16().count()`.
- Failure scenario: a maintainer reads "characters" and changes the count to `chars().count()`. Then `WSTRING(1)` accepts "😀", which needs 2 units on the PLC.
- Proposed fix: change the two comments to "UTF-16 code units" for `WSTRING`, with a cite of ADR-0137 (i).

### R06 — LOW — doc gap — dimension d — ADR-0115 still calls the `DTL` end a restriction below the Siemens range

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:398` "Siemens issued a correction to the 03/2014 manual replacing that endpoint with **2262-04-11 23:47:16.854775807**. The current range agrees with that correction."
- Document: `docs/adr/0115-ranges-narrower-than-the-vendor-range.md:14` "| `DTL` | 1970-01-01 to 2262-04-11, a signed 64-bit count of nanoseconds (`domain-model.md` section 2.1) | to 2554-12-31-23:59:59.999999999 |". Also `:61` "Mitigation: a new ADR widens the range of that type."
- Code: `crates/recipe-domain/src/plant/iec_literal/range.rs:55-56` "`DTL` on S7-1200 and S7-1500: 1970-01-01 to 2262-04-11, never negative, in nanoseconds." `const DTL: RangeInclusive<i128> = 0..=9_223_372_036_854_775_807;`.
- Failure scenario: a reader trusts ADR-0115 and writes a new ADR that widens `DTL` to 2554-12-31. The domain then accepts values after 2262-04-11 that the current Siemens range refuses. The export holds a value that the PLC cannot hold.
- Proposed fix: add an "Amended by" note to ADR-0115 through a new short ADR. The new ADR records that the current Siemens `DTL` range equals the domain range. It removes `DTL` from the list of restrictions and from the widening mitigation.

## 4. Owner questions

### 4.1 Conflicts between a normative document and the Siemens file

The review does not choose a side. A change needs a new ADR in each case.

| Id | Topic | Effect when the owner accepts the Siemens claim | Official source |
|---|---|---|---|
| OQ01 | `ARRAY OF ARRAY` on `S7_300` | CRITICAL class: the editor accepts a shape that the PLC cannot declare | Cited by Codex (TIA ARRAY help), not read again. The S7-SCL V5.3 manual does not settle it |
| OQ02 | Array bound range under standard access | CRITICAL class: the editor accepts a bound outside −32768..32767 on `S7_300` | Cited by Codex (TIA ARRAY help), not read again |
| OQ03 | Nesting depth of structures and UDTs | CRITICAL class: the editor accepts a depth above the PLC limit | Cited by Codex (TIA STRUCT help, V21), not read again |
| OQ04 | `WCHAR` and `WSTRING` character repertoire | CRITICAL class: the editor accepts a character that the PLC type cannot hold | Cited by Codex (TIA WCHAR and WSTRING help, V20), not read again |
| OQ05 | Code page of `STRING` and `CHAR` | Interface rule. No value loss is proven | Codex cites WinCC Unified help (Windows-1252). Not a universal PLC rule |
| OQ06 | Profile of `S7_1200`: G2, firmware conditions | HIGH class: a type availability rule that can be wrong for one firmware | Unverified |
| OQ07 | Notation `STRING[n]` against `STRING(n)` | Display only | Not needed |
| OQ08 | `REAL` precision of 6 or 7 digits | No value loss. Two texts can give one binary32 value | Unverified |
| OQ09 | Based and typed literals (`16#FF`, `REAL#1.0`) | Usability only. Each bit pattern has one decimal text | Not needed |
| OQ10 | End of `DATE` on S7 | No value loss. The domain range lies inside both Siemens ranges | Codex: TIA V21 help gives 2169-06-06, WinCC help gives 2168-12-31 |
| OQ11 | `DATE_AND_LTIME` is not a reserved struct name | A user can name a struct like a Siemens type | Unverified |
| OQ12 | Layout of the three-dimensional `BOOL` array | Offsets of later members can differ from the PLC | Unverified. Needs a TIA measurement |

### OQ01 — `ARRAY OF ARRAY` on `S7_300`

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:515`: "Direct `ARRAY OF ARRAY` declarations are not supported by this construct. Use a multidimensional declaration for several index ranges, or use structures or UDTs containing arrays when a hierarchy is needed."
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:1858`: "S7-300 standard storage | Traditional S7 addressable layout, … standard structure/array rules." S7-300 has standard access only.
- Document: `docs/adr/0148-no-array-of-an-array-on-s7-1200-and-s7-1500.md:59`: "(i) `S7_300`, `CodesysIec`, and `TwinCat3` keep the rule of `domain-model.md` §2.1." `docs/architecture/domain-model.md:150`: "The element of an array is any `TypeExpr`, also another array."
- Document: the same ADR quotes TIA Portal V21 for standard access at line 15: "All data types except ARRAY Multi-instances". Line 45 says: "TIA Portal refuses the type in a block with standard access."
- Code: `crates/recipe-domain/src/plant/type_expr.rs:91`: `let refuses_nesting = matches!(platform, PlcPlatform::S7_1200 | PlcPlatform::S7_1500);`. Test `s7_layout_golden_array_of_array_sizes_on_s7_300` (`s7_layout/tests.rs:202`) keeps such a type on `S7_300`.
- Effect: an `S7_300` machine accepts `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` with an address. The export marks it loadable. The CPU cannot declare the type, and the S7 layout of 8 bytes has no Siemens source. If Siemens confirms line 515 for S7-300, this is CRITICAL: an accepted type that the PLC cannot hold.
- Official manual: unverified. A web search found no SIOS page. The STEP 7 V5.x SCL manual and the TIA Portal help page "ARRAY" are the sources to read.

Second review (Codex XQ04), official check: Current Siemens ARRAY help excludes ARRAY elements. The S7-1200 manual also prohibits nested arrays. [Siemens ARRAY](https://docs.tia.siemens.cloud/r/en-us/v20/data-types/array/basic-information-on-array) The older S7-SCL manual allows broad component types. It does not resolve the retained S7-300 layout cases here. [S7-SCL V5.3 manual](https://cache.industry.siemens.com/dl/files/793/5581793/att_66783/v1/SCL_e.pdf)

### OQ02 — Array bound range under standard access

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:512`: "Standard access | −32,768…32,767". Line 513: "Optimized access | −2,147,483,648…2,147,483,647".
- Document: `docs/architecture/domain-model.md:92`: "`ArrayDimension { lower: i32, upper: i32 }`. One to 3 dimensions (Assumption A38). `lower <= upper`. `upper - lower < 65536`". No platform rule limits a bound.
- Code: `crates/recipe-domain/src/plant/type_expr.rs:199` to `:200`: "Parse a bound `-?[0-9]{1,10}`. The value must fit the INTEGER range." `crates/recipe-domain/src/plant/array_dimension.rs:48` to `:57` checks only the order and the span.
- Effect: `S7_300` has only standard access (Siemens line 1858). It accepts `ARRAY[40000..40001] OF INT` and exports it. The same holds for an `S7_1200` or `S7_1500` parameter with an address, because the S7 layout assumes standard access (ADR-0108 (h)). TIA refuses such a declaration. If confirmed, this is CRITICAL.
- Owner choice: limit the bounds to −32,768 to 32,767 on `S7_300` only, or also for every S7 parameter with an address. A change needs a new ADR and an earlier rule set for stored rows (ADR-0120).
- Official manual: unverified. The web search found no Siemens page.

Second review (Codex XQ03), official check: Siemens specifies signed 16-bit bounds for standard access and signed 32-bit bounds for optimized access. [Siemens ARRAY](https://docs.tia.siemens.cloud/r/en-us/v20/data-types/array/basic-information-on-array)

### OQ03 — Nesting limit of structures and UDTs

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:569`: "Traditional rule, including S7-300, classic S7-1200 and S7-1500 before FW 4.0 | Up to 8 levels". Line 570: "S7-1500, TIA V20 with FW ≥ 4.0 | Up to 26 hierarchy levels."
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:573`: "The top-level structure counts as a level. Under the 26-level rule, an `ARRAY OF STRUCT/UDT` consumes three hierarchy levels."
- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:598`: "The structure nesting limits also apply to UDT hierarchies."
- Document: `docs/architecture/reliability.md:1000`: "`json.max_depth` | 32". Line 488: "`DefineUdt` and `EnsureDerivedType` reject a `TypeTree` deeper than this limit with `E-PLT-018`". No document gives a limit per platform.
- Code: `crates/recipe-domain/src/plant/limits.rs:10`: `pub const MAX_DEPTH: u32 = 32;`. `crates/recipe-domain/src/plant/type_tree.rs:329`: `if metrics.depth > MAX_DEPTH {`. The depth counts one level per struct and one per array dimension (`type_tree.rs:391` to `:409`).
- Effect: an `S7_300` machine accepts a UDT that nests 9 to 32 structs. The PLC refuses a ninth level. The project has no firmware setting, so it cannot apply the 26-level rule of S7-1500 FW 4.0 or later.
- Owner choice: a limit of 8 struct levels on S7, with or without the array count of line 573. A change needs a new ADR, a row in `reliability.md` §7.11, and the earlier rule set of ADR-0120.
- Official manual: unverified.

Second review (Codex XQ05), official check: Siemens documents eight levels normally and 26 for specified newer targets. ARRAY OF STRUCT/UDT consumes three hierarchy levels. [Siemens STRUCT](https://docs.tia.siemens.cloud/r/en-us/v21/data-types/struct-data-structure-anonymous-structures/basic-information-on-struct)

### OQ04 — `WCHAR` and `WSTRING` character repertoire

- Siemens: line 427 "the current TIA V21 `WCHAR` type page gives **`16#0000…16#D7FF`**. Its `WSTRING` page gives a different, stricter prohibition, excluding **`16#D000…16#FFFF`**." Line 429: "Full Unicode support, including surrogate pairs for supplementary characters, is not established by the two-byte width."
- Document: ADR-0137 line 54 "A `WCHAR` value is one Unicode scalar in `U+0001..=U+FFFF`." Line 55: "A `WSTRING` character is any Unicode scalar except `U+0000`." `domain-model.md:237`. `openspec/specs/recipe-management/spec.md:801` accepts `"a😀"` as 3 UTF-16 units.
- Code: `recipe/value/leaf.rs:14` `const WCHAR_MAX: u32 = 0xFFFF;`, `leaf.rs:113` (no repertoire check for `WSTRING`). Test `leaf/tests.rs:62` stores "😀" in `WSTRING(2)`.
- Effect: the editor accepts `WCHAR` `U+E000..U+FFFF` and `U+D000..U+D7FF`, and `WSTRING` characters outside the BMP (surrogate pairs `D800..DFFF`). If the Siemens pages hold, the export carries a value that the PLC refuses or changes. That is the CRITICAL class. Options: keep ADR-0137, narrow to `U+0001..=U+D7FF` for both, or narrow per type.
- Official manual: not checked. One web search found no Siemens page. A CODESYS help page says UCS-2 is `U+0000..U+D7FF` and `U+E000..U+FFFF` (help.plc.abb.com, `_cds_datatype_wstring.html`). That page is not Siemens. Status: unverified.

Second review (Codex XQ01), official check: The official V20 WCHAR page gives U+0000…U+D7FF. The current V21 material retains that limit. [Siemens WCHAR](https://docs.tia.siemens.cloud/r/en-us/v20/data-types/character-strings/character/wchar-s7-1200-s7-1500)

Second review (Codex XQ02), official check: Siemens explicitly prohibits WSTRING code units D000…FFFF. This differs from its WCHAR endpoint. [Siemens WSTRING](https://docs.tia.siemens.cloud/r/en-us/v20/data-types/character-strings/character-strings/wstring-s7-1200-s7-1500)

### OQ05 — Code page of `STRING` and `CHAR`

- Siemens: line 419 "Codes `16#00…16#7F` form the ASCII range; codes above `16#7F` depend on the code page and interface." Line 450: "Its character interpretation follows the same code-page limitations as `CHAR`; it is not implicitly UTF-8."
- Document: ADR-0137 line 61 "(h) A `STRING(n)` length counts characters. Each character is one Latin-1 byte". `domain-model.md:106`, `domain-model.md:237` (`CHAR` is `U+0001..=U+00FF`).
- Code: `recipe/value/leaf.rs:11` `const LATIN_1_MAX: u32 = 0xFF;`, `leaf.rs:110`.
- Effect: the editor maps byte `16#80..16#FF` to Latin-1. A plant whose HMI or PLC uses Windows-1252 or another code page shows other glyphs. The user cannot enter "€" (Windows-1252 byte `16#80`). `api.md` §6 does not tell the transfer client to encode Latin-1. Options: keep Latin-1 and state it in `api.md` §6, or add a code page per machine.
- Official manual: not checked. Status: unverified.

Second review (Codex XQ08), official check: Siemens WinCC documentation explicitly identifies Windows-1252 for STRING. This confirms interface dependence, not a universal PLC encoding. [Siemens S7-1200 character mapping](https://docs.tia.siemens.cloud/r/en-us/v20/configuring-tags-rt-unified/reference-rt-unified/data-types-rt-unified/data-types-for-simatic-s7-1200-rt-unified)

### OQ06 — Profile of `S7_1200`: G2 and firmware conditions

This question merges three group questions. It continues baseline F14.

#### Part NQ01 — S7-1200 G2 and the 64-bit types

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:63` "`LWORD` | 8 | 64-bit string | No | No | Yes | Yes" (columns S7-300, classic S7-1200, S7-1200 G2, S7-1500). Lines 70 and 71 give the same marks for `LINT` and `ULINT`. Line 75 says "The classic S7-1200 and G2 must be treated as separate families."
- Document: `docs/architecture/data-model.md:2260` "`LWORD` | `numeric` | - | 8 | B | yes | yes | no | yes | no" (column `S7_1200` is "no"). `crates/recipe-domain/src/plant/platform.rs:12-13` "Siemens S7-1200 controller. `S7_1200`". No document says whether `S7_1200` means the classic family only.
- Effect: a machine with an S7-1200 G2 CPU uses `S7_1200`. It cannot use `LWORD`, `LINT`, `ULINT`, `LTIME`, `LTOD`, or `LDT`, although G2 supports them. No accepted value is invalid on a G2 CPU, so no wrong export is possible.
- Manual check: not in this run. The repository research record `docs/plan/review/2026-11-08-wp7-r2-r4-platform-research.md:148` cites the G2 manual (S10) and the V21 help, which group G2 with S7-1500. This supports the Siemens claim.
- Question: does the plant use S7-1200 G2? If yes, does it get its own platform (new ADR and seed migration), or does the matrix state that `S7_1200` means the classic family only?

#### Part NQ03 — `LREAL` with an address on classic S7-1200

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:244` "For classic S7-1200 **FW 2.2**, `LREAL` in a global or instance DB is documented only with **optimized access**, and direct addressing is not supported." The same line says the later firmware boundary "is not established here".
- Document: `docs/architecture/domain-model.md:213` "an address is required on `S7_300`, optional on `S7_1200` and `S7_1500`". `docs/architecture/domain-model.md:201` gives an 8-byte primitive the width `B`. `docs/adr/0116-end-of-an-s7-address-and-data-block-number.md:13` says "an optimized block has no offset".
- Code: `crates/recipe-domain/src/plant/s7_width.rs:31` `(_, _) => S7Width::B,` gives `LREAL` an address. No rule refuses an address for `LREAL` on `S7_1200`.
- Effect: on a classic S7-1200 with old firmware, the export can carry `DB1.DBB0` for an `LREAL` that the PLC cannot hold in a standard-access block. Newer firmware can permit it.
- Manual check: unverified. The Siemens file itself marks the later boundary as unknown.
- Question: does the plant run classic S7-1200 firmware before V4? If yes, does the domain refuse an address for `LREAL` on `S7_1200`?

#### Part SQ03 — `WCHAR` and `WSTRING` on classic S7-1200

- Siemens: line 409 "`WCHAR` | 2 | ... | No | Conditional | Yes | Yes". Line 413: "For classic S7-1200, **FW 4.1 / TIA V13 SP1** is a documented supported combination for the wide types. The exact status of a **FW 4.0** target ... is not established".
- Document: `data-model.md:2272` "`WSTRING` | ... | yes | yes | no" and `data-model.md:2274` (`WCHAR`, `S7_1200` yes). No document names a firmware condition.
- Code: `migrations/0004_seed_plc_types.sql:23,25` (`ARRAY['S7_1200', 'S7_1500']`), `plant/string_length.rs:103`.
- Effect: a machine with an early classic S7-1200 firmware accepts `WSTRING` and `WCHAR` parameters that its CPU may not declare. Options: record an Assumption that `S7_1200` means FW 4.1 or later (or G2), or accept the risk.
- Official manual: not checked. Status: unverified.

Second review (Codex XQ07), official check: Searches did not establish the exact historical transition for LREAL or wide-type support. Those transition claims remain **unverified**. Current Siemens documentation distinguishes G2 from classic S7-1200. [Siemens current data-type navigation](https://docs.tia.siemens.cloud/r/en-us/v21/data-types/date-and-time/date)

### OQ07 — Notation `STRING[n]` against `STRING(n)`

- Siemens: line 410 "`STRING` / `STRING[n]` | `n + 2`; default **256**". Line 433: "### 4.3 STRING / STRING[n]".
- Document: `domain-model.md:134` `string := ("STRING" | "WSTRING") ["(" length ")"]`.- Code: `plant/type_expr/tests.rs:24-26` accept `STRING(20)` and `WSTRING(80)`. No test covers `STRING[20]`. The grammar has no bracket form, so the parser refuses it (not run).
- Effect: usability only. An S7 user who types the TIA form `STRING[20]` gets a grammar fault. The export is not affected, because it carries `name` and `string_length` (`api.md:493,496`). Options: keep `(n)`, or also accept `[n]` on S7 platforms.
- Official manual: not needed. The notation is not disputed.

### OQ08 — `REAL` precision: 6 or 7 significant digits

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:204` "Siemens describes the type with a conservative precision of six decimal significant digits; this is not a guarantee that every multi-step calculation preserves six accurate digits."
- Document: `docs/adr/0117-significant-digits-of-real-and-lreal.md:42` "a `REAL` number has at most 7 significant digits". Line 63 records the risk: "Only 6 digits always do. Mitigation: the owner chose the Siemens value of 7." Code: `crates/recipe-domain/src/recipe/value/leaf.rs:17` `const REAL_MAX_DIGITS: u8 = 7;`.
- Effect: the domain accepts two different 7-digit values that the PLC stores as one value. A Python check shows `8589973000` and `8589974000` both become `8589973504.0` in binary32. The page then shows two values that the PLC cannot tell apart. Rounding is monotonic, so a value inside its limits stays inside after the conversion. No limit breach follows.
- Manual check: unverified. The web search reached only third-party sources (solisplc.com), which say "six decimal places". ADR-0117 line 11 cites the Siemens Programming Guideline (entry 81318674) with 7 places.
- Question: does ADR-0117 keep 7 digits with this recorded risk, or does a new ADR set 6?

### OQ09 — Based and typed literals of numeric types

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:104` "`16#00` through `16#FF`", line 112 "`16#8001` is `80 01`", line 216 "`REAL#1.0` encodes as `3F 80 00 00`".
- Document: `docs/architecture/domain-model.md:233` "The text of a `Numeric` leaf is decimal text with a dot." `docs/architecture/reliability.md:485` defines the canonical decimal text. No ADR names the forms `16#`, `8#`, `2#`, or `<TYPE>#`.
- Code: `crates/recipe-domain/src/recipe/value/texts.rs:150` refuses every text outside the JSON number grammar.
- Effect: a user cannot type `16#FF` for a `WORD` mask. Each bit pattern still has one decimal form, so no value is lost. The page shows a mask as a decimal number, which is hard to read.
- Manual check: not needed. The Siemens file and the IEC 61131-3 syntax agree on the forms.
- Question: does the form accept and show the `16#` and `2#` forms for `BYTE`, `WORD`, `DWORD`, and `LWORD`? A change needs a new ADR, because ADR-0039 defines the leaf text.

### OQ10 — End of `DATE` on S7

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:324` "`FF FF` means **2169-06-06**, 65,535 days after the epoch. The last date is the endpoint stated by current TIA V21 type help." Also line 326: "older S7-300/SCL documentation **and the classic S7-1200 FW 2.2 manual** state an endpoint of **2168-12-31**."
- Document: `docs/adr/0114-end-of-date-on-s7-1200-and-s7-1500.md:37` "(a) `DATE` on `S7_1200` and `S7_1500` has the range 1990-01-01 to 2168-12-31." The source of ADR-0114 is the comparison list 02/2025 (entry ID 109986777).
- Code: `crates/recipe-domain/src/plant/iec_literal/range.rs:24` `const DATE_S7: RangeInclusive<i128> = 7_305..=72_683;`.
- Effect: the domain refuses 157 days, 2169-01-01 to 2169-06-06, that a current TIA V21 target accepts. The domain never accepts a date that a target refuses. No data loss occurs. The question is only whether the owner wants the wider V21 end.
- Official manual: not checked in this review. The V21 claim is unverified.

Second review (Codex XQ06), official check: Current STEP 7 DATE help gives 2169-06-06. Other Siemens product documentation still gives 2168-12-31. [STEP 7 DATE](https://docs.tia.siemens.cloud/r/en-us/v21/data-types/date-and-time/date), [WinCC S7-1200 types](https://docs.tia.siemens.cloud/r/en-us/v20/configuring-tags-rt-unified/reference-rt-unified/data-types-rt-unified/data-types-for-simatic-s7-1200-rt-unified)

### OQ11 — `DATE_AND_LTIME` is not a reserved struct name

- Siemens: `sources/Siemens_PLC_Data_Types_Detailed_EN.md:371` "`DATE_AND_LTIME` and `LDT` name the same binary timestamp."
- Document: `docs/adr/0143-iec-identifier-rules-of-struct-and-member-names.md:55` "(c) A struct name is not a reserved word, in any case. The reserved words are the keywords of item (d) and the names of the primitives of `data-model.md` §7." Item (d) takes its words from the Beckhoff keyword page and IEC 61131-3 only. `data-model.md:2288` names `LDATE_AND_TIME` for CODESYS and TwinCAT 3 and does not name `DATE_AND_LTIME`.
- Code: `crates/recipe-domain/src/plant/type_name.rs:201-207` `is_reserved_name` checks the keyword list and `Primitive::name()`. The list holds `LDATE_AND_TIME` (`type_name.rs:108`) and not `DATE_AND_LTIME`. `grep -rn -i DATE_AND_LTIME crates docs migrations openspec` gives no match.
- Effect: a user can define a struct named `DATE_AND_LTIME` on an S7 machine. A TIA project cannot hold a PLC data type with a type keyword as its name. Thus the struct can never match a real UDT. No value is lost. The type list then holds a name that no S7 target can use.
- Official manual: not checked. The claim that TIA reserves `DATE_AND_LTIME` as an identifier is unverified.

### OQ12 — Layout of the three-dimensional `BOOL` array (Codex XQ09)

**Siemens quote**

`sources/Siemens_PLC_Data_Types_Detailed_EN.md:1888` states:

> “Standard BOOL rows start at a new byte”

**Normative quotes**

`docs/adr/0147-s7-layout-from-the-siemens-rules.md:58` states:

> “A `BOOL` array with three dimensions has no decided reading.”

Line 59 states:

> “the type page shows a guess for the offsets of the later members.”

**Code evidence**

`crates/recipe-domain/src/plant/s7_layout/reference.rs:411` names:

> `A03 ARRAY[1..3,1..3,1..3] OF BOOL`

Line 415 marks it:

> `Unverified("needs a TIA measurement")`

The expected size remains ten bytes.

**Effect**

The provisional size can affect later struct-member offsets. Larger total allocation does not prove that those member offsets match the PLC.

The owner already accepted the interim rule. This review does not reopen F08 as a new code defect.

**Official check**

The general Siemens rules support row boundaries. They do not independently verify this specific three-dimensional case in this review.

The target layout remains **unverified**.

**Owner decision**

Obtain compiler-generated offsets for the retained case. Keep the uncertainty visible until that evidence exists.

## 5. Baseline status F01 to F15

| Item | Status | Evidence |
|---|---|---|
| F01 | fixed | ADR-0114 ends `DATE` on `S7_1200` and `S7_1500` at 2168-12-31. `iec_literal/range.rs:24` holds `7_305..=72_683`. Test `range/tests.rs:163`. OQ10 asks about newer Siemens material |
| F02 | fixed | ADR-0115 (a) keeps whole seconds of `DATE_AND_TIME` by decision. `range/tests.rs:447` refuses milliseconds |
| F03 | obsolete | The S7-1200 manual update (SIOS 89851659, page 4) corrects the `DTL` end to 2262-04-11-23:47:16.854775807. `range.rs:56` matches it. R06 covers the stale ADR-0115 text |
| F04 | fixed | ADR-0116 (a). `recipe/draft/s7.rs:159` to `:183`. Tests `draft/s7/tests.rs:566` and `:578` |
| F05 | fixed | ADR-0116 (c) and ADR-0140. `draft/s7.rs:189` to `:205`, `plant/platform.rs:74`. Tests `draft/s7/tests.rs:443` and `:457` |
| F06 | fixed | ADR-0115 (c) keeps the `Decimal` subset of `REAL` and `LREAL` by decision. `recipe/value/numeric.rs:25` |
| F07 | fixed | ADR-0117. `recipe/value/leaf.rs:17` and `:20` check 7 and 15 significant digits. Tests `value/leaf/tests.rs:243`, `:249`, `:262`. OQ08 asks about 6 against 7 digits |
| F08 | fixed | ADR-0147 (h). `s7_layout/reference.rs:473` and `s7_layout/prop.rs:144` use the reference rules. OQ12 keeps one provisional case |
| F09 | fixed | `WSTRING` cases at `s7_layout/reference.rs:324`. `BOOL` dimensions at lines 403 and 411. Nested `S7_300` arrays at `s7_layout/tests.rs:202` |
| F10 | fixed | `recipe-export/tests/golden_types.rs:86` defines 28 S7 primitive cases. Lines 603, 632, 660, and 692 check the platform documents |
| F11 | fixed | `crates/recipe-db/tests/storage/contract/draft/insert.rs:110` checks the 64-bit extremes. `narrow_writes.rs:61` checks later writes |
| F12 | open | `domain-model.md:233` still requires decimal text. No ADR decides on the forms `16#`, `2#`, `8#`, or `<TYPE>#`. Codex rates it obsolete, because each bit pattern has a decimal text. The orchestrator keeps it open as OQ09, because no decision records the choice. R03 covers the missing test |
| F13 | open | R04 |
| F14 | open | `plant/platform.rs:7` has no S7-1200 G2 platform. OQ06 |
| F15 | open | R01. `export-v1.schema.json:306` and `export-v2.schema.json:324` give `s7_address` no pattern |

Totals: 10 fixed (F01, F02, F04 to F11), 1 obsolete (F03), 4 open (F12 to F15).

## 6. Totals

### 6.1 Rows per category

| Siemens § | Rows | SUPPORTED | EXCLUDED-BY-DECISION | GAP |
|---|---:|---:|---:|---:|
| §2 | 15 | 15 | 0 | 0 |
| §3 | 9 | 9 | 0 | 0 |
| §4 | 5 | 4 | 1 | 0 |
| §5 | 5 | 3 | 2 | 0 |
| §6 | 13 | 0 | 13 | 0 |
| §7 | 2 | 0 | 2 | 0 |
| §8 | 76 | 0 | 76 | 0 |
| §9 | 144 | 0 | 144 | 0 |
| §10 | 9 | 0 | 9 | 0 |
| **Total** | **278** | **31** | **247** | **0** |

### 6.2 Findings per severity

| Severity | Count | Ids |
|---|---:|---|
| CRITICAL | 0 | none |
| HIGH | 0 | none |
| MEDIUM | 3 | R01, R02, R03 |
| LOW | 3 | R04, R05, R06 |
| **Total** | **6** | |

Owner questions: 12 (OQ01 to OQ12). Four of them carry a CRITICAL class when the owner accepts the Siemens claim (OQ01 to OQ04).
