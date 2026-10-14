# Design: web-ui-foundation

## Context

See `proposal.md`, section Why. Step P2.5 left these parts of `recipe-web`:

- `base.html` with the `htmx-config` meta tag, `hx-history`, `hx-headers`, `#error-panel`, and `main`. No style sheet, no navigation, no language switch.
- `static/htmx.min.js` 2.0.11 with a SHA-256 test, served by one route row with `Access::Static`.
- The bundles `i18n/pl.ftl` and `i18n/en.ftl`, the type `Texts`, and a test that each English key has a Polish key.
- The pages `/login`, `/me`, `/me/password`, `/users`, `/users/new`, `/users/{login}`, the result pages, and the error page. Each page struct carries `lang`, `title`, `csrf_token`, and `t`.
- `POST /lang` with the rule of the field `next`. No page holds the form yet.
- `IdentityContext::app()` already gives the `AppContext` (`recipe-app/src/identity/context.rs`).
- `xss_tests.rs` forbids `<svg` on every page. `error/tests.rs` expects one `<script` and `HX-Reswap: innerHTML`. `pages/texts/tests.rs` reads `templates/` without recursion.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Tokens, contrast pairs, typefaces, sizes, icons, states, components | `docs/architecture/design-system.md` sections 3 to 12 |
| Decisions of this change | ADR-0036 (a) to (p) |
| Rendering, HTMX, CSP, scripts, template rules | ADR-0001, `api.md` sections 1, 5, 9, `.claude/rules/web.md` |
| `HX-Reswap` of an error answer | `api.md` sections 4 and 5.3, ADR-0036 (p) |
| Static route, language cookie, `POST /lang` | `api.md` sections 2.1 and 8, ADR-0035 (o) |
| Permission check first, HTTP only authenticates | ADR-0013, `api.md` section 3.2 |
| UI texts in `fluent` | ADR-0012, A17 |
| Pipeline, CSRF, error mapping | archived design `identity-and-sessions` D11 to D13 |
| Accessibility target | A44 |
| Users and their tasks | `docs/plan/ux-brief.md` (not normative) |

## Goals / Non-Goals

**Goals:**

- Every full page of the route table has the layout of `design-system.md` section 10.1, in `pl` and in `en`.
- Tests keep `app.css` equal to the token tables and every contrast pair above its minimum.
- No template holds a literal word, an inline style, an inline script, an event handler, or an unescaped output.
- `/recipes` answers every role. The login redirect and the language redirect reach a page.

**Non-Goals:**

- No new crate, no migration, no setting, and no change of the CSP.
- No behavior test of the two scripts in a JavaScript runtime (ADR-0001 has no JavaScript chain). The code review checks them in a browser.
- No component that P3.1 does not render or test. The diff markers come in P3.4. An error line under its input comes in P3.3.

## Decisions

### D1. Scope (answers of the user, 2026-10-14)

Before the propose step:

| Question | Answer | Record |
|---|---|---|
| a. Location of the UX brief and the design document | `docs/plan/ux-brief.md` (not normative) and `docs/architecture/design-system.md` (normative) | ADR-0036 (a) |
| b. Own ADR | ADR-0036 | ADR-0036 |
| c. Minimum touch target | 44 by 44 CSS px, gap 8 px | ADR-0036 (c), A44 |
| d. Icons | inline SVG macros with a fixed tag per icon | ADR-0036 (f) |
| e. `/recipes` | skeleton with a query that has no port | ADR-0036 (l), D9 |
| f. Typefaces | Atkinson Hyperlegible Next and Mono, 8 files in the repository | ADR-0036 (e) |
| g. Palette and risks | amber draft, yellow and black focus ring, color only for meaning | ADR-0036 (b), (d), (g) |

After the plan review (findings F1 to F30, all accepted):

| Question | Answer | Record |
|---|---|---|
| Q1. Tree pattern (F8) | disclosure: nested lists with buttons, no `role="tree"` | ADR-0036 (n) |
| Q2. Switch on a page that answers a POST (F1) | no switch. A GET page keeps `next` with path and query, no length limit | ADR-0036 (j) |
| Q3. Field errors (F5) | `#error-panel` with the label of the field and a link to the input | ADR-0036 (o) |
| Q4. Regions and scroll (F13, F14) | `#error-panel` and `#toast` first in `main`. `HX-Reswap: innerHTML show:#error-panel:top` | ADR-0036 (i), (p) |
| Q5. Legibility (F15) | data in `--text-base`, panel edge `--color-border` | ADR-0036 (e), `design-system.md` 5, 6 |
| Q6. Other findings | all accepted | this design |

### D2. Module layout

- `recipe-app::recipe::queries`: `ListRecipes` and `RecipeList` (D9).
- `recipe-web::static_files` (D3): `StaticFile` and `STATIC_FILES` with 4 files, `FontFile` and `FONTS` with 8 files. The SHA-256 constants of the fonts live in the tests.
- `recipe-web::layout` (new): `Layout`, `NavLink`, and `NavTarget` (D5).
- `recipe-web::error` (D13): `reason.rs` holds `FieldReason`. `values.rs` gives the key and the arguments of each value in an error message.
- `recipe-web::pages::recipes` (new): the handler of `GET /recipes` and its page struct.
- `templates/base.html`: the layout. `templates/components/*.html`: the askama macros of D6. `templates/recipes.html`: the page.
- `templates/user-result-body.html`: the result of a user command. `user-result.html` and `user-result-page.html` include it.
- `templates/test/`: the test templates of the components (D6).
- New test modules, so that no file passes 800 lines:
  - `style_tests.rs` with the parser `style_tests/css.rs` and the rule tests `style_tests/rules.rs` (D4)
  - `template_tests.rs` (D8, D10), `components_tests.rs` (D6), and `layout/tests.rs`
  - `pages/page_tests.rs` (every page in both languages) and `pages/recipes/tests.rs`
  - `pages/sweep_tests.rs`, whose list `PAGES` names every full GET page of the route table (D10)
  - `error/fragment_tests.rs` (the error fragments) and `error/value_tests.rs` (D13)
  - `style_tests/review_rules.rs` and `pages/review_fix_tests.rs` (the fixes of the code review)
- `error/tests.rs` and `csrf/tests.rs` get no new test.

### D3. Static files (ADR-0036 e, h)

- Rows of the route table, each with `Access::Static`: `/static/htmx.min.js`, `/static/app.css`, `/static/row-version.js`, `/static/tree.js`, and one fixed row per font file.
- One table `FONTS` of 8 entries gives the font rows. The function that builds the route table walks it. An unknown file name reaches the router fallback, so its `404` keeps `Cache-Control: no-store`.
- The binary embeds each file with `include_bytes!`, so no file system access happens at run time.
- The media types are `text/css; charset=utf-8`, `application/javascript`, and `font/woff2`. `Cache-Control` is `public, max-age` of `static.cache_max_age`.
- The implementer downloads `https://registry.npmjs.org/@fontsource/<family>/-/<family>-5.3.0.tgz` for both families into an empty directory in the scratchpad. `curl -fsSL -o <path> <url>` is the only download command.
- The SHA-512 of each tarball must equal `dist.integrity` of `https://registry.npmjs.org/@fontsource/<family>/5.3.0`. Only then does the implementer extract the 4 files and `LICENSE` of each family.
- The files go to `static/fonts/`. Each `LICENSE` goes to `static/fonts/OFL-<family>.txt`, because the two texts differ in their copyright lines. No route serves them.
- The SHA-256 constants of the 8 files go into the test module after the check. A font answer has no `ETag`, because `Cache-Control` decides the caching.

### D4. Tokens and style tests (ADR-0036 a, c, d, `design-system.md` 3 to 12)

- `app.css` holds one `:root` block with every token of the token tables of `design-system.md`. Every other rule uses `var(--…)` for a color, a font family, a text size, a space, an icon size, and an edge width.
- A style test reads `design-system.md` with `include_str!` and parses each table with the header `| Token | Value | Use |`. Each row must equal the value of `:root`. A token of `:root` without a row fails too. The SQL test of `data-model.md` section 5.4 is the precedent.
- A contrast test parses the table `| Foreground | Background | Minimum |` of section 4. It computes the WCAG 2.2 ratio from the values of `:root` with `f64`, and each ratio must reach its minimum.
- A rule test parses `app.css` as a flat list of rules. The format has no nesting, no `@import`, and at most one `@media` level. The test fails on a construct that it cannot read. It checks these rules:
  - Outside `:root`, no hex color, no `rgb(`, no `hsl(`, and no named color except `transparent`, `currentColor`, and the system colors of `design-system.md` section 12.
  - `button`, `input`, `select`, `.btn`, `.nav-link`, `.tree-node`, `.target-link`, and `.table-wrap a` set `min-height` to `var(--target)`. The same rules except `input` and `select` set `min-width` to `var(--target)` and `display` to `flex` or `inline-flex`.
  - `.actions`, the navigation list, and the language switch set `gap` to `var(--target-gap)`.
  - `:focus-visible` uses `--color-focus`, `--color-focus-ring`, and a transparent outline of `var(--focus-width)`.
  - The rule of `[aria-current]` in the top bar sets a bottom border of `var(--edge-current)` and the weight 700. `:root` sets `--edge-current` to `3px`.
  - The rule of `.data` sets `overflow-wrap: anywhere`. A flex item with text sets `min-width: 0`.
  - Only the rules of `.nav-link` and `.btn` set `text-decoration: none`.
  - `.htmx-indicator` exists and changes only `opacity`.
  - No rule sets `animation` or `transition`.
  - A block `@media (forced-colors: active)` exists.
  - There is one `@font-face` per font file, with `font-display: swap` and the `unicode-range` of its subset.
- The parser is the test helper `style_tests/css.rs` of about 240 lines. It needs no new crate.

### D5. Layout and navigation (ADR-0036 i, j, m, p)

- `Layout<'a>` holds the language, the page title, the texts, and the navigation links. It also holds the CSRF token or an empty text, the pre-session nonce or `None`, the login of the user or `None`, and `next` or `None`.
- `Layout` also holds the toast text or `None`. A full page that answers a command without HTMX renders its success sentence into `#toast` with the toast macro.
- Each page struct gets the field `layout: Layout<'a>`. `base.html` reads only `layout`. Its constructors:
  - `Layout::for_session(texts, request, title, current)` for a page with a session.
  - `Layout::for_login(texts, lang, title, nonce)` for the login page. `next` is `/login`. The error layer can call it, because it needs no `PageRequest` (F18). The login page keeps the switch also as the answer of a failed `POST /login` (code review R4, spec "Language switch").
  - `Layout::bare(texts, lang, title)` for the full error page. The error layer runs before the session layer, so it has no session data. It has no navigation, no switch, and no `hx-headers`.
- `PageRequest` gets `path_and_query()`, which returns the path and the query of a `GET` request and `None` for another method. `None` means no language switch (F1).
- `NavTarget` has the variants `Recipes`, `Users`, and `Me`. Each variant names its path, its text key, and its query: `ListRecipes`, `ListUsers`, and `GetOwnProfile`.
- A link shows when `operation.requirement().allows(role)` is true. `authorize` checks the same row, so the two cannot drift apart. A session with `must_change_password` gets no link.
- The language switch is a plain form `POST /lang` without an `hx-*` attribute. The buttons carry `lang="pl"` and `lang="en"`. The server answers `303`, and the browser loads the page again.
- The logout form of `logout-form.html` moves into the top bar. The pages lose their own logout form.
- `base.html` keeps the `htmx.min.js` tag of today. It adds `/static/app.css`, then `row-version.js` and `tree.js` with `defer`.
- `main` starts with `#error-panel` (no role) and `#toast` (`role="status"`, `aria-live="polite"`), then the `h1` of the page.
- The full error page shows the `h1` of the key `error-page-title`, the error, and the link `error-page-home` to `/recipes`. Its title is the code, the text of `error-page-title`, and `app-name`.
- `answer.rs` sets `HX-Reswap: innerHTML show:#error-panel:top` on every HTMX error answer except `E-WEB-004` (ADR-0036 p).

### D6. Components (ADR-0036 f, g, n, o, `design-system.md` sections 8 to 10)

- Macros in `templates/components/`:
  - `icons.html`: one macro per icon of section 8. Each output starts with the fixed tag of its icon, which holds `data-icon="<name>"`.
  - `badge.html`: `badge::status(kind, word)`, `badge::user(active, word)`, and `badge::status_large(kind, word)` for the large badge next to the `h1`.
  - `alert.html`: `alert::alert(severity, code, message, reference)` is one `div` with `role="alert"`. It holds the icon of the severity, the code, the sentence, and the reference.
  - `alert::alert_fields(severity, code, message, lines, link, reference)` adds the field lines between the sentence and the reference.
  - `alert::notice(severity, message)` shows a static message with the look of the alert and no role, for example the notice of a one-time password. A page that loads does not announce it.
  - `field.html`: label, hint, required word, input with `aria-required="true"`.
  - `table.html`: the frame with `role="region"`, `tabindex="0"`, `aria-labelledby` on the `caption`, and the empty row.
  - `tree.html`: `node(id, path, name, expanded)` of the disclosure pattern and `leaf(path, name)`. The group id is the fixed prefix `tree-group-` and a UUID. A leaf has the class `tree-leaf`, which aligns it with the words of a node.
  - `toast.html`: `success(text)`.
  - `empty.html`: `empty(text)`.
- The implementer checks the syntax of `{% import %}` and `{% call %}` for askama 0.16.1 with Context7 before the first macro. Each template that calls a macro has its own top-level `{% import %}`.
- A component that no page of P3.1 renders gets a test template in `templates/test/`, and `components_tests.rs` renders it. This applies to the status badge of a version, the tree, the table frame, and the toast.
- The users page shows the user state with the user badge.
- `field-errors.html` calls `alert::alert_fields`. Each line shows the label of the key `field-<name>`, where `_` becomes `-`. `ErrorView` gets the flag `link_fields`. It is true for an HTMX fragment and for the login page with its form. Then a line links to `#<name>`, but only for a field with an input: `login`, `password`, `current_password`, `role`, and `q`. The sentence of `E-WEB-008` also names the label, not the name.
- A success of a user command answers with the content for `#user-result`, which has no live role. The answer also holds an out-of-band swap of `#toast` with the toast macro. A second out-of-band swap empties `#error-panel`. Each `hx-swap-oob` value is a literal. The one-time password stays in `#user-result`.
- The templates drop `required`, `pattern`, `minlength`, and `placeholder`. `maxlength` stays (`design-system.md` section 10.5).
- A stand-alone link gets the class `.target-link`. A link inside a sentence keeps the text style and its underline.

### D7. Scripts (`api.md` section 9, ADR-0036 n)

- `row-version.js`, at most 20 lines: one listener on `document` for the event `rowVersion`. `HX-Trigger: {"rowVersion": <n>}` gives `event.detail.value` as a JavaScript `Number` in HTMX 2.0.11. The script accepts only `Number.isSafeInteger(v) && v >= 0`. It then writes `String(v)` into every `input[name="row_version"]`.
- `tree.js`, at most 50 lines: one listener on `document` for `click` on a button of `.tree` with `aria-controls`. It toggles `aria-expanded` and the `hidden` attribute of the group, and the focus stays on the button. It finds the group with `document.getElementById`. Enter and Space reach the native button. A node without a group does nothing. P3.2 adds the HTMX load of the children.
- Both scripts are a strict IIFE. They hold no `eval`, no `Function(`, no `innerHTML`, no `outerHTML`, no `insertAdjacentHTML`, no `document.write`, no `querySelector` on a built text, and no network call.
- A script test checks the line limits and these words. The code review loads a test page in a browser and records the result, because no JavaScript runtime runs in CI.

### D8. Texts (ADR-0012, ADR-0036 k)

- The Haiku task copies this table into `en.ftl` and `pl.ftl`. The English text follows ASD-STE100. The Polish text is the translation.

| Key | en | pl |
|---|---|---|
| `app-name` | Recipe editor | Edytor receptur |
| `skip-to-content` | Skip to the content | Przejdź do treści |
| `nav-label` | Main | Główna |
| `nav-recipes` | Recipes | Receptury |
| `nav-users` | Users | Użytkownicy |
| `nav-me` | My account | Moje konto |
| `lang-switch-label` | Language | Język |
| `lang-switch-pl` | Polski | Polski |
| `lang-switch-en` | English | English |
| `user-signed-in` | Signed in as | Zalogowano jako |
| `recipes-title` | Recipes | Receptury |
| `recipes-empty` | No recipe to show. | Brak receptur do pokazania. |
| `status-draft` | Draft | Szkic |
| `status-approved` | Approved | Zatwierdzona |
| `status-retired` | Retired | Wycofana |
| `form-required` | required | wymagane |
| `error-page-title` | The request failed | Żądanie nie powiodło się |
| `error-page-home` | Open the recipe list | Otwórz listę receptur |
| `field-form` | Form | Formularz |
| `field-lang` | Language | Język |
| `field-login` | Login | Login |
| `field-password` | Password | Hasło |
| `field-current-password` | Current password | Obecne hasło |
| `field-role` | Role | Rola |
| `field-query` | Search fields | Pola wyszukiwania |
| `field-q` | Login contains | Login zawiera |
| `field-page` | Page | Strona |
| `field-X-Forwarded-For` | Header X-Forwarded-For | Nagłówek X-Forwarded-For |

- The existing keys `logout-submit`, `user-status-active`, `user-status-inactive`, and `error-reference` stay.
- The text test strips `{# #}`, `{% %}`, `{{ }}`, the markup, and the character entities from each template. No letter may remain.
- The test also reads the attributes `aria-label`, `title`, `placeholder`, and `alt`. Outside an askama expression they hold no letter.
- The icon macros hold no text, because each icon has `aria-hidden="true"` and stands next to a word.
- The existing key parity test of `i18n/tests.rs` covers the new keys. The scan of `pages/texts/tests.rs` becomes recursive in the task that adds `templates/components/` (F17).
- A test checks that each field name of a `WebError::BadRequest` in `recipe-web` has its `field-` key.

### D9. `/recipes` skeleton (ADR-0036 l)

- `ListRecipes::execute` runs `authorize(Operation::ListRecipes, …)` as its first statement and returns `RecipeList`. P3.1 has no read port, so the list has no rows. P3.3 adds the port, the filters `q`, `machine`, `status`, `page`, and the rows.
- `recipe-web` builds the query with `ListRecipes::new(state.identity().app().clone())`. The accessor exists.
- The route row is `GET /recipes` with `Operation::ListRecipes`. The session layer, the password gate, and the read-only gate apply without a change. `ListRecipes` runs in read-only mode.
- The page shows the layout, the heading `recipes-title`, and the empty state `recipes-empty`.
- The route tables of `routes/tests.rs` get the row of `GET /recipes` with the answers `[Login, Page, Page, Page, Password, Login]` for the six principals.

### D10. XSS and security

- The corpus of `tests/web/xss-corpus.txt` goes into the query of `GET /me`, `GET /login`, and `GET /recipes`. These routes accept any query. The test sends the bytes that `http` accepts raw, for example `'`, `` ` ``, `{`, `}`, `\`, and `&`.
- The test asserts the askama escape in `next`, for example `&#39;` and `&amp;`. No `hx-*` attribute may hold the payload.
- The rule `<svg` of `FORBIDDEN_MARKUP` becomes this rule: each `<svg` opening tag of a page equals the fixed tag of one icon. The change goes into the first task that renders an icon on a page (F17).
- The corpus gets a line that starts with the icon prefix and holds `/onbegin=`. `assert_clean_html` also finds `on…=` after `/`, `"`, and `'`, and it rejects a `<style` element.
- A template test rejects `{% filter safe %}`, an `escape` or `e` filter with an argument, and `|safe` on a variable. `tests/architecture.rs` rejects `escape =`, `ext =`, `Safe(`, `HtmlSafe`, and `MaybeSafe` in `recipe-web/src`.
- No `hx-*` attribute in a component takes a value from a variable. `hx-swap-oob` values are literals.
- Each of the 12 static files answers with `X-Content-Type-Options: nosniff` and its media type. Two test files check the 12 files:
  - `src/static_files/tests.rs` checks the tables `STATIC_FILES` and `FONTS`, the media types, and the SHA-256 constants.
  - `tests/web/static_files.rs` requests each file without a session and checks the status, the headers, and the body.

### D11. Known changes of test expectations

The implementer reports each change with this reason. Any other change of an expectation needs its own reason in the report.

| Test | Old expectation | New expectation | Reason |
|---|---|---|---|
| `xss_tests::FORBIDDEN_MARKUP` | no `<svg` | each `<svg` equals the fixed tag of an icon | ADR-0036 (f), spec "Escaped user text" |
| `error::tests::error_mapping_page_has_no_inline_script_style_or_handler` | one `<script` | three `<script src="/static/…">` | spec "No inline script or style" |
| Tests in `error/tests.rs` that read `HX-Reswap` | `innerHTML` | `innerHTML show:#error-panel:top` | spec "HTMX error headers", ADR-0036 (p) |
| Tests that compare an exact fragment of `base.html`, `error.html`, or `field-errors.html` | the old markup | the markup of the layout and of the macro `alert` | spec "Page layout", "Field error lines" |
| Tests that read the field name in a field error line | the name, for example `current_password` | the label of the bundle | spec "Field error lines" |
| Tests that look for the logout form or the result in the page body | the form in `main`, the message in `#user-result` | the form in the top bar, the message in `#toast` | D5, D6 |
| `pages::texts::tests::template_keys` | flat scan of `templates/` | recursive scan | `templates/components/` (F17) |
| `routes::tests::matrix()` | the routes of P2.5 | also `GET /recipes` | D9 |
| `routes::tests::expected_routes` | the static row of `/static/htmx.min.js` | also the rows of `app.css`, of the two scripts, and of the 8 font files | D3 |
| `error::tests::error_mapping_forbidden_is_403_and_names_the_operation` | `the operation CreateMachine` | `the operation Create machine` | the operation name comes from the bundle (D13) |
| `error::tests::error_mapping_text_follows_the_language_cookie` | `CreateMachine` on the Polish page | `Utworzenie maszyny` | the operation name comes from the bundle (D13) |
| `tests/web/lang.rs::lang_unknown_language_shows_the_field_error_fragment` | `Field lang is not valid` | `Field Language is not valid` | the sentence of `E-WEB-008` names the label (D6) |
| `pages::users::tests::pages_users_a_login_that_is_not_utf_8_gives_404_e_web_005_on_all_six_routes` | `HX-Reswap: innerHTML` | `HX-Reswap: innerHTML show:#error-panel:top` | ADR-0036 (p) |
| The four page tests `pages_own_login_form_is_polish_by_default_and_english_with_the_cookie`, `pages_users_list_shows_the_users_with_a_search_form`, `pages_users_detail_shows_the_user_the_throttle_state_and_five_forms`, and `pages_users_pages_follow_the_language_cookie` | `<title>` holds the page title only | `<title>` ends with ` · ` and the text of `app-name` | spec "Page layout" |
| Four assertions of `error/fragment_tests.rs` in the tests of a field line with a link | `<a href="#…">` | `<a class="target-link" href="#…">` | code review R1 |

### D12. Signatures

```rust
// recipe-app
pub struct RecipeList;                               // P3.3 adds the rows
#[derive(Clone)] pub struct ListRecipes { ctx: AppContext }
impl UseCase for ListRecipes { const OPERATION: Operation = Operation::ListRecipes; }
impl ListRecipes {
    pub const fn new(ctx: AppContext) -> Self;
    pub async fn execute(&self, user: &CurrentUser, meta: &RequestMeta)
        -> Result<RecipeList, AppError>;              // authorize first
}

// recipe-web
pub(crate) enum NavTarget { Recipes, Users, Me }       // path, text key, Operation
pub(crate) struct NavLink { path: &'static str, text: String, current: bool }
pub(crate) struct Layout<'a> { /* D5 */ }
impl<'a> Layout<'a> {
    pub(crate) fn for_session(texts: Texts<'a>, request: &PageRequest, title: String,
        current: Option<NavTarget>) -> Result<Self, WebError>;  // E-WEB-004 without a session
    pub(crate) fn for_login(texts: Texts<'a>, lang: Lang, title: String, nonce: &str) -> Self;
    pub(crate) const fn bare(texts: Texts<'a>, lang: Lang, title: String) -> Self;
    #[must_use] pub(crate) fn with_toast(self, text: String) -> Self;
}
impl PageRequest { pub(crate) fn path_and_query(&self) -> Option<&str>; } // GET only
pub(crate) struct StaticFile { path: &'static str, bytes: &'static [u8], media_type: &'static str }
pub(crate) const STATIC_FILES: [StaticFile; 4];      // htmx.min.js, app.css, row-version.js, tree.js
pub(crate) struct FontFile { path: &'static str, bytes: &'static [u8] } // hashes live in the tests
pub(crate) const FONTS: [FontFile; 8];
pub enum FieldReason { Form, Lang, LoginLength, LoginControl, LoginGrammar, PasswordLength,
    Role, Query, Q, Page, Forwarded }                // D13
impl FieldReason {
    pub const ALL: [Self; 11];
    pub const fn key(self) -> &'static str;           // reason-<name>
}
```

### D13. Values in error messages (code review R2, ADR-0012)

Every value that an error message shows as text comes from the bundles. Only data stays data: a login, a code, a path, a reference, a number, and a request id.

- `WebError::BadRequest` takes a `FieldReason` of `recipe-web` in place of a `&'static str`. Each variant has its key `reason-<name>`. Its `Display` keeps the English text for the log line.
- `answer.rs` maps each variant of `Missing`, `PasswordReason`, and `TaskStage` of `recipe-app` and `recipe-domain` to its key. The data of a variant goes into the arguments of the message. A number goes in as text, so `fluent` adds no group separator.
- `E-WEB-002` names the operation with the key `operation-` and the name of `Operation::name()` in kebab case, for example `operation-create-machine`.
- A test walks every variant of each enum and every operation of the catalog. Each key exists in both bundles.

Field reasons:

| Key | en | pl |
|---|---|---|
| `reason-form` | the form has a missing field, an unknown field, or a wrong content type | w formularzu brakuje pola, jest nieznane pole albo zły typ treści |
| `reason-lang` | the language must be pl or en | język musi być pl albo en |
| `reason-login-length` | a login has 1 to 64 characters | login ma od 1 do 64 znaków |
| `reason-login-control` | a login holds no control character | login nie zawiera znaku sterującego |
| `reason-login-grammar` | a login is 3 to 64 characters from a to z, 0 to 9, period, underscore, and hyphen | login ma od 3 do 64 znaków: litery a–z, cyfry 0–9, kropka, podkreślnik i łącznik |
| `reason-password-length` | a password has 1 to 128 characters | hasło ma od 1 do 128 znaków |
| `reason-role` | choose a role from the list | wybierz rolę z listy |
| `reason-query` | the address has an unknown parameter or a parameter that is not text | adres ma nieznany parametr albo parametr, który nie jest tekstem |
| `reason-q` | the search text has at most 64 characters from a to z, 0 to 9, period, underscore, and hyphen | tekst wyszukiwania ma najwyżej 64 znaki: litery a–z, cyfry 0–9, kropka, podkreślnik i łącznik |
| `reason-page` | the page is a whole number from 1 | numer strony to liczba całkowita od 1 |
| `reason-forwarded` | every entry must be an IP address | każdy wpis musi być adresem IP |

Missing entities of `E-WEB-005`:

| Key | en | pl |
|---|---|---|
| `missing-machine` | Machine { $code } | Maszyna { $code } |
| `missing-machine-id` | Machine { $id } | Maszyna { $id } |
| `missing-unit` | Unit { $path } of machine { $code } | Jednostka { $path } maszyny { $code } |
| `missing-type` | Type { $name } of machine { $code } | Typ { $name } maszyny { $code } |
| `missing-recipe` | Recipe { $reference } | Receptura { $reference } |
| `missing-version` | Version { $number } of recipe { $reference } | Wersja { $number } receptury { $reference } |
| `missing-user` | User { $login } | Użytkownik { $login } |
| `missing-malformed-login` | User | Użytkownik |

Password reasons of `E-IDN-006` and stages of `E-WEB-011`:

| Key | en | pl |
|---|---|---|
| `password-reason-empty` | it is empty | jest puste |
| `password-reason-too-short` | it has fewer than 12 characters | ma mniej niż 12 znaków |
| `password-reason-too-long` | it has more than 128 characters | ma więcej niż 128 znaków |
| `password-reason-breached` | it is in the breached-password list | jest na liście haseł, które wyciekły |
| `task-stage-password-hash` | password hash | obliczenie skrótu hasła |
| `task-stage-validation` | validation | walidacja |
| `task-stage-export` | export | eksport |
| `task-stage-token` | token generation | generowanie tokenu |

Operations of `E-WEB-002` (key `operation-` and the kebab name):

| Name | en | pl |
|---|---|---|
| `CreateMachine` | Create machine | Utworzenie maszyny |
| `RenameMachine` | Rename machine | Zmiana nazwy maszyny |
| `ArchiveMachine` | Archive machine | Archiwizacja maszyny |
| `AddUnit` | Add unit | Dodanie jednostki |
| `RenameUnit` | Rename unit | Zmiana nazwy jednostki |
| `ArchiveUnit` | Archive unit | Archiwizacja jednostki |
| `DefineUdt` | Define UDT | Zdefiniowanie typu UDT |
| `CreateRecipe` | Create recipe | Utworzenie receptury |
| `RenameRecipe` | Rename recipe | Zmiana nazwy receptury |
| `DeleteRecipe` | Delete recipe | Usunięcie receptury |
| `RestoreRecipe` | Restore recipe | Przywrócenie receptury |
| `CreateVersion` | Create version | Utworzenie wersji |
| `DiscardDraft` | Discard draft | Odrzucenie szkicu |
| `ApproveVersion` | Approve version | Zatwierdzenie wersji |
| `RetireVersion` | Retire version | Wycofanie wersji |
| `AddParameter` | Add parameter | Dodanie parametru |
| `UpdateParameterDefinition` | Change parameter definition | Zmiana definicji parametru |
| `SetParameterValue` | Set parameter value | Ustawienie wartości parametru |
| `ReplaceLimits` | Replace limits | Zastąpienie limitów |
| `UpsertTranslation` | Save translation | Zapisanie tłumaczenia |
| `RemoveTranslation` | Delete translation | Usunięcie tłumaczenia |
| `RemoveParameter` | Delete parameter | Usunięcie parametru |
| `RebindTypeRevision` | Rebind type revision | Przepięcie rewizji typu |
| `Login` | Log in | Logowanie |
| `Logout` | Log out | Wylogowanie |
| `CreateUser` | Create user | Utworzenie użytkownika |
| `SetUserRole` | Set user role | Ustawienie roli użytkownika |
| `DeactivateUser` | Deactivate user | Dezaktywacja użytkownika |
| `ActivateUser` | Activate user | Aktywacja użytkownika |
| `ResetPassword` | Reset password | Reset hasła |
| `ChangeOwnPassword` | Change own password | Zmiana własnego hasła |
| `UnlockLogin` | Unlock login | Odblokowanie logowania |
| `ExportVersion` | Export version | Eksport wersji |
| `ListMachines` | List machines | Lista maszyn |
| `GetMachine` | Show machine | Podgląd maszyny |
| `GetUnitTree` | Show unit tree | Podgląd drzewa jednostek |
| `GetUnit` | Show unit | Podgląd jednostki |
| `ListTypes` | List types | Lista typów |
| `GetTypeTree` | Show type tree | Podgląd drzewa typu |
| `GetTypeUsage` | Show type usage | Podgląd użycia typu |
| `ListRecipes` | List recipes | Lista receptur |
| `GetRecipe` | Show recipe | Podgląd receptury |
| `ListVersions` | List versions | Lista wersji |
| `GetVersion` | Show version | Podgląd wersji |
| `DiffVersions` | Compare versions | Porównanie wersji |
| `GetRecipeHistory` | Show recipe history | Podgląd historii receptury |
| `ListParameters` | List parameters | Lista parametrów |
| `GetParameter` | Show parameter | Podgląd parametru |
| `GetLimits` | Show limits | Podgląd limitów |
| `GetTranslations` | Show translations | Podgląd tłumaczeń |
| `ListUsers` | List users | Lista użytkowników |
| `GetUser` | Show user | Podgląd użytkownika |
| `ListAuditEntries` | List audit entries | Lista wpisów audytu |
| `GetAuditEntry` | Show audit entry | Podgląd wpisu audytu |
| `ListSecurityEvents` | List security events | Lista zdarzeń bezpieczeństwa |
| `GetOwnProfile` | Show own profile | Podgląd własnego profilu |
| `GetMachineForm` | Open machine form | Formularz maszyny |
| `GetUnitForm` | Open unit form | Formularz jednostki |
| `GetUdtForm` | Open UDT form | Formularz typu UDT |
| `GetRecipeForm` | Open recipe form | Formularz receptury |
| `GetVersionForm` | Open version form | Formularz wersji |
| `GetParameterForm` | Open parameter form | Formularz parametru |
| `GetApprovalPreview` | Show approval preview | Podgląd przed zatwierdzeniem |
| `GetUserForm` | Open user form | Formularz użytkownika |
| `GetLoginForm` | Open login form | Formularz logowania |
| `GetOwnPasswordForm` | Open password form | Formularz zmiany hasła |

## Risks / Trade-offs

- [The CSS parser of the tests reads only a strict subset of CSS] → `app.css` uses the flat format of D4. The test fails on a construct that it cannot read, so it never passes by accident.
- [No automated test runs the two scripts] → The static checks of D7 and a browser check in the code review cover them. The pages of P3.2 and P3.3 use them.
- [A test of `recipe-web` reads a file in `docs/`] → The SQL test of `data-model.md` section 5.4 does the same. A change of the document without `app.css` fails the build, which is the goal.
- [The full error page shows no navigation] → It shows the link `error-page-home` to `/recipes`. The login page catches a missing session.
- [The skeleton shows no recipe, also when the database holds one] → No route of P3.1 creates a recipe. P3.3 replaces the skeleton before a release.
- [Every page struct changes for the layout] → Task 4.1 moves every page struct in one step, so the crate compiles after each task. D11 lists the expected changes of old tests.
- [A long `next` from a crafted URL] → Only the user who types the URL gets the long field. `POST /lang` still applies the rule of ADR-0035 (o).
- [8 font files enlarge the binary by about 80 KB] → The SHA-256 test pins them. The license texts live next to them.

## Migration Plan

No data and no configuration change. A deployment replaces the binary. A rollback is a revert of the commits.

## Open Questions

None.
