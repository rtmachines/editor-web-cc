# ADR-0121: The parameter table reads the value of every row

Status: Accepted
Date: 2026-11-04
Deciders: project owner

## Context

ADR-0108 (b) adds the port method `parameter_values(version, keys)`. `ListParameters` calls it only with the keys of the primitive rows of one page. ADR-0108 (a) also states that the table read loads no value of an array or a struct. With these two rules, one page reads less than 5 MiB.

ADR-0118 (b) adds a read check for each part that a read shows. The check uses the current rule set and the earlier rule set of ADR-0120. The parameter table shows the field lines of a value and of an S7 address (ADR-0118 (f)).

An array or a struct can break the address rules of ADR-0116, as a primitive can. The read check takes the stored parameter with its value. Without the value of an array row or a struct row, the table cannot run the check on that row.

## Decision drivers

- The user sees each part that breaks a rule before the approval and the export (R12, R38).
- A storage defect stays visible as a defect (`reliability.md` section 1.1).
- Each rule lives in one function (ADR-0120). A page does not copy a rule of the read check.
- One page of the table has an upper bound of bytes (ADR-0108 (b)).

## Considered options

### Option A: Read the value of every row of the page

- Pros: one read check for every row. An array row or a struct row gets its address lines and its storage defect check.
- Cons: the page reads the value of each composite row. The upper bound of one page gets larger.

### Option B: Check only the address of a composite row, without a value read

- Pros: the page reads no value of a composite row. The bound of ADR-0108 (b) stays.
- Cons: the address check needs a second entry point next to the read check. The table does not find a storage defect in the value of a composite row.

### Option C: Show address lines only for primitive rows

- Pros: no change of the read.
- Cons: an array or a struct at an address past the end of its data block shows no line on the table. The user finds the fault only on the parameter page or at approval.

## Decision

The project uses **Option A**. This is the decision of the project owner of 2026-11-04. It amends ADR-0108 (b) and the last sentence of the composite bullet of ADR-0108 (a).

- (a) `ListParameters` passes the keys of every row of the page to `parameter_values`. This includes each array row and each struct row.
- (b) The read check of ADR-0118 (b) and the address lines of ADR-0118 (f) need the stored value of each row. This is the reason for item (a).
- (c) `ListParameters` runs the read check of ADR-0118 (b) on each row of the page. The table shows no limits, so the check of the table runs without the limit rows.
- (d) The rule of the value cell in ADR-0108 (a) does not change. A primitive row shows its text. An array row and a struct row show the word of `parameters-value-composite`.
- (e) An array row or a struct row that fails the earlier rule set is a storage defect. `ListParameters` gives `500 E-DB-008`, as the parameter page does (ADR-0118 (d)).
- (f) The upper bound of one call of `parameter_values` is `page.parameters` rows times `budget.value_bytes` (`reliability.md` section 7.11). This bound replaces the 5 MiB of ADR-0108 (b).
- (g) `parameter_page` and its statement stay free of values, as ADR-0108 (b) states. `GetVersion` reads no value, except for the check of an `Approved` version in ADR-0119 (l) and (m).

## Consequences

### Positive

- An array row or a struct row with an affected S7 address shows its address line on the table.
- The table, the parameter page, and the export use one read check (ADR-0119 (a)).
- A storage defect in the value of a composite row stops the table read and writes one log line.

### Negative

- One page of the table reads more bytes. It reads the value of each composite row and shows only one word for it.
- The upper bound of one page is larger than the 5 MiB of ADR-0108 (b).

### Risks and mitigations

- Risk: a page with many large composite values can be slow. Mitigation: `page.parameters` limits the rows of one page, and `budget.value_bytes` limits each value.

## Related

- ADR-0108 (parameter table, port method `parameter_values`), ADR-0118 (read check, field lines), ADR-0120 (earlier rule set)
- ADR-0116 (address rules), ADR-0119 (check of the version page of an `Approved` version)
- `api.md` section 2.6, `reliability.md` section 7.11
- `overview.md` section 7: R12, R38
