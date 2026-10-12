# ADR-0019: Export integrity and the export format

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

ADR-0002 defined a JSON export. The design review found that the export was the only external interface. The documents defined it in five different ways (architecture F18, traceability B63 to B65). Retired versions were both forbidden and allowed. Translations were both included and excluded. The document was called immutable although `version.status` changes. `types[]` had no revision.

The security review (C1) found that nothing lets the transfer system detect that a file was edited after download. It also found that the export serialized whatever the database held at that moment.

The owner accepted two decisions. The export carries a content hash now and a signature later. Administrator and ProcessEngineer can export a Retired version.

## Decision drivers

- One source of truth for the export format (R03, R36).
- The transfer system must detect a tampered or stale file (R31).
- Only an Approved version is loadable into a PLC (R18, R27).
- The format must not change in a breaking way after the first integration.

## Considered options

### Option A: Keep the prose definition and add a hash later

- Pros: no work now.
- Cons: adding `content_sha256` later is a breaking change to v1.

### Option B: One JSON Schema file, a content hash in v1, Retired export marked as not loadable

- Pros: golden-file tests, the hash is cheap, the transfer system has one rule.
- Cons: the schema file and the Rust serializer must stay in step. A test enforces it.

### Option C: Signed documents from the start

- Pros: strongest guarantee.
- Cons: a key to manage before any consumer exists.

## Decision

The project uses **Option B**. Option C is the planned next step.

Format:

- The source of truth is `crates/recipe-export/schema/export-v1.schema.json` (JSON Schema 2020-12). `api.md` section 6 describes it. A golden-file test checks every example against the schema.
- Crates. The module `export` of `recipe-domain` owns the hashed-body builder `ExportBody`, the canonical serializer, the hash function, and `ExportError`. `recipe-export` keeps the JSON Schema file, the JSON writer of the HTTP response, and the golden tests. It depends on `recipe-domain`, `serde`, and `serde_json`, and only `recipe-web` uses it. Of the workspace crates, `recipe-app` depends on `recipe-domain` only, so `ApproveVersion` computes the hash without `recipe-export`.
- Fields: `schema_version`, `exported_at`, `exported_by`, `loadable`, `content_sha256`, `machine { code, name, plc_platform }`, `recipe { reference_number, name }`, `version { number, status, approved_at, approved_by }`, `types[]`, `parameters[]`.
- `types[]` holds every UDT, string, and array type in use. Each entry has `id`, `name`, `revision`, `kind`, `string_length`, and the array or struct fields. A struct member is `{name, type_id}`.
- `parameters[]` holds `unit_path`, `tag`, `key`, `name_en`, `type_id`, `s7_address`, `limits[]`, `value`. The field name is `type_id` in every place.
- Array order: `types[]` by `id` in byte order. `parameters[]` by `unit_path` with segments compared as integers, then `tag` in byte order. `limits[]` by `leaf_path` in byte order. `members[]` by the member ordinal of the struct. `api.md` 6.3 states this order as the rule for a consumer that builds the hashed body again.
- `parameter.ordinal` is a sort order of the UI only. It is not exported and not hashed, so every sort key of `parameters[]` is in the document.
- A type reference is the primitive name for a primitive without a length. For every other type it is a `types[].id`.
- Every numeric leaf of a value and every limit is a JSON string with the canonical decimal text. The canonical form of RFC 8785 writes a JSON number as a double, and a decimal can lose precision.
- Translations and descriptions are not exported. ADR-0012 is amended on this point.
- `loadable` is `true` only for an Approved version.

Integrity:

- The hashed body holds identifiers and values only: `machine.code`, `machine.plc_platform`, `recipe.reference_number`, `version.number`, `types[]`, and `parameters[]` without `parameters[].name_en`.
- Outside the hash: `schema_version`, `exported_at`, `exported_by`, `loadable`, `version.status`, `version.approved_at`, `version.approved_by`, `machine.name`, `recipe.name`, `parameters[].name_en`, and `content_sha256`. Names are informational. A rename or a retirement therefore does not change the hash.
- `ApproveVersion` computes `content_sha256` over the RFC 8785 canonical form of the hashed body, through the module `export` of `recipe-domain`. It stores the hash in `recipe_version.content_sha256`. The transition trigger freezes it.
- `ExportVersion` rebuilds the hashed body for an Approved and for a Retired version. It compares the hash with the stored value and refuses with `E-EXP-002 ContentHashMismatch` when it differs. The audit entry records the hash.
- The hashed body is reproducible for a given `(reference_number, number)`. The word "immutable" is not used for the document.

Endpoints:

- `GET /export/v1/recipes/{ref}/approved`: every role with `VersionExport`. 404 `E-EXP-001 NoApprovedVersion` when no version is Approved.
- `GET /export/v1/recipes/{ref}/versions/{n}`: Approved or Retired. Administrator and ProcessEngineer only. A Retired version gets `loadable: false` and the file name prefix `RETIRED_`.
- Every export writes an audit entry (A8).

Planned next step (not in this release): an Ed25519 signature in a `signature` field, with the key in a Docker secret. Per-system API tokens for a transfer system come later (ADR-0004, option C).

## Consequences

### Positive

- The transfer team builds against one file with tests.
- A changed setpoint in a downloaded file is detected by the hash.
- A rollback to a Retired layout is possible for an engineer and impossible for an operator.

### Negative

- The canonical form must be stable across Rust versions. The project pins the serializer and tests the hash against stored vectors.
- `content_sha256` is computed at approval. A change of the export format needs a migration that recomputes it, or a `schema_version` bump with both hashes stored.

### Risks and mitigations

- Risk: a consumer ignores `loadable`. Mitigation: the `RETIRED_` prefix and `status` give two more signals, and the signature step adds a third.

## Related

- Supersedes ADR-0002. Amends ADR-0012.
- ADR-0014 (sessions), ADR-0018 (`types[].revision`), ADR-0022 (approval computes the hash).
- `api.md` sections 2.7 and 6.
- `data-model.md` sections 3.10 and 6.
- Review findings: security C1, M7. Architecture F18. Traceability B63, B64, B65, C3, C4.
