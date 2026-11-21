# ADR-0207: Symbolic transfer on CODESYS and TwinCAT

Status: Accepted
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ09 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- The reference file says that the layout of a DUT depends on the target, the declaration, and `pack_mode` (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:28`, `:217` to `:236`).
- `S7Layout` of `domain-model.md` §2.1 computes a layout on S7 only. A31 gives a `CodesysIec` or `TwinCat3` parameter no address.
- No document says how the transfer system writes a value on these platforms. Thus no document says why alignment, padding, and `pack_mode` do not matter there.
- The export holds no layout field.

The project owner decided on 2026-11-19: record that the transfer system writes each leaf through its symbol on these platforms.

## Decision drivers

- A document states each scope decision (R43).
- The editor never writes data that the PLC treats in another way (R03, R31).

## Considered options

### Option A: An Assumption for the symbolic transfer

- Pros: one row names the decision. No code change.
- Cons: the editor cannot check the transfer system.

### Option B: A layout model for each platform with `pack_mode`

- Pros: a transfer system can write a raw image.
- Cons: a new model, new fields of the struct, and a new export field.

## Decision

The project uses **Option A**.

- (a) Assumption A53: on `CodesysIec` and `TwinCat3`, the transfer system writes each leaf through its symbol. `overview.md` §9 holds the row.
- (b) The editor therefore models no layout, no `pack_mode`, and no `EXTENDS` on these platforms. `S7Layout` stays an S7 rule.
- (c) `plc_type.size_bytes` holds the S7 size on every platform. It does not give the size of a type on these platforms.
- (d) No code changes, and the export does not change.

## Consequences

### Positive

- A reader knows why the editor has no layout on these platforms.

### Negative

- A transfer system that writes a raw image cannot use the export of these platforms.

### Risks and mitigations

- Risk: a plant uses a transfer system that writes a raw image. Mitigation: A53 names the rule. A new ADR can add the layout model of Option B.

## Related

- ADR-0108 (h) (layout table on S7), ADR-0209 (excluded IEC type families)
- `domain-model.md` §2.1 (`S7Layout`), `data-model.md` §7
- `overview.md` §7: R03, R31, R43. §9: A31, A53
