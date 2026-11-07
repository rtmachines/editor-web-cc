# Review record: PLC data types, 2026-11-06

- Date: 2026-11-06.
- Base commit: `47731e8` on main.
- Scope: the handling of PLC data types in the editor. This covers type names and platforms, S7 size and address rules, value and limit parsing, canonical text, SQL constraints, the web form, and the export. The scope also covers a check of the vendor manuals of Siemens, CODESYS, and Beckhoff.
- Reviewers: four Opus reviewers with the default effort (slices A to D) and one Codex reviewer (gpt-6-astra, effort high). The review lead removed duplicate findings.
- Evidence: the review lead checked #1 to #3 and #5 by tracing the code. The lead checked #6 by reading the lock key code. The lead also checked #1 and #5 with queries in a `postgres:16.15-bookworm` container. The tracked files in `D:\editor-web-cc` did not change. Codex ran with `--write`, but `git status` shows only the two changes that existed before the review.
- Result: the code review found no CRITICAL finding. The vendor check added the CRITICAL finding #20.
- This file is not normative.
- Fix plan: [2026-11-06-plc-type-fix-plan.md](../2026-11-06-plc-type-fix-plan.md). It keeps the number `#n` of each finding.

## 1. Findings

The table shows the final severity and the final verdict of each finding. The vendor check changed #8, #13, and #18 and added #20 and #21. The failure scenario of #8, #13, #18, #20, and #21 comes from the vendor check.

| # | Severity | Verdict | Kind | Location | Rule | Failure scenario |
|---|---|---|---|---|---|---|
| 1 | HIGH | `CONFIRMED` | doc | `domain-model.md:219` ↔ `data-model.md` §3.12. Code: `recipe/value/leaf.rs:132`, `pages/parameters/fields.rs:163` | §2.1 range of `CHAR`/`WCHAR`. Storage of the value as `JSONB` | Input: `CHAR` with `value=%00`. Actual: the domain accepts it and PostgreSQL rejects the write with `unsupported Unicode escape sequence \u0000`. Expected: one consistent contract. NUL in `STRING` and `WSTRING` has the same fault. The review did not check the error code that the user gets. |
| 2 | HIGH | `CONFIRMED` | code | `recipe/value/decimal.rs:74-76` (callers: `value/leaf.rs:182` and `limit/parse.rs:226`) | `reliability.md` §4.4, ADR-0117 (b) | Input: REAL `150e-29` (or `100e-30`). Actual: `E-RCP-005`, and `E-RCP-006` as a limit bound. Expected: accept it as `1.5e-27`. Cause: `normalize()` does not remove the trailing zeros of the integer part. The scale becomes 0+29, which is more than 28. |
| 3 | HIGH | `CONFIRMED` | code | `decimal.rs:20,22` (`from_str_exact`) | §4.4, ADR-0117 (b) | Input: INT `1.00000000000000000000000000000` (29 zeros). Actual: `E-RCP-005`. Expected: accept it as `1`. The code rejects a scale above 28 before it removes the zeros. |
| 4 | HIGH¹ | `CONFIRMED` | code | `recipe/limit/parse.rs:85-101, 233-252` | §2.1 steps of `S5TIME`, full day of `LDATE`. ADR-0120 | Input: a stored limit `S5TIME max=15` (or `11`) on S7_1500, or `LDATE max=1` ns. Actual: `from_stored` checks only the range. The approval check and `loadable` pass the limit, and the export can hold `"max":"15"`. Expected: rejection, as for a value. `ParameterValue::from_stored` calls `check_count`, but `ProcessLimit::from_stored` does not. |
| 5 | HIGH¹ | `CONFIRMED` | sql | `migrations/0001_schema.sql:78` (`dims_valid`), `:144` | ADR-0017, A38, §2.1 `lower ≤ upper`, `upper−lower < 65536` | Input: `dims_valid('[0:0]={5}','{0}')` and `dims_valid('[0:0]={0}','{65536}')`. Actual: both give `t`. Expected: `f`. `upper_b[0]` is NULL, so `NOT EXISTS` gives true. The function has no condition `array_lower(..,1)=1`. The unique index can also accept a duplicate shape. |
| 6 | HIGH | `CONFIRMED` | code | `plant/type_name.rs:89-96`, `recipe-app/src/ports/types.rs:50-56`, `plant/derived_lock.rs:24-33` | ADR-0017, `data-model.md` §1.4 (order of the class-2 keys) | Input: `DefineUdt ARRAY` with a member `STRING(5)` takes the keys `[ARRAY, STRING]`. A parallel `AddParameter ARRAY[1..2] OF STRING(5)` takes `[STRING, ARRAY]`. Both commands use the same `hashtext(machine:ARRAY)`, so the lock order inverts and `40P01` can occur. Expected: the system rejects the name `ARRAY`, or the keys do not collide. |
| 7 | HIGH | `CONFIRMED` | code | `decimal.rs:69-70` | §4.4 | Input: REAL `0e99999999999`. Actual: `E-RCP-005`. Expected: accept it as `0`. The code parses the exponent into `u32` before it checks for zero. This is an edge case. |
| 8 | HIGH | `CONFIRMED` | doc | `domain-model.md:189`, `plant/s7_address.rs:132` | §2.1 S7Address, ADR-0116 (c) | Input: `DB60000.DBW0` to `DB65535.DBW0` on S7_1500. Actual: accepted. Expected: rejected. The CPU 1517-3 PN/DP manual (edition 11/2023) gives the range *"1 ... 60 999"*. It splits the range into *"number range that can be used by the user: 1 ... 59 999"* and *"number range of DBs created via SFC 86: 60 000 ... 60 999"*. The numbers 61000 to 65535 do not exist. For S7-1200 the system manual gives 1 to 59999, but the review knows it only from a secondary copy (manualslib). The S7-1500 manuals of 2014 gave 1 to 65535, so the range depends on the firmware. |
| 9 | MEDIUM | `CONFIRMED` | doc | `domain-model.md` §2.1 (grammar of `name` and `member`). Code: `type_name.rs:69,93`, `plc_type.rs:245-254`, `0001_schema.sql:159-161` | IEC 61131-3: identifiers have no case sensitivity, and the standard reserves its keywords | Input: the structs `Int`, `OF`, and `STRUCT`, the members `speed`, `Speed`, and `SPEED`, and `ST_Zone` with `st_zone` on one machine. Actual: the system accepts all of them (Rust and SQL agree). Expected: on the platform these names are duplicates or keywords. The documents have no rule to compare names. |
| 10 | MEDIUM | `CONFIRMED` | doc | §2.1 table of shape identity ↔ ADR-0017. Code: `ensure_derived.rs:240-243`, `rebind/level.rs:40-55` | ADR-0017, ADR-0018 | Input: on CodesysIec, `STRING` and `STRING(80)` are two rows with one shape. A UDT revision that only rewrites `STRING(80)` as `STRING`. Actual: the revision forces a new value (`E-RCP-028`) and rejects the limit (`E-RCP-029`). |
| 11 | MEDIUM | `CONFIRMED` | doc | `api.md:214`, ADR-0041 (a) ↔ ADR-0039 (b). Code: `pages/parameters/fields.rs:256-261`, `new.rs:170` | ADR-0041 | Input: `tag=bad tag&bogus=1&value[*]=2`. Actual: one line `E-WEB-008` for the first bad key. Expected: according to `api.md`, all bad fields at once. |
| 12 | MEDIUM | `CONFIRMED` | doc | `domain-model.md:93` ↔ `reliability.md:473`. Code: `leaf.rs:109`, `check.rs:204` | `StringLength`, ADR-0028 | Input: `WSTRING(1)` with the value "😀". Actual: the code counts 2 UTF-16 units and rejects the value. `domain-model.md` says "number of characters" (1). `reliability.md` says "byte length… UTF-16 units". |
| 13 | HIGH | `CONFIRMED` | doc | `domain-model.md:97`, `string_length.rs:96-99` | §2.1 `StringLength`, ADR-0115 | Input: `STRING(300)` or `WSTRING(300)` on CodesysIec or TwinCat3. Actual: `E-PLT-013`. Expected: accept it. CODESYS says: *"The length of the strings is however not fundamentally limited"*. The limit 255 applies only to functions of the `Standard` library. TwinCAT says: *"TwinCAT does not limit the string length in principle, but some string functions only process lengths of 1-255"*. ADR-0115 does not record this narrowing as a deliberate choice. |
| 14 | MEDIUM | `CONFIRMED` | test | `recipe/value/parse/tests.rs:156`, `limit/parse/tests.rs:85-104` | Brief: every boundary has a test | No test checks min−1, min, max, and max+1 through `ParameterValue::parse` and `ProcessLimit::parse` for the 11 integer types. The behavior meets the rules, because the probe passes. |
| 15 | MEDIUM | `CONFIRMED` | test | `iec_literal/canonical.rs:38-43`, `canonical/tests.rs`, `prop.rs:42-57` | §2.1 steps of `S5TIME` | No test rejects `10_010`, `100_100`, or `1_001_000`. A change of `9_990` to `10_000` would pass all tests. |
| 16 | MEDIUM | `CONFIRMED` | test | `plant/s7_layout/reference.rs:63,242,397` | ADR-0017, §2.1 S7Layout | All 24 reference rows for struct and array have `Unverified("Q10")`. A02 and A10 give different sizes for 9 BOOL bits (Q09). No test uses a Siemens source. |
| 17 | LOW | `CONFIRMED` | code | `recipe/value/check.rs:230`, `limit/parse.rs:163` | §2.1 (messages show the literal), ADR-0120 (d) | Input: DATE on S7_1500 under the rule set `Earlier`, count 72_700. Actual: the message shows `value: "72700", max: "72690"`. Expected: the literal `D#2169-01-18`. `bound_text` always uses `Current`. |
| 18 | LOW | `CONFIRMED`, resolved | doc | `domain-model.md:231` ↔ `iec_literal/range.rs:34,75` | §2.1 table of units, row `LTOD` | The row gives the range 0 to 2^64−1 and also says "below one day". Part 1 could not tell which range applies. The vendor check resolved this. CODESYS, Beckhoff, and the Siemens Guideline all end `LTOD` at `23:59:59.999999999`. The code (`range.rs:34`) agrees with the platforms. The ranges "0 to 2^64−1" and "0 to 2^63−1" in this row do not match the platform. |
| 19 | LOW | `CONFIRMED` | doc | `data-model.md:2048` ↔ rows `:2067-2068` | §7, S7 width | The prose says "D for 4 bytes", so `STRING(2)` (4 bytes) would get `D`. The table and the code give `B`. |
| 20 | CRITICAL | `CONFIRMED` | doc | `domain-model.md:229,233`. Code: `plant/iec_literal/range.rs:29` (`LDATE`), `:39` (`LDT_UNSIGNED`) | §2.1 table of base units ↔ platform | Input: `LDT#2300-01-01-00:00:00` (or `LD#2554-07-21`) on TwinCat3 or CodesysIec. Actual: accepted, and it can reach an approved export. Expected: rejected. CODESYS and Beckhoff give this range for LDT: `LDT#1677-9-21-0:12:43.145224192` to `LDT#2262-4-11-23:47:16.854775807`. They give this range for LDATE: `LD#1677-9-22` to `2262-4-11`. The same mismatch has another side. The dates 1677 to 1969 are valid on the platform. The editor rejects them, because a count is never negative (A43). No ADR records this narrowing. |
| 21 | MEDIUM | `CONFIRMED` | doc | `data-model.md:2074,2076,2078` (column TwinCat3) | §7 type matrix | Beckhoff says about the data types LDATE, LDATE_AND_TIME (LDT), and LTIME_OF_DAY (LTOD): *"TwinCAT version 3.1.4026.0 or later is required"*. The matrix has "yes" without a version. No document in `docs/architecture` or `docs/adr` names a TwinCAT version. A recipe with LDT for a machine with TwinCAT 3.1.4024 passes the approval, but the machine cannot load it. |

¹ #4 and #5 apply only to data that the domain did not write (direct SQL, a damaged row). The rebind does not open a path to #4, because `kept_members` compares `type_id`, so a changed type always removes the limit. Codex rated #4 as CRITICAL. The review lead rates it HIGH, because no application path stores such a limit. The check exists to find such a row.

## 2. Coverage matrix

Columns: name and platform, S7 size and alignment, parse, range per platform, canonical text, limit, JSON export, SQL ↔ Rust.

The rows include the changes of the vendor check. Section 2.1 lists those changes.

| Row | Name/platform | S7 size | Parse | Range | Canonical | Limit | Export | SQL ↔ Rust |
|---|---|---|---|---|---|---|---|---|
| BOOL | ok | ok | ok | n/a | ok | ok | ok | ok |
| BYTE, WORD, DWORD, LWORD, SINT, USINT, INT, UINT, DINT, UDINT | ok | ok | #3 #7 #14 | ok (#14) | ok | #3 #14 | ok | ok |
| LINT, ULINT | ok | ok | #3 #7 #14 | ok | ok | #3 | ok | ok |
| REAL, LREAL | ok | ok | #2 #3 #7 | ok | ok | #2 #3 | ok | ok |
| CHAR, WCHAR | ok | ok | ok | ok | ok | ok | ok | #1 |
| STRING | #10 #13 | #19 | ok | #13 | ok | ok | ok | #1 |
| WSTRING | #10 #13 | ok | #12 | #13 | ok | #12 | ok | #1 |
| TIME, LTIME, DATE, TIME_OF_DAY, DATE_AND_TIME, DTL | ok | ok | ok | ok | ok | #17 | ok | ok |
| LDT | #21 | ok | ok | #20 | ok | #17 | ok | ok |
| LTOD | #21 | ok | ok | #18 | ok | #17 | ok | ok |
| LDATE | #21 | ok | ok | #20 | ok | #4 #17 | ok | ok |
| S5TIME | ok | ok | #15 | ok | ok | #4 #17 | ok | ok |
| Array | n/a | #16 | ok | ok (i64, 1 to 3 dimensions, below 65536) | ok | ok | ok | #5 |
| Struct | #6 #9 | #16 | ok | ok (1 to 1024 members) | ok | ok (rebind `E-RCP-029`) | ok | ok (agree, but see #9) |
| S7Address of a leaf | #8 | ok | ok | #8 | n/a | n/a | not checked (nobody read the address in the export) | ok |
| Web form (key → LeafPath → value) | n/a | n/a | #11 | n/a | ok | ok | n/a | n/a |

The seed `0004_seed_plc_types.sql` (31 rows) agrees with §7 cell by cell. The triggers and the codes `E-PLT-030..036` agree between `reliability.md` §1.2, `data-model.md` §4.11, and `error_codes.rs`.

### 2.1 Changes by the vendor check

| Row | Column | Before | After |
|---|---|---|---|
| LDATE, LDT | range | ok | #20 |
| LDATE, LDT, LTOD | name/platform | ok | #21 |
| STRING, WSTRING | range | #13 (PLAUSIBLE) | #13 (`CONFIRMED`) |
| S7Address | name/platform, range | #8 (PLAUSIBLE) | #8 (`CONFIRMED`) |
| DTL | name/platform | not checked for S7-300 | ok |

## 3. Not checked

Part 1 lists the points that the code review did not check. The vendor check updated the list.

### 3.1 Code review

- Tests of `recipe-db` (Docker, testcontainers): the reviewers did not run them. The review lead ran two queries in a separate container instead.
- `export/`: the reviewers read only `parameters.rs` (JSON form) and `hash.rs` in full. They searched the other files with grep only: `body`, `facts`, `scope`, `listing`, `types`, `type_reference`, `sha256`, and `error`.
- `recipe/value/`: the reviewers only looked at `walk`, `leaves`, `prop`, `expected`, and `texts`.
- `recipe/limit/`: the reviewers only looked at `process`, `list`, and `fault`.
- `recipe/rebind/`: nobody read `build.rs`, `shape.rs`, `mod.rs`, or lines 1 to 99 of `apply.rs`.
- `recipe-app/src/recipe/update_definition`: nobody read lines 1 to 94, `errors.rs`, or `requests.rs`.
- `recipe-web/src/pages/parameters/`: the reviewers checked how the page shows values only in part. They read `fields.rs`, `prepare.rs`, and the grouping in `grid/walk.rs`. Nobody read `grid/limits.rs`, `grid/texts.rs`, `form_view.rs`, `reply.rs`, `value.rs`, `page.rs`, or the translation files. The reviewers read only the called trace of `edit.rs` and `new.rs`.
- Test files only, not the logic: `type_tree/{limits,prop,strategy,fixtures}.rs` and `leaf_path/resolve.rs`.
- The S7Address of a leaf in the export: nobody read it there.

### 3.2 Vendor check

- WSTRING and LTIME in a DB with standard access: still open. The Siemens Guideline says only that a DB without optimization shows an offset. The CPU 1517 manual says: *"For DBs with absolute addressing, the max. size is 64 KB"*. This agrees with `offset 0..=65535`. No source lists the types that such a DB allows or forbids. A test in TIA Portal or the Siemens entry 67655611 can settle this.
- WSTRING on S7-1200: open. The Guideline has a WCHAR/WSTRING table with the heading *"Time data types (only S7-1500)"*. This looks like a copy error. The Siemens comparison list shows the availability as icons that the text does not carry. The row of §7 ("yes" for S7_1200) stays undecided.
- The S7-1200 data block range 1 to 59999: the review knows it only from a secondary copy (manualslib). The range of the S7-1500 depends on the firmware.
- The availability columns in the Siemens PDFs are graphics or a broken table layout, so `pdftotext` distorts them. The reviewer read them from the text layout and not from a clean table.
- Checked now, no longer open: DTL on S7-300, and the ranges of LDATE, LTOD, and LDT on CODESYS and TwinCAT (#18, #20, #21).
- The part 1 reviewers worked offline and read no manual. The vendor check read the manuals. The extracted PDF texts are in the scratchpad of the review session and not in the repository.

## 4. Vendor sources

The report gives no publication date for the CODESYS and Beckhoff pages. The dates below are the editions that the report names.

### 4.1 Siemens

| Source | Edition and id | URL |
|---|---|---|
| CPU 1517-3 PN/DP Equipment Manual | Edition 11/2023. Source of #8 | <https://support.industry.siemens.com/cs/attachments/90471765/s71500_cpu1517_3_pndp_manual_en-US_en-US.pdf> |
| Programming Guideline for S7-1200/1500 | V1.6 (12/2018). The file name starts with the id 81318674. Tables 2-9, 2-12, 4-1, and 4-2 | <https://cache.industry.siemens.com/dl/files/040/90885040/att_970576/v1/81318674_Programming_guideline_DOC_v16_en.pdf> |
| Comparison list for S7-300, S7-400, S7-1200, S7-1500 | The report gives no edition | <https://cache.industry.siemens.com/dl/files/648/109797648/att_1067421/v1/s71500_compare_table_en.pdf> |
| S7-1200 Easy Book | The report gives no edition | <https://media.distrelec.com/Web/Downloads/nu/al/s71200_eng_manual.pdf> |
| S7-1200 System Manual, page 858 (DB 1 to 59999), copy at Manualslib | Secondary copy | <https://www.manualslib.com/manual/1453610/Siemens-S7-1200.html?page=858> |
| Siemens entry 67655611 | Open point: types in a non-optimized DB. The report does not state that anybody read this entry | none |

### 4.2 CODESYS

| Page | URL |
|---|---|
| Data type DATE, DT, TOD, LDATE, LDT, LTOD | <https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_date_and_time_of_day.html> |
| Data type TIME, LTIME | <https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_time.html> |
| Data type STRING | <https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_string.html> |
| Data type WSTRING | <https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_wstring.html> |

### 4.3 Beckhoff InfoSys

| Page | URL |
|---|---|
| Date and time data types | <https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529415819.html> |
| TIME, LTIME | <https://infosys.beckhoff.com/content/1033/tc3_plc_intro/12189021579.html> |
| STRING | <https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529410443.html> |
| WSTRING | <https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529437323.html> |

### 4.4 Items that agree with the vendor sources

| Item | Result | Source |
|---|---|---|
| DTL on S7-300 | ok: §7 has "no", as Siemens does | Programming Guideline v1.6, table 4-2: DTL is "no" on S7-300/400 and "yes" on S7-1200 and S7-1500. Section 3.2 has a note on the availability columns. |
| LWORD, LINT, ULINT, LTIME, and LTOD only on S7-1500. SINT, USINT, UINT, UDINT, and LREAL not on S7-300. S5TIME not on S7-1200 | ok: all of §7 agrees | Guideline, tables 2-9 and 4-1 |
| TIME and LTIME on CODESYS and TwinCAT without a sign (0..2^32−1 ms, 0..2^64−1 ns) | ok | Beckhoff: *"handled internally like a UDINT"*. The upper bound of LTIME is `213503d23h34m33s709ms551us615ns`. The CODESYS page gives the same bounds. |
| LTIME on S7-1500 with a sign | ok | Guideline, table 2-12: `LT#-106751d23h47m16s854ms775us808ns` to `+…807ns` |
| DATE and DT on CODESYS and TwinCAT, 1970 to 2106-02-07 | ok | CODESYS, Beckhoff |

### 4.5 Side notes without a finding

- Prefix `LD#`: TwinCAT allows it. The CODESYS help example has the comment *"LD as data typ prohibited"*, but the table on the same page uses `LD#`. The prefix affects only how the UI shows a value, not the export.
## 5. Proving tests

The four review worktrees hold the proving tests. The worktrees are in `D:/editor-web-cc/.claude/worktrees/` and the tests are not committed. Each subsection below names one worktree, the findings that it proves, and the added test code. The code is the added lines of the git patch of that worktree.

### 5.1 Worktree `agent-a610ab86663e928fd`

Findings: #2, #3, #7, and #14.

Command: `cargo test -p recipe-domain --lib probe_`. The tests `probe_a`, `probe_b`, and `probe_c` fail, and the failure is the proof of #2, #3, and #7. The report names `probe_a` as `Err(Mismatch{NumberDigits{Real,28}})`. The tests `probe_d` and `probe_f` check the ends of the integer types. They pass, so they show that the behavior of #14 meets the rules and that only the tests are missing. The test `probe_e` checks the limit bound of #2. The report does not state the result of `probe_a2`, `probe_e`, `probe_d`, or `probe_f` separately.

File `crates/recipe-domain/src/recipe/limit/parse/tests.rs`:

```rust
// ---- slice A probes (review only) ----
#[test]
fn probe_e_limit_bound_150e_minus_29_on_real() {
    let tree = primitive_tree(REAL);
    assert!(parse(&range("", Some("1.5e-27"), None), &tree).is_ok());
    let result = parse(&range("", Some("150e-29"), None), &tree);
    assert!(result.is_ok(), "150e-29 bound rejected: {result:?}");
}

#[test]
fn probe_f_limit_ends_of_lint_ulint_lword() {
    use crate::plant::fixtures::primitive_row;
    use crate::recipe::value::test_support::tree;
    let cases = [
        (Primitive::Lint, "-9223372036854775809", "-9223372036854775808", "9223372036854775807", "9223372036854775808"),
        (Primitive::Ulint, "-1", "0", "18446744073709551615", "18446744073709551616"),
        (Primitive::Lword, "-1", "0", "18446744073709551615", "18446744073709551616"),
        (Primitive::Sint, "-129", "-128", "127", "128"),
        (Primitive::Byte, "-1", "0", "255", "256"),
    ];
    for (primitive, below, low, high, above) in cases {
        let t = tree(vec![primitive_row(100, primitive)], 100);
        assert!(parse(&range("", Some(low), Some(high)), &t).is_ok(), "{primitive}");
        assert!(matches!(parse(&range("", Some(below), None), &t), Err(LimitFault::OutsideTypeRange { .. })), "{primitive}");
        assert!(matches!(parse(&range("", None, Some(above)), &t), Err(LimitFault::OutsideTypeRange { .. })), "{primitive}");
    }
}
```

File `crates/recipe-domain/src/recipe/value/leaf/tests.rs`:

```rust
// ---- slice A probes (review only) ----
#[test]
fn probe_a_real_150e_minus_29_equals_1_5e_minus_27() {
    let tree = primitive_tree(REAL);
    assert_eq!(
        stored_text("1.5e-27", &tree),
        "0.0000000000000000000000000015"
    );
    let result = ParameterValue::parse("150e-29", &tree);
    assert!(result.is_ok(), "150e-29 rejected: {result:?}");
}

#[test]
fn probe_a2_int_100e_minus_30_is_rejected_although_it_is_1e_minus_28_times_100() {
    let tree = primitive_tree(REAL);
    let result = ParameterValue::parse("10e-29", &tree);
    assert!(result.is_ok(), "10e-29 rejected: {result:?}");
}

#[test]
fn probe_b_int_one_with_29_fraction_zeros() {
    let tree = primitive_tree(crate::recipe::value::test_support::INT);
    let result = ParameterValue::parse("1.00000000000000000000000000000", &tree);
    assert!(result.is_ok(), "1.000..0 (29 zeros) rejected: {result:?}");
}

#[test]
fn probe_d_integer_ends_end_to_end() {
    use crate::plant::fixtures::primitive_row;
    use crate::recipe::value::test_support::tree;
    let cases: [(Primitive, &str, &str, &str, &str); 12] = [
        (Primitive::Sint, "-129", "-128", "127", "128"),
        (Primitive::Usint, "-1", "0", "255", "256"),
        (Primitive::Byte, "-1", "0", "255", "256"),
        (Primitive::Int, "-32769", "-32768", "32767", "32768"),
        (Primitive::Uint, "-1", "0", "65535", "65536"),
        (Primitive::Word, "-1", "0", "65535", "65536"),
        (Primitive::Dint, "-2147483649", "-2147483648", "2147483647", "2147483648"),
        (Primitive::Udint, "-1", "0", "4294967295", "4294967296"),
        (Primitive::Dword, "-1", "0", "4294967295", "4294967296"),
        (
            Primitive::Lint,
            "-9223372036854775809",
            "-9223372036854775808",
            "9223372036854775807",
            "9223372036854775808",
        ),
        (Primitive::Ulint, "-1", "0", "18446744073709551615", "18446744073709551616"),
        (Primitive::Lword, "-1", "0", "18446744073709551615", "18446744073709551616"),
    ];
    for (primitive, below, low, high, above) in cases {
        let t = tree(vec![primitive_row(100, primitive)], 100);
        assert!(ParameterValue::parse(below, &t).is_err(), "{primitive} {below}");
        assert_eq!(stored_text(low, &t), low, "{primitive}");
        assert_eq!(stored_text(high, &t), high, "{primitive}");
        assert!(ParameterValue::parse(above, &t).is_err(), "{primitive} {above}");
        // typed and based literals are not JSON numbers
        for text in ["16#FF", "INT#5", "2#1010", "1_000"] {
            assert!(ParameterValue::parse(&format!("\"{text}\""), &t).is_err());
        }
    }
    // REAL / LREAL digit ends and -0
    let real = primitive_tree(REAL);
    assert_eq!(stored_text("9999999", &real), "9999999");
    assert!(ParameterValue::parse("9999999.1", &real).is_err());
    assert_eq!(stored_text("-0.0", &real), "0");
    assert_eq!(stored_text("-0", &primitive_tree(crate::recipe::value::test_support::INT)), "0");
    let lreal = primitive_tree(LREAL);
    assert_eq!(stored_text("999999999999999", &lreal), "999999999999999");
    assert!(ParameterValue::parse("9999999999999991", &lreal).is_err());
    assert!(ParameterValue::parse("NaN", &real).is_err());
    assert!(ParameterValue::parse("\"NaN\"", &real).is_err());
    assert!(ParameterValue::parse("\"Infinity\"", &real).is_err());
}

#[test]
fn probe_c_real_zero_with_a_huge_exponent() {
    let tree = primitive_tree(REAL);
    let result = ParameterValue::parse("0e99999999999", &tree);
    assert!(result.is_ok(), "0e99999999999 rejected: {result:?}");
}
```

### 5.2 Worktree `agent-a61222304058196ed`

Findings: #4 and #17.

Command: `cargo test -p recipe-domain --lib probe_b_ -- --nocapture`. The output shows `S5TIME limit 15 => true`, `S5TIME value 15 => false`, `LDATE limit 1 => true`, and `LDATE value 1 => false`. This proves #4. The test `probe_b_earlier_out_of_limit_message_shows_raw_count` prints the faults of #17. The test `probe_b_print` in the new file prints the parse result of many time literals and asserts nothing. The report does not link that output to a finding. Each new file needs a `mod` line in its parent module. This record does not show those lines.

File `crates/recipe-domain/src/plant/iec_literal/probe_b_tests.rs` (new file):

```rust
//! Probe tests of slice B (review only).

use super::*;
use crate::plant::{PlcPlatform as P, Primitive as T};

fn c(t: T, p: P, text: &str) -> Result<i128, IecLiteralReason> {
    IecLiteral::parse(TimeType::of(t, p).unwrap(), text).map(|l| l.count())
}

fn show(t: T, p: P, count: i128) -> String {
    IecLiteral::from_count(TimeType::of(t, p).unwrap(), count, RuleSet::Current)
        .map(|l| l.to_string())
        .unwrap_or_else(|e| format!("ERR {e:?}"))
}

#[test]
fn probe_b_print() {
    let cases: &[(T, P, &str)] = &[
        (T::Time, P::CodesysIec, "T#-0ms"),
        (T::Time, P::CodesysIec, "T#-1ms"),
        (T::Time, P::CodesysIec, "T#49d17h2m47s295ms"),
        (T::Time, P::CodesysIec, "T#49d17h2m47s296ms"),
        (T::Time, P::S7_1200, "T#24d20h31m23s647ms"),
        (T::Time, P::S7_1200, "T#24d20h31m23s648ms"),
        (T::Time, P::S7_1200, "T#-24d20h31m23s648ms"),
        (T::Time, P::S7_1200, "T#-24d20h31m23s649ms"),
        (T::Time, P::S7_1200, "T#25h"),
        (T::Time, P::S7_1200, "T#1d25h"),
        (T::Time, P::S7_1200, "T#1h60s"),
        (T::Time, P::S7_1200, "T#0h90m"),
        (T::Time, P::S7_1200, "T#1s1s"),
        (T::Time, P::S7_1200, "T#9999999999999999999d"),
        (T::Time, P::S7_1200, "T#99999999999999999999d"),
        (T::Ltime, P::CodesysIec, "LT#213503d23h34m33s709ms551us615ns"),
        (T::Ltime, P::CodesysIec, "LT#213503d23h34m33s709ms551us616ns"),
        (T::Ltime, P::CodesysIec, "LT#-1ns"),
        (T::Ltime, P::S7_1500, "LT#-106751d23h47m16s854ms775us808ns"),
        (T::Ltime, P::S7_1500, "LT#-106751d23h47m16s854ms775us809ns"),
        (T::Ltime, P::S7_1500, "LT#106751d23h47m16s854ms775us807ns"),
        (T::Ltime, P::S7_1500, "LT#106751d23h47m16s854ms775us808ns"),
        (T::S5Time, P::S7_300, "S5T#5ms"),
        (T::S5Time, P::S7_300, "S5T#9s990ms"),
        (T::S5Time, P::S7_300, "S5T#10s"),
        (T::S5Time, P::S7_300, "S5T#10s10ms"),
        (T::S5Time, P::S7_300, "S5T#1m39s900ms"),
        (T::S5Time, P::S7_300, "S5T#1m40s"),
        (T::S5Time, P::S7_300, "S5T#1m40s100ms"),
        (T::S5Time, P::S7_300, "S5T#16m39s"),
        (T::S5Time, P::S7_300, "S5T#16m40s"),
        (T::S5Time, P::S7_300, "S5T#16m41s"),
        (T::S5Time, P::S7_300, "S5T#2h46m30s"),
        (T::S5Time, P::S7_300, "S5T#2h46m40s"),
        (T::S5Time, P::S7_1200, "S5T#1s"),
        (T::Date, P::S7_1500, "D#1989-12-31"),
        (T::Date, P::S7_1500, "D#1990-01-01"),
        (T::Date, P::S7_1500, "D#2168-12-31"),
        (T::Date, P::S7_1500, "D#2169-01-01"),
        (T::Date, P::S7_300, "D#2100-02-29"),
        (T::Date, P::S7_300, "D#2096-02-29"),
        (T::Date, P::CodesysIec, "D#2106-02-07"),
        (T::Date, P::CodesysIec, "D#2106-02-08"),
        (T::Ldate, P::CodesysIec, "LD#2554-07-21"),
        (T::Ldate, P::CodesysIec, "LD#2554-07-22"),
        (T::TimeOfDay, P::S7_1200, "TOD#23:59:59.999"),
        (T::TimeOfDay, P::S7_1200, "TOD#24:00:00"),
        (T::TimeOfDay, P::S7_1200, "TOD#23:59:60"),
        (T::Ltod, P::S7_1500, "LTOD#23:59:59.999999999"),
        (T::Ltod, P::S7_1200, "LTOD#00:00:00"),
        (T::DateAndTime, P::S7_300, "DT#1990-01-01-00:00:00"),
        (T::DateAndTime, P::S7_300, "DT#1989-12-31-23:59:59"),
        (T::DateAndTime, P::S7_300, "DT#2089-12-31-23:59:59"),
        (T::DateAndTime, P::S7_300, "DT#2090-01-01-00:00:00"),
        (T::DateAndTime, P::S7_1200, "DT#2000-01-01-00:00:00"),
        (T::DateAndTime, P::CodesysIec, "DT#2106-02-07-06:28:15"),
        (T::DateAndTime, P::CodesysIec, "DT#2106-02-07-06:28:16"),
        (T::Ldt, P::S7_1500, "LDT#1970-01-01-00:00:00"),
        (T::Ldt, P::S7_1500, "LDT#1969-12-31-23:59:59.999999999"),
        (T::Ldt, P::S7_1500, "LDT#2262-04-11-23:47:16.854775807"),
        (T::Ldt, P::S7_1500, "LDT#2262-04-11-23:47:16.854775808"),
        (T::Ldt, P::CodesysIec, "LDT#2554-07-21-23:34:33.709551615"),
        (T::Ldt, P::CodesysIec, "LDT#2554-07-21-23:34:33.709551616"),
        (T::Dtl, P::S7_1200, "DTL#2024-02-29-12:00:00.123456789"),
        (T::Dtl, P::S7_1200, "DTL#2023-02-29-12:00:00"),
        (T::Dtl, P::S7_1200, "DTL#1970-01-01-00:00:00"),
        (T::Dtl, P::S7_300, "DTL#1970-01-01-00:00:00"),
    ];
    for (t, p, text) in cases {
        println!("{t:?} {p:?} {text} => {:?}", c(*t, *p, text));
    }
    println!("show S5 10000 => {}", show(T::S5Time, P::S7_300, 10_000));
    println!("show S5 15 => {}", show(T::S5Time, P::S7_300, 15));
    println!("show LTIME s7 min => {}", show(T::Ltime, P::S7_1500, -9_223_372_036_854_775_808));
    println!("show LTIME iec max => {}", show(T::Ltime, P::CodesysIec, 18_446_744_073_709_551_615));
}
```

File `crates/recipe-domain/src/recipe/value/probe_b_check_tests.rs` (new file):

```rust
//! Probe tests of slice B (review only).

use rust_decimal::Decimal;
use serde_json::json;

use crate::plant::fixtures::{id, machine, primitive_row, types_for};
use crate::plant::{LeafPath, PlcPlatform, Primitive, RuleSet, TypeTree};
use crate::recipe::limit::{ProcessLimit, StoredLimit};
use crate::recipe::value::{ParameterValue, StoredValue};

fn tree_on(platform: PlcPlatform, primitive: Primitive) -> TypeTree {
    TypeTree::resolve(
        id(8),
        &types_for(machine(), platform, vec![primitive_row(8, primitive)]),
    )
    .unwrap()
}

fn row(min: Option<i64>, max: Option<i64>) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: min.map(Decimal::from),
        max: max.map(Decimal::from),
        max_length: None,
    }
}

#[test]
fn probe_b_earlier_out_of_limit_message_shows_raw_count() {
    let tree = tree_on(PlcPlatform::S7_1500, Primitive::Date);
    let value =
        ParameterValue::from_stored(&StoredValue::from(json!(72_700)), &tree, RuleSet::Earlier)
            .unwrap();
    let limit = ProcessLimit::from_stored(&row(None, Some(72_690)), &tree, RuleSet::Earlier)
        .unwrap();
    let faults = value
        .check_all_under(&tree, &[limit], RuleSet::Earlier)
        .unwrap_err();
    println!("OUT_OF_LIMIT {:?}", faults.first());
    let order = ProcessLimit::from_stored(&row(Some(72_800), Some(72_700)), &tree, RuleSet::Earlier)
        .unwrap_err();
    println!("MIN_GT_MAX {order:?}");
}

#[test]
fn probe_b_stored_time_limit_skips_step_and_day_rules() {
    let s5 = tree_on(PlcPlatform::S7_1500, Primitive::S5Time);
    let limit = ProcessLimit::from_stored(&row(None, Some(15)), &s5, RuleSet::Current);
    let value = ParameterValue::from_stored(&StoredValue::from(json!(15)), &s5, RuleSet::Current);
    println!("S5TIME limit 15 => {:?}", limit.is_ok());
    println!("S5TIME value 15 => {:?}", value.is_ok());
    let ldate = tree_on(PlcPlatform::CodesysIec, Primitive::Ldate);
    let limit = ProcessLimit::from_stored(&row(None, Some(1)), &ldate, RuleSet::Current);
    let value = ParameterValue::from_stored(&StoredValue::from(json!(1)), &ldate, RuleSet::Current);
    println!("LDATE limit 1 => {:?}", limit.is_ok());
    println!("LDATE value 1 => {:?}", value.is_ok());
    assert!(limit.is_ok());
}
```

### 5.3 Worktree `agent-a5201ad7f441cd395`

Findings: #6, #9, and #10.

Command: `cargo test -p recipe-app proof_`. The tests with the prefix `proof_` in `recipe-domain` are part of the same patch. The test `proof_struct_named_array_shares_the_array_key_and_inverts_the_lock_order` and the test `proof_type_expr_reads_a_struct_named_array` prove #6. The test `proof_string_and_string_80_on_codesys_resolve_to_two_rows` proves #10. The tests `proof_member_names_that_differ_only_in_case_are_accepted` and `proof_type_name_accepts_iec_keywords_and_lower_case_primitive_names` prove #9.

File `crates/recipe-app/src/plant/derived_lock/tests.rs`:

```rust
#[test]
fn proof_struct_named_array_shares_the_array_key_and_inverts_the_lock_order() {
    // DefineUdt of a struct `ARRAY` with a member `STRING(5)`.
    let define = derived_type_locks(Some(&name("ARRAY")), [&expr("STRING(5)")]);
    let define_keys: Vec<&str> = define.iter().map(DerivedTypeLock::key_text).collect();
    assert_eq!(define_keys, ["ARRAY", "STRING"]);
    // AddParameter of `ARRAY[1..2] OF STRING(5)` on the same machine.
    let add = derived_type_locks(None, [&expr("ARRAY[1..2] OF STRING(5)")]);
    let add_keys: Vec<&str> = add.iter().map(DerivedTypeLock::key_text).collect();
    assert_eq!(add_keys, ["STRING", "ARRAY"]);
    // Both commands take hashtext(machine || ':ARRAY') and hashtext(machine || ':STRING'),
    // in opposite orders.
}

#[test]
fn proof_type_expr_reads_a_struct_named_array() {
    let parsed = TypeExpr::parse("ARRAY@1").unwrap();
    assert!(parsed.arrays().is_empty());
}
```

File `crates/recipe-app/src/plant/ensure_derived/tests.rs`:

```rust
#[tokio::test]
async fn proof_string_and_string_80_on_codesys_resolve_to_two_rows() {
    let mut h = harness(PlcPlatform::CodesysIec).await;
    let seed = h.ensure("STRING").await.unwrap();
    let sized = h.ensure("STRING(80)").await.unwrap();
    assert_ne!(seed, sized);
    let array_seed = h.ensure("ARRAY[1..2] OF STRING").await.unwrap();
    let array_sized = h.ensure("ARRAY[1..2] OF STRING(80)").await.unwrap();
    assert_ne!(array_seed, array_sized);
}
```

File `crates/recipe-domain/src/plant/plc_type/tests.rs`:

```rust
#[test]
fn proof_member_names_that_differ_only_in_case_are_accepted() {
    // IEC 61131-3 identifiers are case-insensitive: `speed` and `Speed` are one name.
    let members = vec![member("speed", 1), member("Speed", 2)];
    assert!(PlcType::try_from(udt(1, members)).is_ok());
}
```

File `crates/recipe-domain/src/plant/type_name/tests.rs`:

```rust
#[test]
fn proof_type_name_accepts_iec_keywords_and_lower_case_primitive_names() {
    for text in ["ARRAY", "Array", "int", "Int", "string", "OF", "STRUCT"] {
        assert!(TypeName::parse(text).is_ok(), "{text}");
    }
    assert!(TypeName::parse("INT").is_err());
}
```

### 5.4 Worktree `agent-aa663ce572f0a533e`

Findings: #9 and #8.

Command: run the tests with the prefix `review_c_`. The report gives no exact command. The tests for type names and members prove #9. The test `review_c_data_block_60000_to_65535_pass_on_s7_1200_and_s7_1500` proves #8. The report also lists #19 for this worktree, but the patch holds no test for #19.

File `crates/recipe-domain/src/plant/plc_type/tests.rs`:

```rust
#[test]
fn review_c_members_that_differ_only_in_case_are_accepted() {
    let members = vec![member("Speed", 1), member("speed", 2), member("SPEED", 3)];
    assert!(PlcType::try_from(udt(1, members)).is_ok());
}

#[test]
fn review_c_members_named_like_iec_keywords_are_accepted() {
    let members = vec![member("REAL", 1), member("Int", 2), member("END_STRUCT", 3)];
    assert!(PlcType::try_from(udt(1, members)).is_ok());
}
```

File `crates/recipe-domain/src/plant/type_name/tests.rs`:

```rust
#[test]
fn review_c_type_names_that_are_iec_keywords_are_accepted() {
    for text in ["Int", "real", "Time_Of_Day", "STRUCT", "END_TYPE", "ARRAY", "OF"] {
        assert!(TypeName::parse(text).is_ok(), "{text}");
    }
}
```

File `crates/recipe-domain/src/recipe/draft/s7/tests.rs`:

```rust
#[test]
fn review_c_data_block_60000_to_65535_pass_on_s7_1200_and_s7_1500() {
    let int = tree_of(INT);
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        for text in ["DB60000.DBW0", "DB65535.DBW0"] {
            let result =
                super::check_definition(platform, &tag("SPEED"), Some(address(text)), &int);
            assert!(result.is_ok(), "{platform:?} {text}");
        }
    }
}
```

### 5.5 Container queries

The findings #1 and #5 have no test in a worktree. The review lead checked them with queries in a `postgres:16.15-bookworm` container. The function `dims_valid` in the container is a 1:1 copy of the one in `migrations/0001_schema.sql`.

- #1: the write of a JSONB value with `\u0000` fails with `unsupported Unicode escape sequence \u0000`.
- #5: `dims_valid('[0:0]={5}','{0}')` gives `t`, and `dims_valid('[0:0]={0}','{65536}')` gives `t`. The expected result is `f` in both cases.
