# OpenSpec rules

Source: `docs/plan/implementation-prompts.md` (P0.1, P0.3, P1.1) and `openspec/config.yaml` (language). Load the matching `openspec-*` skill for each step.

## Capabilities

The project uses seven capabilities. Each one is a directory in `openspec/specs/`.

| Capability | Scope |
|---|---|
| `plant-model` | Machines, units, PLC platforms, type catalog, UDT revisions, derived types |
| `recipe-management` | Recipes, drafts, versions, parameters, limits, translations, approval, rebind |
| `identity-access` | Users, roles, sessions, login throttle, passwords, permission matrix |
| `audit` | Audit log, hash chain, security events |
| `export` | Export document v1, content hash, export routes |
| `web-ui` | Pages, fragments, HTMX behavior, i18n, error display |
| `operations` | Compose, migrations, backup, health, settings, supervised jobs |

Do not add an eighth capability without the approval of the user.

## Requirements

- A requirement has the heading `### Requirement: <name>` and at least one `#### Scenario:`.
- The requirement sentence uses SHALL or MUST. Keep SHALL, MUST, and the OpenSpec headings in English.
- Every requirement cites an R-number of `overview.md` §7 or an A-number of `overview.md` §9, for example `(R18)`.
- A scenario uses Given, When, Then. Derive it from an invariant of `domain-model.md` §3 or a row of `data-model.md` §6.
- Use the terms of `domain-model.md` §7 and the permission names of `api.md` §3.
- Specs follow the ASD-STE100 structural rules of `docs.md`.
- A spec states behavior. `design.md` links to `docs/architecture/` and `docs/adr/` and does not repeat them.
- A new decision inside a change is a new ADR in the same change.

## Changes

- One prompt of `docs/plan/implementation-prompts.md` makes at most one change. Use the change name that the prompt gives.
- Phase 0 and P1.0 make no change. P4.2 makes a change only when a fix changes a spec.
- Do not write code before the user approves `tasks.md` of the change.
- Each task starts with a failing test and names its crate. Mark a mechanical task `[mechanical]` for a Haiku subagent.
- The cycle: `openspec-propose`, `plan-eng-review`, `openspec-apply-change`, `code-review`, `superpowers:verification-before-completion`, `openspec-archive-change`.
- Archive only after `cargo test` and `openspec validate` pass.
- A PreToolUse hook runs `cargo test --workspace` before `openspec archive`. A failed test run blocks the archive.

## Commands

```bash
openspec list                                  # active changes, --specs for specs
openspec status --change <name>                # artifact status of one change
openspec instructions <artifact> --change <name>
openspec show <name>
openspec validate <name>                       # one change or spec
openspec validate --all                        # every change and spec
```
