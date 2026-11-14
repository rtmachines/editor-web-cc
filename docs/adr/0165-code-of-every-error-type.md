# ADR-0165: Code of every error type

Status: Accepted. Amends ADR-0027 (the carriers), ADR-0151 (j3), and ADR-0154 (the carrier `PlantErrors`)
Date: 2026-11-14
Deciders: project owner

## Context

`CLAUDE.md` says that every error type has `code()`, except the marker `AcceptLoopEnded` of `recipe-server`. The rules of `reliability.md` §1.1 state the same rule. Two types do not follow it.

- `PlantErrors` in `crates/recipe-domain/src/plant/errors.rs` implements `std::error::Error` and has no `code()`.
- `reliability.md` §1.1 says that `PlantErrors` owns no code, like `RecipeErrors`, and cites ADR-0154. But `RecipeErrors::code()` exists and returns the code of the first entry.
- `CheckResultFault` in `crates/recipe-app/src/ports/check_results.rs` implements `std::error::Error` and has no `code()`.
- ADR-0151 (j3) and `reliability.md` §1.1 call `CheckResultFault` a reason, like `BackupFault`. No other reason of §1.1 implements `std::error::Error`.

A reviewer that applies `CLAUDE.md` rejects both types. A reviewer that applies the table of §1.1 accepts them.

The registry test `crates/recipe-server/tests/error_registry.rs` keeps `SnapshotItem` in its list `EXCLUDED`. `SnapshotItem::UnknownUnit` returns `E-PLT-020`. The §1.2 row of `E-PLT-020` names only `PlantError::UnitNotFound`, and §1.1 does not list `SnapshotItem`.

## Decision drivers

- One rule for every error type, in `CLAUDE.md` and in `reliability.md` §1.1.
- Each error variant has a stable code that identifies the source (R36).
- A log line of a handled fault identifies the fault (R35).
- The registry test collects the code of every error type that an owner cell of §1.2 names.

## Considered options

### Option A: Every error type gets `code()`

- Pros: one rule without a new exception. The registry test covers `CheckResultFault`. The log line of the fault holds a stable code.
- Cons: two new rows of §1.2 and two new constants, for faults that no HTTP answer shows.

### Option B: `CLAUDE.md` names the two types as exceptions

- Pros: no code changes.
- Cons: the exception list grows. A log line of the fault keeps no code. The owner decided that `CLAUDE.md` wins.

## Decision

The project uses **Option A**.

- (a) Every error type has `code(&self) -> ErrorCode`. The one exception stays the marker `AcceptLoopEnded` of `recipe-server` (ADR-0125 (c)).
- (b) A carrier holds other errors and owns no row of §1.2 of its own. Its `code()` returns the code of an inner error. `RecipeErrors`, `DraftError`, and `PlantErrors` are carriers.
- (c) `PlantErrors::code()` returns the code of the first entry, like `RecipeErrors::code()` (ADR-0027).
- (d) `CheckResultFault` is an error type of `recipe-app` and not a reason. It keeps `std::error::Error` and gets `code()`.
- (e) `CheckResultFault::Unreadable` gets `E-WEB-016`. `CheckResultFault::Malformed` gets `E-WEB-017`.
- (f) The context is `WEB`, because §1.2 gives `WEB` to the HTTP layer and to cross-cutting application results. Both numbers are in the range `E-WEB-001..019`.
- (g) Each of the two rows of §1.2 has the status `-`, like `E-WEB-015`. No HTTP answer carries these codes.
- (h) The job `alerts` writes the `WARN` line of a `CheckResultFault` with the field `error_code`. This line stays the one log of the fault (`reliability.md` §1.1).
- (i) `reliability.md` §1.1 lists `CheckResultFault` in the row of `recipe-app` as an error type. It removes the type from the list of reasons.
- (j) `reliability.md` §1.1 lists `SnapshotItem` in the row of `recipe-domain`. `SnapshotItem::code()` returns the code of its inner error. `SnapshotItem::UnknownUnit` returns `E-PLT-020`.
- (k) The §1.2 row of `E-PLT-020` names `SnapshotItem::UnknownUnit` next to `PlantError::UnitNotFound`. Both variants mean a unit that does not exist on the machine.
- (l) The registry test adds `CheckResultFault` to `SCOPE`. `SnapshotItem` stays in `EXCLUDED`, because its `code()` returns the code of an inner error.

This ADR amends three ADRs:

- ADR-0027: "owns no code" of a carrier means "owns no row of §1.2". The `code()` of the carrier returns an inner code. Item (b) adds `PlantErrors`.
- ADR-0151 (j3): `CheckResultFault` owns the codes of item (e). Two rows of §1.2 come in.
- ADR-0154: the carrier `PlantErrors` of a connection request gets the `code()` of item (c). The design of change `plc-connection-config` names that carrier.

## Consequences

### Positive

- `CLAUDE.md` and `reliability.md` §1.1 give one rule to a reviewer.
- The registry test checks the codes of `CheckResultFault`.
- An operator can find a fault of a result file by its code in the log.

### Negative

- The registry holds two codes that no user sees.
- The row of `E-PLT-020` names two variants. The registry test keeps `SnapshotItem` in `EXCLUDED`.

### Risks and mitigations

- Risk: a later change adds a carrier without `code()`. Mitigation: the review applies items (a) and (b).
- Risk: a later change adds a variant to `CheckResultFault` without a row. Mitigation: the registry test asks for a sample and a row of each variant.

## Related

- ADR-0025 (reasons), ADR-0027 (carriers), ADR-0073 (b), ADR-0125 (c), ADR-0151 (j1), (j3), ADR-0154, ADR-0158 (i)
- `reliability.md` §1.1, §1.2
- Change `review-design-fixes`, design D3
- `overview.md` §7: R35, R36
