# ADR-0075: Names of the alert conditions

Status: Accepted. Amended by ADR-0093 (b)
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 builds the job `alerts` and the port `AlertWebhook` in `recipe-app`. `reliability.md` section 3.7 and ADR-0063 leave one name open:

1. The table of section 3.7 gives each condition and its source. It gives no name of a condition.
2. ADR-0063 (l) and section 3.7 give the webhook body `{"alert":"<condition>"}`. No document says which text replaces `<condition>`.
3. Section 3.7 requires one `WARN` line with the field `alert` for each active condition. No document gives the value of the field.
4. ADR-0073 (f) makes the free disk condition active for each volume. No document says how many lines and calls this gives, or which line names the volume.

The enum `AlertCondition` of `recipe-app` already gives each condition a name. The name is an external contract:

- The receiver of the webhook at the plant reads the body.
- The operations staff search the event log for the field `alert`.
- The runbook lists the conditions and their actions.

## Decision drivers

- The system runs 24/7, and the plant must see each alert (R30).
- Each name has one normative place, so the code, the runbook, and the receiver agree (R34).
- The event log holds structured lines, and a search for one condition finds all its lines (R37).
- A name never changes, so a receiver at the plant keeps working after an update.
- No new setting, no new row of section 7.11, and no new error code.

## Considered options

### Option A: The names in a new ADR and in a column of section 3.7

- Pros: the code, the runbook, and the receiver cite one place. A review can compare the enum with the document.
- Cons: a new condition needs a new ADR.

### Option B: The names only in the code until a later step

- Pros: no new ADR now.
- Cons: the code is not normative. The runbook and the receiver then copy each name from a source file. A rename in the code changes the contract without a decision.

### Option C: The names only in the runbook

- Pros: the operator finds the names in the guide that the operator uses.
- Cons: the runbook is a how-to guide, not a normative document. The code and the runbook can drift.

## Decision

The project uses **Option A**.

- (a) Each condition of the table of section 3.7 has one stable name. The names, in the order of the table, are:

  | Condition | Name |
  |---|---|
  | More than `alerts.panic_count` panic events in `alerts.panic_window` | `panics` |
  | `/readyz` not ready for more than `alerts.not_ready_for` | `not_ready` |
  | Latest dump older than `backup.max_age_alert` | `backup_age` |
  | `recipe_replicator` not connected, or its lag at or above `wal.max_lag_bytes` | `replication` |
  | Free disk below `disk.warn_percent` on `pgdata`, `backups`, or `walarchive` | `low_disk` |
  | More than `alerts.failed_login_count` failed logins in `alerts.failed_login_window` | `failed_logins` |
  | Any `csrf_rejected`, or a `forbidden` on `UserManage`, in the last `alerts.security_window` | `security_events` |
  | Audit chain check failed | `audit_chain_broken` |
  | `audit.last_check_at` older than `audit.max_check_age` | `audit_check_age` |
  | One or more `E-EXP-002` in the last `alerts.interval` | `export_hash_mismatch` |

Note: The condition `export_hash_mismatch` is active when the total of `E-EXP-002` is higher than the total of the previous run (ADR-0093 (b)).

- (b) A name has lower-case ASCII letters and underscores only, and it starts with a letter.
  - A name never changes.
  - A removed name is never used again.
  - A new condition gets a new name in a new ADR.
- (c) The webhook body holds the name: `{"alert":"<name>"}`. This item amends ADR-0063 (l).
  - The `WARN` line of an active condition holds the name in the field `alert`.
- (d) The webhook body holds only the key `alert`, as ADR-0063 (l) gives it. The body of `low_disk` therefore names no volume.
  - The port `AlertWebhook` gets only the conditions of a run, and no volume (`AlertWebhook::send_all`).
- (e) The condition `low_disk` writes one `WARN` line for each low or unreadable volume in a run.
  - The line holds the field `alert` with `low_disk` and the field `volume` with `pgdata`, `backups`, or `walarchive`.
  - A run sends one `POST` with the body `{"alert":"low_disk"}` when one or more volumes are low or unreadable.
  - Each other condition writes one `WARN` line per run.

## Consequences

### Positive

- The receiver at the plant, the event log, and the runbook use one name for each condition.
- An update of the application keeps every name, so a receiver at the plant needs no change.
- A review compares the enum `AlertCondition` with one table.

### Negative

- A new condition or a changed condition needs a new ADR and a new name.
- The webhook body of `low_disk` names no volume. The operator finds the volume in the field `volume` of the `WARN` lines in the event log.

### Risks and mitigations

- Risk: the code and section 3.7 drift apart. Mitigation: a unit test of `recipe-app` compares each `AlertCondition::name()` with the name of this ADR.
- Risk: a receiver at the plant expects a name that a later ADR removes. Mitigation: item (b) never uses a removed name again, so an old name never gets a new meaning.

## Related

- Amends ADR-0063 (l) (the text of the webhook body) and ADR-0073 (f) (the field `volume` of the `low_disk` line).
- ADR-0060 (the job `alerts`), ADR-0073 (a), (c), (f) (the conditions of the audit chain, the dump age, and a read fault of a mount). Item (e) adds the field `volume` to the lines of ADR-0073 (f).
- `docs/architecture/reliability.md` section 3.7 (the column "Name", the field `volume`, and the webhook body)
- Baseline requirement: `operations` "Alert conditions". The delta changes `operations` "Webhook of the alerts".
- `overview.md` section 7: R30, R34, R37
