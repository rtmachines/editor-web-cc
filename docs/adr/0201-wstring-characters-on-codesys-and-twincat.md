# ADR-0201: WSTRING characters on CODESYS and TwinCAT

Status: Accepted. Amends ADR-0137 (a) and (b) (the `WSTRING` characters on `CodesysIec` and `TwinCat3`). Amends ADR-0180 (c) (the rule of these two platforms). Amends ADR-0120 (a) and (g) (earlier rule set, copy of a base). Amends ADR-0166 (m) to (s) (the checks that `RuleSet::Earlier` skips). Amended by ADR-0210 (item (a): LF and CR at command input only)
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ02 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- ADR-0137 (b) accepts every `WSTRING` character except `U+0000`. ADR-0180 (c) keeps this rule on `CodesysIec` and `TwinCat3`.
- The reference file says that both vendors describe `WSTRING` as UCS-2 (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:153` and `:169`). UCS-2 holds `U+0000` to `U+D7FF` and `U+E000` to `U+FFFF`.
- A character outside the Basic Multilingual Plane needs two UTF-16 code units in `0xD800..=0xDFFF`. A UCS-2 `WSTRING` cannot hold it as one character.
- The golden file `types_TwinCat3_wstring_non_bmp.json` of `recipe-export` exports such a character with `loadable` `true`.
- `WCHAR` does not exist on `CodesysIec` and `TwinCat3` (`data-model.md` §7). Thus only `WSTRING` is in scope.

The project owner decided on 2026-11-19: adopt the UCS-2 repertoire for new `WSTRING` values on these platforms. The earlier rule set reads stored values, as ADR-0180 does on S7.

## Decision drivers

- An exported value fits the PLC type (R03, R11, R12).
- The domain checks all external input (R38).
- A rule change does not lock a draft (ADR-0028, ADR-0120).
- A stored value never changes without the user (`reliability.md` §4.4).

## Considered options

### Option A: Keep ADR-0180 (c)

- Pros: no change.
- Cons: the PLC gets two surrogate code units for one character. The PLC string functions count them as two characters.

### Option B: The UCS-2 repertoire for new values, earlier rule set for stored values

- Pros: an exported value holds only characters of the vendor `WSTRING`. A draft with a stored value stays editable.
- Cons: an approved version with such a value becomes not loadable.

### Option C: As Option B, but only for new writes

- Pros: an approved version stays loadable.
- Cons: the export can still carry a character that the PLC cannot hold.

## Decision

The project uses **Option B**.

- (a) On `CodesysIec` and `TwinCat3`, each UTF-16 code unit of a `WSTRING` value is in `0x0001..=0xD7FF` or `0xE000..=0xFFFF`. Thus a `WSTRING` character is in `U+0001..=U+D7FF` or `U+E000..=U+FFFF`. The rule refuses a character outside the Basic Multilingual Plane. This item amends ADR-0137 (b) and ADR-0180 (c).
- (b) `WCHAR` gets no new rule, because it does not exist on these platforms. ADR-0137 (a) keeps its `WCHAR` rule.
- (c) A refused value gives `E-RCP-005` with the expected form of its type and platform (ADR-0137 (c)). The form names the length and the two ranges. No code is new.
- (d) Every path that builds a value applies the rule (ADR-0137 (d)). These are a JSON value, the text of a form field, and a new leaf of `RebindTypeRevision`.
- (e) Earlier rule set. The earlier set keeps ADR-0137 (b) on every platform. `ParameterValue::from_stored` skips item (a) under `RuleSet::Earlier`. This item amends ADR-0120 (a) and adds an item to the list of ADR-0166 (m) to (s).
- (f) A command checks the value that it writes with the current set (ADR-0120 (c)). `ReplaceLimits` reads the stored value with the earlier set (ADR-0120 (d)).
- (g) `ApproveVersion` checks every value with the current set. A stored value that breaks item (a) gives an item of `E-RCP-020` (ADR-0120 (f)).
- (h) `CreateVersion` copies a base whose value faults come from this ADR. This item amends ADR-0120 (g).
- (i) The export check uses the current set (ADR-0119 (a)). A stored value that breaks item (a) makes `loadable` `false`. The expected result of the golden file `types_TwinCat3_wstring_non_bmp.json` changes to a rule fault with `loadable` `false`.
- (j) A process limit holds no text (ADR-0137 (e)). The unit of `max_length` stays the UTF-16 code unit (ADR-0137 (i)). Under item (a), each new character counts 1.
- (k) The ops query `deploy/db/check-platform-ranges.sql` lists each stored `WSTRING` value on `CodesysIec` and `TwinCat3` that breaks item (a), in a new part `iec_wide_text`.

## Consequences

### Positive

- An exported `WSTRING` value on `CodesysIec` and `TwinCat3` holds only UCS-2 characters.
- A draft with an affected stored value stays editable.

### Negative

- A plant cannot store a character outside the Basic Multilingual Plane, such as an emoji, in a `WSTRING` on these platforms.
- An approved version with such a value exports with `loadable` `false`. Its hashed bodies, `content_sha256`, and `scope_sha256` do not change (ADR-0119 (d), (i)).
- The `WSTRING` form of `E-RCP-005` gets a new text in the bundles `pl` and `en`.

### Risks and mitigations

- Risk: the stored data holds an affected value. Mitigation: the ops query of item (k) lists it before an upgrade.
- Risk: a plant needs a character outside the Basic Multilingual Plane. Mitigation: a `STRING` on a `Utf8` machine can hold it (ADR-0202).

## Related

- ADR-0028 (draft load), ADR-0119 (export check), ADR-0120 (earlier rule set), ADR-0137 (text rules), ADR-0166 (constructors for stored data), ADR-0180 (the S7 rule)
- `domain-model.md` §2.1 (SQL values, `StringLength`), `reliability.md` §1.2 (`E-RCP-005`), §4.4
- `overview.md` §7: R03, R11, R12, R31, R38
