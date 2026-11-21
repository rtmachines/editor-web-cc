# Review: CODESYS and TwinCAT data types of the reference file against the recipe editor

Date: 2026-11-19. Reviewer role: CODESYS and TwinCAT 3 domain reviewer and Rust reviewer. This review is read-only. It changes no code, no document, no migration, and not the vendor file.

Result: 81 inventory rows. 30 rows are SUPPORTED, 51 rows are EXCLUDED-BY-DECISION, and 0 rows are GAP. 11 findings: 0 CRITICAL, 0 HIGH, 5 MEDIUM, 6 LOW. 9 owner questions. Six of them (OQ01 to OQ06) become CRITICAL when the owner accepts the vendor claim. The editor then accepts a value or a shape that the PLC cannot hold or declare. The owner decided each owner question on 2026-11-19. Section 4.2 gives the decisions.

Baseline of 2026-11-06: 21 items. 15 are fixed, 2 are settled by decision (#13, #20), 2 are obsolete for this scope (#16, #19), and 2 concern S7 only (#8, #15). No item is open. Section 5 gives the detail.

## 1. Scope and sources

### 1.1 Scope

- Platforms: `CodesysIec` and `TwinCat3` of `PlcPlatform`.
- Version profiles: CODESYS V3.5 SP22 and TwinCAT 3.1 build 4026 are the primary profiles. TwinCAT build 4024 is the compatibility baseline. Assumption A46 admits only build 4026 or later as a `TwinCat3` machine.
- Out of scope: the S7 platforms, PLC communication, and code that does not handle types.
- Source under review: `CODESYS_TwinCAT3_PLC_Data_Types_EN.md` (not normative, 434 lines, copy in `docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md`). The inventory rows come from §2 to §8 (lines 44 to 373).
- §1 (lines 20 to 43) gives byte order, bit numbering, and versions. §9 (lines 374 to 434) is a probe procedure. The groups use it as the reference for layout (dimension c). It gives no row.
- The shorthand `vendor:N` means line N of that file.
- Repository base: `/home/rtm/editor-web-cc-codesys-review` at `d5341cc` on `origin/main`. No group read code from the stale branch of the main checkout.
- The vendor file is not normative. A conflict between it and a normative document is an owner question (section 4), not a code finding.
- Version differences in the vendor file: `LDATE`, `LDT`, and `LTOD` need CODESYS SP16 and TwinCAT 4026 (`vendor:29`). The prefix `UTF8#` and the operator `XSIZEOF` differ between builds 4024 and 4026 (`vendor:30`, `vendor:31`). Generic function block inputs need TwinCAT 4026 (`vendor:291`).

### 1.2 Method

Four Opus agents reviewed one section group each. Codex (`gpt-6.2-astra`, effort high) reviewed all sections in parallel as a second review. The orchestrator merged the results, removed duplicates, and checked the MEDIUM findings in the source. The matrix of section 2 uses the Opus inventory.

| Group | Vendor sections | Rows | Baseline items | Reviewer |
|---|---|---|---|---|
| N: numeric and system types | §2 (lines 44 to 83), §3 (lines 84 to 112), native integers of §7 (lines 255 to 267), §8 (lines 299 to 373) | 46 | #2, #3, #7, #14 | Opus |
| T: time and date types | §4 (lines 113 to 146), with §1 (lines 20 to 43) | 8 | #4, #15, #17, #18, #20, #21 | Opus |
| S: characters and strings | §5 (lines 147 to 174) | 7 | #1, #6, #10, #12, #13, #19 | Opus |
| A: arrays, user-defined types, indirect and generic types | §6 (lines 175 to 252), §7 (lines 268 to 298), §9 (lines 374 to 434) as the layout reference | 20 | #5, #6, #9, #10, #16 | Opus |
| X: second review | §1 to §9 | 85 (finer split) | #1 to #21 | Codex `gpt-6.2-astra` high |

Codex counts 85 rows: 25 S, 2 D (supported derived constructs), 58 E, and 0 GAP. It put each type in the same category as the Opus groups.

The row counts differ for three reasons:

- Opus counts 30 SUPPORTED rows. These are the 25 seeded primitives, 2 default string rows (`STRING` and `WSTRING` without a length), and 3 derived rows (fixed array, nested array, struct). Codex counts the 2 derived rows (fixed array and struct) as D.
- Codex has 9 more excluded rows in the sections of group N, because it splits the `CAA.*` names and the `SAFE*` names and lists `T_AmsNetIdArr` and `T_AmsPort`.
- Codex has one fewer excluded row in group A (no row for `EXTENDS`). It has one fewer in group S (no row for UTF-8 `STRING`).

#### Definitions and severity rubric

Category of each inventory row:
- SUPPORTED: a `plc_type` seed row for the platform exists and has rules.
- EXCLUDED-BY-DECISION: a cited ADR, document section, or Assumption excludes the type. An exclusion without a cited decision is a LOW finding (pattern: Siemens R04, later ADR-0185).
- GAP: neither supported nor excluded by a decision.

A row is right when it meets the dimensions a to h of each SUPPORTED row:
- a. name and aliases
- b. availability per platform and per version profile (SP22, 4024, 4026)
- c. size, alignment, padding, byte order. First find in the normative documents whether the editor models a layout for these platforms at all (`s7_layout`, `s7_width`, `s7_address` are S7 only). If it does not, cite the decision or raise an owner question. A missing layout model is not automatically a GAP.
- d. value range and limits
- e. literal grammar
- f. canonical JSONB storage and export representation
- g. string length and encoding, array bounds and nesting, struct and DUT names
- h. tests: positive and negative, each with `path:line`

Severity:
- CRITICAL: the editor accepts or exports a value or a shape that the declared PLC type on that platform cannot hold.
- HIGH: the editor rejects a valid value or type, or a normative rule has no enforcement in code or in the database.
- MEDIUM: one layer lacks a check that another layer enforces, or a boundary has no negative test.
- LOW: a doc gap, a wrong comment, an exclusion without a named decision.

A conflict between the vendor file and a normative document is an owner question, not a finding. Give the severity class that it gets if the owner accepts the vendor claim.

### 1.3 Normative documents read

The groups followed the decision map of `docs/adr/README.md`. The table lists the documents that the group results cite.

| Document | Sections and lines |
|---|---|
| `docs/architecture/data-model.md` | §7 (primitive matrix and notes, lines 2290 to 2336), the array example at line 2417, `size_bytes` note at line 2325 |
| `docs/architecture/domain-model.md` | §2.1 (`PlcType`, `TypeExpr`, `StringLength`, `S7Layout`, base units, `IecLiteral`, lines 84 to 288), §3 (invariants), JSON forms (lines 436 to 444) |
| `docs/architecture/overview.md` | §7 R11, R13. §9 A3, A31, A38, A39, A43, A46, A50 |
| `docs/architecture/api.md` | §6 and §6.1 (export representation and base units, lines 494 to 547) |
| `docs/architecture/reliability.md` | §4.3, §4.4 (lines 503 to 512) |
| `docs/ops/runbook.md` | TwinCAT version check (lines 1371 to 1375) |
| ADRs | 0017, 0019, 0026, 0028, 0108, 0109, 0115, 0117, 0137, 0138, 0139, 0141, 0142, 0143, 0144, 0145, 0146, 0177, 0179, 0180, 0181, 0183, 0185, 0186. The groups checked ADR-0177 to ADR-0186 for an effect on these two platforms. Section 2.2 of each group gives the result |
| Earlier research | `docs/plan/review/2026-11-06-plc-type-review.md` (baseline items `#n`), `docs/plan/review/2026-11-08-wp7-r2-r4-platform-research.md` |

### 1.4 Code read

| Layer | Files |
|---|---|
| Seed and rules | `migrations/0001_schema.sql`, `migrations/0004_seed_plc_types.sql`, `migrations/0011_plc_type_rules.sql` |
| Plant model | `crates/recipe-domain/src/plant/`: `platform.rs`, `plc_type.rs`, `primitive.rs`, `type_expr.rs`, `string_length.rs`, `iec_literal/` (`mod.rs`, `range.rs`, `canonical.rs`, `instant.rs`, `duration.rs`), `array_dimension.rs`, `type_name.rs`, `type_tree/`, `limits.rs`, `lexical.rs` |
| Value and limits | `crates/recipe-domain/src/recipe/value/` (`leaf.rs`, `numeric.rs`, `texts.rs`, `decimal.rs`, `walk.rs`), `recipe/limit/`, `recipe/rebind/`, `recipe/draft/` |
| Export | `crates/recipe-export/schema/export-v1.schema.json`, `export-v2.schema.json`, `crates/recipe-export/tests/golden_types.rs`, `crates/recipe-export/tests/golden/`, `crates/recipe-domain/src/export/` |
| Web and database tests | `crates/recipe-web/src/pages/types/tree.rs`, `crates/recipe-db/tests/storage/` |

### 1.5 Commands run

All commands ran in `/home/rtm/editor-web-cc-codesys-review` at commit `d5341cc`. Every `cargo` run passed. The `recipe-db` tests need Docker and did not run. Codex ran no `cargo` command. Group letters mark the source of each line.

- N: `git log --oneline -1` (review tree). Result: `d5341cc Merge pull request #88`
- N: `grep -n '^#' CODESYS_TwinCAT3_PLC_Data_Types_EN.md`. Result: §2 44, §3 84, §7 253 (native 255, next 268), §8 299, §9 374. The prompt ranges match
- N: Read `migrations/0004_seed_plc_types.sql:6-21`. Result: 15 numeric and `BOOL` rows hold `CodesysIec` and `TwinCat3`
- N: `grep -rn 'serde_json' Cargo.toml`. Result: `arbitrary_precision` at `Cargo.toml:63`
- N: `grep -n 'const PLATFORM' -r plant/type_tree/`. Result: `fixtures.rs:13` `PlcPlatform::TwinCat3`
- N: `grep -rn '16#\|2#1\|BYTE#\|REAL#\|adr_0183' crates --include=*.rs`. Result: `texts_tests.rs:596`, `:604`, `:619`, `type_expr/tests.rs:437`
- N: `rg -n 'NaN\|Infinity\|"inf"' crates -g '*.rs'`. Result: no match (R03)
- N: `grep -rnw '"<name>"' crates` and `grep -cxw` in `type_name.rs` for `BIT XINT UXINT XWORD __XINT __UXINT __XWORD PVOID VERSION __VECTOR TIMESTRUCT T_MaxString`. Result: 0 for each (OQ05)
- N: `grep -n -i 'binary64\|IEEE\|LREAL' docs/architecture/overview.md` and `grep -rln -i 'binary64\|converts LREAL\|LREAL to REAL' docs/adr docs/architecture`. Result: no match (OQ01)
- N: `grep -rn -i 'iec size\|iec_size\|size on codesys\|fn size_bytes\|fixed_size_bytes' crates --include=*.rs`. Result: only S7 sizes (cross-group note)
- N: `grep -n 'CodesysIec\|TwinCat3' crates/recipe-export/tests/golden.rs schema/type_rules.rs` and `golden_types.rs:289`. Result: per-type goldens only for S7. `TwinCat3` golden with one `INT` (R09)
- N: `cargo test -q -p recipe-domain --lib integer_types_accept`. Result: ok, 2 passed
- N: `cargo test -q -p recipe-domain --lib adr_0117`. Result: ok, 29 passed
- N: `cargo test -q -p recipe-domain --lib adr_0183`. Result: ok, 2 passed
- N: `cargo test -q -p recipe-domain --lib decimal_extremes_fit`. Result: ok, 1 passed
- N: `cargo test -q -p recipe-domain --lib float_ranges_are`. Result: ok, 1 passed
- N: `cargo test -q -p recipe-domain --lib bool_text_is_true_or_false`. Result: ok, 1 passed
- N: `cargo test -q -p recipe-domain --lib parse_decimal_follows`. Result: ok, 1 passed
- N: `cargo test -q -p recipe-domain --lib zero_with_an_exponent`. Result: ok, 1 passed
- N: `recipe-db` seed tests (`seed.rs:365`). Result: not run (Docker)
- T: `git log --oneline -1` in the review tree: `d5341cc Merge pull request #88`.
- T: `grep -n '^#' CODESYS_TwinCAT3_PLC_Data_Types_EN.md`: §1 at line 20, §4 at line 113, §5 at line 147.
- T: `cat -n migrations/0004_seed_plc_types.sql`: the eight time rows are at lines 26-33 with the platforms `CodesysIec` and `TwinCat3`.
- T: `grep -rn size_bytes docs/architecture/*.md`: `data-model.md:2325` and `domain-model.md:88` say "S7 size".
- T: `rg -i "iec_size|size_on_platform|platform.*size_bytes|fn .*size.*platform" crates/recipe-domain/src crates/recipe-app/src`: no match (R06).
- T: `grep -n size crates/recipe-export/schema/*.json`: no size field.
- T: `rg -n "SP16|SP ?1[0-9]|SP22|3\.5\.1[0-9]|CODESYS V3" docs/architecture docs/adr docs/ops`: no match (OQ04).
- T: `rg -n '49d17h2m47s295ms|213503d|2106-02-07|LTIME#' crates`: only the calendar check at `range/tests.rs:152` (R09).
- T: `rg -o '"[0-9]{17,20}"' crates/recipe-export/tests/golden/*.json`: `"18446744073709551615"` at `types_S7_1500.json:274`, `:278`.
- T: `cargo test -p recipe-domain --locked iec_literal`: ok, 142 passed, 0 failed.
- T: `cargo test -p recipe-domain --locked adr_0139`: ok, 12 passed.
- T: `cargo test -p recipe-domain --locked ldate`: ok, 11 passed.
- T: `cargo test -p recipe-domain --locked short_or_wrong_names_fail`: ok, 1 passed.
- T: `cargo test -p recipe-domain --locked is_reserved_name_covers`: ok, 1 passed.
- S: `grep -n "^#" CODESYS_TwinCAT3_PLC_Data_Types_EN.md` → §5 at line 147, §6 at 175.
- S: `git -C editor-web-cc-codesys-review log --oneline -1` → `d5341cc Merge pull request #88`.
- S: `grep -n -i "string\|char" migrations/0004_seed_plc_types.sql` → CT rows `:36` (82, 80) and `:37` (164, 80). `CHAR`, `WCHAR` on S7 only (`:24-25`).
- S: `grep -rln "UTF-8\|UTF8\|TcEncoding\|UCS-2" docs/architecture docs/adr` then `grep -rn "UCS-2\|TcEncoding\|UTF8#"` → no line names UCS-2, `TcEncoding`, or `UTF8#`.
- S: `grep -rn "size_bytes" docs/architecture/*.md` → `domain-model.md:88`, `data-model.md:2325` (S7 size).
- S: `grep -rni "IEC size\|iec_size_bytes\|fn iec_size" crates --include=*.rs` → none.
- S: `grep -rn "\.size_bytes()" crates --include=*.rs` → only S7 layout callers. The types page skips CT (`tree.rs:206`).
- S: `grep -rniE 'u\{0*100\}|u\{0*ff\}|\\u0100|\\u00ff|ÿ|Ā' crates --include=*.rs` (no filter) → only `prop/generate.rs:351` and `:413`. No text test value at U+00FF or U+0100.
- S: `grep -nE "WSTRING|WCHAR|UTF-16|code unit" docs/architecture/api.md docs/adr/0109-*.md docs/adr/0181-*.md` → `api.md:140`, `:518`, `:521` only (type names and `string_length`). No line gives the bytes of a `WSTRING` value.
- S: `grep -rn size_bytes crates/recipe-web/src crates/recipe-export/src` → test fixtures and the S7 table of `pages/types/tree.rs:75`, `:217` only.
- S: `grep -rn "WSTRING(0)\|WSTRING, platform, 0" crates --include=*.rs` → none.
- S: `grep -n '\$' CODESYS_TwinCAT3_PLC_Data_Types_EN.md` and `grep -n 'STRING\['` → no line.
- S: `grep -rn '"W\?CHAR"' crates --include=*.rs | grep -i test` → seed and S7 tests only. No CT refusal test.
- S: `cargo test -p recipe-domain --lib -- string_length:: value::leaf::tests type_expr::tests::adr_0183 type_expr::tests::type_expr_malformed` (in the review tree) → `test result: ok. 67 passed; 0 failed`.
- A: `git log --oneline -1` in the review tree: `d5341cc Merge pull request #88`.
- A: `grep -n "^#" CODESYS_TwinCAT3_PLC_Data_Types_EN.md`: headings give §6 at 175, §7 subsections at 268 and 279, §9 at 374.
- A: `grep -n "not in scope\|POINTER\|REFERENCE\|ANY\b\|..." docs/architecture/data-model.md`: exclusion note at 2330, Siemens families at 2331 to 2336.
- A: `grep -rn "ANY_\|UNION\b\|INTERFACE\b\|FUNCTION_BLOCK\|EXTENDS\|open array\|ARRAY\[\*\]" docs/architecture docs/adr/README.md ...`: only `data-model.md:2336` and A50 lines. No IEC family is named.
- A: `grep -n '"UNION\|"END_UNION' crates/recipe-domain/src/plant/type_name.rs`: no match.
- A: `for p in 'ARRAY\[\*' 'POINTER TO' 'REFERENCE TO' 'pack_mode' 'LOWER_BOUND' 'XSIZEOF' 'END_UNION' ...; do grep -rln ...; done` over `crates migrations e2e/tests`: no file for each of these. `UNION` hits are SQL `UNION` only.
- A: `grep -rn -i "iec size\|iec_size\|size_on\|4 bytes on" crates/recipe-domain/src`: no match (R06).
- A: `grep -rn "is_s7()" crates/recipe-domain/src`: platform branches at `type_expr.rs:92`, `type_tree.rs:219`, `:253`, `siemens_names.rs:328`, `iec_literal/range.rs:76`, `connection/shape.rs:61`. No layout branch for the IEC platforms.
- A: `grep -n "plc_platform" crates/recipe-export/tests/golden/*.json`: the golden recipes with an array and a struct are `S7_300` only. The `TwinCat3` golden holds a `WSTRING` only.
- A: `CARGO_TARGET_DIR=<scratchpad>/target-A cargo test -p recipe-domain --lib --locked -- type_expr array_dimension reserved array_of_array two_dimensional_array`: `test result: ok. 78 passed; 0 failed`. The `recipe-app` and `recipe-db` tests above are cited as existing and were not run.
- X: `pwd`, `git rev-parse HEAD origin/main`, `git status --porcelain=v1`, `git diff --stat`. Result: Right review tree. Both revisions resolved to `d5341cc8f347a8a6f5f2c3410adee5d1a5c2f274`. Final status and diff were empty.
- X: `cat CLAUDE.md .claude/rules/{rust,web,sql,docs,openspec}.md`, `cat /home/rtm/.codex/RTK.md`, local skill reads and `ls -a .agents/skills`. Result: Read review controls. The repository-local skill directory was absent.
- X: `nl -ba /home/rtm/editor-web-cc/CODESYS_TwinCAT3_PLC_Data_Types_EN.md` with `sed` over §§1–9. Result: Read assigned vendor evidence, version conditions, layout examples, and probe limitations. This was the only source read from the other tree.
- X: `sed`, `nl -ba`, and `rg -n` over the three historical review/research files named in the brief. Result: Found the report pattern, counts, baseline scenarios, and the S7 scope of the WP7 research.
- X: `nl -ba` on `migrations/{0001_schema,0004_seed_plc_types,0011_plc_type_rules}.sql`. Result: Verified seed marks, S7 size semantics, row kinds, string bounds, dimension guards, and case-insensitive indexes.
- X: `rg -n` and `nl -ba` over `docs/architecture/{overview,domain-model,data-model,reliability,api}.md`, `docs/adr/README.md`, and referenced ADRs. Result: Located current rules, amendments, A3/A26/A31/A38/A39/A43/A46/A50, export units, and the IEC size claim.
- X: `rg --files` and `rg -n 'CodesysIec\|TwinCat3' crates migrations` with the regex alternatives enabled. Result: Located platform-sensitive files. The two-name source search returned 947 matching lines.
- X: `rg -n` and `nl -ba` over `plant/{primitive,platform,type_expr,type_name,plc_type,string_length,array_dimension,limits,type_tree,iec_literal}` and tests. Result: Verified spelling, widths, ranges, nesting, time grammar, UTC-offset absence, repertoire, and positive/negative coverage.
- X: `rg -n` and `nl -ba` over `recipe/{value,limit,rebind,draft}`, domain `export/`, DB storage tests, and application/export tests. Result: Verified decimal normalization, limits, JSONB types, historical rule handling, rebind shape, and export serialization.
- X: `rg -n -i 'iec.size\|size.iec\|iec_byte\|iec.*width\|size_on_platform\|size_bytes_for' crates` and focused `size_bytes` reads. Result: Found S7 helpers and unrelated matches. Native IEC size implementation **not found**.
- X: `rg -n -F 'ARRAY[*' crates`, `rg -n -F 'LTIME#' crates`, `rg -n 'LDATE_AND_TIME' crates`. Result: First two exact spellings **not found**. The long type spelling appeared in the reserved keyword list.
- X: `rg -ni 'CodesysIec.*SP16\|SP16.*CodesysIec\|3\.5\.16\|service pack 16\|SP22\|Codesys.*minimum version' docs/architecture docs/adr docs/ops/runbook.md`, plus `rg -ni '4026' docs/ops/runbook.md`. Result: CODESYS minimum **not found**. TwinCAT commissioning rule found at lines 1371–1375.
- X: `nl -ba` on both export schemas, golden fixtures, schema tests, and archived export design D4. Result: Verified primitive reference union, recursive values, platform/address conditions, declaration limits, and the TwinCAT non-BMP fixture. An initially guessed `types_CodesysIec.json` path did not exist.
- X: `python3 -B` in-memory matrix/seed comparison. Result: 25 normative names and 25 seed names for each platform. Both set differences were empty.
- X: `python3 -B` with `struct.pack`/`struct.unpack` and integer arithmetic. Result: Binary32 conversion of `16777217` returned `16777216`. The final DATE midnight equals `49710 × 86400 = 4294944000` native seconds.
- X: `python3 -B -c 'import jsonschema'`, then `/home/rtm/.local/share/uv/tools/serena-agent/bin/python -B` with `Draft202012Validator`. Result: System Python lacked `jsonschema`. The installed alternate interpreter ran the schema validator on all four platform/schema combinations. Each accepted the R01 cases and rejected string capacity zero. Documents were constructed in memory.
- X: Primary-source reads of the linked CODESYS and Beckhoff WSTRING, STRING, duration, calendar, floating-point, native-width, keyword, structure, and library-time pages. Result: Corroborated the material vendor qualifications in the owner questions. One guessed CODESYS identifier-page URL was inaccessible and supplies no evidence.
- X: `rg --files --no-ignore target/debug/deps` filtered for domain/schema test executables, and `ls -la .reviewdog.y*`. Result: No matching prebuilt test executable or repository reviewdog configuration found. No build or lint run.
- Orchestrator (R01): read `crates/recipe-export/schema/export-v2.schema.json:152`, `:141-147`, `:235`, and `docs/architecture/api.md:533`. Result: `value` accepts any string or boolean, the `type_ref` enum lists `CHAR` and `WCHAR` for every platform, and `string_length` has the maximum 16382.
- Orchestrator (R02): `rg "POINTER TO|REFERENCE TO|ARRAY\[\*\]" crates`. Result: no match. The parser refuses the three texts today.
- Orchestrator (R05): read `crates/recipe-domain/src/plant/string_length/tests.rs:47`. Result: it tests the length 0 for `STRING` only.
- Orchestrator (R06): read `docs/architecture/data-model.md:2325` and `crates/recipe-domain/src/plant/primitive.rs:229`. Result: `fixed_size_bytes` gives S7 sizes only.
- Merge check (R08): `sed -n` on `crates/recipe-domain/src/plant/string_length.rs` (lines 12, 16, 61), `primitive.rs:41`, and `migrations/0001_schema.sql:95`. Result: lines 12 and 16 hold the revised wording of Siemens R05. Line 61, `primitive.rs:41`, and `0001_schema.sql:95` still say "characters" or "two bytes per character".

### 1.6 Sources outside the reference file

- The Opus group results cite no source outside the reference file and the repository.
- Codex cites official pages of CODESYS and Beckhoff for the owner questions OQ01 to OQ08. This report marks each such claim as "official source cited by Codex, not read again".
- Codex found that one guessed CODESYS identifier-page URL was not accessible. That URL gives no evidence.

| Owner question | Page cited by Codex |
|---|---|
| OQ01 | [CODESYS REAL and LREAL](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_real.html) |
| OQ02 | [CODESYS WSTRING](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_wstring.html), [Beckhoff WSTRING](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529437323.html) |
| OQ03 | [CODESYS STRING](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_string.html), [Beckhoff STRING](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529410443.html) |
| OQ05 | [Beckhoff keywords](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529971595.html) |
| OQ06 | [CODESYS structure declaration](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_structure.html). Its syntax table says "at least 2" declarations. This source lies outside the vendor file |
| OQ07 | [CODESYS time constants](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operands_constant_time.html), [Beckhoff time constants](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529294987.html) |
| OQ08 | [Beckhoff calendar types](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529415819.html) |
| Row 27 of group N (`SysTime`) | [CODESYS runtime time types](https://content.helpme-codesys.com/en/LibDevSummary/date_time.html). Codex reads `SysTime` as an alias of `ULINT` from this page |

## 2. Coverage matrix

Cell values:

- Platform column: "CodesysIec / TwinCat3", each as "vendor mark / editor mark". `Y` means available and `N` means not available. `q` means qualified.
- "impl. A3" and "implicit" mean that A3 (the matrix is the list) closes the list, but no rule names the type.
- Verdict: `OK`, a finding id `Rnn` of section 3, or an owner question id `OQnn` of section 4.
- Each group has its own row numbers. A reference such as "row 19" points to the table of the same group.

### 2.1 Inventory

The count is 81 rows: group N 46, group T 8, group S 7, group A 20. The ids in the verdict column are the report ids. Section 3.1 maps them to the group ids.

#### Group N: numeric and system types (§2, §3, native integers of §7, §8)

Profile rule. Vendor §2, §3, the native integer table, and §8 state no version difference between CODESYS V3.5 SP22, TwinCAT 3.1.4024, and TwinCAT 3.1.4026. Every row below is judged against SP22 and 4026. No row hides a 4024 difference. The 4024 differences of vendor §1 (`vendor:29` to `:31`) concern the long date types, `UTF8#`, and `XSIZEOF`. They are not in this group.

| # | Vendor § (line) | Type (aliases) | Category | plc_type name | CodesysIec / TwinCat3 (vendor / editor) | Profile | Document cite | Code path | Verdict | Note |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 2 (50) | `BOOL` | SUPPORTED | `BOOL` | Y/Y, Y/Y | SP22, 4026 (no difference) | `data-model.md:2290`, requirement R11 | `0004_seed_plc_types.sql:7` | OK | JSON boolean, no layout |
| 2 | 2 (51) | `BIT` | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, Y/N | SP22, 4026 | `overview.md:248` (A3) | `plant/primitive.rs:118` (no variant) | R07, OQ05 | Struct and FB member only (`vendor:51`) |
| 3 | 2 (52) | `BYTE` | SUPPORTED | `BYTE` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2291` | `0004_seed_plc_types.sql:8` | OK | Decimal text only (ADR-0183 (b)) |
| 4 | 2 (53) | `WORD` | SUPPORTED | `WORD` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2292` | `0004_seed_plc_types.sql:9` | OK | as `BYTE` |
| 5 | 2 (54) | `DWORD` | SUPPORTED | `DWORD` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2293` | `0004_seed_plc_types.sql:10` | OK | as `BYTE` |
| 6 | 2 (55) | `LWORD` | SUPPORTED | `LWORD` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2294` | `0004_seed_plc_types.sql:11` | OK, R09 | as `BYTE` |
| 7 | 2 (69) | `SINT` | SUPPORTED | `SINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2295` | `0004_seed_plc_types.sql:12` | OK | |
| 8 | 2 (70) | `USINT` | SUPPORTED | `USINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2296` | `0004_seed_plc_types.sql:13` | OK | |
| 9 | 2 (71) | `INT` | SUPPORTED | `INT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2297` | `0004_seed_plc_types.sql:14` | OK | |
| 10 | 2 (72) | `UINT` | SUPPORTED | `UINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2298` | `0004_seed_plc_types.sql:15` | OK | |
| 11 | 2 (73) | `DINT` | SUPPORTED | `DINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2299` | `0004_seed_plc_types.sql:16` | OK | |
| 12 | 2 (74) | `UDINT` | SUPPORTED | `UDINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2300` | `0004_seed_plc_types.sql:17` | OK | |
| 13 | 2 (75) | `LINT` | SUPPORTED | `LINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2301` | `0004_seed_plc_types.sql:18` | OK, R09 | |
| 14 | 2 (76) | `ULINT` | SUPPORTED | `ULINT` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2302` | `0004_seed_plc_types.sql:19` | OK, R09 | |
| 15 | 3 (88) | `REAL` (binary32) | SUPPORTED | `REAL` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2303`, ADR-0028, ADR-0115 (c), ADR-0117 (a) | `0004_seed_plc_types.sql:20` | OK by decision, R03 | Siemens OQ08 (6 or 7 digits) applies here too |
| 16 | 3 (89) | `LREAL` (binary64) | SUPPORTED | `LREAL` | Y/Y, Y/Y | SP22, 4026 | `data-model.md:2304`, ADR-0115 (c), ADR-0117 (a) | `0004_seed_plc_types.sql:21` | OQ01, R03 | CODESYS target exception (`vendor:109`) |
| 17 | 7 (259) | `__XINT` (TwinCAT `XINT`) | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, Y/N | SP22, 4026 | `overview.md:248` | `type_expr.rs:215` (grammar error) | R07, OQ05 | Width follows the target (`vendor:264`) |
| 18 | 7 (260) | `__UXINT` (TwinCAT `UXINT`) | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, Y/N | SP22, 4026 | `overview.md:248` | `type_expr.rs:215` | R07, OQ05 | |
| 19 | 7 (261) | `__XWORD` (TwinCAT `XWORD`) | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, Y/N | SP22, 4026 | `overview.md:248` | `type_expr.rs:215` | R07, OQ05 | |
| 20 | 7 (262) | `PVOID` (TwinCAT, alias of `UXINT`) | EXCLUDED-BY-DECISION | none | N/N, Y/N | 4026 | `overview.md:284` (A39 alias), `data-model.md:2330` (`POINTER`) | `type_expr.rs:215` | OK, OQ05 | Not a reserved struct name |
| 21 | 8 (305) | `VERSION` (CODESYS) | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, N/N | SP22 | `overview.md:248` | `primitive.rs:118` | R07, OQ05 | A user UDT of four `UINT` members can copy it |
| 22 | 8 (306) | `__VECTOR[n] OF REAL` / `OF LREAL` (CODESYS) | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, N/N | SP22 | `overview.md:248` | `type_expr.rs:208-217` (grammar error) | R07 | |
| 23 | 8 (307) | `__SYSTEM.ExceptionCode` | EXCLUDED-BY-DECISION | none | Y/N, Y/N | SP22, 4026 | `overview.md:284` (A39 ENUM) | `primitive.rs:118` | OK | `UDINT`-based enum |
| 24 | 8 (308) | `__SYSTEM.TYPE_CLASS` (CODESYS) | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 ENUM) | `primitive.rs:118` | OK | `DWORD`-based enum |
| 25 | 8 (309) | `__SYSTEM.MEMORY_AREA` (CODESYS) | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 ENUM) | `primitive.rs:118` | OK | `INT`-based enum |
| 26 | 8 (310) | `__SYSTEM.VAR_INFO` | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, Y/N | SP22, 4026 | `overview.md:248` | `primitive.rs:118` | R07 | Compiler-defined structure |
| 27 | 8 (318) | CODESYS `SysTime` | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, N/N | SP22 | `overview.md:248` | `primitive.rs:118` | R07 | The vendor file gives no base declaration |
| 28 | 8 (319) | CODESYS `RTS_SYSTIMEDATE` | EXCLUDED-BY-DECISION (impl. A3) | none | Y/N, N/N | SP22 | `overview.md:248` | `primitive.rs:118` | R07, OQ05 | Library struct, nine `UINT` |
| 29 | 8 (320) | `Tc2_Utilities.T_FILETIME` | EXCLUDED-BY-DECISION (impl. A3) | none | N/N, Y/N | 4026 | `overview.md:248` | `primitive.rs:118` | R07, OQ05 | Library struct, two `DWORD` |
| 30 | 8 (321) | `Tc2_Utilities.T_FILETIME64` | EXCLUDED-BY-DECISION | none | N/N, Y/N | 4026 | `overview.md:284` (A39 alias of `ULINT`) | `primitive.rs:118` | OK | A `ULINT` parameter holds the count |
| 31 | 8 (322) | `Tc2_EtherCAT.T_DCTIME64` | EXCLUDED-BY-DECISION | none | N/N, Y/N | 4026 | `overview.md:284` (A39 alias of `ULINT`) | `primitive.rs:118` | OK | as row 30 |
| 32 | 8 (323) | `Tc2_Utilities.TIMESTRUCT` | EXCLUDED-BY-DECISION (impl. A3) | none | N/N, Y/N | 4026 | `overview.md:248` | `primitive.rs:118` | R07, OQ05 | Library struct, eight `WORD` |
| 33 | 8 (324) | `Tc2_System.T_AmsNetID` | EXCLUDED-BY-DECISION | none | N/N, Y/N | 4026 | `overview.md:284` (A39 alias of `STRING(23)`) | `primitive.rs:118` | OK | Group S owns `STRING(23)` |
| 34 | 8 (325) | `Tc2_System.ST_AmsAddr` | EXCLUDED-BY-DECISION (impl. A3) | none | N/N, Y/N | 4026 | `overview.md:248` | `primitive.rs:118` | R07, OQ05 | Library struct |
| 35 | 8 (344) | `CAA.PVOID`, `CAA.HANDLE` | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK | Base `__XWORD` (row 19) |
| 36 | 8 (345) | `CAA.COUNT`, `CAA.SIZE`, `CAA.TICK` | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK | Base `__UXINT` (row 18) |
| 37 | 8 (346) | `CAA.IDENT` | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK | Base `DWORD` (row 5) |
| 38 | 8 (347) | `CAA.SIGNED`, `CAA.UNSIGNED` | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK | Bases `INT`, `UINT` |
| 39 | 8 (348) | `CAA.FLOAT` | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK, OQ01 | Base `LREAL` |
| 40 | 8 (349) | `CAA.DURATION` | EXCLUDED-BY-DECISION | none | Y/N, N/N | SP22 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK | Base `TIME` (group T) |
| 41 | 8 (350) | `CAA.FILENAME`, `Tc2_System.T_MaxString` | EXCLUDED-BY-DECISION | none | Y/N, Y/N | SP22, 4026 | `overview.md:284` (A39 alias) | `primitive.rs:118` | OK | Base `STRING(255)` (group S) |
| 42 | 8 (360) | `SAFEBOOL` (CODESYS Safety Extension) | EXCLUDED-BY-DECISION (impl. A3) | none | Y (Safety profile)/N, N/N | SP22 with Safety Extension | `overview.md:248`. A50 (`overview.md:295`) is S7 only | `platform.rs` (no safety mark) | R07 | |
| 43 | 8 (361) | `SAFEINT`, `SAFEDINT` | EXCLUDED-BY-DECISION (impl. A3) | none | Y (Safety)/N, N/N | as row 42 | as row 42 | as row 42 | R07 | |
| 44 | 8 (362) | `SAFEWORD` | EXCLUDED-BY-DECISION (impl. A3) | none | Y (Safety)/N, N/N | as row 42 | as row 42 | as row 42 | R07 | |
| 45 | 8 (363) | `SAFETIME` | EXCLUDED-BY-DECISION (impl. A3) | none | Y (Safety)/N, N/N | as row 42 | as row 42 | as row 42 | R07 | |
| 46 | 8 (364) | `SAFEBYTE`, `SAFEDWORD` | EXCLUDED-BY-DECISION (impl. A3) | none | Y (Safety)/N, N/N | as row 42 | as row 42 | as row 42 | R07 | |

Count: 46 rows. SUPPORTED 15. EXCLUDED-BY-DECISION 31: 14 with a named decision (A39 or `data-model.md:2330`) and 17 by implication of A3 only. GAP 0.

Counting rules:

- One row per vendor table row. A vendor row with two or three names (rows 35, 36, 38, 41, 43, 46) stays one row. The names share one base and one decision.
- A `CAA.*` or `Tc2_*` name with a scalar or string base is an alias (A39). A library name with members is a struct and gets "impl. A3". A user can copy such a struct as a UDT, but the editor has no seed row for it.

**Constructs without a row of group N**

| Spelling (line) | Row | Reason |
|---|---|---|
| `XINT`, `UXINT`, `XWORD` (`vendor:264`) | 17, 18, 19 | TwinCAT spellings of the native integers |
| `WORD#16#8001`, `DWORD#16#80000001`, `LWORD#16#8000000000000001` (`vendor:53` to `:55`), `DWORD#16#12345678` (`:33`), mask `16#10` (`:57`) | 4, 5, 6 | Typed and based literals. ADR-0183 (b) refuses them (dimension e) |
| `INT#-1`, `UINT#65535` (`vendor:80`) | 9, 10 | Typed literals. Refused as above |
| `xReady AT %IX0.3 : BOOL` and the areas `%I`, `%Q`, `%M` (`vendor:59`) | 1 | `AT` mapping, not a type. A31 forbids an address on these platforms (`overview.md:276`) |
| IEEE classes: ±0, subnormal, ±infinity, NaN (`vendor:95` to `:99`) | 15, 16 | Value classes of `REAL` and `LREAL` (dimension d) |
| `T_AmsNetIdArr`, `T_AmsPort` (`vendor:325`) | 34 | Member types of `ST_AmsAddr` |
| `VAR_INFO` fields `ByteOffset`, `BitSize`, `NumElements`, `ElemBitSize`, `BitAddress`, `Area` (`vendor:309`, `:310`) | 26 | Members of a compiler structure |
| Safety Extension `BOOL` as one logical bit (`vendor:366`) | 1, 42 | A form of `BOOL` in the Safety profile. The editor models no Safety profile (R07) |
| `ADST_BIGTYPE`, `ADST_STRING`, `ADST_WSTRING`, `ADST_REAL80` (`vendor:370`) | none | ADS metadata ids, not PLC types. The vendor says `ADST_REAL80` adds no 80-bit `REAL` |
| TwinSAFE (`vendor:366`) | none | A separate Beckhoff product, named only. Covered by R07 together with rows 42 to 46 |

#### Group T: time and date types (§4)

Profiles: CODESYS V3.5 SP22 and TwinCAT 3.1.4026. Build 4024 is the compatibility baseline. Vendor line 29 marks one version difference: on TwinCAT, `LDATE`, `LDT`, and `LTOD` need 4026.

Layout model (dimension c): the editor models no layout on these two platforms. A31 (`overview.md:276`) and `domain-model.md:217` say that a CODESYS or TwinCAT 3 parameter has no address. The type page hides the S7 table on these platforms (`crates/recipe-web/src/pages/types/tree.rs:206`). `plc_type.size_bytes` holds the S7 size (`domain-model.md:88`, `data-model.md:2286`). The export schema has no size field (`grep size crates/recipe-export/schema/*.json`: no match). So dimension c is "no layout by decision (A31)" on every row. R06 covers one wrong sentence about IEC sizes.

| # | Vendor § (line) | Type (aliases) | Category | plc_type name | CodesysIec / TwinCat3 (vendor mark / editor mark) | Profile | Document cite | Code path | Verdict | Note |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | §4 (119) | `TIME` | SUPPORTED | `TIME` | Y/Y, Y/Y | SP22, 4026. 4024 is the same | `data-model.md:2309`, `domain-model.md:247` | `0004_seed_plc_types.sql:26`, `range.rs:12` | OK (R09) | Unsigned 32-bit ms on both sides |
| 2 | §4 (120) | `LTIME` | SUPPORTED | `LTIME` | Y/Y, Y/Y | SP22, 4026. 4024 also has `LTIME` (vendor line 29) | `data-model.md:2310`, `domain-model.md:248` | `0004_seed_plc_types.sql:27`, `range.rs:16` | OK (R09, OQ07) | Unsigned 64-bit ns. The vendor literal `LTIME#` is refused (OQ07) |
| 3 | §4 (121) | `DATE` (literal `D#`) | SUPPORTED | `DATE` | Y/Y, Y/Y | SP22, 4026. 4024 is the same | `data-model.md:2311`, `:2325`, `domain-model.md:250`, ADR-0026 line 44 | `0004_seed_plc_types.sql:28`, `range.rs:22` | OK (R06, R09) | The editor counts days (A43). The PLC counts seconds. The seed size is the S7 size of 2 bytes |
| 4 | §4 (122) | `DATE_AND_TIME` (`DT`) | SUPPORTED | `DATE_AND_TIME` | Y/Y, Y/Y | SP22, 4026. 4024 is the same | `data-model.md:2315`, `domain-model.md:254`, ADR-0026 line 47 | `0004_seed_plc_types.sql:32`, `range.rs:44` | R06 (R09) | Seed size is 8 bytes (S7). Vendor size is 4 bytes. No document notes this |
| 5 | §4 (123) | `TIME_OF_DAY` (`TOD`) | SUPPORTED | `TIME_OF_DAY` | Y/Y, Y/Y | SP22, 4026. 4024 is the same | `data-model.md:2313`, `domain-model.md:252` | `0004_seed_plc_types.sql:30`, `range.rs:38` | OK | `TOD` is literal syntax only (`data-model.md:2322`) |
| 6 | §4 (124) | `LDATE` (literal `LD#`) | SUPPORTED | `LDATE` | Y (SP16 or later)/Y, Y (4026 or later, N on 4024)/Y | SP22, 4026. Version difference on TwinCAT 4024 | `data-model.md:2312`, `:2324`, `domain-model.md:251`, ADR-0139 (b), (c) | `0004_seed_plc_types.sql:29`, `range.rs:32` | OK by decision (OQ04) | Editor range 1970-01-01 to 2262-04-11. Vendor range begins 1677-09-22. ADR-0139 (c) narrows it |
| 7 | §4 (125) | `LDATE_AND_TIME` (`LDT`) | SUPPORTED | `LDT` | Y (SP16 or later)/Y, Y (4026 or later, N on 4024)/Y | SP22, 4026. Version difference on TwinCAT 4024 | `data-model.md:2316`, `:2322`, `:2324`, `domain-model.md:255`, ADR-0139 (a), (c) | `0004_seed_plc_types.sql:33`, `range.rs:48` | OK by decision (OQ04) | Editor range 0 to 2^63-1 ns. Vendor range begins at -2^63 ns (1677-09-21) |
| 8 | §4 (126) | `LTIME_OF_DAY` (`LTOD`) | SUPPORTED | `LTOD` | Y (SP16 or later)/Y, Y (4026 or later, N on 4024)/Y | SP22, 4026. Version difference on TwinCAT 4024 | `data-model.md:2314`, `:2322`, `:2324`, `domain-model.md:253`, ADR-0139 (h) | `0004_seed_plc_types.sql:31`, `range.rs:40` | OK (OQ04) | Below one day on both sides |

Counts: SUPPORTED 8, EXCLUDED-BY-DECISION 0, GAP 0. Rows 6 and 7 have a decided partial range narrowing (ADR-0139 (c)). It is not a row exclusion.

Seed to vendor file: the editor offers exactly these eight time types on `CodesysIec` and `TwinCat3` (`0004_seed_plc_types.sql:26-33`). `S5TIME` and `DTL` are not offered on these platforms (`0004_seed_plc_types.sql:34-35`, `range.rs:82`, `:96`). Tests: `s5time_range_exists_on_s7_1500_and_s7_300_only` (`range/tests.rs:118`), `dtl_range_per_platform` (`range/tests.rs:399`), `literals_of_a_missing_type_are_not_on_the_platform` (`range/tests.rs:507`). The vendor file has neither type for these platforms. This agrees.

Paths: `range.rs`, `mod.rs`, `instant.rs`, `duration.rs`, and `canonical.rs` are in `crates/recipe-domain/src/plant/iec_literal/`. `range/tests.rs`, `instant/tests.rs`, `duration/tests.rs`, `canonical/tests.rs`, and `prop.rs` are below that directory.

**Constructs without a row of group T**

- `DT`, `TOD`, `LDT`, `LTOD`, `LDATE_AND_TIME`, `LTIME_OF_DAY` (vendor lines 122-126, 143) are the alias spellings of rows 4, 5, 7, and 8. The editor has one name per type (`data-model.md:2322`). `Primitive::parse` refuses `TOD`, `DT`, and `LTIME_OF_DAY` (`primitive/tests.rs:88`). A struct cannot take one of these names: `type_name.rs:55` (`DT`), `:108` (`LDATE_AND_TIME`), `:114` (`LTIME_OF_DAY`), `:176` (`TOD`). Test: `is_reserved_name_covers_keywords_and_primitives_in_any_case` (`type_name/tests.rs:143`). The editor names `LDT` and `LTOD` are valid CODESYS and TwinCAT type names (vendor lines 125-126). So the export type reference is valid.
- The literal prefixes `T#`, `LTIME#`, `D#`, `DT#`, `TOD#`, `LD#`, `LDT#`, `LTOD#` (vendor lines 134-143) are the literal syntax of the rows. Dimension e covers them. OQ07 covers `LTIME#`.
- `LWORD` and `ULINT` as names of the 64-bit container of the long calendar types (vendor line 130) are documentation names, not declared types. Group N owns those rows.
- The example table (vendor lines 132-141) gives examples, not types.
- Cross-group pointer (group A): ADR-0143 (l) lets a member name be a keyword, for example `TOD` or `DT`. CODESYS refuses a keyword as an identifier. I did not review this point.

#### Group S: characters and strings (§5)

Profiles: CODESYS V3.5 SP22 and TwinCAT 3.1.4026. The baseline is 4024 (`vendor:4`). §5 has no version difference for a type. The only version differences in §5 are literal prefixes (`UTF8#`, `UCHAR#`). The editor does not take literals for text leaves (see "Constructs without a row").

Layout model (dimension c): the editor models no layout on `CodesysIec` and `TwinCat3`. A31 (`docs/architecture/overview.md:276`): "A CODESYS or TwinCAT 3 parameter has no address". `domain-model.md:88`: `size_bytes` is the "S7 size of one element in bytes. ... `STRING(n)` is `n + 2`. `WSTRING(n)` is `2n + 4`". The S7 layout table of the types page is skipped off S7 (`crates/recipe-web/src/pages/types/tree.rs:206`). Thus a missing IEC layout is not a GAP. See R06 for one wrong sentence.

| # | Vendor § (line) | Type (aliases) | Category | plc_type name | CodesysIec / TwinCat3 (vendor mark / editor mark) | Profile | Document cite | Code path | Verdict | Note |
|---|---|---|---|---|---|---|---|---|---|---|
| S-1 | §5 (`vendor:151`, 156, 160-163, 167) | `STRING(n)` | SUPPORTED | `STRING` (machine row from `EnsureDerivedType`) | yes / yes (vendor), yes / yes (editor, `1..=255`) | SP22, 4026. 4024 same | `domain-model.md:106-114`, `:135`, `:140`, `data-model.md:2305`, `:2327`, ADR-0137 (b), (h), ADR-0141 (a), ADR-0181 (b) | `plant/string_length.rs:96-106`, `recipe-app/src/plant/ensure_derived.rs:165-185`, `recipe/value/leaf.rs:115-143` | OQ03, R04, R06 | Max 255 is a recorded narrowing (ADR-0141). Encoding is Latin-1 only: OQ03 |
| S-2 | §5 (`vendor:152`) | `STRING` (= `STRING(80)`) | SUPPORTED | `STRING` seed row `5eed...1e` | yes / yes, yes / yes | SP22, 4026. 4024 same | `data-model.md:2327`, `domain-model.md:150` | `migrations/0004_seed_plc_types.sql:36`, `string_length.rs:70-78`, `ensure_derived.rs:160-164` | R06, OQ03 | Default 80 equals vendor. `size_bytes` 82 is the S7 formula, vendor reserves 81: R06 |
| S-3 | §5 (`vendor:153`, 156, 164-165, 169) | `WSTRING(n)` | SUPPORTED | `WSTRING` (machine row) | yes / yes, yes / yes (`1..=255`) | SP22, 4026. 4024 same | `domain-model.md:106-114`, `:241`, ADR-0137 (b), (i), ADR-0141 (a), ADR-0180 (c) | `string_length.rs:96-99`, `leaf.rs:115-155` | OQ02, R05, R06, R10 | Editor accepts non-BMP characters as surrogate pairs. Vendor says UCS-2: OQ02 |
| S-4 | §5 (`vendor:154`) | `WSTRING` (= `WSTRING(80)`) | SUPPORTED | `WSTRING` seed row `5eed...1f` | yes / yes, yes / yes | SP22, 4026. 4024 same | `data-model.md:2328`, `domain-model.md:150` | `0004_seed_plc_types.sql:37`, `string_length.rs:70-73` | OQ02, R06, R10 | Default 80 equals vendor. `size_bytes` 164 against vendor 162: R06 |
| S-5 | §5 (`vendor:167`) | `STRING` with UTF-8 encoding (CODESYS option "UTF8 Encoding for STRING", TwinCAT `{attribute 'TcEncoding' := 'UTF-8'}`) | EXCLUDED-BY-DECISION | none (same `STRING` rows, Latin-1 rule) | documented / documented (vendor), no / no (editor) | SP22, 4026. `vendor:167` gives build 4026 for the `UTF8#` prefix only and names no build for the attribute, so 4024 is not proved | ADR-0137 (h) `:61`, ADR-0181 (a), (b) `:42-43`, `api.md:533` | `leaf.rs:11`, `:130-135` | OQ03 | The decision text has no platform limit. Its Context cites only Siemens facts (ADR-0181 `:13-16`) |
| S-6 | §5 (`vendor:171`) | `CHAR` | EXCLUDED-BY-DECISION | `CHAR` (S7 rows only) | not listed / not listed (vendor), no / no (editor) | SP22, 4026 | `data-model.md:2307` (matrix "no" for both, Assumption A3) | `0004_seed_plc_types.sql:24` | OK | Vendor: "Neither vendor's standard-type inventory lists standalone `CHAR` or `WCHAR`". Agrees |
| S-7 | §5 (`vendor:171`) | `WCHAR` | EXCLUDED-BY-DECISION | `WCHAR` (S7 rows only) | not listed / not listed, no / no | SP22, 4026 | `data-model.md:2308` | `0004_seed_plc_types.sql:25` | OK | Agrees with vendor |

Count: SUPPORTED 4, EXCLUDED-BY-DECISION 3, GAP 0.

**Constructs without a row of group S**

| Spelling (vendor line) | Reason |
|---|---|
| `UTF8#'…'` literal (`vendor:30`, `:167`) | Literal syntax, not a type. CODESYS compiler 3.5.18.0 or later. TwinCAT 4026 or later, not 4024. The editor takes the text of a `String` leaf as the string itself (`domain-model.md:237` "The text of a `String` or `Char` leaf is the string"). No version difference reaches the editor |
| `UCHAR#'Ł'` literal, base type `UDINT` (`vendor:171`) | Literal of a `UDINT`, not a type. TwinCAT from 4026. Belongs to the integer rows of group N. The editor has no `UCHAR#` form |
| String indexing `s[0]` (`BYTE`), `ws[0]` (`WORD`) (`vendor:169`) | Operator, not a type |
| `STRING(1)` = 2 bytes, `WSTRING(1)` = 4 bytes (`vendor:169`) | Size facts of rows S-1 and S-3 |
| Character escapes (`$N`, `$L`, `$'`, `$$`, `$hh`) | Not in the vendor file: `grep -n '\$' CODESYS_TwinCAT3_PLC_Data_Types_EN.md` gives no line. The editor takes raw text, so no escape grammar exists (`domain-model.md:237`) |
| `STRING[n]`, `WSTRING[n]` | Not in the vendor file: `grep -n 'STRING\['` gives no line. ADR-0183 (a) (`docs/adr/0183-string-notation-and-decimal-text-only.md:42`) refuses it on every platform. Test `type_expr/tests.rs:437` |

#### Group A: arrays, user-defined types, indirect and generic types (§6, §7)

Profile: every row is judged against CODESYS V3.5 SP22 and TwinCAT 3.1.4026 (A46, `overview.md:291`). Build 4024 is the compatibility baseline. In these sections only two points depend on the version. TwinCAT 4026 lets function blocks use generic inputs (vendor:291). TwinCAT 4026 also adjusts pointers and references at an online change (vendor:295). Both apply to rows that are excluded, so no SUPPORTED row hides a version difference.

Counting rule: the three SUPPORTED rows have no seed row in `0004_seed_plc_types.sql`. An array or a struct is a derived `plc_type` row of one machine, with `platforms = ARRAY[machine.plc_platform]` (`domain-model.md:87`). `EnsureDerivedType` and `DefineUdt` create these rows. The "rules" of these rows are the `TypeExpr` grammar, `ArrayDimension`, `PlcType::check_struct`, and the CHECKs of `0001_schema.sql`.

| # | Vendor § (line) | Type (aliases) | Category | plc_type name | CodesysIec / TwinCat3 (vendor mark / editor mark) | Profile | Document cite | Code path | Verdict | Note |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | §6 (179 to 198) | `ARRAY[L..U(, L..U){0,2}] OF T`, a fixed array with 1 to 3 dimensions | SUPPORTED | `kind = 'array'` row of the machine | yes/yes, yes/yes | SP22, 4026 (also 4024) | R13 `overview.md:176`, A38 `overview.md:283`, `domain-model.md:92`, `:137` to `:148`, `:443` | `type_expr.rs:149`, `array_dimension.rs:48`, `0001_schema.sql:72` to `:80`, `0011_plc_type_rules.sql:56` | R11, OQ09 | A struct or string element is a form of this row |
| 2 | §6 (198) | `ARRAY[..] OF ARRAY[..] OF T` (nested array) | SUPPORTED | outer and inner `kind = 'array'` rows | yes/yes, yes/yes | SP22, 4026 | `domain-model.md:151`, ADR-0177 (b) `docs/adr/0177-no-array-of-an-array-on-s7-300.md:45` | `type_expr.rs:91` to `:92` (S7 only) | OK | ADR-0148 and ADR-0177 (a) apply to the S7 platforms only |
| 3 | §6 (187) | `ARRAY[*] OF T`, `ARRAY[*,*] OF INT` (open array, `VAR_IN_OUT` only) | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | grammar `bound := -?[0-9]{1,10}` `domain-model.md:139`. `data-model.md:2336` names `ARRAY[*]` under "Siemens families" (ADR-0185) only | `type_expr.rs:200`, `lexical.rs:31` to `:35` | R07, R02 | Refused as a grammar error. No test (R02) |
| 4 | §6 (202 to 238) | `STRUCT ... END_STRUCT` DUT (`TYPE ST_x : STRUCT`) | SUPPORTED | `kind = 'struct'` row with `name` and `revision` | yes/yes, yes/yes | SP22, 4026 | R13 `overview.md:176`, `domain-model.md:85`, `:93`, `:119` to `:128`, ADR-0143, ADR-0146 | `plc_type.rs`, `type_name.rs:14`, `:201`, `0001_schema.sql:110`, `:112`, `:185`, `0011_plc_type_rules.sql:69` to `:75` | OQ09, OQ05 | No layout model (OQ09) |
| 5 | §6 (204) | Struct extension `EXTENDS` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | A3 `overview.md:248` (closed list), A39 `overview.md:284` | `type_name.rs` (keyword `EXTENDS` reserved as a struct name) | R07 | The editor struct has no base type |
| 6 | §6 (244) | Alias `TYPE T_BatchId : UDINT; END_TYPE` | EXCLUDED-BY-DECISION | none | yes/no, yes/no | SP22, 4026 | A39 `overview.md:284`, `domain-model.md:128`, `data-model.md:2330` | `0001_schema.sql:110` (`kind IN ('primitive','array','struct')`) | OK | |
| 7 | §6 (245) | Subrange `UINT (0..1000)` | EXCLUDED-BY-DECISION | none | yes/no, yes/no | SP22, 4026 | A39, `domain-model.md:128`, `data-model.md:2330` | `0001_schema.sql:110` | OK | A `ProcessLimit` can hold the same bounds on the base type |
| 8 | §6 (246, 249) | Enumeration with default `INT` base or explicit base, `{attribute 'strict'}` | EXCLUDED-BY-DECISION | none | yes/no, yes/no | SP22, 4026 | A39, `domain-model.md:128`, `data-model.md:2330` | `primitive/tests.rs:88` ("ENUM" unknown), `recipe-db/tests/storage/constraints/types.rs:230` (`kind = 'enum'` refused) | OK | |
| 9 | §6 (247, 249) | `UNION` DUT | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | A3 `overview.md:248`. Not named in `data-model.md:2330` | `0001_schema.sql:110`. `UNION` is not a reserved struct name (`type_name.rs:14`) | R07, OQ05 | |
| 10 | §7 (272) | `POINTER TO T` | EXCLUDED-BY-DECISION | none | yes/no, yes/no | SP22, 4026 | `data-model.md:2330` (`POINTER`) | `type_name.rs:14` (`POINTER` reserved) | OK (R02 test gap) | |
| 11 | §7 (273) | `REFERENCE TO T` | EXCLUDED-BY-DECISION | none | yes/no, yes/no | SP22, 4026 | `data-model.md:2330` (`REFERENCE`) | `type_name.rs:14` (`REFERENCE`, `REF`, `REF_TO` reserved) | OK (R02 test gap) | 4026 online-change adjustment (vendor:295) is n/a |
| 12 | §7 (275) | `INTERFACE` variable | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | A3, A39 ("A parameter uses a primitive, an array, or a UDT") | `type_name.rs:14` (`INTERFACE` reserved) | R07 | |
| 13 | §7 (276) | `FUNCTION_BLOCK` instance (example `TON`) | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | A3, A39. ADR-0185 names Siemens multi-instances only (`data-model.md:2332`) | `type_name.rs:14` (`FUNCTION_BLOCK` reserved). `TON` is not reserved | R07 | |
| 14 | §7 (277, 283) | `ANY` | EXCLUDED-BY-DECISION | none | yes/no, yes/no | SP22, 4026 | `data-model.md:2330` (`ANY`) | `type_name.rs:14` (`ANY` reserved) | OK | |
| 15 | §7 (284) | `ANY_BIT` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | A3, A39. `data-model.md:2330` names `ANY` only | `type_name.rs:14` (reserved) | R07 | A formal input type, never a variable type (vendor:291) |
| 16 | §7 (285) | `ANY_INT` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | as row 15 | as row 15 | R07 | |
| 17 | §7 (286) | `ANY_REAL` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | as row 15 | as row 15 | R07 | |
| 18 | §7 (287) | `ANY_NUM` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | as row 15 | as row 15 | R07 | |
| 19 | §7 (288) | `ANY_DATE` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | as row 15 | as row 15 | R07 | |
| 20 | §7 (289) | `ANY_STRING` | EXCLUDED-BY-DECISION (implicit) | none | yes/no, yes/no | SP22, 4026 | as row 15 | as row 15 | R07 | |

Count: 20 rows. SUPPORTED 3. EXCLUDED-BY-DECISION 6 with a named decision (rows 6, 7, 8, 10, 11, 14). EXCLUDED-BY-DECISION (implicit) 11 (rows 3, 5, 9, 12, 13, 15 to 20). GAP 0.

Seed to vendor direction: on these platforms the editor offers only `kind = 'array'` and `kind = 'struct'` for my sections (`0001_schema.sql:110`, export `types[].kind` enum `export-v2.schema.json:234`). Both appear in the vendor file. No offered construct contradicts §6 or §7.

**Constructs without a row of group A**

- `ARRAY[1..3] OF ST_Sample` (vendor:186) and `ARRAY[1..3] OF STRING(4)` (vendor:185). These are forms of row 1 with a struct or string element. The string slot size belongs to the strings group.
- The vendor initializer tables of `ARRAY[1..2,3..4,5..6] OF INT` (vendor:189 to vendor:196). This is the initializer order of row 1, not a type. Finding R11 covers the effect on the JSON form.
- `{attribute 'pack_mode' := 'n'}` and the CODESYS *Compatibility layout* setting (vendor:217 to vendor:236). These are layout pragmas, not types. Dimension c of row 4 and OQ09 cover them.
- `VAR_IN_OUT` (vendor:274). It is a parameter-passing convention, not a type.
- The call descriptor `AnyType` with `typeclass`, `pvalue`, `diSize` (vendor:277, vendor:293). It is the runtime form of row 14 and rows 15 to 20.
- `ANY_DURATION` (vendor:291). The vendor file says that it is unimplemented and absent from both supported generic-input lists. The editor reserves the name (`type_name.rs:14`).
- `{attribute 'strict'}` (vendor:249). It is a form of row 8.
- `CheckRangeUnsigned` (vendor:249). It is a check function of the PLC, not a type.
- Operators `ADR`, `p^`, `REF=`, `LOWER_BOUND`, `UPPER_BOUND`, `SIZEOF`, `XSIZEOF` (vendor:187, vendor:272, vendor:273, vendor:31). These are operators. OQ05 asks about the unreserved ones.
- The packed `BIT` (vendor:284) belongs to the §2 group.
- The `[*]` segment of `LeafPath` (`domain-model.md:182`, `:187`) means "every element" in a limit path. It is not the open array of row 3.
- §9 (vendor:374 to vendor:434) is a probe procedure. It gives no type spelling of its own. Group A uses it as the reference for dimension c.

#### Named decision or closed list

Of the 51 EXCLUDED-BY-DECISION rows, 21 cite a named decision and 30 rely on the closed catalog (A3) only.

| Group | Excluded rows | Named decision | Closed catalog only (A3) |
|---|---:|---:|---:|
| N | 31 | 14 (A39 or `data-model.md:2330`) | 17 |
| S | 3 | 1 (row S-5: ADR-0137 (h), ADR-0181 (b)) | 2 (rows S-6 and S-7: the matrix of `data-model.md:2307` and `:2308`, A3) |
| A | 17 | 6 (rows 6, 7, 8, 10, 11, 14) | 11 |
| T | 0 | 0 | 0 |
| **Total** | **51** | **21** | **30** |

Section 3 (R07) covers the 30 rows without a named exclusion.

### 2.2 Dimension detail of the SUPPORTED rows

Each SUPPORTED row covers the dimensions a to h:

- a. name and aliases
- b. availability per platform and per version profile
- c. size, alignment, padding, byte order (the editor models no layout on these platforms)
- d. value range and limits
- e. literal grammar
- f. canonical JSONB storage and export representation
- g. string length and encoding, array bounds and nesting, struct and DUT names
- h. tests

The ids in this section are the report ids. Group X uses evidence keys. The Codex path abbreviations are expanded to repository paths.

### Group N

#### Shared evidence of the 15 SUPPORTED rows

- a. Names. `crates/recipe-domain/src/plant/primitive.rs:164-180` returns the names. `primitive.rs:150-160` parses the exact upper-case name. `plant/type_expr.rs:215` uses that parse. The vendor sections write every name in upper case. No vendor alias exists for these 15 types in §2 and §3. Test: `plant/primitive/tests.rs:53` `every_name_matches_the_matrix`.
- b. Availability. Seed rows `migrations/0004_seed_plc_types.sql:7-21` hold `CodesysIec` and `TwinCat3` for all 15 rows. This equals `data-model.md:2290-2304` and the vendor inventories. Test: `crates/recipe-db/tests/storage/seed.rs:365` `each_seed_row_equals_its_row_of_the_matrix` (constants `ALL`, `NO_S7_300`, `CT_1500` at `seed.rs:17-19`). This test needs Docker and was not run. No negative availability case exists, because all 15 types are available on both platforms.
- c. Layout. The editor models no layout on these platforms. A31 (`overview.md:276`): "A CODESYS or TwinCAT 3 parameter has no address". `data-model.md:2329` applies the S7 layout rules "for every parameter with an address". The export carries no size, offset, or byte order. `api.md:533` writes each numeric leaf as canonical decimal text. Thus the byte order of `vendor:26` and the alignment of `vendor:28` are a concern of the transfer client, by decision. `size_bytes` of the seed (`0004:7-21`) equals the vendor size of each of the 15 types (`vendor:50`, `:52` to `:55`, `:69` to `:76`, `:88`, `:89`). Cross-group note at the end.
- d. Ranges. `crates/recipe-domain/src/recipe/value/numeric.rs:15-29` gives the 12 integer and bit-string ranges and the `Decimal` range of `REAL` and `LREAL`. `numeric()` at `recipe/value/leaf.rs:217` takes no platform, so one rule serves both platforms (`leaf.rs:90`).
- e. Literal. `domain-model.md:237`: "The text of a `Numeric` leaf is decimal text with a dot." ADR-0183 (b) (`docs/adr/0183-string-notation-and-decimal-text-only.md:43`) has no platform qualifier. Its context cites only TIA Portal, but the rule applies on every platform. Code: `recipe/value/texts.rs:150` (`is_json_number`). The group prompt calls ADR-0183 S7 only. That is not right for item (b).
- f. Storage and export. `reliability.md:512`: canonical decimal text, no exponent, no trailing zeros, `-0` becomes `0`. `api.md:533`: "Every numeric ... leaf is a JSON string with the canonical decimal text". Code `leaf.rs:244-248`. Schema pattern of limits `crates/recipe-export/schema/export-v1.schema.json:122`. `serde_json` has `arbitrary_precision` (`Cargo.toml:63`), so 64-bit ends keep all digits.
- g. n/a for all 15 rows.
- h. Test platform. The value fixtures use `PLATFORM = PlcPlatform::TwinCat3` (`crates/recipe-domain/src/plant/type_tree/fixtures.rs:13`, used by `types()` at `:200`). Thus every `primitive_tree` and `integer_tree` test is `TwinCat3` evidence. `CodesysIec` has no own numeric test. It is covered because `numeric()` reads no platform (`leaf.rs:90`, `:217`). This is not a finding by itself.

#### Per row

| Type | a name | b platform | c size, layout | d range | e literal | f storage, export | g | h tests |
|---|---|---|---|---|---|---|---|---|
| `BOOL` | OK | OK (`data-model.md:2290`) | OK: 1 byte as on the vendor (`vendor:50`). No layout (A31) | OK: `true`/`false` only. Bytes other than `00`/`01` (`vendor:50`) cannot occur | OK by decision: form text `true`/`false` only (`domain-model.md:237`, `texts.rs:157-160`) | OK: JSON boolean (`api.md:533`) | n/a | `texts_tests.rs:144` `bool_text_is_true_or_false` (refuses `TRUE`, `1`), `leaf/tests.rs:73`, `:81` |
| `BYTE` | OK | OK | OK: 1 B | OK: 0..255 | OK by decision: decimal only (ADR-0183 (b)) | OK: decimal text | n/a | `leaf/tests.rs:350`, `limit/parse/tests.rs:108`, `texts_tests.rs:596` |
| `WORD` | OK | OK | OK: 2 B | OK: 0..65535 | as `BYTE` | OK | n/a | as `BYTE` |
| `DWORD` | OK | OK | OK: 4 B | OK: 0..4294967295 | as `BYTE` | OK | n/a | as `BYTE` |
| `LWORD` | OK | OK | OK: 8 B | OK: 0..2^64-1 (`numeric.rs:24`) | as `BYTE` | OK: `"18446744073709551615"` only in S7 golden files. R09 | n/a | as `BYTE`. R09 |
| `SINT` | OK | OK | OK: 1 B | OK: -128..127 (`vendor:69`) | OK | OK | n/a | `leaf/tests.rs:350`, `limit/parse/tests.rs:108` (table `test_support.rs:125`) |
| `USINT` | OK | OK | OK | OK: 0..255 | OK | OK | n/a | as `SINT`, `texts_tests.rs:105` |
| `INT` | OK | OK | OK: 2 B | OK: -32768..32767 | OK | OK: `TwinCat3` export of `"120"` (`crates/recipe-export/tests/golden.rs:442-454`, `:467`) | n/a | as `SINT`, `texts_tests.rs:105`, `:119` |
| `UINT` | OK | OK | OK | OK | OK | OK | n/a | as `SINT` |
| `DINT` | OK | OK | OK: 4 B | OK | OK | OK | n/a | as `SINT` |
| `UDINT` | OK | OK | OK | OK | OK | OK | n/a | as `SINT` |
| `LINT` | OK | OK | OK: 8 B | OK: i64 | OK | OK, R09 | n/a | as `SINT`. R09 |
| `ULINT` | OK | OK | OK: 8 B | OK: u64 | OK | OK, R09 | n/a | as `SINT`. R09 |
| `REAL` | OK | OK | OK: 4 B | OK by decision: `Decimal` range (ADR-0115 (c) at `docs/adr/0115-ranges-narrower-than-the-vendor-range.md:43`), 7 digits (ADR-0117 (a) at `0117-...:42`, `leaf.rs:23`). Specials below | OK: JSON number grammar with exponent (`texts_tests.rs:87`). `REAL#1.0` refused (`texts_tests.rs:619`) | OK. `-0` becomes `0` (`leaf/tests.rs:346`) | n/a | `leaf/tests.rs:196`, `:204`, `:244`, `:263`, `:272`, `numeric/tests.rs:103`, `limit/parse/tests.rs:372`, `:454`, `:571`. R03 |
| `LREAL` | OK | OK on the matrix. OQ01 for targets without binary64 | OK: 8 B, see OQ01 | OK by decision: `Decimal` range, 15 digits (`leaf.rs:26`) | as `REAL` | as `REAL` | n/a | `leaf/tests.rs:250`, `:263`, `:292`, `limit/parse/tests.rs:417`, `:462`, `:480`. R03 |

Special values of `REAL` and `LREAL` (dimension d):

- NaN and ±infinity cannot enter. The form path needs the JSON number grammar (`texts.rs:150`). The JSON path parses through `parse_decimal`, which also needs that grammar (`decimal.rs:23`). No test names them (R03).
- `-0` becomes `0` (`reliability.md:512`). Tests: `leaf/tests.rs:346` (`-0e-99999999999999999999` on `REAL`), `texts_tests.rs:88` (`-0` on `INT`).
- No subnormal can enter. The smallest non-zero `Decimal` magnitude is `1e-28` (`leaf/tests.rs:204` refuses `1e-29`). That is above the smallest normal `REAL`, 2^-126 ≈ 1.18e-38 (`vendor:104`). The largest `Decimal` ≈ 7.9e28 is below 3.4e38 (`vendor:103`), test `numeric/tests.rs:103`.
- Precision. ADR-0117 (a) allows 7 digits for `REAL`. `vendor:88` gives a 24-bit significand, which always keeps only 6 decimal digits. ADR-0117 line 63 records this risk. The open Siemens owner question OQ08 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md:939`) covers it for every platform. This group raises no new question for it.

### Group T

#### Row 1: TIME (§4 line 119)

- a. OK. `TIME`, prefix `T#` (`mod.rs:145`, `canonical.rs:54`).
- b. OK. Both platforms (`0004_seed_plc_types.sql:26`). The vendor file gives no version limit.
- c. No layout by decision (A31). Vendor: 4 bytes. Seed `size_bytes` 4 (`primitive.rs:239`) agrees by chance.
- d. OK. `0..=4_294_967_295` ms (`range.rs:12`, `:78`) equals vendor line 119 ("0…4,294,967,295 ms"). The sign `-` parses and then fails the range check (`domain-model.md:281`, `range.rs:109-113`).
- e. OK. Units `d` to `ms` (`duration.rs:14-20`), at most 19 digits in one part (`duration.rs:8`). The vendor example `T#1s250ms` (line 134) fits the grammar (`domain-model.md:264-266`).
- f. OK. JSONB holds the count as a JSON number (`recipe/value/leaf.rs:186-211`). The export writes canonical decimal text in ms (`api.md:533`, `:541`, `reliability.md:507-511`). This equals the PLC counter.
- g. n/a.
- h. Count level: `time_range_per_platform` (`range/tests.rs:96`) checks both ends and one step outside each end on both platforms. Sign: `negative_time_depends_on_the_platform` (`range/tests.rs:421`), `negative_time_on_twincat3_gives_a_fault` (`recipe/value/leaf/tests.rs:170`). The literal at the upper end (`T#49d17h2m47s295ms`) has no test (R09).

#### Row 2: LTIME (§4 line 120)

- a. OK for `LT#`. The vendor writes `LTIME#1us` (line 135). The editor refuses it (OQ07).
- b. OK. Both platforms. TwinCAT 4024 also has `LTIME` (vendor line 29). The editor needs no version assumption for this type.
- c. No layout by decision (A31). 8 bytes on both sides (`primitive.rs:246`).
- d. OK. `0..=18_446_744_073_709_551_615` ns (`range.rs:16`, `:80`) equals vendor line 120 (0…2⁶⁴−1 ns).
- e. OK. Every unit `d` to `ns` (`duration.rs:26-34`). The upper end needs the canonical split form, because `2^64-1` has 20 digits and one part takes at most 19 (`duration/tests.rs:188`). The canonical rendering uses that split form (`canonical.rs:107-115`).
- f. OK. Nanoseconds (`api.md:542`). Test `ltime_literal_is_stored_as_a_count_of_nanoseconds` (`recipe/value/leaf/tests.rs:174`). The writer already emits the 20-digit text `"18446744073709551615"` for `ULINT` and `LWORD` (`golden/types_S7_1500.json:274`).
- g. n/a.
- h. Count level: `ltime_range_per_platform` (`range/tests.rs:102`) checks both ends and one step outside each end. Literal tests are on `S7_1500` only (`duration/tests.rs:3-5`, `:54`, `:59`). There is no IEC literal test at the upper end (R09).

#### Row 3: DATE (§4 line 121)

- a. OK. `DATE`, prefix `D#` (`mod.rs:148`).
- b. OK. Both platforms.
- c. No layout by decision (A31). Vendor: 4 bytes. Seed: 2 bytes, the S7 size (`0004_seed_plc_types.sql:28`, `primitive.rs:236`). `data-model.md:2325` notes this. R06 covers the wrong part of that note.
- d. OK. `0..=49_710` days (`range.rs:22`, `:83`) is 1970-01-01 to 2106-02-07 (`range/tests.rs:150-153`). Vendor line 121 gives the same days. Day 49 710 at midnight is 4 294 944 000 s. This is below 2^32-1, so every editor value fits the PLC counter.
- e. OK. `D#YYYY-MM-DD`, no fraction, two-digit fields (`instant.rs:146`, `:162-174`). The vendor example `D#1970-01-02` (line 136) fits.
- f. OK by decision. The count is days since 1970-01-01 (A43 `overview.md:288`, `api.md:543`). The PLC counter is seconds (vendor line 121). A day count holds only midnight values. The vendor file recommends this canonical form (line 128). The consumer multiplies by 86 400. `api.md:537` says that the unit comes from the type name. No document states the factor for the PLC. The Siemens report treats the 1990 offset of S7 the same way, so I raise no finding.
- g. n/a.
- h. Count level: `date_range_per_platform` (`range/tests.rs:144`), `date_numbers_match_the_calendar_dates_of_adr_0026` (`range/tests.rs:150`). The literal `D#2106-02-07` and the literal `D#2106-02-08` have no test (R09).

#### Row 4: DATE_AND_TIME / DT (§4 line 122)

- a. OK. Type name `DATE_AND_TIME`. `DT#` is literal syntax (`data-model.md:2322`, `mod.rs:152`).
- b. OK. Both platforms (`0004_seed_plc_types.sql:32`).
- c. No layout by decision (A31). Vendor: 4 bytes. Seed: 8 bytes, the S7 size (`primitive.rs:249`). No document notes this difference (R06).
- d. OK. `0..=4_294_967_295` s (`range.rs:44`, `:91`) is 1970-01-01-00:00:00 to 2106-02-07-06:28:15 (`range/tests.rs:251`). This equals vendor line 122. Whole seconds equal the native unit, so ADR-0115 narrows nothing on these platforms.
- e. OK. `DT#YYYY-MM-DD-HH:MM:SS`, no fraction (`instant.rs:119`). The vendor example `DT#1970-01-01-00:00:01` (line 137) fits. Test `fraction_on_date_and_time_is_a_fraction_error` (`instant/tests.rs:143`, platform `TwinCat3` per `instant/tests.rs:3-9`).
- f. OK. Seconds since 1970-01-01 (`api.md:547`). This equals the PLC counter.
- g. n/a.
- h. Count level: `date_and_time_range_per_platform` (`range/tests.rs:332`, IEC at `:340`). Positive literal: `date_and_time_counts_seconds_since_epoch` (`instant/tests.rs:55`). The literal at the upper end has no test (R09).

#### Row 5: TIME_OF_DAY / TOD (§4 line 123)

- a. OK. Type name `TIME_OF_DAY`. `TOD#` is literal syntax.
- b. OK. Both platforms.
- c. No layout by decision (A31). 4 bytes on both sides.
- d. OK. `0..=86_399_999` ms (`range.rs:38`, `:88`) equals vendor line 123. A count of one day or more is refused (`instant.rs:289-298`), as vendor line 128 asks.
- e. OK. At most 3 fraction digits (`instant.rs:117`). Hour `00..=23` (`instant.rs:226`). The vendor example `TOD#00:00:01.234` (line 138) fits.
- f. OK. Milliseconds since midnight (`api.md:545`).
- g. n/a.
- h. OK. `time_of_day_range_is_the_same_on_every_platform` (`range/tests.rs:298`) covers both IEC platforms. Literal tests on `TwinCat3`: `tod_half_second_counts_45000500_ms` (`instant/tests.rs:29`), `hour_twenty_four_is_a_calendar_error` (`:111`), `four_fraction_digits_on_tod_are_a_fraction_error` (`:135`), `tod_count_of_one_day_is_out_of_range` (`:199`).

#### Row 6: LDATE (§4 line 124)

- a. OK. `LDATE`, prefix `LD#` (`mod.rs:149`).
- b. OK by A46 for TwinCAT: 4026 or later (`overview.md:291`, `data-model.md:2324`, ADR-0142 (a)-(d), `docs/ops/runbook.md:1371`). The vendor file gives SP16 or later for CODESYS (line 29). No document names a CODESYS version (OQ04).
- c. No layout by decision (A31). 8 bytes on both sides.
- d. OK by decision. The editor range is 0 to 2^63-1 ns in whole days, 1970-01-01 to 2262-04-11 (`range.rs:32`, `:87`, ADR-0139 (b)). The vendor range is 1677-09-22 to 2262-04-11 (line 124). ADR-0139 (c) refuses 1677 to 1969 on purpose. The upper end agrees. The earlier rule set keeps 0 to 2^64-1 for stored data (`range.rs:36`, ADR-0139 (e)). Such data gives `loadable: false` in the export (ADR-0139 (g)).
- e. OK. `LD#YYYY-MM-DD` (`instant.rs:116`). The vendor example `LD#1970-01-02` (line 139) fits. A whole day is enforced for a stored count (`instant.rs:263-266`), as vendor line 128 asks.
- f. OK. Nanoseconds since 1970-01-01 (`api.md:544`). This equals the PLC counter for each non-negative value.
- g. n/a.
- h. OK. `ldate_range_per_platform` (`range/tests.rs:263`), `adr_0139_the_last_ldt_and_ldate_literals_pass_on_both_sets` (`:531`), `adr_0139_the_literal_after_the_last_ldt_and_ldate_is_out_of_range` (`:563`), `adr_0139_the_earlier_set_keeps_the_old_end_of_ldt_and_ldate` (`:595`). Whole day: `ldate_count_of_one_nanosecond_is_out_of_range` (`instant/tests.rs:191`), `stored_ldate_max_that_is_no_whole_day_is_outside_the_type_range_in_both_sets` (`recipe/limit/parse/tests.rs:603`). Value path: `set_value_stores_the_last_ldate_day_on_both_platforms` (`recipe/draft/earlier_ldt_tests.rs:92`), `set_value_with_the_day_after_the_last_ldate_gives_e_rcp_005` (`:128`).

#### Row 7: LDATE_AND_TIME / LDT (§4 line 125)

- a. OK. Type name `LDT`. `LDATE_AND_TIME` is a reserved struct name (`type_name.rs:108`).
- b. Same as row 6 (A46 for TwinCAT, OQ04 for CODESYS).
- c. No layout by decision (A31). 8 bytes on both sides.
- d. OK by decision. `0..=9_223_372_036_854_775_807` ns (`range.rs:48`, `:94`) is 1970-01-01-00:00:00 to 2262-04-11-23:47:16.854775807 (ADR-0139 (a)). The upper end equals vendor line 125. The lower end narrows the vendor range on purpose (ADR-0139 (c)). The vendor example `LDT#1969-12-31-23:59:59.999999999` (line 140) is therefore refused by decision.
- e. OK. Up to 9 fraction digits (`instant.rs:120`).
- f. OK. Nanoseconds since 1970-01-01 (`api.md:544`). A non-negative count has the same bits as the signed PLC counter.
- g. n/a.
- h. OK. `ldt_range_per_platform` (`range/tests.rs:346`) checks 0, -1, 2^63-1, 2^63, and the earlier ends on both platforms. Literal tests at `range/tests.rs:531`, `:563`. Export: `adr_0139_a_stored_ldt_after_the_current_end_on_twincat3_makes_the_version_not_loadable` (`export/rule_check/tests.rs:317`), `export_routes_of_an_ldt_after_the_current_range_on_twincat3_send_loadable_false` (`recipe-web/src/pages/export/platform_range_tests.rs:56`). Approval: `snapshot/tests.rs:509`, `:517`.

#### Row 8: LTIME_OF_DAY / LTOD (§4 line 126)

- a. OK. Type name `LTOD`. `LTIME_OF_DAY` is a reserved struct name (`type_name.rs:114`).
- b. Same as row 6 (A46 for TwinCAT, OQ04 for CODESYS).
- c. No layout by decision (A31). 8 bytes on both sides.
- d. OK. `0..=86_399_999_999_999` ns (`range.rs:40`, `:89`) equals vendor line 126 and ADR-0139 (h).
- e. OK. Up to 9 fraction digits (`instant.rs:118`). The vendor example `LTOD#00:00:00.000000001` (line 141) fits.
- f. OK. Nanoseconds since midnight (`api.md:546`).
- g. n/a.
- h. OK. `ltod_range_is_below_one_day_where_it_exists` (`range/tests.rs:316`) checks both ends and one step outside each end on both platforms. Also `ltod_last_nanosecond_of_the_day_counts_86399999999999` (`instant/tests.rs:79`) and `ten_fraction_digits_on_ltod_are_a_grammar_error` (`instant/tests.rs:151`).

Property tests cover every row on both platforms: `stored_count_survives_display_and_parse` (`prop.rs:92`) and `canonical_text_survives_parse_and_display` (`prop.rs:101`). `time_types()` (`prop.rs:29-39`) takes every platform of `PlcPlatform::ALL`. Random draws do not pin the ends.

### Group S

Test fixture platform: `crates/recipe-domain/src/plant/type_tree/fixtures.rs:13` `pub(crate) const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;`. Thus every `string_tree` and `wstring_tree` test of `recipe/value/leaf/tests.rs` runs on `TwinCat3`. No value test runs a `STRING` leaf on `CodesysIec`. Both platforms share each match arm (`leaf.rs:105-108`, `string_length.rs:98-99`).

#### S-1 `STRING(n)`

- a. OK. Name `STRING`, notation `STRING(n)` (`domain-model.md:135` `string := ("STRING" | "WSTRING") ["(" length ")"]`). The vendor writes `STRING(n)` (`vendor:151`). Parser `plant/type_expr.rs:221-247`. Uppercase only, length `[1-9][0-9]{0,4}` (`domain-model.md:140`).
- b. OK. Both platforms, every profile. No version difference (`vendor:151` has no version mark). `data-model.md:2305` yes / yes. Seed row `0004_seed_plc_types.sql:36` `ARRAY['CodesysIec', 'TwinCat3']`.
- c. Not modelled (A31). Vendor `n + 1` bytes with a zero terminator (`vendor:151`). The editor stores the S7 size `n + 2` (`string_length.rs:84-90`, `ensure_derived.rs:171-173`). The value is stored and checked (`plant/plc_type.rs:205-217`), never shown or exported off S7 (`pages/types/tree.rs:206`). R06.
- d. OK by decision. `1..=255` (`domain-model.md:110`, `string_length.rs:98-99`). ADR-0141 (a) `:44` "the length of a `STRING(n)` and of a `WSTRING(n)` stays `1..=255`". Vendor gives no type maximum and names 255 as a library limit (`vendor:167` "Older string library functions can impose their own processing limits, commonly 255"). This agrees with ADR-0141 (b) `:45`. DB permits `1..=16382` (`migrations/0001_schema.sql:122-123`). The domain is the one check (`data-model.md:2326`), as documented. Process limit `max_length <= string_length` (`reliability.md:503`), DB `1..=16382` (`0001_schema.sql:471`).
- e. n/a. JSON string, no literal (`domain-model.md:231`, `:237`).
- f. OK. Stored as the string in JSONB (`domain-model.md:231` "The string"). Export: `types[].string_length` (`crates/recipe-export/schema/export-v1.schema.json:213`, `minimum 1, maximum 16382` for every platform), value as a JSON string, Latin-1 bytes for the transfer client (`api.md:533`). OQ03.
- g. Unit: characters, one Latin-1 byte each (ADR-0137 (h) `:61`, `leaf.rs:149-155`). Repertoire `U+0001..=U+00FF` (ADR-0137 (b) `:55`, `leaf.rs:11`, `:130-138`). Vendor: Latin-1 or UTF-8, capacity in bytes (`vendor:161-163`, `:167`). OQ03. No `U+0000` (`leaf.rs:137`): equals the vendor terminator rule (`vendor:156` "Logical end: First `00`", ADR-0137 `:19`). Thus ADR-0137 is no narrowing on these platforms.
- h. Length: `string_length/tests.rs:11` `string_on_codesys_accepts_1_and_255_and_rejects_256`, `:21` `string_on_twincat3_accepts_255_and_rejects_256`, `:47` `zero_length_is_out_of_range_on_every_platform`, `type_expr/tests.rs:142` (`"STRING(0)"`, `"STRING(020)"` grammar errors), `ensure_derived/tests.rs:130` `the_first_sized_string_creates_one_row_and_one_entry` (TwinCat3, size 22). Value (TwinCat3): `leaf/tests.rs:25` refuses "Łódź" (U+0141), `:32` passes "Zéàü" (max U+00FC), `:37` and `:42` (length n+1 refused, n passes), `:47` (characters, not bytes), `:116` (NUL refused), `:132` (U+0001 passes). Limit: `limit/parse/tests.rs:189`, `:195`. Gap: no test at U+00FF / U+0100. R04.

#### S-2 `STRING` (default)

- a. OK. `STRING` without a length means the seed row (`domain-model.md:150`, `data-model.md:2327`).
- b. OK. Seed row `0004_seed_plc_types.sql:36`.
- c. Not modelled (A31). Vendor 81 bytes (`vendor:152`), seed `size_bytes` 82. R06.
- d. OK. Default 80 equals vendor `vendor:152` "Equivalent to `STRING(80)`". `string_length.rs:72-73` gives 80.
- e. n/a.
- f. OK. As S-1. The seed row and a machine `STRING(80)` row have one shape (ADR-0146 (c), `docs/adr/0146-shape-of-a-member-type-at-a-rebind.md:49`).
- g. As S-1.
- h. `string_length/tests.rs:101` `default_length_matches_the_seed_rows_for_all_ten_combinations`, `:124` `default_length_is_valid_on_its_platform`, `ensure_derived/tests.rs:188` `a_string_without_length_uses_the_seed_row_of_the_platform` (TwinCat3, seed 80), `recipe-db/tests/storage/seed.rs:273-280` (row `1e`: CT, 82, 80), `:328` `each_platform_has_exactly_one_global_string_row`, `:365` `each_seed_row_equals_its_row_of_the_matrix`. Shape: `recipe/rebind/kept_member_tests.rs:210`, `:230` (CodesysIec).

#### S-3 `WSTRING(n)`

- a. OK. `WSTRING(n)` (`domain-model.md:135`, `vendor:153`).
- b. OK. Both platforms (`data-model.md:2306`, `string_length.rs:98-99`).
- c. Not modelled (A31). Vendor `2 × (n + 1)` (`vendor:153`), editor S7 size `2n + 4` (`string_length.rs:88`). R06.
- d. OK by decision. `1..=255` (ADR-0141 (a), `string_length.rs:99`).
- e. n/a.
- f. JSON string. Golden file of a TwinCat3 `WSTRING` with U+1D11E, `loadable: true` (`crates/recipe-export/tests/golden_types.rs:562-596`). `api.md:533` names the bytes of `STRING` and `CHAR` only. R10. OQ02.
- g. Unit: UTF-16 code units, a non-BMP character counts 2 (ADR-0137 (i) `:62`, `leaf.rs:149-152`). Repertoire on CT: every scalar except `U+0000` (ADR-0137 (b) `:55`, ADR-0180 (c) `:51`, `leaf.rs:130-133` `(true, false) => u32::from(char::MAX)`). Vendor: UCS-2, `U+0000…U+D7FF` and `U+E000…U+FFFF`, supplementary characters need another representation (`vendor:169`). OQ02.
- h. Length: `string_length/tests.rs:66` `wstring_on_codesys_and_twincat_accepts_255_and_rejects_256`. No `WSTRING` length 0 test (R05). Value (TwinCat3): `leaf/tests.rs:57` `wstring_counts_utf16_units` (negative), `:63` `wstring_with_one_surrogate_pair_fits_two_units`, `:68` (Ł passes), `:124` (NUL refused), `:459` `adr_0180_wstring_on_codesys_and_twincat_keeps_adr_0137` (both CT platforms: U+D000 and 😀 pass). Read check: `recipe/read_check/tests.rs:695` `adr_0180_a_stored_wstring_with_u_d000_on_twincat3_gives_no_fault`. Rebind: `rebind/kept_member_tests.rs:270` (TwinCat3).

#### S-4 `WSTRING` (default)

- a. OK (`domain-model.md:150`).
- b. OK. Seed row `0004_seed_plc_types.sql:37`.
- c. Not modelled (A31). Vendor 162 bytes (`vendor:154`), seed 164. R06.
- d. OK. Default 80 (`vendor:154`, `string_length.rs:73`).
- e. n/a.
- f. As S-3.
- g. As S-3.
- h. `string_length/tests.rs:101`, `:124`. `recipe-db/tests/storage/seed.rs:281-288` (row `1f`: CT, 164, 80), `:339` `each_platform_but_s7_300_has_exactly_one_global_wstring_row`.

#### S-5 to S-7 (excluded rows)

- S-5: Latin-1 only. Evidence under OQ03.
- S-6 and S-7: the seed pins the platforms (`recipe-db/tests/storage/seed.rs:177-192`, `:365`). `EnsureDerivedType` filters global rows by platform (`ensure_derived.rs:193-199`). An app-level test that refuses `CHAR` on CT: not found (`grep -rn '"W\?CHAR"' crates --include=*.rs | grep -i test`). The generic test `ensure_derived/tests.rs:295` `a_primitive_missing_on_the_platform_is_not_on_the_platform` uses `LREAL` on `S7_300`. No finding, because the seed test pins the row.

### Group A

#### Row 1: fixed array `ARRAY[L..U, ...] OF T`

- a. Name: OK. The grammar is `array := "ARRAY[" dim ("," dim){0,2} "] OF " type_expr` (`domain-model.md:137`). It uses the vendor form of vendor:179. Keywords are upper case and spaces are fixed (`domain-model.md:147`). Test `type_expr_malformed_texts_are_grammar_errors` (`crates/recipe-domain/src/plant/type_expr/tests.rs:142`) refuses lower-case `array` and other spacing.
- b. Availability: OK on both platforms and all three profiles (vendor:179). Dimensions: A38 allows 1 to 3 (`overview.md:283`, `limits.rs:18`). The vendor file lists up to 3 dimensions (vendor:189) and states no maximum. Search: `grep -n "dimension" CODESYS_TwinCAT3_PLC_Data_Types_EN.md` gives no limit.
- c. Layout: no model on these platforms. `S7Layout` is defined "on S7" (`domain-model.md:219`). A31 forbids an address on `CodesysIec` and `TwinCat3` (`overview.md:276`). ADR-0108 (h) names the layout table on the S7 platforms only (`docs/adr/0108-version-page-and-type-page-additions.md:63`). Test `types_s7_layout_is_not_shown_for_the_codesys_and_the_twincat_platforms` (`crates/recipe-web/src/pages/types/s7_layout_tests.rs:141`). The vendor file makes the strides target-specific (vendor:198, vendor:425, vendor:432). No document says that the editor needs no layout on these platforms. See OQ09.
- d. Bound range: bounds are `i32` with `lower <= upper` and `upper - lower < 65536` (`domain-model.md:92`, `array_dimension.rs:48` to `:56`). The vendor file gives no bound range or element limit. Negative bounds are valid (vendor:183 `ARRAY[-2..2]`), and the editor accepts them. ADR-0145 (a) fixes only the PostgreSQL subscript of `dim_lower` and `dim_upper` (`0011_plc_type_rules.sql:56`). Leaf limit 16 384 (`limits.rs:7`).
- e. Literal: n/a for `IecLiteral`. The type text round trips: `type_expr_examples_round_trip` (`type_expr/tests.rs:21`).
- f. JSONB and export: one JSON array level per dimension with `upper - lower + 1` elements (`domain-model.md:443`). A flat list is refused (`crates/recipe-domain/src/recipe/value/parse/tests.rs:187`). The export writes `dimensions[]` as `{lower, upper}` in `i32` range, 1 to 3 entries (`export-v2.schema.json:210` to `:213`, `:240` to `:241`). The order of the dimensions in the nesting is not stated and not pinned by a test with unequal counts (R11).
- g. Bounds and nesting: OK as in d. The `json.max_depth` limit (32, `limits.rs:10`) caps the total levels (`type_expr.rs:160` to `:166`). The vendor file gives no nesting limit for these platforms. ADR-0179 applies on S7 only (`domain-model.md:152`).
- h. Tests. Positive: `span_of_65535_passes_with_65536_elements` (`crates/recipe-domain/src/plant/array_dimension/tests.rs:24`), `two_dimensional_array_takes_an_array_of_arrays` (`parse/tests.rs:181`), `array_with_16384_leaves_passes` (`crates/recipe-domain/src/plant/type_tree/limits.rs:65`), `a_two_dimensional_array_of_a_struct_keeps_the_member_and_element_order` (`crates/recipe-domain/src/export/parameters/tests.rs:344`), `dims_valid_accepts_one_to_three_dimensions_with_a_span_below_65536` (`crates/recipe-db/tests/storage/constraints/types.rs:416`). Negative: `span_of_65536_fails_with_span` (`array_dimension/tests.rs:16`), `four_dimensions_fail_with_count` (`array_dimension/tests.rs:83`), `type_expr_four_dimensions_is_grammar_error` (`type_expr/tests.rs:69`), `type_expr_reversed_bounds_is_out_of_range` (`type_expr/tests.rs:77`), `type_expr_lower_bound_above_integer_max_is_out_of_range` (`type_expr/tests.rs:118`), `array_with_16385_leaves_fails_with_leaves` (`type_tree/limits.rs:71`), `dims_valid_rejects_a_bound_array_that_starts_at_another_subscript_than_1` (`crates/recipe-db/tests/storage/constraints/type_rules.rs:37`). No platform-specific test is needed because the rules do not depend on the platform.

#### Row 2: nested array `ARRAY[..] OF ARRAY[..] OF T`

- a. Name: OK. The element of an array is any `TypeExpr`, also another array (`domain-model.md:151`). This is the vendor form of vendor:198.
- b. Availability: OK. ADR-0177 (b) keeps the nested array on `CodesysIec` and `TwinCat3` (`docs/adr/0177-no-array-of-an-array-on-s7-300.md:45`). The rule check `require_no_nested_array` refuses only when `platform.is_s7()` (`type_expr.rs:91` to `:92`). The vendor file supports nested arrays (vendor:198). ADR-0148 and ADR-0177 do not exclude them on these platforms.
- c. Layout: no model, as row 1 (OQ09). vendor:198 says that each inner `ARRAY[1..3] OF UINT` occupies 6 bytes. The editor computes no size here.
- d. Bound range: each level follows row 1.
- e. Literal: n/a. Test `type_expr_nested_array_has_two_prefixes` (`type_expr/tests.rs:38`).
- f. JSONB and export: one JSON level per dimension of each array level. Test `array_of_array_has_six_leaves_and_depth_two` (`crates/recipe-domain/src/plant/type_tree/tests.rs:84`). Each array level is its own `types[]` entry with `element_type_id` (`api.md` §6, `export-v2.schema.json:272`).
- g. Nesting: the only limit on these platforms is `json.max_depth` (32). `type_expr_32_one_dimensional_prefixes_parse` (`type_expr/tests.rs:276`) and `type_expr_33_one_dimensional_prefixes_are_too_deep` (`type_expr/tests.rs:284`). The vendor file gives no limit. Search: `grep -n -i "nest\|depth\|level" CODESYS_TwinCAT3_PLC_Data_Types_EN.md` gives only vendor:198 and vendor:204.
- h. Tests. Positive: `type_expr_array_of_array_passes_on_the_other_platforms` (`type_expr/tests.rs:397`, both platforms), `an_array_of_an_array_on_twincat_3_adds_the_parameter` (`crates/recipe-app/src/recipe/add_parameter/tests.rs:625`), `adr_0177_an_array_of_an_array_on_codesys_iec_adds_the_parameter` (`crates/recipe-app/src/recipe/add_parameter/tests/adr_0177.rs:37`), `a_member_array_of_an_array_on_twincat_3_succeeds` (`crates/recipe-app/src/plant/define_udt/tests.rs:471`), `adr_0177_a_member_array_of_an_array_on_codesys_iec_succeeds` (`crates/recipe-app/src/plant/define_udt/tests/adr_0177.rs:45`), `a_changed_type_to_an_array_of_an_array_on_twincat_3_succeeds` (`crates/recipe-app/src/recipe/update_definition/nested_array_tests.rs:116`), `adr_0179_rebind_an_array_of_ten_levels_on_twincat3_passes` (`crates/recipe-app/src/recipe/rebind/tests.rs:383`). Negative: `type_expr_array_of_array_is_not_on_the_three_s7_platforms` (`type_expr/tests.rs:373`) pins the platform boundary from the S7 side. `type_expr_33_one_dimensional_prefixes_are_too_deep` (`type_expr/tests.rs:284`) pins the depth limit.

#### Row 4: `STRUCT` DUT

- a. Name: OK. A struct is a named revision `udt_name "@" revision` (`domain-model.md:136`, `:141`). The name grammar is `[A-Za-z_][A-Za-z0-9_]{0,63}` (`domain-model.md:85`, `type_name.rs:10`, `0001_schema.sql:112`). An anonymous struct maps to a named struct. The export gives `name` and `revision` (`export-v2.schema.json:286`).
- b. Availability: OK on both platforms and profiles (vendor:202). Siemens reserved names apply on S7 only (`domain-model.md:124`). Test `adr_0186_a_siemens_name_on_codesys_iec_creates_the_struct` (`crates/recipe-app/src/plant/define_udt/tests/adr_0186.rs:126`).
- c. Layout: no model, as row 1. The vendor file gives natural layout (12 bytes) against `pack_mode := '1'` (8 bytes) (vendor:217 to vendor:224). TwinCAT default alignment is 8 bytes (vendor:28). `EXTENDS` needs the compiled base layout (vendor:204). The editor models none of this (`domain-model.md:219` S7 only, A31 `overview.md:276`). See OQ09.
- d. Value range: n/a. Member count 1 to 1024 (`domain-model.md:93`, `limits.rs:4`).
- e. Literal: n/a.
- f. JSONB and export: a JSON object with exactly the member names (`domain-model.md:444`). The export lists `members[]` by member ordinal (`api.md:524`, `:637`). Test `a_struct_inside_an_array_keeps_its_member_order` (`export/parameters/tests.rs:335`).
- g. Names: struct names are unique without case (`domain-model.md:122`, index `0011_plc_type_rules.sql:69` to `:70`). Member names are unique without case (`domain-model.md:121`, `0011_plc_type_rules.sql:74` to `:75`). A struct name is not one of 211 reserved words (`domain-model.md:123`, `type_name.rs:14`, `:201`). `UNION` and `END_UNION` are not in the list (OQ05). A member name may be a keyword (ADR-0143 (l), OQ05). No cycle (`domain-model.md:120`). Nesting: `json.max_depth` only. `adr_0179_nine_levels_pass_on_twincat3_and_codesys` (`type_tree/limits.rs:356`). The vendor file gives no name rule and no nesting limit.
- h. Tests. Positive: `valid_struct_keeps_the_member_order` (`crates/recipe-domain/src/plant/plc_type/tests.rs:271`), `struct_with_1024_members_is_valid` (`plc_type/tests.rs:290`), `type_name_of_64_characters_passes_and_65_fails` (`crates/recipe-domain/src/plant/type_name/tests.rs:22`), `struct_chain_of_32_levels_passes` (`type_tree/limits.rs:99`). Negative: `struct_with_1025_members_is_plt_018_with_the_members_limit` (`plc_type/tests.rs:295`), `duplicate_member_name_is_plt_021_and_names_the_first_duplicate` (`plc_type/tests.rs:312`), `member_names_that_differ_only_in_case_are_plt_021_under_both_rule_sets` (`plc_type/tests.rs:405`), `reserved_struct_name_is_plt_013_with_reserved_name` (`plc_type/tests.rs:435`), `reserved_words_are_refused_in_any_case` (`type_name/tests.rs:73`), `member_of_the_own_type_is_plt_005` (`plc_type/tests.rs:328`), `struct_cycle_over_two_structs_gives_type_cycle` (`type_tree/tests.rs:109`), `struct_chain_of_33_levels_fails_with_depth` (`type_tree/limits.rs:106`).

### Group X (Codex second review)

The evidence keys below apply to every row that cites them. They distinguish **JSONB numbers** from the **quoted decimal strings in the export**.

| Key | Dimension and evidence |
|---|---|
| **A** | **a.** Exact uppercase primitive spelling comes from `crates/recipe-domain/src/plant/primitive.rs:155–159` and `crates/recipe-domain/src/plant/primitive.rs:164–194`. `crates/recipe-domain/src/plant/type_expr.rs:208–217` uses that parser. `crates/recipe-domain/src/plant/primitive/tests.rs:74` round-trips every name. `crates/recipe-domain/src/plant/primitive/tests.rs:87` rejects wrong names and several vendor aliases. |
| **B** | **b.** The row-specific seed and matrix lines are in the inventory. `crates/recipe-db/tests/storage/seed.rs:365` compares every seed with the matrix. `crates/recipe-domain/src/plant/type_tree/tests.rs:172` excludes a row from the wrong platform. `crates/recipe-domain/src/plant/platform.rs:7` has one enum value per platform. docs/adr/0142-twincat-version-of-the-long-date-types.md:42–45 and `docs/ops/runbook.md:1371–1375` place the TwinCAT version check at commissioning. |
| **C** | **c.** `docs/architecture/overview.md:276` says a CODESYS or TwinCAT parameter has no address. docs/adr/0017-derived-type-identity.md:61 confines layout checks to addressed parameters. `docs/architecture/domain-model.md:88` defines catalog `size_bytes` as S7 size. The type page skips layout on these platforms at `crates/recipe-web/src/pages/types/tree.rs:205`. Address tests are `crates/recipe-domain/src/recipe/draft/s7/tests.rs:171`, `:184`, `:201`. V1 and v2 require null `s7_address` at `crates/recipe-export/schema/export-v1.schema.json:91` and `export-v2.schema.json:99`. The widths below describe the PLC. There is no IEC padding, alignment, bit-offset, or byte-order calculation in the recipe export. |
| **N** | **d/e.** `crates/recipe-domain/src/recipe/value/numeric.rs:15–25` selects the numeric range. `crates/recipe-domain/src/recipe/value/leaf.rs:219–247` checks integrality, range, and precision. `crates/recipe-domain/src/recipe/value/texts.rs:138–155` requires JSON-number syntax. An integral decimal or exponent form can pass for an integer. docs/adr/0183-string-notation-and-decimal-text-only.md:43 excludes ST typed constants, base prefixes, and separators. `crates/recipe-domain/src/recipe/limit/parse.rs:219–250` checks bounds. `docs/architecture/domain-model.md:432–436` specifies limit applicability and approval. |
| **J** | **f.** `docs/architecture/reliability.md:510–512` specifies exact decimal normalization. `crates/recipe-domain/src/recipe/value/leaf.rs:208–210` and `:244–247` construct JSON numbers. `crates/recipe-db/tests/storage/contract/draft/insert.rs:104–114` checks numeric JSONB and 64-bit examples. `crates/recipe-domain/src/export/parameters.rs:203–210` writes a JSON number as a string, preserves booleans/text, and descends into containers. `docs/architecture/api.md:530–547` defines the export values, limits, and units. `crates/recipe-domain/src/export/types.rs:70–89` uses the primitive name or a length-bearing row UUID. |
| **G** | **g.** A scalar has no string capacity, bounds, or DUT name of its own. Its use inside arrays and named members follows `docs/architecture/domain-model.md:126–151`, `crates/recipe-domain/src/plant/type_expr.rs:149–173`, and `crates/recipe-domain/src/recipe/value/walk.rs:108–139,146–163`. |
| **H** | **h.** Every integer and bit-string row has a positive test at both endpoints and negative tests one step outside each endpoint. `crates/recipe-domain/src/recipe/value/leaf/tests.rs:350–367` exercises the value path. `crates/recipe-domain/src/recipe/limit/parse/tests.rs:108–121` exercises the limit path. The individual data rows appear in `crates/recipe-domain/src/recipe/value/test_support.rs:125–209`. |
| **T** | **d/e/f.** `crates/recipe-domain/src/plant/iec_literal/mod.rs:111–125,143–155` dispatches exact prefixes and applies the current range. `crates/recipe-domain/src/plant/iec_literal/instant.rs:239–251` computes count units. `:257–282` rebuilds time and checks midnight where required. `crates/recipe-domain/src/recipe/limit/parse.rs:234–250` checks stored counts. `docs/architecture/domain-model.md:275–288` defines precision, rendering, and errors. JSONB stores the integer count. Export writes its exact decimal string, as in J. No time-zone conversion occurs. |
| **W** | **d/e/f/g.** `crates/recipe-domain/src/plant/string_length.rs:47–55,70–73,96–99` enforces capacity 1–255, default 80. `crates/recipe-domain/src/plant/type_expr.rs:230–246` parses the length. `crates/recipe-domain/src/recipe/value/leaf.rs:115–153` checks repertoire, NUL, and capacity. `crates/recipe-domain/src/recipe/value/texts.rs:162` preserves input text. `crates/recipe-domain/src/recipe/value/check/tests.rs:117–143` covers `max_length`. `crates/recipe-domain/src/export/types.rs:81–89` exports capacity in `types[]`. `crates/recipe-domain/src/export/parameters.rs:208` preserves the string. |

**Layout reference for C.** CODESYS byte order depends on the target. TwinCAT PLC host order is little-endian. Bit 0 has weight 1 within a byte. Native addresses are 4 or 8 bytes. TwinCAT’s default system alignment of 8 does not align every field to 8. The worked `ST_Sample` has natural size/stride 12 and packed size/stride 8. The §9 probe distinguishes initializer order from measured element stride. The reference explicitly says the probe was never run on either profile. `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:22–40`, `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:217–226`, `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:419–432`. These facts do not justify treating S7 widths as an IEC ABI.

| # / type | a. Name | b. Availability | c. Native size and layout | d. Range and limits | e. Input grammar | f. JSONB → export | g. Shape / encoding | h. Positive and negative tests |
|---|---|---|---|---|---|---|---|---|
| 1 BOOL | A, `crates/recipe-domain/src/plant/primitive.rs:166` | B, P | C, ordinary byte `00/01`. Eight ordinary BOOL elements use eight bytes | Boolean, no process limit, `docs/architecture/domain-model.md:232,433` | JSON `true/false`. Form uses lowercase `true/false`, `crates/recipe-domain/src/recipe/value/texts.rs:157` | Boolean → boolean, J | G. Packed BIT is separate | `crates/recipe-domain/src/recipe/value/leaf/tests.rs:81` accepts both. `:73` rejects number and quoted text. `crates/recipe-domain/src/recipe/limit/parse/tests.rs:51` rejects a bound |
| 3 BYTE | A, `crates/recipe-domain/src/plant/primitive.rs:167` | B, P | C, 1 B | N, 0…255 | N | J, e.g. `255` → `"255"` | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:141`. Based/typed rejection `crates/recipe-domain/src/recipe/value/texts_tests.rs:596` |
| 4 WORD | A, `crates/recipe-domain/src/plant/primitive.rs:168` | B, P | C, 2 B | N, 0…65535 | N | J, `"65535"` at max | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:162`. Based/typed rejection `crates/recipe-domain/src/recipe/value/texts_tests.rs:596` |
| 5 DWORD | A, `crates/recipe-domain/src/plant/primitive.rs:169` | B, P | C, 4 B | N, 0…4294967295 | N | J, `"4294967295"` at max | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:183`, `crates/recipe-domain/src/recipe/value/texts_tests.rs:596` |
| 6 LWORD | A, `crates/recipe-domain/src/plant/primitive.rs:170` | B, P | C, 8 B | N, 0…18446744073709551615 | N | J, exact 20-digit max | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:204`, `crates/recipe-domain/src/recipe/value/texts_tests.rs:596` |
| 7 SINT | A, `crates/recipe-domain/src/plant/primitive.rs:171` | B, P | C, 1 B, two’s complement | N, −128…127 | N | J, `"-128"` at min | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:127` |
| 8 USINT | A, `crates/recipe-domain/src/plant/primitive.rs:172` | B, P | C, 1 B | N, 0…255 | N | J, `"255"` at max | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:134` |
| 9 INT | A, `crates/recipe-domain/src/plant/primitive.rs:173` | B, P | C, 2 B, two’s complement | N, −32768…32767 | N | J | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:148`. Fraction rejected and integral `1.0` accepted at `crates/recipe-domain/src/recipe/value/parse/tests.rs:165,176` |
| 10 UINT | A, `crates/recipe-domain/src/plant/primitive.rs:174` | B, P | C, 2 B | N, 0…65535 | N | J | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:155` |
| 11 DINT | A, `crates/recipe-domain/src/plant/primitive.rs:175` | B, P | C, 4 B, two’s complement | N, −2147483648…2147483647 | N | J | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:169` |
| 12 UDINT | A, `crates/recipe-domain/src/plant/primitive.rs:176` | B, P | C, 4 B | N, 0…4294967295 | N. `UCHAR#'Ł'` must be entered as `321` | J | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:176`. Decimal-only rule at `crates/recipe-domain/src/recipe/value/texts.rs:150` |
| 13 LINT | A, `crates/recipe-domain/src/plant/primitive.rs:177` | B, P | C, 8 B, two’s complement | N, −2⁶³…2⁶³−1 | N | J, exact signed endpoints | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:190` |
| 14 ULINT | A, `crates/recipe-domain/src/plant/primitive.rs:178` | B, P | C, 8 B | N, 0…2⁶⁴−1 | N | J, exact unsigned endpoints | G | H, `crates/recipe-domain/src/recipe/value/test_support.rs:197` |
| 15 REAL | A, `crates/recipe-domain/src/plant/primitive.rs:179` | B, P | C, binary32, 4 B | Decimal envelope, scale ≤28, ≤7 significant digits for values and bounds. No NaN, infinity, or binary subnormal input. Signed zero canonicalizes to zero. docs/adr/0115-ranges-narrower-than-the-vendor-range.md:43, docs/adr/0117-significant-digits-of-real-and-lreal.md:42–45 | N, decimal with optional exponent | J, canonical decimal string | G | `crates/recipe-domain/src/recipe/value/leaf/tests.rs:263,244,204`, `crates/recipe-domain/src/recipe/limit/parse/tests.rs:454,372`, `crates/recipe-domain/src/recipe/value/texts_tests.rs:618` |
| 16 LREAL | A, `crates/recipe-domain/src/plant/primitive.rs:180` | B, P, OQ01 | C, binary64 8 B when supported. CODESYS can lower it to binary32 | Same decimal envelope, ≤15 significant digits. `crates/recipe-domain/src/recipe/value/leaf.rs:57–60` | N | J | G | `crates/recipe-domain/src/recipe/value/leaf/tests.rs:263,250`, `crates/recipe-domain/src/recipe/limit/parse/tests.rs:480,462`, `crates/recipe-domain/src/recipe/value/numeric/tests.rs:103` |
| 17 TIME | A, `crates/recipe-domain/src/plant/primitive.rs:185` | B, P | C, 4 B | T, 0…4294967295 ms, min/max | `T#1s250ms` → 1250. Integer components down to ms | J/T, `1250` → `"1250"` | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:49,96` tests both ends and adjacent failures on both platforms. `crates/recipe-domain/src/recipe/value/leaf/tests.rs:149,170` |
| 18 LTIME | A, `crates/recipe-domain/src/plant/primitive.rs:186` | B, P including 4024 | C, 8 B | T, 0…2⁶⁴−1 ns, min/max | `LT#1us` → 1000. Vendor `LTIME#1us` rejected, OQ07 | J/T, `"1000"` | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:49,102`, `crates/recipe-domain/src/plant/iec_literal/duration/tests.rs:54,59,77` |
| 19 DATE | A, `crates/recipe-domain/src/plant/primitive.rs:187` | B, P | C, **4 B native**, 2 in S7 metadata, R06 | T, 1970-01-01…2106-02-07, 0…49710 days | `D#YYYY-MM-DD`, real calendar date | J/T. `D#1970-01-02` → `"1"`. Native midnight counter is 86400 seconds | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:49,144,150`, `crates/recipe-domain/src/plant/iec_literal/instant/tests.rs:45,127` |
| 20 DATE_AND_TIME | A, `crates/recipe-domain/src/plant/primitive.rs:191`, vendor DT | B, P | C, **4 B native**, 8 in S7 metadata | T, unsigned seconds 0…4294967295. Last instant 2106-02-07 06:28:15 | `DT#YYYY-MM-DD-hh:mm:ss`. Whole seconds only | J/T, `"1"` one second after epoch | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:332,454`, `crates/recipe-domain/src/plant/iec_literal/instant/tests.rs:55,143`, `crates/recipe-domain/src/plant/primitive/tests.rs:87` |
| 21 TIME_OF_DAY | A, `crates/recipe-domain/src/plant/primitive.rs:189`, vendor TOD | B, P | C, 4 B | T, 0…86399999 ms, min/max | `TOD#hh:mm:ss[.fff]` | J/T, `"1234"` for `TOD#00:00:01.234` | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:298`, `crates/recipe-domain/src/plant/iec_literal/instant/tests.rs:29,135,199` |
| 22 LDATE | A, `crates/recipe-domain/src/plant/primitive.rs:188`, `LD` is literal-only | B, L | C, signed epoch ns in 8 B | T, whole days only. Current 1970-01-01…2262-04-11. Vendor begins 1677-09-22. Explicit narrowing, docs/adr/0139-range-of-ldate-and-ldt-on-codesys-and-twincat.md:46–51 | `LD#YYYY-MM-DD` | J/T, `"86400000000000"` for 1970-01-02 | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:263,531,563`, `crates/recipe-domain/src/plant/iec_literal/instant/tests.rs:191`, `crates/recipe-domain/src/recipe/limit/parse/tests.rs:603,626` |
| 23 LDT | A, `crates/recipe-domain/src/plant/primitive.rs:192`, vendor LDATE_AND_TIME | B, L | C, signed epoch ns in 8 B | T, current 0…2⁶³−1. Vendor also permits negative counts down to −2⁶³ | `LDT#YYYY-MM-DD-hh:mm:ss[.fffffffff]` | J/T, `"1"` one ns after epoch. Earlier out-of-range data remains readable with `loadable=false` | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:346,531,563,595`, `crates/recipe-web/src/pages/export/platform_range_tests.rs:56` |
| 24 LTOD | A, `crates/recipe-domain/src/plant/primitive.rs:190`, vendor LTIME_OF_DAY | B, L | C, 8 B | T, 0…86399999999999 ns. Reject one full day | `LTOD#hh:mm:ss[.fffffffff]` | J/T, `"1"` one ns after midnight | G | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs:49,316`, `crates/recipe-domain/src/plant/iec_literal/instant/tests.rs:79,151` |
| 25 STRING | A, `crates/recipe-domain/src/plant/primitive.rs:181`, W | B, P | C, native `n+1`, default 81 B. One zero terminator. S7 catalog metadata is `n+2` | W. Non-NUL Latin-1, up to `n` bytes, `max_length` within capacity. The 255 declaration cap is deliberate, docs/adr/0141-string-length-on-codesys-and-twincat.md:44–47 | Raw text or JSON string. Bare `STRING` or `STRING(n)` type. Parentheses selected by docs/adr/0183-string-notation-and-decimal-text-only.md:42 | J/W, JSON string → same text and separate capacity | Latin-1 by docs/adr/0181-latin-1-bytes-of-string-and-char.md:42–43. OQ03 for UTF-8 declarations | `crates/recipe-domain/src/recipe/value/leaf/tests.rs:25,32,37,42,116`, `crates/recipe-domain/src/plant/string_length/tests.rs:11,21,47,101`, `crates/recipe-domain/src/recipe/value/check/tests.rs:117,126` |
| 26 WSTRING | A, `crates/recipe-domain/src/plant/primitive.rs:182`, W | B, P | C, native `2(n+1)`, default 162 B. Aligned zero word terminates. S7 metadata is `2n+4` | W. Non-NUL, up to `n` UTF-16 units. The PLC repertoire is UCS-2, OQ02 | Raw text or JSON string. Bare or parenthesized type | J/W. The fixture exports `"Z𝄞"` with capacity 16 | Editor accepts supplementary scalars as two units. Vendor permits only non-surrogate BMP scalars in the payload | `crates/recipe-domain/src/recipe/value/leaf/tests.rs:57,63,124,459`, `crates/recipe-domain/src/plant/string_length/tests.rs:47,66,101`, `crates/recipe-export/tests/golden_types.rs:586` |
| 29 Fixed ARRAY | `crates/recipe-domain/src/plant/type_expr.rs:149,181`, kind `array` with no name | P. `crates/recipe-domain/src/plant/type_expr.rs:91` allows nested arrays on both | C. Native stride from compiled element. JSON traverses declared dimensions with the last index varying fastest | Signed i32 bounds, lower ≤ upper, span <65536, 1–3 dimensions per declaration, ≤16384 leaves per parameter, depth ≤32. Limits belong to leaves | `ARRAY[lo..hi,…] OF T`. Nested prefix allowed. Values use one JSON level per dimension | `crates/recipe-domain/src/export/types.rs:120` exports element reference and bounds. `crates/recipe-domain/src/recipe/value/walk.rs:146,186` maps JSON positions to PLC indices | Nesting, arbitrary lower bounds, strings and structs supported. Vendor flat initializer order does not define JSON order or physical stride | `crates/recipe-domain/src/plant/array_dimension/tests.rs:8,16,24,67,83`, `crates/recipe-domain/src/plant/type_expr/tests.rs:397`, `crates/recipe-domain/src/recipe/value/parse/tests.rs:181,187`, `crates/recipe-domain/src/plant/type_tree/leaves/tests.rs:56` |
| 31 STRUCT / DUT | Revision reference `name@revision`, `crates/recipe-domain/src/plant/type_expr.rs:253`. Defined through `DefineUdt` | P, same-machine member types, `docs/architecture/domain-model.md:119–127` | C. Native declaration order, packing, nested and tail padding are compiler properties | 1–1024 members, no cycles, ≤16384 leaves, depth ≤32. Values and limits follow each member’s type | UDT definition plus reference. JSON object must have exactly the declared keys | `crates/recipe-domain/src/export/types.rs:142–162` writes name, revision and ordered members. `crates/recipe-domain/src/recipe/value/walk.rs:108–139` checks exact keys | ASCII identifiers ≤64, case-insensitive uniqueness, exact reference spelling, reserved struct names. Member keyword exception in OQ05 | `crates/recipe-domain/src/plant/plc_type/tests.rs:271,279,290,295,405,435`, `crates/recipe-domain/src/plant/type_tree/tests.rs:109`, `crates/recipe-domain/src/recipe/value/parse/tests.rs:34,50,58` |

**Applicability of the Siemens-era decisions.** docs/adr/0177-no-array-of-an-array-on-s7-300.md:45 explicitly preserves nested arrays on these platforms. docs/adr/0179-nesting-limit-of-structs-on-s7.md:58 preserves their general JSON depth budget. docs/adr/0180-wchar-and-wstring-characters-on-s7.md:51 retains their earlier wide-character rule. docs/adr/0181-latin-1-bytes-of-string-and-char.md:42–43 and docs/adr/0183-string-notation-and-decimal-text-only.md:42–44 apply to their text and numeric inputs. docs/adr/0186-siemens-type-names-reserved-on-s7.md:45 preserves the common keyword list, while its extra Siemens names apply on S7. A50 specifically names an S7 CPU at `docs/architecture/overview.md:295`. The remaining S7 address, firmware, DATE, DTL, and packing decisions supply no IEC memory-layout rule.

## 3. Findings

### 3.1 Map of the group ids

| Report id | Severity | Kind | Group ids | Orchestrator check |
|---|---|---|---|---|
| R01 | MEDIUM | missing check | X04 | Verified: `export-v2.schema.json:152` `value` accepts any string or boolean. `:141-147` `type_ref` enum lists CHAR and WCHAR for every platform. `:235` string_length max 16382. `api.md:533` "A `BOOL` is a JSON boolean". Analog of Siemens R01 (ADR-0189) |
| R02 | MEDIUM | missing-test | A01, X05 | Verified: `rg "POINTER TO\|REFERENCE TO\|ARRAY\[\*\]" crates` finds nothing. The parser refuses them today |
| R03 | MEDIUM | missing-test | N02 | No test refuses NaN or Infinity on a REAL or LREAL leaf. Code refuses them today |
| R04 | MEDIUM | missing-test | S01 | No test of the Latin-1 boundary (U+00FF accepted, U+0100 refused) of a STRING character. `leaf.rs:131` |
| R05 | MEDIUM | missing-test | S02 | Verified: `string_length/tests.rs:47` tests zero only for STRING. No WSTRING length 0 test |
| R06 | LOW | code-vs-doc | T01, A04, S03, X02 | Verified: `data-model.md:2325` "The domain holds the IEC sizes in code". `primitive.rs:229` `fixed_size_bytes` gives S7 sizes only |
| R07 | LOW | doc gap | N01, A02, X01 | IEC-platform type families without a named exclusion. Pattern of Siemens R04 and ADR-0185 |
| R08 | LOW | code-vs-doc | X03 | WSTRING length unit in comments: `string_length.rs:61`, `primitive.rs:41`, `0001_schema.sql:95` |
| R09 | LOW | missing-test | N03, T02 | No per-type export golden file and no literal test at the IEC ends of TIME, LTIME, DATE, DT on these platforms. Count-level tests exist |
| R10 | LOW | doc gap | S04 | `api.md:533` states the bytes of STRING and CHAR but not of WSTRING |
| R11 | LOW | doc gap | A03 | No document says which array dimension is the outer JSON level. No test pins the order with unequal counts |

Owner questions: NQ01 and XQ03 are OQ01. SQ01 and XQ01 are OQ02. SQ02 and XQ02 are OQ03. TQ01 and XQ06 are OQ04. NQ02, AQ02, AQ03, and XQ07 are OQ05. XQ08 is OQ06. TQ02 and XQ04 are OQ07. XQ05 is OQ08. AQ01 is OQ09.

The findings are in the order of the merge spec: MEDIUM first, then LOW. No finding is CRITICAL or HIGH. The source blocks below keep the group ids in their headings.

### R01 — MEDIUM — missing check — dimensions b, d, f, and g — the export schemas accept documents that the editor rejects

The orchestrator read the schema lines again. `export-v2.schema.json:152` lets `value` take any string or boolean. `:141-147` lists `CHAR` and `WCHAR` in the `type_ref` enum for every platform. `:235` gives `string_length` the maximum 16382. `docs/architecture/api.md:533` says "A `BOOL` is a JSON boolean". This is the analog of Siemens R01 (ADR-0189).

#### Source X04 (Codex): MEDIUM — missing check — b, d, f, g — Schema validation alone accepts IEC-platform documents the editor rejects

**Vendor evidence.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:50` specifies a logical FALSE/TRUE, `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:52` gives BYTE eight bits, `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:123` ends TOD at 86,399,999 ms, . `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:171` reports no standard standalone CHAR or WCHAR on either platform.

**Normative evidence.** `docs/architecture/api.md:533` says “A `BOOL` is a JSON boolean.” `docs/architecture/data-model.md:2307–2308` marks CHAR/WCHAR unavailable. `docs/architecture/domain-model.md:93,106,110` gives the member and string-capacity limits. docs/adr/0019-export-integrity-and-contract.md:45 calls the schema the export contract’s “source of truth.”

**Code and probe evidence.** The shared reference and value definitions are `crates/recipe-export/schema/export-v1.schema.json:129,144` and `export-v2.schema.json:137,152`. The string and member constraints are at v1 `:213,222` and v2 `:235,244`. In both schema versions and for both platform values, a Draft 2020-12 validator accepted each of the following independently:

- A BOOL parameter with JSON string `"true"`, a BYTE with `"256"`, and a TIME_OF_DAY with `"86400000"`.
- A direct `CHAR` or `WCHAR` type reference.
- A WSTRING declaration of 256 and a struct declaration with 1025 distinct members. The same probe rejected zero string capacity.

The Rust checks are present at `crates/recipe-domain/src/recipe/value/leaf.rs:88–97,231–234`, `crates/recipe-domain/src/plant/iec_literal/range.rs:107–112`, `crates/recipe-domain/src/plant/string_length.rs:47–55`, and `crates/recipe-domain/src/plant/plc_type.rs:270–279`. The archived schema design explicitly describes a structural recursive value union at `openspec/changes/archive/2026-10-23-export-v1/design.md:109–128`.

**Effect and fix.** This is a consumer-validation limitation. The probe used constructed documents and checked schema validity only. It did not establish that the writer produces them or that their hashes are valid. Specify which checks a consumer must perform after schema validation. A version-compatible schema can also enforce direct scalar kinds, platform-specific primitive names, and declaration maxima. Keep UUID-dependent checks in a type-aware pass.

### R02 — MEDIUM — missing-test — dimensions a, e, g, and h — no negative test for the open array, pointer, and reference types

The orchestrator ran `rg "POINTER TO|REFERENCE TO|ARRAY\[\*\]" crates` and found no match. The parser refuses the three texts today. Two groups report the missing negative test. Group A also names `POINTER TO` and `REFERENCE TO`.

#### Source A01 (group A): MEDIUM — missing-test — dimensions a and h — no test refuses open array, pointer, and reference type texts

- Vendor: vendor:187 "`ARRAY[*,*] OF INT` as `VAR_IN_OUT`". vendor:272 "`POINTER TO T`". vendor:273 "`REFERENCE TO T`".
- Normative: `domain-model.md:139` "`bound     := -?[0-9]{1,10}`". `data-model.md:2330` "Types that are not in scope: IEC `ENUM`, subrange, and alias (Assumption A39), and `POINTER`, `REFERENCE`, `ANY`."
- Code: `crates/recipe-domain/src/plant/type_expr.rs:200` `parse_bound` calls `parse_integer`, which returns `IntegerError::Grammar` for `*` (`crates/recipe-domain/src/plant/lexical.rs:33` to `:34`). `Primitive::parse` refuses `POINTER TO INT` as unknown (`primitive.rs:155` to `:159`). The refusal is right today.
- Test: not found. `grep -rln 'ARRAY\[\*' crates migrations e2e/tests`, `grep -rln 'POINTER TO' ...`, and `grep -rln 'REFERENCE TO' ...` give no file. `type_expr_malformed_texts_are_grammar_errors` (`type_expr/tests.rs:142`) holds none of these texts.
- Effect: `LeafPath` already has a `[*]` segment (`domain-model.md:182`). A later change can share the index parser between `LeafPath` and `TypeExpr`. Then `ARRAY[*] OF INT` can pass and no test fails. The editor would export a type that a variable cannot have on either platform (vendor:187: an open array exists only as a `VAR_IN_OUT` formal).
- Suggested fix: add `ARRAY[*] OF INT`, `ARRAY[*,*] OF INT`, `ARRAY[1..2,*] OF INT`, `POINTER TO INT`, and `REFERENCE TO INT` to `type_expr_malformed_texts_are_grammar_errors` with `DerivedTypeReason::Grammar`.

#### Source X05 (Codex): MEDIUM — missing-test — e, g, h — The type parser has no negative example for an open-array declaration

**Vendor evidence.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:187` gives `ARRAY[*,*] OF INT` as `VAR_IN_OUT` and says “An open-array formal receives the caller’s bounds.”

**Normative evidence.** `docs/architecture/domain-model.md:137–139` requires `dim := bound ".." bound`. `docs/architecture/domain-model.md:443` requires exactly `upper - lower + 1` elements per dimension. The limit-path wildcard is a separate rule at `docs/architecture/domain-model.md:436`.

**Code evidence.** `crates/recipe-domain/src/plant/type_expr.rs:192–196` splits every type dimension on `..`, so the current parser rejects `*`. The negative grammar table at `crates/recipe-domain/src/plant/type_expr/tests.rs:142–162` includes empty and malformed dimensions. An explicit open-array type test was **not found** with `rg -n -F 'ARRAY[*' crates`.

**Effect and fix.** A change adding wildcard type parsing could leave the existing malformed-dimension cases green. Add negative `TypeExpr::parse` cases for `ARRAY[*] OF INT` and `ARRAY[*,*] OF INT`. Keep the positive fixed-array and process-limit wildcard tests.

### R03 — MEDIUM — missing-test — dimensions d and h — no test refuses NaN or infinity on a `REAL` or `LREAL` leaf

- Vendor: `vendor:98` "All one | Zero | Positive or negative infinity". `vendor:99` "All one | Nonzero | NaN."
- Document: `reliability.md:512` "before storage the domain rewrites every numeric ... leaf to the canonical decimal text of `Decimal`." ADR-0115 (c) (`0115-...:43`) "A number outside this range ... gives `E-RCP-005`." `Decimal` has no NaN and no infinity.
- Code: `crates/recipe-domain/src/recipe/value/texts.rs:150` `if !is_json_number(text) {` refuses them today. The refusal lists `texts_tests.rs:95` (`["1,5", "abc", "", " 5", "+5", ".5", "0x10"]`) and `leaf/tests.rs:314` (`"1_000", "+5", ".5", ...`) hold no `NaN`, `Infinity`, `-Infinity`, or `inf`. `rg -n 'NaN|Infinity|"inf"' crates -g '*.rs'` finds nothing.
- Effect: a later change that parses the text through `f64` (for example `str::parse::<f64>` accepts `NaN` and `inf`) passes every test. Such a value can reach the export, which `REAL` holds but the canonical decimal contract forbids.
- Suggested fix: add `NaN`, `nan`, `Infinity`, `-Infinity`, and `inf` to `texts_tests.rs:95`, to `leaf/tests.rs:314`, and to one limit bound test in `limit/parse/tests.rs`. Pin the code `E-RCP-005` or the `Expected::Decimal` fault.

### R04 — MEDIUM — missing-test — dimensions g and h — no test pins the Latin-1 edge of a `STRING` character

- Vendor: `vendor:161` "`STRING(2)` containing `é`, Latin-1 | `E9 00 xx` | One character uses one payload byte."
- Document: `docs/adr/0137-no-nul-in-a-text-value-and-the-unit-of-a-string-length.md:55` "(b) A `STRING` character is in `U+0001..=U+00FF`."
- Code: `crates/recipe-domain/src/recipe/value/leaf.rs:11` `const LATIN_1_MAX: u32 = 0xFF;` and `:135` `let is_in_range = text.chars().all(|c| u32::from(c) <= highest);`.
- Test: `leaf/tests.rs:25` refuses U+0141 ("Łódź"). `leaf/tests.rs:32` accepts at most U+00FC. `grep -rniE 'u\{0*100\}|u\{0*ff\}|\\u0100|\\u00ff|ÿ|Ā' crates --include=*.rs` finds only `prop/generate.rs:351` and `:413`, no test value at U+00FF or U+0100. The property generator draws `'\u{1}'..='\u{FF}'` (`prop/generate.rs:413`), so U+00FF passes only by chance.
- Effect: `LATIN_1_MAX` and the comparison at `leaf.rs:135` are pinned by other tests (`leaf/tests.rs:107` expects `Expected::Char { max: 0xFF }`, and the S7 test `leaf/tests.rs:415` accepts U+CFFF). The `STRING` arm `(false, _) => LATIN_1_MAX` at `leaf.rs:131` is not pinned at its edge. A change of that arm to `0x13F` passes every test and accepts U+0100..=U+013F, which the Latin-1 transfer cannot write as one byte (CRITICAL class). A change to `0xFE` also passes every test and refuses "ÿ" (HIGH class).
- Fix: in `leaf/tests.rs`, accept "ÿ" (U+00FF) and refuse "Ā" (U+0100) on a `STRING(1)` leaf on `CodesysIec` and on `TwinCat3`. Expect `Expected::String { length: 1 }`.

### R05 — MEDIUM — missing-test — dimensions d and h — no test refuses a `WSTRING` length of 0

- Vendor: the vendor file states no minimum length. The closest line is `vendor:169` "`STRING(1)` occupies 2 bytes and `WSTRING(1)` occupies 4 bytes."
- Document: `docs/architecture/domain-model.md:110` "| `CodesysIec`, `TwinCat3` | `1..=255` | `1..=255` |".
- Code: `crates/recipe-domain/src/plant/string_length.rs:54` `if (1..=max).contains(&length)`.
- Test: `string_length/tests.rs:47-54` loops `StringLength::parse(STRING, platform, 0)` only. `type_expr/tests.rs:144` tests `"STRING(0)"` only. `grep -rn "WSTRING(0)\|WSTRING, platform, 0" crates --include=*.rs` finds none.
- Effect: low today, because both kinds share one check. A later split of the range per kind loses the lower bound of `WSTRING` without a failing test.
- Fix: in `zero_length_is_out_of_range_on_every_platform`, also parse `WSTRING` with 0 on each platform that has it. Add `"WSTRING(0)"` to `type_expr_malformed_texts_are_grammar_errors`.

### R06 — LOW — code-vs-doc — dimension c — `data-model.md` says that the domain holds IEC sizes, but no code holds them

The orchestrator read `docs/architecture/data-model.md:2325`: "The domain holds the IEC sizes in code." It read `crates/recipe-domain/src/plant/primitive.rs:229` and found that `fixed_size_bytes` gives S7 sizes only. Four group results report the same wrong sentence. Group N found the same state and raised no finding. The sentence hides these size differences:

- The seed rows `STRING` and `WSTRING` store the sizes 82 and 164 (`migrations/0004_seed_plc_types.sql:36`, `:37`). The vendor sizes are 81 and 162 (`vendor:152`, `vendor:154`).
- The seed row `DATE_AND_TIME` stores the size 8 (`0004_seed_plc_types.sql:32`). The vendor size is 4 (`vendor:122`). No document notes this difference.
- The seed row `DATE` stores the size 2 (`0004_seed_plc_types.sql:28`). The vendor size is 4 (`vendor:121`). `data-model.md:2325` notes this case.

The effect on data is none, because A31 forbids an address on these platforms and the types page skips the S7 table (`crates/recipe-web/src/pages/types/tree.rs:206`). Group X cites `tree.rs:205` for the same skip.

#### Source T01 (group T): LOW, code-vs-doc and doc gap, dimension c. A note claims IEC sizes in code that does not exist, and the `DATE_AND_TIME` size difference has no note

- Vendor evidence: line 121 "`DATE` | 4 | Unsigned seconds from epoch". Line 122 "`DATE_AND_TIME` / `DT` | 4 | Unsigned seconds from epoch".
- Normative evidence: `data-model.md:2325` "`DATE` is 4 bytes on CODESYS and TwinCAT 3. `size_bytes` holds the S7 size … The domain holds the IEC sizes in code." `data-model.md:2315` gives `DATE_AND_TIME` the S7 size 8. No note names the 4 bytes on CODESYS and TwinCAT.
- Code evidence: `primitive.rs:233-254` (`fixed_size_bytes`) has one size for each primitive and no platform input. `0004_seed_plc_types.sql:28` (`DATE`, 2) and `:32` (`DATE_AND_TIME`, 8) store one size for all platforms. The search `rg -i "iec_size|size_on_platform|platform.*size_bytes|fn .*size.*platform" crates/recipe-domain/src crates/recipe-app/src` finds no match.
- Effect: none at runtime. A31 forbids an address on these platforms, and `tree.rs:206` hides the S7 table. A reader of `plc_type.size_bytes` gets 2 and 8 bytes for two 4-byte types. The documented "IEC sizes in code" does not exist.
- Suggested fix: replace the note with "`DATE` and `DATE_AND_TIME` are 4 bytes on CODESYS and TwinCAT 3. `size_bytes` holds the S7 size. The editor models no layout on these platforms (A31)."

#### Source A04 (group A): LOW — code-vs-doc — dimension c — `data-model.md` says that the domain holds IEC sizes, but no code holds them

- Vendor: vendor:217 to vendor:224 give the offsets and the size from the member sizes (`UDINT` 4 bytes, `UINT` 2 bytes). vendor:185 gives 5 bytes for `STRING(4)`.
- Normative: `data-model.md:2325` "`DATE` is 4 bytes on CODESYS and TwinCAT 3. `size_bytes` holds the S7 size because every platform with an address needs an address width. The domain holds the IEC sizes in code."
- Code: `crates/recipe-domain/src/plant/primitive.rs:230` to `:233` "Return the S7 size of this primitive in bytes". `Date` gives 2 (`primitive.rs:236`). `plc_type.rs:213` uses that value for every platform. Test `machine_string_20_on_twincat3_has_size_22` (`crates/recipe-domain/src/plant/plc_type/tests.rs:85`) pins the S7 size `n + 2` on `TwinCat3`. Search: `grep -rn -i "iec size\|iec_size\|size_on\|4 bytes on" crates/recipe-domain/src` gives nothing.
- Effect: none on data, because no layout is computed or exported on these platforms. The sentence is the only normative statement of a non-S7 size, and it claims code that does not exist. A reader can think that the editor has a layout model for `CodesysIec` and `TwinCat3`.
- Suggested fix: remove the last sentence of `data-model.md:2325`, or name the code. Cross-group: the §4 time group (`DATE`) and the §5 strings group (`STRING(n)` size) can report the same line.

#### Source S03 (group S): LOW — doc gap — dimension c — the CT string seed sizes have a wrong stated reason

- Vendor: `vendor:152` "`STRING` | 81 | Equivalent to `STRING(80)`". `vendor:154` "`WSTRING` | 162".
- Document: `docs/architecture/data-model.md:2325` "`size_bytes` holds the S7 size because every platform with an address needs an address width. The domain holds the IEC sizes in code." A31 (`overview.md:276`): "A CODESYS or TwinCAT 3 parameter has no address".
- Code: `migrations/0004_seed_plc_types.sql:36` `'STRING', ARRAY['CodesysIec', 'TwinCat3'], 82, 80` and `:37` `164, 80`. `string_length.rs:84-90` is the only size of a string. `grep -rni "IEC size\|iec_size_bytes\|fn iec_size" crates --include=*.rs` finds nothing.
- Effect: the rows `1e` and `1f` serve only platforms without an address, so the stated reason does not apply. The domain holds no IEC size of a string in code. A reader can take 82 and 164 as the PLC sizes. The export schemas have no size field (`grep -n size_bytes crates/recipe-export/schema/*.json` gives none), and the types page skips CT (`pages/types/tree.rs:206`). The audit `after` JSON of `DerivedTypeCreated` was not checked for the field. The `DATE` part of the sentence belongs to group T.
- Fix: rewrite the note. `size_bytes` of a CT row holds the S7 formula for one row shape. The editor models no CT layout (A31). The vendor size of `STRING(n)` is `n + 1`, of `WSTRING(n)` is `2(n + 1)`.

#### Source X02 (Codex): LOW — code-vs-doc — c — The architecture claims an IEC size table that the domain does not provide

**Vendor evidence.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:121–122` assigns four bytes each to DATE and DATE_AND_TIME. `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:151–154` gives 81 and 162 bytes for the default strings. The reference’s §9 predicts 5 and 6 bytes for `STRING(4)` and `WSTRING(2)` at `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:421`.

**Normative evidence.** `docs/architecture/data-model.md:2325` says “The domain holds the IEC sizes in code”. The file `docs/architecture/domain-model.md:88` defines `size_bytes` as “S7 size of one element in bytes”. The file docs/adr/0017-derived-type-identity.md:61 limits layout calculations to addressed parameters.

**Code evidence.** `crates/recipe-domain/src/plant/primitive.rs:230–250` explicitly implements S7 sizes and returns 2 for DATE and 8 for DATE_AND_TIME. `crates/recipe-domain/src/plant/string_length.rs:80–89` implements S7 string formulas. `crates/recipe-domain/src/plant/plc_type.rs:203–216` checks those sizes. The non-S7 type page exits before computing layout at `crates/recipe-web/src/pages/types/tree.rs:205`. An IEC-specific size implementation was **not found** with `rg -n -i 'iec.size|size.iec|iec_byte|iec.*width|size_on_platform|size_bytes_for' crates`.

**Effect and fix.** A maintainer following the architecture can mistake an S7 metadata helper for a native-size API. Change the last sentence of `docs/architecture/data-model.md:2325` and identify the four differing examples. The export contains logical types, units, lengths, and bounds. This finding does not require a new layout calculator.

#### Source N (cross-group note, not a group N finding)

`data-model.md:2325` says: "`size_bytes` holds the S7 size ... The domain holds the IEC sizes in code." `grep -rn -i 'iec size\|iec_size\|size on codesys\|fn size_bytes\|fixed_size_bytes' crates --include=*.rs` finds only `Primitive::fixed_size_bytes` (`plant/primitive.rs:230-233`, doc "Return the S7 size"). It gives `DATE` 2 bytes. No code holds an IEC size. This concerns `DATE` (group T). It does not change the 15 rows of this group, because their S7 size equals the vendor size.

### R07 — LOW — doc gap — dimensions a, b, and g — the IEC type families have no named exclusion

This is the pattern of Siemens R04, closed for S7 by ADR-0185. `docs/architecture/data-model.md:2330` names only `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, and `ANY`. A3 (`docs/architecture/overview.md:248`) is a closed list, so it excludes every other name only implicitly. Three group results report the gap for these families:

- Group N: 17 rows (rows 2, 17, 18, 19, 21, 22, 26, 27, 28, 29, 32, 34, and 42 to 46 of its table). The families are `BIT`, the native integers in both spellings (`__XINT`, `__UXINT`, `__XWORD`), `VERSION`, and `__VECTOR`. They also include `__SYSTEM.VAR_INFO`, `SysTime`, `RTS_SYSTIMEDATE`, `T_FILETIME`, `TIMESTRUCT`, `ST_AmsAddr`, and the CODESYS Safety Extension types.
- Group A: 11 rows (rows 3, 5, 9, 12, 13, and 15 to 20 of its table). The families are the open array `ARRAY[*]`, struct `EXTENDS`, `UNION`, `INTERFACE`, function block instances, and the six `ANY_*` families.
- Codex X01: the same families, with `PVOID`, `T_FILETIME`, and the Safety Extension rows.

Two rating differences exist. Codex rates the `ANY_*` rows as covered by the explicit `ANY` exclusion. Group A rates them implicit. This report follows group A. Codex rates `SysTime` as an alias of `ULINT` (A39) from a CODESYS library page. Group N rates it implicit, because the vendor file gives no base declaration.

The fix is one new ADR that extends the note of `data-model.md` §7 for `CodesysIec` and `TwinCat3`. A50 gets a counterpart: standard application only, no Safety Extension, no TwinSAFE.

#### Source N01 (group N): LOW — doc gap — dimensions a and b — CODESYS/TwinCAT families outside the matrix have no named exclusion

- Vendor: `vendor:51` "`BIT` | 1 bit in a permitted declaration | A named `FALSE`/`TRUE` field in a structure or function block". Then `vendor:264` "TwinCAT also documents the spellings `XINT`, `UXINT` and `XWORD`. CODESYS documents the double-underscore spellings". Then `vendor:306` (`__VECTOR`), `:305`, `:310`, `:319`, `:320`, `:323`, and `:325` (system and library structs). Then `:356` "The **CODESYS Safety Extension** has its own language profile."
- Document: `docs/architecture/data-model.md:2330` "Types that are not in scope: IEC `ENUM`, subrange, and alias (Assumption A39), and `POINTER`, `REFERENCE`, `ANY`." `data-model.md:2331` names only Siemens families (ADR-0185). `overview.md:295` A50 "The editor models the standard program of an S7 CPU only." `overview.md:248` A3 is a closed list.
- Code: `crates/recipe-domain/src/plant/primitive.rs:118` `pub const ALL: [Self; 29]` has no `BIT`, native integer, vector, or `SAFE*` variant. `crates/recipe-db/tests/storage/seed.rs:40` `const SEED: [SeedRow; 31]`. `plant/type_expr.rs:215` gives a grammar error for each such name.
- Rows: 2, 17, 18, 19, 21, 22, 26, 27, 28, 29, 32, 34, 42 to 46.
- Effect: no wrong data is possible, because A3 is closed. A reader cannot tell a decision from an omission. The pattern is Siemens R04, closed for S7 by ADR-0185.
- Suggested fix: a new ADR that extends the note of `data-model.md` §7 for `CodesysIec` and `TwinCat3`. The ADR names `BIT`, the native integers in both spellings, `__VECTOR`, the compiler structures (`VERSION`, `VAR_INFO`), the library structs, and the CODESYS Safety Extension types. A50 gets a CODESYS and TwinCAT counterpart (standard application only, no Safety Extension, no TwinSAFE).

#### Source A02 (group A): LOW — doc gap — dimensions a and b — the excluded IEC families have no named exclusion

- Vendor: vendor:4 "The reference also covers every DUT category in the TwinCAT inventory". Also vendor:187 (open array), vendor:204 "Structure extension with `EXTENDS`", vendor:247 `UNION`, vendor:275 `INTERFACE` variable, vendor:276 `FUNCTION_BLOCK` instance, and vendor:284 to vendor:289 `ANY_BIT`, `ANY_INT`, `ANY_REAL`, `ANY_NUM`, `ANY_DATE`, `ANY_STRING`.
- Normative: `data-model.md:2330` names only IEC `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, `ANY`. `data-model.md:2331` "Siemens families that are not in scope (ADR-0185)" introduces the list that holds `ARRAY[*]` (`:2336`). ADR-0185 is titled "Excluded Siemens type families" and names Siemens sections only. A3 (`overview.md:248`) and A39 (`overview.md:284`, "A parameter uses a primitive, an array, or a UDT") exclude the rest only implicitly.
- Code: `0001_schema.sql:110` allows only `primitive`, `array`, `struct`. `type_expr.rs:200` refuses `*`. `type_name.rs:14` reserves `ANY_*`, `INTERFACE`, `FUNCTION_BLOCK`, `EXTENDS` as struct names but not `UNION`.
- Effect: no wrong data. A reader of `data-model.md` §7 cannot tell whether these are out of scope by decision on `CodesysIec` and `TwinCat3`. The families are `UNION`, `INTERFACE`, FB instances, `EXTENDS`, the `ANY_*` families, and the open array. This is the pattern of Siemens R04 (later ADR-0185).
- Suggested fix: one new ADR, or an amendment note in the pattern of ADR-0185. It adds an "IEC families that are not in scope" bullet to `data-model.md` §7. The bullet lists `UNION`, struct `EXTENDS`, `INTERFACE`, function block instances, the `ANY_*` generic families, and the open array `ARRAY[*]` on every platform.

#### Source X01 (Codex): LOW — doc gap — a, b, g — Several IEC-platform exclusions have no named decision

**Vendor evidence.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:51` describes BIT as “1 bit in a permitted declaration.” `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:187` describes an open-array formal. `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:247` says union alternatives begin at offset zero. `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:259–264`, `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:275–276`, `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:305–310`, and `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:356–366` identify native-width, instance, system, and optional safety families.

**Normative evidence.** `docs/architecture/overview.md:248` says “The list of supported data types per platform is the matrix.” `docs/architecture/data-model.md:2330` explicitly excludes `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, and `ANY`. The following list has the heading “Siemens families that are not in scope” at `docs/architecture/data-model.md:2331`. The safety assumption at `docs/architecture/overview.md:295` specifies an S7 CPU.

**Code evidence.** `crates/recipe-domain/src/plant/primitive.rs:118–148` enumerates the global scalar names. `crates/recipe-domain/src/plant/type_expr.rs:149–173,208–217` accepts fixed array prefixes and a restricted base. `crates/recipe-domain/src/plant/platform.rs:7–17` has no CODESYS safety profile. `migrations/0001_schema.sql:110` admits three type kinds.

**Effect and fix.** A reader can determine that the marked types are unavailable, but cannot tell whether the project considered them individually. Add an IEC-platform scope note for packed BIT, open arrays, unions, native-width/address types, instances/interfaces, system and library records, vectors, structure extension, and the Safety Extension. Preserve the existing explicit decisions for aliases, enums, pointers, references, and generics. A family-level decision suffices.

### R08 — LOW — code-vs-doc — dimension g — comments give inconsistent units of a `WSTRING` length

**Vendor evidence.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:153` specifies “Up to `n` UCS-2 code units.” `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:169` excludes supplementary characters from native WSTRING.

**Normative evidence.** Under the current editor decision, docs/adr/0137-no-nul-in-a-text-value-and-the-unit-of-a-string-length.md:62 says “A character outside the Basic Multilingual Plane counts 2.” `docs/architecture/domain-model.md:106` also defines UTF-16 code units.

**Code evidence.** `crates/recipe-domain/src/plant/string_length.rs:12` says “one UTF-16 code unit per character” and then says an outside-BMP character counts two. `crates/recipe-domain/src/plant/string_length.rs:61` calls the return value a count of characters. `crates/recipe-domain/src/plant/primitive.rs:41` says “two bytes per character.” `migrations/0001_schema.sql:95` calls the column a character count. The implementation uses `encode_utf16().count()` at `crates/recipe-domain/src/recipe/value/leaf.rs:149–153`. `crates/recipe-domain/src/recipe/value/leaf/tests.rs:63` pins the two-unit case.

**Effect and fix.** A caller using scalar count can disagree with value and limit validation. Describe the value as a capacity in the units of its string kind. Change maintained comments and SQL documentation without editing a shipped migration. OQ02 separately addresses whether the permitted repertoire should change.

Group X also cites `string_length.rs:12`. The merge spec keeps only `string_length.rs:61`, `primitive.rs:41`, and `migrations/0001_schema.sql:95`, because Siemens R05 fixed `string_length.rs:12` and `:16`. It did not fix these three places.

### R09 — LOW — missing-test — dimensions d, e, f, and h — no export golden and no literal test at the IEC ends

Count-level tests exist for each end of `TIME`, `LTIME`, `DATE`, and `DATE_AND_TIME`. The export goldens for `CodesysIec` and `TwinCat3` hold at most one `INT` or one `WSTRING`. The literal path has no test at the IEC ends of the four time types on these platforms.

#### Source N03 (group N): LOW — missing-test — dimensions f and h — no export golden holds the numeric types on `CodesysIec` or `TwinCat3`

- Vendor: `vendor:55` `LWORD`, `:75` `LINT`, `:76` `ULINT`, `:89` `LREAL` exist on both platforms.
- Document: `data-model.md:2294`, `:2301`, `:2302`, `:2304` mark them "yes" for `CodesysIec` and `TwinCat3`. `api.md:533` defines one export form for every numeric leaf.
- Code: `crates/recipe-export/tests/golden_types.rs:289` `fn platform_cases() -> [PlatformCase; 3]` holds only `S7_300`, `S7_1200`, `S7_1500`. The only `TwinCat3` golden with a numeric leaf is one `INT` with the value `"120"` (`crates/recipe-export/tests/golden.rs:449-454`). No golden uses `CodesysIec`. The 64-bit ends are pinned only in `types_S7_1500.json:205`, `:210`.
- Effect: low. The value writer has no platform branch (`leaf.rs:90`, `recipe-export/src/document.rs:141` reads the platform only for the header). A platform-specific regression in `not_on_platform` (`export/body.rs:93`) for these types would not appear. LOW, because the severity scale has no class for a missing positive test of a shared path.
- Suggested fix: add `types_CodesysIec.json` and `types_TwinCat3.json` to `platform_cases()`, with one parameter for each available primitive and the 64-bit ends.

#### Source T02 (group T): LOW, missing-test, dimensions d, e, h. The literal path and the export have no test at the IEC ends of `TIME`, `LTIME`, `DATE`, and `DATE_AND_TIME`

- Vendor evidence: line 119 "0…4,294,967,295 ms, ending at 49 d 17 h 2 min 47.295 s". Line 120 "ending at 213,503 d 23 h 34 min 33.709551615 s". Line 121 "1970-01-01…2106-02-07". Line 122 "…2106-02-07 06:28:15".
- Normative evidence: `domain-model.md:284` "The count is inside the range of the base-unit table". Also `api.md:494` "Golden-file tests compare the serializer output with the schema and with stored examples."
- Code evidence: the count ends have tests (`range/tests.rs:96`, `:102`, `:144`, `:332`). Duration literal tests run only on `S7_1500` (`duration/tests.rs:3-5`). The search `rg -n '49d17h2m47s295ms|213503d|2106-02-07' crates` finds only `range/tests.rs:152` (a calendar check). The golden type documents cover only S7 (`recipe-export/tests/golden_types.rs:289`, `platform_cases() -> [PlatformCase; 3]`). `golden/` has no `CodesysIec` or `TwinCat3` time document.
- Effect: low. The parser and `from_count` call the same `range::check` (`mod.rs:121`, `canonical.rs:85`). The writer already emits 20-digit counts (`types_S7_1500.json:274`). The class is LOW, not MEDIUM, because each boundary already has a negative test at the count level.
- Suggested fix: add IEC literal tests at each end and one step beyond: `T#49d17h2m47s295ms` and `T#49d17h2m47s296ms`, `LT#213503d23h34m33s709ms551us615ns` and `…616ns`, `D#2106-02-07` and `D#2106-02-08`, `DT#2106-02-07-06:28:15` and `…:16`. Add one golden type document for `TwinCat3` or `CodesysIec` with the eight time types.

### R10 — LOW — doc gap — dimension f — `api.md` states the bytes of `STRING` and `CHAR` but not of `WSTRING`

- Vendor: `vendor:153` "Up to `n` UCS-2 code units in target byte order, followed by a zero word". `vendor:164-165` give the LE and BE bytes.
- Document: `docs/architecture/api.md:533` "The transfer client writes each `STRING` or `CHAR` character as one byte whose value is the scalar value of the character (ISO 8859-1, ADR-0181)". No sentence covers `WSTRING` or `WCHAR`.
- Code: `crates/recipe-export/tests/golden_types.rs:562-596` exports a TwinCat3 `WSTRING` with U+1D11E as a JSON string. Search: `grep -nE "WSTRING|WCHAR|UTF-16|code unit" docs/architecture/api.md docs/adr/0109-*.md docs/adr/0181-*.md` gives only `api.md:140`, `:518`, `:521`.
- Effect: a transfer client for CT must guess the code units of a `WSTRING` (UTF-16 with surrogates, or UCS-2). It must also guess the meaning of the length (ADR-0137 (i)).
- Fix: add one sentence to the `parameters[].value` row: a `WSTRING` character is written as UTF-16 code units, the count of ADR-0137 (i). Settle OQ02 first.

### R11 — LOW — doc gap, missing-test — dimension f — no document says which array dimension is the outer JSON level

- Vendor: vendor:189 "The vendors' published three-dimensional initializer examples assign the **second** initializer to different indices". Then vendor:193 to vendor:196 show `[2,3,5]` = 20 in CODESYS and 50 in TwinCAT. The file adds vendor:198 "use measured or exported byte strides".
- Normative: `domain-model.md:443` "array: JSON array with exactly `(upper - lower + 1)` elements per dimension". `data-model.md:2417` "A two-dimensional array `ARRAY[0..1,1..2]` is a JSON array of 2 arrays of 2 elements." Neither sentence says which dimension is the outer JSON level.
- Code and tests: every test that I found uses equal counts per dimension. `parse/tests.rs:181` and `:194` use bounds `(0,1),(1,2)`. `export/parameters/tests.rs:344` uses `(1,2),(0,1)`. With equal counts, both nesting orders give the same JSON shape. `type_tree/tests.rs:84` uses 3×2, but it is a nested array, not one multi-dimensional prefix, and it checks the leaf count only. Search: `grep -n "int_array_tree\|array_of(" crates/recipe-domain/src/recipe/value/parse/tests.rs crates/recipe-domain/src/export/parameters/tests.rs`.
- Effect: the transfer system must map JSON positions to PLC indexes. The two vendors disagree on flat initializer order. A consumer that guesses the nesting order can write a value to the wrong element on one vendor. No test fails if the parser changes the order.
- Suggested fix: state in `domain-model.md` §2.1 that the first dimension is the outer JSON level, so the last index changes fastest. Add a test with an asymmetric shape, for example `ARRAY[1..2,1..3] OF INT`, that accepts `[[1,2,3],[4,5,6]]`, refuses `[[1,2],[3,4],[5,6]]`, and maps the leaf `[2,1]` to the JSON position `[1][0]`. Check the code before the doc change. This review did not run such a test.

## 4. Owner questions

### 4.1 Conflicts between a normative document and the vendor file

The review does not choose a side. A change needs a new ADR in each case. Class = the severity that the item gets if the owner accepts the vendor claim.

| Id | Class | Topic | Effect when the owner accepts the vendor claim | Source outside the vendor file |
|---|---|---|---|---|
| OQ01 | CRITICAL | `LREAL` compiled as `REAL` on some CODESYS targets | The editor exports a 15-digit `LREAL` value that such a target stores as binary32 | Official source cited by Codex, not read again |
| OQ02 | CRITICAL | `WSTRING` as UCS-2 against ADR-0137 and ADR-0180 (c) | The editor accepts and exports a character that a UCS-2 `WSTRING` cannot hold | Official sources cited by Codex, not read again |
| OQ03 | CRITICAL | `STRING` encoding UTF-8 against Latin-1 (ADR-0137 (h), ADR-0181 (b)) | A value that fits by character count can exceed the capacity in bytes | Official sources cited by Codex, not read again |
| OQ04 | CRITICAL | Minimum CODESYS version (SP16) for `LDATE`, `LDT`, `LTOD` | Below SP16 the editor exports a type that the PLC does not have | None. The vendor file only |
| OQ05 | CRITICAL | Identifier rules on these platforms: (a), (b), (c) | The consumer cannot declare the exported shape | Beckhoff keyword page, cited by Codex, not read again |
| OQ06 | CRITICAL | One-member ordinary `STRUCT` on CODESYS | The editor accepts a struct that the CODESYS compiler may refuse | CODESYS structure page, cited by Codex, not read again |
| OQ07 | LOW | Long literal prefixes (`LTIME#1us`) | The editor rejects a spelling, not a value | Official sources cited by Codex, not read again |
| OQ08 | LOW | Vendor aliases `TOD`, `DT`, `LTIME_OF_DAY`, `LDATE_AND_TIME` as request names | The editor rejects a spelling, not a value | Official source cited by Codex, not read again |
| OQ09 | LOW | Check that the editor models no byte layout on these platforms | A documentation gap | None |

REAL precision of 6 or 7 digits is the open owner question OQ08 of the Siemens review. It is outside the scope of the change `siemens-type-rules`. It applies to `REAL` on these platforms too (group N, row 15). This review raises no new question for it.

### 4.2 Owner decisions of 2026-11-19

The owner decided each question on 2026-11-19. One OpenSpec change records the decisions in new ADRs and fixes R01 to R11.

| Id | Decision |
|---|---|
| OQ01 | Add an Assumption: every `CodesysIec` machine compiles `LREAL` as an eight-byte binary64. Add a check to the commissioning runbook, as A46 does for TwinCAT. |
| OQ02 | Adopt the UCS-2 repertoire for new `WSTRING` values on `CodesysIec` and `TwinCat3`: `U+0001` to `U+D7FF` and `U+E000` to `U+FFFF`. The earlier rule set of ADR-0120 reads stored values, as ADR-0180 does on S7. |
| OQ03 | Add a `STRING` encoding per machine (Option B of ADR-0181). On `CodesysIec` and `TwinCat3`, a UTF-8 machine counts the length in UTF-8 bytes. The export gets a new field. |
| OQ04 | Add an Assumption: every `CodesysIec` machine runs CODESYS V3.5 SP16 or later. Add a check to the commissioning runbook, as ADR-0142 does for TwinCAT. |
| OQ05 | Reserve, on `CodesysIec` and `TwinCat3`, the keywords, `UNION`, `END_UNION`, and the primitive names as struct names. Reserve the native types (`BIT`, `XINT`, `__XWORD`, and the others of part (a)) as struct names too. Reserve the keywords as member names too. The earlier rule set reads stored names, as ADR-0186 does on S7. |
| OQ06 | A struct on `CodesysIec` has at least 2 members. The earlier rule set reads stored structs with one member. |
| OQ07 | Accept the long literal prefixes (`LTIME#`, `TIME#`, `DATE#`, `TIME_OF_DAY#`, `DATE_AND_TIME#`, and the long `L` forms) as input aliases. The canonical form, the counts, and the hashes do not change. |
| OQ08 | The request boundary normalizes the aliases `TOD`, `DT`, `LTIME_OF_DAY`, and `LDATE_AND_TIME` to the catalog names. The seed and the export do not change. |
| OQ09 | Record in an Assumption or an ADR that the transfer system writes each leaf of a `CodesysIec` or `TwinCat3` machine through its symbol. The editor therefore models no layout, no `pack_mode`, and no `EXTENDS`. |

### OQ01 — `LREAL` compiled as `REAL` on some CODESYS targets

Class if the owner accepts the vendor claim: CRITICAL. Group N and Codex agree. Codex gives the example value 16777217. It is exact in binary64 and passes the editor digit limit of 15. A binary32 conversion gives 16777216.

#### Part NQ01 (group N): `LREAL` on a CODESYS target whose compiler converts `LREAL` to `REAL`

- Vendor claim: `vendor:109` "CODESYS documents targets whose compiler converts `LREAL` to `REAL`. Confirm the target's floating-point support and compiled symbol size before binding an external binary64 reader." Also `vendor:101` "`LREAL` with binary64 support".
- Normative claim: `data-model.md:2304` gives `LREAL` "yes" on `CodesysIec` without a condition. ADR-0117 (a) (`0117-...:42`) "On every platform, ... an `LREAL` number at most 15." `overview.md` §9 has no Assumption on binary64 (`grep -n -i 'binary64\|IEEE\|LREAL' docs/architecture/overview.md` finds nothing). Compare A46 (`overview.md:291`), which fixes a TwinCAT version for other types.
- Code: `crates/recipe-domain/src/recipe/value/leaf.rs:26` `const LREAL_MAX_DIGITS: u8 = 15;`. `numeric.rs:25` gives `LREAL` the full `Decimal` range on every platform.
- Class if the owner accepts the vendor claim: CRITICAL. The editor accepts and exports a 15-digit `LREAL` value that such a target stores as binary32 with about 7 digits.
- Options:
  - A. New Assumption: every `CodesysIec` target supports binary64 `LREAL`. The runbook names the check before a plant adds a `CodesysIec` machine (pattern of A46). No code change.
  - B. A machine attribute "LREAL is binary32". The domain then applies the `REAL` digit rule to `LREAL`.
  - C. Apply 7 digits to `LREAL` on `CodesysIec` always.

#### Part XQ03 (Codex): Does `CodesysIec` require native binary64 for LREAL? Conditional CRITICAL

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:109` says some targets convert LREAL to REAL. This qualification applies even with the same engineering service pack. [CODESYS REAL and LREAL](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_real.html)

**Normative claim and code.** `docs/architecture/data-model.md:2304` offers LREAL on `CodesysIec`. docs/adr/0117-significant-digits-of-real-and-lreal.md:42 says “On every platform” and permits 15 significant digits for LREAL. `crates/recipe-domain/src/recipe/value/leaf.rs:57–60,231–242` checks the declared primitive. `crates/recipe-domain/src/plant/platform.rs:7–11` distinguishes no CODESYS target capability. `crates/recipe-domain/src/recipe/value/leaf/tests.rs:244` already illustrates the relevant REAL precision issue with `16777217`.

**Options.** Require a compiled eight-byte binary64 LREAL for eligible CODESYS machines, or record the target capability and apply the effective type. `16777217` is exact in binary64 and passes the editor’s LREAL digit limit. The binary32 conversion gives `16777216`. This is a concrete value change, independent of the editor’s deliberate decimal exponent limits.

### OQ02 — `WSTRING` as UCS-2 against ADR-0137 and ADR-0180 (c)

Class if the owner accepts the vendor claim: CRITICAL. The orchestrator verified that `crates/recipe-domain/src/recipe/value/leaf.rs:133` allows `char::MAX` on these platforms (the match arm sits in the block `:130-138`). The vendor file says UCS-2 at `vendor:153` and `vendor:169`. The golden file `crates/recipe-export/tests/golden/types_TwinCat3_wstring_non_bmp.json` marks the type loadable.

#### Part SQ01 (group S): `WSTRING` characters outside the BMP on `CodesysIec` and `TwinCat3`

- Vendor claim: `vendor:169` "Both vendors describe `WSTRING` as UCS-2, covering U+0000…U+D7FF and U+E000…U+FFFF, with U+0000 reserved here for termination. Supplementary characters such as 😀 require another representation, such as a UTF-8 `STRING`." `vendor:153` "Up to `n` UCS-2 code units".
- Normative claim: `docs/adr/0180-wchar-and-wstring-characters-on-s7.md:51` "(c) `CodesysIec` and `TwinCat3` keep ADR-0137 (a) and (b)". Then ADR-0137 `:55` "A `WSTRING` character is any Unicode scalar except `U+0000`". Then `:62` "A character outside the Basic Multilingual Plane counts 2."
- Code: `crates/recipe-domain/src/recipe/value/leaf.rs:132` `(true, false) => u32::from(char::MAX),`. Tests `leaf/tests.rs:459-469` (😀 passes on both CT platforms), `crates/recipe-export/tests/golden_types.rs:583-596` (U+1D11E exported with `loadable: true`).
- ADR-0180 is the S7 equivalent. Its item (c) decides CT explicitly and keeps the wider rule. The decision used only Siemens sources (`:14-15`).
- Class if the owner accepts the vendor claim: CRITICAL. The editor accepts and exports a character that a UCS-2 `WSTRING` cannot hold as one character. The PLC gets two surrogate code units in `0xD800..=0xDFFF`, outside the vendor range.
- Options:
  - (A) Keep ADR-0180 (c). Record in a new ADR that a CT `WSTRING` gets surrogate pairs, and that PLC string functions count them as 2 units.
  - (B) New ADR in the pattern of ADR-0180: on CT, each UTF-16 code unit of a `WSTRING` value is in `0x0001..=0xD7FF` or `0xE000..=0xFFFF`. The earlier rule set keeps ADR-0137 (ADR-0120). The ops query of ADR-0180 (l) gets a CT part. The golden file `types_TwinCat3_wstring_non_bmp.json` changes its expectation to a rule fault.
  - (C) As (B), but only for new writes, with a notice on the version page.

#### Part XQ01 (Codex): Should the IEC profiles enforce UCS-2 in WSTRING? Conditional CRITICAL

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:169` says “Both vendors describe `WSTRING` as UCS-2.” U+1D11E and U+1F600 have no UCS-2 representation. Both primary manuals give the BMP ranges excluding surrogate code points. [CODESYS WSTRING](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_wstring.html) · [Beckhoff WSTRING](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529437323.html)

**Normative claim and code.** docs/adr/0137-no-nul-in-a-text-value-and-the-unit-of-a-string-length.md:55 says “A `WSTRING` character is any Unicode scalar except `U+0000`.” docs/adr/0180-wchar-and-wstring-characters-on-s7.md:51 retains that rule on both platforms. `crates/recipe-domain/src/recipe/value/leaf.rs:130–138` allows `char::MAX`. `crates/recipe-domain/src/recipe/value/leaf/tests.rs:459–466` expressly accepts 😀 on both. `crates/recipe-export/tests/golden/types_TwinCat3_wstring_non_bmp.json:5,25,47` contains `loadable: true`, `WSTRING`, and `"Z𝄞"`. `crates/recipe-export/tests/golden_types.rs:586` compares writer output with that fixture.

**Options.** Adopt the native UCS-2 repertoire for new values, test U+D7FF/U+E000/U+FFFF and U+10000, and mark affected historical exports non-loadable under the current-rule check. Alternatively, define a separately qualified UTF-16 target profile backed by target evidence. Capacity alone cannot resolve this case.

### OQ03 — `STRING` encoding UTF-8 against Latin-1 (ADR-0137 (h), ADR-0181 (b))

Class if the owner accepts the vendor claim: CRITICAL. The vendor lines 161 to 167 document both encodings and count the capacity in bytes.

#### Part SQ02 (group S): Encoding and length unit of `STRING` on `CodesysIec` and `TwinCat3`

- Vendor claim: `vendor:167` "Both vendors document Latin-1 and UTF-8 interpretations of `STRING`. In CODESYS check the project compile option *UTF8 Encoding for STRING* ... TwinCAT supports `{attribute 'TcEncoding' := 'UTF-8'}` ... Capacity is measured in bytes even when a character takes two to four bytes". Also `vendor:162` "`STRING(2)` containing `é`, UTF-8 | `C3 A9 00` | The same character fills both payload bytes."
- Normative claim: ADR-0137 (h) `:61` "A `STRING(n)` length counts characters. Each character is one Latin-1 byte". ADR-0181 (b) `:43` "The transfer client writes each `STRING` or `CHAR` character as one byte. The byte value is the scalar value of the character (ISO 8859-1)." `api.md:533` repeats it for every platform. ADR-0181 Context (`:13-16`) cites only Siemens and WinCC facts.
- Code: `leaf.rs:11` `LATIN_1_MAX = 0xFF`, `leaf.rs:153` `text.chars().count()`. Test `leaf/tests.rs:25` refuses "Łódź" and `:47` accepts "éééé" in `STRING(4)`.
- Class if the owner accepts the vendor claim: CRITICAL. Under UTF-8, "éééé" passes `STRING(4)` (`leaf/tests.rs:47`, 4 characters) but needs 8 bytes, and capacity counts bytes (`vendor:162-163`, `:167`). The declared type cannot hold that value.
  - Second effect, HIGH: on a project with UTF-8 `STRING`, the editor refuses a valid value such as "Ł" (U+0141). The Latin-1 byte `E9` of ADR-0181 (b) is malformed UTF-8 there.
- Options:
  - (A) Keep Latin-1 on CT. Add an Assumption that a CT project uses Latin-1 `STRING` encoding, and state it in `api.md` §6.1.
  - (B) A per-machine encoding (Option B of ADR-0181) with a UTF-8 byte count as the length unit on CT. New field in the export.
  - (C) On CT, restrict `STRING` characters to `U+0001..=U+007F`. Latin-1 and UTF-8 then give the same bytes, and the character count equals the byte count. The cost: no "é" on CT.

#### Part XQ02 (Codex): Which STRING encoding may a commissioned target use? Conditional CRITICAL for UTF-8 targets

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:161–167` gives one payload byte for Latin-1 `é` and two for UTF-8 `é`. The declaration capacity counts bytes. TwinCAT supports the `TcEncoding` attribute. Its `UTF8#` literal requires 4026. CODESYS SP22 supports the project encoding option. [CODESYS STRING](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_string.html) · [Beckhoff STRING](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529410443.html)

**Normative claim and code.** docs/adr/0181-latin-1-bytes-of-string-and-char.md:43 requires that “The byte value is the scalar value of the character (ISO 8859-1).” `docs/architecture/domain-model.md:106` counts one Latin-1 byte per character. `crates/recipe-domain/src/recipe/value/leaf.rs:130–138,149–153` accepts `é` in `STRING(1)`. The type entry at `crates/recipe-domain/src/export/types.rs:81–89` carries its capacity. `docs/architecture/api.md:516–524` defines no encoding field.

**Options.** Retain Latin-1 and explicitly qualify eligible symbols at commissioning, or model encoding at the level where it can vary and check encoded byte lengths. On a UTF-8 `STRING(1)`, `é` cannot fit. On a Latin-1 `STRING(1)`, it fits exactly. UTF-8 literal support and runtime string encoding are distinct version questions.

### OQ04 — minimum CODESYS version for `LDATE`, `LDT`, and `LTOD`

Class if the owner accepts the vendor claim: CRITICAL. Group T classes it MEDIUM. Codex classes it CRITICAL below SP16. The orchestrator classes it CRITICAL by the rubric, because below SP16 the editor exports a type that the PLC does not have. No CODESYS twin of A46 exists. The primary profile SP22 has the three types.

#### Part TQ01 (group T): minimum CODESYS version of `LDATE`, `LDT`, and `LTOD`

- Vendor claim: line 29 "`LDATE`, `LDATE_AND_TIME`, `LTIME_OF_DAY` | Introduced as language features in V3.5 SP16." The same row gives "Available from 3.1.4026.0" for TwinCAT.
- Normative claim: the matrix gives "yes" on `CodesysIec` without a version (`data-model.md:2312`, `:2314`, `:2316`). A46 (`overview.md:291`) and ADR-0142 (a)-(d) cover only TwinCAT. The search `rg -n "SP16|SP ?1[0-9]|SP22|3\.5\.1[0-9]|CODESYS V3" docs/architecture docs/adr docs/ops` finds no match.
- Code: `0004_seed_plc_types.sql:29`, `:31`, `:33` offer the three types on `CodesysIec`. `range.rs:87`, `:89`, `:94` give them a range there with no version check.
- Class if the owner accepts the vendor claim: MEDIUM. This is the class of baseline #21, the same gap on TwinCAT. A recipe for a controller older than SP16 passes approval, and the controller cannot load it. The primary profile SP22 has the types.
- Options: (1) add an Assumption "every `CodesysIec` machine runs V3.5 SP16 or later" and a runbook check, as ADR-0142 did for TwinCAT. (2) Leave the matrix as it is and record the decision in an ADR.

#### Part XQ06 (Codex): What minimum CODESYS compiler profile does `CodesysIec` promise? Conditional CRITICAL below SP16

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:29` dates LDATE, LDATE_AND_TIME, and LTIME_OF_DAY to CODESYS SP16 and TwinCAT 4026. TwinCAT 4024 already supports LTIME.

**Normative claim and code.** `docs/architecture/overview.md:174` includes all three long calendar types on `CodesysIec`. `docs/architecture/overview.md:291` says “Every `TwinCat3` machine runs TwinCAT 3.1.4026.0 or later.” docs/adr/0142-twincat-version-of-the-long-date-types.md:45 deliberately stores no TwinCAT version. `migrations/0004_seed_plc_types.sql:29,31,33` offers the three types unconditionally, and `crates/recipe-domain/src/plant/platform.rs:7–11` supplies no version discriminator. An equivalent CODESYS minimum was **not found** with `rg -ni 'CodesysIec.*SP16|SP16.*CodesysIec|3\.5\.16|service pack 16|SP22|Codesys.*minimum version' docs/architecture docs/adr docs/ops/runbook.md`.

**Options.** Record SP22 as the supported profile, require at least SP16 for the long types, or represent capabilities for older targets. **The reviewed SP22 and 4026 profiles satisfy the feature gate.** A46 already resolves the 4024 policy. If an operator skips that check, all three unavailable types remain selectable on a 4024 machine.

### OQ05 — identifier rules of struct and member names on `CodesysIec` and `TwinCat3`

Class if the owner accepts the vendor claim: CRITICAL when the owner decides that exported names must be PLC identifiers. The consumer then cannot declare the shape. The groups gave other classes: group N MEDIUM for part (a), group A MEDIUM for parts (b) and (c), Codex CRITICAL for all three. The three parts have their own lines.

- Part (a): `BIT`, `XINT`, `__XWORD`, `PVOID`, and the library names are not reserved struct names (parts NQ02 and XQ07).
- Part (b): `UNION` and `END_UNION` are missing from the keyword list of ADR-0143 (d) (part AQ02).
- Part (c): ADR-0143 (l) lets a member name be a keyword. The Beckhoff source says that a keyword cannot be a variable name (parts AQ03 and XQ07).

#### Part NQ02 (group N, part a): vendor type names outside the matrix are not reserved struct names on `CodesysIec` and `TwinCat3`

- Vendor claim: `vendor:51` `BIT` is a type. `vendor:262` `PVOID` is a TwinCAT type. `vendor:264` "TwinCAT also documents the spellings `XINT`, `UXINT` and `XWORD`. CODESYS documents the double-underscore spellings." `vendor:305`, `:319`, `:320`, `:323`, `:325` name the library types `VERSION`, `RTS_SYSTIMEDATE`, `T_FILETIME`, `TIMESTRUCT`, `ST_AmsAddr`.
- Normative claim: `docs/adr/0143-iec-identifier-rules-of-struct-and-member-names.md:55` says "(c) ... The reserved words are the keywords of item (d) and the names of the primitives of `data-model.md` §7". `docs/adr/0186-siemens-type-names-reserved-on-s7.md:44` "(a) On `S7_300`, `S7_1200`, and `S7_1500`, a struct name is not a Siemens reserved name". No ADR reserves CODESYS or TwinCAT type names.
- Code: `crates/recipe-domain/src/plant/type_name.rs:201-208` `is_reserved_name` checks only `RESERVED_KEYWORDS` and `Primitive::ALL`. A grep for each of `BIT`, `XINT`, `UXINT`, `XWORD`, `__XINT`, `__UXINT`, `__XWORD`, `PVOID`, `VERSION`, `TIMESTRUCT` in the list of `type_name.rs` counts 0. The grammar `^[A-Za-z_][A-Za-z0-9_]{0,63}$` (`type_name.rs:212`, schema `export-v1.schema.json:127`) accepts `__XWORD`.
- Class if the owner accepts the vendor claim: MEDIUM. The editor accepts a struct name that the PLC already uses for a system or library type. Whether CODESYS or TwinCAT refuses a DUT with such a name is unverified. This is the pattern of Siemens OQ11 and ADR-0186.
- Options:
  - A. New ADR: reserve these names on `CodesysIec` and `TwinCat3`, as ADR-0186 does on S7. Add the names of R07.
  - B. Also refuse a struct name that begins with `__` on these platforms (system prefix of CODESYS).
  - C. No change. Record the decision in the ADR of R07.

#### Part AQ02 (group A, part b): `UNION` and `END_UNION` are not reserved struct names

- Vendor claim: `UNION` is a DUT form of both vendors (vendor:247 "`UNION` | All alternatives begin at offset 0"). vendor:4 covers "every DUT category in the TwinCAT inventory". The vendor file does not say that the word is a reserved keyword. The operators `LOWER_BOUND` and `UPPER_BOUND` (vendor:187) and `XSIZEOF` (vendor:31) are also absent from the list, while `SIZEOF`, `ADR`, and `BITADR` are in it.
- Normative claim: ADR-0143 (d) gives a fixed list of 182 words (`docs/adr/0143-iec-identifier-rules-of-struct-and-member-names.md:56` to `:244`). It goes from `UNTIL` to `USING`. ADR-0143:281 says "the union with the Beckhoff list covers each word of TwinCAT 3". ADR-0143 driver (line 26): "An exported type is valid in the PLC project of the machine".
- Code: `crates/recipe-domain/src/plant/type_name.rs:14` holds the 182 words without `UNION` and `END_UNION`. `grep -n '"UNION\|"END_UNION' crates/recipe-domain/src/plant/type_name.rs` gives nothing. A struct named `UNION` therefore passes on `TwinCat3`. I did not read the Beckhoff "Keywords" page, so I do not know if it lists `UNION`.
- Class if the owner accepts the vendor claim: MEDIUM (the class of baseline #9). The PLC project refuses the struct at import. No value is silently wrong.
- Options: (1) A new ADR adds `UNION` and `END_UNION`, and optionally `LOWER_BOUND`, `UPPER_BOUND`, `XSIZEOF`, to the list. Its ops query and the count test change too. (2) Keep the list as the two sources give it.

#### Part AQ03 (group A, part c): A member name can be a keyword on `CodesysIec` and `TwinCat3`

- Vendor claim: a struct member is declared like a variable, `name : TYPE;` (vendor:207 to vendor:214). The vendor file says nothing about keywords as member names.
- Normative claim: ADR-0143 (l): "A member name can be a keyword or a primitive name" (`docs/adr/0143-iec-identifier-rules-of-struct-and-member-names.md:261`). The same ADR cites the Beckhoff page: "a keyword cannot be a variable name" (`:57`). Its driver (`:26`) asks for an exported type that is valid in the PLC project. These two statements conflict inside the ADR.
- Code: `crates/recipe-domain/src/plant/type_name/tests.rs:156` `member_names_may_be_keywords` accepts `END_STRUCT`, `Int`, `array`. `type_name/tests.rs:66` accepts `INT`, `STRING`, `TIME_OF_DAY`. A struct with the member `END_STRUCT : INT;` cannot compile on either platform.
- Class if the owner accepts the vendor claim: MEDIUM, as baseline #9.
- Options: (1) A new ADR reserves the keywords (and maybe the primitive names) as member names on `CodesysIec` and `TwinCat3`. It keeps an earlier rule set for stored rows. (2) Keep ADR-0143 (l) and state why the transfer system accepts such a member.

#### Part XQ07 (Codex, parts a and c): Must exported DUT and member names be usable as PLC identifiers? Conditional CRITICAL for an unrepresentable named shape

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:204–214` shows a named DUT and declared members. `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:259–264` gives the compiler’s native type names, including `__XINT`. The Beckhoff keyword reference says keywords cannot be variable names and describes double underscores in user implementation identifiers as reserved for implicit code. [Beckhoff keywords](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529971595.html)

**Normative claim and code.** docs/adr/0143-iec-identifier-rules-of-struct-and-member-names.md:261 expressly says “A member name can be a keyword or a primitive name.” `docs/architecture/domain-model.md:85` permits underscores, including consecutive ones. `crates/recipe-domain/src/plant/type_name.rs:269–280,331–341` enforces only the identifier pattern for members. `crates/recipe-domain/src/plant/type_name/tests.rs:156` accepts `END_STRUCT`, `Int`, and `array`. `crates/recipe-domain/src/plant/type_name.rs:201–207` restricts struct names to a fixed keyword/primitive denylist. `__XINT` is absent. `crates/recipe-domain/src/export/types.rs:148–161` emits the names.

**Options.** If names must match directly declared PLC symbols, qualify vendor-specific reserved identifiers and reject incompatible new names. Preserve historical reads. If names are logical editor identifiers, specify how the consumer maps them to compiled symbols. The original #9 case-insensitive uniqueness defect is fixed. It is independent of this intentional naming policy.

### OQ06 — one-member ordinary `STRUCT` on CODESYS

Class if the owner accepts the vendor claim: CRITICAL. The source is outside the vendor file (section 1.6). The CODESYS online reference says "at least 2" declarations. The vendor file gives no minimum.

#### Part XQ08 (Codex): Does the CODESYS profile permit a one-member ordinary STRUCT? Conditional CRITICAL if the documented minimum is enforced

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:204–214` introduces ordinary structures and links the CODESYS declaration reference at `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:238`. The linked syntax table says “at least 2” declarations. Its separate BIT syntax illustrates one placeholder. The supplied vendor file gives no minimum and records no SP22 compilation of a one-member ordinary DUT. [CODESYS structure declaration](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_structure.html)

**Normative claim and code.** `docs/architecture/domain-model.md:93` specifies “1 to 1024 members.” `crates/recipe-domain/src/plant/plc_type.rs:270–279` rejects zero and more than 1024. A single ordinary member passes this count check. `crates/recipe-domain/src/plant/plc_type/tests.rs:279–295` pins the empty and upper boundaries.

**Options.** Check the one-member ordinary declaration on the selected SP22 compiler and target. Retain the current rule if it compiles. If the compiler enforces two, introduce a CODESYS-specific lower bound and a loadability rule for affected stored shapes. The available evidence does not establish a TwinCAT minimum of two.

### OQ07 — long literal prefixes such as `LTIME#`

Class if the owner accepts the vendor claim: LOW. Group T classes it LOW. Codex classes it HIGH. The orchestrator keeps LOW, because the editor rejects a spelling and not a value. The user can enter the same value as `LT#1us`.

#### Part TQ02 (group T): long literal prefixes, for example `LTIME#`

- Vendor claim: line 135 shows the literal `LTIME#1us`. Line 143 says that `T#`, `D#`, and `LD#` are short prefixes, and that `DT`, `TOD`, `LDT`, `LTOD` are also type names. The vendor file never writes `LT#`.
- Normative claim: `domain-model.md:264` "duration := ("T#" | "LT#" | "S5T#") …". `domain-model.md:277` names one prefix per type. No long form (`TIME#`, `LTIME#`, `DATE#`, `TIME_OF_DAY#`, `DATE_AND_TIME#`, `LDATE#`, `LTIME_OF_DAY#`, `LDATE_AND_TIME#`) is in the grammar.
- Code: `mod.rs:143-156` (`literal_prefix`) knows only the short forms. `canonical.rs:55` writes `LT#`. `duration/tests.rs:180` (`lower_case_prefix_is_grammar_error`) also refuses `t#5s`.
- Effect: a user who pastes `LTIME#1us` from a CODESYS or TwinCAT project gets `E-RCP-005`. The same value stays valid as `LT#1us`. The export holds the count, so the export does not change.
- Class if the owner accepts the vendor claim: LOW. The value stays valid with another spelling. Under the literal reading of HIGH ("rejects a valid value"), the owner can rate it HIGH.
- Options: (1) accept `LTIME#` and the other long forms as input aliases, and keep the canonical rendering. (2) Show `LTIME#` on IEC platforms if `LT#` is not valid there. (3) Keep the grammar and record that the UI literal is editor syntax.

#### Part XQ04 (Codex): Should duration entry and rendering accept vendor literal spellings? Conditional HIGH

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:135` gives `LTIME#1us`. Both vendors’ published constant grammars list `LTIME`/`ltime` for the long-duration prefix. They also list full `TIME` and short `T` forms for TIME. [CODESYS time constants](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operands_constant_time.html) · [Beckhoff time constants](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529294987.html)

**Normative claim and code.** `docs/architecture/domain-model.md:264,277` defines `LT#` for LTIME. `crates/recipe-domain/src/plant/iec_literal/mod.rs:143–155` recognizes `"LT"` and `"T"`, and `crates/recipe-domain/src/plant/iec_literal/canonical.rs:52–55` emits `LT#`. `crates/recipe-domain/src/plant/iec_literal/duration/tests.rs:59` accepts the editor form. A test containing the vendor spelling was **not found** with `rg -n -F 'LTIME#' crates`.

**Options.** Accept the vendor prefixes on input and select a documented vendor spelling for rendering, or label the existing grammar as editor-specific. Preserve counts and hashes. The editor currently rejects the exact vendor example even though the value 1000 ns is representable on every reviewed profile.

### OQ08 — vendor type aliases as request names

Class if the owner accepts the vendor claim: LOW. Codex classes it HIGH. The orchestrator keeps LOW for the same reason as OQ07. `data-model.md:2322` decides one name per type.

#### Part XQ05 (Codex): Should vendor type aliases resolve at the request boundary? Conditional HIGH

**Vendor claim.** `CODESYS_TwinCAT3_PLC_Data_Types_EN.md:122–126,143` states that “`DT`, `TOD`, `LDT` and `LTOD` are also type names.” It pairs the short and full forms. Beckhoff shows both declarations and literals. [Beckhoff calendar types](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529415819.html)

**Normative claim and code.** `docs/architecture/data-model.md:2322` selects one name per type and says TIME_OF_DAY and DATE_AND_TIME “are never written as `TOD` or `DT`.” `docs/architecture/domain-model.md:134,147` restricts requests to catalog names. `crates/recipe-domain/src/plant/primitive.rs:155–159,189–192` compares exact spelling. `crates/recipe-domain/src/plant/primitive/tests.rs:87–92` explicitly rejects `TOD`, `DT`, and `LTIME_OF_DAY`. The same lookup rejects `LDATE_AND_TIME`.

**Options.** Keep the four stable seed/export names and normalize these four alternate request spellings, or retain the restricted grammar. In that case, identify the excluded spellings as valid *vendor* declaration aliases. `T`, `D`, and `LD` should continue to be distinguished from declaration names. This question does not require duplicate seed rows.

### OQ09 — no byte layout on `CodesysIec` and `TwinCat3`

Class if the owner accepts the vendor claim: LOW. Group A adds the class HIGH when the transfer system writes a raw image. Group T used A31 as "no layout by decision". A31, `domain-model.md:219`, and ADR-0108 (h) only imply the decision.

#### Part AQ01 (group A): No byte layout on `CodesysIec` and `TwinCat3`

- Vendor claim: layout depends on the target, the declaration, and the packing attributes (vendor:28). `pack_mode` changes offsets and the array stride (vendor:217 to vendor:234). "Place the attribute above the DUT declaration. For CODESYS symbol export, also record any *Compatibility layout* setting" (vendor:236). Padding makes a whole-image checksum differ for equal field values (vendor:226). Multi-dimensional strides are target-specific (vendor:198, vendor:432).
- Normative claim: `S7Layout` computes the layout "on S7" (`domain-model.md:219`). A31: "A CODESYS or TwinCAT 3 parameter has no address" (`overview.md:276`). ADR-0108 (h) names the layout table only on the S7 platforms. No document says how the transfer system writes a value on these platforms, for example member by member through the symbol. So no document says why alignment, padding, and `pack_mode` do not matter.
- Code: no layout code for these platforms. `crates/recipe-web/src/pages/types/s7_layout_tests.rs:141` pins that the type page shows no layout. The export has no layout field (`export-v2.schema.json:229` to `:292`).
- Class if the owner accepts the vendor claim: LOW (doc gap) when the transfer is symbolic. HIGH when the transfer system writes a raw image, because then the export lacks the packing and alignment information that the image needs.
- Options: (1) An Assumption or an ADR note. On `CodesysIec` and `TwinCat3` the transfer system writes each leaf through its symbol. So the editor models no layout, no `pack_mode`, and no `EXTENDS`. (2) Add a layout model per platform with a `pack_mode` field of the struct. (3) No change.

## 5. Baseline status

The table lists each item `#n` of `docs/plan/review/2026-11-06-plc-type-review.md` that concerns `CodesysIec` or `TwinCat3`. The items #8 and #15 concern S7 only (data-block addresses, `S5TIME`). They carry no status here.

| Item | Status | Evidence |
|---|---|---|
| #1 (`U+0000` in `STRING` and `WSTRING`) | fixed | ADR-0137 (a) to (c). `crates/recipe-domain/src/recipe/value/leaf.rs:137` `let has_nul = text.contains('\0');`. Tests `leaf/tests.rs:116`, `:124` (TwinCat3), `:459` (both platforms), `texts_tests.rs:522` |
| #2 (`REAL` `150e-29` refused) | fixed | `recipe/value/leaf/tests.rs:324` `exact_decimal_with_trailing_zeros_below_the_scale_limit_is_stored_exact`. `limit/parse/tests.rs:552` (`"150e-29"` at `:556`). `decimal/tests.rs:82`, `:87`, `:92` |
| #3 (`INT` `1.` and 29 zeros refused) | fixed | `leaf/tests.rs:337` `fraction_of_29_zeros_is_stored_as_the_whole_number`. `limit/parse/tests.rs:554`, `:559`. `decimal/tests.rs:97` |
| #4 (stored `LDATE` limit between midnights) | fixed | `recipe/limit/parse.rs:244-250` calls `IecLiteral::from_count`. Tests `limit/parse/tests.rs:603`, `:626`, `recipe-app/src/recipe/queries/parameters_tests.rs:628`. The `S5TIME` part is S7 only |
| #5 (`dims_valid` accepts a bound array with subscript 0) | fixed | ADR-0145 (a). `migrations/0011_plc_type_rules.sql:56`. Tests `crates/recipe-db/tests/storage/constraints/type_rules.rs:37`, `:63` |
| #6 (struct named `ARRAY` inverts the class-2 key order) | fixed | `ARRAY` is reserved (`type_name.rs:14`, ADR-0143 (d)). Tests `type_expr/tests.rs:182`, `type_name/tests.rs:73`. ADR-0144 orders the keys by value. `crates/recipe-db/src/transaction/type_rows.rs:47-49` |
| #7 (`REAL` `0e99999999999` refused) | fixed | `leaf/tests.rs:343` `zero_with_an_exponent_above_the_range_of_i64_is_stored_as_zero`. `limit/parse/tests.rs:560`. `decimal/tests.rs:105`, `:111` |
| #9 (case-insensitive names and keywords) | fixed for the reported cases | ADR-0143. `0011_plc_type_rules.sql:69` to `:75`. Tests `plc_type/tests.rs:405`, `:435`, `type_name/tests.rs:73`. OQ05 (a), (b), and (c) are follow-ups |
| #10 (`STRING` and `STRING(80)` are two rows with one shape) | fixed | ADR-0146 (c). Tests `recipe/rebind/kept_member_tests.rs:210`, `:230` (CodesysIec), `:270` (TwinCat3), `rebind/level/tests.rs:119`, `:127`, `:135` |
| #11 (changed form against field aggregation) | fixed | Codex only. A cross-cutting input issue. ADR-0138 (lines 49 to 54). `crates/recipe-web/src/pages/parameters/fields_tests.rs:245` |
| #12 (unit of a `WSTRING` length) | fixed | ADR-0137 (i). `domain-model.md:106`. `leaf.rs:149-155`. Tests `leaf/tests.rs:57`, `:63`, `check/tests.rs:143`. R08 covers the comments. OQ02 covers the repertoire |
| #13 (`STRING(n)` and `WSTRING(n)` capped at 255) | settled by decision | ADR-0141 (a) to (d). Tests `string_length/tests.rs:11`, `:21`, `:66`. The editor rejects a vendor-valid declaration of 300 by decision |
| #14 (no min-1, min, max, max+1 tests) | fixed | `leaf/tests.rs:350`, `limit/parse/tests.rs:108` `integer_types_accept_their_ends_and_refuse_the_values_next_to_them`. Table of 12 types at `test_support.rs:125` |
| #16 (S7 layout reference rows `Unverified`) | obsolete for this scope | No layout exists on these platforms. OQ09 asks for the decision |
| #17 (bound error text under the earlier rules) | fixed | Codex: `recipe/limit/render.rs:16-30`, test `limit/parse/tests.rs:636`. Group T read #17 as an S7 `DATE` item. This review takes the status with evidence |
| #18 (`LTOD` range text) | fixed | `domain-model.md:253` gives `0..=86_399_999_999_999` (ADR-0139 (h)). `range.rs:40`. Test `range/tests.rs:316` |
| #19 (S7 width of `STRING(2)`) | obsolete for this scope | S7 width letters only. These platforms have no address (A31) |
| #20 (`LDT` and `LDATE` end at 2^64-1 ns) | settled by decision | ADR-0139 (a) to (c). `range.rs:32`, `:48`. Tests `range/tests.rs:263`, `:346`, `:531`, `:563`. `recipe-web/src/pages/export/platform_range_tests.rs:56`. The range 1677 to 1969 stays narrowed by decision |
| #21 (TwinCAT version of the long date types) | fixed | A46 (`overview.md:291`), `data-model.md:2324`, ADR-0142, `docs/ops/runbook.md:1371-1375`. The CODESYS twin is OQ04 |

Totals: 15 fixed (#1 to #7, #9 to #12, #14, #17, #18, #21). 2 settled by decision (#13, #20), 2 obsolete for this scope (#16, #19), 2 S7 only (#8, #15). No item is open.

## 6. Totals

### 6.1 Rows per vendor section and category

| Vendor § | Rows | SUPPORTED | EXCLUDED-BY-DECISION | GAP |
|---|---:|---:|---:|---:|
| §2 | 14 | 13 | 1 | 0 |
| §3 | 2 | 2 | 0 | 0 |
| §4 | 8 | 8 | 0 | 0 |
| §5 | 7 | 4 | 3 | 0 |
| §6 | 9 | 3 | 6 | 0 |
| §7 | 15 | 0 | 15 | 0 |
| §8 | 26 | 0 | 26 | 0 |
| §9 | 0 | 0 | 0 | 0 |
| **Total** | **81** | **30** | **51** | **0** |

§7 holds 4 rows of group N (native integers) and 11 rows of group A (indirection and generic types). §9 is a probe procedure and gives no row.

Of the 51 excluded rows, 21 cite a named decision and 30 rely on the closed catalog (A3) only. Section 2.1 gives the split per group.

Codex inventory: 85 rows. 25 S, 2 D, 58 E, 0 GAP. Codex found 0 CRITICAL, 0 HIGH, 2 MEDIUM, and 3 LOW findings.

### 6.2 Findings per severity

| Severity | Count | Ids |
|---|---:|---|
| CRITICAL | 0 | none |
| HIGH | 0 | none |
| MEDIUM | 5 | R01, R02, R03, R04, R05 |
| LOW | 6 | R06, R07, R08, R09, R10, R11 |
| **Total** | **11** | |

Owner questions: 9 (OQ01 to OQ09). Six of them carry a CRITICAL class when the owner accepts the vendor claim (OQ01 to OQ06). Three carry the class LOW (OQ07 to OQ09).
