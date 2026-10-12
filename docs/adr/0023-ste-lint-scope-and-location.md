# ADR-0023: Scope and location of the STE lint in CI

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

`reliability.md` section 8 said that CI runs `ste-lint.py` on every Markdown file in `docs/`. It named the path `.claude/skills/asd-ste100/scripts/ste-lint.py`. Step P0.2 builds the CI workflow and found four problems:

- `docs/plan/review/` holds the review history. The history is not normative. The reports quote findings and old draft text. The linter finds 818 hard violations there.
- `docs/plan/implementation-prompts.md` is an operator file in Polish. The linter checks English rules only, so its result for a Polish file has no meaning.
- Bash without `globstar` expands `docs/**/*.md` to `docs/*/*.md`. The documented command therefore skips `docs/plan/review/`, and the command and the text give two different scopes.
- `.claude/skills/asd-ste100` is a symbolic link with an absolute path on the developer host. A skill manager creates it, and Git does not track it. A CI runner does not have the script.

## Decision drivers

- One hard violation in a normative or user-facing document fails the job (R40).
- The CI result must be deterministic. The job must not need network access to a third-party repository.
- The scope in the text and the scope in the command must be equal.
- The review history stays unchanged.

## Considered options

### Option A: Lint the normative and user-facing documents only

- Pros: 0 hard violations today. No change to the review history. The scope matches the normative documents of `docs.md`.
- Cons: `docs/plan/` has no STE gate.

### Option B: Move `docs/plan/review/` out of `docs/`

- Pros: the text "every Markdown file in `docs/`" stays true.
- Cons: `CLAUDE.md`, `.claude/rules/docs.md`, the operator prompts, and the review links change. CI still lints the Polish operator file as English.

### Option C: Lint all of `docs/` with `--baseline 818`

- Pros: no file moves.
- Cons: the baseline is one count for all files. A new review report fails the job or raises the baseline. A fix in the history hides a new violation in a normative document.

For the location of the script, the owner compared three options:

- a copy in the repository,
- a clone of the upstream repository at a pinned commit in CI,
- a commit of `.agents/skills/asd-ste100`.

## Decision

The project uses **Option A** and a copy of the script in the repository.

Scope. CI lints these files with baseline 0:

- every Markdown file in `docs/architecture/`, `docs/adr/`, `docs/ops/`, and `docs/user/`, nested directories included,
- `README.md`, `CONTRIBUTING.md`, and `CHANGELOG.md` in the repository root.

`docs/plan/` is out of scope. It holds the operator prompts in Polish and the review history. Both are not normative (`docs.md`).

Command. The CI step sets `globstar` and `nullglob` in Bash, so `**` includes nested directories and an empty directory adds no argument.

Location. CI runs `tools/ste-lint/ste-lint.py`. This file is a copy of the script of the skill `asd-ste100` (upstream `danyuchn/asd-ste100-skill`, MIT license). `tools/ste-lint/LICENSE` holds the license text. `tools/ste-lint/README.md` records the upstream commit and the SHA-256 of the copy. Local runs use the same copy, so the local result and the CI result are equal. A new upstream version enters the repository in its own pull request.

## Consequences

### Positive

- CI does not need the skill installation or network access.
- The scope is the same in this ADR, in `reliability.md` section 8, and in the CI step.
- The review history keeps its original text.

### Negative

- The repository holds a third-party script. Its updates are a manual step.
- `docs/plan/` can contain STE violations.

### Risks and mitigations

- Risk: the copy and the skill drift apart. Mitigation: CI and local commands use only the copy. The skill gives writing guidance, and the copy is the gate.
- Risk: a new normative directory under `docs/` is outside the scope. Mitigation: the ADR that adds the directory also adds it to the scope of this ADR and to the CI step.

## Related

- `reliability.md` section 8 (code conventions, lint gate)
- `overview.md` section 7 (R40, R42)
- `.claude/rules/docs.md` (Diátaxis layout of `docs/`)
- `.github/workflows/ci.yml` (job `ste-lint`)
