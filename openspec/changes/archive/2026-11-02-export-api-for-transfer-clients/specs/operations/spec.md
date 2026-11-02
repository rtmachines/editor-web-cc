## MODIFIED Requirements

### Requirement: HTTPS access through the proxy
Users SHALL reach the application over HTTPS through the `proxy` service. The `proxy` service SHALL handle TLS and redirect HTTP to HTTPS. Its HTTP site SHALL answer each path under `/export/` with `403` and no redirect. In production, it SHALL read its certificate and key from secret files. It SHALL return `404` for `/healthz*` and `/readyz*` (A18).

#### Scenario: Health path through the proxy
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /healthz` to the public address
- **THEN** `proxy` returns `404`

#### Scenario: Readiness path through the proxy
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /readyz?x=1` to the public address
- **THEN** `proxy` returns `404`

#### Scenario: Plain HTTP request
- **GIVEN** the service `proxy` with a site certificate
- **WHEN** a client sends `GET /login` over plain HTTP
- **THEN** `proxy` answers with a permanent redirect to the same path over HTTPS
- **AND** the answer holds CSP, `nosniff`, and `Referrer-Policy`, but no HSTS

#### Scenario: Plain HTTP request of an export path
- **GIVEN** the service `proxy` with a site certificate
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` over plain HTTP
- **THEN** `proxy` returns `403` without a `Location` header
- **AND** the request does not reach `app`

#### Scenario: HTMX request while the application restarts
- **GIVEN** the service `proxy` in front of the service `app`, and `app` does not answer
- **WHEN** a client sends a request with `HX-Request: true` to the public address
- **THEN** `proxy` returns `502` with `HX-Retarget: #error-panel` and `HX-Push-Url: false`
- **AND** the answer holds HSTS, CSP, `nosniff`, and `Referrer-Policy`, and no `Server` header

#### Scenario: Variant of a health path
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /HEALTHZ`, `GET /healthz/`, or `GET /readyz%2F` to the public address
- **THEN** `proxy` returns `404` for each request
