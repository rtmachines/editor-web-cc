# Recipe editor

The recipe editor is a Rust web application. It edits and versions the recipes of the machines of one production line. It stores typed parameters with process limits in PostgreSQL and records every change in an audit log. It exports an approved version as a JSON document with a content hash.

The application runs 24/7 on one on-premises Docker host. Caddy handles TLS. The server renders HTML with axum, askama, and HTMX, and PostgreSQL 16 stores the data. The editor never communicates with a PLC. Data safety has priority over performance.

## Status

This repository holds release 0.1.0. It has the route groups of `docs/architecture/api.md` section 2, the Compose stack of `docs/architecture/reliability.md` section 7, and the backup and the restore. The [changelog](CHANGELOG.md) lists every change.

## Known gaps of 0.1.0

- The soak test of `reliability.md` section 6 is not implemented. No soak script and no nightly CI job exist. The 2 h local soak run with an RSS chart did not run, so no `docs/ops/soak-<date>.md` exists.
- Seven tests with `#[cfg(unix)]` ran only in CI on Linux.
- The operations gaps are in the section [Known gaps](docs/ops/runbook.md#known-gaps) of the runbook.

The section "Known gaps" of the [changelog](CHANGELOG.md) is the authority for this list.

## Documentation map

| Reader | Document |
|---|---|
| Users | [User guide](docs/user/README.md) |
| Operators | [Runbook](docs/ops/runbook.md) |
| Developers and reviewers | [Architecture overview](docs/architecture/overview.md), [Architecture Decision Records](docs/adr/README.md), [Reliability rules](docs/architecture/reliability.md) |
| Contributors | [Contribution rules](CONTRIBUTING.md) |

## Quick start for developers

Run these commands from the repository root. [CONTRIBUTING.md](CONTRIBUTING.md) lists the full set of local checks.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

The tests of `recipe-db` need Docker for `testcontainers`, or a `DATABASE_URL` that names a Compose database.

To see the pages in a browser without a database, run the development server. It uses in-memory adapters and loses all data when it stops.

1. Set the variable `RECIPE_DEV_PASSWORD` to a password of 12 to 128 characters.
2. Run `cargo run -p recipe-web --example dev_server`.
3. Open `http://127.0.0.1:8080/login`.

The full stack starts with `docker compose -f deploy/compose.yaml up -d`. Do not use that command in production before you prepare the host, the environment file, and the secret files. The production start is in the section [First start, first administrator, and image transfer](docs/ops/runbook.md#first-start-first-administrator-and-image-transfer) of the runbook.

## Workspace

The Cargo workspace has six crates in `crates/`.

| Crate | Role |
|---|---|
| `recipe-domain` | Entities, value objects, invariants, `Permission`, error codes, and the hashed export body. No I/O. |
| `recipe-app` | Use cases, ports (traits), permission checks, and audit entries. |
| `recipe-db` | PostgreSQL adapters with `sqlx`, the session store, and the unit of work. |
| `recipe-export` | Export JSON Schema, the JSON writer of the HTTP response, and golden tests. |
| `recipe-web` | axum routers, askama templates, HTMX fragments, i18n, security headers, and the session cookie. |
| `recipe-server` | Binary and library. Configuration, wiring, the supervised jobs, and the subcommands `serve`, `migrate`, `create-admin`, and `healthcheck`. |

Other paths: `migrations/` (forward-only SQL), `deploy/` (Compose, Caddy, and the backup scripts), and `openspec/` (specifications and changes).
