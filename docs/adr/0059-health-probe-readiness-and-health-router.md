# ADR-0059: Health probe subcommand, readiness rule, and health router layers

Status: Accepted
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 7.1 gives `app` the healthcheck `GET http://localhost:9000/readyz`. It names no tool. Sections 7.2 and `api.md` section 2.10 define the two health endpoints. Step P4.1 wires them, and the plan found these gaps:

- The image of ADR-0064 has no shell, no `curl`, and no `wget`. Docker needs a program inside the container for the healthcheck.
- `overview.md` section 5 and CLAUDE.md name three subcommands: `serve`, `migrate`, and `create-admin`.
- The documents give only the body `ok` or `not ready`. Docker and a probe read the status code, and no document gives it.
- Section 7.2 says that the schema version matches the binary. With equality, the previous image stays `not ready` after an expand migration. That breaks steps 3 and 6 of section 7.10.
- The probe has no timeout key. A probe of `/readyz` must wait longer than `health.db_check_timeout` (2 s), or it fails before the readiness check answers.
- Without layers, the health router breaks two baseline rules. Each response carries `X-Request-Id` and the headers of `api.md` section 9. A `5xx` answer gets an `ERROR` line.
- With the status 503, a `not ready` answer every `health.check_interval` would write an `ERROR` line each time.
- The layer `client_address` gives `E-WEB-010` for a request from a trusted proxy without `X-Forwarded-For` (ADR-0035 (p)). The healthcheck of `proxy` sends no such header.

## Decision drivers

- Docker detects a stalled or unready `app` (R30, A41).
- The image keeps no shell (ADR-0064).
- Seven services stay seven (`reliability.md` section 7.1, ADR-0020).
- An upgrade with an expand migration keeps the previous image ready (section 5, section 7.10).
- Each response carries `X-Request-Id` (R37). Every number comes from section 7.11.

## Considered options

### Option A: A fourth subcommand `recipe-server healthcheck`

- Pros: no new binary and no new crate. The probe code is shared with the watchdog.
- Cons: `overview.md` section 5 and CLAUDE.md change.

### Option B: A second static binary in the image

- Pros: the subcommands stay three.
- Cons: a second build target and a second binary to keep in step.

### Option C: A sidecar container with `curl`

- Pros: no code.
- Cons: an eighth service. It breaks "Seven services" of section 7.1 and ADR-0020.

### Option D: The `debug-nonroot` image with `busybox`

- Pros: no code.
- Cons: a shell in the production image.

## Decision

The project uses **Option A**.

Subcommand:

- (a) `recipe-server healthcheck [--path <path>]` sends `GET` to `127.0.0.1` on `http.internal_port`. The default path is `/readyz`. It exits with code 0 for the status 200 and with code 1 otherwise.
- (b) The probe uses `std` only: `TcpStream::connect_timeout`, a read timeout and a write timeout, `Connection: close`, and a bounded read of the status line. It needs no runtime, no secret, and no database.
- (c) The subcommand reads `RECIPE_HTTP__INTERNAL_PORT` and no other value. `main` returns an error for exit code 1, so the subcommand needs no `#[allow(clippy::exit)]`.
- (d) The watchdog thread of section 2.4 rule 5 uses the same probe on `/healthz`.
- (e) Both use the budget `watchdog.probe_timeout`, 3 s. The budget is above `health.db_check_timeout`, so a probe of `/readyz` waits for the readiness check.
- (f) The parser of the arguments stays hand-written, as ADR-0031 decides for the subcommands.

Endpoints:

- (g) `/healthz` and `/readyz` answer `ok` with the status 200, or `not ready` with the status 503. Both answers are `text/plain`.
- (h) `/readyz` is ready when the health pool runs `SELECT 1` within `health.db_check_timeout`. It also requires that every migration of the binary is in `_sqlx_migrations` with the same checksum and that no failed row exists.
- (i) A database with a newer migration stays ready. The previous image then serves during an upgrade with an expand migration.
- (j) The result is cached for `health.readiness_cache`. Readiness turns on after the first successful check after the start (ADR-0061), and it turns off at the signal (ADR-0058).

Health router:

- (k) The health router is a separate `Router`. The internal listener serves it, and the public router answers `404` for `/healthz` and `/readyz`.
- (l) The health router has the layer `request_id`, the security headers of `api.md` section 9, and `TraceLayer`. Its finish line has the level `DEBUG` for every status. This is the one exception to the level rule of `reliability.md` section 3.3.
- (m) The health router has no layer `client_address`, no session layer, and no CSRF layer. It has no `CatchPanicLayer`. A panic in a health handler ends only its connection (ADR-0058 (f)), and the probe counts a failure.

Documents and settings:

- (n) `overview.md` section 5 and the workspace table of CLAUDE.md name four subcommands: `serve`, `migrate`, `create-admin`, and `healthcheck`.
- (o) The Compose healthcheck of `app` in section 7.1 runs `recipe-server healthcheck --path /readyz`.
- (p) Section 7.11 gets the row `watchdog.probe_timeout` with the value 3 s. The row text says that it is the budget of the subcommand `healthcheck` and that it is above `health.db_check_timeout`.

## Consequences

### Positive

- The healthcheck of `app` works in an image without a shell.
- A rollback to the previous image after an expand migration keeps `app` ready.
- A health answer has `X-Request-Id` and the security headers, like every other answer.

### Negative

- The binary has a fourth subcommand.
- A failed readiness check writes no `ERROR` line. The alert condition of `alerts.not_ready_for` in section 3.7 is the signal.

### Risks and mitigations

- Risk: a slow readiness check fails the Docker probe. Mitigation: a unit test runs the probe against a local listener with a slow answer, an answer of 503, garbage, and silence.

## Related

- ADR-0020 (health endpoints). This ADR amends it.
- ADR-0031 (the parser of the arguments), ADR-0035 (p), ADR-0058, ADR-0061, ADR-0064
- `docs/architecture/overview.md` section 5 and CLAUDE.md (the workspace table)
- `docs/architecture/api.md` section 2.10 and section 9
- `docs/architecture/reliability.md` section 3.3 (the finish line at `DEBUG`), sections 7.1, 7.2, and 7.11
- Baseline requirements: operations "Health endpoints on the internal port", "Readiness check", "Event log format", and "Log level by HTTP status"
- `overview.md` section 7: R30, R37. Section 9: A41
