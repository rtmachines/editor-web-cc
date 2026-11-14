# ADR-0164: Second pick of the approved version

Status: Accepted. Amends ADR-0019 (the answer `404 E-EXP-001` of the route `/approved`) and ADR-0109 (ah)
Date: 2026-11-14
Deciders: project owner

## Context

The export flow is in `crates/recipe-app/src/export/flow.rs`. Step 2 (`find`) reads the recipe in a read-only transaction and picks the version. Step 3 (`lock_and_check`) locks that version `FOR SHARE` and checks its state again.

`ApproveVersion` can commit between step 2 and step 3. The approval then retires the picked version (R27). On the route `/approved`, `check_exportable` maps every state other than `Approved` to `NoApprovedVersion`. The flow does not pick again.

An example follows. Version 3 is approved and version 4 is a draft. Step 2 picks version 3. An approval of version 4 commits and retires version 3. Step 3 finds version 3 retired. The client gets `404 E-EXP-001`, although version 4 is approved.

ADR-0019 and the row `ExportVersion` of `data-model.md` §6 give `E-EXP-001` only when no version is approved. The polling rule of `api.md` §2.7 and ADR-0109 (ah) expect approvals while a client polls. The Basic route and the v2 unit route share the flow.

The full repository review of 2026-11-13 reports this defect as finding F-02. The owner chose a second pick (`docs/plan/review/2026-11-13-fix-plan.md` §1).

## Decision drivers

- `404 E-EXP-001` must mean that the recipe has no approved version (ADR-0019).
- The flow must end after a fixed count of picks.
- The lock order of ADR-0015 and the permit rules of ADR-0056 must hold.

## Considered options

### Option A: Pick once more after a lost approved state

- Pros: a concurrent approval gives the new approved version. The flow runs at most two picks.
- Cons: a second concurrent approval in one export still gives a transient `404 E-EXP-001`. The flow gets one more read-only transaction and one more lock.

### Option B: Pick again until the state is stable

- Pros: no count of concurrent approvals gives a transient `404 E-EXP-001`.
- Cons: the loop has no bound.

### Option C: Accept the transient 404 in the documents

- Pros: no code change.
- Cons: `E-EXP-001` then also means that the approved version changed during the request. The owner rejected this option (`docs/plan/review/2026-11-13-fix-plan.md` §1).

## Decision

The project uses **Option A**.

- (a) This item applies to the route `/approved` only. The lock of step 3 can find a state other than `Approved`. Then the flow ends the transaction of the lock without a commit. The flow runs step 2 once more and locks the new pick.
- (b) The second lock checks the state again. A state other than `Approved` there gives `404 E-EXP-001`. A second pick that finds no approved version gives the same answer. No third pick runs.
- (c) An example follows. The second pick finds version 4. An approval of version 5 retires version 4 before the second lock. The flow then gives `404 E-EXP-001` and writes no entry `VersionExported`.
- (d) The route by number does not change.
- (e) The Basic route and the v2 unit route share the flow, so they get the second pick too. The second pick does not run the Basic check, T1, or T2 of ADR-0109 (m) again.
- (f) The second pick runs inside `timeout.export` and keeps the same `export.permits` permit. The permit moves into the blocking task of step 5 only after the second check (ADR-0056 (b), (c)).
- (g) The flow holds one version lock at a time, so the lock order of ADR-0015 holds.
- (h) A test in `recipe-app` makes the concurrent approval. `MemoryStore` gets a hook that runs once before the next `lock_version_shared`, like the hook `on_read_draft`.
- (i) This ADR amends the answer `404 E-EXP-001` of the route `/approved` in ADR-0019. It also amends item (ah) of ADR-0109. The client rule of item (ah) stays in force.
- (j) Change `review-design-fixes` updates `api.md` §2.7 and the row `ExportVersion` of `data-model.md` §6.

## Consequences

### Positive

- One concurrent approval no longer gives a false `404 E-EXP-001` on the three `/approved` routes of `api.md` §2.7.
- The flow runs at most two picks and two locks.

### Negative

- Two concurrent approvals in one export still give a transient `404 E-EXP-001`.
- A second pick costs one more read-only transaction and one more lock inside `timeout.export`.

### Risks and mitigations

- Risk: a later change removes the second pick. Mitigation: the test of item (h) fails.
- Risk: the second pick passes `timeout.export`. Mitigation: the client gets `503 E-WEB-009`, and the flow commits no entry (ADR-0029, ADR-0056 (b)).

## Related

- ADR-0015, ADR-0019, ADR-0029, ADR-0056 (b), (c), ADR-0109 (m), (ah)
- `api.md` §2.7, `data-model.md` §6 row `ExportVersion`
- Finding F-02 of `docs/plan/review/2026-11-13-full-repo-review.md` §4
- Change `review-design-fixes`, capability `export`
- `overview.md` §7: R18, R27, R31. §9: A4
