# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

The recipe editor is a Rust web application that edits and versions the recipes of the machines of one production line.
It stores typed parameters with process limits in PostgreSQL, records every change, and exports an approved version as hashed JSON.
It runs 24/7 on one on-premises Docker host, never communicates with a PLC, and gives data safety priority over performance.

Start with [docs/architecture/overview.md](docs/architecture/overview.md).
The build order is in [docs/plan/implementation-prompts.md](docs/plan/implementation-prompts.md), one prompt per step.

## Language rules

- Chat with the user in Polish.
- Write every artifact in English: code, identifiers, comments, docs, specs, and commit messages.
- The UI has a Polish `pl` bundle (default) and an English `en` bundle (ADR-0012).
- All English text follows the ASD-STE100 structural rules (R40). Load the skill `asd-ste100` before you write prose.
- A sentence has at most 25 words. Use the active voice. Do not use semicolons. Use one verb for one action in one file.
- `tools/ste-lint/ste-lint.py` must report 0 hard violations on the scope of ADR-0023. Passive voice and compound tenses are advisory.
- Exception: `docs/plan/implementation-prompts.md` is an operator file in Polish.

## Source of truth and change rule

- `docs/architecture/*.md` and `docs/adr/*.md` are normative. Code, specs, and tests follow them.
- A new design decision is a new ADR. Change `docs/architecture/` only in the same change as that ADR.
- An accepted ADR changes only by its Status line and an "Amended by" note. A full replacement is a new ADR.
- The decision map of `docs/adr/README.md` names the current ADR of each topic. Do not apply a superseded ADR.
- `general-draft.txt` holds the requirements. `overview.md` §7 maps them to R01 to R45. §9 lists the Assumptions A1 to A43.
- `docs/plan/review/` holds the review history. It is not normative.
- When two documents disagree, or a request disagrees with a document, stop and ask the user.

## Workspace map

Six crates in `crates/` (`overview.md` §5). Every crate has `#![forbid(unsafe_code)]` and `[lints] workspace = true`.

| Crate | Kind | Role | Error type |
|---|---|---|---|
| `recipe-domain` | lib | Entities, value objects, invariants, `Permission`, error codes, module `export` (hashed body, content hash). No I/O, no port trait | `PlantError`, `RecipeError`, `IdentityError`, `ExportError` |
| `recipe-app` | lib | Use cases, ports (traits), permission checks, audit entries | `AppError`, `AuditError`, `StorageError` |
| `recipe-db` | lib | PostgreSQL adapters (`sqlx`), session store, unit of work | None public |
| `recipe-export` | lib | Export JSON Schema, JSON writer of the HTTP response, golden tests | None. It returns `ExportError` |
| `recipe-web` | lib | axum routers, askama templates, HTMX fragments, i18n, security headers, session cookie | `WebError` |
| `recipe-server` | bin + lib | Configuration, wiring, subcommands `serve`, `migrate`, `create-admin`, `healthcheck` | `anyhow::Error` |

Dependency rule:

- Among the workspace crates, `recipe-app` depends on `recipe-domain` only. It never uses `sqlx` or `axum`.
- `recipe-db` implements the ports of `recipe-app`. It has no public error type. It maps `sqlx::Error` by SQLSTATE into `StorageError`.
- Only `recipe-web` uses `recipe-export`. `recipe-server` does not depend on it.
- `recipe-web` uses no `sqlx` and no session library. The `session` table and its adapter belong to the project (ADR-0014).
- Other paths: `migrations/` (forward-only SQL), `deploy/` (Compose, Caddy, `db/initdb/01-roles.sh`), `openspec/` (specs and changes).

## Hard rules

- No panic paths. Outside tests the lints deny `unwrap`, `expect`, `panic!`, indexing, lossy casts, and unchecked arithmetic (`reliability.md` §2).
- Library crates use `thiserror`. Only `recipe-server` uses `anyhow`, and `main` returns `anyhow::Result<()>` (`reliability.md` §1.1, A37).
- A command runs in one transaction. `Login` and `ChangeOwnPassword` use the two transactions T1 and T2 (ADR-0035 (b)). A command locks rows in the order `machine` → `recipe` → `recipe_version` → `unit` → children (ADR-0015).
- Inside a transaction, no use case waits for I/O or for a lock that it does not hold. Validation runs before it (`reliability.md` §7.4).
- A command writes its audit entries through `UnitOfWork::commit(NonEmpty<AuditEntry>)`. `domain-model.md` §5 gives each commit path and the two cases without an entry.
- Security events go through `SecurityEventSink` in their own short transaction (ADR-0015, ADR-0022).
- The permission check is the first statement of every command use case and every query use case. HTTP only authenticates (ADR-0013).
- In read-only mode, every command except `Login`, `Logout`, and `ExportVersion` returns `503 E-WEB-006` and writes nothing (A24).
- Every error code has exactly one row in `reliability.md` §1.2. Add the row before the variant that returns the code. Never reuse a code.
- Every error type has `code()`. An error variant never carries a password, a hash, a session token, or a cookie value (`reliability.md` §1.1).
- Log each error once: in the `WebError` to HTTP mapping, in the backstop panic layer, or in the supervisor of a background job. A job logs a handled fault once at `WARN` (`reliability.md` §1.1).
- Every numeric setting comes from `reliability.md` §7.11. A value repeated elsewhere must equal that table.
- A value object has one `parse` constructor and no public `new`. Code does not check it again after construction (`reliability.md` §4.3).
- This applies to `UnitPath`, `S7Address`, `TypeExpr`, `IecLiteral`, `LeafPath`, and every other value object.
- A numeric, duration, or instant leaf in JSONB is canonical decimal text. A time leaf is an integer count of its base unit (`reliability.md` §4.4, A43).

## Workflow

Each OpenSpec change follows this cycle. The names in backticks are skills.

1. Propose with `openspec-propose` (Opus).
2. Review the plan with `plan-eng-review` (Opus). Apply the findings with `openspec-update-change`.
3. Apply with `openspec-apply-change` and `superpowers:test-driven-development` (Sonnet, Haiku for mechanical tasks). Write the failing test first.
4. Review the code with `code-review` (Opus). Fix every CRITICAL and HIGH finding.
5. Run `superpowers:verification-before-completion`. Show the passing clippy, test, and fmt output before you report success.
6. Archive with `openspec-archive-change` after `cargo test` and `openspec validate` pass.

- Do not start an implementation before the user approves `tasks.md` of the change.
- Name the model in every `Agent()` call: Haiku for mechanical work, Sonnet for implementation, Opus for design and review.
- A PreToolUse hook runs `cargo test --workspace` before `openspec archive`. A failed test run blocks the archive.
- Commit messages follow Conventional Commits in English, for example `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`.

## Commands

```bash
cargo clippy --all-targets --all-features -- -D warnings  # lint gate of CI
cargo test                                                 # every test of the workspace
cargo test -p recipe-domain unit_path                      # one crate, tests whose name contains "unit_path"
cargo fmt --check
cargo deny check                                           # licenses, advisories, yanked crates
cargo llvm-cov --workspace                                 # coverage, CI target 80 % lines per crate
docker compose -f deploy/compose.yaml up -d                # full stack, start order in reliability.md 7.1
openspec validate --all
openspec list
bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'  # scope of ADR-0023
```

- The `recipe-db` tests need Docker for `testcontainers`, or `DATABASE_URL` of a Compose database (`reliability.md` §6).
- Run `ste-lint.py` from Bash. The command sets `globstar` and `nullglob` like the CI job. Without `globstar`, Bash expands `**` to `*`.

## Where to look

| Question | Document and section |
|---|---|
| Which code does this error get? | `reliability.md` §1.2 |
| Which error type owns this variant? | `reliability.md` §1.1 |
| Which lock comes first? | `data-model.md` §6 intro, ADR-0015 |
| Which advisory lock class and key? | `data-model.md` §1.4 |
| What is in the export hash? | `api.md` §6 |
| Is this an Assumption? | `overview.md` §9 |
| Which requirement does this satisfy? | `overview.md` §7 |
| Which trigger enforces I-n? | `domain-model.md` §3 |
| Which errors and audit entries does a command have? | `domain-model.md` §5, `data-model.md` §6 |
| Which permission does a use case need? | `api.md` §3, `domain-model.md` §5.5 |
| Which value has this timeout, limit, or budget? | `reliability.md` §7.11 |
| What is the grammar of a value object? | `domain-model.md` §2, `reliability.md` §4.3 |
| What is the SQL name of a Rust field? | `data-model.md` §1.2 |
| Which grants does `recipe_app` have? | `data-model.md` §5.2 |
| Which status and fragment does an error get? | `api.md` §5 |
| How does the `row_version` protocol work? | `api.md` §4, ADR-0008 |
| What do the CSP and the HTMX configuration permit? | `api.md` §9 |
| Which lints and panic exceptions apply? | `reliability.md` §2 |
| Which tests does each level need? | `reliability.md` §6 |
| Which ADR is current for a topic? | `docs/adr/README.md`, decision map |
| What does a domain term mean? | `domain-model.md` §7 |
