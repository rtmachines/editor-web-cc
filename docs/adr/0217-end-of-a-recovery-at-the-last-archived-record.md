# ADR-0217: End of a recovery at the last archived record

Status: Accepted. Amends ADR-0106 (g) step 2, (p) step 4, and `reliability.md` §7.7. Amended by ADR-0221 (step 5 of item (b))
Date: 2026-11-22
Deciders: project owner

## Context

ADR-0106 (g) step 2 checks the end of a recovery without a target time. The end must lie in the newest segment of `/wal` on the timeline of the base backup. `PITR_EXPECTED_LSN` also bounds the end, but the operator can read that value only when `db` runs (ADR-0106 (i) step 3).

PostgreSQL treats an invalid record as the end of the WAL. So a damaged record in the newest segment ends the replay early, and the end still lies in that segment. Without `db`, check 2 then passes. The audit chain check passes on the prefix, and `pitr-volume` writes `pitr volume completed`.

A CI run of the job `pitr-recovery` showed this case. The run inverted one byte at offset 859124 of the `.partial` file of the newest segment. The replay ended at offset 859104, before the marker 2. `pitr-volume` ended with the code 0.

These facts of PostgreSQL 16 apply. The sources are `xlogrecord.h`, `xlog_internal.h`, `xlogreader.c`, and the section "Backup Control Functions" of the documentation:

- A record starts with the header `XLogRecord` of 24 bytes. Its first field is `xl_tot_len` (4 bytes). The field `xl_crc` starts at byte 20.
- Each record starts at a multiple of 8 bytes. So the field `xl_tot_len` always lies on the page where its record starts.
- A page starts with a page header of 24 bytes. The first page of a segment has the long page header of 40 bytes.
- `xl_tot_len` counts the header and the data of the record. It does not count the page headers that the record crosses.
- The page header holds `xlp_rem_len`. On a continuation page, it holds the bytes of the record that remain at that page.
- At a segment boundary, `pg_walfile_name` names the preceding segment.
- The history file of the new timeline holds the end of the replay. That position is the end of the last valid record.

`walarchive-entry` runs `pg_receivewal` without compression. `pg_receivewal` fills each new file with zero bytes and then writes only the received WAL.

## Decision drivers

- Data safety comes before the time of the procedure. A recovery that ends early never becomes production without a stop (ADR-0106, R31).
- A recovery after a crash must reach the newest commit before the torn last record (A35).
- The check works without `db` and without a manifest of the archive.
- The check refuses no complete archive.

## Considered options

### Option A: Keep the segment check of ADR-0106 (g) step 2

- Pros: no change of code.
- Cons: a damaged record in the newest segment ends the replay early without an `ERROR` line. The operator can then switch to a volume that lacks commits.

### Option B: The strict form for every case

The check refuses each end with a non-zero byte after the offset of the end.

- Pros: one simple rule. It refuses every damaged record.
- Cons: it also refuses a torn last record after a crash. A run with `--target-time` then cannot reach the newest commit, because a target after the newest commit gives the `ERROR` line of ADR-0106 (a).

### Option C: The tail rule

The check reads the length of the record at the end of the replay. It refuses the end when a non-zero byte follows the extent of that record.

- Pros: it refuses a damaged record that has more records after it. It accepts a torn last record after a crash.
- Cons: one damaged length can still pass (see "Risks and mitigations"). The check reads the WAL format of PostgreSQL 16.

## Decision

The project uses **Option C**. This is a decision of the owner of 2026-11-22.

- (a) The tail rule is part of check 2 of ADR-0106 (g), in step 9 of ADR-0106 (f). It runs only without a target time. It runs after check 1 and after the `INFO` line of the SHA-256 values of step 7.
- (b) The tail rule has these steps:
  1. The check reads the end of the replay from the history file of the new timeline, as ADR-0106 (g) step 2 does.
  2. At a segment boundary (the end modulo `wal_segment_size` is 0), `pg_walfile_name` names the preceding segment. The check then uses the next segment. When `/wal` does not hold that segment, the check passes.
  3. When the segment of the end is the newest segment of `/wal` on the base timeline, the check opens that file or its `.partial` file.
  4. The check reads `xl_tot_len` (4 bytes) of the record at the offset of the end. At a page boundary, the record starts after the page header: 24 bytes, or 40 bytes at a segment start.
     - A record header that crosses a page boundary gives the strict form of Option B. The check then reads the bytes from the offset of the end.
  5. A length of 0 is a clean end. The check passes and writes no `WARN` line.
  6. Otherwise the check computes the physical extent of the record. `xl_tot_len` does not count the page headers, so the check adds each page header that the record crosses. A record that crosses the end of the segment continues in the next segment file.
  7. Each page inside the extent after its first page is a continuation page. An all-zero page counts as "not written", which agrees with a torn record. Any other page must have the flag `XLP_FIRST_IS_CONTRECORD` and the `xlp_rem_len` that the length gives, else the check refuses the end.
  8. The check reads the bytes from the end of the extent to the end of the newest file. When one of these bytes is not zero, the check refuses the end.
  9. Otherwise the last record is torn or damaged, and only zero bytes follow it. The check passes and writes one `WARN` line. The line says that the newest record may be lost, because it is torn or damaged.
- (c) A refusal writes the `ERROR` line of ADR-0106 (g) step 2: `the recovery ended before the end of the WAL archive`. The run then ends with a code other than 0. No new error line and no new key of `reliability.md` §7.11 exist.
- (d) The tail rule tells two cases apart. A damaged record has more records after it, so a non-zero byte follows its extent. A torn last record after a crash has only zero bytes after it.
  - In the CI run of the context, the inverted byte lies in the field `xl_crc` of the record at offset 859104. So the length of that record stays readable, and the marker 2 follows its extent.
- (e) ADR-0106 (p) step 4 gets a fourth run. It uses a copy of `/wal` with one inverted byte in the record before the marker 2. That record lies in the `.partial` file of the newest segment.
  - The run inverts a byte of `xl_crc`. When the record header crosses a page boundary, it inverts the first byte of the record.
  - The run uses no target time and no `PITR_EXPECTED_LSN`. It expects the `ERROR` line of (c).
- (f) The body of ADR-0106 does not change. Its Status line gets the note "Amended by ADR-0217" with the scope of (a) to (e).
- (g) The job `pitr-recovery` does not run on a pull request (ADR-0094). A dispatched run of the job on the branch of the change must show the refusal of (e) in the step `refusals`.
- (h) The follow-up edits of this ADR are:
  - `reliability.md` §7.7 states the tail rule in check 2.
  - `docs/ops/runbook.md` names the tail rule. It also names the next step after a refusal: a run with `--target-time` before the damaged record.
  - The `operations` spec "Checks of a recovered cluster" gets the scenarios of the damaged record, the torn last record, and the clean end.
  - `pitr-volume`, its tests, `deploy/ci/pitr-recovery.sh`, and `deploy/ci/test_pitr_recovery.py` implement (b) to (e).
  - `docs/adr/README.md` gets the index row and the note in the Status cell of ADR-0106.

## Consequences

### Positive

- A damaged record with later records after it ends the run with an `ERROR` line, also without `db`.
- A torn last record after a crash does not stop the recovery. The recovered cluster holds every commit before that record.
- The check needs no manifest of the archive, no new service, and no new key.

### Negative

- A torn last record gives a `WARN` line. The operator cannot tell from the line alone whether the record was torn or damaged.
- The check reads the WAL format of PostgreSQL 16. A major upgrade of PostgreSQL can change that format.
- After a refusal, the operator needs a run with `--target-time` before the damaged record. The recovered cluster then holds no commit after that time.

### Risks and mitigations

- Risk: a damaged length can end its extent inside the last written page, after the later records of that page. No continuation page is then in the extent, and only zero bytes follow it. The check then passes a damaged archive with one `WARN` line. Mitigation: the `WARN` line names a possible loss of the newest record. The `xlp_rem_len` of each crossed page refuses most other damaged lengths (step 7 of (b)). The fault needs one damaged byte inside the 4 bytes of `xl_tot_len`.
- Risk: a fact of the context does not hold for a later PostgreSQL release. Mitigation: the stub cases of the `pitr-volume` tests pin each fact that the check uses. The fourth run of (e) uses the PostgreSQL image of each run of the job `pitr-recovery`.
- Risk: a segment after `pg_switch_wal()` ends at a segment boundary. Mitigation: step 2 of (b) uses the next segment. The marker runs of ADR-0106 (p) steps 4 and 6 must still pass.

## Related

- ADR-0106 (f), (g), (i), (p) (the recovery from the WAL archive), ADR-0068 (the WAL archive)
- ADR-0077 (the markers), ADR-0094 (the trigger of the CI jobs)
- `reliability.md` §7.7
- `docs/ops/runbook.md` section "Recover from the WAL archive"
- Baseline requirement of `operations`: "Checks of a recovered cluster"
- Change `review-design-fixes-2`
- `overview.md` §7: R31, R34. `overview.md` §9: A35
