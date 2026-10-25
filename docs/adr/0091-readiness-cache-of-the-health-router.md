# ADR-0091: Readiness cache of the health router

Status: Accepted. Amended by a note (item (e)).
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0059 (j) caches the result of the readiness probe for `health.readiness_cache`. It does not give the form of the cache. `reliability.md` section 8 lists the only places of interior mutability. One item of that list names "the readiness cache of 7.2 and the readiness state of 3.7, as atomics in `AppState`".

Step P4.1 builds the health router in `crates/recipe-web/src/health.rs`. The code differs from that item:

- The cache is a `tokio::sync::Mutex<Option<Cached>>` in the state of the health router, not in `AppState`. A `Cached` value holds one probe result and the instant when it arrived.
- A request holds the lock of the cache during the probe call. Thus only one probe call runs at a time.
- The flag `draining` is an `AtomicBool` in the state of the health router. `HealthState::begin_shutdown()` sets it at the signal. The list of section 8 does not name it.
- The readiness state of section 3.7 is an atomic. The health router and the job `alerts` share it through `HealthState`, not through `AppState`.

Facts of the probe:

- The probe runs inside the outer timeout `health.db_check_timeout`. The port also stops its own check at that limit.
- Docker, the proxy, and an operator can send `/readyz` at the same time. Without one probe call at a time, each request that finds an old entry starts a probe on the health pool.

## Decision drivers

- A slow database does not get a stampede of probe calls from the health router (R30).
- After the signal, `/readyz` answers `not ready` and the readiness state stays not ready (`reliability.md` section 7.3).
- The list of section 8 names every place of interior mutability.
- No new number in section 7.11, no new configuration key, and no new error code.

## Considered options

### Option A: An async lock held across the probe call

- Pros: one probe call runs at a time. A request that waits for the lock then reads the fresh entry. The health pool gets no stampede.
- Cons: a request can wait for the probe call of another request. The lock is a second kind of `Mutex` next to `parking_lot::Mutex` of section 2.2.

### Option B: Atomics for the cache

- Pros: no lock. The item of section 8 stays as it is.
- Cons: two requests that find an old entry at the same time both start a probe call. A slow database then gets more probe calls.

The user chose **Option A** on 2026-10-24.

## Decision

- (a) The readiness cache is a `tokio::sync::Mutex<Option<Cached>>` in the state of the health router.
  - A request of `/readyz` holds the lock across the probe call. The outer timeout `health.db_check_timeout` limits the probe call.
  - Thus only one probe call runs at a time. A request that waits for the lock reads the entry that the holder stored, while that entry is fresh.
  - The entry keeps a ready or a not ready result for `health.readiness_cache`.
  - `tokio::sync::Mutex::lock()` cannot return an error, like `parking_lot::Mutex` of section 2.2. The async lock is necessary because the guard stays alive across an `.await`.
- (b) The flag `draining` is an `AtomicBool` in the state of the health router. `begin_shutdown()` sets it once. No code clears it.
  - After `begin_shutdown()` sets the flag, `/readyz` answers `503` with `not ready` and starts no probe call.
  - The drain wins over a late result. A probe result that arrives after `begin_shutdown()` sets the flag, or while `begin_shutdown()` runs, must not leave `Ready` in the shared `ReadinessState`.
  - Such a result also does not change the answer of `/readyz`. The answer stays `not ready`.
- (c) The list of `reliability.md` section 8 names these places for readiness:
  - the readiness state of section 3.7, as an atomic that the health router and the job `alerts` share,
  - the readiness cache of section 7.2, as the `tokio::sync::Mutex` of item (a),
  - the flag `draining` of item (b).
- (d) The rules of sections 7.2 and 7.3 stay as they are. The router keeps the layers of ADR-0059 (l) and (m).
- (e) Note on (a) and (b): `HealthState` has one more flag, `gate_open` (`AtomicBool`). It starts closed. The start of `serve` opens it once, after the public listener is bound (ADR-0061 step 10). While it is closed, `/readyz` answers `not ready`, and the checker runs no probe and writes no cache entry. The flag `draining` still wins. The flag adds no `Mutex`.

## Consequences

### Positive

- A slow database gets at most one probe call at a time from the health router.
- The list of section 8 agrees with the code.
- After the signal, the job `alerts` and `/readyz` both see the server as not ready.

### Negative

- A request of `/readyz` can wait for the lock. The timeout `health.db_check_timeout` does not cover that wait.
- The code has two kinds of `Mutex`. Only the readiness cache uses `tokio::sync::Mutex`.

### Risks and mitigations

- Risk: a request waits for the probe calls in front of it. Mitigation: each holder in front of it holds the lock for at most one probe window of `health.db_check_timeout`. The first holder usually stores a fresh entry, and the next request reads it.
- Risk: the client of a holder disconnects, and the server drops the request of the holder. Mitigation: the drop releases the guard. Then the next request that waits runs the probe call.
- Risk: a dropped holder can make the next request wait for two probe windows. That request can then exceed `watchdog.probe_timeout`, and one Docker probe may fail. Mitigation: Docker marks the container unhealthy only after `health.check_failures` failures.
- Risk: a probe result and `begin_shutdown()` run at the same time. Mitigation: item (b) requires that the drain wins, and a test covers that order of events.

## Related

- ADR-0059 (g), (j), (l), (m). ADR-0059 does not give the form of the cache, so it gets no note.
- ADR-0058 (l) (readiness turns off at the signal), ADR-0061 (readiness turns on after the first successful check).
- `docs/architecture/reliability.md` sections 2.2, 3.7, 7.2, 7.3, 7.11, and 8.
- `openspec/changes/operations-compose/design.md` D5.
- `crates/recipe-web/src/health.rs` (`HealthState`, `Checker`), `crates/recipe-app/src/operations/readiness_state.rs` (`ReadinessState`).
- `overview.md` section 7: R30.
