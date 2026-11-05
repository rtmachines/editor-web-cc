# ADR-0129: Stage in the log line of a timeout

Status: Accepted
Date: 2026-11-05
Deciders: project owner

## Context

`AppError::Timeout { stage }` owns `E-WEB-009` (`reliability.md` §1.1). Every timeout of the server gives this one code.

- `TimeoutStage` has five values. Their `Display` texts are "password hash permit", "validation", "export", "request", and "diff".
- The values of `argon2.acquire_timeout`, `timeout.validation`, `timeout.export`, and `http.request_timeout` differ (`reliability.md` §7.11).
- `log_error` of `recipe-web` writes the one log line of an error. Its field `chain` holds the `Display` text of each error in the chain (`reliability.md` §1.1).
- The `Display` text of `AppError::Timeout` is the message template of the row `E-WEB-009` of `reliability.md` §1.2. It holds no stage.
- ADR-0128 (w) counts the `E-WEB-009` lines with the cause "password hash permit". It compares that count with the count of `503` answers of the login flood.

The log of the CI run 42037298871 holds the real line. The leg `musl` got 110 answers `503`, and the gate found 0 lines with the cause. Thus the gate of ADR-0128 (w) cannot pass.

An operator has the same problem. A line of `E-WEB-009` does not tell which limit of §7.11 the request reached.

## Decision drivers

- The gate of ADR-0128 (w) reads the cause of each `503` answer from the log.
- An operator finds the stage of a timeout in one log line.
- The operator and the gate read the stage in the same way.
- The user message, the status, and the level of `E-WEB-009` stay unchanged.
- The server logs an error exactly once (`reliability.md` §1.1).
- The log line holds no secret (`reliability.md` §3.4).

## Considered options

### Option A: Append the stage to the field `chain`

- Pros: the change stays in `log_error`. Every error line keeps the same set of fields.
- Cons: a reader finds the stage only with a text search in `chain`. The field `chain` then mixes the display chain with a text that only the log has.

### Option B: A new structured field `stage` in the log line

- Pros: a reader filters the log by the field `stage` without a text search. The operator and the gate read the same field. The field `chain` keeps only the display chain.
- Cons: the log line of `E-WEB-009` gets a field that no other error has. `log_error` writes this field only for `AppError::Timeout`.

### Option C: Add the stage to the `Display` text of `AppError::Timeout`

- Pros: the chain holds the stage with no change of `log_error`.
- Cons: the `Display` text is also the user message. The user message then changes, and the row of §1.2 changes with it.

## Decision

The project uses **Option B**. A reader filters the log by a field without a text search. The operator and the gate of ADR-0128 (w) read the same field.

- (a) The one log line of an `AppError::Timeout` carries the field `stage`. Its value is the `Display` text of `TimeoutStage`.
- (b) The log line of another error has no field `stage`. The field `chain` holds only the display chain and no stage text.
- (c) The user message, the status `503`, the level, and the row `E-WEB-009` of `reliability.md` §1.2 do not change.
- (d) The gate of ADR-0128 (w) counts the JSON lines with `fields.error_code` = `E-WEB-009` and `fields.stage` = `password hash permit`.
- (e) A unit test of `recipe-web` checks the field `stage` for each of the five stages. It also checks that the line of another error has no field `stage`.

## Consequences

### Positive

- The gate of ADR-0128 (w) can count the `503` answers that come from the Argon2 semaphore.
- An operator can filter the log by the field `stage` and tell the five stages apart.
- The field `chain` keeps the same content as before.
- The user sees the same message as before.

### Negative

- The log line of `E-WEB-009` has a field that no other error line has.
- A log reader must accept the field `stage` as optional on an error line.

### Risks and mitigations

- Risk: a change of the `Display` text of `TimeoutStage` breaks the gate of ADR-0128 (w). Mitigation: the unit test of item (e) pins the five texts.

## Related

- ADR-0058 (g), (j) (`AppError::Timeout { stage: Request }` of the request timeout and of the body deadline)
- ADR-0128 (w) (cause of each `503` answer of the login flood)
- ADR-0035 (no `Debug` output in a log line)
- `reliability.md` §1.1, §1.2 row `E-WEB-009`, §3.4, §7.4, and §7.11
- Change `load-and-smoke`, capability `operations`
- `overview.md` §7: R30
