# ADR-0180: WCHAR and WSTRING characters on S7

Status: Accepted. Amends ADR-0137 (a) and (b) (the `WCHAR` and `WSTRING` characters on S7). Amends ADR-0120 (a) and (g) (earlier rule set, copy of a base). Amends ADR-0166 (m) to (s) (the checks that `RuleSet::Earlier` skips). Amended by ADR-0201 (item (c): `CodesysIec` and `TwinCat3` get the UCS-2 rule of `WSTRING`). Amended by ADR-0210 (items (a), (b): LF and CR at command input only)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ04 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4).

These facts shape the decision:

- ADR-0137 (a) accepts a `WCHAR` value in `U+0001..=U+FFFF`. ADR-0137 (b) accepts every `WSTRING` character except `U+0000`. These rules hold on every platform.
- The Siemens reference file says that the TIA V21 `WCHAR` page gives `16#0000…16#D7FF`. The `WSTRING` page excludes the code units `16#D000…16#FFFF` (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:427`).
- The same file says that the two-byte width does not establish surrogate pairs (`:429`).
- `WCHAR` and `WSTRING` exist on `S7_1200` and `S7_1500` (`data-model.md` §7). `S7_300` has neither type.
- ADR-0137 (c) gives `E-RCP-005` with the expected form of the type for a refused character.

The project owner decided on 2026-11-17: one rule for each type, on the S7 platforms only.

## Decision drivers

- An exported value fits the PLC type (R03, R11, R12).
- The domain checks all external input (R38).
- A rule change does not lock a draft (ADR-0028, ADR-0120).
- A stored value never changes without the user (`reliability.md` §4.4).

## Considered options

### Option A: Keep ADR-0137

- Pros: no change.
- Cons: the export can carry a character that the PLC refuses or changes.

### Option B: `U+0001..=U+D7FF` for both types on S7

- Pros: one range.
- Cons: a `WSTRING` then accepts `U+D000..=U+D7FF`, which the `WSTRING` page excludes.

### Option C: One rule for each type on S7

- Pros: each type follows its own Siemens page.
- Cons: a `WCHAR` accepts `U+D000..=U+D7FF`, and a `WSTRING` does not.

## Decision

The project uses **Option C**.

- (a) On `S7_1200` and `S7_1500`, a `WCHAR` value is one Unicode scalar in `U+0001..=U+D7FF`.
- (b) On `S7_1200` and `S7_1500`, each UTF-16 code unit of a `WSTRING` value is in `0x0001..=0xCFFF`. Thus a `WSTRING` character is in `U+0001..=U+CFFF`. A character outside the Basic Multilingual Plane needs two code units in `0xD800..=0xDFFF`, so the rule refuses it.
- (c) `CodesysIec` and `TwinCat3` keep ADR-0137 (a) and (b).
- (d) A refused value gives `E-RCP-005` with the expected form of its type and platform (ADR-0137 (c)). The `WCHAR` form names `U+D7FF`. The `WSTRING` form names the length and the range `U+0001` to `U+CFFF`.
- (e) Every path that builds a value applies the rule (ADR-0137 (d)). These are a JSON value, the text of a form field, and a new leaf of `RebindTypeRevision`.
- (f) Earlier rule set. The earlier set keeps ADR-0137 (a) and (b) on every platform. `ParameterValue::from_stored` skips items (a) and (b) under `RuleSet::Earlier`. This item amends ADR-0120 (a) and adds an item to the list of ADR-0166 (m) to (s).
- (g) A command checks the value that it writes with the current set (ADR-0120 (c)). `ReplaceLimits` reads the stored value with the earlier set (ADR-0120 (d)).
- (h) `ApproveVersion` checks every value with the current set. A stored value that breaks item (a) or (b) gives an item of `E-RCP-020` (ADR-0120 (f)).
- (i) `CreateVersion` copies a base whose value faults come from this ADR. This item amends ADR-0120 (g).
- (j) The export check uses the current set (ADR-0119 (a)). A stored value that breaks item (a) or (b) makes `loadable` `false`.
- (k) A process limit holds no text (ADR-0137 (e)). The unit of `max_length` stays the UTF-16 code unit (ADR-0137 (i)).
- (l) The ops query `deploy/db/check-platform-ranges.sql` lists each stored `WCHAR` and `WSTRING` value on `S7_1200` and `S7_1500` that breaks item (a) or (b), in a new part `wide_text`.

## Consequences

### Positive

- An exported `WCHAR` or `WSTRING` value on S7 holds only characters of the Siemens pages.
- A draft with an affected stored value stays editable.

### Negative

- A plant cannot store `U+D000..=U+FFFF` or a character outside the Basic Multilingual Plane in an S7 `WSTRING`.
- The `WSTRING` form of `E-RCP-005` gets a new text in the bundles `pl` and `en`.

### Risks and mitigations

- Risk: a newer TIA Portal accepts more characters. Mitigation: a new ADR can widen the range.
- Risk: the stored data holds an affected value. Mitigation: the ops query of item (l) lists it before an upgrade.

## Related

- ADR-0028 (draft load), ADR-0119 (export check), ADR-0120 (earlier rule set), ADR-0137 (text rules), ADR-0166 (constructors for stored data)
- `domain-model.md` §2.1 (SQL values, `StringLength`), `reliability.md` §1.2 (`E-RCP-005`), §4.4
- `overview.md` §7: R03, R11, R12, R31, R38
