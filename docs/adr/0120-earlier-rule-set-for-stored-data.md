# ADR-0120: Earlier rule set for stored data

Status: Accepted. Amended by ADR-0139 (item (a): the earlier set keeps `0..=2^64-1` ns for `LDATE` and `LDT` on `CodesysIec` and `TwinCat3`. Item (g): `CreateVersion` also copies a base with such a stored count). Amended by ADR-0140 (item (a): the earlier set has no data block rule)
Date: 2026-11-03
Deciders: project owner

## Context

ADR-0114 and ADR-0117 narrow the value rules. ADR-0116 adds two address rules. A row that a user stored before these rules can break them. The examples are a `DATE` count from 72 684 to 72 840 and a `REAL` number with 8 significant digits.

ADR-0028 lets a draft load such a row, and the snapshot check reports a fault at approval. Three command paths still parse a stored part with the new rules:

- `SetParameterValue` parses the stored limits of its parameter. `ReplaceLimits` parses the stored value of its parameter. A parameter with an affected value and an affected `max` then refuses both commands.
- `CreateVersion` runs the snapshot check on its base (ADR-0028). An affected approved version then cannot be the base of a new draft that fixes it.
- `CreateVersion` and the approval call one snapshot check. A change for one command changes the other.

The address rules of ADR-0116 run only for the parameters that a command writes (ADR-0116 (e)). This ADR covers the value and limit rules.

## Decision drivers

- A rule change does not lock a draft (ADR-0028).
- No approval and no export of a version that breaks a current rule (R03, R12, R38).
- The system never changes a stored value without the user (`reliability.md` section 4.4).
- Each rule lives in one function. Two rule sets do not copy a rule.

## Considered options

### Option A: One parameter that selects the rule set

- Pros: one check for each rule. A command and a read select the set that they need.
- Cons: the snapshot check, the stored parsers, the range table, and the digit check get one more parameter.

### Option B: Skip the parse of the stored part that a command does not write

- Pros: no second rule set.
- Cons: the process-limit check I7 between the value and the limits needs both parts as value objects. `CreateVersion` still needs its own exception.

## Decision

The project uses **Option A**.

- (a) The earlier rule set is the set of rules before ADR-0114 and ADR-0117. On `S7_1200` and `S7_1500`, a `DATE` count ends at 72 840. A `REAL` or `LREAL` number has up to 28 significant digits.
- (b) The current rule set holds every rule. Every rule that ADR-0114 and ADR-0117 do not change is the same in both sets.
- (c) A command checks the data that it writes with the current set. This is the value or the limit of the request, as `domain-model.md` section 5.2 gives it for each command.
- (d) A command checks a stored part that it reads and does not write with the earlier set. `SetParameterValue` reads the stored limits, and `ReplaceLimits` reads the stored value.
- (e) The process-limit check I7 between the written part and the stored part still runs. A new value outside a stored limit still gives its code.
- (f) `ApproveVersion` checks every parameter with the current set. An affected value or limit gives an item of `E-RCP-020`.
- (g) `CreateVersion` checks its base with the earlier set. It copies a base whose value and limit faults all come from ADR-0114 or ADR-0117. Every other fault still gives `E-RCP-020`, and the system creates no draft.
- (h) The snapshot check does not run the S7 rules (ADR-0028). `CreateVersion` therefore copies a base with an address that breaks ADR-0116. `ApproveVersion` refuses it (ADR-0116 (e)).
- (i) The rule set is one parameter of the shared checks. These are the snapshot check, the parsers of a stored value and a stored limit, the range table, and the digit check.
- (j) No write path could store a part that fails the earlier set. Such a part is a storage defect (ADR-0118 (d)).

## Consequences

### Positive

- The user can fix the value and the limits of one parameter in either order.
- The user can fix an affected approved version in a new draft that starts from it.
- One function holds each rule.

### Negative

- The shared checks get one more parameter, and each caller selects the set.
- The old end 72 840 of `DATE` stays in the code as a named constant.

### Risks and mitigations

- Risk: a caller selects the earlier set for data that it writes. Mitigation: the tests of item (c) send a new value and a new limit that break only the current set.
- Risk: `CreateVersion` copies a fault other than item (g). Mitigation: a test covers a `DATE` count above 72 840.

## Related

- ADR-0028 (draft load, snapshot check), ADR-0114, ADR-0116, ADR-0117 (the new rules), ADR-0118 (reads), ADR-0119 (export)
- `domain-model.md` sections 2.1, 3 (I7), 5.2
- `overview.md` section 7: R03, R12, R38
