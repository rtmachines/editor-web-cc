# ADR-0002: Export contract as a versioned JSON document

Status: Superseded by ADR-0019
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "It edits recipe data only. It does not communicate with machines." The draft does not say how an `Approved` version reaches the PLC. Without an answer the application has no output and the recipe data has no use.

## Decision drivers

- The application must not talk to a PLC (R03).
- An external system must be able to read an approved version without reading the database.
- The contract must be stable across application releases.
- An Operator must be able to get the approved version with no help from a developer (Assumption A4).

## Considered options

### Option A: Out of scope

- Pros: no work now.
- Cons: the application has no output. Every plant would read the database directly. The schema would become a public contract.

### Option B: File export in the UI

- Pros: simple, works with a USB stick or a shared folder, no new authentication.
- Cons: a transfer system cannot pull the file without a human.

### Option C: Read-only API for a transfer system

- Pros: automation is possible, the transfer system can poll for the current approved version.
- Cons: needs machine authentication, needs a stable schema for the response.

## Decision

We use Option B and Option C together with one shared document format.

- The export format is a JSON document with a `schema_version` field. The first version is `1`.
- `recipe-export` owns the format. It serializes one `Approved` version. It never serializes a `Draft` or a `Retired` version.
- The UI offers a download of the document for an `Approved` version.
- The endpoint `GET /export/v1/recipes/{ref}/approved` returns the same document. It uses the session of the user in the first release. API tokens for a transfer system are a later change (ADR-0004).
- The document contains the machine code, platform, recipe reference, version number, approval time, and approver login. For each parameter it contains the unit path, tag, type tree, S7 address when present, limits, and value.
- Every export is written to the audit log (Assumption A8).
- The transfer of the document to the PLC is out of scope.

## Consequences

### Positive

- The application stays free of PLC drivers and PLC libraries.
- The database schema is internal. Only the JSON document is a contract.
- A transfer system can be built later by any team in any language.

### Negative

- A change of the document needs a new `schema_version` and a compatibility note.
- The application must keep the serializer for every published schema version.

### Risks and mitigations

- Risk: a plant needs a vendor format (for example a TIA Portal import file). Mitigation: a converter reads the JSON document. The application does not change.
- Risk: a transfer system reads a `Retired` version from a stale file. Mitigation: the document carries `status`, `version`, and `exported_at`. The transfer system must check them.

## Related

- ADR-0004 (authentication, API tokens later)
- ADR-0005 (export is audited)
- `domain-model.md` section 1 (Export context), section 5
- `api.md` (export endpoint)
