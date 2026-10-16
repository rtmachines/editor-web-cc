# ADR-0048: Browser regression suite with Playwright

Status: Accepted
Date: 2026-10-16
Deciders: project owner

## Context

Step P3.3 built the pages of the recipe editor in the change `web-recipe-editor`. The archive of the change is `openspec/changes/archive/2026-10-16-web-recipe-editor/`.

Design D16 of `web-recipe-editor` names states that only a browser shows. Agents checked these states in a browser with the gstack tool `browse`. Those runs leave no regression test.

Several defects appeared only in a browser:

- A summary link widened the page at 320 CSS px. The cause was a long word in a box with `display: inline-flex`.
- The sticky path cell of the leaf grid fully hid focused inputs at 320 CSS px. It hid them again between 481 and about 690 CSS px (WCAG 2.4.11).

The CSS rule tests in `crates/recipe-web/src/style_tests/` check the declarations of a style sheet. They do not check the layout that a browser computes.

`reliability.md` section 6 names a `playwright-cli` smoke suite on the Compose stack in the row "End to end". No ADR records the tool, its runtime, the pins of its versions, or the place of its files. The runtime of Playwright is Node.

The product has no JavaScript build chain (ADR-0001). Only three scripts exist in `static/` (`api.md` section 9). That must stay true.

The example `crates/recipe-web/examples/dev_server.rs` runs the app with an in-memory store. It needs `RECIPE_DEV_PASSWORD` with 12 to 128 characters, and `RECIPE_DEV_PORT`. It has the users `admin`, `engineer`, and `operator`. A browser suite can run against it without Docker.

`playwright-cli` drives a browser and writes Playwright TypeScript for each action. Playwright Test, the package `@playwright/test`, runs these tests with `npx playwright test`.

## Decision drivers

- A defect that only a browser shows gets a regression test (R39, A44).
- The product keeps its three scripts and has no build chain (ADR-0001, `api.md` section 9).
- The regression suite runs in CI without Docker on each push and each pull request.
- The test tool chain has exact versions and a lock file.
- No password is in the repository.
- A test does not change the product that it tests.

## Considered options

### Option A: Playwright Test in TypeScript, written with `playwright-cli`

- Pros: the test runner has web-first assertions and traces. A test can read computed styles and request events. `playwright-cli` writes a test from the browser session of an agent.
- Cons: Node and npm enter the tool chain of development and CI. The repository gets a second lock file.

### Option B: A Rust browser driver over WebDriver or CDP

- Pros: the tests and the product use one language.
- Cons: the tests need a separate driver process. The driver has fewer helpers for layout and accessibility. No tool writes a test from a browser session.

### Option C: Agent checks only

- Pros: no new tool.
- Cons: an agent check leaves no regression test. A later change can bring back a defect, and nobody sees it until the next agent run.

## Decision

The project uses **Option A**. This ADR amends no earlier ADR. It gives the tool of the "End to end" row of `reliability.md` section 6 and records the new row "Browser regression".

- (a) Place of the suite.
  - The suite lives in `e2e/` at the root of the repository, outside the Cargo workspace.
  - `e2e/` holds `package.json`, `package-lock.json`, `playwright.config.ts`, and `tests/`.
  - Node is a test tool only. No file of `e2e/` enters a crate, the `static/` directory, or the runtime image.
  - The product keeps its three scripts and has no build chain (ADR-0001, `api.md` section 9).
- (b) Versions.
  - `package.json` gives `@playwright/test` an exact version. The repository holds `package-lock.json`.
  - CI installs the packages with `npm ci`.
  - The CI job and `e2e/.nvmrc` pin the major version of Node to one LTS line.
  - CI installs only the Chromium build of that Playwright version.
  - The nightly audit job also checks `e2e/package-lock.json` with `npm audit`.
- (c) Two suites.
  - The regression suite runs against `examples/dev_server.rs` with its in-memory store.
  - A global setup seeds the data through the HTTP routes, as the seed script of step P3.3 did.
  - The CI workflow runs the regression suite on each push to `main` and on each pull request.
  - The regression suite covers the browser-only checks of design D16 of `web-recipe-editor` and of `design-system.md`:
    - The page has no sideways scroll at 320 CSS px.
    - The keyboard path through the leaf grid reaches each input.
    - Enter in a value input saves the value. The request sends neither `fields` nor `limit_mode`.
    - A test checks which element has the focus after a swap.
    - No other element fully hides a focused element (WCAG 2.4.11).
    - A summary link moves the focus to its input.
    - A long word in the error summary does not widen the page.
  - Each page that the suite opens also gets the CSP checks:
    - The page has no `style` attribute, no `<style>` element, no inline script, and no `on*` attribute.
    - Each `hx-*` value is a literal (ADR-0037).
    - The console shows no CSP error.
  - The smoke suite is the "End to end" row of `reliability.md` section 6. It runs on the Compose stack.
  - The smoke suite needs the steps P3.4, P3.5, and P4.1.
- (d) A test only reads.
  - A test reads the page through the DOM, the computed styles, and the request events only.
  - A test never changes a style, a script, or a file of the product.
  - The tests use the widths 320, 768, and 1280 CSS px.
  - The tests use the default language `pl`. One test for each page area uses `en`.
- (e) Secrets and failures.
  - The passwords of the test users come from environment variables. No password is in the repository.
  - CI runs the tests with 0 retries, so a flaky test fails.
  - A failed test keeps its trace as a CI artifact.
- (f) Plan.
  - Step P4.2 builds both suites.
  - Until then, the browser checks of a change stay agent checks, as in step P3.3.
  - After step P4.2, a change adds a test to the regression suite for each new browser-only check.

## Consequences

### Positive

- Each browser-only state of design D16 and of `design-system.md` gets a regression test.
- The smoke suite of `reliability.md` section 6 gets a place and a tool with pinned versions.
- An agent writes a test from a `playwright-cli` session.

### Negative

- Node and npm enter the tool chain of development and CI.
- The repository has a second lock file.
- CI downloads a browser build.
- The CI run takes more time.

### Risks and mitigations

- Risk: a test fails at random because of timing. Mitigation: the tests use web-first assertions and no fixed sleep. CI runs with 0 retries, and each failed test keeps its trace.
- Risk: a package from the npm registry carries malicious code or a known vulnerability. Mitigation: exact versions, the lock file, `npm ci`, and the nightly `npm audit`.
- Risk: the seed data drifts from the routes. Mitigation: the global setup uses the HTTP routes. A change of a route therefore makes the setup fail with a clear error.

## Related

- ADR-0001, ADR-0036, ADR-0037, ADR-0047
- `reliability.md` section 6
- `design-system.md` sections 6 and 7
- `api.md` section 9
- `overview.md` section 7: R39. Section 9: A44
