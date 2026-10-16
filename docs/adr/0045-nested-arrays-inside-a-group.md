# ADR-0045: Nested arrays inside a group

Status: Accepted
Date: 2026-10-15
Deciders: project owner

## Context

ADR-0039 (l) splits an array with more than `form.array_chunk` elements into groups (`reliability.md` section 7.11). A group is one `tbody` of the leaf grid. Its toggle row stands in its own `tbody` before the group.

ADR-0039 (l) does not say what happens to an array inside an element of a group:

- An element of an array can be a struct, and a member of that struct can be an array. Example: the type `ARRAY[1..120] OF ST_Zone`, where `ST_Zone` holds the member `profile` of type `ARRAY[1..120] OF INT`.
- The outer array has 120 elements, so it splits into two groups. Each member `profile` has 120 elements too, so ADR-0039 (l) would split it into groups as well.
- A group of `profile` needs its own `tbody`. HTML does not let a `tbody` hold a `tbody`. The parser closes the outer `tbody` at the inner one, so the outer toggle no longer hides the rows of its group.
- Step P3.3 implemented a rule that the documents do not hold: an array inside a group forms no groups of its own. The code review asked the user to decide this point.

## Decision drivers

- The leaf grid stays one table with the columns of ADR-0039 (j) and the row headers of ADR-0039 (m).
- `tree.js` and its limit of 50 lines do not change (`api.md` section 9). It toggles one group by its `id` (`design-system.md` section 10.8).
- The HTML of the grid is valid, so each toggle hides exactly the rows of its group.
- The size of a page has a bound (A26).

## Considered options

### Option A: An array inside a group shows all its rows in that group

- Pros: the grid stays one valid table. `tree.js` does not change.
- Cons: an open group can hold many rows.

### Option B: A nested table for each nested array

- Pros: each array level can have its own groups.
- Cons: the columns of the nested table do not line up with the columns of the grid. The row headers of ADR-0039 (m) then need a second scheme.

### Option C: The outer group splits into several bodies at each nested array

- Pros: each array level can have its own groups in one table.
- Cons: one toggle then controls several bodies. `tree.js` finds one group with `document.getElementById`, so the script must change. The toggle rows of the nested groups also stand between the rows of the outer group.

## Decision

The project uses **Option A**. This ADR amends ADR-0039 (l) with the points below.

- (a) Only an array that does not stand inside a group splits into groups of `form.array_chunk` elements.
  - Examples: an array of the top level, or an array member of a struct that does not stand inside a group.
  - The reason: a group is one `tbody`, and a `tbody` cannot hold another `tbody`.
- (b) An array inside a group shows all its rows inside that group. It has no toggle of its own.
  - Example: the member `profile` of type `ARRAY[1..120] OF INT` of each element of `ARRAY[1..120] OF ST_Zone`. Each element of the first group shows its 120 rows of `profile` in that group.
  - The outer toggle opens and closes the rows of the nested array together with the other rows of its group.
  - The limit modes of ADR-0039 (k) do not change. The row `[*]` and the mode button of the nested level stand inside the group.
- (c) An open group can hold many rows. The first group of the example of point (b) holds 12 000 leaf rows and the rows `[*]`.
  - The bound `budget.leaves_per_parameter` (16 384, A26) limits the rows of one parameter.
  - Every row is in the HTML already, so this rule does not change the size of the page. Groups only shorten the visible page.

## Consequences

### Positive

- The leaf grid stays one valid table, and each toggle hides exactly the rows of its group.
- `tree.js` and `design-system.md` section 10.8 keep their pattern.
- The size of a page does not change.

### Negative

- An open group with nested arrays can be long on the screen. The user scrolls more in that group.
- The groups of an array depend on its place. The same array type splits at the top level, but it does not split inside a group.

### Risks and mitigations

- Risk: a panel PC renders a long open group slowly. Mitigation: the bound of A26 limits the rows, and the load test of `reliability.md` section 6 measures the page in step P4.2.
- Risk: a user does not find a leaf in a long group. Mitigation: each row header holds the leaf path (ADR-0039 (m)), and a group with an error line is open (ADR-0041 (e)).

## Related

- ADR-0039, amended by this ADR in (l): the groups of a nested array.
- ADR-0036, ADR-0041
- `reliability.md` section 7.11
- `design-system.md` section 10.8
- `overview.md` section 9: A26
