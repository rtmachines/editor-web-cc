# Design

## Context

ADR-0158 records the decision. This design gives the places in the code. A probe on `main` at `774c409` showed the faults.

- F1 on `200`: the value input showed `11` after the user typed `22` during the request.
- F1 on `422`: the value input showed `abc` after the user typed `33`.
- F1 in a translation row: `Typed while saving` became `Sent before request`.
- F3: `route.abort("connectionfailed")` left `#error-panel` and `#toast` empty. In a second probe the server stored `55`, the browser lost the answer, and the page showed nothing.

The vendored `htmx.min.js` fires `htmx:afterRequest` in `onload`, `onerror`, `onabort`, and `ontimeout`. In `onload` it runs the answer handling and the swap first, then fires `htmx:afterRequest`. When the swap deleted the element of the request, HTMX fires the event on a kept ancestor. `event.detail.elt` is then that ancestor. When the answer handling throws, HTMX fires `htmx:onLoadError` and no `htmx:afterRequest`. Each event detail holds the same `xhr`.

Rules: ADR-0158, ADR-0001, ADR-0049 (j), ADR-0036, ADR-0041, `api.md` sections 5 and 9, `design-system.md` sections 10.6 and 10.7, and `reliability.md` sections 1.2 and 7.11.

## Goals / Non-Goals

**Goals:**

- No answer deletes text that the user typed after the send.
- Each end of a request without an answer gives a visible message with a code.
- The save forms of a draft show a request in flight.

**Non-Goals:**

- A merge of typed text into the answer.
- A retry, an offline queue, or a check of the result in the background.
- A change of a route, a form field, or a server error.

## Decisions

### D1. Script `static/request-state.js`

The script is one IIFE in strict mode, like `row-version.js`. It keeps a `WeakMap` from the `XMLHttpRequest` of a request to its form. It counts the open requests of each form. Its listeners are on `document`.

- `htmx:beforeRequest`: when `event.detail.elt` is a `form` or is inside one, the script locks the form of `event.detail.elt.closest("form")` and records it under `event.detail.xhr` (ADR-0158 (b)). It adds one to the count of open requests of that form. It also deletes the element `[data-transport-error]` from `#error-panel` (ADR-0158 (h)).
- `htmx:afterRequest` and `htmx:onLoadError`: the script finds the form that it recorded under `event.detail.xhr` and lowers its count (ADR-0158 (d)). The count never goes below zero. The script unlocks the form only when the count is zero, so two requests of one form keep the lock until the last one ends. It does not read `event.detail.elt`, because that element can be an ancestor.
- `htmx:sendError` and `htmx:timeout`: the script copies `#transport-error` into `#error-panel` with `replaceChildren`. It empties `#toast` and calls `scrollIntoView` on the panel (ADR-0158 (f)).
- A capture listener for `keydown`, `mousedown`, and `click` refuses a change of a `select`, a check box, or a radio button inside `form[aria-busy="true"]`. It permits Tab, each key with Ctrl, Alt, or Meta, and the keys F1 to F12, so the shortcuts of the browser work. It calls `preventDefault` for each other key, and does not move the focus (ADR-0158 (c)).

A text input needs no listener, because `readonly` stops the input. A request of a link (`hx-boost` on an `a`) has no form. It gets no lock, but its network error gives the message.

### D2. Static file and page

- `src/static_files.rs` gets `REQUEST_STATE_BYTES` and the row `/static/request-state.js` with the media type `application/javascript`. `src/routes/tests.rs` (`expected_routes`) gets the route.
- `base.html` loads `<script src="/static/request-state.js" defer>` after `row-version.js`.
- `base.html` renders `<template id="transport-error">` after `</main>` in the full page only. Its content is `<div data-transport-error>` around `alerts::alert("error", code, sentence, "")` of `design-system.md` section 10.6.
- The macro `alert` of `components/alert.html` renders the element `.alert-ref` only for a reference that is not empty. Each other caller passes a reference, so its output stays the same.
- The meta tag `htmx-config` gets `"timeout":45000`. A test compares the number with `htmx.request_timeout` of `reliability.md` section 7.11.
- `app.css` gets `form[aria-busy="true"] { cursor: progress; }` beside the rules of `.htmx-indicator` (ADR-0158 (n)).

### D3. Error code `E-WEB-015`

- `reliability.md` section 1.2 gets the row `E-WEB-015`. The owner column names the script `request-state.js`. The HTTP column holds `-`. The text column holds the English sentence below.
- `crates/recipe-web/src/error/status/tests.rs` reads the HTTP column of each row. A test-first task makes it skip a row with `-` and still check every other row.
- `error_codes.rs` gets `ErrorCode::WEB_015` in the constants and in the list of all codes. `base.html` reads the code text from that constant.
- The sentence is the key `error-E-WEB-015` in `en.ftl` and `pl.ftl`. The i18n test of `MESSAGE_PREFIXES` then finds it, and the English text equals the row. The English text: "The server sent no answer. The result of the action is not known. Reload the page and check the data before you try again."

### D4. Indicator of the save forms

`components/grid.html` puts `<span class="htmx-indicator">` with `form-in-progress` after the save button of the parameter form. `translation-row.html` puts it after the save button of a row. The delete form of a row gets no indicator. The key stays in `*-approval.ftl`, because the bundles load that file on every page.

### D5. Documents

The change edits these documents in the same commit as ADR-0158:

- `api.md` section 5.2: the second exception for `E-WEB-015`. Section 9: the script table, the HTMX configuration with `timeout`, and the rule of the lost answer.
- `reliability.md` sections 1.2 and 7.11: the row of the code and the setting `htmx.request_timeout` with the check `http.request_timeout < htmx.request_timeout`.
- `design-system.md` section 10.6: the fragment of a lost answer without a reference. Section 10.7: the script empties `#toast` on a lost answer.
- The Status lines of ADR-0001, ADR-0036, ADR-0047, and ADR-0049 get "Amended by ADR-0158".
- `.claude/rules/web.md` and `docs/adr/README.md`: the four scripts, the exception of the reference, the index row, and the decision map row "Frontend".

## Risks / Trade-offs

- [A slow request blocks the input up to 45 s] → The server stops a request at 30 s with `503 E-WEB-009`. The indicator and the cursor show the wait.
- [A browser test cannot hold a request for 45 s] → The browser test of the timeout sets `htmx.config.timeout` low with `page.evaluate`. A Rust test checks the value of the meta tag.
- [The capture listener can block a key that a screen reader or the browser needs] → The listener blocks only some keys. Those keys are on a `select`, a check box, or a radio button of a busy form. Tab, keys with Ctrl, Alt, or Meta, and F1 to F12 pass, so F5 and Ctrl+R still reload the page. A manual check with a screen reader is a task.
- [A swap deletes the form before `htmx:afterRequest`] → The script finds the form through the `xhr`, not through `event.detail.elt`. The unlock of a deleted form changes nothing on the page.
- [The answer handling throws] → `htmx:onLoadError` also unlocks. A browser test answers with `HX-Retarget: #missing` and checks the unlock.
- [After a lost save, the hidden `row_version` can be stale] → The next save can get `409 E-WEB-001`. The sentence of `E-WEB-015` tells the user to reload first.
