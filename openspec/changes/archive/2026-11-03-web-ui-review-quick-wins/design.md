# Design

## Context

See proposal.md, section Why. The review notes in `target/ui-review/` are not normative. Each item of this change was checked against the templates, the handlers, and the documents below.

- Page layout and HTMX answers: `design-system.md` §10.1, ADR-0036, ADR-0037.
- Danger disclosure and buttons: `design-system.md` §10.5. Danger actions of a version: ADR-0049 (f). Notice ids: ADR-0110.
- Pagers and empty state: `design-system.md` §10.10, §10.12.
- Parameter page and translation rows: ADR-0039 (i), ADR-0043 (f), (j).
- Literal `hx-*` values and the CSP: `api.md` §9, ADR-0037 (f).
- Type usage query: `api.md` §2.3.
- Forced password change and `ExportClient`: `api.md` §2.8, ADR-0109 (e).

The page tests of `recipe-web` live in the `*_tests.rs` modules under `crates/recipe-web/src/pages/`. The tasks follow that place. Browser checks live in `e2e/tests/`.

## Goals / Non-Goals

**Goals:**

- Fix each item of proposal.md, section What Changes. The only document changes are ADR-0110, its note in ADR-0049, and its rule in `design-system.md` §10.5.
- Keep the three scripts, the CSP, the literal `hx-*` values, and the POST-only mutations.

**Non-Goals:**

- No new route, no new script, no migration, no new bundle language.
- No busy indicators. They wait for the ADR that changes `design-system.md` line 620 (proposal.md, Out of scope).
- No change of the translation section. A3 and B4 wait for `web-ui-data-safety` (proposal.md, Out of scope).

## Decisions

### D1. User page refresh out of band

The facts `dl` gets the id `user-facts`. The forms that depend on the user get the container `user-actions`. These forms are the role form, the deactivate form, the activate form, and the unlock form. The reset disclosure stands after that container and outside it.

An HTMX answer sends the parts out of band with literal `hx-swap-oob="true"`. The target stays `#user-result`. An answer to a request without `HX-Request` stays the full result page of `command_result`. It holds no out-of-band element and runs no `GetUser`.

- `SetUserRole`, `DeactivateUser`, `ActivateUser`, and `UnlockLogin` run `GetUser` after the commit. The HTMX answer sends `#user-facts` and `#user-actions`. A read error goes to the `WebError` mapping, which logs it once. A second try of these commands is harmless: it gives the same state or a `409` with its code.
- `ResetPassword` runs `GetUser` before the command. The HTMX answer sends `#user-facts` only. The facts show the forced password change from the role: true, except for an `ExportClient` account (`api.md` §2.8, ADR-0109 (e)). A read error stops the request before the command runs. A read error after the commit can therefore never hide a one-time password.
- The facts of the reset answer show the state that `GetUser` read before the command. A change by another Administrator between the read and the command may not show in them.
- Alternative: `HX-Refresh`. Rejected, because the reload drops the one-time password. Alternative: a second request with `hx-get`. Rejected, because the `hx-get` value would hold the login (ADR-0037 (f)).

With HTMX, the handler runs `GetUser` before `ResetPassword`. The read comes first on purpose. A failed read then never hides a one-time password. A crafted reset request from a user without the permission gets `403` from `GetUser`. The `forbidden` security event names `GetUser`, not `ResetPassword`. Only a crafted request reaches this case, because such a user cannot open the user page. The change accepts this consequence.

### D2. Applicable actions and focus

The deactivate button and the activate button share the id `user-status-submit`. After the swap, HTMX 2 finds the new element with the id of the old focus and puts the focus on it.

The reset disclosure is not part of `user-actions`. A command answer never swaps it. After a reset, the focus therefore stays on the reset button, and no focus lands on a hidden element.

The unlock button has no partner. After an unlock, the focus falls back to `body`. The browser then continues from the place of the button that the swap deleted. The next Tab reaches the first focusable element of the new answer in `#user-result`. Today this element is the link "Back to the user", which stands before `user-actions`. The user accepted this natural order on 2026-11-02.

The e2e check of task 1.1 asserts this target with `toBeFocused()` on the first link inside `#user-result`.

### D3. Reset disclosure on the user page

The reset form stays in `user.html` as one `details` element of `design-system.md` §10.5. The macros of `components/danger.html` target `#content` and push the URL, so they do not fit. The warning gets the id `danger-notice-reset` (design D10).

Task 1.2 names the keys. The key `user-reset-submit` exists without an argument. Its value changes to take the argument `login`. The keys `user-reset-summary` and `user-reset-warning` are new in `pl.ftl` and `en.ftl`.

### D4. Parent link and parameter heading

Each page view already holds the parent link of the end of the page. The template renders that link a second time in the header. The link follows the `h1`. A page with a large state badge shows the link after the row of that badge. The badge stays next to the `h1` (`design-system.md` §10.9). The link stands before the first `dl` of the page.

The key `parameter-title` gets the arguments `reference` and `number`. The `title` element uses the same key, so the browser tab names the recipe too. The key has two callers: `parameter_page` in `pages/parameters/page.rs` and `form_page` in `pages/parameters/reply.rs`. The second one renders the form page for `fields=1` and for the limit mode. Both callers pass all three arguments.

### D5. Type usage links

`TypeUsageRow` in `recipe-app` gets the field `key: ParameterKey`. The SQL of `recipe-db` selects `p.parameter_key` (`data-model.md` §1.2). The usage list holds only recipes that are not deleted (`api.md` §2.3), so each link reaches a page.

- The table grant on `parameter` already allows the `SELECT` (`grants.expected`). The change needs no grant change. The usage query keeps `READ ONLY` isolation and takes no lock.
- A new public field breaks every struct literal of `TypeUsageRow`. One task changes them all: `recipe-db/src/transaction/type_usage.rs`, `recipe-app/src/plant/queries/usage_tests.rs`, and `recipe-db/tests/storage/contract/types/usage.rs`. The same task fixes the doc comment of the struct, which says "five columns".
- The query result now names the key. The requirement "Type usage" of `plant-model` changes in this change, so the spec and the port agree.
- The page builds three `href` values. The same paths have builders in `pages/parameters/page.rs` and in `diff_view.rs`. One shared builder serves all three places.

### D6. `lang` on translations

The field macros of `components/field.html` get an optional argument `lang` with the default empty text. The translation row passes its language code. The code is a value of A17, so it is safe in an attribute that is not `hx-*`.

### D7. Jump link after the grid

A plain link with `href="#parameter-save-submit"` or `href="#parameter-add-submit"` follows the grid. HTMX does not boost a link whose `href` starts with `#`, so the link sends no request. The `scroll-padding-top` of `app.css` keeps the button clear of the sticky toast.

### D8. Small items

- Filtered recipe list: the page view gets `filtered`. It is true when `q`, `machine`, or `status` holds a non-empty value, or when `show_deleted` is on. The filter form sends the three fields also when they are empty, so a submit with empty values is not filtered. A new key `recipes-empty-filtered` and the existing link style serve the clear link. The `.empty` element holds one sentence (`design-system.md` §10.10), so the link stands outside the empty table row.
- User pager: the user list uses `pagers::pager` with the current page and `pagers::after_end` for a page after the last page (`design-system.md` §10.12). The keys `users-prev` and `users-next` go away in both bundles.
- Role selects: `user.html` and `user-new.html` use `fields::select`. The macro reads `option.value` and `option.text` and compares `option.value` with one `selected` value. `RoleOption` in `pages/users/templates.rs` therefore holds `value` and `text`, and each form passes its `selected` value. The new-user form stays without `hx-boost` (proposal.md, Out of scope). The handlers pass the error lines of a bad role to the views, so the select shows the line, `aria-invalid`, and `aria-describedby` (ADR-0041, `design-system.md` §10.5).
- Login focus: the macros `named`, `field`, and `plain` of `components/field.html` get an optional argument `autofocus`. Its default is false. Only `login.html` passes it, and only for the login input of a page without field errors.
- Type list: the key `type-latest` gets a capital letter. A new key `type-superseded` fills the other rows.
- Colons: a new key `user-heading-label` holds the label of the `h1` of the user page with its colon. The key `user-title` stays, because the `title` element and the result page use it without a colon. The values of `user-password-label` and `user-password-label-technical` end with the colon. The templates then hold no `: ` literal.
- Style sheets: delete `.panel` and its forced-colors entry, delete `.leaf-grid tbody[hidden]`, and merge the two `input, select` rules. Keep `.tree-group[hidden]`, because `.tree, .tree-group` sets `display: flex`. Keep `.btn-secondary` and `.hash` (`design-system.md` §10.5, §10.11).

### D9. Polish terms

The user chose "Wersja robocza" for the state Draft and "Tag" for a parameter tag (proposal.md, Decisions of the user). Only the files `pl.ftl` and `pl-approval.ftl` change. The message keys stay as they are. The bundle `en` stays as it is.

- Every `pl` value that holds a form of `szkic` or `znacznik` changes. This includes values for discard actions, export errors, the audit action, and the field error text.
- A form of `szkic` gets the matching case of `wersja robocza`. The writer checks the grammar of each sentence.
- The Rust tests that quote the old words change with the bundle values, in the same task. Many tests in other files quote a bundle value, so a split would leave the suite red.

### D10. Notice ids of danger disclosures

ADR-0110 permits several danger actions on one page and amends ADR-0049 (f). Each notice gets an id that is unique in the page.

- The macros `danger_versioned` and `danger_action` keep `danger-notice`. The version page and the recipe page hold at most one of them.
- The user page gets `danger-notice-reset`. The parameter page gets `danger-notice-parameter` and `danger-notice-translation-<lang>`.
- "Deactivate" stays a plain form with a danger button. `ActivateUser` undoes it, so it needs no disclosure (ADR-0110 (f)).
- A page test checks that each `id` occurs once and that each `aria-describedby` names an element of the page (ADR-0110 (g)).

### D11. Dev server settings for the e2e suite

The e2e server builds its state with `WebSettings::default()` (`crates/recipe-web/examples/dev_server.rs`). The default list of translation languages holds only `pl`. A row for another language gets no edit form, and a save for it gives a rejection. The translation checks of the e2e suite need `de`.

The example sets `translation_langs` to `pl` and `de`. This is the only change of that file. The production default stays as it is (`reliability.md` §7.11).

## Risks / Trade-offs

- [The new word "Wersja robocza" is longer than "Szkic" in the state badge.] → Task 8.7 runs the axe scan at 320 px.
- [An inflected form of `szkic` can stay in a long sentence.] → The test of task 6.1 searches every `pl` value for `szkic` in any letter case.
- [A read error after a commit shows an error, but the command took effect.] → This applies only to the four commands of D1. A second try of them is harmless. `ResetPassword` reads first.
- [The second parent link adds one tab stop per page.] → The link stands before the content. A keyboard user therefore reaches the way back first.
- [A CSS deletion can change a page and pass every unit test.] → Task 8.7 runs the axe scan and the page sweep.

## Migration Plan

No data migration. A rollback is a revert of the release.
