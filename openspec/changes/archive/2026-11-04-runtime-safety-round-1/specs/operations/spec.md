## ADDED Requirements

### Requirement: No password in the log of the init script
The `db` init script SHALL turn off three logs in its session before its first statement with a password. They are the statement log, the error statement log, and the slow-statement log. The log of `db` SHALL hold no password of a role that the script creates. This SHALL also hold for a role statement above the slow-statement limit (R34, R37, A1, ADR-0123).

#### Scenario: Slow role statement on the first start
- **GIVEN** an empty volume of the service `db` and a server option that logs every statement with its duration
- **WHEN** the init script creates the login roles and the init ends
- **THEN** the log of `db` holds a duration line
- **AND** the log of `db` holds no line with `PASSWORD` followed by a quote or by `E` and a quote
- **AND** the log of `db` holds no password of a login role

#### Scenario: Order of the settings in the script
- **GIVEN** the text of the init script
- **WHEN** a test reads each statement that holds a password
- **THEN** each such statement comes after the three settings of the session
- **AND** no such statement comes after a change of the database connection

### Requirement: No open transaction in a released connection
The pool of `app` SHALL ask the database for an open transaction when a connection goes back to the pool. The question SHALL use a protocol that shows no open transaction on a healthy connection. The pool SHALL close a connection that holds an open transaction or that cannot answer. The next user of the pool SHALL get a connection without an open transaction (R29, R30, A15, ADR-0124).

#### Scenario: Released connection with an open transaction
- **GIVEN** a pool of `app` with one connection
- **AND** a client that sends `BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY` on that connection and releases it without a `COMMIT` or a `ROLLBACK`
- **WHEN** the client acquires a connection from the pool again
- **THEN** the database shows no open transaction on that connection
- **AND** the connection is not read-only

#### Scenario: Normal release
- **GIVEN** a pool of `app` with one connection
- **WHEN** a client runs a command that commits and then a query, and releases the connection after each of them
- **THEN** each acquire gets the same database session

### Requirement: Stop after the end of an accept loop
The service `app` SHALL wait for the stop signal and for the end of each accept loop at the same time. When an accept loop ends before the stop signal, `app` SHALL run the graceful shutdown and write one `ERROR` line that names the listener. Then `app` SHALL exit with code 70. Docker SHALL restart the container (R30, A41, ADR-0125).

#### Scenario: An accept loop ends before the signal
- **GIVEN** the service `app` after a complete start
- **WHEN** the accept loop of the public listener ends before the stop signal
- **THEN** `app` runs the steps of the graceful shutdown in their order
- **AND** `/readyz` returns `not ready`
- **AND** `app` writes one `ERROR` line that names the public listener
- **AND** `app` exits with code 70
- **AND** Docker restarts the container

#### Scenario: An accept loop panics
- **GIVEN** the service `app` after a complete start
- **WHEN** the accept loop of the internal listener panics
- **THEN** `app` runs the steps of the graceful shutdown in their order
- **AND** the `ERROR` line names the internal listener and tells that the loop panicked
- **AND** the shutdown writes no `WARN` line for that loop
- **AND** `app` exits with code 70

#### Scenario: A start step fails
- **GIVEN** the service `app` during its start
- **WHEN** a start step fails
- **THEN** `app` writes one `ERROR` line
- **AND** `app` exits with code 1
