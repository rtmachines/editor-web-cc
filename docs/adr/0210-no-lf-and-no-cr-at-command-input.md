# ADR-0210: No LF and no CR in a text value at command input

Status: Accepted. Amends ADR-0137 (a), (b), (d), (g), and (k) (LF and CR at command input, new expected texts). Amends ADR-0180 (a) and (b) (LF and CR at command input). Amends ADR-0201 (a) (LF and CR at command input). Amends ADR-0202 (f) (LF and CR at command input)
Date: 2026-11-22
Deciders: project owner

## Context

The full repository review of 2026-11-21 found a silent change of a stored text value (`docs/plan/review/2026-11-21-full-repo-review.md` §4.1).

These facts shape the decision:

- ADR-0137 (a) and (b) refuse only `U+0000` in a `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value. The ranges of ADR-0180 (a) and (b), ADR-0201 (a), and ADR-0202 (f) also hold LF (`U+000A`) and CR (`U+000D`).
- The edit form of a parameter shows each text leaf in an `input type="text"` (`crates/recipe-web/templates/components/grid.html`). ADR-0112 records that such an input removes every LF and CR from its value.
- One save of the form sends the value with the definition (ADR-0039 (i)). `recipe-web` keeps the text of a `String` or `Char` leaf as sent (ADR-0039 (e)).
- Thus the next save of the form removes a stored LF or CR, also when the user changes only the description. No message tells the user.
- The browser form cannot send LF or CR in a value. Such a value comes from a request that a client builds by hand.
- The diff of ADR-0050 (c) shows the change before approval. `separation_of_duties` is a setting, so a second person does not always read that diff.
- The value rules have two modes. `Mode::Request` checks the value that a command receives. `Mode::Stored` reads a stored value (`crates/recipe-domain/src/recipe/value/leaf.rs`).
- The approval check passes `RuleSet::Current` to the snapshot check. Thus each rule of the current rule set also applies to a stored value at approval and in the export check (ADR-0119 (a), ADR-0120).
- The vendors permit `$N`, `$R`, and `$L` in a string. A stored LF or CR is therefore a valid PLC value.

The project owner decided on 2026-11-21 (`docs/plan/review/2026-11-21-fix-plan.md` §1). The domain refuses LF and CR in a text value at the input of a command. The read of a stored row does not change.

## Decision drivers

- A save of a form does not change a stored value without a message to the user (R31).
- The domain checks all external input (R38).
- A rule change does not lock a draft (ADR-0028, ADR-0120).
- The rule does not change a stored row, the `loadable` flag of an approved version, or a content hash.

## Considered options

### Option A: The domain refuses LF and CR at command input only

- Pros: a new value cannot hold a character that the form removes. Every stored row stays readable, approvable, and loadable. No new error code.
- Cons: a user cannot enter a line end in a text value. The next save of the form still removes a stored line end.

### Option B: The current rule set refuses LF and CR

This option follows the pattern of ADR-0180 (f) to (j). The rule goes into `RuleSet::Current`, and the earlier rule set skips it.

- Pros: one rule for each path. The approval check and the export check also find a stored line end.
- Cons: `ApproveVersion` gives an item of `E-RCP-020` for a stored line end (ADR-0180 (h)). The export check makes `loadable` `false` for an approved version that the PLC can load (ADR-0180 (j)). The read of a stored row changes, against the decision of the owner.

### Option C: A `textarea` keeps the line ends of a text leaf

This option follows the pattern of ADR-0112.

- Pros: a user can enter and keep a line end.
- Cons: a browser sends each line end of a `textarea` as CRLF (ADR-0112). The server cannot know the stored form of a line end, so a stored LF or CR can still change. A `CHAR` leaf holds one character. The leaf grid gets a multi-line cell for each text leaf.

### Option D: Keep the rules

- Pros: no change.
- Cons: the next save of the form changes a stored value without a message.

## Decision

The project uses **Option A**. The rule is a deliberate narrowing of the vendor range, in the pattern of ADR-0115. It applies only to the input of a command.

- (a) A `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value that a command receives holds no LF (`U+000A`) and no CR (`U+000D`). The rule applies on every platform and with each string encoding.
- (b) Item (a) narrows ADR-0137 (a) and (b), ADR-0180 (a) and (b), ADR-0201 (a), and ADR-0202 (f) at command input. The ranges of these items stay the ranges of a stored value.
- (c) Every path that receives a value applies the rule. These are a JSON value, the text of a form field, and a new member of `RebindTypeRevision`. This item amends ADR-0137 (d).
- (d) A refused value gives `E-RCP-005` with the expected form of its type and platform (ADR-0137 (c)). No code is new.
- (e) The rule is not part of `RuleSet::Current` or `RuleSet::Earlier` of ADR-0120. This item amends ADR-0137 (g) for LF and CR. The `U+0000` rule stays in both rule sets.
- (f) No read of a stored value applies the rule. Examples are the approval check, the snapshot check of `CreateVersion` and `ApproveVersion`, and the export check of `loadable` (ADR-0119 (a)).
- (g) Thus a stored value with LF or CR stays readable, approvable, and loadable. Its content hash does not change.
- (h) The seam is `Mode::Request` of the value rules. `ParameterValue::parse` and `request_leaf` use this mode. `request_leaf` serves the form texts and the new members of a rebind.
- (i) The approval check and every stored read use `Mode::Stored`. The form path converts its texts with `Mode::Stored` only after `request_text` checked each text.
- (j) The six expected forms of `E-RCP-005` for `CHAR`, `WCHAR`, `STRING`, and `WSTRING` name LF and CR. The `expected-*` keys of the bundles `pl` and `en` do the same. This item amends ADR-0137 (k).
- (k) The ops query `deploy/db/check-platform-ranges.sql` gets the part `line_break`. It lists each stored text leaf with LF or CR, on every platform and in every version status.
- (l) The header of the query lists the rows that the current rule set refuses. A `line_break` row passes every rule set, but the next save of the form changes it. The header gets a second sentence for this part.
- (m) `docs/ops/runbook.md` explains the part `line_break`. An approved row stays loadable. Before the next save of the form of a draft row, the owner changes the value or accepts that the save removes the line end.
- (n) `domain-model.md` §2.1 and `reliability.md` §4.4 state the rule of item (a) next to the character ranges.

## Consequences

### Positive

- A new value cannot hold a character that the next save of the form removes.
- No stored row, `loadable` flag, or content hash changes.
- The user gets `422 E-RCP-005` with a field line, and no new error code.

### Negative

- A user cannot enter LF or CR in a text value.
- The next save of the form still removes a stored LF or CR.
- The UI phrases of six expected forms change in the bundles `pl` and `en`.

### Risks and mitigations

- Risk: the stored data holds a value with LF or CR. Mitigation: the part `line_break` of item (k) lists it before an upgrade, and the runbook of item (m) names the next step of the owner.
- Risk: a later change adds the rule to `RuleSet::Current`. Then an approved version with a stored line end stops being loadable. Mitigation: guard tests approve and export a stored value with LF through `Mode::Stored`.
- Risk: a test generator draws LF or CR for a valid value. Mitigation: the property generators map LF and CR to another character, as they map `U+0000`.

## Related

- ADR-0028 (draft load), ADR-0039 (parameter value form), ADR-0050 (diff), ADR-0112 (multi-line text field)
- ADR-0115 (ranges narrower than the vendor range), ADR-0119 (export check), ADR-0120 (earlier rule set)
- ADR-0137 (text rules), ADR-0180 (S7 wide characters), ADR-0201 (IEC wide characters), ADR-0202 (string encoding)
- `domain-model.md` §2.1, `reliability.md` §1.2 (`E-RCP-005`, `E-RCP-020`), §4.4
- `docs/plan/review/2026-11-21-full-repo-review.md` §4.1, `docs/plan/review/2026-11-21-fix-plan.md` §1
- `overview.md` §7: R11, R12, R31, R38
