# Code review of the change codesys-twincat-type-rules (2026-11-20)

## Scope and result

- Scope: `git diff e633b45 b4c6fa2` (all 57 tasks).
- Reviewers: Opus (0 CRITICAL, 0 HIGH, 0 MEDIUM, 5 LOW) and Codex gpt-6.2-astra high (0 CRITICAL, 3 HIGH, 5 MEDIUM, 1 LOW), in parallel and read only.
- No finding put stored data or a stored hash at risk.
- The owner decided F2, F4, and the LOW items on 2026-11-20.
- All fixes are merged into `feat/codesys-twincat-type-rules` at `d570924`. The gate of task 12.1 passed there: 9902 tests passed, 0 failed, 2 ignored. Clippy, fmt, `cargo doc`, `migration_rewrite.py`, `openspec validate --strict`, and ste-lint (0 hard) passed.

## Fixes

| Fix | Findings | Commit | Result |
|---|---|---|---|
| F1 | CR-C01 | 771d6e0 | The UDT form links a reserved struct name to `type_name` and a reserved member name to its name input. |
| F2 | CR-C02, CR-O01, CR-C08 | d9694ee | Both proposal lists take the latest revision of each name first and leave out a refused name. |
| F3 | CR-C03, CR-O04 | de2ddcc | Unit tests pin that the storage decoder refuses a stored `TOD` row with `E-DB-008`. |
| F4 | CR-C04, CR-O05 | 16fee9e | The answer of `409 E-PLT-054` swaps an empty form with the same id out of band, so the encoding form goes away. |
| F5 | CR-C05 | e221ce1 | The rollback query test has a fourth case: a machine created with `Utf8` gives `(1, 0)`. |
| F6 | CR-C06 | 1f60afd | The IEC matrix test rereads the stored `content_sha256` and `scope_sha256` after the upgrade and after the restore. |
| L1 | CR-C09 | 4bd1bda | `design.md` shows `check_codesys_members(platform, name, members)`. The same commit records F2 and F4 in `design.md` and the spec deltas. |
| L2 | CR-O02 | 6ed2a50 | `StringEncoding` has no `Default`. |
| L3 | CR-O03 | 36016df | A test pins the notice of a Retired version on a `Utf8` machine. |
| L4 | CR-C07 | 202a078, 1ad1431, bcfbfce, b7c1840 | `GetParameterForm::execute` has 46 lines. The test files that this change grew past 800 lines are split. |

## Triage

## Triage of the code review of `codesys-twincat-type-rules` (2026-11-20)

Sources: `cr-opus.md` (Opus: 0 CRITICAL, 0 HIGH, 0 MEDIUM, 5 LOW, verdict ready) and `cr-codex.md` (Codex gpt-6.2-astra high: 3 HIGH, 5 MEDIUM, 1 LOW, verdict not ready). Scope `e633b45..b4c6fa2`.

No finding puts stored data or a stored hash at risk. The orchestrator checked the code of F1 and F6.

### Fix (MEDIUM, one Sonnet high agent for each finding)

| Fix | Findings | Severity | Decision |
|---|---|---|---|
| F1 | CR-C01 | MEDIUM (Codex HIGH) | Confirmed: `check_iec_names` and `check_codesys_members` give `type_expr` = struct name. `refused_input` in `pages/types/define.rs:219` only matches a member type, so `ReservedName`, `ReservedMemberName`, and `TooFewMembers` get no link and no line under an input. This breaks ADR-0041 (d), (e), (g). Fix in `recipe-web`: map the reason to the input that failed. |
| F2 | CR-C02 = CR-O01, CR-C08, CR-C07 (production part) | MEDIUM (Codex HIGH, Opus LOW) | The parameter form filters before it takes the latest revision. The UDT form takes the latest first. Use the rule of the UDT form in both lists: take the latest revision of each name, then drop it if refused. This also removes the scan of old revisions (CR-C08). `GetParameterForm::execute` has 57 lines: split it below 50 lines in the same fix. |
| F3 | CR-C03 = CR-O04 (N6) | MEDIUM (Codex HIGH, Opus LOW) | The test of the task 7.3 bullet only calls the parser. Add a test that sends a stored type text `TOD` through the real storage decoder (`recipe-db`, `transaction/type_rows.rs`) and asserts the corrupt-row error. |
| F4 | CR-C04 = CR-O05 (N3) | MEDIUM (Opus LOW) | After `409 E-PLT-054`, the answer swaps the editable encoding form back in. ADR-0202 (p) offers the change only while the machine has no version. Fix: the answer of `StringEncodingFrozen` removes the form, and a test asserts it. |
| F5 | CR-C05 (N8) | MEDIUM | Add a fourth case to `rollback_query.rs`: a machine created with `Utf8` through `CreateMachine` gives `(1, 0)`. Keep case 2 as it is. |
| F6 | CR-C06 | MEDIUM | ADR-0202 (j2) protects the stored `scope_sha256` of the `VersionExported` entries. The matrix test only checks a fresh export. Read the stored entry after the upgrade and after the restore, and compare it with the fixture. |

### LOW (ask the owner first)

- L1 CR-C09 = N5: `design.md` shows `check_codesys_members(platform, members)`. The code takes `(platform, name, members)`.
- L2 CR-O02: `StringEncoding` derives `Default` (`Latin1`). Only its own test uses it. `design.md` rejects a default.
- L3 CR-O03 (N2): no test for a Retired version on a `Utf8` machine (only the retired notice).
- L4 CR-C07 (test part): `golden.rs` has 1091 lines. Five other test files have more than 800 lines, and `codesys_type_table` has 118 lines. No project rule sets these limits. The source is the global code style rule of the owner.
- L5 (earlier, still open): the CHAR/WCHAR comments question.

### Notes N1 to N9

- N1 OK (both). N2 = L3. N3 = F4. N4 OK (both). N5 = L1. N6 = F3. N7 OK (both). N8 = F5. N9 OK (both).

### Run plan

Three chains in their own worktrees from `b4c6fa2`, each task in series, one Sonnet high agent for each fix:

- Chain X (`recipe-web`): F1, then F4.
- Chain Y (`recipe-app`, `recipe-db`): F2, then F3.
- Chain Z (`recipe-db` tests): F5, then F6.

### Update after the advisor check (2026-11-20)

- F2 waits for the owner. `require_current_type` does not refuse an older revision, and the spec scenarios use only revision 1, so the rule for a refused latest revision is an owner decision. The split of `GetParameterForm::execute` (57 lines) moves to L4.
- F4 waits for the owner. ADR-0202 (p) says "The page offers the change only while item (d) permits it". ADR-0041 (e) says a form answer swaps the form out of band. The web-ui scenario "Frozen encoding through HTMX" asserts only `409 E-PLT-054` and `HX-Retarget`.
- F3 is a unit test of the decoder in `crates/recipe-db/src/transaction/type_rows/tests.rs`. It does not touch `tests/storage/`.
- F6 also rereads the stored `recipe_version.content_sha256`.
- Launched (Sonnet high): F1 in chain X, F3 in chain Y, F5 in chain Z. F6 follows F5 in chain Z.
- After the merge: run the seven commands of task 12.1 again.

### Owner answers 2026-11-20

- F2: "Pomiń nazwę". Both lists take the latest revision of each name first, then leave out the name when that revision is refused (the rule of the UDT form now).
- F4: "Usuń formularz". The answer of `409 E-PLT-054` puts the line into `#error-panel` and removes the encoding form out of band. ADR-0202 (p) is the authority: the page offers the change only while item (d) permits it. The web-ui delta gets one more `AND` line.
- LOW: do L1, L2, L3, and L4 now.

### Final run plan

- Chain X (`recipe-web`): F1, F4, L3.
- Chain Y (`recipe-db` unit, `recipe-app`): F3, F2, L4a (split `GetParameterForm::execute` below 50 lines).
- Chain Z (`recipe-db` storage tests): F5, F6, L4b-db (split `tests/storage/platform_ranges.rs`, 815 lines).
- Chain W (`/home/rtm/editor-web-cc-ctr-cr-w`, branch `ctr/cr-w`): L2, L4b-domain (`plant/error/tests.rs` 833, `recipe/limit/parse/tests.rs` 803, `recipe/value/leaf/tests.rs` 824), L4b-export (`tests/golden.rs` 1091, `codesys_type_table` 118 lines), D1 (Haiku: `design.md` signature of L1, the rules of F2 and F4 in `design.md` and the spec deltas).
- The test files of L4b crossed 800 lines in this change (753, 775, 778, 482, 733 lines at `e633b45`).

## Opus report

### Code review of `codesys-twincat-type-rules` (Opus, CR-O)

Scope: `git diff e633b45 HEAD` on branch `feat/codesys-twincat-type-rules`, HEAD `b4c6fa2`, worktree `/home/rtm/editor-web-cc-codesys-review`. Read-only review. I ran no `cargo`, `docker`, or writing `git` command (brief, Limits). The gate result of task 12.1 comes from the brief. I did not run it again.

#### Summary

- CRITICAL 0, HIGH 0, MEDIUM 0, LOW 5.
- I checked the encoding command, the migration and its grants, the machine and draft reads, the value rules of ADR-0201 and ADR-0202, the export (v1 `loadable`, v2 field and scope hash), the IEC name and member rules, the aliases of ADR-0205 and ADR-0206, the v2 schema, the two ops queries, and the web route, form, page, and notices.
- No stored hash changes. A `Latin1` scope body keeps the bytes of the old `MachineBody`. The v1 hashed body does not get the encoding, and each of the three v1 routes has a `loadable: false` test (`pages/export/adr_0202_tests.rs`). Lock order, first-statement permission check, read-only stop, audit entry through `commit`, and the error-code rows are correct.
- The five findings are a proposal-list difference, an unused `Default`, two test gaps, and a stale form after `409`.
- Verdict: ready to archive. The LOW findings can go to a follow-up.

#### Findings

##### CR-O01 LOW The two proposal lists treat a refused latest revision differently

Location: `crates/recipe-app/src/recipe/queries/parameter_form.rs:265`, `crates/recipe-app/src/plant/queries/types.rs:424`

Evidence: the parameter form filters each row before it takes the maximum revision:

```rust
for row in types.rows() {
    if let RecordShape::Struct { name, revision, .. } = row.shape()
        && !is_refused_struct(types.platform(), row)
    {
        let entry = latest.entry(name).or_insert(*revision);
        *entry = (*entry).max(*revision);
```

The UDT form takes the latest revision first and then drops it:

```rust
.filter_map(|(_, group)| group.first())
.filter(|latest| {
    types
        .get(latest.id)
        .is_none_or(|row| !is_refused_struct(types.platform(), row))
})
```

Impact: before this change, a refusal depended only on the name, so every revision of a name had the same result. Now `is_refused_struct` also reads the members (ADR-0203 (d), ADR-0204 (a)), so two revisions of one name can get different results. Example: a stored legacy `ST_X@2` with one member on `CodesysIec` and a valid `ST_X@1`. The parameter form proposes `ST_X@1`, and the UDT form proposes no `ST_X`. Both lists still leave out the refused row (ADR-0203 (j), ADR-0204 (d)), so this is not a safety defect. But `design.md` says "Both proposal lists call it", and the two lists now differ.

Fix: use one rule in both lists. The scenarios "Proposals leave out a stored reserved struct" and "Proposals leave out a stored one-member struct" of the `plant-model` delta spec use only revision 1, so they do not decide the case. Either fall back to the latest permitted revision in `types.rs`, or skip the name in `parameter_form.rs` when its latest revision is refused. Add one test with a refused latest revision for each list.

##### CR-O02 LOW `StringEncoding` derives `Default` with `Latin1`, and only its own test uses it

Location: `crates/recipe-domain/src/plant/string_encoding.rs:12`

Evidence:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StringEncoding {
    /// One Latin-1 byte for each character.
    #[default]
    Latin1,
```

`rg "StringEncoding::default"` finds only `crates/recipe-domain/src/plant/string_encoding/tests.rs:32`. No production code and no struct that derives `Default` uses it.

Impact: `design.md` ("Carrier of the encoding") rejects a default of `Latin1` because "a missed call gives the `Latin1` rule on a `Utf8` machine with no compile error". The public `Default` brings back that risk for later code: `..Default::default()` or `unwrap_or_default()` silently selects `Latin1`.

Fix: remove `Default` and `#[default]`, and remove the test that pins it. `DEFAULT_ENCODING` in `pages/machines/form.rs` already names the form proposal explicitly.

##### CR-O03 LOW No test covers the notice of a Retired version on a `Utf8` machine (N2)

Location: `crates/recipe-web/src/pages/versions/actions.rs:227`, `crates/recipe-web/src/pages/versions/adr_0202_notice_tests.rs`

Evidence: only the `Approved` branch chains the encoding notice:

```rust
VersionState::Approved => (
    Operation::ExportApprovedVersion,
    ...
    rule_notice(t, view)
        .into_iter()
        .chain(encoding_notice(t, view))
        .collect(),
```

The doc comment of `encoding_notice` says "A Retired version shows only its own notice". The tests are `latin1_healthy`, `latin1_faulty`, `utf8_healthy`, `utf8_faulty`, and `polish`. None of them uses a Retired version.

Impact: the behavior matches the spec. The requirement "Notice of a v1 document of a Utf8 machine" names only an Approved version. But a later change can add the encoding notice to the Retired branch, or remove the retired notice there, and no test fails.

Fix: add `adr_0202_version_notice_utf8_retired_shows_only_the_retired_notice`. It asserts one notice with the id `export-notice`, no `export-encoding-notice`, and `aria-describedby="export-notice"`.

##### CR-O04 LOW The test of the task 7.3 bullet "A stored row with the type text `TOD` stays a corrupt row on the page" does not use the page (N6)

Location: `crates/recipe-web/src/pages/parameters/adr_0206_tests.rs:265`

Evidence:

```rust
#[test]
fn adr_0206_a_stored_read_of_the_alias_text_stays_a_grammar_error() {
    // The stored reads keep `from_stored(.., RuleSet::Earlier)`, so a stored row with the type text
    // `TOD` is a corrupt row. Only the request parser maps the alias.
    assert!(TypeExpr::from_stored("TOD", RuleSet::Earlier).is_err());
    assert_eq!(TypeExpr::parse("TOD").unwrap().to_string(), "TIME_OF_DAY");
}
```

Impact: the test checks the domain parser only, which `recipe-domain` already tests. It does not prove the claim of the task bullet, a stored row on a page of `recipe-web`. A read path that later calls `TypeExpr::parse` (current set, with the alias) for stored text would pass this test and would silently accept a corrupt row (ADR-0206 (f), `design.md` "The stored reads keep `from_stored(..., RuleSet::Earlier)`").

Fix: seed a parameter or a struct member with the stored type text `TOD` in the memory store. Then assert that the version page or the parameter form gives the corrupt-row answer that the existing corrupt-row tests assert, and not a grid of `TIME_OF_DAY`.

##### CR-O05 LOW After `409 E-PLT-054`, the answer swaps the encoding form back in, and the page keeps the form (N3)

Location: `crates/recipe-web/src/pages/machines/string_encoding_commands.rs:145`

Evidence:

```rust
Err(match refusal_lines(error, target_of) {
    Ok(lines) => refused_encoding(state, request, code, (sent, lines)),
    Err(other) => other,
})
```

`refusal_lines` gives a line for every `AppError::Plant` (`draft_reply.rs:86`). Thus `StringEncodingFrozen` gets `StringEncodingForm::new(...).out_of_band(errors)` with `shown: true`.

Impact: ADR-0202 (p) says "The page offers the change only while item (d) permits it". After the `409`, the encoding is frozen, but the page shows the form again with the sent value until a reload. A second submit gives the same `409` and writes nothing, so data stays safe. The behavior follows ADR-0041 (e), so it is a UX gap, not a rule break.

Fix (optional): for `StringEncodingFrozen`, swap an out-of-band form with `shown(false)`, or let the error go to `#error-panel` without the form. Then add a test that asserts that the `409` answer holds no `/string-encoding"` form.

#### Notes N1 to N9

- N1 OK. `check_export` (`deploy/ci/upgrade_smoke.py:102`) reads the v1 document. The v1 body never holds `string_encoding` (ADR-0202 (k)). The upgrade fixture is a `Latin1` machine, so `loadable is True` is the correct check. The golden bytes of task 10.3 pin the absence of the field.
- N2 LOW defect, a test gap only. The code shows only the retired notice for a Retired version, as intended. See CR-O03.
- N3 LOW defect, UX only. No write happens, and the answer follows ADR-0041 (e). See CR-O05.
- N4 OK. `CreateFields.string_encoding` is required, as `api.md` §2.2 lists it, and the select always sends it with `Latin1` selected. Every scripted client sends it: `e2e/tests/support/{seed,pages}.ts`, `e2e/tests/connection.spec.ts`, and `deploy/ci/load_seed.py`. A missing field is a changed form body (`E-WEB-008`, ADR-0138).
- N5 OK. `check_codesys_members(platform, name, members)` needs `name` for `type_expr` of `PlantError::DerivedTypeInvalid`, as `check_iec_names` does. Only the signature in `design.md` is out of date. The behavior is the same.
- N6 LOW defect, the test does not prove its bullet. See CR-O04. The code is correct: `vendor_alias` runs only under `RuleSet::Current` (`type_expr.rs:215`).
- N7 OK. `seed_time_primitive_types` is memory test support (`#[cfg(any(test, feature = "memory"))]`). Its platform sets match the platform columns of the table in `data-model.md` §7 (lines 2305 to 2333) and the `type_ref_<platform>` enums of the v2 schema: `DATE_AND_TIME` is not on `S7_1200`, and `LTOD`/`LDT` are on `CodesysIec`, `TwinCat3`, and `S7_1500`. The offset 100 cannot collide with the 11 rows of `SEEDS`.
- N8 OK. Case 2 (`rollback_query.rs:81`) asserts `(1, 1)`. `encoding_entries` 1 is the correct count after one `SetMachineStringEncoding`, and the test checks more than the task text asks.
- N9 OK. `pub(crate)` in `crates/recipe-db/tests/storage/stored_rules.rs` is visible only inside the test crate. No production visibility changes.

## Codex report

#### Summary

**Verdict: not ready to archive.** 3 HIGH, 5 MEDIUM, 1 LOW, 0 CRITICAL.  
The parameter form can propose a superseded type, and the UDT form loses the field responsible for a reserved-name error.  
The encoding command checks permission first, locks the machine before checking for versions, and commits the update with its audit entry.  
I reviewed `e633b45..b4c6fa2` read-only. The 9,890-test gate is the result reported in the brief. I did not rerun Cargo or Docker.

#### Findings

##### CR-C01 HIGH Reserved-name errors lose the offending UDT field

**Location:** `crates/recipe-domain/src/plant/iec_names.rs:92`, `crates/recipe-web/src/pages/types/define.rs:219`  
**Evidence:** `check_iec_names` detects a reserved member with `.any(|member| is_iec_reserved_member_name(member.as_str()))`, then returns `type_expr: name.to_string()`. That is the enclosing struct name. The web handler looks for a **member type** where `parsed.to_string() == *type_expr`.  
**Impact:** Submitting `ST_Coil` with an `END_IF: BOOL` member produces `E-PLT-013`, but neither `BOOL` input matches `ST_Coil`. The summary cannot link to `END_IF`, and the member-name input gets no inline error. A new struct named `VERSION` loses its name-field target the same way. The HTTP test checks only `reply.body.contains(&reason)`, so it passes. This violates ADR-0041(d), (e), and (g).  
**Fix:** Preserve which input failed. Map a prohibited new struct name to `type_name`, a prohibited submitted member name to `members-{row}-name`, and a prohibited referenced revision to the submitted member-type field. Assert both the summary link and the inline error in the HTTP tests.

##### CR-C02 HIGH The parameter form can propose a superseded struct revision

**Location:** `crates/recipe-app/src/recipe/queries/parameter_form.rs:268`, `crates/recipe-app/src/plant/queries/types.rs:427`  
**Evidence:** The parameter form applies `&& !is_refused_struct(types.platform(), row)` **before** `*entry = (*entry).max(*revision)`. The struct form takes `group.first()` **before** applying the refusal check. The parameter form documents its contract as “the proposals of the `datalist` of the type input, as `GetUdtForm` gives them.”  
**Impact:** On an upgraded CODESYS machine, `ST_Pair@1` can have two valid members while the later `ST_Pair@2` has one. The parameter form offers `ST_Pair@1` as a suggestion. The struct form omits `ST_Pair`. An engineer can therefore select an obsolete layout from a list intended to show only the latest revision.  
**Fix:** Select the highest revision for each name first, then apply the platform rules to that revision. Add a mixed-revision test that checks both forms.

##### CR-C03 HIGH Task 7.3 has no stored-row test for `TOD`

**Location:** `crates/recipe-web/src/pages/parameters/adr_0206_tests.rs:265`, `crates/recipe-db/src/transaction/type_rows.rs:153`, `openspec/changes/codesys-twincat-type-rules/tasks.md:272`  
**Evidence:** The test asserts `TypeExpr::from_stored("TOD", RuleSet::Earlier).is_err()`. The database read actually uses `Primitive::parse(name).map_err(bad)?`. The checked-off task says “A stored row with the type text `TOD` stays a corrupt row on the page.”  
**Impact:** A change that normalizes `TOD` while decoding a stored primitive would pass this test. The request-side HTTP tests establish that the alias works on input, but the stored-row assertion never reaches the decoder or the page error path.  
**Fix:** Feed a primitive record named `TOD` through the actual storage decoder and assert `E-DB-008`. Exercise the page error mapping with a corrupt read result. Keep the existing parser assertion as a separate unit test.

##### CR-C04 MEDIUM A frozen-encoding response reinstates the editable form

**Location:** `crates/recipe-web/src/pages/machines/string_encoding_commands.rs:145`, `crates/recipe-web/src/pages/machines/string_encoding_form.rs:48`  
**Evidence:** The domain refusal goes through `Ok(lines) => refused_encoding(...)`. That function constructs `StringEncodingForm::new(...).out_of_band(errors)`, and the constructor sets `shown: true`.  
**Impact:** If another user creates the first recipe while the machine page is open, the next submit correctly gets `409 E-PLT-054`, but the response puts the usable submit button back. Every retry fails until reload. ADR-0202(p) permits the page to offer the change only while the machine has no version.  
**Fix:** For `StringEncodingFrozen`, keep the conflict in `#error-panel` and remove or replace the form out of band. Extend the conflict test to check what happens to the existing form.

##### CR-C05 MEDIUM The rollback test never isolates a machine created with `Utf8`

**Location:** `crates/recipe-db/tests/storage/rollback_query.rs:90`, `crates/recipe-db/tests/storage/rollback_query.rs:132`  
**Evidence:** The test calls `set_encoding(&app, seed.code, StringEncoding::Utf8).await` and expects `(1, 1)`. The other cases expect `(0, 0)` and `(0, 2)`. `CreateMachine` can create a `Utf8` machine directly, which produces a `MachineCreated` entry and zero `MachineStringEncodingSet` entries.  
**Impact:** The runbook SQL currently counts the two conditions independently. A regression that counted only UTF-8 machines with an encoding-change entry would pass all three tests and permit an unsafe image rollback for a machine created with `Utf8`.  
**Fix:** Create the machine with `Utf8` in case 2, keep the stored `é` assertion, and expect `(1, 0)`. The change-back case already exercises the opposite condition.

##### CR-C06 MEDIUM The upgrade matrix does not reread the historical scope hash

**Location:** `crates/recipe-db/tests/storage/stored_rules_iec.rs:360`, `crates/recipe-db/tests/storage/stored_rules.rs:495`  
**Evidence:** The post-upgrade assertion is `assert_eq!(unit.document["scope_sha256"], scope_hash.as_str())`. It checks a **fresh export** against the fixture. The pre-migration snapshot uses `stored_rows`, whose documented contents are “the rows of `parameter` and `plc_type` as text.” The query that reads `a.after_state->>'scope_sha256'` runs when generating the fixture.  
**Impact:** The test proves the hash can be recomputed. It would still pass if the upgrade changed or removed the hash in the original `VersionExported` audit entry. ADR-0202(j2) explicitly protects that historical value.  
**Fix:** Capture the original audit entry ID and its scope hash before migration. Read that same entry after migration and after backup restore, and compare its stored value with the fixture.

##### CR-C07 MEDIUM Changed Rust files and functions exceed the repository size limits

**Location:** `crates/recipe-export/tests/golden.rs:801`, `crates/recipe-app/src/recipe/queries/parameter_form.rs:339`, `crates/recipe-export/tests/golden_types/table.rs:180`  
**Evidence:** The repository rule says “A function has at most 50 lines, and a file has at most 800 lines.” `golden.rs` now has 1,091 lines. `GetParameterForm::execute` spans lines 339–395, and `pub fn codesys_type_table() -> [TypeCase; 25]` spans lines 180–297. Four other changed Rust files also cross 800 lines: `storage/platform_ranges.rs` (815), `plant/error/tests.rs` (833), `recipe/limit/parse/tests.rs` (803), and `recipe/value/leaf/tests.rs` (824).  
**Impact:** The change crosses both explicit review limits, including in the production query that now collects form data, checks the proposed type, and builds the preview.  
**Fix:** Move the new test groups into focused modules. Split the production query at the preview-building step and organize the long fixture table into smaller groups.

##### CR-C08 MEDIUM Reserved-name checks scan every member of every revision when building parameter proposals

**Location:** `crates/recipe-app/src/recipe/queries/parameter_form.rs:266`, `crates/recipe-domain/src/plant/iec_names.rs:49`  
**Evidence:** The proposal loop calls `is_refused_struct` for every struct row before grouping revisions. For each member, the predicate searches `RESERVED_KEYWORDS.iter().chain(IEC_EXTRA_KEYWORDS.iter()).any(...)`.  
**Impact:** Opening a parameter form pays for up to 184 case-insensitive comparisons per member of every historical IEC revision, even though the form displays at most one revision per name. The work grows as revisions accumulate.  
**Fix:** Apply the ordering change in CR-C02 so only candidate revisions undergo member validation. This removes the historical scans without changing the domain rule.

##### CR-C09 LOW The design records an obsolete helper signature

**Location:** `openspec/changes/codesys-twincat-type-rules/design.md:79`, `crates/recipe-domain/src/plant/iec_names.rs:110`  
**Evidence:** The design shows `check_codesys_members(platform, members)`. The implementation takes `platform`, `name: &TypeName`, and `members`, and uses `name` for `type_expr: name.to_string()`.  
**Impact:** The archived design would give maintainers the wrong interface. The extra argument has a concrete diagnostic purpose.  
**Fix:** Update the design signature to include `name`.

#### Notes N1 to N9

- **N1 — OK.** The smoke check requests `"/export/v1/recipes/CI-UPGRADE/versions/1"`. Its `("code", "plc_platform")` projection is correct for the v1 hashed body. `deploy/ci/upgrade_smoke.py:132`
- **N2 — OK.** Task 11.19 specifies Approved versions. Retired versions already show `"version-export-retired-notice"`, which tells the reader the file is not loadable, and the retired-link test covers that notice. `crates/recipe-web/src/pages/versions/actions.rs:227` · `crates/recipe-web/src/pages/versions/export_link_tests.rs:100`
- **N3 — MEDIUM, CR-C04.** The server enforces the freeze, but its conflict response recreates the editable form.
- **N4 — OK.** The create contract lists `string_encoding`, and `string_encoding: String` makes it required. The HTML form supplies `Latin1` by default. `docs/architecture/api.md:85` · `crates/recipe-web/src/pages/machines/form.rs:46`
- **N5 — LOW, CR-C09.** The implementation needs the struct name for the error payload. The design signature is stale.
- **N6 — HIGH, CR-C03.** The request-side `TOD` cases have HTTP coverage, including the struct member. The stored-row case calls only the expression parser. `crates/recipe-web/src/pages/types/adr_0206_tests.rs:17`
- **N7 — OK.** `seed_time_primitive_types` adds four canonical primitive rows to the memory fixture. `TIME_SEED_OFFSET: usize = 100` keeps their IDs separate from the existing seed rows. `crates/recipe-app/src/memory/types/seed.rs:94`
- **N8 — MEDIUM, CR-C05.** `(1, 1)` is correct for the command used. The coverage gap is the reachable `(1, 0)` rollback state.
- **N9 — OK.** `pub(crate)` lets sibling modules of the storage integration-test crate reuse `stored_rows`, `copy_row`, and the restore helpers. The visibility change is confined to the test crate. `crates/recipe-db/tests/storage/stored_rules.rs:475`
