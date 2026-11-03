# ADR-0113: Pages in read-only mode

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

A24 puts the application into read-only mode when the free disk space is low or unknown. In that mode every command except `Login`, `Logout`, and the export returns `503 E-WEB-006` and writes nothing. ADR-0071 adds a banner at the start of `main` (`design-system.md` section 10.1).

- The page views do not read the mode. Every form and every submit button of a command stays on the page.
- A user can fill a leaf grid with many values and then gets `503 E-WEB-006`. HTMX puts the error into `#error-panel`, and the form keeps the input (ADR-0036). The user cannot save the input until the mode ends, and the effort is lost when the tab closes.
- `PageRequest` already holds the mode of the request (`ReadOnlyFlag`). `Operation::runs_in_read_only` already names the operations that the mode permits.
- The page views already hide the forms of a command when the role has no permission for it. The parameter page then shows the read-only grid (ADR-0043 (k)).

The UI review of 2026-11-02 (finding A6, F06, UI-04) reports the problem. A check of `base.html`, `gates.rs`, and `pages/parameters/page.rs` finds the same fault.

## Decision drivers

- A user sees before typing that a save is not possible (R31, R39).
- A24 stays as it is: every query runs, and every blocked command still returns `503 E-WEB-006`.
- The pages reuse the markup and the tests of the view without a permission.
- The UI meets WCAG 2.2 AA (A44). A control that cannot work does not take a place in the tab order.

## Considered options

### Option A: Hide each control of a blocked command

- Pros: the user cannot type into a form that cannot save. The page uses the view of a reader without the permission, which has tests today. No new style.
- Cons: the user does not see where the form would be. The banner must carry the reason.

### Option B: Show each submit button with `disabled` and `aria-describedby` to the banner

- Pros: the user sees the form.
- Cons: the inputs stay open, so the user can still type input that the system cannot save. A disabled button leaves the tab order, so a screen reader rarely reads its `aria-describedby`.

### Option C: Put each form into `fieldset disabled`

- Pros: one attribute disables every control of a form.
- Cons: the stored values show with `--color-text-muted` on `--color-sunken`. A disabled input leaves the tab order, so a keyboard user cannot reach or copy a value. Each form needs a new wrapper.

### Option D: Answer `503 E-WEB-006` on each form route

- Pros: no form appears.
- Cons: a form route is a query, and A24 keeps every query. The page with the stored data is then also lost.

## Decision

The project uses **Option A**.

- (a) `PageRequest` gets the method `may_run(operation)`. It returns true when the role of the session has the permission of the operation and the mode permits the operation. Without a session it returns the error `E-WEB-004` of `auth()`.
  - The mode permits an operation when the mode is off or `Operation::runs_in_read_only` is true.
  - The method reads the `ReadOnlyFlag` of the request. The gate and the page therefore use one value of the mode (ADR-0071).
- (b) Each page view that shows a control of a command uses `may_run` in place of the bare permission check. In read-only mode the page then shows the view of a reader without the permission.
  - A control is a form that sends the command, or a link or button whose only target is the form of the command.
  - Examples: the parameter page shows the read-only grid and the translation texts. The version page shows no link "Add a parameter", no rebind form, no link "Approve", and no danger disclosure. A list page shows no link to a new item. The recipe page shows no link to a new version, and the type page shows no link to a new revision.
- (c) These controls stay in read-only mode:
  - the login form, the logout form, and the language switch,
  - the export link,
  - every form with `method="get"`, every pager, and every navigation link.
- (d) A form route of a blocked command answers `200` in read-only mode. The route runs its permission check first (ADR-0046).
  - The page or fragment shows its heading and the notice `readonly-form-unavailable` in place of the form.
  - The notice is `alert::notice("warning", text)` with the icon `alert-warning` (`design-system.md` section 10.6).
  - The form routes are the routes of `api.md` section 2 that answer a form, for example `GET /recipes/new` and `GET /me/password`.
- (e) The banner of `design-system.md` section 10.1 keeps its text. It already says that changes are blocked and what still works. No hidden control remains, so no element needs `aria-describedby` to the banner.
- (f) The mode can start after a page loads. The page then keeps its forms. A submit gets `503 E-WEB-006` in `#error-panel`, and the form keeps the input (ADR-0036). A page that loads after the end of the mode shows its forms again.
- (g) A session with `must_change_password` still gets `303` to `/me/password` (`api.md` section 7.4). In read-only mode that page shows the notice of point (d) and the logout button.
- (h) The browser test starts a second instance of the example `dev_server` on its own port. That instance reads the variable `RECIPE_DEV_READ_ONLY_FILE`. The mode is on while that file exists. The instance checks the file every 250 ms without a blocking call.
  - A second Playwright project `read-only` runs only the read-only test. Its base URL names the port of the second instance. The first project ignores that test.
  - The seed fixture seeds the instance at the base URL of its project through the HTTP routes. Then the test creates the file and waits for the banner.
  - One module of the browser tests holds the absolute path of the file. The configuration and the test import it.
  - The tests on the first instance never see the mode, so they can run in parallel.
  - Only the example has this variable. The server binary `recipe-server` does not read it.

## Consequences

### Positive

- In read-only mode a user cannot type input that the system cannot save.
- Each page uses one rule, and the rule reuses the tested view without a permission.
- A24, the gate, and the error answer do not change.

### Negative

- A page that loaded before the mode started still shows its forms until the next load.
- The user does not see the place of a hidden form. The banner gives the reason.

### Risks and mitigations

- Risk: a new page uses the bare permission check and shows a form in read-only mode. Mitigation: a test reads every Rust source of `recipe-web/src/pages` except the test files.
  - The test ignores all whitespace, so it also finds a chain that rustfmt splits over two lines.
  - It finds no `requirement().allows` outside `pages/request.rs`. The navigation of `layout.rs` lies outside `pages/` and keeps its own check.
- Risk: the file of point (h) stays after a failed test run. Mitigation: the test deletes the file in `afterAll`, and the instance uses a path under `e2e/test-results/`.

## Related

- ADR-0036, ADR-0043 (k), ADR-0046, ADR-0048, ADR-0071, ADR-0073
- `docs/architecture/api.md` sections 2 and 5.1, `docs/architecture/design-system.md` sections 10.1 and 10.6
- `overview.md` section 7: R24, R28, R31, R39. Section 9: A24, A44
