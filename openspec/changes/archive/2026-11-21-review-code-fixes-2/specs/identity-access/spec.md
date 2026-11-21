## MODIFIED Requirements

### Requirement: Client address from trusted proxies
The server SHALL read `X-Forwarded-For` and `X-Forwarded-Proto` only from an address in the trusted-proxy list. For every other request, the server SHALL use the socket address as the client address. The server SHALL convert an IPv4-mapped IPv6 address to its IPv4 form before it compares or stores the address (R38, A22).

#### Scenario: Forwarded header from an untrusted address
- **GIVEN** a request from the address `10.0.0.9`, which is not in the trusted-proxy list
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses `10.0.0.9` as the client address

#### Scenario: Proxy on an IPv6 socket
- **GIVEN** the trusted-proxy list `172.20.0.0/16` and a request from the socket address `::ffff:172.20.0.2`
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses `10.0.0.5` as the client address

#### Scenario: Trusted-proxy entry in IPv4-mapped form
- **GIVEN** the trusted-proxy list `::ffff:172.28.0.2/128` and a request from the socket address `172.28.0.2`
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses `10.0.0.5` as the client address

#### Scenario: Forwarded entry that is not an address
- **GIVEN** a request from a trusted proxy
- **WHEN** the request carries the header `X-Forwarded-For: not-an-address`
- **THEN** the server rejects the request with `E-WEB-008`

#### Scenario: Trusted proxy without a forwarded header
- **GIVEN** a request from a trusted proxy
- **WHEN** the request carries no header `X-Forwarded-For`
- **THEN** the server rejects the request with `E-WEB-010`
- **AND** the system inserts no login attempt row
