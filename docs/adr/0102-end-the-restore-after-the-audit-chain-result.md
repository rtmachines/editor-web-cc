# ADR-0102: End the restore after the result of the audit chain check

Status: Accepted
Date: 2026-10-26
Deciders: project owner

## Context

Three documents state the end of a restore into production. They do not agree:

- ADR-0100, section "Negative", last bullet: "The operator reads the result after the run, not at the end of the restore procedure."
- `reliability.md` section 7.7, restore bullet: "Last, the operator waits for the next completed run of the job `audit_chain_check` and reads its result (ADR-0100)."
- `docs/ops/runbook.md` section "Restore into production", step 10: "Check the audit chain. Wait for the next completed run of the job `audit_chain_check`. Run no command for it."

The bullet of ADR-0100 puts the result outside the procedure. Section 7.7 and step 10 put the wait and the result into the procedure as its last step.

An accepted ADR changes only by its Status line and an "Amended by" note. This ADR therefore records the decision and amends ADR-0100.

## Decision drivers

- Two documents never disagree.
- The operator ends a restore only with a known state of the production chain.
- The decision changes no code, no job, and no setting.

## Considered options

### Option A: The procedure ends after the result

- Pros: it matches section 7.7 and step 10. The operator ends the restore with a known state of the chain.
- Cons: the procedure lasts until the next completed run of the job.

### Option B: The procedure ends before the result

- Pros: the procedure ends sooner.
- Cons: section 7.7 and step 10 need a change. No step of the procedure owns the result of the check.

## Decision

The project uses **Option A**.

- (a) Step 10 of the runbook section "Restore into production" is the last step of the procedure.
- (b) The procedure ends only after the operator reads the result of the next completed run of the job `audit_chain_check` and accepts it.
- (c) When the operator cannot accept the result, the restore stays open. No document gives a complete next action. The runbook section "Known gaps" lists the action for `audit_chain_broken` and the action for a run that fails or does not complete.
- (d) This ADR amends the last bullet of the negative consequences of ADR-0100. The result still comes after the run. The restore procedure ends only with that result.
- (e) This ADR adds no job rule, no setting, and no command. It does not change the wait rules of ADR-0100 (a) to (e).
- (f) The case that the run fails or does not complete is a known gap in the runbook section "Known gaps". The next action for that case is out of the scope of this ADR.

Changes of other documents:

- (g) The follow-up edits of this ADR are:
  - `docs/adr/0100-wait-for-the-audit-chain-job-after-a-restore.md`: the Status line gets "Amended by ADR-0102". No other line changes.
  - `docs/adr/README.md`: a new index row for ADR-0102 and the same note in the Status cell of row 0100.
  - `docs/adr/README.md` decision map, row "Deployment, migrations, backup, panic policy, health": "Also 0102" after the item of 0101.
  - `docs/architecture/reliability.md` section 7.7, the bullet of the restore into production: the citation becomes "(ADR-0100, ADR-0102)". The text does not change.
  - `docs/ops/runbook.md` step 10 already states (a). The section "Related" gets a link to this ADR. This ADR changes no other line of the runbook.
  - No code, no script, no spec of `openspec/specs/`, and no number of `reliability.md` section 7.11 changes.

## Consequences

### Positive

- ADR-0100, `reliability.md` section 7.7, and the runbook agree on the end of a restore.
- The operator ends a restore only with a known state of the production chain.
- No code, no job, no setting, and no command changes.

### Negative

- The restore procedure lasts until the operator reads and accepts the result of the next completed run of the job. Without the catch-up run, the next run starts up to one day later. The wait also includes the time of the check and has no fixed upper limit.
- The operator keeps the restore open until the operator accepts that result.
- The runbook gives no complete action for a run that fails or does not complete. Item (f) records that case as a known gap.

## Related

- Amends ADR-0100 (the last bullet of the negative consequences).
- ADR-0086 (the second risk, amended by ADR-0100).
- `docs/architecture/reliability.md` section 7.7 (restore).
- `docs/ops/runbook.md` sections "Restore into production" and "Known gaps".
- `overview.md` section 7: R20.
