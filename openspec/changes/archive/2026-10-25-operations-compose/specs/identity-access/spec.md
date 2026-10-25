# Spec Delta

## ADDED Requirements

### Requirement: Default trusted proxy
The default trusted-proxy list SHALL hold only the address of the service `proxy` in the Compose network. A request from another address of that network SHALL use its socket address as the client address. The Compose file SHALL set the list and the fixed address of `proxy` from one variable (R38, A22).

#### Scenario: Forwarded header from a neighbor container
- **GIVEN** the default trusted-proxy list and a request from a container of the Compose network that is not `proxy`
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses the socket address of the container as the client address
