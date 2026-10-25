# Rust rules

Source: `docs/architecture/reliability.md` §1, §2, §8. Load the skills `idiomatic-rust` and `rust-errors` before you write Rust.

## Lints and panic policy (reliability §2)

- Every crate has `#![forbid(unsafe_code)]` and `[lints] workspace = true`. No `static mut`. No global mutable state.
- `[workspace.lints]` sets these clippy lints to `deny`: `unwrap_used`, `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`.
- It also denies `indexing_slicing`, `arithmetic_side_effects`, the four `cast_*` lints of §2.1, `exit`, `print_stdout`, and `print_stderr`.
- `unused_must_use` is `deny`. `clippy::large_futures` is `warn`.
- `[profile.release]` keeps `overflow-checks = true` and `panic = "unwind"`.
- Integer arithmetic uses `checked_*` and returns `Err` on `None`. Use `saturating_*` only for a counter that the UI shows.
- Integer conversion uses `try_from`. Never use `as` between integer types.
- Access uses `get`, `first`, `last`, `entry`, and iterators. Never write `v[i]`, `map[&key]`, or `s[a..b]`.
- An `Option` ends in `let Some(x) = opt else { return Err(..) };` or in `opt.ok_or(..)?`.
- A regex is a literal in a `static` of type `LazyLock<Result<Regex, regex::Error>>`. The caller maps `Err` with `map_err`.
- A unit test compiles every regex pattern, so a bad pattern fails the build.
- `parking_lot::Mutex` replaces `std::sync::Mutex`. Never call `block_on` inside async code.
- Argon2id hashing and snapshot checks run in `spawn_blocking`. Map the `JoinError` to `AppError::TaskFailed { stage }`.
- Application code never calls `catch_unwind`. §2.4 rule 6 names the six places that catch a panic (ADR-0057 (h), ADR-0058 (f), ADR-0092).
- The places are the inner `CatchPanicLayer`, the backstop `CatchPanicLayer`, and the supervisor of the jobs.
- The next two places are the join of a `spawn_blocking` task and the `JoinSet` of the connection tasks.
- The sixth place is the `JoinSet` of the webhook calls of one run of `alerts`.

Permitted exceptions (reliability §2.3):

- Test code (`#[cfg(test)]`, `tests/`) may use `unwrap`, `expect`, and indexing.
- The watchdog function of §2.4 is the only item with `#[allow(clippy::exit)]`. A comment on the attribute names §2.3.
- No `expect` in `main`. `main` returns `anyhow::Result<()>`. A failed startup check writes one `ERROR` line and exits with code 1.
- A review rejects every other `#[allow]` of these lints outside tests.

## Errors (reliability §1)

- `recipe-domain` has one `thiserror` enum per bounded context: `PlantError`, `RecipeError`, `IdentityError`, `ExportError`.
- Every error type has `fn code(&self) -> ErrorCode`. The code is stable. The message template can change.
- Add the row to `reliability.md` §1.2 first. Then add the variant and its row in `crates/recipe-domain/src/error_codes.rs`.
- A variant carries the values that identify the failure, such as `reference`, `number`, `tag`, and `leaf`. It never carries a secret.
- `?` converts between layers through `From`. Use `map_err` only when it adds context.
- Lower layers add context and propagate. Only the `WebError` mapping, the backstop panic layer, and the supervisor of the jobs log an error. A job logs a handled fault once at `WARN` (reliability §1.1).
- `Debug` on a type that holds a secret redacts it. Write that `Debug` by hand.

## Types and naming (skill `idiomatic-rust`, `NAMING.md`)

- Every identifier is a newtype, for example `MachineId`, `UnitId`, `PlcTypeId`, `RecipeId`, `RecipeVersionId`, `UserId`, `ParameterKey`.
- A bare `Uuid`, `String`, or integer never stands for a domain identifier in a signature.
- A value object has one `parse(input) -> Result<Self, Error>` and no public `new`. Code does not check it again (reliability §4.3).
- Use the Rust names of `domain-model.md`. `data-model.md` §1.2 maps them to SQL names.
- A new name has no weasel word: `Service`, `Manager`, `Helper`, `Util`, `Data`, `Info`.
- `as_` is a free view, `to_` allocates, `into_` consumes. A field accessor has no `get_` prefix.
- Acronyms are words: `HttpClient`, `Uuid`, `Json`.
- Implement `From` or `TryFrom` instead of an ad hoc conversion function. Prefer derives over hand-written impls.
- A helper that does not need `Self` is a free function, not an associated function.
- A design pattern names the problem that it solves in its doc comment (R43).
- Every public item of a library crate has a doc comment. `cargo doc --no-deps` runs with `-D warnings`.
