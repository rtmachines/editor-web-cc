# ADR-0166: Constructor for stored data

Status: Accepted. Amends ADR-0120 (i) (parsers of a stored part) and ADR-0143 (i) and (i1) (stored names and stored types)
Date: 2026-11-14
Deciders: project owner

## Context

The full repository review of 2026-11-13 found F-13. `docs/plan/review/2026-11-13-full-repo-review.md` §4 holds it.

These facts shape the decision:

- `reliability.md` §4.3 says that each value object has one constructor `parse`. `CLAUDE.md` names `TypeExpr` under this rule.
- ADR-0120 (i) gives the stored parsers one parameter for the rule set. ADR-0120 does not amend `reliability.md` §4.3.
- The code has a second public constructor `from_stored` on five types. No document names `from_stored`.
- A reviewer can thus refuse valid code. A new value object can copy the second constructor without a review.
- The stored form and the input form of one value can differ. A time leaf is a count in storage and an IEC literal in a request.
- `VersionState::from_stored` in `crates/recipe-domain/src/recipe/state.rs` line 105 has the same name. It maps the stored status and the deletion mark, and it takes no rule set.

## Decision drivers

- A rule change does not lock a draft (ADR-0028, ADR-0120).
- A value object checks its input in its constructor (R38).
- Each rule lives in one function (ADR-0120).
- A rule of the documents matches the shipped code. Thus a review does not refuse valid code.

## Considered options

### Option A: Name `from_stored` as a permitted second constructor

- Pros: no code change. The documents match the code.
- Cons: the rule of `reliability.md` §4.3 gets one exception.

### Option B: Fold the rule set into `parse`

- Pros: one constructor for each value object.
- Cons: every caller of `parse` passes a rule set. The stored form of a time leaf still needs a second path.

### Option C: Remove the rule set and parse stored data with `parse`

- Pros: one constructor and one rule set.
- Cons: a rule change locks a draft again. This breaks ADR-0028 and ADR-0120.

## Decision

The project uses **Option A**.

- (a) A value object can have a second public constructor `from_stored`. It checks stored data under the rule set of that data (ADR-0120 (i), ADR-0143).
- (b) The last parameter of `from_stored` is a `RuleSet`. The other parameters give the stored part and its context, such as the `TypeTree` of the parameter.
- (c) A caller that reads a stored part and does not write it passes `RuleSet::Earlier`. A caller that writes a part passes `RuleSet::Current` (ADR-0120 (c), (d)).
- (d) `parse` and `from_stored` use one shared check function. Thus a rule lives in one function, and the rule set selects the branch.
- (e) Code does not check a value again after `from_stored`. The rule of `reliability.md` §4.3 holds for both constructors.

### Holders of `from_stored`

- (f) `TypeName::from_stored(text, rules)` in `crates/recipe-domain/src/plant/type_name.rs` line 304. The stored form and the input form are one text. `parse` calls `from_stored` with `RuleSet::Current`.
- (g) `TypeExpr::from_stored(text, rules)` in `crates/recipe-domain/src/plant/type_expr.rs` line 67. The stored form and the input form are one text. `parse` calls `from_stored` with `RuleSet::Current`.
- (h) `PlcType::from_stored(record, rules)` in `crates/recipe-domain/src/plant/plc_type.rs` line 140. Both forms are one `PlcTypeRecord`. `TryFrom<PlcTypeRecord>` calls `from_stored` with `RuleSet::Current`.
- (i) `ProcessLimit::from_stored(row, tree, rules)` in `crates/recipe-domain/src/recipe/limit/parse.rs` line 85. It reads a `StoredLimit`, whose time bound is a count. `parse` reads a `LimitInput`, whose bounds are texts.
- (j) `TryFrom<(&StoredLimit, &TypeTree)>` for `ProcessLimit` calls `from_stored` with `RuleSet::Earlier`.
- (k) `ParameterValue::from_stored(stored, tree, rules)` in `crates/recipe-domain/src/recipe/value/stored.rs` line 53. It reads a time leaf as a JSON number with the count.
- (l) `ParameterValue::parse(json, tree)` reads a time leaf as an IEC literal in the text. `TryFrom<(&StoredValue, &TypeTree)>` calls `from_stored` with `RuleSet::Earlier`.

### Checks that `RuleSet::Earlier` skips

- (m) The end of `DATE` on `S7_1200` and `S7_1500` (ADR-0114). The current set ends at the count 72 683. The earlier set ends at 72 840 (ADR-0120 (a)).
- (n) The end of `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` (ADR-0139). The current set ends at `2^63-1` ns. The earlier set ends at `2^64-1` ns (ADR-0139 (e)).
- (o) The significant digits of `REAL` and `LREAL` (ADR-0117). `ParameterValue::from_stored` and `ProcessLimit::from_stored` skip this check under `RuleSet::Earlier`.
- (p) The reserved names of a struct (ADR-0143 (i)). The current set refuses a keyword and a primitive name in any case. The earlier set refuses only a primitive name in exact case.
- (q) Item (m) and item (n) live in `crates/recipe-domain/src/plant/iec_literal/range.rs` at lines 84, 86, and 93.
- (r) Item (o) lives in `crates/recipe-domain/src/recipe/value/leaf.rs` line 209 and in `crates/recipe-domain/src/recipe/limit/parse.rs` line 171.
- (s) Item (p) lives in `crates/recipe-domain/src/plant/type_name.rs` line 308 and in `crates/recipe-domain/src/plant/plc_type.rs` line 252.
- (t) The data block rule of ADR-0140 (d) and the array rule of ADR-0148 (e) have no branch of `RuleSet`. A command runs them only on the data that it writes.

### Other constructors with a name other than `parse`

- (u) `OneTimePassword::from_bytes` builds the password from 20 random bytes that the application supplies. Each byte selects one character of the alphabet. It reads no external input.
- (v) `IecLiteral::from_count(ty, count, rules)` builds a literal from a stored count under a rule set. A caller that reads a count and does not write it passes `RuleSet::Earlier` (ADR-0120 (d)).
- (w) `IecLiteral::parse` reads the text of a literal. `TryFrom<(TimeType, i128)>` for `IecLiteral` calls `from_count` with `RuleSet::Current`.
- (x) `ParameterValue::from_texts(tree, texts)` builds a value from the texts of its leaves (ADR-0039 (c)). It uses the rules of `ParameterValue::parse` and `RuleSet::Current`.
- (y) `ParameterValue::from_texts` collects every fault of the texts in one `ValueFaults`.

### Scope

- (z) No code changes. `reliability.md` §4.3, `CLAUDE.md`, and `.claude/rules/rust.md` name `from_stored`. They name the stored form and the input form apart.
- (aa) This ADR does not cover `VersionState::from_stored`.
- (ab) This ADR amends ADR-0120 (i). The parsers of a stored value and a stored limit are the constructors `from_stored` of items (i) and (k).
- (ac) This ADR amends ADR-0143 (i) and (i1). The stored reads of a name and of a type use the constructors `from_stored` of items (f), (g), and (h).

## Consequences

### Positive

- A review accepts the second constructor of the five holders.
- A reader finds each check that `RuleSet::Earlier` skips in one list.
- The documents name the stored form and the input form apart.

### Negative

- The rule of one constructor for each value object gets one exception.
- A later rule change that adds a check to `RuleSet::Current` needs a new item in this list.

### Risks and mitigations

- Risk: a new value object copies `from_stored` without a stored part. Mitigation: item (a) permits `from_stored` only for stored data under a rule set.
- Risk: a caller passes `RuleSet::Earlier` for data that it writes. Mitigation: the tests of ADR-0120 (c) send a new value and a new limit that break only the current set.

## Related

- ADR-0028 (draft load), ADR-0039 (one field for each leaf), ADR-0114, ADR-0117, ADR-0139 (value rules)
- ADR-0120 (earlier rule set), ADR-0140, ADR-0143, ADR-0148 (amendments of the earlier set)
- `reliability.md` §4.3, `domain-model.md` §2
- `overview.md` §7: R38
