# ADR-0192: Input parsing before the transaction, checks of the locked state under the lock

Status: Accepted. Amends the row "Idle in transaction" of `reliability.md` §7.4
Date: 2026-11-17
Deciders: project owner

## Context

The row "Idle in transaction" of `reliability.md` §7.4 ends with the sentence "Validation runs before the transaction". The hard rules of `CLAUDE.md` repeat it: "Validation runs before it (`reliability.md` §7.4)."

The semantic review of 2026-11-17 found that the code runs a part of its checks inside the transaction. The review record lists it as DC-3 (`docs/plan/review/2026-11-17-semantic-review.md`):

- `OpenDraft::open` (`crates/recipe-app/src/recipe/draft_session.rs`) locks the recipe and the version. Then it resolves the type trees of the draft. All eight draft commands use this path.
- `SetParameterValue` (`set_value.rs`) then checks the value against the type tree and the stored limits of the draft.
- `RebindTypeRevision` (`rebind.rs`) maps and checks each affected parameter after the locks.
- `DefineUdt` (`crates/recipe-app/src/plant/define_udt.rs`) checks the complete type tree after the machine lock and the class-2 locks.

Each of these checks needs the locked state: the draft, its parameters, and the type catalog. A check before the transaction would read a state that can change before the lock. The plan of 2026-11-06 (`docs/plan/2026-11-06-plc-type-fix-plan.md`, row F-1) named the same gap. ADR-0137 uses the words "Validation runs before the first write of the command".

The code does the input work in two places:

1. Before the transaction, the code parses the request structure and each value object that needs no locked state. Examples are the path values, the form fields, `TypeExpr`, `UnitPath`, and `S7Address`. The value of a parameter stays a `ValueInput`: a JSON text or one text per leaf. A bad input of this kind opens no transaction.
2. Under the lock, the code converts the `ValueInput` against the locked type tree and checks it. `Draft::set_value` (`set_value.rs`) and the add path (`draft/add/step_content.rs` in `recipe-domain`) call `ParameterValue::parse` or `ParameterValue::from_texts_checked` there. The other checks of the locked state also run there.

The work under the lock uses the CPU only. It waits for no I/O and for no lock that the command does not hold.

## Decision drivers

- Inside a transaction, no use case waits for I/O or for a lock that it does not hold (R29, `reliability.md` §7.4).
- A check of the locked state reads the state that the command then writes (R29, I12).
- The text states the behavior of the shipped code when the code meets the requirements (R34).

## Considered options

### Option A: Change the text to the code

- Pros: no code change. The rule then names the two kinds of checks and the place of each.
- Cons: a long check holds the row locks of the draft for its duration.

### Option B: Check a read snapshot first, then check again under the lock

- Pros: the row locks are shorter for a request that the check refuses.
- Cons: each command runs its checks twice. The second run under the lock is still necessary, because the state can change between the snapshot and the lock. The change touches every draft command and `DefineUdt`.

## Decision

The project uses **Option A**. No code changes.

- (a) The parsing of the request structure and of each value object that needs no locked state runs before the transaction. A bad input of this kind gives its error and opens no transaction.
- (b) The conversion of a `ValueInput` against the locked type tree runs under the lock. Each other check that needs the locked state also runs under the lock. A bad value or a failed check gives its error, and the rollback removes each earlier write of the command.
- (c) A check under the lock runs before the first write of a parameter row (ADR-0191 (h)). For `DefineUdt`, it runs before the insert of the struct row. `EnsureDerivedType` can insert a new derived type before the check. `AddParameter`, `UpdateParameterDefinition`, `RebindTypeRevision` (ADR-0137 (c)), and `DefineUdt` use this path.
- (d) The row "Idle in transaction" of `reliability.md` §7.4 gets a new end. This text replaces its last sentence: "The parsing of the request and of its value objects runs before the transaction. The conversion of a value against the locked type tree and each other check of the locked state run under the lock. They use the CPU only and wait for no I/O."
- (e) The hard rule of `CLAUDE.md` "Validation runs before it" gets the same text as item (d).
- (f) The snapshot validation of `CreateVersion` and `ApproveVersion` in `spawn_blocking` stays before the transaction (`data-model.md` §6 rules). Item (b) does not change it.
- (g) Work under the lock uses the CPU only. It waits for no I/O and for no lock that the command does not hold.

## Consequences

### Positive

- The rule states the shipped code. A review finds no conflict between the rule and the draft commands.
- A check of the locked state cannot use a stale read.

### Negative

- A check under the lock holds the row locks of the draft while it runs. A concurrent edit of the same draft waits for it.

### Risks and mitigations

- Risk: a large draft makes a check under the lock slow. Mitigation: the size budgets of I18 bound the draft. `db.idle_in_transaction_timeout` and `db.lock_timeout` of `reliability.md` §7.11 bound each wait.
- Risk: a later change adds an I/O step under the lock. Mitigation: item (g) forbids it, and a review checks it.

## Related

- ADR-0015, ADR-0028, ADR-0137, ADR-0168
- `reliability.md` §7.4, §7.11, `data-model.md` §6, `CLAUDE.md` "Hard rules"
- Change `semantic-review-fixes`, doc conflict DC-3 of the semantic review of 2026-11-17
- `overview.md` §7: R29, R34
