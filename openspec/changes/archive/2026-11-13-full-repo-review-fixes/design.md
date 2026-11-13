# Design

## Context

The review of 2026-11-11 reports R-01 to R-06. `proposal.md` gives the list. The rules come from these sources. `reliability.md` §2 gives the arithmetic and panic policy. ADR-0133 and ADR-0151 give the time limit of a read. ADR-0144 gives the order of the class-2 locks. ADR-0154 gives the PLC connection and `SecretField`. A24 gives the read-only mode.

Facts on `main` at 32fc55f:

- `limited_read.rs` counts the marks of a path in `HashMap<PathBuf, usize>`. `Call::finish` and `Call::mark_if_running` change the count with `saturating_sub` and `saturating_add`. `finish` runs in `Drop` of `FinishOnDrop`, so it must not panic.
- `credential_cipher.rs` computes the capacity of the sealed buffer with `saturating_add`. `create_admin.rs` and `check_results.rs` compute "limit plus one" with `saturating_add` on a constant.
- `SecretField` is in `pages/machines/connection_request.rs`. Its test file is a sibling module (`machines.rs:142`), so it cannot read the private field.
- Three adapters call `read_with_limit`, and each has its own map of stalled paths (ADR-0151 (o)). `SystemDiskSpace` measures the free space. `FileBackupDirectory` scans the backup directory. `FileCheckResults` reads the two result files.
- The jobs `disk_check` and `alerts` share `SystemDiskSpace`. `alerts` also uses the two other adapters (`serve/jobs.rs`). It measures its three volumes one after the other and waits for each.
- The supervisor starts the next run of a job only after the end of the last run (`supervisor.rs`, module doc).
- `read_with_limit` checks the mark before it starts a call. The mark comes only when a call reaches the limit. A call of `disk_check` and a call of `alerts` can therefore both stall on `pgdata`.
- The trait `TypeCatalog` has two adapters of `lock_derived_types`. The PostgreSQL adapter computes, removes duplicates, and sorts the key values (`type_rows.rs:46`). The memory adapter locks in the given order (`memory/types/mod.rs:103`). ADR-0144 (d) permits this.

## Goals / Non-Goals

**Goals:**

- No integer `saturating_add` or `saturating_sub` in the production code of `recipe-server`. `Instant::saturating_duration_since` in `watchdog.rs` stays, because it is not integer arithmetic.
- A build that fails when the field of `SecretField` loses `ZeroizeOnDrop`.
- One true statement of the bound of stalled calls in ADR-0161, `reliability.md` §7.8 and §8, and the `operations` spec. A test in `recipe-app` holds the premise of the bound.
- No file above 800 lines and no function above 50 lines in the files of R-04.
- Doc comments and the runbook that agree with ADR-0144 and A24.

**Non-Goals:**

- A shared read of one path for several jobs (option (b) of R-03).
- A change of behavior, of a route, of a setting, or of an error code.
- The `saturating_*` calls in `recipe-app`, `recipe-domain`, and `recipe-web`. The review did not report them.
- A permanent guard test against `saturating_*`. A later change can add a lint script.

## Decisions

### D1. Marks of a path: checked arithmetic with a defined result (R-01)

- `mark_if_running` uses `checked_add(1)` and returns `Result<(), LimitFault>`. On `None` it keeps the count, sets `marked` back to `false`, and returns the new fault `LimitFault::MarkOverflow`. `read_with_limit` returns that fault in place of `TimedOut`. `into_io_error` maps it to an I/O error of the kind `TimedOut`, so each adapter gives its read fault.
- `finish` uses `checked_sub(1)`. A marked call always has its mark in the count, so `None` cannot occur. On `None`, `finish` removes the entry, like a count of zero. `Drop` stays free of a panic. `finish` cannot return `Err`. ADR-0161 (i) records it as the one exception to the rule "Return `Err` on `None`" of `reliability.md` §2.2.
- Alternative: a set of call identifiers in place of the count. Rejected, because an identifier needs its own counter or an address of an `Arc`.
- The overflow is not reachable in production. D5 gives the bound.
- A unit test sets the count to `usize::MAX`. `mark_if_running` must give `MarkOverflow`, the count must stay `usize::MAX`, and `finish` must leave it there. The test module is a child of `limited_read`, so it can build a `Call`.

### D2. Sealed buffer without arithmetic (R-01)

`seal` builds the buffer with `[[VERSION].as_slice(), nonce_bytes.as_slice(), encrypted.as_slice()].concat()`. The standard library computes the capacity. The form has no slice index, so no review asks about `indexing_slicing`. The bytes stay the same. The test `sealed_form_has_version_nonce_ciphertext_and_tag` of `credential_cipher/tests.rs` holds the format.

### D3. "Limit plus one" as a constant (R-01)

- `create_admin.rs` gets `const STDIN_READ_LIMIT: u64 = STDIN_LIMIT_BYTES + 1;`. `check_results.rs` gets `const READ_LIMIT_BYTES: u64 = MAX_RESULT_BYTES + 1;`.
- An overflow in a constant fails the build. `HEADER_LENGTH` of `credential_cipher.rs` uses this form, and clippy with the workspace lints accepts it.
- Each new constant gets a doc comment. The comment of `READ_LIMIT_BYTES` cites `reliability.md` §7.11, key `check_result.max_bytes`, like `MAX_RESULT_BYTES`.
- The size-limit tests of `create_admin/tests.rs` and `check_results/tests.rs` hold the behavior.

### D4. Drop contract of `SecretField` (R-02)

- `connection_request.rs` gets a compile-time assertion beside the type: `const _: fn(&SecretField) -> &dyn zeroize::ZeroizeOnDrop = |field| &field.0;`. A comment above it names the problem that it solves.
- A plain `String` field does not implement `ZeroizeOnDrop`. The build of the crate then fails with `E0277` in every profile. The check reads no freed memory.
- The test `a_secret_field_clears_its_buffer_on_drop` loses its `needs_drop` check. The test goes away, because the assertion replaces it.
- Red step: change the field to `String` and the constructor to `Self(text)`. Then `cargo build -p recipe-web` must fail with `E0277` on `String: ZeroizeOnDrop`, and with no other error. Restore both lines.
- Alternative: a `#[cfg(test)] pub(super)` function that returns `&impl ZeroizeOnDrop`. Rejected, because it adds an API only for a test and checks only the test build.
- Alternative: `#[derive(ZeroizeOnDrop)]`. Rejected, because it needs the feature `derive` of `zeroize` and one more proc-macro crate.

### D5. Bound of stalled calls (R-03, option (a))

New ADR-0161 records the decision. It amends the consequence "at most one blocking thread per path" of ADR-0133. It cites ADR-0151 (o), (p) in its section Related.

- A path holds at most one stalled call for each pair of a job and an adapter that reads the path.
- The bound holds for three reasons. Each job waits for each of its reads, so a read that reaches the limit marks its path. One run of a job reads one path at most once at a time through one adapter. No two runs of one job overlap.
- Each adapter has one set of stalled paths for the life of the process. `serve/jobs.rs` builds each adapter once. A second call of `Host::backups` or `Host::check_results` makes a new adapter and therefore a new pair.
- The totals per volume:

| Volume | Stalled calls | Pairs |
|---|---|---|
| `pgdata` | 2 | `disk_check` and `alerts` through `SystemDiskSpace` |
| `backups` directory | 2 | `alerts` through `SystemDiskSpace` and through `FileBackupDirectory` |
| `backups/restore-test.result` | 1 | `alerts` through `FileCheckResults` |
| `walarchive` | 1 | `alerts` through `SystemDiskSpace` |
| `walarchive/base-backup.result` | 1 | `alerts` through `FileCheckResults` |

- All paths together hold at most 7 blocked threads. This number is far below the blocking pool of Tokio.
- A new pair raises the bound by one. A new pair is a new job that reads a path, or a new adapter that reads a path. ADR-0161 says so, so the next change sees it.
- ADR-0133 gets the Status line "Accepted. Amends ADR-0073 (d). Amended by ADR-0161 (bound of the stalled calls of a path)". Its text does not change. Its row in the index of `docs/adr/README.md` gets the same Status.
- The decision map lists ADR-0161 in the row of the read-only banner and in the row of deployment, backup, and health.
- `reliability.md` §7.8 gets one line with the bound. §8 gets one sentence in the item of the maps of stalled paths. Both cite ADR-0161.
- The spec delta states the bound in both requirements of the time limit of a read.
- A test in `crates/recipe-app/src/jobs/alerts/tests/disks.rs` holds the second premise. A disk double counts the calls in flight on each path. The test asserts a peak of 1 on each path during one run of `alerts`.
- The doc comments of `Alerts::low_disk` and of `StalledPaths` cite ADR-0161.
- The supervisor tests hold the third premise through `overlaps()` (`supervisor/support.rs`).

### D6. Split of `SetPlcConnection::execute` (R-04)

- A free function `lock_for_write` takes the transaction, the machine, the code, and the expected `row_version`. It locks `machine`, checks that the machine is live, locks `plc_connection`, and compares the `row_version`.
- It returns `Result<Option<Locked<StoredConnection>>, AppError>`. `None` means the first save of the machine.
- `execute` keeps the order: authorize, read, plan, seal, start the transaction, `lock_for_write`, write, commit. The lock order of ADR-0015 and the one transaction stay. An early return still rolls the transaction back.
- A free function follows the rule "a helper that does not need `Self` is a free function".
- The line count goes from the line of `pub async fn execute` to its closing brace. The result is about 49 lines. If it is above 50, the build of `SetPlcConnectionOutput` also moves into a free function.

### D7. Split of the test files (R-04)

- Four files hold test scenarios: `set_plc_connection/tests.rs`, `storage/plc_connection.rs`, `storage/constraints/plc_connection.rs`, and `deploy_values/file_mutations.rs`. Each moves into modules by scenario group. The test bodies do not change.
- `deploy_values/files.rs` holds public check functions and tables, and only two tests. `file_mutations.rs` and `inputs.rs` import from it. It splits by the family of the checked file: env, Caddy, crontab, and pgtools scripts. `files.rs` keeps `pub use` lines, so the importers do not change.
- The split of `files.rs` runs before the split of `file_mutations.rs`, because both touch the same imports.
- The layout follows the sibling files of each folder. A new module file needs no `#![allow]` line, because it takes the line of `storage/main.rs` or `deploy_values.rs`.
- The number of tests stays the same. `cargo test -p <crate> --test <target> -- --list` shows the same count before and after. `git diff --color-moved=zebra` shows only moved lines.

### D8. Text fixes (R-05, R-06)

- R-05: the doc comment of `lock_derived_types` names the work of each adapter. The PostgreSQL adapter computes the key values, removes duplicate values, sorts them, and locks in that order (ADR-0144 (a) to (c)). The memory adapter locks in the given order (ADR-0144 (d)). `recipe-app` collects the keys without duplicate texts through `derived_type_locks`.
- R-06: the runbook sentence at `runbook.md:2397` becomes "`Login`, `Logout`, every query, every export, and `ExportPlcConnection` continue (A24 in `overview.md` 9)". This is the list and the verb of `reliability.md` §7.8.
- `CHANGELOG.md` gets one line under `Changed`: the bound of stalled reads per path (ADR-0161).

## Risks / Trade-offs

- [The bound of D5 depends on the job structure] → ADR-0161 names the three premises and the effect of a new pair. A test holds each premise.
- [D1 keeps a value that cannot occur] → The unit test fixes the defined result. A later edit cannot change it without a red test.
- [A test split can lose a test] → D7 compares the counts of `--list` and shows the moved lines.
- [The assertion of D4 is code that only the compiler uses] → It has no run-time cost. A comment names its problem.
