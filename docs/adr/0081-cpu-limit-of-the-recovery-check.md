# ADR-0081: CPU limit of the recovery check

Status: Accepted
Date: 2026-10-24
Deciders: project owner

## Context

`reliability.md` section 7.8 and ADR-0070 (f) say "Each service gets `cpus`". The review of `deploy/compose.yaml` found a gap for the service `pitr-check`:

1. ADR-0068 (o) and section 7.7 give the temporary container of `pitr-check` the memory limit `container.backup_memory`. No row of section 7.11 gives it a CPU value.
2. The row `container.small_cpus` of section 7.11 names five services: `proxy`, `backup`, `walarchive`, `migrate`, and `backup-premigrate` (ADR-0070 (j)).
3. `pitr-check` is in the profile `pitr` and is not one of the seven services of section 7.1 (ADR-0077 (d)). The file gives it `mem_limit` and `memswap_limit`, but no `cpus`.
4. A `cpus` value without a row of section 7.11 breaks the rule of that section. The service therefore has no CPU limit.

## Decision drivers

- Each container on the host has a limit. One container cannot take the CPU of `db` (R30, A1).
- Every number of `deploy/` equals section 7.11 (ADR-0070 (u)).
- A new key of section 7.11 is necessary only when no row fits.

## Considered options

### Option A: The row `container.small_cpus` also covers the temporary container of `pitr-check`

- Pros: the rule "Each service gets `cpus`" holds for every service of `compose.yaml`. The row follows the pattern of `container.backup_memory`, which already covers this container. Section 7.11 gets no new key.
- Cons: the recovery of `pitr-check` runs on 0.5 CPU and can take longer in CI.

### Option B: The CPU rule of section 7.8 covers only the seven services

- Pros: section 7.11 and `compose.yaml` stay as they are.
- Cons: `pitr-check` runs without a CPU limit. The rule of section 7.8 and the requirement "Container limits" get an exception.

## Decision

The project uses **Option A**.

The services of `container.small_cpus`:

- (a) The row `container.small_cpus` of section 7.11 names the temporary container of `pitr-check` next to the five services. The value stays 0.5. This item amends ADR-0070 (j).
  - Section 7.8 names the temporary container of `pitr-check` in the list of `container.small_cpus`.
  - Section 7.7 gives the temporary container of `pitr-check` `container.backup_memory` and `container.small_cpus`. This item amends ADR-0068 (o).
  - The service `pitr-check` of `compose.yaml` gets `cpus: 0.5`. The test of ADR-0070 (u) compares the value with section 7.11.

The reasons:

- (b) The decision keeps three rules:
  - The rule "Each service gets `cpus`" of section 7.8 and ADR-0070 (f) holds for each service of `compose.yaml`. A service in a profile is no exception.
  - The row `container.backup_memory` covers `backup` and the temporary container of `pitr-check`. The CPU row now follows the same pattern.
  - Section 7.11 gets no new key.

## Consequences

### Positive

- Each container of `compose.yaml` has a CPU limit.
- The test of ADR-0070 (u) can compare every `cpus` line with section 7.11.

### Negative

- The recovery of `pitr-check` runs on 0.5 CPU, so the CI job can take longer.

### Risks and mitigations

- Risk: the recovery on 0.5 CPU takes longer than `scratch.pitr_start_timeout`. Mitigation: the CI job writes the time of the recovery to its log. A higher value then needs a new ADR with an own key.

## Related

- Amends ADR-0070 (j) (the services of `container.small_cpus`) and ADR-0068 (o) (the CPU limit of the temporary container of `pitr-check`).
- ADR-0067 (b) (`container.backup_memory`), ADR-0070 (f) (each service gets `cpus`), ADR-0077 (d) (`pitr-check` in the profile `pitr`).
- `docs/architecture/reliability.md` sections 7.7, 7.8, and the row `container.small_cpus` of section 7.11.
- Baseline requirement: `operations` "Seven Compose services". The delta keeps `operations` "Container limits".
- `overview.md` section 7: R04, R30. Section 9: A1.
