# ADR-0042: Snapshot reads of draft queries

Status: Accepted
Date: 2026-10-15
Deciders: project owner

## Context

ADR-0015 gives the isolation of each command: `READ COMMITTED`, and `SERIALIZABLE` for `SetUserRole` and `DeactivateUser`. No document gives the isolation of a query.

The queries of step P3.2 use `Isolation::ReadOnly`. `recipe-db` begins it with `BEGIN ISOLATION LEVEL READ COMMITTED READ ONLY`. Each statement of such a transaction sees the rows of the moment when it begins.

A version page renders the `row_version` of the draft into every form (`api.md` section 4). The query reads the version row and the parameter rows with more than one statement. A draft command can commit between two statements. Then the page can hold a new `row_version` with old parameter data. A save from that page passes the `row_version` check and overwrites a change that the user never saw.

PostgreSQL never raises a serialization failure (`40001`) in a read-only transaction with `REPEATABLE READ`. All its statements see one snapshot.

## Decision drivers

- The `row_version` of a page belongs to the data of the same page (ADR-0008).
- A query does not retry.
- A query holds no row lock, so it never blocks a command.

## Considered options

### Option A: One snapshot for a draft query

- Pros: the data and the `row_version` always match. A query never fails because of the isolation.
- Cons: a new isolation value in `recipe-app` and `recipe-db`.

### Option B: `READ COMMITTED` with the `row_version` read first

- Pros: no new isolation value.
- Cons: each query must keep a statement order. A stale page then gives a false `409`.

### Option C: No change

- Pros: no work.
- Cons: a save can overwrite a change that the user never saw.

## Decision

The project uses **Option A**.

- (a) `recipe-app` gets the isolation value `Isolation::RepeatableRead`. `recipe-db` begins it with `BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY`. The name avoids the word "snapshot", which already names the snapshot check and the version (`domain-model.md` section 7).
- (b) The queries `GetVersion`, `ListParameters`, `GetParameter`, `GetParameterForm`, `GetLimits`, and `GetTranslations` read in one `RepeatableRead` transaction. Each of them returns a `row_version` together with data of the version.
- (c) Every other query keeps `Isolation::ReadOnly`.
- (d) A `RepeatableRead` transaction does not retry. A failure maps to its `StorageError` as in `reliability.md` section 1.1.
- (e) The memory adapter treats `RepeatableRead` as `ReadOnly`, because it reads one locked state.
- (f) A query ends the transaction before it resolves a type tree or builds a view (`reliability.md` section 7.4).

## Consequences

### Positive

- A page never shows a `row_version` that is newer than its data.
- The rule needs no statement order inside a query.

### Negative

- One more isolation value to test in `recipe-db`.

### Risks and mitigations

- Risk: a long snapshot keeps old row versions in PostgreSQL. Mitigation: `db.statement_timeout` and `db.idle_in_transaction_timeout` limit the time of a query transaction (`reliability.md` section 7.11).

## Related

- ADR-0008, ADR-0015, ADR-0028
- `api.md` section 4
- `reliability.md` sections 1.1, 7.4, 7.11
- `data-model.md` section 6
- PostgreSQL 16 documentation, section 13.2.2 "Repeatable Read Isolation Level"
