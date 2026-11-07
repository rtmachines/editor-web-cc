# ADR-0116: End of an S7 address and the data block number on S7-300

Status: Accepted. Amended by ADR-0140 (item (c): the data block number is at most 59 999 on `S7_1200` and `S7_1500`)
Date: 2026-11-03
Deciders: project owner

## Context

`domain-model.md` section 2.1 gives `S7Address` the data block number `1..=65535` and the offset `0..=65535` on every platform. No document limits the end of a parameter. The overlap rule computes the bytes `[offset, offset + layout size)`, but no rule compares the end with the size of a data block.

The Siemens "Programming Guideline for S7-1200/1500" (V1.6, 12/2018, entry ID 81318674), section 2.7.1, Table 2-7, gives two limits:

- A data block has at most 64 kB on S7-300, S7-1200, and S7-1500. Only an optimized block of an S7-1500 can be larger, and an optimized block has no offset.
- An S7-300 or S7-400 CPU has at most 16 000 data blocks.

The domain therefore accepts an address that no data block can hold. An example is a `STRING(254)` at `DB1.DBB65534`. Another example is `DB20000.DBW0` on `S7_300`.

## Decision drivers

- An exported address must fit a data block of the PLC (R03, R14).
- The domain checks all external input (R38).
- A rule of the domain gives one error code and a clear message (`reliability.md` section 1.2).

## Considered options

### Option A: Two platform rules of the domain with their own codes

- Pros: the user sees the reason. The rules use the layout size that the domain already computes.
- Cons: two new error codes.

### Option B: Narrow the grammar of `S7Address`

- Pros: no new rule in the draft.
- Cons: `S7Address::parse` does not know the platform or the layout size. A parse reason gives `E-WEB-008` with no detail.

### Option C: A data block limit for each machine

- Pros: the limit can follow the CPU model.
- Cons: a new machine setting and a new form. The owner chose a limit per platform.

## Decision

The project uses **Option A**.

- (a) End rule. On every platform that permits an address, the offset plus the S7 layout size of the type is at most 65 536 bytes. An address with a bit number, such as `DBX65535.7`, occupies one bit, as the overlap rule counts it. Every such address fits.
- (b) A parameter that ends after the last byte of its data block gives `E-RCP-056`. The variant names the tag, the address, and the layout size.
- (c) Data block rule. On `S7_300`, the data block number is at most 16 000. A higher number gives `E-RCP-057`. The variant names the tag and the address. On `S7_1200` and `S7_1500`, the number stays `1..=65535`.
- (d) The grammar of `S7Address` does not change. The two rules run in the draft, with the platform rule, the width rule, and the alignment rule of `domain-model.md` section 2.1.
- (e) The rules run for the parameter that `AddParameter` or `UpdateParameterDefinition` writes and for each parameter that `RebindTypeRevision` rebinds. `ApproveVersion` runs them for every parameter.
- (e1) A stored address of another parameter does not refuse a command. Then a rule change does not lock a draft (ADR-0028). The overlap check of I20 still covers the whole draft.
- (f) The checks of one parameter run in this order: platform rule, data block rule, width, alignment, end rule. The overlap check of the whole draft comes last.
- (g) The database adds no constraint for these rules, as for the alignment and overlap rules (`data-model.md` section 3.12).
- (h) The values 65 536 and 16 000 are facts of the platform. They are not settings, so `reliability.md` section 7.11 does not hold them.

## Consequences

### Positive

- Every new address fits a data block of its platform.
- The error message names the cause.

### Negative

- Two new error codes, with texts in the `pl` and `en` bundles.
- A stored draft with such an address cannot be approved until the user changes the address.

### Risks and mitigations

- Risk: an S7-300 CPU has fewer data blocks than 16 000. Mitigation: the rule is the upper bound of the family. A limit for each CPU model needs a new ADR.
- Risk: an approved version holds such an address. Approved versions are immutable (ADR-0009). Mitigation: `ApproveVersion` refuses the address in a new draft (item (e)). ADR-0118 shows a field line, and ADR-0119 sets `loadable` to `false` at export.

## Related

- ADR-0017 (S7-300 layout), ADR-0024 (S7 address width), ADR-0028 (overlap check at rebind), ADR-0009 (immutable snapshot)
- `domain-model.md` section 2.1 (`S7Address`, `S7Layout`), `data-model.md` section 3.12
- `overview.md` section 7: R03, R14, R38. Section 9: A31
