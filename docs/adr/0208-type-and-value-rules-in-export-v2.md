# ADR-0208: Type and value rules in the export schema v2

Status: Accepted. Amends ADR-0109 (the content of the v2 schema file). Amends ADR-0189 (more rules of the v2 schema file next to the address pattern)
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 found R01 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §3). R01 is the IEC twin of the Siemens finding that ADR-0189 closed.

These facts shape the decision:

- `export-v2.schema.json` lets each leaf of `value` be any string or a boolean. A `BOOL` parameter with the string `"true"` passes the schema.
- The enum `type_ref` holds every primitive name on every platform. A `CHAR` or `WCHAR` reference on `TwinCat3` passes the schema, but `data-model.md` §7 gives neither type there.
- `string_length` has the maximum 16382 on every platform. The `StringLength` table of `domain-model.md` §2.1 gives `1..=255` on `CodesysIec` and `TwinCat3`.
- `members` has no maximum. `type.max_members` is 1024 (`reliability.md` §7.11).
- `api.md` §6 says that the schema file is the source of truth. ADR-0189 added the S7 address pattern to v2 only, because the pattern refuses no document that the writer can emit.
- A range check, such as `BYTE` `"256"`, needs the type of the leaf. For a leaf below a type reference with a UUID, the schema cannot find the type.

The project owner decided on 2026-11-19: the schema v2 refuses what the editor refuses, in the pattern of ADR-0189. The v1 file does not change.

## Decision drivers

- The schema file states the grammar of each field (`api.md` §6, ADR-0019).
- No breaking change of a published schema version (ADR-0019).
- The domain checks all external input, and a consumer can check the document (R38).

## Considered options

### Option A: Add the type and kind rules to v2 only

- Pros: a v2 consumer refuses more faulty documents with the schema alone. The v1 file stays as the first integration knows it.
- Cons: the two schema files differ in more fields.

### Option B: The rules in v1 and v2

- Pros: one rule set in both files.
- Cons: the owner keeps v1 unchanged.

### Option C: No change, and a list of the consumer checks only

- Pros: no schema change.
- Cons: a consumer that trusts the schema accepts a faulty document.

## Decision

The project uses **Option A**.

- (a) The primitive names of `type_ref` follow the platform. For each value of `machine.plc_platform`, the v2 file permits only the primitives with "yes" in that column of `data-model.md` §7. Thus `CHAR` and `WCHAR` fail on `CodesysIec` and `TwinCat3`.
  - (a1) The comparison set is the primitives of the platform that the document names by name. A name sits at the three positions `parameters[].type_id`, `types[].members[].type_id`, and `types[].element_type_id`.
  - (a2) `STRING` and `WSTRING` have a length, so the document names them by a UUID reference to a `types[]` entry. Thus a bare name `STRING` or `WSTRING` fails the schema on every platform.
  - (a3) A UUID reference stays valid at the three positions.
- (b) `types[].string_length` follows the `StringLength` table of `domain-model.md` §2.1 for the platform and the type name. A `WSTRING` entry fails on `S7_300`.
- (c) `types[].members` has at most 1024 entries (`type.max_members`).
- (d) A parameter whose `type_id` is a primitive name gets the JSON kind of its `LeafClass`. A `BOOL` value is a boolean. A `CHAR` or `WCHAR` value is a string with one character. Every other value is a string with the canonical decimal text of `reliability.md` §4.4.
- (e) The schema does not check a range, and it does not check a leaf below a type reference with a UUID. `api.md` §6.3 names these checks as the type-aware pass of a consumer.
- (f) The schema does not hold a rule that a stored row can break and that the export still emits. Thus the rules of ADR-0201, ADR-0203, and ADR-0204 are not in the schema.
- (g) `export-v1.schema.json` does not change. `schema_version` stays `"1"` and `"2"`.
- (h) The writer, the hashed body, `content_sha256`, and `scope_sha256` do not change.
- (i) Schema tests of `recipe-export` give one passing and one failing document for each item from (a) to (d). Every golden v2 document still passes the schema.
- (j) `api.md` §6.4 names the rules of v2.

## Consequences

### Positive

- A v2 consumer refuses a wrong primitive, a wrong length, too many members, and a wrong JSON kind with the schema alone.

### Negative

- A v1 consumer still gets none of these rules from the schema.
- A consumer still needs the type-aware pass of item (e).

### Risks and mitigations

- Risk: a stored row breaks a rule of items (a) to (d), and the writer emits a document that the schema refuses. Mitigation: the tasks of the change check the stored-data matrix, and item (f) keeps such rules out.
- Risk: the schema and the domain differ later. Mitigation: the tests of item (i) run the same cases against the domain and the schema.

## Related

- ADR-0019 (export contract), ADR-0109 (export document v2), ADR-0189 (address pattern in v2), ADR-0202 (`machine.string_encoding` in v2)
- `api.md` §6.1, §6.3, §6.4, `domain-model.md` §2.1 (`StringLength`, `LeafClass`), `data-model.md` §7
- `overview.md` §7: R03, R38
