# ADR-0150: No per-login limit over all addresses

Status: Accepted. Amends ADR-0035 (known limit of code review M9)
Date: 2026-11-08
Deciders: project owner

## Context

T1 of `Login` counts the failed attempts of one client address in the throttle window (`data-model.md` section 6, `domain-model.md` section 2.3). The statement gives two counts:

- The count per `(login, client_ip)` starts the progressive delay. `login.delay_after_failures` is 3 failures per `(login, client_ip)` in 15 min. `login.delay_formula` is `2^(n-3)` s, cap 60 s.
- The count per `client_ip` is the per-address budget. `login.ip_budget` is 100 failures per `client_ip` in 15 min, then 429.

In this ADR, the free attempts are the 3 attempts of a pair before the delay starts.

ADR-0035 names a known limit (code review M9). The throttle has no count per login over all addresses. A client with many addresses gets the free attempts and its own budget at each address. ADR-0035 says that step P4.2 decides on a per-login limit in a new ADR.

`ChangeOwnPassword` and the Basic check of the export routes run the same T1 (ADR-0035 (b), ADR-0109 (m)). The limit applies to them too.

The project owner decided on 2026-11-08 (fix plan, question Q-F, option (1)).

## Decision drivers

- The throttle has no permanent lockout. An Administrator can unlock a login (A22, R30).
- A host at one address must not stop the login of a user at another address.
- Only an Administrator holds the permission `LoginUnlock` (`api.md` section 3).
- Every numeric value comes from `reliability.md` section 7.11.

## Considered options

### Option A: Accept the limit

- Pros: no code, no migration, and no new setting. The failures of one address do not start a delay at another address.
- Cons: a client with many addresses gets the free attempts and the per-address budget at each address.

### Option B: A capped per-login delay in T1 under the class-1 lock

T1 holds the class-1 lock of the login (`data-model.md` section 1.4). T1 can also count the failures of the login over all addresses. A second delay with a cap then applies to every address.

- Pros: the number of attempts on one login has a bound. The bound does not grow with the number of addresses.
- Cons: T1 gets a new count of the stored attempt rows of the login. `reliability.md` section 7.11 gets new rows, and the specs change.
- Cons: a lockout path. Any host on the plant network can send one wrong password per delay and keep the delay active for every address.
- Cons: ADR-0035 (f) records a throttled attempt as a failure. The throttled attempts of the real user then also start a new delay.
- Cons: an attacker can delay the login of every Administrator. Then no user with `LoginUnlock` can log in to clear the delay.

## Decision

The project uses **Option A**.

- (a) The login throttle keeps its two counts: per `(login, client_ip)` and per `client_ip`.
- (b) The throttle gets no count per login over all addresses. It gets no per-login delay and no per-login lockout.
- (c) This ADR records the decision on the known limit of code review M9 of ADR-0035. The limit stays. This ADR amends ADR-0035 (section "Decision", known limit M9).

Reasons:

1. The password policy needs 12 to 128 characters and refuses a password from the breached-password list (`reliability.md` section 7.11 `password.min` / `password.max`, A23). An online attack from a word list is unlikely to succeed, because of this policy.
2. Each attempt that reaches the password check holds an Argon2 permit. `argon2.permits` is 4, so at most 4 password checks run on the server at the same time.
3. The Argon2id cost is `argon2.m_cost` / `t_cost` / `p_cost` of 19 MiB / 2 / 1. This cost and the permits bound the rate of attempts per server, for all addresses together.
4. The alert `failed_logins` reports a flood of failures on some paths (`reliability.md` section 3.7). Section "Risks and mitigations" names the paths.
5. A per-login limit lets any host on the plant network start a lockout of an Administrator. Option B shows the path. The current counts keep the delay at the address of the attacker.

## Consequences

The decision needs no code, no migration, and no spec change. No document of `docs/architecture/` changes. The Status line of ADR-0035 gets an "Amended by ADR-0150" note, and `docs/adr/README.md` gets the index row.

### Positive

- The failures of one address do not start a delay or a budget stop at another address.
- Failures from another address do not count against the pair or the address of an Administrator.

### Negative

- Many addresses give many attempts against one login. Each address gets the free attempts, the delay up to its cap of 60 s, and its own budget.
- A flood from many addresses can still hold all `argon2.permits`. A login then waits for `argon2.acquire_timeout` and returns `E-WEB-009` (`reliability.md` section 7.11). This delays every login, also that of an Administrator.

### Risks and mitigations

- Risk: a client with many addresses on the plant network makes many attempts against one login.
- Mitigation: the control is the alert `failed_logins`, not a stop.
  - Rows of `login_failed` aggregate per `(client_ip, kind, minute)`. A later failure in the same minute adds to `count` (`data-model.md` section 3.8, ADR-0014).
  - The alert sums `count` over all addresses. It becomes active above `alerts.failed_login_count` in `alerts.failed_login_window`. The values are 50 and 15 min (`reliability.md` sections 3.7 and 7.11).
- Paths that the alert covers: `Login` and the Basic check of the export routes. A wrong password or an unknown login writes `login_failed` on both paths.
- Path that the alert does not cover: `ChangeOwnPassword`. A wrong current password returns `InvalidCurrentPassword` and writes no security event. The attempt still counts in T1, but the alert reads only `login_failed`.
- Residual risk: a client with a session can try the current password of that login from many addresses without an alert. A later change can add an event for this path. That change needs its own ADR.
- Operator action: the runbook row `failed_logins` (`docs/ops/runbook.md`, section "Alarms") gives the steps.
  1. Open `/security-events`.
  2. Filter the kind `login_failed`.
  3. Read `login` and `client_ip`.
- If a user cannot log in, an Administrator opens `/users/<login>` and uses the unlock form. The runbook gives no step that blocks an address.

## Related

- Amends ADR-0035 (known limit of code review M9, section "Decision").
- ADR-0014 (session and identity model), ADR-0096 (closed the sibling limit M7 of ADR-0035), ADR-0109 (m) (Basic check of the export routes).
- `crates/recipe-db/src/login_attempts.rs` (statement `COUNT_FAILURES`: `pair_failures` and `address_failures`).
- `domain-model.md` section 2.3 (`LoginThrottle`). `data-model.md` sections 1.4 and 6 (row `Login`).
- `reliability.md` section 3.7 (alert `failed_logins`) and section 7.11 (rows `login.delay_after_failures`, `login.delay_formula`, `login.ip_budget`, `argon2.m_cost` / `t_cost` / `p_cost`, `argon2.permits` / `argon2.acquire_timeout`, `password.min` / `password.max`, `alerts.failed_login_count` / `alerts.failed_login_window`).
- `api.md` section 3 (permission `LoginUnlock`). `docs/ops/runbook.md` (section "Alarms").
- `overview.md` section 7: R30. Section 9: A22, A23.
