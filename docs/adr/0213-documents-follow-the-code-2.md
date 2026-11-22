# ADR-0213: Documents follow the code, and the pair of value and limit fields

Status: Accepted. Amends ADR-0028, ADR-0029, ADR-0168 (a) to (e), `data-model.md` §6, `domain-model.md` §2.4, `overview.md` §5, `reliability.md` §7.7, §8, and `api.md` §2.6, §4
Date: 2026-11-22
Deciders: project owner

## Context

The review of 2026-11-21 (`docs/plan/review/2026-11-21-full-repo-review.md` §4) found nine places where a normative text and the code disagree. Each finding has the severity LOW.

Seven places are text that does not state the shipped code or an accepted decision:

- `data-model.md` §6 intro (line 2227) says that every command runs in exactly one transaction. Six plant commands and the recipe and draft commands first read in a `READ ONLY` transaction. Examples are `live_machine_id` (`crates/recipe-app/src/plant/resolve.rs`) and `read_recipe` (`crates/recipe-app/src/recipe/resolve.rs`). ADR-0168 (a) to (e) named this read only for three commands.
- `domain-model.md` §2.4 (line 564) says that three audit actions store a state object. `data-model.md` §10 and the code have four. The fourth is `PlcConnectionExported` (ADR-0154 (j)).
- `overview.md` §5 (line 133) and ADR-0029 say that `recipe-app` links `parking_lot` only with the feature `memory`. Since ADR-0172 (y), the production build always links it (`crates/recipe-app/Cargo.toml` line 22). ADR-0029 also links `time` only with `memory`. `UnlockLogin` writes RFC 3339 text, so the build always links `time` (lines 24 and 25).
- `reliability.md` §8 (line 1248) gives a closed list of interior mutability. Five production places are not in it.
- `reliability.md` §8 (line 1244) says that `cargo deny check` fails on a denied license, a yanked crate, or a security advisory. `deny.toml` also has a `[bans]` table, and `CONTRIBUTING.md` line 77 names a banned dependency edge.
- `api.md` §4 step 1 (line 410) puts `row_version` into every form of the version page. The `GET` compare form of ADR-0108 (d) and the header forms `POST /lang` and `POST /logout` have no such field.
- `reliability.md` §7.7 (line 871) names the drill command `pitr-volume --drill`. ADR-0106 (f) step 1 says that `--drill` requires `--target-time`.

The eighth place is ADR-0028 (line 63). It says that `check_for_approval` and `VersionSnapshot::check` reject `LiveUnits` of another machine with `E-PLT-013` or `E-PLT-020`. The code compares only the machine of the `TypeTrees` (`crates/recipe-domain/src/recipe/snapshot/check.rs` lines 36 to 38). The application loads the units of the machine of the draft, so no route reaches the case.

The ninth place is `api.md` §2.6 (line 215). The row `POST .../definition` says that the value fields and the limit fields are optional without a type change. ADR-0029 says that a request without them keeps the value and the limits. A request with value fields and no `limits[i]` field replaces the limits with an empty set (`crates/recipe-web/src/pages/parameters/edit.rs` lines 39 to 43). So a changed request can delete every limit of a parameter without a message.

`CLAUDE.md` ("Source of truth and change rule") lets a change of `docs/architecture/` go only with an ADR. ADR-0168 and ADR-0191 recorded corrections of the text to the code in the same way.

## Decision drivers

- The normative text must state the behavior of the shipped code and of the accepted decisions (R34).
- A command runs atomically, and validation runs before its transaction (R29, `reliability.md` §7.4).
- A change of the stored data must not occur without a check of all input (R38). A process limit must not disappear without a request for it (R09).
- A Status note must not hide the ADR that made a decision.

## Considered options

### Option A: One ADR. The texts follow the code, and a pair rule for the value and limit fields

- Pros: no code change for eight places. The texts then state what the system does. The pair rule fails closed and keeps the stored limits.
- Cons: one check more in `recipe-web`. Two rules (`CLAUDE.md` and `.claude/rules/sql.md`) get one clause more.

### Option B: Change the code to the texts

- Pros: the texts stay.
- Cons: the command transaction then also resolves the identifiers, and many commands change with no data fault to fix. A second machine check for `LiveUnits` guards a case that no route reaches.

### Option C: Each part of the value and limit fields is optional alone

- Pros: a client can send a value without limits.
- Cons: a new input shape in `recipe-app` and in `Draft::set_definition`. A save can keep old limits next to a new value without a check of the pair.

### Option D: Keep the code of the definition route, and document it

- Pros: one sentence in `api.md` §2.6.
- Cons: a changed request deletes every limit of the parameter without a message.

## Decision

The decision selects Option A. The owner decided the pair rule and the alignment of ADR-0028 on 2026-11-22.

Items (b), (e), (f), and (g) only align a text with a current decision. This ADR lists them for the Status notes and the decision map. It makes no new decision for them. Items (a), (d), (h), and (i) make a decision. Item (c) aligns a text for `parking_lot` and makes a decision for `time`.

- (a) A command can read in one `READ ONLY` transaction before its command transaction. This rule extends ADR-0168 (a) to (e) from three commands to every command.
  - The read resolves identifiers, for example the machine of a machine code or the version of a reference number.
  - The read takes no lock. It holds nothing into the command transaction.
  - The command transaction locks the rows and checks the facts of the read again. A constraint can also check such a fact.
  - A row of `data-model.md` §6 that names a step "Before the transaction" keeps that step. T1 and T2 of ADR-0035 (b) and ADR-0109 (m) stay.
  - An error path of `ApproveVersion` can read once more (`read_root_again` in `crates/recipe-app/src/recipe/approve_version/prepare.rs`). That path then ends without a command transaction.
  - `data-model.md` §6 intro states the rule. `CLAUDE.md` ("Hard rules") and `.claude/rules/sql.md` ("Locks and transactions") each get one clause for it.
- (b) `domain-model.md` §2.4 names four actions that store a state object. The fourth is `PlcConnectionExported`. This item restates `data-model.md` §10 and ADR-0154 (j).
- (c) `recipe-app` always links `parking_lot` and `time`. `overview.md` §5 names both without the feature `memory`.
  - For `parking_lot`, this item restates ADR-0172 (y).
  - For `time`, this item is new. `UnlockLogin` writes the time of an attempt as RFC 3339 text.
- (d) The list of interior mutability in `reliability.md` §8 gets five places, each with its source:
  - the slots of the detached tasks, as an owned semaphore (ADR-0172 (x)),
  - the `JoinSet` of the detached tasks behind a `parking_lot::Mutex` (ADR-0172 (y)),
  - the permits of the diff gate and of the export gate in `crates/recipe-app/src/blocking.rs` (ADR-0050 (g), ADR-0056 (a)),
  - the semaphore of `BasicGate` in `crates/recipe-app/src/identity/basic_permit.rs` (ADR-0109 (n)),
  - the counter of the layer `request_id` (`AtomicU64`, `crates/recipe-web/src/pipeline.rs` line 70). The public router and the health router each own one counter.

  The first four places restate their sources. This ADR permits the fifth place. Without random bytes, the counter keeps two request ids of one router apart.
- (e) `reliability.md` §8 says that `cargo deny check` also fails on a banned dependency edge. This item restates the `[bans]` table of `deny.toml` and `CONTRIBUTING.md` line 77.
- (f) `api.md` §4 step 1 puts `row_version` into every form that mutates the version. This item restates ADR-0008. The compare form of ADR-0108 (d), `POST /lang`, and `POST /logout` have no such field.
- (g) The drill command of `reliability.md` §7.7 is `pitr-volume --drill --target-time <time>`. This item restates ADR-0106 (o) and the argument rule of ADR-0106 (f) step 1.
- (h) `check_for_approval` and `VersionSnapshot::check` compare only the machine of the `TypeTrees` with the machine of the recipe. This item replaces the `LiveUnits` part of the last rule of ADR-0028 "Approval".
  - `LiveUnits` of another machine give one `E-PLT-020` item for each parameter inside `E-RCP-020`.
  - The empty case: a snapshot with no parameter gives no item for such `LiveUnits`. The approval of a draft with no parameter gives `E-RCP-008` on another path.
  - The rule of ADR-0028 for `TypeTrees` of another machine or platform stays. No code changes.
- (i) Without a type change, the value fields and the limit fields of `POST .../definition` come only as a pair (`api.md` §2.6). This item replaces the last rule of ADR-0029 for `UpdateParameterDefinition` without a type change.
  - A request with both parts replaces the value and the limits. The checks are the same as after a type change.
  - A request with neither part keeps the value and the limits.
  - A request with only one part gets `422 E-WEB-008`. The parameter does not change.
  - A type with no leaf that accepts a limit has no limit fields. A leaf accepts a limit unless its class is `Bool` or `Char`.
  - `recipe-web` checks the pair before the use case runs. After a type change, both parts stay required, and missing value fields still give `422 E-RCP-005`.

## Consequences

### Positive

- `data-model.md`, `domain-model.md`, `overview.md`, `reliability.md`, and `api.md` state the shipped code and the accepted decisions.
- No migration and no code change follow for items (a) to (h).
- A changed request to `POST .../definition` cannot delete the limits of a parameter without a message.
- Each Status note names the source of a restating item, so the decision map still leads to the ADR that made the decision.

### Negative

- A reader of `data-model.md` §6 must keep the read before the transaction apart from the command transaction.
- `recipe-web` reads the type of the form before the use case, to find a leaf that accepts a limit.

### Risks and mitigations

- Risk: a later command reads before its transaction and does not check the fact again under its locks. Mitigation: item (a) requires the second check, and the review of each command row tests it against `data-model.md` §6.
- Risk: a later route passes `LiveUnits` of another machine. Mitigation: the guard tests of the empty case and of the foreign units pin the behavior of item (h).
- Risk: a hand-built client sends a value without limit fields. Mitigation: it gets `422 E-WEB-008`, and nothing changes. The browser form always sends both parts, because the grid renders `limits[i].leaf_path` for each leaf with a limit.

## Related

- ADR-0008 (`row_version` in every form that mutates the draft), ADR-0028 (the approval token), ADR-0029 (the application clarifications)
- ADR-0050 (g), ADR-0056 (a), ADR-0106 (f), (o), ADR-0108 (d), ADR-0109 (n), ADR-0154 (j), ADR-0172 (x), (y)
- ADR-0168 (a) to (e) (the read before the transaction of three commands), ADR-0191 (a correction of the text to the code)
- `data-model.md` §6, §10, `domain-model.md` §2.4, `overview.md` §5, `reliability.md` §7.4, §7.7, §8, `api.md` §2.6, §4
- Change `review-design-fixes-2`
- `overview.md` §7: R09, R29, R34, R38
