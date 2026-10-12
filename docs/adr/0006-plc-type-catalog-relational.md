# ADR-0006: Relational PLC type catalog with immutable UDT revisions

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft requires the IEC data types of CODESYS and TwinCAT 3, the Siemens S7-1200, S7-1500, and S7-300 types, one-dimensional and multidimensional arrays, user-defined types (UDTs), and nested structures. A change of a UDT structure must create a new recipe version.

The draft says "required" types but gives no list. The list per platform is Assumption A3 and is in `data-model.md` section 7.

## Decision drivers

- The structure of a type must be queryable: "which parameters use UDT X" must be one SQL query.
- An `Approved` version must keep the exact type structure it was approved with.
- Validation of a value against its type must be exact (array size, member names, scalar range).
- The database must reject a type that does not exist on the platform of the machine (I6).

## Considered options

### Option A: Relational catalog

Tables `plc_type` (kind, name, platforms, dimensions, element type, revision link) and `plc_type_member` (ordered struct members). Values stored as JSONB and validated in the domain against the resolved type tree.

- Pros: constraints and foreign keys protect the structure, queries over usage are simple, UDT revisions are rows.
- Cons: resolving a deep type needs a recursive query or several reads. Values are opaque to the database.

### Option B: JSON Schema per parameter

- Pros: one column, standard format, generic validators exist.
- Cons: usage queries need JSON path scans, no foreign keys between a UDT and its members, a UDT change must rewrite every schema that embeds it.

### Option C: Type as a string parsed in the application

For example `ARRAY[1..8] OF MyUdt`.

- Pros: close to the PLC source text.
- Cons: no database protection, the parser is a source of bugs, UDT members are not addressable, no revision control.

## Decision

We use Option A.

- `plc_type` rows are immutable. An edit of a UDT inserts a new row with `supersedes = <old id>`. Parameters that reference the old row keep it (ADR-0009).
- Primitive types are seeded by a migration with `platforms` as `TEXT[]`. The application does not insert primitive rows.
- An array type is a row with `kind = 'array'`, `element_type_id`, and dimensions as two integer arrays (lower bounds, upper bounds).
- A struct type is a row with `kind = 'struct'` and ordered rows in `plc_type_member`.
- `STRING` and `WSTRING` carry `string_length`.
- The domain resolves a `TypeTree` from the rows and rejects cycles (I8).
- A parameter value is JSONB with the shape of the type tree. The domain validates every leaf (I7). The database does not validate the JSON.
- A trigger on `parameter` checks that the platform of the version is in `plc_type.platforms` (I6).

## Consequences

### Positive

- "Where is UDT X used" and "which parameters have no address" are plain SQL.
- An approved version is reproducible years later. The UDT revision it used still exists.
- New platforms or types are a seed migration, not a code change.

### Negative

- Reading a parameter with a deep type needs a recursive query. The read model caches resolved type trees per version.
- The application must keep JSON values and type rows consistent. The database cannot help with the JSON.

### Risks and mitigations

- Risk: the type matrix (Assumption A3) is wrong for a plant. Mitigation: the matrix is data. A migration corrects it without a code change.
- Risk: a UDT with hundreds of revisions makes the catalog hard to browse. Mitigation: the UI shows the latest revision by default and a history on request.

## Related

- ADR-0007 (limits per leaf of the type tree)
- ADR-0009 (versions reference immutable type revisions)
- `domain-model.md` section 2.1, invariants I6, I7, I8
- `data-model.md` (`plc_type`, `plc_type_member`, section 7)
