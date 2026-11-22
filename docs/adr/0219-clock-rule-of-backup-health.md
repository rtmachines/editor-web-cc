# ADR-0219: Clock rule of `backup-health`

Status: Accepted. Amends ADR-0066 (d) and `reliability.md` §7.7, §7.11
Date: 2026-11-22
Deciders: project owner

## Context

The review of 2026-11-21 found that the healthcheck `backup-health` trusts a manifest time in the future. The finding has the severity LOW.

In this ADR, the manifest time is the modification time of a manifest (`stat -c %Y`). The clock is the time `date +%s` in the container `backup`.

ADR-0066 (d) requires a complete set that is younger than `backup.max_age_alert`. The script computes the age as the clock minus the manifest time. The script has no lower bound for the age. A manifest time in the future gives a negative age, and the check then passes.

The failure scenario has three steps:

1. The clock of the host is one month ahead when a dump completes.
2. The operator sets the clock back to the true time, and the dumps stop.
3. The healthcheck reports `backup is healthy` for about one month.

The job `alerts` already has a clock rule (ADR-0093 (a), `reliability.md` §3.7). A stored time later than `database_now` by more than `alerts.interval` is untrusted, and its condition stays active. So the alert `backup_age` stays active in the scenario. Only the health status of the container `backup` is wrong.

No normative document states a clock rule for `backup-health`.

## Decision drivers

- A backup is on a second disk, or the healthcheck fails (A35). A clock fault never hides a missing dump (R31).
- The project runs 24/7, and the operator reads the health status of each container (R30).
- A failed healthcheck is the safe direction. An extra failure is better than a hidden missing dump.
- Each rule lives in one document. The healthcheck and the job `alerts` use the same tolerance for one clock fault.
- No new number in `reliability.md` §7.11 and no new configuration key.

## Considered options

### Option A: Reuse `alerts.interval` as the tolerance, and apply the rule to each complete set

- Pros: the healthcheck and the job `alerts` trust the same skew. §7.11 gets no new key. A future time on any set shows the clock fault.
- Cons: `alerts.interval` gets a third meaning. A backward step of the clock by more than `alerts.interval` can fail the healthcheck for a recent set.

### Option B: A new key for the tolerance of the healthcheck

- Pros: each setting keeps one meaning.
- Cons: two tolerances for one clock fault can disagree. §7.11 gets one more key, and the test guards of the deploy values get one more row.

### Option C: Keep the literal rule "younger than"

- Pros: the script stays as it is.
- Cons: a future time gives a negative age for any distance. The healthcheck passes until the clock passes the manifest time and the maximum age. During that time a missing dump gives no failed healthcheck.

## Decision

The decision selects Option A.

- (a) `backup-health` uses `alerts.interval` (60 s) as the tolerance of the clock. No new §7.11 key follows. ADR-0093 (a) uses the same value as the largest skew that the job `alerts` trusts.
- (b) The rule applies to each complete set that the loop of `backup-health` reads, not only to the newest set.
- (c) The script computes the age of each complete set in this way:
  - A manifest time at or before the clock has the age clock minus manifest time.
  - A manifest time later than the clock by at most `alerts.interval` has the age 0. The border is inclusive, so the script trusts a time exactly 60 s ahead.
  - A manifest time later than the clock by more than `alerts.interval` is untrusted. The healthcheck then fails, because a future time hides the age.
- (d) An untrusted set fails the healthcheck also when a newer set is complete and younger than `backup.max_age_alert`.
- (e) The rule lives in the bullet `backup-health` of `reliability.md` §7.7. The §7.11 row `alerts.interval` adds §7.7 to its sections.
- (f) The new constant of `backup-health` gets a row in `SCRIPT_ASSIGNMENTS` of `crates/recipe-server/tests/deploy_values/files/scripts.rs`. It also gets a row in `SCRIPT_CONSTANTS` of `deploy_values/file_mutations/scripts.rs`.
- (g) The date stub of `deploy/pgtools/tests/lib/stubs.sh` gets a fixed epoch. The tests hold the border: the script trusts a time 60 s ahead and does not trust a time 61 s ahead.
- (h) A mixed case gets a test. An older complete set has a manifest time 61 s ahead, and the newest set is healthy. The healthcheck fails. The entry point `backup-test` calls each new case by name.
- (i) `docs/ops/runbook.md` gets one step for a manifest time in the future. The operator compares the clock of the host with a time server and fixes the clock. Then the operator waits for the next healthcheck.

## Consequences

### Positive

- A manifest time in the future can no longer keep the container `backup` healthy after the dumps stop.
- The healthcheck and the alert `backup_age` show the same clock fault with the same tolerance.
- No new §7.11 key and no new configuration key are necessary.

### Negative

- `alerts.interval` is also the tolerance of `backup-health`. A change of its value changes the job `alerts` and the healthcheck together.
- After a backward step of the clock by more than `alerts.interval`, a recent set is untrusted. The healthcheck then fails, although the dumps run.

### Risks and mitigations

- Risk: a set from a time with a clock ahead stays untrusted until the clock passes its manifest time minus `alerts.interval`. The retention of ADR-0066 (k) compares the same manifest time, so it does not delete that set earlier. When the operator fixes the clock, the healthcheck still fails. Mitigation: the failed healthcheck is the safe direction (R31). This ADR gives no step that clears such a set earlier.

## Related

- ADR-0066 (d), (k) (the checks of `backup-health` and the retention), ADR-0093 (a) (the clock rule of the job `alerts`)
- `reliability.md` §3.7, §7.7, §7.11
- `docs/ops/runbook.md`
- Change `review-design-fixes-2`
- `overview.md` §7: R30, R31. `overview.md` §9: A35
