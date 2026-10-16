# ADR-0044: Bound of the rebind forms

Status: Accepted
Date: 2026-10-15
Deciders: project owner

## Context

ADR-0043 (l) puts one rebind form per outdated struct revision on the page of a draft. The form holds one field per new member leaf of each affected parameter (ADR-0039 (f)). The documents set no bound on the sum of these fields:

- `GetVersion` computes the new member leaves of every outdated parameter for every reader. An Operator gets this work too, although the Operator gets no form.
- A struct revision that many parameters use can need up to `budget.leaves_per_version` new values (A26). That is 262 144 fields on one page.
- 262 144 fields give tens to hundreds of MB of HTML. The page then uses much memory and time on the server and in the browser.
- `budget.leaves_per_parameter` limits each parameter, but no setting limits the sum over the parameters of one page.

## Decision drivers

- The work and the size of a page have a bound for each reader (R30, A26).
- A reader that cannot rebind gets no rebind work.
- The bound of a page equals the size of the leaf grid that ADR-0039 accepts.
- Each numeric setting comes from `reliability.md` section 7.11.

## Considered options

### Option A: A bound on the version page

- Pros: the query and the page have a bound. The rebind command does not change.
- Cons: a rebind over the bound is not possible on the page.

### Option B: One page for each rebind, with groups of fields

- Pros: every rebind is possible in the browser.
- Cons: a new route, a new page, and a new save protocol for a partial input. The body limit of the rebind route still applies.

### Option C: No bound

- Pros: no change.
- Cons: one outdated draft can stop the server and the browser of each reader.

## Decision

The project uses **Option A**. This ADR amends ADR-0043 (l) with the points below.

- (a) `GetVersion` computes the new member leaves of a rebind only for a caller with `ParameterEdit` on a live `Draft`.
  - Every other reader gets the list of outdated parameters without the new member leaves. The page shows that reader no rebind form.
  - The query reads the permissions of the caller. A caller without `ParameterEdit` gets no error and no security event.
- (b) The new setting `form.rebind_leaves` is 16 384 (`reliability.md` section 7.11). This value equals `budget.leaves_per_parameter`.
  - The sum of the new member leaves of all rebind forms of one version page is at most `form.rebind_leaves`.
  - The query adds the count of the new member leaves of each parameter to a running sum. It stops the computation of new member leaves when the sum passes the bound.
- (c) When the sum passes the bound, each rebind form of the page shows no field of a new member and no submit button.
  - Each form shows a notice with the text key `rebind-too-large`. The notice gives the sum that the query reached and the bound.
  - The sum in the notice is a lower limit, because the query stops at the first parameter that passes the bound.
- (d) The command `RebindTypeRevision` does not change. Its body limit `http.body_limit.value` and the budgets of A26 stay its limits.

## Consequences

### Positive

- The rebind work of the version page has a bound for each reader.
- An Operator and every reader of a version that is not a live `Draft` get no rebind work.
- The version page holds at most as many rebind fields as one leaf grid.

### Negative

- A rebind that needs more than `form.rebind_leaves` new values is not possible on the version page.
- The version page shows the notice in place of the fields. The user cannot remove the outdated state of the draft on this page.

### Risks and mitigations

- Risk: a plant needs a struct revision that adds many members to a struct that many parameters use. Mitigation: the plant defines the change as a sequence of smaller revisions, and each rebind stays under the bound.
- Risk: a smaller revision is not possible. Mitigation: a later step can add a rebind page with groups of fields (Option B). Until then, the draft can be approved with `allow_outdated` (A29).

## Related

- ADR-0018, ADR-0039, ADR-0043
- `api.md` section 2.5
- `reliability.md` section 7.11
- `overview.md` section 9: A26, A29
