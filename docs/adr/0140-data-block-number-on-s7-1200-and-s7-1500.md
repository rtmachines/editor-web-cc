# ADR-0140: Data block number on S7-1200 and S7-1500

Status: Accepted. Amends ADR-0116 (c) (data block rule) and ADR-0120 (a) (earlier rule set)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found an address fault. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #8.

These facts shape the decision:

- ADR-0116 (c) keeps the data block number `1..=65535` on `S7_1200` and `S7_1500`. Only `S7_300` has a lower limit, 16 000.
- The CPU 1517-3 PN/DP Equipment Manual (edition 11/2023) gives the range 1 to 60 999. It names 1 to 59 999 as the range for the user. It names 60 000 to 60 999 as the range of the blocks that SFC 86 creates.
- The S7-1200 System Manual gives 1 to 59 999. The review read it only in a secondary copy.
- The S7-1500 manuals of 2014 gave 1 to 65 535. Thus the range depends on the firmware.
- The editor accepts `DB60000.DBW0` to `DB65535.DBW0` on `S7_1500`. No current CPU of the two families can hold such a block for the user.
- The message of `E-RCP-057` names the limit 16000 and the S7-300 controller.
- ADR-0118 (e) gives `E-RCP-057` as a field line on a read. Thus a stored address passes the earlier rule set, and only the current set has the data block rule.

The project owner decided on 2026-11-06 (fix plan, question Q-C, option (1)).

## Decision drivers

- An exported address fits a data block of the PLC (R03, R14).
- The domain checks all external input (R38).
- A rule change does not lock a draft, and a stored address stays readable (ADR-0116 (e1), ADR-0118).
- The error message names the limit of the platform of the machine.

## Considered options

### Option A: 1 to 59 999 on both platforms

- Pros: the range of the user on every current CPU of the two families. No machine setting.
- Cons: a CPU with an older firmware can hold a block above 59 999. The editor refuses it.

### Option B: 1 to 60 999 on S7_1500

- Pros: the full range of the 11/2023 manual.
- Cons: the user does not create the blocks 60 000 to 60 999. SFC 86 creates them at run time, so a recipe cannot rely on them.

### Option C: A firmware attribute of the machine selects the range

- Pros: the range follows the CPU.
- Cons: a new machine setting, a new form, and a migration. The owner chose a limit per platform (ADR-0116, Option C).

## Decision

The project uses **Option A**.

- (a) Data block rule. On `S7_1200` and `S7_1500`, the data block number is at most 59 999. A higher number gives `E-RCP-057`. This item replaces the last sentence of ADR-0116 (c).
- (b) On `S7_300`, the data block number stays at most 16 000 (ADR-0116 (c)).
- (c) The rule runs on the paths of ADR-0116 (e) and (e1). These are the written parameter of `AddParameter` and `UpdateParameterDefinition`, each parameter of `RebindTypeRevision`, and every parameter at `ApproveVersion`. A stored address of another parameter does not refuse a command.
- (d) The earlier rule set of ADR-0120 has no data block rule. A stored address passes it when it matches the grammar of `S7Address`, so its number is `1..=65535`. This item amends ADR-0120 (a). It records the rule of ADR-0118 (e).
- (e) Thus a read shows a stored address above the limit with a field line of `E-RCP-057` (ADR-0118). The export sets `loadable` to `false` (ADR-0119). `CreateVersion` copies the address (ADR-0120 (h)).
- (f) The variant `S7DataBlockOutOfRange` also carries `max`, the highest data block number of the platform. The message names `max` and no platform. `reliability.md` §1.2 holds the template.
- (g) The value 59 999 is a fact of the platform family. It is not a setting, so `reliability.md` §7.11 does not hold it (ADR-0116 (h)).
- (h) The grammar of `S7Address` does not change (ADR-0116 (d)).

## Consequences

### Positive

- Every new address on `S7_1200` and `S7_1500` uses a data block that the user can create.
- The message of `E-RCP-057` is true on every S7 platform.

### Negative

- A draft with a stored address above 59 999 cannot be approved until the user changes the address.
- `RebindTypeRevision` refuses a parameter with such an address. The user changes the address first with `UpdateParameterDefinition`.
- The message texts of `E-RCP-057` change in the bundles `pl` and `en`.

### Risks and mitigations

- Risk: a CPU with an older firmware holds a recipe block above 59 999. Mitigation: the ops query of the change lists such addresses before the upgrade. Vendor point R4 of the fix plan checks the range of S7-1200 by firmware. A limit per firmware needs a new ADR with Option C.
- Risk: an approved version holds such an address. Approved versions are immutable (ADR-0009). Mitigation: `ApproveVersion` refuses the address in a new draft, and the export sets `loadable` to `false`.

## Related

- ADR-0116 (end of an S7 address, data block number), ADR-0118 (reads), ADR-0119 (export), ADR-0120 (earlier rule set), ADR-0009 (immutable snapshot)
- `domain-model.md` §2.1 (`S7Address`), `reliability.md` §1.2 (`E-RCP-057`)
- `overview.md` §7: R03, R14, R38. §9: A31
