# ADR-0189: Pattern of the S7 address in the export schema v2

Status: Accepted
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 found R01 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §3). R01 continues the baseline item F15.

These facts shape the decision:

- The schema files give `"s7_address": { "type": ["string", "null"] }` and no pattern (`crates/recipe-export/schema/export-v1.schema.json`, `export-v2.schema.json`). A document with `DB1.DBX0.8` passes both files.
- The domain parser refuses such an address (`domain-model.md` §2.1, `S7Address`). The writer thus never emits it.
- `api.md` §6 says that the schema file is the source of truth. A breaking change of the export document creates the next prefix (ADR-0019).
- A breaking change of the document makes a document of the writer invalid, or makes a consumer read a field in a new way. ADR-0119 (g) changed the rules of `loadable` in both schema files without a new version.

The project owner decided on 2026-11-17: add the pattern to the schema v2 only, when the schema policy permits a change of v2. The policy permits it. The pattern refuses no document that the writer can emit, so the change is not breaking.

## Decision drivers

- The schema file states the grammar of each field (`api.md` §6, ADR-0019).
- No breaking change of a published schema version (ADR-0019).
- The domain checks all external input, and a consumer can check the document (R38).

## Considered options

### Option A: The pattern in v2 only

- Pros: a v2 consumer can check the address with the schema. The v1 file stays as the first integration knows it.
- Cons: the two schema files differ in one field.

### Option B: The pattern in v1 and v2

- Pros: one rule in both files.
- Cons: the owner keeps v1 unchanged.

### Option C: No pattern

- Pros: no change.
- Cons: a consumer that trusts the schema accepts a malformed address.

## Decision

The project uses **Option A**.

- (a) `parameters[].s7_address` of `export-v2.schema.json` gets a `pattern` for the string form. The pattern follows the grammar of `S7Address` in `domain-model.md` §2.1. The data block number is `1..=65535` without a leading zero. The offset is `0..=65535` with at most 5 digits. The bit is `0..=7` and comes only after `DBX`.
- (b) `export-v1.schema.json` does not change. `schema_version` stays `"1"` and `"2"`.
- (c) The writer, the hashed body, and `content_sha256` do not change.
- (d) Schema tests of `recipe-export` give one passing address of each width `X`, `B`, `W`, and `D`. They give the failing forms `DB1.DBX0.8`, `DB0.DBW0`, `DB65536.DBB0`, `DB1.DBW65536`, `DB1.DBB0.1`, `DB1.DBX0`, and `db1.dbw0`.
- (e) `api.md` §6.4 names the pattern of v2.

## Consequences

### Positive

- A v2 consumer can refuse a malformed address with the schema alone.

### Negative

- A v1 consumer still gets no address pattern from the schema.

### Risks and mitigations

- Risk: the pattern and the parser of `S7Address` differ later. Mitigation: the tests of item (d) run the same forms against the parser and the schema.

## Related

- ADR-0019 (export contract), ADR-0109 (export document v2), ADR-0119 (g) (schema change without a new version)
- `api.md` §6.1, §6.4, `domain-model.md` §2.1 (`S7Address`)
- `overview.md` §7: R03, R38. §9: A31
