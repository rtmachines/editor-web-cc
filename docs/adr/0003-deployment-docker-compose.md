# ADR-0003: Deployment with Docker Compose on one on-premises host

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "Use PostgreSQL and backend service in a Docker container as the database." The sentence is broken. We read it as: PostgreSQL is the database, and both the database and the backend run as Docker containers (N1). The draft does not name a deployment model.

The application runs in an industrial plant, on the plant network, with no cloud connection. It must run 24/7 (R30).

## Decision drivers

- The draft names Docker.
- One host is enough for the expected scale (Assumption A19).
- The plant staff must be able to restart, back up, and restore the system with a short runbook.
- Fewer moving parts means fewer failure modes.

## Considered options

### Option A: Docker Compose on one host

- Pros: one file describes the system, simple restart policy, named volumes for data, easy backup of volumes, well known.
- Cons: no automatic failover, host maintenance means downtime.

### Option B: Kubernetes or k3s

- Pros: automatic restart and scheduling, rolling updates, path to multi-node.
- Cons: a large control plane for one application, more skills required, more failure modes, no benefit on one host.

### Option C: Bare metal with systemd units

- Pros: no container runtime, direct access to the host.
- Cons: PostgreSQL and Rust binary versions tied to the host OS, harder to reproduce, no isolation between services.

## Decision

We use Option A. The Compose file defines four services:

| Service | Image | Role |
|---|---|---|
| `proxy` | Caddy | TLS termination. Optional when the plant network provides TLS (Assumption A18) |
| `app` | `recipe-server` (multi-stage build, distroless or Debian slim) | The application |
| `db` | `postgres:16` | The database. Data on a named volume |
| `backup` | `postgres:16` with a cron script | Daily `pg_dump` to a backup volume (Assumption A16) |

Rules:

- Every service has `restart: always` and a healthcheck.
- `app` waits for `db` to be healthy before it starts.
- `app` runs migrations at start and refuses to start when a migration fails.
- Secrets (database password, session key) come from Docker secrets or an env file with restricted permissions. They are never in the image.
- The backup volume is on a different disk or is synchronized to backup storage by the plant.

## Consequences

### Positive

- One command starts or restarts the whole system.
- The runbook is short: start, stop, back up, restore, upgrade.
- The image pins the Rust binary and its libraries. The host OS does not matter.

### Negative

- No high availability. A host failure stops the application until the host returns or a restore is done.
- Upgrades cause a short downtime.

### Risks and mitigations

- Risk: the data volume is lost with the host. Mitigation: daily dumps to a separate volume plus a plant-level copy. Restore is tested on each release.
- Risk: the application starts before the database and fails. Mitigation: `depends_on` with `condition: service_healthy` and a retry loop in `recipe-server`.

## Related

- ADR-0004 (session store in PostgreSQL, no extra service)
- `reliability.md` section 7 (24/7 operation and backup)
- `overview.md` section 3 (container view), Assumptions A1, A16, A18
