# ADR-0017: Identity of derived PLC types and S7-300 layout

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

ADR-0006 keyed every `plc_type` row by `(machine_id, name, revision)`. The PostgreSQL review (C2) executed the schema and showed that a second `STRING(n)` on one machine violates this key. Array types need a `name` that matches the identifier pattern, and no document defined how to build one (traceability C8). The architecture review (F5) added that `UpsertParameter` created these rows from the Recipe Management context.

The same review (F15) and the author's PLC check found that S7-300 addresses were checked for format only. There was no alignment rule (`DBW3` is invalid on S7-300), no overlap check inside a data block, and no layout size for structs and arrays.

The `created_at = now()` trick that closed a struct after its creating transaction failed when the application set `created_at` explicitly (PostgreSQL M2). The shape CHECKs passed on NULL values (PostgreSQL M1).

## Decision drivers

- A derived type is identified by its shape, not by a name (R11, R12, R13).
- A UDT is identified by name and revision (ADR-0006).
- Recipe Management does not write Plant Model tables (overview section 4).
- A wrong S7-300 address writes one value over another in the PLC (R14, R31).

## Considered options

### Option A: Generated names for derived types

- Pros: one unique key for every row.
- Cons: the name has no meaning, two equal shapes can get two names, the type table grows with every edit.

### Option B: Shape identity with partial unique indexes and get-or-create

- Pros: one row per shape per machine, no invented names, the index is the deduplication.
- Cons: three indexes instead of one.

### Option C: Inline derived types in the parameter row as JSON

- Pros: no `plc_type` rows for arrays and strings.
- Cons: the "where used" query (ADR-0018) cannot see them, and the immutability of a referenced type is lost.

## Decision

The project uses **Option B**.

Identity:

- UDTs: `UNIQUE (machine_id, name, revision) NULLS NOT DISTINCT WHERE kind <> 'array' AND string_length IS NULL`.
- Strings: `UNIQUE (machine_id, name, string_length) NULLS NOT DISTINCT WHERE string_length IS NOT NULL`.
- Arrays: `UNIQUE (machine_id, element_type_id, dim_lower, dim_upper) WHERE kind = 'array'`.
- A request names a type with a `TypeExpr` (`domain-model.md` 2.1), for example `ARRAY[1..8] OF STRING(16)` or `ST_Zone@3`. `AddParameter` and `UpdateParameterDefinition` carry it in `type`. `DefineUdt` carries it in `members[i].type`. The text is an input format only. The catalog stays relational (ADR-0006).
- `EnsureDerivedType` (internal operation, ADR-0016) resolves the `TypeExpr`, the innermost element first. It finds a primitive and a struct revision by lookup. For a `STRING(n)`, a `WSTRING(n)`, and an array it does `INSERT ... ON CONFLICT DO NOTHING` and then selects the row. It is the only path that creates string and array rows. `E-PLT-013 DerivedTypeInvalid` covers a parse error, an unknown struct or revision, and a type that is not on the platform. It also covers more than 3 dimensions and a bound or a length outside its range. For a type from a `TypeExpr`, `E-PLT-013` is the only code for a type that is not on the platform. A type above a size limit is `E-PLT-018 TypeTooLarge`: more than 16 384 leaves, more than `json.max_depth` levels, or more than 1024 members.
- `EnsureDerivedType` and `DefineUdt` take `pg_advisory_xact_lock(hashtextextended(machine_id::text || ':' || name, 0))` before the insert. An array has no name, so its key uses the text `ARRAY`. There is no row lock on `plc_type`. The unique indexes are the second line (`domain-model.md` 5.1).
- `plc_type_string_length_required`: a `STRING` or `WSTRING` primitive has `string_length`, every other row has none. `plc_type_string_length_range` allows 1 to 16382. The domain applies the range of the platform from the `StringLength` table of `domain-model.md` 2.1, which is the only definition. `size_bytes` is `INTEGER`, because `WSTRING(16382)` needs 32 768 bytes.
- `dims_valid` is NULL-safe. It rejects `upper - lower >= 65536` and more than 3 dimensions (Assumption A38).
- `recipe_app` gets a column-level INSERT grant on `plc_type` that excludes `created_at`. The trigger function `plc_type_immutable` of `data-model.md` 4.6 closes the members of a struct after the creating transaction, because it compares `created_at` with `now()`. The deferred constraint trigger `plc_type_struct_has_member` of 4.6 requires at least one member at commit.

S7-300 layout:

- `size_bytes` is stored for primitives only. It is the storage unit, and a `BOOL` has 1. `S7Layout` computes the size of structs and arrays in the domain and does not store it.
- A `BOOL` parameter is bit-addressed (`DBX<byte>.<bit>`). In a struct, consecutive `BOOL` members pack 8 per byte. The next non-`BOOL` member starts at the next byte, or at the next even byte when it is wider than 1 byte.
- Every element wider than 1 byte, every `STRING`, every struct, and every array starts on an even byte. A 1-byte element (`BYTE`, `CHAR`, `SINT`, `USINT`) and a packed `BOOL` may start on any byte. A struct and an array end on an even byte. The offset is at most 65535.
- `AddParameter`, `UpdateParameterDefinition` and `ApproveVersion` check that no two parameters of one version occupy the same bit inside one data block. The error is `E-RCP-022 S7AddressOverlap`, status 422. `E-RCP-023 S7AddressMisaligned` covers alignment. `domain-model.md` 2.1 holds the full rules.
- The layout rules apply to every parameter with an address: on S7-300, and on S7-1200 and S7-1500 when the address is set (A31).
- A property test generates random layouts and checks that the computed sizes match a reference table from the Siemens manual.

## Consequences

### Positive

- A machine can have `STRING(16)` and `STRING(20)`. Each exists once.
- The type table does not grow on every parameter edit.
- An export can no longer carry two parameters that share bytes in the PLC.

### Negative

- The domain holds a layout calculator that mirrors the S7-300 rules. It needs a reference table and tests.
- Three indexes to maintain on a small table.

### Risks and mitigations

- Risk: the layout rules differ from the PLC compiler in a corner case. Mitigation: the reference table is test data from the Siemens manual, and the transfer system reports the sizes it computed, which the next release compares.

## Related

- Amends ADR-0006 (derived types). ADR-0016 (EnsureDerivedType), ADR-0018 (revisions).
- `data-model.md` sections 3.3 and 4.6.
- `domain-model.md` section 2.1 and 2.2.
- Review findings: PostgreSQL C2, M1, M2. Architecture F5, F15. Traceability C7, C8, C17.
