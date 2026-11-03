# ADR-0110: Several danger actions on one page

Status: Accepted
Date: 2026-11-02
Deciders: project owner

## Context

ADR-0049 (f) gives the notice of a danger action the fixed id `danger-notice`. The danger button names that id in `aria-describedby`. ADR-0049 (f) also states that a page holds at most one danger action, so the id is unique.

The pages break that rule today:

- The parameter page holds the delete disclosure of the parameter (ADR-0043 (k)).
- The same page holds one delete disclosure for each stored translation (ADR-0043 (f)).

These notices have no id, and their buttons have no `aria-describedby`. A screen reader therefore does not read the warning with the button.

The UI review of 2026-11-02 (finding A1) moves "Reset password" on the user page into a danger disclosure of `design-system.md` section 10.5. A reset replaces the password, and the system cannot undo it. The user page also holds "Deactivate" next to it.

A fixed id on two notices of one page breaks HTML, and `aria-describedby` then names the wrong element.

## Decision drivers

- Each danger button names its own warning, so a screen reader reads the effect before the click (R39, A44).
- Each `id` of a page is unique.
- No new script, no inline style, and each `hx-*` value stays a literal (`api.md` section 9, ADR-0037).
- The pages and macros that follow ADR-0049 (f) today keep their markup.

## Considered options

### Option A: One page, one danger action

- Pros: the fixed id `danger-notice` stays unique with no other rule.
- Cons: the parameter page must move the delete of a translation to another page. The user page cannot hold the reset disclosure.

### Option B: A unique notice id for each danger disclosure

- Pros: the pages keep their actions. Each button names its own warning.
- Cons: a template with several danger disclosures must build each id. A test must find a repeated id.

### Option C: An id from a counter of the page

- Pros: no name rule.
- Cons: the id changes when a row comes or goes. An out-of-band swap of one row can then repeat an id.

## Decision

This is the decision of the user of 2026-11-02. The project uses **Option B**. It amends ADR-0049 (f) on one point: the sentence "A page holds at most one danger action, so the id is unique."

- (a) A page can hold more than one danger action.
- (b) The notice of each danger disclosure has an id that is unique in the page and no role. The danger button names that id in `aria-describedby`.
- (c) The id `danger-notice` stays for the macros `danger_versioned` and `danger_action` of `components/danger.html`.
  - The version page holds at most one of them. A retire needs an Approved version and a discard needs a live `Draft`, and a version has one state.
  - The recipe page holds one of them, the delete action of ADR-0049 (g).
  - A page holds the id `danger-notice` at most once.
- (d) Every other danger disclosure gives its notice the id `danger-notice-<suffix>`.
  - The suffix starts with a literal word of the action, for example `reset` or `parameter`.
  - When a page can hold the same action for several objects, the suffix adds a stable key of the object after a hyphen. An example is `danger-notice-translation-de`.
  - The key is a value with a closed grammar, for example a language code of A17. It is not an `hx-*` value, so ADR-0037 (f) permits it.
- (e) The pages of this change get these ids:
  - user page, reset of the password: `danger-notice-reset`,
  - parameter page, delete of the parameter: `danger-notice-parameter`,
  - parameter page, delete of a translation: `danger-notice-translation-<lang>`, where `<lang>` is the language code of the row.
- (f) "Deactivate" on the user page stays a plain form with a danger button, outside a disclosure. `ActivateUser` undoes it, and `design-system.md` section 10.5 puts only an action that the system cannot undo into a disclosure.
- (g) A page test of each page with several danger disclosures checks that every `id` occurs once. It also checks that each `aria-describedby` of a danger button names an element of the page.

## Consequences

### Positive

- The parameter page and the user page keep their actions, and each danger button names its own warning.
- The version page and the recipe page keep their markup and their tests.

### Negative

- A template with several danger disclosures builds each id from a literal and a key. A wrong key can repeat an id. The test of point (g) finds that fault.
- The macro `danger` of the machine and unit archive keeps a notice without an id. This ADR does not change it.

### Risks and mitigations

- Risk: a later page adds a second macro action with the id `danger-notice`. Mitigation: point (c) permits that id once in a page, and the test of point (g) finds a repeated id.

## Related

- Amends ADR-0049 (f) (several danger actions on one page, one unique notice id for each).
- ADR-0037 (f), ADR-0043 (f), (k), ADR-0049 (g)
- `docs/architecture/design-system.md` section 10.5
- `overview.md` section 7: R39. Section 9: A17, A44
