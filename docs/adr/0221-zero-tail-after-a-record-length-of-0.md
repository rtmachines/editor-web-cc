# ADR-0221: Zero tail after a record length of 0

Status: Accepted. Amends ADR-0217 (b) step 5 and `reliability.md` §7.7, and adds the item of a short file and a known limit
Date: 2026-11-22
Deciders: project owner

## Context

ADR-0217 (b) adds the tail rule to check 2 of ADR-0106 (g). Its step 5 says: "A length of 0 is a clean end. The check passes and writes no `WARN` line."

The code review of the change `review-design-fixes-2` found a silent pass in step 5 (finding F1 of the Opus review). Two faults give a length of 0 at the end of the replay:

- A block of zero bytes on the archive disk starts at the page where the replay ended.
- One damaged byte changes the field `xl_tot_len` to 0.

In both cases, later records can follow the zero length. Step 5 then passes without a line, and the recovered cluster lacks these records.

The same code review found a short read in the helper of the tail rule (finding F02 of the Codex review). The helper took a file shorter than the offset of the end as a clean end. The code now refuses such a file, but ADR-0217 does not state that rule.

The code review also found a case that the tail rule does not reach (finding F01 of the Codex review). The last record is torn and crosses the end of its segment into an existing next file, for example `N+1.partial`. The segment of the end is then not the newest segment. So step 3 of ADR-0217 (b) does not apply, and the gap check of ADR-0106 (g) step 2 refuses the end.

ADR-0217 (a) states the fact that this ADR uses: `pg_receivewal` fills each new file with zero bytes and then writes only the received WAL. So a clean end of the archive has only zero bytes after it.

## Decision drivers

- Data safety comes before the time of the procedure. A recovery that ends early never becomes production without a stop (ADR-0106, R31).
- The check works without `db` and without a manifest of the archive (ADR-0217).
- The check refuses no complete archive. Item (d) keeps one exception: a torn record that crosses into an existing next segment.

## Considered options

### Option A: Keep step 5 of ADR-0217 (b)

- Pros: no change of code.
- Cons: a zero block or one damaged length at the end hides every later record. The run then ends without a `WARN` line and without an `ERROR` line.

### Option B: A zero tail after a length of 0

At a length of 0, the check also reads the bytes from the start of the record to the end of the newest file. A non-zero byte refuses the end.

- Pros: it refuses a zero length that has later records after it. A complete archive has only zero bytes after its end (ADR-0217 (a)), so it still passes.
- Cons: one more read of the tail of the newest file.

### Option C: A `WARN` line at each length of 0

- Pros: the operator sees each length of 0.
- Cons: every clean end then gives a `WARN` line. The line does not tell a clean end from a lost record, and a recovery that ends early still completes.

## Decision

The project uses **Option B**. This is a decision of the owner of 2026-11-22.

- (a) Step 5 of ADR-0217 (b) changes. At a length of 0, the check reads the bytes from the record start of step 4 to the end of its file.
  - At a page boundary, the start of the record lies after the page header of step 4. This is also the start of the read of the strict form of step 4.
  - When one of these bytes is not zero, the check refuses the end.
  - When all of these bytes are zero, the end is a clean end. The check passes and writes no `WARN` line, as before.
- (b) The check refuses the end when the file of step 3 of ADR-0217 (b) is shorter than the segment size. It also refuses the end when a read of the helper returns fewer bytes than it asks for.
  - A read of the zero tail to the end of the file is the exception. It reads until the end of the file.
  - `pg_receivewal` fills each file to the full segment size (ADR-0217 (a)). So a shorter file is not a file of `pg_receivewal`, and the check cannot trust it.
- (c) A refusal of (a) or (b) writes the `ERROR` line of ADR-0217 (c): `the recovery ended before the end of the WAL archive`. No new error line and no new key of `reliability.md` §7.11 exist.
- (d) Known limit: a torn last record that crosses into an existing next segment file. The segment of the end is then older than the newest segment of `/wal`.
  - The gap check of ADR-0106 (g) step 2 refuses the end with the `ERROR` line of (c). The project keeps this refusal, because it is safe.
  - The operator cannot tell this refusal from a gap or a damaged record.
  - The next step is the step of ADR-0217 (h): a run with `--target-time` before the newest commit. The operator selects the time by ADR-0106 (b).
  - The recovered cluster then holds no commit after the target time.
- (e) The body of ADR-0217 does not change. Its Status line gets the note "Amended by ADR-0221 (step 5 of item (b))".
- (f) The follow-up edits of this ADR are:
  - `reliability.md` §7.7 states (a), (b), and (d).
  - The delta of the `operations` spec "Checks of a recovered cluster" in the change `review-design-fixes-2` gets the scenarios of (a) and (b).
  - `pitr-volume` and the cases of its tests implement (a) and (b). A red case writes a non-zero byte after a length of 0.
  - `docs/ops/runbook.md` names the next step of (d).
  - `docs/adr/README.md` gets the index row and the note in the Status cell of ADR-0217.

## Consequences

### Positive

- A zero length with later records after it ends the run with an `ERROR` line, also without `db`.
- A clean end of the archive still passes without a line.
- A file that `pg_receivewal` did not write ends the run with an `ERROR` line.

### Negative

- After a crash with a torn record across a segment boundary, the recovery without a target time stops with an `ERROR` line (d).
  - The newest commits before the torn record are then not available without a target time.
  - This case is an exception to drivers 2 and 4 of ADR-0217 and to its first positive consequence.
- The check reads the tail of the newest file once more at a length of 0.

### Risks and mitigations

- Risk: a zero block covers every later record of the archive. The tail is then all zero, and the check passes a damaged archive without a line. Mitigation: none in the check. The fault needs a zero block from the end of the replay to the end of the newest file.
- Risk: the damaged length of the risks of ADR-0217 stays. Mitigation: the `WARN` line of ADR-0217 (b) step 9 applies, as before.
- Risk: the refusal of (d) stops a recovery after a crash. Mitigation: the next step of (d) recovers each commit before the target time. The runbook names that step.

## Related

- ADR-0217 (a), (b), (c), (h) (the tail rule), ADR-0106 (a), (b), (g) (the recovery from the WAL archive)
- ADR-0068 (the WAL archive)
- `reliability.md` §7.7
- `docs/ops/runbook.md` section "Recover from the WAL archive"
- Baseline requirement of `operations`: "Checks of a recovered cluster"
- Change `review-design-fixes-2`
- `overview.md` §7: R31. `overview.md` §9: A35
