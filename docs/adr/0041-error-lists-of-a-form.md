# ADR-0041: Error lists of a form

Status: Accepted. Amended by ADR-0046 (a form route runs its form query before it reads the fields in (a)). Amended by ADR-0050 (point (d): a query value without an input gives a line with no link)
Date: 2026-10-15
Deciders: project owner

## Context

A form answer of step P3.2 shows one field error at a time. The code review of P3.2 deferred two findings to step P3.3:

- U25: a form shows only one field error, also when several fields are bad.
- U33: the error line stands only in `#error-panel`, not under the input. This applies also to a member row of the UDT form.

`design-system.md` section 10.5 says that step P3.3 adds an error line under the input.

The parameter form of ADR-0039 can have 16 384 leaves. Today `ParameterValue::parse` and `ParameterValue::check` of `recipe-domain` stop at the first fault. A user would then fix one leaf per save.

ADR-0027 gives `recipe-domain` the list type `RecipeErrors` and the sum `DraftError`. Their `code()` returns the code of the first entry, and `recipe-web` shows one line for each entry. Today only the approval check and the rebind plan return a list.

`WebError` has the variants `BadRequest` and `BadRow`, and each holds one field. `answer.rs` builds the field lines from these two variants only.

## Decision drivers

- A user sees every error of one save at once (U25).
- Each error line stands under its input and also in a summary with a link (U33, `design-system.md` section 10.5).
- No new error code. `reliability.md` section 1.2 stays the only registry.
- The CSP, the scripts, and the literal `hx-*` attributes do not change.

## Considered options

### Option A: The domain and the web layer both collect

- Pros: one save shows every error. The lists use the types of ADR-0027.
- Cons: the value check and the limit check of `recipe-domain` change.

### Option B: Only the web layer collects

- Pros: `recipe-domain` does not change.
- Cons: a value with many bad leaves still needs one save per leaf.

### Option C: One error at a time

- Pros: no change.
- Cons: U25 stays open.

## Decision

The project uses **Option A**.

- (a) `recipe-web` reads every field of a form before it calls a use case. It collects every bad field (`E-WEB-008`) and answers once with all of them.
- (b) `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, and `ReplaceLimits` check every leaf of the value and every limit row. They collect every fault of one parameter in one `RecipeErrors` and return it through `DraftError` (ADR-0027).
  - Each entry keeps its code (`E-RCP-004`, `E-RCP-005`, `E-RCP-006`, `E-RCP-017`, `E-RCP-018`, `E-RCP-019`) and its leaf.
  - The entries come in the preorder of the leaves, then in the order of the sent limit rows.
  - A check of the unit, the type, the tag, the S7 address, or a budget keeps its single error.
- (c) `WebError` gets one variant with a non-empty list of field errors. Each entry holds a code, its message arguments, and the field or the leaf. `code()` returns the code of the first entry, so the status follows that code (`api.md` section 5.1).
  - The variant also holds the rendered form in a type whose `Debug` writes no content, because the form holds the CSRF token.
- (d) An HTMX answer puts `field-errors.html` into `#error-panel` (`api.md` section 5.3). The fragment shows the count and at most 20 lines, then the count of the other lines. Each line shows its code and links to its input.
  - Only the code and the count stand inside the element with `role="alert"`, so a screen reader reads a short alert.
- (e) The same answer swaps the form out of band. The form has a fixed `id`, and the attribute `hx-swap-oob="true"` is a literal. The form keeps the sent texts and the sent `row_version`, and it shows each error line under its input (`design-system.md` section 10.5).
  - A group of the leaf grid that holds an error is open (ADR-0039 (l)).
  - Each submit button of the form has a fixed `id`, so HTMX keeps the focus on the button after the swap.
- (f) A line of `E-RCP-020` names the parameter and the leaf of its item. A line of a list of `RecipeErrors` names the tag and the leaf.
- (g) This ADR applies to the forms of the recipe pages and of the plant pages. It also applies to the member rows of the UDT form (ADR-0037 (k)). The forms of the login, the own password, and the users keep one line per answer until step P4.2.
- (h) A success answer of a form also swaps the form out of band. The form then holds the saved texts and no error line.

## Consequences

### Positive

- One save shows every error of a form.
- Each error stands under its input, so a user finds it without a search.

### Negative

- An error answer of a form is larger, because it holds the form again.
- The value check and the limit check of `recipe-domain` walk every leaf also after a fault.

### Risks and mitigations

- Risk: a list grows to thousands of lines for a large value. Mitigation: `budget.leaves_per_parameter` limits the count, and the summary shows the count first.
- Risk: the out-of-band form and the summary disagree. Mitigation: one function builds both from the same list, and a test compares them.

## Related

- ADR-0027, amended by this ADR in (b): the draft commands also return `RecipeErrors`.
- ADR-0036, ADR-0037, ADR-0039
- `api.md` sections 5.1, 5.2, 5.3
- `design-system.md` sections 10.5, 10.6
