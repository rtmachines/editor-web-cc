# ADR-0135: The STE lint scope includes the OpenSpec specs

Status: Accepted
Date: 2026-11-06
Deciders: project owner

## Context

ADR-0023 sets the CI scope of `tools/ste-lint/ste-lint.py`. The scope lists `docs/architecture/`, `docs/adr/`, `docs/ops/`, `docs/user/`, and three files in the repository root. The baseline is 0.

`.claude/rules/openspec.md` says that the specs follow the ASD-STE100 structural rules. No CI job checks `openspec/specs/`. The specs are normative text of the seven capabilities, and an agent or a reader parses them as the requirements.

The linter reports 18 violations in `openspec/specs/**/*.md`, in 74153 words. Three of them are hard violations of the rule `synonym-rotation`:

- `openspec/specs/export/spec.md` uses `correct` and `fix` in one file.
- `openspec/specs/identity-access/spec.md` uses `correct` and `fix` in one file.
- `openspec/specs/operations/spec.md` uses `removed` and `delete` in one file.

## Decision drivers

- One hard violation in a normative document fails the job (R40).
- The rule in `openspec.md` and the CI scope must be equal.
- An active change is a draft. The archive is history.
- The scope in the text and the scope in the command must be equal (ADR-0023).

## Considered options

### Option A: Add `openspec/specs/` to the scope

- Pros: the specs get a gate. The scope matches the rule in `openspec.md`. The three hard violations are small to fix.
- Cons: the CI job lints more text. Each new spec must pass the lint.

### Option B: Add all of `openspec/` to the scope

- Pros: one directory name in the command.
- Cons: `openspec/changes/` holds drafts and archived changes. A draft changes often. The archive is history and stays unchanged.

### Option C: Keep the scope of ADR-0023

- Pros: no change.
- Cons: the specs have no STE gate. The rule in `openspec.md` has no check.

## Decision

The project uses **Option A**.

- (a) CI lints every Markdown file in `openspec/specs/`, nested directories included, with baseline 0.
- (b) `openspec/changes/` stays out of scope. This covers active changes and the archive.
- (c) The step sets `globstar` and `nullglob` in Bash, as ADR-0023 requires.
- (d) The change that adopts this ADR fixes the three hard violations of the Context. It keeps every requirement and scenario unchanged.
- (e) This ADR amends the scope of ADR-0023. All other parts of ADR-0023 stay in force.

## Consequences

### Positive

- The specs have the same STE gate as the other normative documents.
- The rule in `openspec.md` has a check.

### Negative

- A spec author must pick one word for one action in each spec file.
- Each document that states the scope changes with this ADR. The list is the job `ste-lint` in `.github/workflows/ci.yml`, `CLAUDE.md`, `CONTRIBUTING.md`, `.claude/rules/docs.md`, `CHANGELOG.md`, `docs/architecture/overview.md` (R40), and `docs/architecture/reliability.md` section 8.
- The three specs `openspec/specs/export/spec.md`, `openspec/specs/identity-access/spec.md`, and `openspec/specs/operations/spec.md` change one phrase each, to remove the hard violations.

### Risks and mitigations

- Risk: a new spec file is outside the scope. Mitigation: the glob matches every Markdown file in `openspec/specs/`, so a new file enters the scope without a change to the step.
- Risk: the archive of a change updates a spec with a new violation. Mitigation: the job `ste-lint` fails on the pull request, and the author fixes the spec before the merge.

## Related

- ADR-0023 (scope and location of the STE lint)
- `.claude/rules/openspec.md` (rules for the specs)
- `.claude/rules/docs.md` (ASD-STE100 structural rules)
- `overview.md` §7: R40
