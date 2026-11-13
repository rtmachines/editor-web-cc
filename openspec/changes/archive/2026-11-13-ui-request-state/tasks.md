# Tasks

## 1. Decision and normative documents

- [x] 1.1 (docs/adr) Set ADR-0158 to `Accepted` after the owner approves this change. Add "Amended by ADR-0158" to the Status lines of ADR-0001, ADR-0036, ADR-0047, and ADR-0049. Add the index row and the decision map entry in the row "Frontend". Check: `ste-lint.py` on the six files.
- [x] 1.2 (docs/architecture) Update `api.md` section 5.2 with the exception of `E-WEB-015`. Update section 9: the fourth script with 40 lines, `"timeout":45000`, and the message of a lost answer. Check: `ste-lint.py`.
- [x] 1.3 (docs/architecture) Update `design-system.md` section 10.6 for the fragment without a reference. Update section 10.7: the script empties `#toast` on a lost answer. Add the cursor rule of ADR-0158 (n). Check: `ste-lint.py`.
- [x] 1.4 (.claude/rules) [mechanical] In `.claude/rules/web.md`, list the four scripts with their limits and add `timeout` to the HTMX configuration. Add the exception of `E-WEB-015` to the rule of the error fragment. Check: `grep -n "request-state\|E-WEB-015" .claude/rules/web.md`.

## 2. Error code (recipe-web, recipe-domain)

- [x] 2.1 (recipe-web) Write a failing test in `src/error/status/tests.rs` that a registry row with `-` in the HTTP column is skipped. Every other row must still parse. Then change the parse of the HTTP column.
- [x] 2.2 (recipe-domain, recipe-web) Write failing tests. `ErrorCode::WEB_015` has the text `E-WEB-015` and stands in the list of all codes. The i18n test finds `error-E-WEB-015` in `en.ftl` and `pl.ftl`, and the English text equals the registry row. Then add, in one step, the row `E-WEB-015` of design D3 to `reliability.md` section 1.2, `htmx.request_timeout` to section 7.11, the constant, and the two keys. The registry tests read the row, the constant, and the keys, so they land together. Check: `cargo test --workspace` passes.

## 3. Page parts (recipe-web)

- [x] 3.1 (recipe-web) Write a failing render test of the macro `alert`. An empty reference gives no element `.alert-ref`. A reference that is not empty keeps it. Then add the condition to `components/alert.html`.
- [x] 3.2 (recipe-web) Write failing page tests in `src/pages/page_tests.rs`. A full page holds `"timeout":45000` in `htmx-config`. It holds `<template id="transport-error">` after `main` with `data-transport-error`, `role="alert"`, `E-WEB-015`, and no `.alert-ref`. Then change `base.html`.
- [x] 3.3 (recipe-web) Write a failing page test. The edit page and the new form of a parameter have the indicator of `form-in-progress` in the parameter form. Each translation row has it in its save form. The delete form of a row has none. Then change `components/grid.html` and `translation-row.html`.
- [x] 3.4 (recipe-web) Write a failing style test that `app.css` holds `form[aria-busy="true"]` with `cursor: progress` and no other property. Then add the rule.

## 4. Script (recipe-web)

- [x] 4.1 (recipe-web) Write failing tests in `src/static_files/tests.rs`, `tests/web/static_files.rs`, and `src/routes/tests.rs` (`expected_routes`). `GET /static/request-state.js` gives `200` with `application/javascript` and `nosniff`. The file has at most 40 lines. A full page loads the four scripts and no other script. Then add an empty `static/request-state.js`, its row in `static_files.rs`, and the tag in `base.html`.
- [x] 4.2 (recipe-web) Write failing browser tests in a new `e2e/tests/request-state.spec.ts` for the lock on `200` and on `422` of the parameter form. Hold the request with `page.route`. Then write the lock and the unlock by `xhr` of design D1 in `static/request-state.js`. Check: `npm test -- request-state`.
- [x] 4.3 (recipe-web) Write failing browser tests for the translation row on `200` and on `422`, and for Enter and a click. Then complete the script. Check: `npm test -- request-state`.
- [x] 4.4 (recipe-web) Write failing browser tests for the unlock after an abort and after an answer with `HX-Retarget: #missing` (`route.fulfill`). Then add the listener of `htmx:onLoadError`. Check: `npm test -- request-state`.
- [x] 4.5 (recipe-web) Write failing browser tests for the scenarios of "Message of a lost answer". Use `route.abort("connectionfailed")` before the send, and `route.fetch` plus `route.abort` after the save. For the timeout, set `htmx.config.timeout` low with `page.evaluate`. Cover the empty `#toast` after a success. Then write the listeners of design D1. Check: `npm test -- request-state`.
- [x] 4.6 (recipe-web) Write failing browser tests. A `select` of the leaf grid of a busy form keeps its value after ArrowDown, and Tab moves the focus. An input that the test sets `readonly` with `page.evaluate` before the send stays `readonly` after the request. Then complete the capture listener. Check: `npm test -- request-state`.

## 5. Checks of the change

- [x] 5.1 (openspec/changes/ui-request-state) Run `openspec validate ui-request-state --strict` and `ste-lint.py` on the change and on the documents of group 1. Check: 0 errors and 0 hard violations.
- [x] 5.2 (e2e) Run the full browser suite with `E2E_PASSWORD=... npm test`. The tests `accessibility`, `focus`, `leaf-grid`, and `translations` must still pass. Check: every test passes.
- [ ] 5.3 (e2e) Check by hand with Orca or NVDA. The reader announces the message `E-WEB-015` once. The reader reads a locked field while the user types in it, and reads it again after the unlock. Check: a note in the task report with the reader, the browser, and each result. Skipped by the owner on 2026-11-12: no screen reader is available. The risk of the capture listener in design.md stays open.

## Workflow follow-up

- Review the code with `code-review` (Opus). Fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion` with clippy, test, and fmt of CI.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate --strict` pass.
