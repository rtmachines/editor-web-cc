# Design

## Context

See proposal.md, section Why. The review `docs/plan/review/plc-s7-type-coverage-review.md` is not normative. The tests follow these normative sources:

- Layout rules, base units, and literal grammar: `domain-model.md` §2.1.
- Primitive matrix and S7 sizes: `data-model.md` §7 (A3).
- JSONB canonical text: `reliability.md` §4.4. Test levels: `reliability.md` §6.
- Export fields, type references, and base units of the document: `api.md` §6.1, §6.3.
- Layout reference table and property test: ADR-0017, section "S7-300 layout".
- ADR-0045 changes the page layout only. It changes no type, no value, and no `S7Layout` result.

Siemens sources of the review, §1.4:

- S1: "Comparison list for programming languages", Edition 02/2025, SIOS 109986777, "Overview of data types", pages 9 to 14.
- S2: "Programming Guideline for S7-1200/1500", SIOS 81318674, V1.6.

S1 gives the size of each primitive. No source in reach gives the even-byte rules of a non-optimized data block (review Q10).

Existing tests that this change extends:

| Area | File | Tests |
|---|---|---|
| Layout golden | `crates/recipe-domain/src/plant/s7_layout.rs` | `s7_layout_golden_struct_and_array_sizes`, `s7_layout_golden_leaf_roots_are_not_rounded`, `s7_members_golden_*` |
| Layout property | `crates/recipe-domain/src/plant/s7_layout/prop.rs` | `last_top_level_member_end_rounded_up_is_the_size` and five more |
| Row builders | `crates/recipe-domain/src/plant/type_tree/fixtures.rs` | `primitive_row`, `string_row`, `wstring_row`, `array_row`, `struct_row`, `on_platform`, `types_for` |
| String size | `crates/recipe-domain/src/plant/string_length.rs` | `StringLength::size_bytes` |
| Export golden | `crates/recipe-export/tests/golden.rs` (730 lines) | `writer_output_of_the_api_example_equals_its_golden_file` and others |
| JSONB writes | `crates/recipe-db/tests/storage/contract/draft/narrow_writes.rs` | `set_value_keeps_the_canonical_decimal_text` |
| JSONB reads | `crates/recipe-db/tests/storage/contract/recipe_reads/parameter_reads.rs` | `parameter_values_give_the_stored_value_of_each_requested_key_only` |

## Goals / Non-Goals

**Goals:**

- Pin the current behavior of `S7Layout`, the export writer, and the JSONB round trip. Each expected value comes from a human or from an independent source.
- Keep each "unverified" layout row visible, so a later TIA Portal check (Q10) can change its mark.

**Non-Goals:**

- No change of production code, migrations, ADRs, or `docs/architecture/`. The one exception is the stale doc comment of D7, and only with the approval of the user.
- No test of the rules of `plc-type-ranges` (proposal.md, Out of scope).
- No new dev-dependency.

## Decisions

### D1. Characterization tests and the red step

The tests check existing behavior, so they pass on the current code. The red step of TDD is a mutation check:

1. Write the test with the expected values of this design.
2. Run it. It must pass.
3. Change one expected value, run the test, and see a failure message that names the case. Restore the value.

When step 2 fails, the task stops. The task reports the case, the expected value, and the actual value as a finding. It does not change production code. It does not change the expected value to the actual value without a decision of the user.

The tests pass at once. The mutation check replaces the failing-first step of the project workflow, and this needs the acceptance of the owner.

### D2. Reference table of `S7Layout` (F08 in part)

The table is test data in a new module `crates/recipe-domain/src/plant/s7_layout/reference.rs`. `s7_layout.rs` declares it with `#[cfg(test)] mod reference;`, next to `mod prop;`.

- One shared catalog holds every type of the table. Each type has one fixed id. `PlatformTypes::collect` returns `E-PLT-013` on a repeated id, so no type appears twice.
- Each row of the table holds a name, the id of its root type, and the expected size. It also holds the expected `S7Start` and the expected rows of `s7_members`.
- Each row holds two marks: `size_source` and `layout_source`. A mark is `Siemens("S1 ...")`, `Rule("domain-model.md §2.1")`, or `Unverified("Q10")`.
- The catalog uses `PlcPlatform::S7_1500`, which holds every S7 primitive except `LDATE`. It uses `on_platform` and `types_for` of `fixtures.rs`.
- The expected sizes are the literals of the tables below. No expected value comes from production code such as `StringLength::size_bytes`.
- `string_row` and `wstring_row` of `fixtures.rs` build the string types.

Ids of the catalog. A number `n` is the id `id(n)` of `fixtures.rs`.

| Id | Type |
|---|---|
| 101 to 130 | The primitive rows P01 to P30 in the order of the table. `STRING(3)` is 118, and `WSTRING(3)` is 120 |
| 131 | `WSTRING(1)`, the type of the member `a` of C07 |
| 201 | `ST_Flags` |
| 211 | `ARRAY[1..2] OF BOOL`, the type of the member `b` of C09 |
| 212 | `ARRAY[1..3] OF BYTE`, the element of A09 |
| 213 | `ARRAY[1..3] OF BOOL`, the element of A10 |
| 301 to 313 | The roots of C01 to C13. The struct of the row C`nn` has the name `ST_Cnn` |
| 401 to 411 | The roots of A01 to A11 |
| 900 | The struct of the property test of D3. Only that test adds it |

Alternative: put the rows into the existing `struct_cases` and `array_cases`. Rejected, because those cases have no source marks. The existing golden tests stay unchanged. Some rows repeat an existing case on purpose, because each rule of §2.1 needs one row.

#### Primitive rows

The size source of each row is S1, except the storage unit of `BOOL`. The layout source of each row is "unverified, Q10", because no Siemens source states the start classes.

| Row | Type | Size | Start | Size source |
|---|---|---|---|---|
| P01 | `BOOL` | 1 | `Bit` | `domain-model.md` §2.1 storage unit. S1 gives 1 bit |
| P02 | `BYTE` | 1 | `Byte` | S1 |
| P03 | `WORD` | 2 | `Even` | S1 |
| P04 | `DWORD` | 4 | `Even` | S1 |
| P05 | `LWORD` | 8 | `Even` | S1 |
| P06 | `SINT` | 1 | `Byte` | S1 |
| P07 | `USINT` | 1 | `Byte` | S1 |
| P08 | `INT` | 2 | `Even` | S1 |
| P09 | `UINT` | 2 | `Even` | S1 |
| P10 | `DINT` | 4 | `Even` | S1 |
| P11 | `UDINT` | 4 | `Even` | S1 |
| P12 | `LINT` | 8 | `Even` | S1 |
| P13 | `ULINT` | 8 | `Even` | S1 |
| P14 | `REAL` | 4 | `Even` | S1 |
| P15 | `LREAL` | 8 | `Even` | S1 |
| P16 | `CHAR` | 1 | `Byte` | S1 |
| P17 | `WCHAR` | 2 | `Even` | S1 |
| P18 | `STRING(3)` | 5 | `Even` | S1 (`n + 2`) |
| P19 | `STRING(254)` | 256 | `Even` | S1 (`n + 2`) |
| P20 | `WSTRING(3)` | 10 | `Even` | S1 (`2n + 4`) |
| P21 | `WSTRING(254)` | 512 | `Even` | S1 (`2n + 4`) |
| P22 | `S5TIME` | 2 | `Even` | S1 |
| P23 | `TIME` | 4 | `Even` | S1 |
| P24 | `LTIME` | 8 | `Even` | S1 |
| P25 | `DATE` | 2 | `Even` | S1 |
| P26 | `TIME_OF_DAY` | 4 | `Even` | S1 |
| P27 | `LTOD` | 8 | `Even` | S1 |
| P28 | `DATE_AND_TIME` | 8 | `Even` | S1 |
| P29 | `LDT` | 8 | `Even` | S1 |
| P30 | `DTL` | 12 | `Even` | S1 |

#### Composite rows

Every composite row is "unverified, Q10" for its size and its offsets. The rows apply the rules of `domain-model.md` §2.1. `ST_Flags` is a struct of the three `BOOL` members `a`, `b`, and `c`. An offset is `byte.bit` from the start of the row.
The offsets column lists every row of `s7_members` in order, nested members included. An array row has an empty list.
A nested member counts its offset from the start of the row, not from its parent. `s7_members_golden_nested_struct_after_a_bool_starts_at_the_even_byte` pins this rule.

| Row | Type | Size | Offsets of the members | Rule of §2.1 | Finding |
|---|---|---|---|---|---|
| C01 | struct of 9 `BOOL` `a` to `i` | 2 | `a` 0.0, `b` 0.1, `c` 0.2, `d` 0.3, `e` 0.4, `f` 0.5, `g` 0.6, `h` 0.7, `i` 1.0 | `BOOL` members pack 8 per byte | |
| C02 | struct `{a BOOL, b BYTE}` | 2 | `a` 0.0, `b` 1.0 | A 1-byte member starts at the next byte | |
| C03 | struct `{a BOOL, b INT}` | 4 | `a` 0.0, `b` 2.0 | A wider member starts at the next even byte | |
| C04 | struct `{a BYTE, b CHAR, c SINT, d USINT}` | 4 | `a` 0.0, `b` 1.0, `c` 2.0, `d` 3.0 | A 1-byte element starts at any byte | |
| C05 | struct `{a BYTE, b STRING(3)}` | 8 | `a` 0.0, `b` 2.0 | A `STRING` starts on an even byte | |
| C06 | struct `{a BYTE, b WSTRING(3)}` | 12 | `a` 0.0, `b` 2.0 | A `WSTRING` starts on an even byte | F09 |
| C07 | struct `{a WSTRING(1), b BYTE}` | 8 | `a` 0.0, `b` 6.0 | A 1-byte member after a `WSTRING` starts at the next byte | F09 |
| C08 | struct `{a BYTE, b ST_Flags}` | 4 | `a` 0.0, `b` 2.0, `b.a` 2.0, `b.b` 2.1, `b.c` 2.2 | A struct starts on an even byte | |
| C09 | struct `{a BOOL, b ARRAY[1..2] OF BOOL}` | 4 | `a` 0.0, `b` 2.0 | An array starts on an even byte | |
| C10 | struct `{a BYTE, b BYTE, c BYTE}` | 4 | `a` 0.0, `b` 1.0, `c` 2.0 | A struct size is even | |
| C11 | struct `{a STRING(3), b BYTE}` | 6 | `a` 0.0, `b` 5.0 | A 1-byte member after an odd `STRING(n)` | F09 |
| C12 | struct `{a STRING(3), b INT}` | 8 | `a` 0.0, `b` 6.0 | A wider member starts on an even byte | |
| C13 | struct `{a BYTE, b DTL}` | 14 | `a` 0.0, `b` 2.0 | A wider member starts on an even byte | |
| A01 | `ARRAY[1..17] OF BOOL` | 4 | none | `BOOL` elements pack 8 per byte | |
| A02 | `ARRAY[1..3,1..3] OF BOOL` | 2 | none | `BOOL` elements pack 8 per byte over all dimensions | F09 |
| A03 | `ARRAY[1..3,1..3,1..3] OF BOOL` | 4 | none | `BOOL` elements pack 8 per byte over all dimensions | F09 |
| A04 | `ARRAY[1..3] OF BYTE` | 4 | none | 1-byte elements pack one per byte | |
| A05 | `ARRAY[1..3] OF CHAR` | 4 | none | 1-byte elements pack one per byte | |
| A06 | `ARRAY[1..4] OF STRING(3)` | 24 | none | Wider elements start on even bytes | |
| A07 | `ARRAY[1..2] OF WSTRING(3)` | 20 | none | Wider elements start on even bytes | F09 |
| A08 | `ARRAY[0..1,1..2] OF INT` | 8 | none | Wider elements start on even bytes | |
| A09 | `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` | 8 | none | The element of an array is an array. It starts on an even byte | F09, also Q09 |
| A10 | `ARRAY[1..3] OF ARRAY[1..3] OF BOOL` | 6 | none | The element of an array is an array. It starts on an even byte | F09, also Q09 |
| A11 | `ARRAY[1..2] OF ST_Flags` | 4 | none | A struct element starts on an even byte | |

Every composite row has the start class `Even`. A10 and A02 hold the same 9 bits and get different sizes. The table keeps both rows, because the TIA Portal check (Q10) must decide both.

### D3. Tests of the reference table (F08, F09)

- `reference.rs` holds the table test `s7_layout_matches_the_reference_table`. It loops over all rows. It asserts the size, the start class, and the complete list of `s7_members` for each row. The failure message names the row and its two source marks.
- A second table test `reference_table_marks_every_composite_row_unverified` asserts that every composite row carries `Unverified("Q10")`. A later change that verifies a row changes this test on purpose.
- `prop.rs` gets the property test of ADR-0017: `members_keep_their_reference_layout_in_a_random_struct`. It draws 1 to 16 root ids from the rows of the table.
- The test adds one struct row with the id 900 and the members `m01` to `m16` on top of the shared catalog.
  - Each top-level member of `s7_members` has the size and the start class of its row.
  - The first member starts at bit 0. Each later member starts at the first fitting position at or after the end of the member before it.
  - A `Bit` member fits at any bit. A `Byte` member fits at a multiple of 8 bits. An `Even` member fits at a multiple of 16 bits.
  - The end of a member is its start plus 1 bit for `Bit`. For the other classes, the end is its start plus its size in bits.
  - The size of the struct equals the end of the last member, rounded up to an even byte.
  - The table, and not the layout code, gives the size and the start class of each member.

The property states the placement rule of `domain-model.md` §2.1 once, because that rule is the subject of the property. Extra padding between two members makes the property fail.

Alternative: a property test that computes the size of each member with its own copy of the size rules. Rejected, because a second copy of the size rules repeats the same assumptions and asserts nothing new.

### D4. Export golden documents per S7 platform (F10)

A new file `crates/recipe-export/tests/golden_types.rs` holds the tests. `golden.rs` stays unchanged. It has 730 lines, and the limit is 800. The new file copies the few row builders that it needs.

- The new file also stays under 800 lines. A `Spec` value per parameter, as in `golden.rs`, passes that limit with 62 parameters.
- The file therefore holds one type table of 28 rows. The columns are the tag, the primitive, the request value, the document value, and the limit.
- A hand-written list per platform filters the table. A separate list gives the 14 `S7_300` addresses.
- The file starts with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` and a comment that names `reliability.md` §2.3, as `clippy.toml` requires.
- A limit holds `min` and `max` as text and parses each into `Decimal`. The `range` builder of `golden.rs` takes `i64`, and the `LWORD` and `ULINT` maximum does not fit.
- Three golden files: `crates/recipe-export/tests/golden/types_S7_300.json`, `types_S7_1200.json`, and `types_S7_1500.json`.
- Machine `M12`, unit `U1`, recipe `R-200`, version 1, status `Approved`. One parameter per primitive of the platform. The tag of a parameter is the type name.
- The test builds each value with `ParameterValue::parse(request, tree)` and `to_stored()`. The canonical text therefore comes from the domain and not from the test.
- Only `S7_300` parameters have an address (A31). The other platforms write `null`.

Primitives per platform, from `data-model.md` §7 (hand-written list in the test, not `Primitive::ALL`):

- `S7_300` (14): `BOOL`, `BYTE`, `WORD`, `DWORD`, `INT`, `DINT`, `REAL`, `CHAR`, `STRING(16)`, `S5TIME`, `TIME`, `DATE`, `TIME_OF_DAY`, `DATE_AND_TIME`.
- `S7_1200` (20): `BOOL`, `BYTE`, `WORD`, `DWORD`, `SINT`, `USINT`, `INT`, `UINT`, `DINT`, `UDINT`, `REAL`, `LREAL`, `CHAR`, `WCHAR`, `STRING(16)`, `WSTRING(16)`, `TIME`, `DATE`, `TIME_OF_DAY`, `DTL`.
- `S7_1500` (28): every primitive of the matrix except `LDATE`.

The writer gives a primitive name as the `type_id` of a fixed-size primitive. A string parameter gets the id of its type row and an entry in `types`:

| Type row | `type_id` | `types` entry |
|---|---|---|
| `STRING(16)` | `00000000-0000-4000-8000-000000000016` | `name` `STRING`, `kind` `Primitive`, `string_length` 16 |
| `WSTRING(16)` | `00000000-0000-4000-8000-000000000116` | `name` `WSTRING`, `kind` `Primitive`, `string_length` 16 |

The `S7_300` document has the `STRING(16)` entry only. The other documents have both entries. `types` holds no other entry.

Expected value and limit of each type. A limit is `min..max` as canonical decimal text, or `max_length`. The time counts follow the base units of `domain-model.md` §2.1. They were computed by hand from 1970-01-01 = day 0 and 2026-10-12 = day 20738.

| Tag | Request value | Value in the document | Limit in the document | Note |
|---|---|---|---|---|
| `BOOL` | `true` | `true` | none (A36) | |
| `BYTE` | `255` | `"255"` | `0..255` | |
| `WORD` | `65535` | `"65535"` | `0..65535` | |
| `DWORD` | `4294967295` | `"4294967295"` | `0..4294967295` | |
| `LWORD` | `18446744073709551615` | `"18446744073709551615"` | `0..18446744073709551615` | |
| `SINT` | `-128` | `"-128"` | `-128..127` | |
| `USINT` | `255` | `"255"` | `0..255` | |
| `INT` | `-32768` | `"-32768"` | `-32768..32767` | |
| `UINT` | `65535` | `"65535"` | `0..65535` | |
| `DINT` | `-2147483648` | `"-2147483648"` | `-2147483648..2147483647` | |
| `UDINT` | `4294967295` | `"4294967295"` | `0..4294967295` | |
| `LINT` | `-9223372036854775808` | `"-9223372036854775808"` | `-9223372036854775808..9223372036854775807` | |
| `ULINT` | `18446744073709551615` | `"18446744073709551615"` | `0..18446744073709551615` | |
| `REAL` | `1.5e2` | `"150"` | `0..200` | 3 significant digits |
| `LREAL` | `-0.1250` | `"-0.125"` | `-1..1` | |
| `CHAR` | `"é"` (U+00E9) | `"é"` | none | Pins Q06 |
| `WCHAR` | `"Ω"` (U+03A9) | `"Ω"` | none | |
| `STRING` | `"Zone é"` | `"Zone é"` | `max_length` `16` | `STRING(16)` row |
| `WSTRING` | `"Z𝄞"` (U+1D11E) | `"Z𝄞"` | `max_length` `16` | `WSTRING(16)` row. Pins Q07 |
| `S5TIME` | `"S5T#1s"` | `"1000"` | `0..9990000` | |
| `TIME` | `"T#-5s"` | `"-5000"` | `-2147483648..2147483647` | S7 `TIME` is signed |
| `LTIME` | `"LT#1m30s"` | `"90000000000"` | `-9223372036854775808..9223372036854775807` | |
| `DATE` | `"D#2026-10-12"` | `"20738"` | `20454..20818` | 2026-01-01 to 2026-12-31 |
| `TIME_OF_DAY` | `"TOD#12:30:00.5"` | `"45000500"` | `0..86399999` | |
| `LTOD` | `"LTOD#12:30:00.000000001"` | `"45000000000001"` | `0..86399999999999` | |
| `DATE_AND_TIME` | `"DT#2026-10-12-12:30:00"` | `"1791808200"` | `1767225600..1798761599` | 2026-01-01 to 2026-12-31-23:59:59 |
| `LDT` | `"LDT#2026-10-12-12:30:00.000000001"` | `"1791808200000000001"` | `0..9223372036854775807` | |
| `DTL` | `"DTL#2026-10-12-12:30:00.5"` | `"1791808200500000000"` | `0..9223372036854775807` | |

No value or limit hits a rule of `plc-type-ranges`. No date is in 2169. No `REAL` has more than 7 significant digits.

Addresses of `S7_300` in `DB10`. Each address is aligned, and no two parameters overlap. The last byte is 57.

| Tag | Address | Tag | Address |
|---|---|---|---|
| `BOOL` | `DB10.DBX0.0` | `CHAR` | `DB10.DBB18` |
| `BYTE` | `DB10.DBB1` | `STRING` | `DB10.DBB20` |
| `WORD` | `DB10.DBW2` | `S5TIME` | `DB10.DBW38` |
| `DWORD` | `DB10.DBD4` | `TIME` | `DB10.DBD40` |
| `INT` | `DB10.DBW8` | `DATE` | `DB10.DBW44` |
| `DINT` | `DB10.DBD10` | `TIME_OF_DAY` | `DB10.DBD46` |
| `REAL` | `DB10.DBD14` | `DATE_AND_TIME` | `DB10.DBB50` |

Tests of `golden_types.rs`:

1. `each_s7_type_document_holds_the_expected_value_and_limit_of_each_type`: the parsed writer output against the table above. This test does not read the golden files.
2. `each_s7_type_document_has_one_parameter_for_each_primitive_of_its_platform`: the `type_id` values against the hand-written lists above. The two string rows use the ids of the table above. The test also asserts the `types` entries of that table.
3. `writer_output_of_each_s7_type_document_equals_its_golden_file`: byte for byte, as in `golden.rs`.
4. `each_s7_type_golden_file_passes_the_v1_schema`.
5. `consumer_rebuilds_content_sha256_of_each_s7_type_golden_file`: the test builds the hashed body of `api.md` §6.1 from the golden file. It computes the SHA-256 of the JCS form with `serde_json_canonicalizer` and `sha2`, not with `recipe-domain`.

Tests 1, 2, and 5 are independent of the writer. A golden file can therefore start as writer output, but only after tests 1 and 2 pass.

### D5. JSONB round trip of the 64-bit extremes (F11)

The cases go into the contract suite, so each case runs on `MemoryStore` and on PostgreSQL with `testcontainers`. Storage does not check a value against its type. The cases therefore use the `DINT` parameter of the fixtures, as `set_value_keeps_the_canonical_decimal_text` does. `texts_tests.rs` of `recipe-domain` already checks the domain text of these extremes.

Values (review F11):

| Leaf | Text |
|---|---|
| `ULINT`, `LWORD` maximum | `18446744073709551615` |
| `LINT` minimum, `LTIME` minimum in ns | `-9223372036854775808` |
| `LDT` maximum in ns | `9223372036854775807` |

- `set_value_keeps_the_64_bit_extremes` in `narrow_writes.rs`: each text alone, then one object `{"ulint", "lword", "lint", "ltime", "ldt"}` with all five values. The read is `read_draft`, the decoder of `parameter_rows.rs`.
- `parameter_values_keep_the_64_bit_extremes` in `parameter_reads.rs`: the read is `parameter_values`, the decoder of `parameter_reads.rs`.
- `replace_limits_keeps_the_64_bit_extremes` in `narrow_writes.rs`: the limit `-9223372036854775808..18446744073709551615` through the `NUMERIC` columns of `parameter_limit`.

- Optional: `insert_parameter_keeps_the_64_bit_extremes_in_the_image` in `draft/insert.rs`. It reads the five values back from the audit image of `insert` (`draft_writes.rs` line 189). Task 3.4 adds the case only when `insert.rs` stays under 800 lines.

Each case compares the read JSON with the written JSON and the `to_string()` text of each number. Equality of `serde_json::Value` alone can hide a format change.

Each case uses a fixture number that no other case of its directory uses. The cases in `draft/` use `live_draft` 19, 46, and 47. The case in `recipe_reads/` uses `plant(h, 35, "M35")`.

### D6. Model and order of the tasks

The groups 1, 2, and 3 touch different crates and files, so they can run in parallel. Inside group 1, the tasks run in order, because they share `reference.rs`. Every task that writes a test goes to a Sonnet agent. Only the comment task 1.5 is `[mechanical]`.

### D7. Stale doc comment

The doc comment of `S7Layout::of` (`s7_layout.rs` lines 29 to 31) says that the reference table "is not in the repository yet". After group 1 this sentence is wrong. Task 1.5 changes the comment only. The user approves or strikes this task when the user approves `tasks.md`.

The new comment says that the composite rows of the table are not verified until the TIA Portal check (Q10). It names the path `s7_layout/reference.rs` as plain text. An intra-doc link to a `cfg(test)` module fails `cargo doc` with `-D warnings`.

## Risks / Trade-offs

- Risk: the reference table pins rules that a TIA Portal check can disprove. Mitigation: each composite row carries `Unverified("Q10")`, and a test keeps that mark visible. A later change replaces the mark with the TIA Portal source.
- Risk: the change closes F08 and F09 only in part, because ADR-0017 asks for a table from the Siemens manual. Mitigation: Q10 stays an open item of the review.
- Risk: a hand-computed count in D4 can hold an arithmetic error. Mitigation: a task checks a time count once with the `time` crate before it reports a finding.
- Risk: a golden file that starts as writer output asserts nothing alone. Mitigation: tests 1, 2, and 5 of D4 are independent oracles.
- Risk: the tests pin the behavior of Q06, Q07, and Q09, and the owner can change it. Mitigation: the test comments name the question. A decision on the question then changes the test on purpose.
- Trade-off: the new `golden_types.rs` repeats some builders of `golden.rs`. `golden.rs` is near the file limit and is the test vector of `docs/user/check-export-document.md`.
