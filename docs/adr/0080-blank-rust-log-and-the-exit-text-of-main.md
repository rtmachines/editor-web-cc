# ADR-0080: Blank RUST_LOG and the exit text of main

Status: Accepted
Date: 2026-10-24
Deciders: project owner

## Context

`reliability.md` section 3.1 and ADR-0061 (c) give the default filter `info,sqlx=warn,hyper=warn` for a missing `RUST_LOG`. Step P4.1 writes the filter, and the review found these gaps:

1. No document says what an empty `RUST_LOG`, a text of spaces, or a text of commas gives. An env file with the line `RUST_LOG=` sets an empty text.
2. The parser of `tracing-subscriber` 0.3.23 splits the text at each comma and does not trim a piece. The text `info, debug` gives the target ` debug`, which matches no event.
3. The same parser gives a filter with no directive for a text of commas only. The server then writes no line at all.
4. The filter compares a field value as literal text (ADR-0061 (c)). No document says how an operator writes a string value.
5. For each `Err` of `main`, the std `Termination` writes `Error: ` and the `Debug` text of the error to stderr. ADR-0061 (b) and section 3.1 name only the one `ERROR` line.

## Decision drivers

- The server never starts without logs (R30, R37).
- A bad configuration stops the start with one clear `ERROR` line (ADR-0061 (b), (d)).
- `main` returns `anyhow::Result<()>` (R33, A37, `reliability.md` section 2.3).
- No output holds a secret (`reliability.md` section 1.1).
- Each external input has one grammar and gets a check (R38).

## Considered options

### Option A: The default filter for a text without a directive, and the stderr text of std as a documented fact

- Pros: an empty value of an env file gives the normal production level. A space after a comma has no effect. `main` keeps its return type.
- Cons: an operator who sets an empty text by mistake gets no warning. A failed start gives the error twice: once in the JSON line and once on stderr.

### Option B: A refusal of a text without a directive, and `main` with `ExitCode` and no stderr text

- Pros: a failed start gives exactly one text. An empty value is an explicit fault.
- Cons: an empty line in an env file stops the start. `main` no longer returns `anyhow::Result<()>`, against A37 and section 2.3.

## Decision

The project uses **Option A**.

A `RUST_LOG` without a directive:

- (a) A `RUST_LOG` without a directive gives the default filter of `reliability.md` section 3.3. This item amends ADR-0061 (c).
  - A text has no directive when the variable is missing or empty, or when it holds only spaces and commas.
  - The server splits the text at each comma, trims each piece, and drops the empty pieces. It joins the other pieces with commas before the parse.
  - A bad directive still stops the start. The fallback logger writes the one `ERROR` line (ADR-0061 (d), section 3.1).
  - A value that is not valid UTF-8 also stops the start with one `ERROR` line.

String values in a directive:

- (b) A string field value in a directive needs double quotes, for example `[span{field="text"}]=debug`.
  - The filter compares the `Debug` text of the field value with the text of the directive (`with_regex(false)`, ADR-0061 (c)).
  - The `Debug` text of a string holds the double quotes. A value without quotes therefore matches no string field.
  - A value that parses as a `bool`, a `u64`, an `i64`, or an `f64` matches by value and needs no quotes.

The stderr text of `main`:

- (c) After the JSON `ERROR` line of a failed start or subcommand, the std `Termination` of `main` writes `Error: ` and the error chain to stderr.
  - The text is the `Debug` text of `anyhow::Error`. It holds the `Display` text of each error in the chain, not the `Debug` text of an inner error.
  - When `RUST_BACKTRACE` or `RUST_LIB_BACKTRACE` turns on the capture, the text also holds a backtrace.
  - The text is not a JSON line of section 3.1 and not a log event. The rule of section 1.1 that logs each error exactly once stays true.
  - The text holds no secret, because no error variant carries a secret (section 1.1).
  - The exit code stays 1.

## Consequences

### Positive

- An empty or blank `RUST_LOG` gives the production level and never a server without logs.
- A space after a comma in `RUST_LOG` does not drop a directive.
- The operator knows how to write a field filter for a string value.
- `main` keeps `anyhow::Result<()>`, and no code writes to stderr.

### Negative

- A failed start gives the error on stdout and again on stderr. Docker collects both streams.
- A blank `RUST_LOG` by mistake gives no warning line.

### Risks and mitigations

- Risk: a log reader that expects JSON finds the plain stderr text. Mitigation: the text comes only after the last JSON line, and the process then ends.
- Risk: a later version of `tracing-subscriber` changes the parse of a piece. Mitigation: the unit tests of the filter cover `","`, `" , "`, `"info, debug"`, and `"info, sqlx=warn"`.

## Related

- Amends ADR-0061 (c) (a `RUST_LOG` without a directive). Item (c) adds the stderr text after the `ERROR` line of ADR-0061 (b).
- ADR-0060 (bb) (library target), ADR-0061 (b), (d)
- `docs/architecture/reliability.md` sections 1.1, 2.1, 2.3, 3.1, and 3.3
- Source of `tracing-subscriber` 0.3.23: `src/filter/env/builder.rs` (`Builder::parse`), `src/filter/env/field.rs` (`parse_non_regex`), and the doc of `EnvFilter`
- Source of `anyhow` 1.0.104: `src/fmt.rs`. The std `Termination` of `Result`.
- `overview.md` section 7: R30, R33, R35, R37, R38. Section 9: A37
