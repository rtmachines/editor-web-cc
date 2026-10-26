# ADR-0065: TLS of the proxy and the default trusted proxy

Status: Accepted. Amended by a note on the risk of `PROXY_ADDRESS`. The check reads the event `login_failed` of a login with a wrong password in the security events. A successful login is an audit entry (A33).
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 writes `deploy/Caddyfile` and `deploy/compose.yaml`. The documents leave these points of the service `proxy` open:

- A18 says: "When the plant terminates TLS elsewhere, Caddy re-encrypts to the upstream". ADR-0020 and `reliability.md` section 7.1 repeat it. `recipe-web` has no TLS listener, and `overview.md` section 3 shows "HTTP, internal network" from `proxy` to `app`.
- No document names the source of the certificate and the key in production, in development, or in CI.
- `recipe-web` sets CSP, `nosniff`, `Referrer-Policy`, HSTS, and `Cache-Control` on its own answers (`api.md` section 9). Caddy also writes answers, for example the `404` of the health paths and a `502` during a restart of `app`.
- Caddy keeps an idle upstream connection for 2 minutes by default. `app` closes an idle connection after `http.body_read_timeout` (10 s). Caddy then answers a `POST` on a closed connection with `502`.
- The default of `http.trusted_proxies` is the whole Compose network. Every container of that network can then send a false `X-Forwarded-For`, and the throttle of `api.md` section 7.4 trusts it.
- A plant device in front of Caddy appends its own address. Without a trusted entry for the device, every client gets the address of the device as `client_ip` and shares one login budget.
- The network `backend` has `internal: true` (ADR-0070). The webhook of ADR-0063 cannot then reach a host outside the Compose networks.

## Decision drivers

- TLS ends in one place that the project controls (A18).
- The answers of Caddy carry the same security headers as the answers of `app` (`api.md` section 9).
- A `POST` never gets a `502` because of an idle upstream connection.
- Only the address of `proxy` can set the client address of a request (`api.md` section 7.1, ADR-0035 (p)).
- A wrong address in the Compose file cannot give every client the same address.
- `app` is the only service that reaches a host outside the Compose networks.

## Considered options

### Option A: Caddy terminates every TLS connection, plain HTTP to `app`, and a trusted list of one address

- Pros: `recipe-web` needs no TLS listener and no certificate. The trusted list holds only the address of `proxy`. The plant PKI gives the certificate in production.
- Cons: the traffic from `proxy` to `app` is plain HTTP inside the Compose network. A plant device in front of Caddy needs one more entry in two places.

### Option B: Caddy re-encrypts to a TLS listener of `app`

- Pros: TLS also protects the traffic inside the Compose network.
- Cons: `recipe-web` needs a TLS listener, a certificate, and a rotation of that certificate. No document designs them.

### Option C: Caddy with its internal CA in production, and the whole Compose network as the trusted list

- Pros: no certificate files from the plant.
- Cons: each workstation must trust the root of the Caddy CA. Every container of the network can set the client address.

## Decision

The project uses **Option A**.

TLS:

- (a) Caddy terminates every TLS connection of a client. A plant device in front of Caddy sends TLS to Caddy, and Caddy terminates it. Caddy sends plain HTTP to `app:8080` inside the Compose network.
- (b) In production, `deploy/Caddyfile` reads the certificate and the key of the plant PKI from the secret files `tls_cert` and `tls_key`. In the container, the paths are `/run/secrets/tls_cert` and `/run/secrets/tls_key`.
  - On the host, the files are `${RECIPE_SECRETS_DIR}/tls_cert` and `${RECIPE_SECRETS_DIR}/tls_key`, outside the repository.
  - The files `tls_cert` and `tls_key` belong to uid 0, the user of the Caddy image, with the mode `0400`.
  - The production file sets the global option `auto_https disable_certs`. Caddy then never asks a public CA for a certificate, also when the plant certificate does not cover `SITE_ADDRESS`.
  - In development and in CI, `deploy/Caddyfile.dev` uses `tls internal`, and the CI job needs no certificate file. The internal CA needs the certificate automation of Caddy, so this file has no `disable_certs`.
- (c) A certificate rotation recreates the container `proxy`. A running container keeps its secret files, and `admin off` blocks a reload.
- (d) Caddy redirects each plain HTTP request to the same path over HTTPS with a permanent redirect. The Caddyfile writes the redirect explicitly.
- (e) Caddy sets `protocols h1 h2`, because the Compose file publishes no UDP port for HTTP/3.

Headers and health paths:

- (f) Caddy sets HSTS, CSP, `nosniff`, and `Referrer-Policy` only as defaults with the prefix `?`. A default applies only when the answer has no value. The answers of `app` therefore keep their own values, and the answers of Caddy get the same headers. The HSTS value equals `http.hsts_max_age`.
  - Two snippets of the Caddyfile hold the defaults and the deletion of (g). The first snippet holds CSP, `nosniff`, `Referrer-Policy`, and the deletion. The second snippet imports the first and adds HSTS.
  - The HTTPS site block and its error route `handle_errors` import the second snippet. The redirect block of (d) imports only the first.
  - The redirect of (d) gets CSP, `nosniff`, and `Referrer-Policy`, but no HSTS. RFC 6797 section 7.2 forbids HSTS on an answer over plain HTTP.
- (g) Caddy deletes the header `Server`.
- (h) Caddy answers `404` for `/healthz*` and `/readyz*`. The `path` matcher of Caddy ignores case, cleans the path, and decodes escapes. The CI job sends the variants of `operations` "HTTPS access through the proxy" and expects `404` for each.

Upstream connection:

- (i) `reverse_proxy app:8080` sets `transport http { keepalive 5s }`. The new row `proxy.upstream_keepalive` holds 5 s. The value must stay below `http.body_read_timeout`, so Caddy closes an idle connection before `app` does.

Trusted proxies and networks:

- (j) The default of `http.trusted_proxies` changes from the Compose network to `172.28.0.2/32`, the address of `proxy`. A request from another container uses its socket address as `client_ip`.
- (k) `.env.example` holds `COMPOSE_SUBNET=172.28.0.0/16`, `PROXY_ADDRESS=172.28.0.2`, and `COMPOSE_IP_RANGE=172.28.1.0/24`. The code, the tests, and `01-roles.sh` already use this subnet.
  - `compose.yaml` gives `proxy` the fixed address `${PROXY_ADDRESS}` on `backend`.
  - It sets `RECIPE_HTTP__TRUSTED_PROXIES: ${PROXY_ADDRESS:?}/32` for `app` from the same variable.
  - A change of the subnet edits the three values together. The runbook names the step.
- (l) The network `backend` has `ip_range: ${COMPOSE_IP_RANGE}`, which excludes `PROXY_ADDRESS`. Docker therefore never gives that address to another container that starts first.
- (m) When a plant device stands in front of Caddy, the device CIDR goes into the global `trusted_proxies` of Caddy and into `http.trusted_proxies` of `app`. The runbook advises one `/32` for each device.
- (n) The network `edge` holds `proxy` and the published ports. The network `egress` holds `app` only and publishes no port. `app` uses it for the webhook of ADR-0063. ADR-0070 gives the network `backend`.
- (o) `deploy/compose.dev.yaml` binds its published port to `127.0.0.1` only. A development host therefore does not publish the application on its LAN.

Error answers of Caddy:

- (p) An error answer of Caddy, for example the `502` while `app` restarts, holds the status code and the status text of Caddy. It has no error code and no `request_id`, because `app` did not answer.
- (q) When the request has `HX-Request: true`, that answer also sets `HX-Retarget: #error-panel`, `HX-Reswap: innerHTML show:#error-panel:top`, and `HX-Push-Url: false` of `api.md` section 5.3. The page keeps its form, and the text goes into the error panel.

## Consequences

### Positive

- `recipe-web` has no TLS code and no certificate.
- Each HTTPS answer of Caddy and of `app` carries HSTS, CSP, `nosniff`, and `Referrer-Policy`, and no `Server` header. The redirect over plain HTTP carries CSP, `nosniff`, and `Referrer-Policy`.
- An HTMX request during a restart of `app` keeps its page. The text of Caddy goes into the error panel.
- Caddy never asks a public CA for a certificate in production.
- A `POST` after an idle period gets no `502` from Caddy.
- A neighbor container cannot set the client address. One variable feeds both lines, so the two values cannot differ.

### Negative

- The traffic from `proxy` to `app` is plain HTTP inside the Compose network. ADR-0070 limits that network.
- A certificate rotation interrupts the service for the restart of `proxy`.
- A plant device in front of Caddy needs one manual entry in two files. The runbook names both.
- An error answer of Caddy has no error code and no `request_id`. The user sees only the status text.
- A plant certificate that does not cover `SITE_ADDRESS` gives a TLS error in the browser. Caddy gets no other certificate.

### Risks and mitigations

- Risk: a wrong `PROXY_ADDRESS` gives every client the address of `proxy`. Mitigation: the runbook checks `client_ip` of the first login in the security events. The test `recipe-server/tests/deploy_values.rs` checks the trusted proxies line and the `ip_range`.
- Risk: an edit of the Caddyfile opens a health path to the public site. Mitigation: the CI job sends the path variants and expects `404`.

## Related

- Amends ADR-0020 (the service `proxy`: no re-encryption to the upstream).
- ADR-0035 (p) (client address), ADR-0063 (webhook), ADR-0070 (networks, capabilities, stack test).
- `overview.md` section 9: A18 (Caddy terminates TLS, plain HTTP to `app`).
- `api.md` section 7.1 (the default trusted proxy and the device entry). Section 5.3 (the headers of an HTMX error answer).
- RFC 6797 section 7.2 (no HSTS on an answer over plain HTTP).
- `reliability.md` section 7.1 (the row `proxy`). Section 7.11: the new row `proxy.upstream_keepalive` 5 s and the changed row `http.trusted_proxies` with the default `172.28.0.2/32`.
- Baseline requirements: `operations` "HTTPS access through the proxy" and `identity-access` "Client address from trusted proxies". The delta adds `identity-access` "Default trusted proxy".
- `overview.md` section 7: R38. Section 9: A18, A22.
