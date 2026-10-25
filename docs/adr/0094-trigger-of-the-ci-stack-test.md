# ADR-0094: Trigger of the CI stack test

Status: Accepted
Date: 2026-10-25
Deciders: project owner

## Context

ADR-0070 (w) says that the CI job `compose` runs on every push and every pull request. The job is the stack test of `reliability.md` section 6.

Step P4.1 wrote the job in `.github/workflows/ci.yml`. The job has these facts:

- The job builds two images, `app` and `pgtools`.
- The job starts the full stack and runs the backup, the restore test, and the recovery check.
- The job has `timeout-minutes: 45` and can use all of it.

The trigger `push` of `ci.yml` lists the branch `main` only. The workflow also has the trigger `workflow_dispatch`. The text of ADR-0070 (w), `reliability.md` section 6, and the spec "Stack test in CI" therefore disagree with the workflow.

## Decision drivers

- A run takes up to 45 minutes. A run on every push costs more time than it gives value.
- `main` must always have a complete stack test (R04, R30, R31).
- A developer must be able to test a branch before a merge.

## Considered options

### Option A: Run the job on every push

- Pros: each commit gets a stack result.
- Cons: each push builds two images and takes up to 45 minutes. Most pushes change no deployment file.

### Option B: Run the job on `workflow_dispatch` and on a `push` to `main`

- Pros: `main` gets a full stack test after each merge. A developer starts a run on any branch when the change needs it. The cost stays low.
- Cons: a branch that nobody dispatches gets no stack result before the merge.

The project owner chose **Option B** on 2026-10-25.

## Decision

- (a) The job `compose` runs on `workflow_dispatch` and on a `push` to the branch `main`.
- (b) The job does not run on a push to another branch.
- (c) The reason is cost. The job builds two images and takes up to 45 minutes.
- (d) A developer starts the job by hand with `workflow_dispatch` on the branch before a merge. The merge needs a green run of that branch when the change touches `deploy/`, `migrations/`, or the images.
- (e) ADR-0070 (w) is amended for its trigger rule. The other facts of (w) stay: `ubuntu-24.04`, `workflow_dispatch`, and `timeout-minutes: 45`.

## Consequences

### Positive

- A push to a work branch does not use 45 minutes of runner time.
- `main` keeps a full stack test after each merge.
- The text of the documents agrees with the triggers of `ci.yml`.

### Negative

- A developer must remember to dispatch the job before a merge.
- A defect in the stack can reach `main` before the first run on `main` finds it.

### Risks and mitigations

- Risk: a merge skips the dispatch run. Mitigation: item (d) names the merge condition, and the code review checks that a green run exists.
- Risk: the run on `main` fails after a merge. Mitigation: a failed run on `main` blocks the next merge until a fix gives a green run.

## Related

- ADR-0070 (w). This ADR amends it.
- `docs/architecture/reliability.md` section 6, row Stack.
- `openspec/changes/operations-compose/design.md` D11 and `specs/operations/spec.md`, requirement "Stack test in CI".
- `.github/workflows/ci.yml` (`on`, job `compose`).
- `overview.md` section 7: R04, R30, R31.
