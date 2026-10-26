# ADR-0100: Wait for the scheduled audit chain check after a restore

Status: Accepted. Amended by ADR-0102 (the restore procedure ends after the operator reads and accepts the result of the next completed run)
Date: 2026-10-26
Deciders: project owner

## Context

Two documents disagree about the last step of a restore into production:

- `reliability.md` section 7.7 ends the bullet of the restore with "Last, the operator runs the audit chain check."
- `docs/ops/runbook.md` section "Restore into production" says that no command runs the chain check on demand. Its section "Known gaps" holds the same statement.
- The second risk of ADR-0086 also says that the operator runs the audit chain check after each restore.

The code and the documents give these facts on the tools:

- The binary `recipe-server` has four subcommands: `migrate`, `serve`, `create-admin`, and `healthcheck` (`crates/recipe-server/src/config/args.rs`). No subcommand runs the check.
- The scripts of `recipe-pgtools` run the check only on a scratch cluster. `restore-test` checks the newest dump (section 7.7, step 7). `pitr-check` checks a recovered cluster (ADR-0077 (e)).
- No script runs the check on the database `recipe` of `db`.
- The supervised job `audit_chain_check` in `app` is the only check of the production chain. It runs every night at `audit.check_time` (`data-model.md` section 4.9).
- At a start of `app`, the job runs one catch-up check after `audit.catchup_delay`. It does so only when `audit.last_check_at` is missing or older than `audit.catchup_age` (ADR-0060 (v), (w), ADR-0088 (b)).
- The restore brings back `app_setting` from the dump. `audit.last_check_at` then holds the time of the last check before the dump (ADR-0086 (f)).
- Step 1 of the runbook procedure runs `restore-test` on the set that step 3 restores. The chain of that set thus gets one check before the restore.

The code and the documents give these facts on the result of a check:

- A check is completed when it reaches the head or a break. Then the job records `audit.last_check_seq` and then `audit.last_check_at` (`data-model.md` section 4.9).
- On a break, the job writes a `security_event` row of the kind `audit_chain_broken` and returns `E-AUD-002`.
- The supervisor then writes one `ERROR` line with the field `error_code`. The line gets the field `job` from the span `job` (ADR-0088 (a), ADR-0089 (a)).
- The job `alerts` raises the condition `audit_chain_broken` while the newest row of that kind is younger than `audit.max_check_age` (ADR-0073 (a)). Its `WARN` line holds the name in the field `alert` (ADR-0075 (c)).
- A check without a break writes no log line. The supervisor writes a line only for an `Err` or a panic of a run.
- A run that returns another error, for example `E-DB-002`, is not a completed check. `audit.last_check_at` keeps its value.

## Decision drivers

- Two documents never disagree. Each rule lives in one document, and the other documents link to it.
- The production chain gets a check after each restore. A break never stays silent.
- The decision changes no code. It adds no subcommand, no script, no setting, and no number to `reliability.md` section 7.11.
- The operator reads the result from existing sources: `app_setting`, the event log, and the alert conditions.
- Data safety comes before the time of the procedure.

## Considered options

### Option A: Wait for the next run of the job `audit_chain_check`

- Pros: no code changes. The job is the tested check of the production chain. Its result reaches the existing alerts and the webhook.
- Cons: the result comes only after the next run. That run is the catch-up run or the nightly run.

### Option B: Add an on-demand command

- A new subcommand of `recipe-server` or a new script of `recipe-pgtools` runs the check at once.
- Pros: the result comes before users work on the restored database.
- Cons: new code, new tests, and a change of the hand-written parser of ADR-0059 (f).
- Cons: a second writer of `audit.last_check_seq` and `audit.last_check_at` can run at the same time as the job. A script that writes nothing gives no alert.

### Option C: Keep the disagreement as a known gap

- Pros: no document changes now.
- Cons: section 7.7 is normative and names a step that no tool supports. The operator cannot tell which document applies.

## Decision

The project uses **Option A**.

- (a) After a restore into production, the operator runs no command for the audit chain check. The operator waits for the next completed run of the job `audit_chain_check`.
- (b) The next completed run is the applicable run that completes first. `data-model.md` section 4.9 gives the rules of both runs:
  - The catch-up run after `audit.catchup_delay`. It applies when the restored `audit.last_check_at` is missing or older than `audit.catchup_age` at the start of `app`.
  - The nightly run at `audit.check_time`. It stays active in all cases.
  - A newer completed nightly check suppresses the catch-up run.
- (c) The operator reads the result in three places:
  - `audit.last_check_at` in `app_setting`. The operator records the restored value before the start of `app`. A value that differs from the recorded value after the start shows a completed check. A value equal to the recorded value is not a new check. The restored value can lie in the future (`reliability.md` section 3.7, clock trust). An active `audit_check_age` is unresolved evidence about the age or the clock trust of the check (ADR-0093 (a)). It does not prove that no check completed. The operator investigates it (section "Alarms, read-only mode, and restart loop" of the runbook, `reliability.md` section 3.7) before accepting the result.
  - The event log of `app`. An `ERROR` line of the job `audit_chain_check` with `error_code` `E-AUD-002` shows a break.
  - The alert condition `audit_chain_broken` of `reliability.md` section 3.7. It is active after a break, and the webhook sends its name (ADR-0063).
- (d) The durable proof of a chain without a break is the table `security_event`. A completed check as in (c) and no new row of the kind `audit_chain_broken` show a chain without a break.
  - The operator records the highest `id` of `security_event` before the start of `app`, together with the value of `audit.last_check_at`. A new row is a row with a higher `id`.
  - The job writes the row of a break before it records `audit.last_check_at` (`data-model.md` section 4.9). A completed check after a break therefore leaves a new row.
  - The dump holds the old rows of `security_event` (`reliability.md` section 7.7). The comparison with `id` excludes them. The column `occurred_at` is not safe, because a restored row can carry a future time.
  - The `ERROR` line and the alert `audit_chain_broken` are diagnostic signals. Docker can drop log lines (`reliability.md` section 3.5). A failed snapshot of the job `alerts` evaluates no condition (`reliability.md` section 3.7).
  - A missing `E-AUD-002` line or an inactive alert alone does not show a chain without a break.
- (e) A run that returns another code is not a completed check. The operator reads the code in the `ERROR` line and follows its row in `reliability.md` section 1.2. The condition `audit_check_age` becomes active when `audit.last_check_at` is older than `audit.max_check_age`.
- (f) This ADR adds no rule of the job. The rules stay in `data-model.md` section 4.9, `reliability.md` sections 3.7 and 7.11, ADR-0060, ADR-0073, ADR-0086, and ADR-0088.
- (g) This ADR amends the second risk of ADR-0086. The mitigation "The operator runs the audit chain check after each restore" now means items (a) to (e).

Changes of other documents:

- (h) The follow-up edits of this ADR are:
  - `docs/architecture/reliability.md` section 7.7, the bullet of the restore into production: the last sentence states (a) and cites ADR-0100.
  - `docs/adr/0086-bounds-and-anchor-of-the-audit-chain-check.md`: the Status line gets "Amended by ADR-0100". No other line changes.
  - `docs/adr/README.md`: a new index row for ADR-0100 and the same note in the Status cell of row 0086.
  - `docs/adr/README.md` decision map, row "Deployment, migrations, backup, panic policy, health": "Also 0100" after the item of 0099.
  - `docs/ops/runbook.md` section "Restore into production", step 10: the three places of (c) and the rules of (d) and (e).
  - `docs/ops/runbook.md` section "Known gaps": the bullet on the on-demand check is no longer a gap of the documents.
  - No code, no script, no spec of `openspec/specs/`, and no number of `reliability.md` section 7.11 changes.

## Consequences

### Positive

- `reliability.md` section 7.7, ADR-0086, and the runbook agree on the last step of a restore.
- No code, no subcommand, no script, and no setting changes.
- The check after a restore is the same check as each night. It uses the same alerts and the same webhook.
- The restore test of step 1 checks the chain of the restored set before the restore.

### Negative

- The result of the check comes only after the next run of the job.
- Without the catch-up run, the delay is up to one day, the interval of the nightly run at `audit.check_time`.
- With the catch-up run, the delay is `audit.catchup_delay` plus the time of the check, or less when the nightly run completes first.
- Users can work on the restored database before the first check of the production chain completes.
- The operator reads the result after the run, not at the end of the restore procedure.

### Risks and mitigations

- Risk: a break in the restored chain stays unknown until the next run. Mitigation: the restore test of step 1 checked the same set. The alert `audit_chain_broken` and the webhook report a break of the next run without the operator.
- Risk: the operator reads the restored `audit.last_check_at` as the time of a new check. Mitigation: item (c) compares the value with the value that the operator recorded before the start of `app`. A restored value in the future gives no false proof.
- Risk: the restored `audit.last_check_at` is younger than `audit.catchup_age`, so no catch-up run applies. Mitigation: the nightly run stays active at `audit.check_time`, as item (b) states.

## Related

- Amends ADR-0086 (the second risk: the audit chain check after a restore).
- ADR-0059 (f) (hand-written parser), ADR-0060 (v), (w) (catch-up), ADR-0063 (webhook), ADR-0067 (restore test).
- ADR-0073 (a) (window of the alert), ADR-0075 (c) (field `alert`), ADR-0077 (e) (`pitr-check`), ADR-0088 (a), (b) (span `job`, `RunKind`), ADR-0089 (a) (`AppError::Audit`).
- `docs/architecture/data-model.md` section 4.9 (the job, the catch-up, the state keys).
- `docs/architecture/reliability.md` sections 1.2 (`E-AUD-002`), 3.2 (span `job`), 3.7 (`audit_chain_broken`, `audit_check_age`), 7.7 (restore), and 7.11.
- `docs/ops/runbook.md` sections "Restore into production" and "Known gaps".
- `crates/recipe-server/src/config/args.rs` (the parser of the subcommands), `crates/recipe-server/src/supervisor.rs` (the lines of a failed run).
- Baseline requirements: audit "Nightly audit chain check" and "Catch-up audit chain check". The change adds no delta.
- `overview.md` section 7: R20.
