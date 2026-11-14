# Design

## Context

`proposal.md` gives the motivation. `docs/plan/review/2026-11-13-full-repo-review.md` §4 gives the location, the failure scenario, and the verifier note of each finding. The design checked each finding in the code at HEAD `8db04c7`:

- F-03: `rg ErrorCode crates/recipe-server` prints nothing. `reliability.md` §1.2 states a registry test in `recipe-server`.
- F-04: `rg 'Uuid::new_v4\(\)'` finds the 11 call sites of the review in 7 files of `recipe-app/src/recipe/` and `recipe-app/src/plant/`. Three doc comments of `identity/issue.rs` also name `Uuid::new_v4`. `random_uuid` in `identity/issue.rs` is `pub(super)`.
- F-05: `pager_links` exists in `machines/list.rs`, `recipes.rs`, and `types/usage.rs`. `users.rs` has its own `page_url`, which writes `q` without encoding.
- F-08: `member_input` in `pages/types/define.rs` returns at once for a row with one empty input.
- F-10: `DraftFault` and `draft_lines` have no caller outside tests. `grid.rs` has an inner `expect(dead_code)` with the reason "wave 5".
- F-11: `FieldLines`, `Choice`, and `TokenFields` live in `pages/recipes/` and `pages/users.rs`.
- F-20: `Connections::collect` in `listener.rs` writes `tracing::error!` for a panic.
- F-21: `read_bounded` in `create_admin.rs` calls `read_to_string` before the length test.
- F-23: `is_site_path` in `pages/render.rs` accepts a tab.
- F-30 to F-33: the test code matches the review text.

## Goals / Non-Goals

**Goals:**

- Each fix keeps the rules of `reliability.md` §1 and §2 and `.claude/rules/web.md`.
- Each task starts with a test that fails on the defect, or states why no such test exists.

**Non-Goals:**

- No change of `docs/architecture/` or `docs/adr/`. No new ADR.
- F-09 and F-22. The owner moved both to change B `review-design-fixes`. See "Findings left out".

## Decisions

### D1. Registry test of the error codes (F-03)

- A new integration test `crates/recipe-server/tests/error_registry.rs` with one module per error type family in `tests/error_registry/`.
- The scope is the coded types of `reliability.md` §1.1: `PlantError`, `RecipeError`, `IdentityError`, `ExportError`, `AppError`, `AuditError`, `StorageError`, and `WebError`.
- Each module gives one sample value for each own variant. An exhaustive `match` without a wildcard gives the name `Type::Variant` of a sample. A new variant then stops the build of the test.
- A variant that only passes on the code of an inner error is a delegate, for example `AppError::Storage` or `WebError::Fields`. The `match` names it as a delegate, and it gets no sample.
- The test reads the rows of §1.2 like `recipe-web/src/error/status/tests.rs`. For each row it collects the `Type::Variant` names of the owner cell.
- The test asserts four facts:
  1. `code()` of each sample is in `REGISTRY`.
  2. `code()` of each sample is the code of a row that names its variant. A second variant that reuses a code fails here.
  3. Each pair of a row and an in-scope variant that the row names has a sample of that variant with the code of that row.
  4. Each type that an owner cell of §1.2 names is in the scope or in the list of excluded types. The list holds `SnapshotItem` only.
- `StorageError::Constraint` carries a trigger code, and many rows name it. Fact 3 needs one sample for each of these rows. A missing sample of one trigger code therefore fails the test.
- The scope grows task by task. The first task holds `AuditError`, `StorageError`, `ExportError`, and `IdentityError`. Each later task adds its types and their samples, so each task ends green. Fact 4 comes with the last task, and the full scope holds after it.
- Alternative: a source scan of the `code()` arms, like `recipe-app/tests/error_types.rs`. Rejected, because §1.2 says that the test collects `code()`, and an or-pattern or a delegate arm makes a text scan fragile.

Decision of the owner (2026-11-13): `SnapshotItem::UnknownUnit` (`recipe/snapshot/item.rs`) returns `E-PLT-020`. The §1.2 row of `E-PLT-020` names `SnapshotItem::UnknownUnit` next to `PlantError::UnitNotFound`, and §1.1 lists `SnapshotItem`. The meaning is the same. Only documents change, with an ADR in change B. No new code. D1 leaves `SnapshotItem` out of the scope.

### D2. Random identifiers in recipe-app (F-04)

- `random_uuid` becomes `pub(crate)`. `identity.rs` re-exports it with `pub(crate) use`. Its doc comment names the new callers.
- Each of the 11 sites calls `random_uuid()?`. Each caller already returns `Result<_, AppError>`, so the error is `AppError::TaskFailed { stage: Token }` (`E-WEB-011`), as for the identity commands (`reliability.md` §1.1, ADR-0035).
- In `CreateMachine` and the other plant and recipe commands, a failed read gives the text "The operation token generation did not complete. Try again." The change accepts this text and adds no stage.
- A source test `crates/recipe-app/tests/random_ids.rs` scans `src/` for the call `Uuid::new_v4(`. It skips each line that starts with `//`, so a comment or a doc comment does not count. It skips test files and the module `memory`, like `tests/error_types.rs`.
- Alternative: a `disallowed-methods` entry in `clippy.toml`. Rejected, because the lint set of `reliability.md` §2 is normative, and a new lint is a design decision.

### D3. One pager function (F-05)

- `pager_links` moves from `recipes.rs` to `pages/render.rs`, next to `page_url`. The machine list, the recipe list, the usage list, and the user list call it. The two version tables keep their call and change the import.
- The user list passes the pair `("q", q)` only when `q` is not empty. Its local `page_url` goes away. Its first link comes from `page_url` of `pages/render.rs` with the same pairs. The fields `previous` and `next` of `UsersPager` change from `Option<String>` to `String`, and an empty text means no link, as in `pager_links`. The usage list keeps its link to the first page and its `rev` pair.
- The behavior of a valid request does not change. Each link of the user list keeps `q`, for example `/users?q=ab&page=1`. The grammar of `q` in `users/query.rs` holds only characters that the encoder keeps.
- `ListRequest` of `users/query.rs` holds a `Page` from `path_values::page` in place of its fields `page` and `index`. Today it reads `page` with `text.parse::<u32>()`, which accepts `+2` and `02`. `page=01` and `page=+2` then give `422 E-WEB-008`, as on the other lists. `pager_links` needs a `Page`.

Decision of the owner (2026-11-14): the user list reads `page` with `path_values::page`, like the other lists. A non-canonical page number gives `422 E-WEB-008`.

### D4. Both inputs of a member row (F-08)

- `member_input` checks the name and the type independently. An empty input gives `MemberRows`. A bad name gives `MemberName`. A bad type gives the domain line `E-PLT-013`. A row with two empty inputs stays `None`.
- The lines keep the order name, then type (ADR-0041 (a), ADR-0138 (f)). The new scenario of the `web-ui` delta states the result.

### D5. Dead code in recipe-web (F-10)

- `DraftFault`, `draft_lines`, their re-export in `error.rs`, and `error/draft_lines_tests.rs` go away (YAGNI). The draft forms use other line builders.
- The inner `expect(dead_code)` of `pages/parameters/grid.rs` and the one of `ParameterRowFragment` go away. A non-test build names each dead item.
- An item without a caller goes away. An item that only a test calls moves into the test module or gets `#[cfg(test)]`. No `expect(dead_code)` stays in `recipe-web/src`.

### D6. Shared form module (F-11)

- A new module `pages/form_fields.rs` holds `FieldLines`, `Choice`, and `TokenFields`. Each importer uses the new path. The items keep their names and their code.
- Only the `pub(crate)` `Choice` of `pages/recipes.rs` moves. The private `Choice` of `pages/diff.rs` and of `components_tests/errors.rs` stays.
- The entry of `TokenFields` in `pages/xss_coverage_tests/registry.rs` changes its path from `pages/users.rs` to `pages/form_fields.rs`. `compare.rs` refuses a stale entry.
- A test in `crates/recipe-web/tests/architecture.rs` requires that `pages/form_fields.rs` defines the three `pub(crate)` items.

### D7. One ERROR line for a connection panic (F-20)

- `collect` writes no log line for a panic. The panic hook logs and counts it (ADR-0058 (f), `reliability.md` §2.4 rule 6). The doc comment says so.
- `listener/tests.rs` sets a global subscriber in the same test binary. The new test therefore uses `tracing::subscriber::with_default` and calls `collect` on the test thread. One control event inside the same scope shows that the subscriber captures events.

### D8. Byte limit before the UTF-8 decode (F-21)

- `read_bounded` reads at most `STDIN_READ_LIMIT_BYTES` bytes with `read_to_end`. It tests the length first and decodes with `String::from_utf8` after. Both messages stay.

### D9. Control characters in a site path (F-23)

- `is_site_path` also refuses each ASCII control character, as `same_origin_path` of `lang.rs` does. It keeps the space, because the plan names only control characters.

### D10. Test quality (F-30 to F-33)

- F-30, silent listener: the test measures in the parent process, from the spawn of the child to its exit. That span is at least the budget, also on a slow runner. The floor is a literal 3 s, the value of `watchdog.probe_timeout` in `reliability.md` §7.11. The test does not read the crate constant `PROBE_BUDGET`, so a change of that constant does not move the floor. The ceiling of 4 s goes away. `CHILD_LIMIT` stays the hang guard.
- F-30, accept bound: `a_fake_server_without_a_connection_ends_at_its_accept_bound` keeps its assertion of the error outcome. Its ceiling of 2 s goes away. The test asserts that the panic text of the server thread names the bound of 100 ms. The deadline assertion of `accept_within` ends the thread, so the join cannot hang.
- F-30, twin: `a_fake_server_without_a_connection_ends_at_its_accept_bound` in `crates/recipe-server/src/probe/tests.rs` has the same ceiling of 2 s. It gets the same change. Its ceiling of 2 s goes away. It asserts that the panic text of the server thread names the bound of 100 ms. `serve_once_within` calls `accept_within`, which panics with "no connection in {accept_wait:?}".

  Decision of the owner (2026-11-14): the twin test gets the same change as the test in `tests/healthcheck.rs`.
- F-30, constant: `defaults_equal_the_settings_table` in `watchdog/tests.rs` already asserts that `PROBE_BUDGET` equals `watchdog.probe_timeout` of §7.11. The change adds no unit test for it.
- F-31: the closure of the work owns the sender of a `tokio::sync::oneshot` channel. The receiver gives `Err` when the closure drops without a run. A timeout of 5 s guards the wait.
- F-32: both positive tests assert `min()` and `max()` of the parsed limit.
- F-33: the `after` body uses the parameters of `example_parameters` with one changed bound. Only the limits differ.

## Findings left out

- F-09: `api.md` §7.1 gives the cookie the attribute `Secure` "yes" and the name prefix `__Host-`. A browser refuses a `__Host-` cookie without `Secure`. `api.md` §7.3 and §8 give the pre-session cookie and `recipe_lang` the attribute `Secure` too. No document says what `session.secure=false` changes in a `dev` build. A fix needs a decision.
  Decision of the owner (2026-11-13): change B `review-design-fixes` writes a new ADR. The ADR states what `session.secure=false` does with each cookie, also with the `__Host-` prefix. Change B then changes the code.
- F-22: a mark for a call that did not reach the limit contradicts ADR-0133 (c), (d). A timer task that marks after the limit is a new catch place of `reliability.md` §2.4 rule 6. Both need an ADR. The only caller that drops the future is the drain of the supervisor at the stop.
  Decision of the owner (2026-11-13): change B decides F-22 together with F-06 and F-07, because all three are cancellation-safety findings. The design of change B chooses a fix or a known limit.

## Risks / Trade-offs

- [D1 samples] A variant can hold a type that a test outside its crate cannot build. → The task stops and reports the variant. It adds no public constructor.
- [D10 F-30] The parent span includes the start of the child, so a probe that ends a little early can pass. → The unit tests of `probe` keep the floor `elapsed >= budget`.
- [Tasks] Several tasks edit one file. → `tasks.md` gives the order of these tasks and names the shared files.
- [D5] A non-test build can name an item that a coming change needs. → The change of that item adds it back with its caller.
