# AGENTS.md

This file gives Codex the instructions of this repository. Claude Code reads `CLAUDE.md`. Both agents follow the same sources.

## Read first

1. `CLAUDE.md`: the purpose, the workspace map, the dependency rule, the hard rules, and the commands.
2. `.claude/rules/rust.md`, `.claude/rules/web.md`, `.claude/rules/sql.md`, `.claude/rules/docs.md`, and `.claude/rules/openspec.md`.
3. The `design.md` of the active change in `openspec/changes/`, when the diff belongs to one.

The documents in `docs/architecture/` and `docs/adr/` are normative. Code, specs, and tests follow them. When two documents disagree, report the conflict as a finding.

## Role of Codex

- Codex reviews. The plugin ccx-loop calls the review through the Codex plugin of Claude Code.
- A Claude agent implements and fixes. Each agent owns one task and a fixed file set.
- During a review, change no file. Make no commit, no branch, no stash, and no tag.
- Run no cargo command. The review sandbox cannot write `target/`, and the agents run the tests.

## Scope of a review

- Review the diff of the tracked files: `crates/`, `migrations/`, `deploy/`, `docs/`, `openspec/`, `tools/`, `.github/`, `e2e/`, and the documents in the root.
- Do not review the untracked tool files: `.agents/`, `.claude/agents/`, `.claude/hooks/`, `.claude/skills/`, `.codex/`, and `skills-lock.json`.
- Git ignores `target/`. Do not review it.

## Severity of a finding

Use the four levels of ccx-loop.

| Level | Meaning |
|---|---|
| critical | A security fault, a loss of data, a broken audit hash chain, or a leaked secret. Also a command that writes in more than one transaction, except `Login` and `ChangeOwnPassword` (ADR-0035 (b)) |
| high | A bug, a violation of a hard rule of `CLAUDE.md` or `.claude/rules/`, a missing test of a required state, or a lint that CI denies |
| medium | A deviation from a design decision, a weak test that a plausible bug passes, or a maintainability fault |
| low | A naming, style, or wording fault, or a small suggestion |

Give each finding the file and the line, the fault, the evidence, and a proposed fix.

## Checks with the highest value

- No panic path outside tests: no `unwrap`, `expect`, `panic!`, indexing, `as` between integer types, or unchecked arithmetic (`reliability.md` section 2).
- The permission check is the first statement of every command use case and every query use case (ADR-0013).
- A command runs in one transaction and writes its audit entries through `UnitOfWork::commit` (ADR-0015). `Login` and `ChangeOwnPassword` use two transactions (ADR-0035 (b)).
- Every error code has one row in `reliability.md` section 1.2. An error variant never carries a password, a hash, a token, or a cookie value.
- Templates have no inline script, no inline style, and no inline event handler. Each `hx-*` attribute holds a literal, except `hx-headers` on `<body>` in `base.html`, which holds the CSRF token. No URL holds the CSRF token. No `|safe` applies to a value that is not a literal.
- Each mutating route checks CSRF and puts the error code into the fragment. Each mutating request on a draft also carries `row_version` (`.claude/rules/web.md`).
- A value, a limit, or a number in a diff or an audit view is canonical decimal text (A43).
- A migration that adds or changes a generated column or a column default calls only built-in functions of `pg_catalog` in that expression (ADR-0084 (d)).
- A new role statement in `deploy/db/initdb/01-roles.sh` has a form of the role grammar in `deploy/pgtools/lib/globals-grammar.sh` (ADR-0095).
- A function has at most 50 lines, and a file has at most 800 lines. A violation of these two limits is a medium finding.
- Each task starts with a failing test. A test of a required state must fail on a plausible bug.
- English text follows the ASD-STE100 rules: at most 25 words in a sentence, the active voice, and no semicolons.

## Skills

The skills in `.agents/skills/` help a review. These files and `target/tools/` exist only in the local workspace. Skip a skill or a file that is missing. Read the `SKILL.md` that fits the diff. Skip every init, setup, or onboarding flow of a skill.

- Rust: `idiomatic-rust`, `rust-errors`, `async-rust`, `rust-concurrency`, `rust-testing`, `rust-code-review`.
- Web (`crates/recipe-web`): `askama`, `interaction-design`, `axum-errors/axum-errors-extractor-rejections`, `axum-impl/axum-impl-testing`, `accessibility-audit`, `web-design-guidelines`.
- The skill `web-design-guidelines` names a URL. Do not fetch it. Read `target/tools/web-interface-guidelines.md`.
- SQL (`crates/recipe-db`, `migrations/`): `sql-optimization-patterns`, `postgresql-table-design`.
- Texts and documents: `ux-writing`, `asd-ste100`.
