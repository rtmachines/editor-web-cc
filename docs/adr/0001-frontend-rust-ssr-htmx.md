# ADR-0001: Frontend with Rust server-side rendering and HTMX

Status: Accepted. Amended by ADR-0015 (row-version.js)
Date: 2026-10-12
Deciders: project owner

## Context

The draft requires a "web based application in Rust". It does not name a frontend technology. The application is a form-and-table editor. The most complex screen is the editor for a parameter whose type is a nested UDT or a multidimensional array.

The application must run 24/7 for many years in an industrial plant. The team that maintains it is a Rust team. A second toolchain (Node, npm, bundler) adds a maintenance cost that the draft does not ask for.

## Decision drivers

- One language for the whole code base (maintenance, R34).
- No client state that can diverge from the server state (R39, data safety R31).
- A clear and reliable UI with a visible result for every action (R39).
- The nested UDT editor must be possible without a large client framework.
- Long-term stability of the dependency set.

## Considered options

### Option A: Rust SSR with axum, askama, and HTMX

- Pros: one language, no npm toolchain, server holds all state, templates are checked at compile time, HTML fragments map well to forms and tables.
- Cons: a highly interactive editor needs more round trips, tree expand/collapse needs a few lines of JavaScript.

### Option B: Rust WASM frontend (Leptos or Dioxus) with SSR and hydration

- Pros: one language, rich client interactivity, shared types between client and server.
- Cons: larger and less stable ecosystem, two build targets, hydration bugs are hard to debug, bigger binary and longer builds.

### Option C: JavaScript SPA (React or Svelte) with a Rust REST API

- Pros: largest UI ecosystem, best tooling for complex client-side editors.
- Cons: two languages and two toolchains. Client state must be kept in sync with the server. More code for the same screens. A JSON API that must be versioned for the UI alone.

## Decision

We use Option A: axum for HTTP, askama for templates, and HTMX for partial page updates.

Rules:

- `recipe-web` renders full pages and HTML fragments. It does not render JSON for the UI.
- The nested UDT and array editor is built from HTML fragments. Each struct member or array row is one fragment. HTMX loads and swaps fragments.
- JavaScript is allowed only for tree expand/collapse, for focus management, and for `row-version.js`. `row-version.js` copies the new `row_version` from the response header into the hidden form fields (amended by ADR-0015). No script holds application state.
- HTMX 2 is vendored as a pinned file in the repository. The application does not load scripts from a CDN.
- Every mutating form carries the `row_version` of the draft (ADR-0008).

## Consequences

### Positive

- One toolchain: `cargo build` produces the whole application.
- The server is the single source of truth. A page refresh always shows the current state.
- Compile-time template checks catch missing fields before deployment.
- The dependency set is small and stable.

### Negative

- Editing a large UDT needs more HTTP round trips than a client-side editor.
- Some UI behaviour (expand/collapse, keyboard shortcuts) needs hand-written JavaScript.
- Developers must learn the HTMX fragment pattern.

### Risks and mitigations

- Risk: the UDT editor becomes too slow with many round trips. Mitigation: render a whole struct subtree in one fragment. Measure on a 4 000-parameter version (Assumptions A19, A26).
- Risk: a future requirement needs rich client interactivity. Mitigation: HTMX fragments can coexist with an island built in Option B. A new ADR would record that change.

## Related

- ADR-0008 (optimistic concurrency, `row_version` in forms)
- ADR-0015 (amendment: `row-version.js`)
- ADR-0012 (UI language with fluent)
- `overview.md` section 5 (crate `recipe-web`), section 7 (R01, R39)
