# Design: plc-type-ranges

## Context

See `proposal.md` for the motivation. The review `docs/plan/review/plc-s7-type-coverage-review.md` §3 (F01 to F07) and §4.1 (Q01 to Q05) give the evidence. The review is not normative.

The decisions are in nine new ADRs:

| ADR | Decision | Review items |
|---|---|---|
| ADR-0114 | `DATE` on `S7_1200` and `S7_1500` ends at 2168-12-31, count 72 683. Amends ADR-0026 | F01, Q01 |
| ADR-0115 | `DATE_AND_TIME` whole seconds, `DTL` to 2262-04-11-23:47:16.854775807, `REAL` and `LREAL` in the `Decimal` range stay as deliberate restrictions | F02, F03, F06, Q02, Q03, Q05 |
| ADR-0116 | Offset plus layout size at most 65 536 bytes. Data block at most 16 000 on `S7_300`. Item (e) gives the parameters that a command checks | F04, F05, Q04 |
| ADR-0117 | At most 7 significant digits for `REAL`, 15 for `LREAL`. Amends ADR-0028 | F07 |
| ADR-0118 | A read shows a stored part that breaks a newer rule with a field line. Amends ADR-0050, ADR-0108 | Owner answer O1 |
| ADR-0119 | `loadable` is `false` when a value, a limit, or an address breaks a current rule. Notice on the version page. Amends ADR-0019, ADR-0052, ADR-0108, ADR-0109 | Owner answer O2 |
| ADR-0120 | The earlier rule set for a stored part that a command does not write and for `CreateVersion`. Amends ADR-0028 | Owner answer O1, follow-up 2 |
| ADR-0121 | The parameter table reads the value of every row of one page for the read check. Amends ADR-0108 | Owner answer of 2026-11-04 |
| ADR-0122 | Limits of the table check and of the version page check of an `Approved` version. Amends ADR-0042, ADR-0056 | Owner answer of 2026-11-04 |

The current code, read for this design:

- `crates/recipe-domain/src/plant/iec_literal/range.rs` holds `DATE_S7 = 7_305..=72_840` and `DATE_S7_300 = 7_305..=72_683`. `range` selects a constant by type and platform.
- `crates/recipe-domain/src/recipe/value/leaf.rs` `numeric` checks the 28-digit limit and the range. A fault there is an `Expected`, which gives `E-RCP-005`.
- `crates/recipe-domain/src/recipe/value/decimal.rs` `canonical` counts the significant digits without the zeros at the end.
- `crates/recipe-domain/src/recipe/limit/parse.rs`: the request parser and `TryFrom<(&StoredLimit, &TypeTree)>` both end in `finish`. `finish` holds the type check of a bound.
- `crates/recipe-domain/src/recipe/draft/value.rs`: `set_value` parses the stored limits, and `replace_limits` parses the stored value. Both end in `replace_parameters`.
- `crates/recipe-domain/src/recipe/snapshot/check.rs` holds `check_snapshot`. `VersionSnapshot::check` (from `CreateVersion`) and `Draft::check_for_approval` (`draft/approval.rs`) call it.
- `crates/recipe-domain/src/recipe/draft/s7.rs`: `check_definition` runs the platform and tree rules for one parameter. Only `AddParameter` and `UpdateParameterDefinition` call it. `check_s7_layout` covers the whole draft. `occupy` counts one bit for an address with a bit number.
- `crates/recipe-domain/src/recipe/draft/replace.rs`: `replace_parameters` runs `check_s7_layout` on the whole draft. Four callers use it: add, value, definition, and rebind.
- `crates/recipe-domain/src/recipe/diff/pairs.rs` `check_stored` and `recipe/value/texts.rs` `literal_text` build the text of a stored part with the current ranges.
- `crates/recipe-app/src/recipe/queries/approval_preview.rs` returns an error of the base diff when the draft has no blocker.
- `crates/recipe-app/src/export/document.rs`: the blocking task builds the hashed body, compares the hash, and then calls `facts.complete`.
- `crates/recipe-app/src/export/output.rs`: `ExportedVersion::loadable` depends only on the status. `crates/recipe-domain/src/export/facts.rs` does the same.
- `crates/recipe-export/schema/export-v1.schema.json` and `export-v2.schema.json` require `loadable` `true` for an `Approved` document.

## Goals / Non-Goals

**Goals:**

- Apply ADR-0114, ADR-0116, and ADR-0117 in `recipe-domain` and show the three new codes in `recipe-web`.
- Pin the restrictions of ADR-0115 with tests.
- Apply ADR-0120 in the draft commands and in `CreateVersion`.
- Apply ADR-0118 in the reads, and ADR-0119 in the export and on the version page.
- Update the normative documents in the same change as the ADRs.

**Non-Goals:**

- No migration and no database constraint.
- No change of the export fields, the hashed body, `content_sha256`, or `schema_version`. The schema files change one rule (ADR-0119 (g)).
- No change of a stored row. Approved versions are immutable (ADR-0009).
- No tests of F08 to F11. The change `plc-type-test-coverage` owns them.

## Decisions

### D1. New error codes

`reliability.md` §1.2 gives `E-RCP-050..069` to domain variants after `E-RCP-029`. The last used code is `E-RCP-054`. The next free `E-PLT` code is `E-PLT-022`. This change needs none.

| Code | Variant | HTTP | Proposed message |
|---|---|---|---|
| `E-RCP-055` | `RecipeError::PrecisionExceeded { tag, leaf, field, type_name, max_digits }` | 422 | A number of {tag}{leaf} has too many significant digits for {type_name}. Use at most {max_digits} digits. |
| `E-RCP-056` | `RecipeError::S7AddressPastBlockEnd { tag, address, size }` | 422 | Parameter {tag} at {address} needs {size} bytes. It ends after byte 65535 of the data block. |
| `E-RCP-057` | `RecipeError::S7DataBlockOutOfRange { tag, address }` | 422 | Address {address} of {tag} uses a data block above 16000. An S7-300 controller has no such data block. |

The row of `reliability.md` §1.2 comes first. Then `error_codes.rs`, then the variant.

### D2. Rule set and DATE range

- `recipe-domain` gets one enum `RuleSet` with the values `Current` and `Earlier` (ADR-0120 (a), (b), (i)).
- `range.rs` gets `DATE_S7 = 7_305..=72_683` for all three S7 platforms. `DATE_S7_300` goes away.
- `range.rs` gets the named constant `DATE_S7_EARLIER = 7_305..=72_840` with a doc comment that cites ADR-0120 (a).
- `range` takes the rule set. `Earlier` selects `DATE_S7_EARLIER` only for `DATE` on `S7_1200` and `S7_1500`. `S7_300` keeps 72 683 in both sets.
- Every other range is the same in both sets.

Alternative: a second range table for the earlier set. Rejected, because two tables copy every other range.

### D3. Significant digits

- `decimal.rs` gets one function that returns the count of significant digits. `canonical` and the new check use it, so one count exists.
- `leaf.rs` `numeric` checks the count after the range for `REAL` (7) and `LREAL` (15). The check runs only in the current set.
- The fault needs its own kind, so that `ValueFaults` maps it to `E-RCP-055` and not to `E-RCP-005`. The shape of the kind is a choice of the implementer.
- `limit/parse.rs` runs the same check in `finish` for `min` and `max`. Thus the request parser and the stored parser both get it. A fault maps to `E-RCP-055` (ADR-0117 (c)).
- The request parser always uses the current set. The stored parser takes the rule set from its caller.

Alternative: check only in the request parser. Rejected, because then a stored draft with such a number could be approved.

### D4. Address rules

- The data block rule for `S7_300` needs no tree. The end rule uses `S7Layout::of(tree)?.size_bytes()` and `checked_add`. An overflow gives `E-RCP-056`.
- An address with a bit number uses one bit, as `occupy` counts it. `DB1.DBX65535.7` therefore passes (ADR-0116 (a)).
- Two named constants hold 65 536 and 16 000, with a doc comment that cites ADR-0116.
- `check_definition` runs both rules. `AddParameter` and `UpdateParameterDefinition` call it already.
- `rebind/apply.rs` runs both rules for each rebound parameter before it calls `replace_parameters` (ADR-0116 (e)).
- `check_for_approval` runs both rules for every parameter.
- `replace_parameters` and `check_s7_layout` do not run the two rules. The overlap check I20 still covers the whole draft (ADR-0116 (e1)).

Alternative: a new reason of `S7Address::parse`. Rejected in ADR-0116 (option B).

### D5. Web layer

- `crates/recipe-web/src/error/status.rs` maps the three codes to 422.
- The field lines of ADR-0041 put `E-RCP-056` and `E-RCP-057` under the field `s7_address`. They put `E-RCP-055` under the value field or the limit field of its leaf, as for `E-RCP-005` and `E-RCP-006`.
- `PrecisionExceeded` has the field `field` of type `PrecisionField` with the values `Value`, `LimitMin`, and `LimitMax`. The web layer uses it to put the line under the value input or under the `min` or `max` input of the limit.
- The approval blockers of ADR-0049 list the three codes.
- `i18n/pl.ftl` and `i18n/en.ftl` get one text for each code.

### D6. Commands with the rule set (ADR-0120)

- `check_snapshot` takes the rule set and passes it to the stored parsers of a value and a limit.
- `VersionSnapshot::check` uses `Earlier` for `CreateVersion`. `check_for_approval` uses `Current`.
- `set_value` parses the stored limits with `Earlier`. `replace_limits` parses the stored value with `Earlier`. The request part uses `Current`.
- I7 between the written part and the stored part still runs (ADR-0120 (e)).
- The comment at the start of `snapshot/check.rs` names the rule set of each caller.
- The snapshot check does not run the S7 rules. `CreateVersion` therefore copies an affected address (ADR-0120 (h)).

### D7. Reads of a stored part (ADR-0118)

- `recipe-domain` gets one read check of a stored parameter. It checks with `Earlier` first and with `Current` second.
- A fault with `Earlier` is a structure fault. The read keeps `500 E-DB-008` (ADR-0118 (d)).
- A fault with `Current` only is a rule fault. It holds the leaf, the code, and the kind: value, limit, or address.
- The read check also runs the two address rules of D4 when the parameter has an address. The codes are `E-RCP-056` and `E-RCP-057`.
- `texts.rs` `literal_text` and `pairs.rs` `check_stored` build the text with `Earlier`. A `DATE` count 72 700 shows as `D#2169-01-17` (ADR-0118 (g)).
- `value_texts_of` in `recipe-app/src/recipe/queries/parameters.rs` returns the stored text and the rule faults.
- `diff_page.rs` gives the rule faults to the diff rows. The approval preview diff does the same on both sides.
- The approval preview no longer returns an error of its base diff for a rule fault. A structure fault of the base still gives `E-DB-008` (ADR-0118 (h)).
- The parameter page shows the value lines, the limit lines, and the address lines. The parameter table shows the value lines and the address lines (ADR-0118 (a), (f)). It shows no limits, so it has no limit lines.
- `ListParameters` reads the stored value of every row of the page, also of an array or a struct row (ADR-0121).
- `ListParameters` runs the read check in `Blocking::validate` after its transaction ends, inside `timeout.validation` and without a permit (ADR-0122 (a), (b)).
- The field line uses the text of its code in the `pl` and `en` bundles. No new text is necessary.

### D8. Loadable flag (ADR-0119)

- `recipe-domain/src/export` gets one function that runs the read check of D7 on every parameter of the version. It uses the platform of the machine.
- A structure fault there stays a storage defect, and the export gives `500 E-DB-008`.
- `DocumentFacts` gets the result. `loadable` is `true` only for `Approved` and no rule fault.
- `document.rs` runs the function in the blocking task after the test `stored != Some(hash)` and before `facts.complete` (ADR-0119 (j), (k)).
- `ExportedVersion` takes `loadable` from the facts and not from the status.
- The v1 routes and the v2 route check the whole version (ADR-0119 (f)).
- Both schema files keep `if Retired then loadable false` and drop the `else` branch (ADR-0119 (g)).

### D9. Notice on the version page (ADR-0119 (l), (m))

- The version page query of an `Approved` version runs the function of D8 on every parameter.
- It reads the values in a read-only transaction. It writes nothing and no audit entry.
- It takes a permit of `export.permits` outside any transaction and reads the values in a second transaction. `timeout.export` limits the wait, the read, and the check (ADR-0122).
- The page shows the notice in the export area of ADR-0052 (e) with the code of each broken rule.
- The export link names the notice in `aria-describedby`. A `Draft` and a `Retired` version get no rule notice.

## Stored data that breaks a new rule

The owner answers O1 to O4 of 2026-11-03 decide the effects. This list gives the result.

1. **Live draft.** A command checks the part that it writes with the current set. It checks a stored part that it only reads with the earlier set (ADR-0120). The address rules run only for the written parameters (ADR-0116 (e)).
2. **Approval of a draft.** The snapshot check returns `E-RCP-020` with items `E-RCP-005`, `E-RCP-006`, or `E-RCP-055`. The S7 check reports `E-RCP-056` or `E-RCP-057`.
3. **New draft from an affected version.** `CreateVersion` copies the base when each fault comes from ADR-0114 or ADR-0117 (ADR-0120 (g)). Every other fault still gives `E-RCP-020`.
4. **Reads.** The parameter table, the parameter page, the diff, and the approval preview show the stored text with a field line (ADR-0118).
5. **Approved and retired versions.** They are immutable (ADR-0009). The reads show the same field lines.
6. **Export.** An affected version gets `loadable` `false` with the same hashed body and `content_sha256` (ADR-0119). The version page shows a notice.

## Owner answers

- **O1. Pages with an affected stored value.** A read shows the stored part with a field line. Decision: ADR-0118 and ADR-0120.
- **O2. Export of an affected approved version.** The export sets `loadable` to `false`. The hashed body and `content_sha256` do not change. Decision: ADR-0119.
- **Follow-up 1. Addresses in the export check.** The export check also runs the two address rules. Decision: ADR-0119 (b).
- **Follow-up 2. CreateVersion from an affected base.** `CreateVersion` copies the base. Decision: ADR-0120 (g), which amends ADR-0028.
- **O3. Check before the release.** No scan before the release. ADR-0118 and ADR-0119 make each affected part visible on the pages, at approval, and at export.
- **O4. Limits and significant digits.** The digit rule covers `min` and `max` of a `REAL` or `LREAL` limit with `E-RCP-055`. ADR-0117 (c) already states this rule.

## Risks / Trade-offs

- [A read takes a storage defect for a rule fault] → D7 uses the earlier set as the border. Tasks test a `DATE` count above 72 840.
- [A caller uses the earlier set for written data] → Tasks 7.3 and 7.4 send a part that breaks only the current set.
- [An affected approved version reaches the PLC] → ADR-0119 sets `loadable` to `false`. The transfer client must check it (`api.md` §6.3).
- [The version page of an `Approved` version reads every value] → The page reads as much as one export. ADR-0119 records this cost.
- [A decimal of 7 digits does not always give back the same digits after IEEE 754 single precision] → ADR-0117 records the risk. The owner chose 7.
- [Some S7-300 CPUs have fewer than 16 000 data blocks] → ADR-0116 records the risk. A limit per CPU model needs a new ADR.

## Migration Plan

1. The release ships the new image. It has no migration and no scan (owner answer O3).
2. After the release, the pages show each affected part, and the export marks each affected version as not loadable.
3. Rollback: the previous image. No stored row changes, so the rollback needs no data step.

## Known gaps

- Q06 to Q10 of the review stay open. `proposal.md` (Out of scope) lists them.
- A rule that a later ADR adds to the current set needs its own entry in the earlier set or a third set. ADR-0120 covers only ADR-0114 and ADR-0117.
