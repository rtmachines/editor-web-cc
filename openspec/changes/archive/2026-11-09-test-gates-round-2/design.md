# Design

## Context

See `proposal.md`, section "Why". ADR-0153 holds the one new decision. ADR-0062 (b), ADR-0094, ADR-0127 (n) to (u), ADR-0136, ADR-0048 (c), ADR-0128 (ai), and `reliability.md` §6 hold the context. This design links to them and does not repeat them.

The table shows the state of the code on `main` at `943f5e6`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| Secure refusal test | `serve_secure_false_is_refused_in_a_build_without_the_dev_feature` has `#[cfg(not(feature = "dev"))]` | `crates/recipe-server/src/config/serve/tests.rs` line 346 |
| Literal refusal test | `serve_secure_false_stops_the_start_without_the_dev_build` calls `check_secure` with literal flags in every build | `config/serve/tests.rs` lines 325 to 338 |
| Build flag | `const DEV_BUILD: bool = cfg!(feature = "dev");` | `crates/recipe-server/src/config/serve.rs` line 52 |
| Feature `dev` | `dev = []`. The feature changes only `recipe-server` | `crates/recipe-server/Cargo.toml` line 53 |
| Secure step | `rust-tests.sh secure` runs `cargo test -p recipe-server --no-default-features --lib` on one test and expects `1 passed` | `deploy/ci/rust-tests.sh` lines 33 to 45 |
| Jobs that run the step | Only `compose` (`if: github.event_name != 'pull_request'`). Its three last steps before the diagnostics install the toolchain, restore the cache `compose-config`, and run the step | `.github/workflows/ci.yml` lines 42, 139 to 147 |
| Job `test` | `cargo test --workspace --all-features --locked`. `--all-features` enables `dev`, so the build omits the refusal test | `ci.yml` lines 715 to 729 |
| Structure checks of `ci.yml` | `check.sh` reads `ci.yml` as lines and checks the jobs `migration-freeze`, `ci-scripts`, `load`, and `compose` | `deploy/ci/check.sh` lines 108 to 320 |
| Domain state of the rewrite check | `check_files` makes a new set `domains` for each file. `CREATE DOMAIN` adds a name. `DROP DOMAIN` removes it. The set holds no constraints | `deploy/ci/migration_rewrite.py` lines 564 to 570, 585, 586 |
| Domain forms | `other_forms` reads `ALTER DOMAIN`. It moves a renamed domain and gives the two domain forms of ADR-0136 | `migration_rewrite.py` lines 521 to 537 |
| Added column | `added_forms` reads only inline constraints. `volatile_default` reads `serial*`, identity columns, and the `DEFAULT`. No function reads the column type | `migration_rewrite.py` lines 271 to 362 |
| Table exemption | `alter_table` gives no form for a table in `created` | `migration_rewrite.py` line 397 |
| Body forms | `body_forms` checks each body statement with new copies of the state. No body statement sees the state of an earlier body statement | `migration_rewrite.py` lines 449 to 463 |
| Volatile rule | `volatile_default` calls a function volatile unless `NON_VOLATILE` holds its name | `migration_rewrite.py` lines 38, 271 to 313 |
| Tests of the forms | `test_migration_rewrite_forms.py` holds the ADR-0136 forms and imports the helpers of `test_migration_rewrite.py`. The job `migration-rewrite` runs both files | `deploy/ci/test_migration_rewrite_forms.py`, `ci.yml` lines 837 to 852 |
| Shipped migrations | `0001` to `0011`. No file creates a domain | `migrations/` |
| Login corpus test | `xss_corpus_login_field_of_post_login_is_escaped` sends the fixed `password=wrong` | `crates/recipe-web/src/pages/xss_tests.rs` line 206 |
| Password structs | `LoginFields { login, password, csrf_pre, csrf_token }` and `PasswordFields { current_password, password, csrf_token }` | `crates/recipe-web/src/pages/own.rs` lines 46 to 69 |
| Request structs | 36 derives of `Deserialize` and 36 attributes `#[serde(deny_unknown_fields)]` in the production files of `recipe-web/src`. Several fields carry `rename`. No item carries `rename_all`, `flatten`, or `alias` | `crates/recipe-web/src/**` |
| Pair readers | `read_pairs` and two handlers with `Form<Vec<(String, String)>>` read keys that no struct names | `pages/parameters/fields.rs` line 227, `pages/types/define.rs` lines 350, 367 |
| Source walk | `files_below` and `is_test_path` serve the guard tests that read the crate | `crates/recipe-web/src/test_support/source_walk.rs` |
| Smoke export step | Asserts `loadable`, the hash format, and the rebuilt hash only. `PARAMETER` holds `SPEED`, `INT`, `50`, `0`, `100` | `e2e/compose/end-to-end.spec.ts` lines 23, 187 to 206 |
| Pure helper test | `jcs.spec.ts` tests `compose/support/jcs.ts` without a server against the golden file of `recipe-export` | `e2e/tests/jcs.spec.ts` |
| Export golden file | `M12`, `R-100`, version `3`, parameter `SPEED` with `INT`, value `120`, one limit `10` to `250` on leaf `""` | `crates/recipe-export/tests/golden/recipe_R-100_v3.json` |

## Goals / Non-Goals

**Goals:**

- Each finding of `docs/plan/2026-11-08-fix-plan.md` section 7 has one decision below.
- Each new gate fails on the defect of its finding before the fix of the gate.
- No production code changes.

**Non-Goals:**

- A change of `check_secure` or of the feature `dev`.
- A change of a template, a handler, or a request struct of `recipe-web`.
- A change of the export document or of `recipe-export`.
- A run of the job `compose` on a pull request (ADR-0094).
- The CI step of the ADR index script (plan section 3.1 "Follow-up"). The owner decided Q3: the step is not part of WP4.

## Decisions

### D1: One spec delta in `operations` (brief decision 1)

Two requirements of `openspec/specs/operations/spec.md` state a gate that this change changes:

- "CI rejects a migration that locks or rewrites an existing table" lists the rejected forms. The new form gets its own ADDED requirement "CI rejects an added column of a constrained domain". The exemption rule of the new form differs from the sentence "An object that the same file creates ... SHALL pass". An ADDED requirement states that rule without a rewrite of the old block.
- "End-to-end smoke on the Compose stack" says that the suite checks the content hash. The MODIFIED block also requires a check of the content. Its scenario "Flow of the smoke suite" names the checked fields.

No requirement names the CI job of the secure refusal (D6-1). The requirement "Session cookie" of `identity-access` states the refusal itself, and that behavior stays. So D6-1 needs no delta.

No requirement names the XSS corpus (D6-3). The requirement "Escaped user text" of `web-ui` states the behavior, and that behavior stays. The corpus rule lives in `.claude/rules/web.md` and `reliability.md` §6 row "HTTP". So D6-3 needs no delta.

`openspec validate test-gates-round-2 --strict` accepts the delta.

### D2: `data-model.md` §8 lists the forms (brief decision 2)

`data-model.md` §8 lists each rejected form and the two exemption sentences. This change therefore edits §8 together with ADR-0153. The new bullet names the form. The bullet of the volatile default gets the domain default of ADR-0153 (i). One new sentence says that a domain of the same file gives no exemption for `ADD COLUMN`.

`reliability.md` §5 links to `data-model.md` §8 and lists no form. It needs no edit. The docstring of `migration_rewrite.py` lists the forms. Task 2.3 updates it.

### D3: Rule of the new form (brief decision 3)

ADR-0153 (a) to (i) give the rule. This section maps the rule to the parser.

- Two new sets of type `set[Table]` live in `check_files` beside `tables`: `constrained` and `volatile_domains`. They persist over the files.
- The two sets travel together as one more argument of `check_statement`, `other_forms`, `alter_table`, `forbidden_forms`, and `body_forms`.
- `CREATE DOMAIN name [AS] base ...` adds `name` to `constrained` when `outer_tokens` of the rest hold `check`, or `not null` that does not follow `is`. It also adds `name` when `base` resolves to a member of `constrained`.
- `CREATE DOMAIN` adds `name` to `volatile_domains` when its `DEFAULT` expression is volatile under the rule of `volatile_default`. Without its own `DEFAULT`, it adds `name` when `base` is a member of `volatile_domains`.
- `ALTER DOMAIN` adds the domain to `constrained` on `ADD [CONSTRAINT n] CHECK` and on `SET NOT NULL`. It adds the domain to `volatile_domains` on a volatile `SET DEFAULT`.
- `ALTER DOMAIN` and `ALTER TYPE` with `RENAME TO` or `SET SCHEMA` move the name in both sets. `DROP DOMAIN` removes the name from both sets.
- `ALTER TYPE` does not move a name of the per-file set `domains`. The behavior of ADR-0136 for that set stays.
- `forbidden_forms` gets the form `add column of a constrained domain` when an `ADD` action is a column whose type resolves to a member of `constrained`.
- `forbidden_forms` gets the form `volatile default` when such a column has no `DEFAULT` of its own and its type resolves to a member of `volatile_domains`.
- The type is the token after the column name, with an optional `schema .` prefix. `table_name` resolves it, so an unqualified name maps to `public`.
- `alter_table` already gives no form for a table in `created`. The new forms thus inherit the table exemption without new code. The per-file set `domains` does not take part in the new forms.
- `body_forms` copies `constrained` and `volatile_domains` once for each body. It passes these copies to each statement of that body and to each `EXECUTE` string in it.
- `body_forms` keeps the copies of `tables`, `created`, and `domains` for each statement. A body statement thus never changes the state of the file.

The owner decided Q1 on 2026-11-09. A constrained domain that the same file creates gives no exemption for the new form. Only a table that the same file creates stays exempt. The domain exemption of ADR-0136 (b) stays for the two domain forms only. The plan sentence of section 7.2 "keeps the exemption of ADR-0136 (b)" thus means the domain forms only.

The new tests go into `test_migration_rewrite_forms.py`, not into `test_migration_rewrite.py` as the plan says. The forms file holds each form of ADR-0136, and the job `migration-rewrite` runs both files. `test_migration_rewrite.py` has 680 lines, so new cases there move it near the limit of 800 lines.

Alternatives: Option B and Option C of ADR-0153.

### D4: Guard of the corpus coverage (brief decision 4)

A new test file `crates/recipe-web/src/pages/xss_coverage_tests.rs` holds one guard test. `pages.rs` declares it under `#[cfg(test)]`.

- The test walks `src/` with `test_support::source_walk::files_below`. It skips each file for which `is_test_path` is true.
- In each file, it finds each `#[derive(...)]` attribute that names `Deserialize`. It reads the next item and each field up to the closing brace.
- The test fails when that item is not a `struct` with named fields. It also fails when the count of parsed structs differs from the count of `Deserialize` derives in the text.
- Each scanned struct must carry `deny_unknown_fields` in a `#[serde(...)]` attribute. The attribute can hold more items, for example `#[serde(deny_unknown_fields, rename = "x")]`.
- The test fails on `rename_all`, `flatten`, or `alias` in a `#[serde(...)]` attribute of a scanned struct or of its fields. Each of them changes the wire names that the table lists.
- The wire name of a field is the value of `rename = "..."` in its `#[serde(...)]` attribute. Without `rename`, the wire name is the field name.
- A constant table in the test file maps each triple of file, struct, and wire name to one case. A case is either the name of a corpus test, or a reason that the field carries no free text.
- The test fails on a scanned field without an entry, and on an entry without a scanned field.
- The test also fails when a named corpus test has no `fn <name>` in a test file of `src/`.

The source walk is the mechanism of the other guard tests of the crate, for example `template_tests.rs` and `pages/texts/tests.rs`. A trait or a macro on each request struct needs a production change, so the design rejects it. The review of the table checks each reason entry.

The guard reads structs only. Two pair readers take keys that no struct names. Corpus tests of today cover the key text and one value field of both:

- `parameters/fields.rs` `read_pairs`: `recipe_xss_the_payload_in_a_field_name_gives_one_line_e_web_008` (`recipe_xss_tests.rs` line 514) sends the payload in the key.
- `approval_text_xss_a_string_value_accepts_every_plain_payload_and_shows_it_escaped` (`approval_text_xss_tests.rs` line 262) sends the payload as a parameter `value`.
- `types/define.rs` pairs: `plant_xss_udt_row_index_gives_form_error` (`plant_xss_tests.rs` line 433) sends the payload in the key.
- `plant_xss_udt_rows_with_add_are_echoed_escaped` and `plant_xss_revision_rows_with_add_are_echoed_escaped` (lines 445, 466) send it as values.

Known limit: a new pair reader is outside the guard. The review of each new form handler checks its corpus test.

Today the test fails on `password` of `LoginFields`, and on `current_password` and `password` of `PasswordFields`. Task 3.1 reports every other field that has no entry. When the report holds a text field beyond these three, the orchestrator adds one task for each such field.

### D5: The secure refusal runs in the job `test` (finding D6-1)

The job `test` gets one step after "Run tests": `bash deploy/ci/rust-tests.sh secure`. The job `compose` drops its three steps of the check: the toolchain install, the cache `compose-config`, and the secure step.

- The job `test` runs on each pull request and on each push to `main`. So `main` keeps the check.
- The step builds `recipe-server` without the feature `dev`. `-p recipe-server` also drops the feature `test-support` of `recipe-db`, so the step rebuilds more than one crate. Task 1.2 measures the time of the step and records it.
- The job name stays, so the branch protection of ADR-0127 (y) needs no new entry.
- A new structure check `test_secure_refusal_step` in `check.sh` fails unless exactly one step of `ci.yml` runs the command. That step must be in the job `test`. The job `test` and the step have no `if:`, no `needs:`, and no `continue-on-error`.

Alternative: a new job `secure-refusal`. It needs a new entry in the branch protection of `main` (ADR-0127 (y)) and a step of the owner after the merge. The design rejects it, because the step in `test` needs no step of the owner.

Task 1.3 shows that the step finds a production build with the dev flag. It sets `const DEV_BUILD: bool = true;` in the work tree. Then `cargo test --workspace --all-features` passes, and `rust-tests.sh secure` fails. Fix plan section 7.1 task 2 names an inverted check in `check_secure` as the demo. That demo is weaker: `serve_secure_false_stops_the_start_without_the_dev_build` fails on it in every build. The fix plan stays as it is.

### D6: A pure content check of the export (finding D6-2)

A new module `e2e/compose/support/export-content.ts` exports `exportContentFaults(document, expected)`. It returns a list of fault texts, and an empty list means a match.

- `expected` holds the machine code, the reference number, the version number, the unit path, and the tag.
- `expected` also holds the type, the value, and the minimum and maximum of the one limit.
- The function compares `machine.code`, `recipe.reference_number`, and `version.number`.
- It requires exactly one parameter with the tag. That parameter has `type_id`, `value`, and exactly one limit with `leaf_path` `""`, `min`, `max`, and `max_length` `null`.
- A new pure test `e2e/tests/export-content.spec.ts` follows the pattern of `jcs.spec.ts` (ADR-0128 (ai)). The golden file `recipe_R-100_v3.json` gives no fault for its own values.
- The same test changes one field of a copy at a time and expects a fault that names the field. It also expects a fault for a copy without `SPEED`.
- The smoke step of `end-to-end.spec.ts` calls the function with `run` and `PARAMETER` and expects an empty list.

The regression suite runs `e2e/tests/` on each pull request, so a fault of the check itself fails a pull request. The smoke suite runs only in the job `compose`. A manual dispatch of `compose` on the branch shows the smoke step (ADR-0094 (d)).

Alternative: inline `expect` calls in the smoke step only. Their first red run needs a dispatch of the job `compose` of up to 45 minutes. The design rejects it.

### D7: No other ADR, no error code

D6-1, D6-2, and D6-3 apply rules that exist: ADR-0062 (b), `CONTRIBUTING.md` "Local checks", ADR-0048 (c), `reliability.md` §6, and `.claude/rules/web.md`. They need no new ADR. The change adds no error code and no row of `reliability.md` §7.11.

## Risks / Trade-offs

- Risk: the guard of D4 parses Rust as text. A struct with an unusual layout can give a wrong field list, or the parser can miss a struct.
  - Mitigation: the test fails on an unknown field and on a count of parsed structs that differs from the count of `Deserialize` derives.
  - Mitigation: the test fails on `rename_all`, `flatten`, and `alias`. A unit test of the parser covers `rename`, doc comments, attributes with more items, and each refused key.
- Risk: a new pair reader is outside the guard of D4. Mitigation: the review of each new form handler checks its corpus test.
- Risk: the guard finds more uncovered fields than the review names. Mitigation: D4 adds one task for each field.
- Risk: the content check of D6 fails after an intended change of the export shape. Mitigation: the golden test of `recipe-export` changes at the same time, and the pure test of D6 reads the same golden file.
- Trade-off: ADR-0153 (c) keeps a domain in the set after `DROP CONSTRAINT`. A false alarm is possible. The script stays small.
- Known false alarm: an array of a constrained domain, or of a domain with a volatile default, gives a form. PostgreSQL 16 does not rewrite the table for it (ADR-0153, Risks).
- Trade-off: D5 moves the secure step out of `compose`. The stack test no longer shows the step. The job `test` shows it on each pull request and on each push to `main`.

## Migration Plan

No migration and no deploy step. The change is in CI, tests, and documents only. A revert of the merge commit restores the old gates.

## Open Questions

- Q1 (decided by the owner): a constrained domain that the same file creates gives no exemption. Only a table that the same file creates stays exempt. D3 applies it.
- Q2 (owner): ADR-0153 stays "Proposed" until the owner approves `tasks.md`. Task 2.5 then sets "Accepted".
- Q3 (decided by the owner): the CI step of the ADR index script of plan section 3.1 is not part of WP4.
