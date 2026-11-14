# ADR-0170: Secret mounts of the db service

Status: Accepted
Date: 2026-11-14
Deciders: project owner

## Context

`deploy/compose.yaml` gives the service `db` five secrets. One is `postgres_password`. The other four are the passwords of the login roles: `migrator_password`, `app_password`, `backup_password`, and `replicator_password`.

The mounts stay in `db` after the first start. A compromise of `db` therefore gives the clear text of the four passwords. An attacker can use them outside `db`.

The review of 2026-11-13 found no document that gives a reason for the four mounts. Its note says that only `01-roles.sh` reads them on an empty volume.

`01-roles.sh` reads the four files before its first change (lines 60 and 67 to 70). The runbook also reads them inside `db`. "Restore into production" step 6 sets each role password from its file (ADR-0103 (b), ADR-0033). "Change a database password" sets one new password from its file.

## Decision drivers

- Each empty volume of `db` gets its roles from `01-roles.sh` (ADR-0103 (k)).
- A container holds only the secrets that it needs.
- A change must not add an operator step that can fail without a sign.

## Considered options

### Option A: Keep the four mounts and record the risk

- Pros: no change of `deploy/`. `01-roles.sh` and the runbook find the files on each start of `db`.
- Cons: a compromise of `db` gives the clear text of the four passwords.

### Option B: Mount the four files only for the first start

- Pros: a compromise of `db` after the first start gives no clear text of the four passwords.
- Cons: Compose has no mount for the first start only. The operator must change the Compose files before and after each init.
- Cons: the superuser of `db` can already set each role password. The gain is therefore small.

## Decision

The project uses **Option A**.

- (a) The service `db` keeps the mounts of `migrator_password`, `app_password`, `backup_password`, and `replicator_password` (`deploy/compose.yaml` lines 237 to 242). No change of `deploy/` follows.
- (b) The project keeps the mounts for three reasons.
  1. `01-roles.sh` needs the four files on each empty volume (ADR-0103).
  2. Compose has no mount for the first start only.
  3. The superuser of `db` can already set each role password.
- (c) The project accepts this risk: a compromise of `db` gives the clear text of the four passwords for a use outside `db`.

## Consequences

### Positive

- `01-roles.sh` and the runbook find the four files in `db` with no extra step.
- The documents now give the reason for the four mounts and name the risk.

### Negative

- The clear text of the four passwords stays readable in `db` for the life of the container.

### Risks and mitigations

- Risk: an attacker in `db` reads the four passwords and logs in as `recipe_app` from another host. Mitigation: the `pg_hba.conf` of ADR-0069 accepts each login role only from the Compose subnet.
- Risk: an attacker uses a password on another system. Mitigation: the runbook makes each value with `openssl rand -hex 32` (ADR-0062 (o)). No other system shares it.
- Risk: a later change removes one of the four mounts. Mitigation: `01-roles.sh` then stops before its first change and writes one message.

## Related

- ADR-0031 (secret files of the roles), ADR-0033 (passwords from the secret files), ADR-0062 (secret files), ADR-0069 (database access), ADR-0103 (b), (k)
- `data-model.md` §5.1, `reliability.md` §5
- `docs/plan/review/2026-11-13-full-repo-review.md` §4 F-25, `docs/plan/review/2026-11-13-fix-plan.md` §1
- Change `review-design-fixes`, capability `operations`
- `overview.md` §7: R31, R34
