# Recipe editor

The recipe editor is a Rust web application. It edits and versions the recipes of the machines of one production line. It stores typed parameters with process limits in PostgreSQL and records every change in an audit log. It exports an approved version as a JSON document with a content hash.

The application runs 24/7 on one on-premises Docker host. Caddy terminates TLS. The server renders HTML with axum, askama, and HTMX, and PostgreSQL 16 stores the data. The editor never communicates with a PLC. Data safety has priority over performance.

The repository is a Cargo workspace with six crates in `crates/`. The build follows the steps of `docs/plan/implementation-prompts.md`. The current state is the workspace skeleton and the CI workflow. The crates contain no application code yet.

- [Architecture overview](docs/architecture/overview.md)
- [Architecture Decision Records](docs/adr/README.md)
- [Reliability rules: errors, lints, tests, operations](docs/architecture/reliability.md)
- [Contribution rules](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)
