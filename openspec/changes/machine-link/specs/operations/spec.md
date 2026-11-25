## MODIFIED Requirements

### Requirement: HTTPS access through the proxy
Users SHALL reach the application over HTTPS through the `proxy` service. The `proxy` service SHALL handle TLS and redirect HTTP to HTTPS. Its HTTP site SHALL answer each path under `/export/` and under `/adapter/` with `403` and no redirect. In production, it SHALL read its certificate and key from secret files. It SHALL return `404` for `/healthz*` and `/readyz*` (A18, A55).

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

#### Scenario: Plain HTTP request of an adapter path
- **GIVEN** the service `proxy` with a site certificate
- **WHEN** a client sends `POST /adapter/v1/reports` over plain HTTP
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

### Requirement: Startup settings comparison
At startup, the server SHALL compare each of its configuration keys with the stored value in `app_setting` in one transaction. For each changed value, the server SHALL write one `SettingChanged` audit entry with `actor = None`. The comparison SHALL also run in read-only mode (A24, A34, A55).

#### Scenario: Changed separation of duties
- **GIVEN** the stored value `true` of `approval.separation_of_duties`
- **WHEN** the operator runs `docker compose up -d app` with `approval.separation_of_duties` set to `false`
- **THEN** the audit log holds one `SettingChanged` entry with the old value `true` and the new value `false`

#### Scenario: Comparison in read-only mode
- **GIVEN** free space on `pgdata` at 4 % and a changed value of `session.idle_timeout`
- **WHEN** the service `app` starts
- **THEN** the audit log holds one `SettingChanged` entry for `session.idle_timeout`
- **AND** every command except `Login`, `Logout`, `ExportVersion`, `ExportPlcConnection`, and `ReportLoadResult` returns `E-WEB-006`
- **AND** the status reports of the adapters apply
