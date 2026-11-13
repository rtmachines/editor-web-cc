# Source review of the omitted files (task 1.1)

This file records the evidence of task 1.1. It classifies each tracked production file that the local LLVM report omits.
It is a planning artifact, not a normative document. It creates no entry in `tools/coverage-policy.json`.

## Run facts

The orchestrator recorded these facts of the local measurement:

| Fact | Value |
|---|---|
| Command | `cargo llvm-cov --workspace --all-features --locked --json --output-path <scratch>/llvm-cov-report.json` |
| Result | Exit 0 after 236 s. 9088 tests passed and 0 tests failed. |
| Report size | 27,256,118 bytes |
| Revision | `091216de7e215fa2b7fe20307403dbc1c281e581` on branch `chore/coverage-measurement-integrity` |
| Work tree | Clean, except this change directory |
| Tools | Local cargo-llvm-cov 0.9.1 and rustc 1.99.1. CI pins cargo-llvm-cov 0.6.24. |
| Export | `type` is `llvm.coverage.json.export`. `version` is 3.1.0. |
| Manifest | `cargo_llvm_cov.manifest_path` is `/home/rtm/editor-web-cc/Cargo.toml` |

## Method

1. The review read the tracked files with `git ls-files 'crates/*/src/*.rs' 'crates/*/src/**/*.rs'`.
2. The review made each report path relative to the repository root. No report path is outside the root.
3. The omitted set is the tracked set minus the report set.
4. The review loaded `tools/coverage-gate.py` by its file path under `python3 -I`. It called `workspace_crates` and `source_scan`.
5. `source_scan` applies ADR-0127 (a) through (g). Its excluded set gives the test modules. Its `violations` list was empty.
6. The review read the source of each omitted production file and assigned one class.

## Counts

| Set | Files |
|---|---|
| Tracked `crates/*/src/**/*.rs` | 1334 |
| Report entries (all inside the root, all under `crates/*/src/`) | 654 |
| Report entries with zero lines | 0 |
| Duplicate report paths | 0 |
| Report paths that Git does not track | 0 |
| Omitted tracked files | 680 |
| Omitted test modules (classifier of ADR-0127) | 633 |
| Omitted production files | 47 |
| Reported test modules (the gate removes them, ADR-0127 (h)) | 167 |
| Reported production files | 487 |
| Production files in total | 534 |

The orchestrator estimated 629 test modules by file name. The classifier finds 633. This review uses the classifier result.

Classes of the 47 omitted production files:

| Class | Files |
|---|---|
| Declaration-only | 47 |
| Other configuration | 0 |
| Required (real omission) | 0 |

## Findings

### No required file

Each omitted production file holds no function body, no closure, and no block expression. Thus the report omits no executable line.

### No file of another configuration

No production file compiles only under a cfg that the `--all-features` GNU host build disables.
Each `#[cfg(unix)]`, `#[cfg(not(unix))]`, and `#[cfg(windows)]` attribute of a production file gates an item, not a module.
`--all-features` enables the features `memory`, `test-support`, and `dev`. Thus a module behind one of these features compiles in the measurement.

### Derive and attribute macros make no lines

In the 397 reported files with a `#[derive(...)]` line, 0 of 1039 derive lines have a counted segment.
In the reported files, 0 of 84 `#[async_trait]` lines have a counted segment.
Thus on rustc 1.99.1 a derive and `#[async_trait]` on a trait without default bodies add no line to the report.
This explains why a file with only types and traits has no report entry.

## Ambiguous cases of the declaration-only rule

Design D1 and ADR draft item (d) permit `mod`, `use`, `pub use`, attributes, comments, and "empty items".
"An empty item" is an item without a body or an item with an empty body. "Any other token makes the file required."
A literal application of this text can give a wrong result for 22 of the 47 files.

1. A struct with fields and an enum with variants have a non-empty body. 8 files hold only such types. A literal reading makes them required.
2. A trait with method signatures and no default bodies has a non-empty body. 12 files hold such traits. A literal reading makes them required.
3. A `const` has an initializer after `=`. `plant/limits.rs` uses literals. `ports/setting.rs` and `recipe-db/src/lib.rs` use a path to another const.
4. `recipe-db/src/lib.rs` holds an inline module `statements` with two consts. D1 names `mod` declarations, but not inline module bodies.
5. `#[derive(...)]` and `#[async_trait]` are macros. D1 permits attributes, but a macro attribute can generate executable code.
6. `jobs.rs` and `export/scope.rs` use `#[default]` on an enum variant. `optimistic.rs` has a generic enum.
7. The files use the visibility forms `pub(crate)`, `pub(super)`, and `pub(in crate::recipe)`. Inner attributes `#![forbid(...)]` and `#![deny(...)]` also occur.

The gate fails a correct report with exit 1 when it classifies these 22 files as required.
Thus D1 and ADR draft item (d) need an explicit list of item kinds. The orchestrator decides the text (task 1.2).
A possible list follows:

- `mod` declarations, and inline modules whose content follows these rules,
- `use`, `pub use`, and `extern crate` declarations, with any visibility,
- outer and inner attributes, doc comments, comments, and blank lines,
- `struct`, `enum`, and `union` definitions, and `type` aliases,
- `trait` definitions with no default body for an associated function or constant,
- `const` and `static` items whose initializer is a literal, a path, or an array of literals and paths.

A closure, a block, a call, or a macro call in an initializer makes the file required. A closure body is runtime code.
The report override of D1 stays the safeguard. A macro attribute that generates instrumented lines gives a positive count, and the file becomes required.

Owner decision 2026-11-13: the list above is the rule of D1 and ADR-0162 (d). The two item-level gates are a known limit.

## Limits outside file accounting

- Build-time code: Git tracks `crates/recipe-db/build.rs`. The report does not hold it. It stays outside the runtime metric (D1).
- Test items in production files: ADR-0127 counts test-only items inside a production file as production lines. This limit stays.
- Item-level target gates: `crates/recipe-server/src/disk.rs` line 99 and `crates/recipe-server/src/serve/process.rs` line 170 gate a function with `#[cfg(not(unix))]`.
- The Linux build does not compile these two functions. The report shows no line for them. A file-level policy entry cannot express this case. The owner may accept it as a known limit.

## Report format

The current gate reads these fields: `data` (an array), `files` of each export, `filename`, `summary.lines.count`, and `summary.lines.covered`.
Design D2 adds `cargo_llvm_cov.manifest_path` and the count of export objects in `data`.

| Field | 0.9.1, export 3.1.0 (this run) | 0.6.24, export 2.0.1 |
|---|---|---|
| `data` with one export | Yes | Yes, as the orchestrator reports |
| `files[].filename` | Yes | Yes, as the orchestrator reports |
| `files[].summary.lines.count`, `.covered` | Yes | Yes, as the orchestrator reports |
| `cargo_llvm_cov.manifest_path` | Yes | Not confirmed |

The 0.6.24 fixtures no longer exist (`qualification.md`). Thus this review cannot confirm `cargo_llvm_cov` for 0.6.24.
D2 permits a report without that field. The CI run of task 5 must confirm the root rule on 0.6.24.
Export 3.1.0 also adds `mcdc_records` and `summary.mcdc`. The gate does not read them.
The omitted set comes from 0.9.1. The default test filter of 0.9.1 can differ from 0.6.24 (`follow-up.md`). The CI run of task 5 must confirm the omitted set.

## Table of omitted production files

Abbreviations: "mod" means `mod` declarations, "use" means `use` and `pub use` declarations, and "attr" means attributes.

| Path | Class | Evidence (item kinds) | Notes |
|---|---|---|---|
| `crates/recipe-app/src/audit/mod.rs` | declaration-only | mod, use, attr | `pub(crate) use` |
| `crates/recipe-app/src/audit/queries/mod.rs` | declaration-only | mod, use, attr | |
| `crates/recipe-app/src/auth/mod.rs` | declaration-only | mod, use, attr | `pub(crate) use` |
| `crates/recipe-app/src/error/mod.rs` | declaration-only | mod, use | |
| `crates/recipe-app/src/export/scope.rs` | declaration-only | use, enum, struct, derive | Case 1 and case 6 |
| `crates/recipe-app/src/identity.rs` | declaration-only | mod, use, attr | Feature `memory` cfg |
| `crates/recipe-app/src/identity/queries.rs` | declaration-only | mod, use, attr | |
| `crates/recipe-app/src/jobs.rs` | declaration-only | enum, derive, `pub mod` | Case 1 and case 6 |
| `crates/recipe-app/src/lib.rs` | declaration-only | mod, inner attr | Feature `memory` cfg |
| `crates/recipe-app/src/operations.rs` | declaration-only | mod, use, attr | |
| `crates/recipe-app/src/plant/mod.rs` | declaration-only | mod, use, attr | `pub(crate) use` |
| `crates/recipe-app/src/ports/clock.rs` | declaration-only | use, trait without default bodies | Case 2 |
| `crates/recipe-app/src/ports/document_writer.rs` | declaration-only | use, trait without default bodies | Case 2 |
| `crates/recipe-app/src/ports/draft.rs` | declaration-only | use, `#[async_trait]` trait without default bodies | Case 2 and case 5 |
| `crates/recipe-app/src/ports/isolation.rs` | declaration-only | enum, derive | Case 1 |
| `crates/recipe-app/src/ports/maintenance.rs` | declaration-only | use, `#[async_trait]` trait without default bodies | Case 2 and case 5 |
| `crates/recipe-app/src/ports/mod.rs` | declaration-only | mod, use, attr | Feature `memory` cfg |
| `crates/recipe-app/src/ports/plant.rs` | declaration-only | use, struct, derive, `#[async_trait]` trait | Case 1, 2, and 5 |
| `crates/recipe-app/src/ports/plc_connection.rs` | declaration-only | use, struct, enum, derive, `#[async_trait]` trait | Case 1, 2, and 5 |
| `crates/recipe-app/src/ports/read_only.rs` | declaration-only | trait without default bodies | Case 2 |
| `crates/recipe-app/src/ports/recipe.rs` | declaration-only | use, struct, derive, `#[async_trait]` trait | Case 1, 2, and 5 |
| `crates/recipe-app/src/ports/recipe_reads.rs` | declaration-only | use, struct, derive, `#[async_trait]` trait | Case 1, 2, and 5 |
| `crates/recipe-app/src/ports/security.rs` | declaration-only | use, `#[async_trait]` trait without default bodies | Case 2 and case 5 |
| `crates/recipe-app/src/ports/setting.rs` | declaration-only | use, const with a path, struct, derive, `#[async_trait]` trait, mod | Case 1, 2, 3, and 5 |
| `crates/recipe-app/src/ports/units.rs` | declaration-only | use, `#[async_trait]` trait without default bodies | Case 2 and case 5 |
| `crates/recipe-app/src/recipe/mod.rs` | declaration-only | mod, use, attr | `pub(crate) use` |
| `crates/recipe-db/src/lib.rs` | declaration-only | mod, use, inner attr, inline mod with consts | Case 3 and case 4. Feature `test-support` cfg |
| `crates/recipe-db/src/transaction/identity.rs` | declaration-only | mod | |
| `crates/recipe-domain/src/audit/mod.rs` | declaration-only | mod, use | |
| `crates/recipe-domain/src/export/listing.rs` | declaration-only | use, struct, derive | Case 1 |
| `crates/recipe-domain/src/identity/mod.rs` | declaration-only | mod, use | |
| `crates/recipe-domain/src/lib.rs` | declaration-only | mod, inner attr | |
| `crates/recipe-domain/src/plant/connection.rs` | declaration-only | mod, use | `pub(crate) mod` |
| `crates/recipe-domain/src/plant/connection/fields.rs` | declaration-only | use, struct, derive | Case 1. Derive `Default` |
| `crates/recipe-domain/src/plant/connection/password_plan.rs` | declaration-only | use, enum, derive | Case 1 |
| `crates/recipe-domain/src/plant/limits.rs` | declaration-only | const with a literal | Case 3 |
| `crates/recipe-domain/src/plant/mod.rs` | declaration-only | mod, use, attr | `pub(crate) use` |
| `crates/recipe-domain/src/recipe/limit/mod.rs` | declaration-only | mod, use | `pub(in crate::recipe) use` |
| `crates/recipe-domain/src/recipe/mod.rs` | declaration-only | mod, use | `pub(crate) use` |
| `crates/recipe-domain/src/recipe/optimistic.rs` | declaration-only | generic enum, derive | Case 1 and case 6 |
| `crates/recipe-export/src/lib.rs` | declaration-only | mod, use, inner attr | |
| `crates/recipe-server/src/lib.rs` | declaration-only | mod, inner attr | |
| `crates/recipe-server/src/serve.rs` | declaration-only | mod, use, attr | |
| `crates/recipe-web/src/error/retry.rs` | declaration-only | use, enum, derive | Case 1 |
| `crates/recipe-web/src/lib.rs` | declaration-only | mod, use, inner attr | |
| `crates/recipe-web/src/pages.rs` | declaration-only | mod, use, attr | `pub(crate) mod` |
| `crates/recipe-web/src/pages/approval.rs` | declaration-only | mod, attr | `pub(crate) mod` |

The 25 files without a case number pass even the literal D1 text. The other 22 files need the item list of the section on ambiguous cases.

## Re-run of task 5.2 (2026-11-13)

This section records the re-run of task 5.2. The re-run uses the code after the review changes of task 5.1.

### Commands

| Step | Command |
|---|---|
| Inventory | `python3 -I -B tools/coverage-gate.py --write-inventory target/coverage-inventory.json` |
| Measurement | `cargo llvm-cov --workspace --all-features --locked --json --output-path target/llvm-cov-report.json` |
| Gate | `python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json --inventory target/coverage-inventory.json` |

### Run facts

| Fact | Value |
|---|---|
| Date | 2026-11-13 |
| Local tool | cargo-llvm-cov 0.9.1. CI pins 0.6.24. |
| Measurement | Exit 0. 27 test binaries. 9088 tests passed and 0 tests failed. |
| Report size | 27.3 MB |

The local tool differs from the CI tool. The CI run of the `Coverage` job is the final proof.

### Inventory

| Set | Files |
|---|---|
| Required | 487 |
| `declaration_only` | 47 |
| `test_only` | 800 |
| Untracked files | 0 |

`crates/recipe-db/build.rs` is build-time code. It stays outside the runtime metric.

### Gate result

The gate exits 0. Every crate shows `ok`.

| Crate | Covered | Lines | Percent | Uncovered |
|---|---|---|---|---|
| `recipe-app` | 7964 | 8186 | 97.29 | 222 |
| `recipe-db` | 3408 | 3576 | 95.30 | 168 |
| `recipe-domain` | 9269 | 9372 | 98.90 | 103 |
| `recipe-export` | 142 | 143 | 99.30 | 1 |
| `recipe-server` | 2874 | 2988 | 96.18 | 114 |
| `recipe-web` | 13451 | 13723 | 98.02 | 272 |

The summary lists the required files with uncovered lines. It also lists the 47 declaration-only files.
No crate reaches 100 %. The total uncovered count is 880 lines.
