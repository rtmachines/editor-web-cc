# Semantic review: 2026-11-17

This page is a review record. It is not normative. The review changed no code, doc, spec, or test.

## 1. Scope and HEAD

- Repository: `editor-web-cc`, worktree `/home/rtm/editor-web-cc-wt/semantic-review`.
- HEAD: `77f51129f0c33359f66f910d01bb72c06a8c112f` (origin/main, merge of PR #76).
- Goal: plan section 5 of `docs/plan/review/2026-11-13-fix-plan.md`. Each command of `recipe-app` must do what the normative documents say. Each invariant of `recipe-domain` must hold.
- Earlier reviews checked style, lints, and structure. This review checks meaning: the specified steps, in the specified order, with the specified errors and audit entries.
- Normative sources:
  - `domain-model.md` §3 to §5, `data-model.md` §1.4, §4, §5.2, §6, §10, and `api.md` §3.
  - `reliability.md` §1.1, §1.2, §7.4, §7.11, the decision map of `docs/adr/README.md`, and the hard rules of `CLAUDE.md`.
- Method: a static read. No agent ran `cargo`, a build, Docker, or a browser. A claim that needs a run is PLAUSIBLE and names the test that proves it.
- The nine checklist items of each command:
  1. The permission check is the first statement.
  2. Read-only mode refuses the command.
  3. Validation runs before the transaction.
  4. The transaction and the lock order follow the rules.
  5. The error cases follow `data-model.md` §6 and `domain-model.md` §5.
  6. The audit entries follow `domain-model.md` §5 and `data-model.md` §10.
  7. A draft command follows the draft protocol.
  8. An invariant has its mechanism and a test that fails without it.
  9. The memory adapter and the PostgreSQL adapter give the same semantics.
- Severity scale: the scale of `docs/plan/review/2026-11-13-full-repo-review.md` §1. MEDIUM includes a missing test of a specified rule.
- Verdicts: CONFIRMED means that the reviewer saw the defect in the code, or the conflict in both documents. PLAUSIBLE means that the failure needs a condition that the reviewer did not see in the code.
- The review did not repeat F-01 to F-33 of the review of 2026-11-13.

### 1.1 Agents, models, and effort

The orchestrator used seven review agents. Each agent read the shared rules file and one area brief. Each Claude agent ran at the default effort.

| # | Agent | Area | Model | Effort | Output |
|---|---|---|---|---|---|
| 1 | Area PLT | Plant Model commands of `domain-model.md` §5.1, `EnsureDerivedType`, class-2 locks | Opus | default | PLT |
| 2 | Area RCA | Draft edit commands of §5.2, draft protocol, ADR-0008, A43 | Opus | default | RCA |
| 3 | Area RCB | Recipe lifecycle and export commands, I1, I2, I4, I12, I14, I16, state machine of §4 | Opus | default | RCB |
| 4 | Area IDN | Identity commands of §5.3, class-1 and class-4 locks, throttle, Basic check, `SecurityEventSink` | Opus | default | IDN |
| 5 | Area QRY | The 45 queries of §5.5 | Opus | default | QRY |
| 6 | Area DOM | Invariants I1 to I23, value objects of §2, triggers of `data-model.md` §4 | Opus | default | DOM |
| 7 | Codex CLI | Second independent review of §5.1 to §5.5 and I1 to I23 | gpt-6-astra | high | CDX |

The orchestrator merged the seven reports and read the code of RCB-01, QRY-01, DOM-01, DOM-02, and CDX-01. It found each of the five defects in the code.

## 2. Coverage

Status values:

- **Full**: the agent checked each item of its area against the nine checklist items.
- **Partial**: the agent left out a part. The note names the part.

| Area | Agent | Status | Not checked, with the reason |
|---|---|---|---|
| Plant Model commands (9) and `EnsureDerivedType` | PLT (Opus) | Full | The plant queries (area QRY). The other checklist items of the callers of `EnsureDerivedType` (areas RCA, RCB) |
| Draft edit commands (8) | RCA (Opus) | Full | The domain rules inside `Draft::replace_parameters` (area DOM). The `audit_log` insert of `UnitOfWork::commit` |
| Lifecycle and export commands (10), I1, I2, I4, I12, I14, I16 | RCB (Opus) | Partial | The inner rules of `Draft::check_for_approval`, `VersionSnapshot::check`, and `HashedBody::build` (area DOM). The trigger SQL of `recipe_version_transition` |
| Identity commands (9), Basic check, locks, throttle, sink | IDN (Opus) | Full | The identity queries (area QRY). The `create-admin` wiring of `recipe-server`. The web mapping of `E-IDN-004` |
| Queries (45) | QRY (Opus) | Full | The `recipe-web` handlers that call the queries. The inner domain logic of the diff, the outdated check, and the rule check |
| Invariants I1 to I23, value objects | DOM (Opus) | Partial | The S7 layout rules of I20 in depth. `Login`, `Password`, and the token value objects. The full grammar of `IecLiteral` and `LeafPath` |
| All commands, queries, and invariants | CDX (gpt-6-astra, high) | Full | Runtime checks, as the brief required |

Limits of the coverage:

- Every claim is static. No test ran.
- Two independent reviews cover each command: one area agent and Codex.

## 3. Findings

Totals after the merge:

| Severity | Total | CONFIRMED | PLAUSIBLE |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 5 | 5 | 0 |
| LOW | 6 | 5 | 1 |

- The seven reports had 12 findings and 4 doc conflicts. The merge gives 11 findings and 3 doc conflicts.
- Codex rated CDX-02 MEDIUM. The orchestrator moved it to the doc conflicts as DC-3, because the code meets the rule "no I/O inside a transaction". Only the text of the rule differs (section 4).
- The RCB note on the memory guard of ADR-0030 and CDX-01 name the same gap. They merge into CDX-01.
- The doc conflicts RCA D1, RCB DC-1, and IDN DC-2 name one gap: incomplete error columns. They merge into DC-2.
- IDN found no code defect.

### 3.1 MEDIUM

#### RCB-01: `ApproveVersion` checks the draft before it compares the row version

- severity: MEDIUM
- verdict: CONFIRMED (orchestrator read the code)
- location: `crates/recipe-app/src/recipe/approve_version/prepare.rs` (`read_root`: the replay check, then `check_approvable`, then the validation of the draft)
- rule: `domain-model.md` §4: "The order of the checks is the idempotent case, then the row version, then the transition (`api.md` section 4)."
- rule: `data-model.md` §6 rules: "Inside the transaction the use case checks `row_version` again."
- failure scenario: user B adds a parameter without limits to the draft, so `row_version` goes from 5 to 6. User A approves with `row_version=5`. The spec gives `409 E-WEB-001`. The code gives `E-RCP-016 LimitMissing` for a parameter that A never saw. A stale request on an approved version gives `E-RCP-007` in place of `E-WEB-001`.
- impact: no data changes, because the compare inside the transaction stops the write. The user gets a wrong code and no conflict fragment.
- sources: RCB-01 (Opus)
- note: the `web-ui` requirement "Order of the approval errors" gives the code of the first check, also with a stale `row_version`. Plan review R3 made RCB-01 a doc fix (§7).

#### QRY-01: No test pins the `ReadOnly` isolation of six queries

- severity: MEDIUM
- verdict: CONFIRMED (orchestrator read the code)
- locations: `crates/recipe-app/src/audit/queries/list.rs`, `entry.rs`, `history.rs`, `security_events.rs`, `crates/recipe-app/src/export/list_exportable_recipes.rs`, `crates/recipe-app/src/plant/queries/usage.rs`
- rule: `reliability.md` §7.11: "A query runs in `READ COMMITTED READ ONLY`." ADR-0042 (c): "Every other query keeps `Isolation::ReadOnly`."
- failure scenario: a refactor changes one of the six queries to `Isolation::ReadCommitted`. PostgreSQL then no longer stops an accidental write, and every test stays green.
- sources: QRY-01 (Opus)

#### DOM-01: No test isolates the frozen-column clauses of `recipe_version_transition`

- severity: MEDIUM
- verdict: CONFIRMED (orchestrator read the code)
- locations: `migrations/0002_triggers.sql` lines 199 and 207, `crates/recipe-db/tests/storage/triggers/transition.rs`, `crates/recipe-db/tests/storage/grants/app_role.rs`
- rule: `domain-model.md` §3 I4 (the frozen columns before and after approval). `.claude/rules/sql.md`: "Every trigger has a positive test and a negative test."
- failure scenario: each frozen-column test updates an approved version and keeps `status = 'approved'`. The status clause rejects that update first. A later migration that drops the clause at line 207 fails no test. A defect in the retire statement can then change `content_sha256` of a released version.
- sources: DOM-01 (Opus)

#### DOM-02: I17 has no test for duration, instant, `CHAR`, or `WCHAR` leaves

- severity: MEDIUM
- verdict: CONFIRMED (orchestrator read the code)
- locations: `crates/recipe-domain/src/recipe/draft/approval.rs` (`needs_limit`), `crates/recipe-domain/src/recipe/draft/approval/tests.rs` (`bool_char_and_wchar_leaves_need_no_limit` builds only a `BOOL` parameter)
- rule: `domain-model.md` §3 I17: "At approval every numeric, string, duration, and instant leaf has a limit. A bool or char leaf has none."
- failure scenario: a change removes `LeafClass::Duration | LeafClass::Instant` from `needs_limit`. Every test stays green, and an approved version carries a `TIME` setpoint without a process limit.
- sources: DOM-02 (Opus)

#### CDX-01: The memory status writes omit the guard of ADR-0030

- severity: MEDIUM
- verdict: CONFIRMED (orchestrator read the code)
- locations: `crates/recipe-app/src/memory/recipe/mod.rs` (`change_version`), `crates/recipe-db/tests/storage/contract/recipe_write/guard.rs` (four `postgres_only` cases)
- rule: ADR-0030: a status write without the lock of its version gives `E-DB-008` and writes nothing. Checklist item 9: both adapters give the same semantics.
- failure scenario: a command loses its version lock in a refactor. The memory-backed use-case tests still pass, and only PostgreSQL refuses the write.
- sources: CDX-01 (Codex gpt-6-astra, high), RCB note on item 9 (Opus)

### 3.2 LOW

#### RCB-02: The second pick of `/approved` does not apply the machine filter of the `{code}` routes

- severity: LOW
- verdict: PLAUSIBLE
- location: `crates/recipe-app/src/export/flow.rs`, `pick_lock_and_write` (the second call of `find` passes `None` for the machine)
- rule: `data-model.md` §6 row `ExportVersion`: "The machine filter of ADR-0109 (h) and (j) applies to the routes with `{code}`."
- rule: `api.md` §2.7: "A reused code hides the older archived machine from these routes."
- failure scenario: between the first pick and its lock, an approval retires the pick, `ArchiveMachine` archives the machine, and `CreateMachine` reuses its code. The second pick exports the recipe of the archived machine. The spec gives `404 E-WEB-005`. The window is one request inside `timeout.export`.
- sources: RCB-02 (Opus)

#### QRY-02: The memory store accepts writes in a read-only transaction

- severity: LOW
- verdict: CONFIRMED
- location: `crates/recipe-app/src/memory/core/transaction.rs` (`state_mut`, `append_audit`, `finish`)
- rule: ADR-0042 (e) and checklist item 9. PostgreSQL refuses a write in a `READ ONLY` transaction with SQLSTATE `25006`.
- failure scenario: a later change adds a write to a query. The memory tests pass, because the write succeeds and the store drops it. Production gives `500` on each request.
- sources: QRY-02 (Opus)

#### DOM-03: The memory adapter does not emulate the parameter triggers and the platform `CHECK`

- severity: LOW
- verdict: CONFIRMED
- location: `crates/recipe-app/src/memory/draft/mod.rs` (`check_parameter_references`, `insert_parameter`)
- rule: checklist item 9. `reliability.md` §4.1, layer (c).
- failure scenario: a regression removes the unit check of the domain. A memory-backed use-case test commits a parameter with a unit of another machine and passes. PostgreSQL refuses the write with `E-RCP-032`.
- sources: DOM-03 (Opus)

#### PLT-01: The second-line `DuplicateMemberName` names the first member, not the repeated one

- severity: LOW
- verdict: CONFIRMED
- location: `crates/recipe-app/src/plant/define_udt.rs` (`insert_struct_row`)
- rule: `reliability.md` §1.1: a variant carries the values that identify the failure. `data-model.md` §6 row `DefineUdt`.
- failure scenario: the domain check finds each repeated ASCII name first, so the path is unreachable today. A later grammar with non-ASCII names can reach it, and `E-PLT-021` then names the wrong member.
- sources: PLT-01 (Opus)

#### PLT-02: The memory double journals the class-2 locks in text order, and a test name calls it the lock order

- severity: LOW
- verdict: CONFIRMED
- locations: `crates/recipe-app/src/memory/types/mod.rs` (`lock_derived_types`), `crates/recipe-app/src/plant/define_udt/tests.rs` (`the_journal_holds_the_machine_lock_and_then_the_advisory_keys_in_the_order_of_the_texts`)
- rule: `data-model.md` §1.4 and ADR-0144: the adapter sorts the key values in ascending order.
- failure scenario: no wrong result in production. The `recipe-db` tests prove the real order. A reader can take the `recipe-app` test name as the rule.
- sources: PLT-02 (Opus)

#### RCA-01: The touch statement runs after the unit lock and the class-2 locks, not at its documented position

- severity: LOW
- verdict: CONFIRMED
- locations: `crates/recipe-app/src/recipe/add_parameter.rs`, `update_definition.rs`, `rebind.rs`, `draft_session.rs`
- rule: `data-model.md` §6, rows `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` (touch before the unit lock and the class-2 locks).
- failure scenario: none observable. The touch updates a row that the command already holds, so the lock waits do not change. Only the text differs from the code.
- sources: RCA-01 (Opus)

## 4. Doc conflicts

- DC-1: ADR-0035 (t) says that a lost event `login_failed`, `login_rejected_inactive`, or `login_throttled` gives the storage error. ADR-0172 (q), `reliability.md` §1.1, and `domain-model.md` §5 give the refusal answer. No Status line records the amendment. The code follows ADR-0172. The event `csrf_rejected` still follows ADR-0035 (t). Source: IDN DC-1 (Opus).
- DC-2: `domain-model.md` §5 calls the error column complete. §5.3 omits `E-WEB-005` for five identity commands. §5.4 omits `E-WEB-011` for `ExportVersion`. The guard column of `data-model.md` §6 omits `E-RCP-022`, `E-RCP-055`, `E-RCP-056`, and `E-RCP-057` in five draft rows. The code follows the complete sets. Sources: IDN DC-2, RCB DC-1, RCA D1 (Opus).
- DC-3: `reliability.md` §7.4 and `CLAUDE.md` say "Validation runs before the transaction". The draft commands and `DefineUdt` run the checks of the locked state under the lock. These checks use the CPU only. Source: CDX-02 (Codex gpt-6-astra, high).

## 5. Owner decisions

The owner decided on 2026-11-17. Change `semantic-review-fixes` carries every item.

- Fix (MEDIUM): RCB-01, QRY-01, DOM-01, DOM-02, and CDX-01.
  - RCB-01: `ApproveVersion` compares the row version right after the replay check, before `check_approvable`. Superseded after plan review R3 (§7): a doc fix of `domain-model.md` §4.
  - QRY-01: tests pin `Isolation::ReadOnly` for the six queries.
  - DOM-01: negative trigger tests for the two frozen-column clauses, each with an allowed transition.
  - DOM-02: I17 tests for duration and instant leaves (a limit is necessary) and `CHAR` and `WCHAR` leaves (no limit is necessary).
  - CDX-01: the memory adapter mirrors the version guard of ADR-0030. The negative guard cases run on both adapters.
- Fix (LOW): RCB-02, QRY-02, DOM-03, PLT-01, PLT-02, and RCA-01. RCA-01 changes the three rows of `data-model.md` §6 to the order of the code.
- Doc conflicts: the documents follow the code, each with a short ADR.
  - DC-1: an "Amended by" note on ADR-0035 (t). The scope leaves out `csrf_rejected`.
  - DC-2: the error columns of `domain-model.md` §5.3, §5.4, and `data-model.md` §6 become complete.
  - DC-3: the wording of `reliability.md` §7.4 and `CLAUDE.md`. Input parsing runs before the transaction. A check that needs the locked state runs under the lock, uses the CPU only, and waits for no I/O. The code does not change.

## 6. Notes without a finding

- PLT: `DerivedTypes::ensure_row` does not add a row when the adapter returns `created: None`. Under the class-2 locks, no reachable path misses the row.
- PLT: `DefineUdt` can insert derived rows before a refusal. The transaction rolls back, so no row stays (ADR-0159 (d)).
- IDN: the after-image of `UnlockLogin` holds `login_attempt_count: 0` next to the empty array. No sentence forbids the extra field.
- DOM: the web form parses the type text with `TypeExpr::from_stored(.., Earlier)`, and the command checks it again with `require_current_type`. ADR-0143 (i2) asks for this second check.

## 7. Plan review R1

Codex (R1-01 to R1-04) and Opus (P-01 to P-08) reviewed the plan of change `semantic-review-fixes`. The owner approved a fix of every item. The plan then changed as follows:

- R1-01: ADR-0191 (h) and ADR-0192 (c) keep the exception of ADR-0137 (c). The touch runs before the first write of a parameter row, and `EnsureDerivedType` can write before it.
- R1-02: ADR-0192 (a) and (b) separate the parsing of the request from the conversion of a value against the locked type tree.
- R1-03: design D8 and task 6.2 also cover `MemoryTransaction::replace_limits` with two contract cases.
- R1-04 and P-07: task 5.3 calls `insert_struct_row` itself. Design D4 records that `run` cannot reach the branch.
- P-01: design D2 keeps the compare in the second read of the root. The race test expects `E-WEB-001`.
- P-02 to P-06 and P-08: design D3 and D6 and tasks 1.0, 2.1, 3.1, 4.1, and 5.5 got the fixes of the reports.

Plan review R2: Codex gave READY, and Opus found R2-01, R2-02 (MEDIUM), and three LOW items. The owner approved a fix of every item. For R2-01, design D2 and task 5.1 rename the stale test of `approve_version/tests/write.rs`, and its journal holds only `Begin(ReadOnly)`. For R2-02, design D2 puts the check of `E-RCP-012` (A14) after the replay and before the compare. The delta gets a scenario, and task 5.1 gets a test. For the LOW items, `proposal.md` gives four scenarios, the `design.md` Context gives the full clause lines, and task 7.1 names the discard race.

Plan review R3: Opus found R3-01 (HIGH). The plan of RCB-01 contradicted the accepted requirement "Order of the approval errors" of the `web-ui` spec and three test files of `recipe-web`. Codex found R3-01 (HIGH) and R3-02 (MEDIUM) about design D2. The owner decided on 2026-11-17 that the `web-ui` requirement and the code state the intended order. The rule "Idempotent approve" of `domain-model.md` §4 is too general. RCB-01 is therefore a doc fix with no code change.

Design D2 records the decision, ADR-0191 (i) gives the old and the new sentence, and task 2.4 applies it. The plan removed the compare of D2, the `recipe-management` delta, task 5.1, and the RCB-01 line of task 7.1. Codex R3-01 and R3-02 close with D2.

Plan review R4: Codex found R4-01, R4-02, and R4-03 (MEDIUM), and Opus found R4-01 and R4-02 (LOW). The owner approved a fix of every item. §5 and §3.1 now mark the RCB-01 decision as superseded. ADR-0191 (i) and task 2.4 put the check of `E-RCP-012` first under the locks. Design D2 and ADR-0191 separate the order of execution from the precedence of the answer. Design D2 gives the full paths of the three test files.
