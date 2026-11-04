# ADR-0119: Loadable flag and the current rules of values and addresses

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

ADR-0019 sets `loadable` to `true` only for an `Approved` version. `api.md` section 6.1 holds the same rule. The JSON Schema files of v1 and v2 enforce it: an `Approved` document must have `loadable` `true`.

ADR-0114 and ADR-0117 add value rules. ADR-0116 adds two address rules. An approved version can hold a value, a limit, or an S7 address that breaks them. Examples are a `DATE` after 2168-12-31, a `REAL` number with 8 significant digits, and a `DINT` at `DB1.DBD65534`.

Approved versions are immutable (ADR-0009). `ExportVersion` rebuilds the hashed body from the stored rows. It does not check a value against its type or an address against its data block. Such a version therefore exports with `loadable` `true`.

The transfer client must check `loadable` and the hash before a PLC load (`api.md` section 6.3).

## Decision drivers

- An exported value loads into the PLC with the value that the user sees (R03, R12, R31).
- An exported address fits a data block of the PLC (R03, R14).
- The hashed body of one version never changes (`api.md` section 6.3, ADR-0019).
- A consumer gets one clear signal that it must not load a document.

## Considered options

### Option A: Keep `loadable` `true`

- Pros: no change of the export.
- Cons: a transfer client loads a value that the PLC does not hold as stored, or an address outside its data block.

### Option B: Refuse the export with a new code

- Pros: no document of an affected version leaves the system.
- Cons: the version is lost for audit and rollback use. A new code and a new error answer on five routes.

### Option C: Set `loadable` to `false`

- Pros: the document stays available. The consumer rule of `api.md` section 6.3 already stops the load.
- Cons: the meaning of `loadable` and the JSON Schema files change.

## Decision

The project uses **Option C**.

- (a) `ExportVersion` checks every stored value and limit of the version against the current value rules. It uses the current rule set of ADR-0120, the same check as a read of ADR-0118 (b). A part that fails the earlier rule set is a storage defect (ADR-0118 (d)).
- (b) `ExportVersion` also checks every S7 address of the version with the end rule and the data block rule of ADR-0116 (a) and (c).
- (c) `loadable` is `true` only when `version.status` is `Approved` at export time and no value, limit, or address breaks a rule. Otherwise `loadable` is `false`.
- (d) The hashed body and `content_sha256` do not change. The hash compare of `api.md` section 6.3 runs first, and a difference still gives `500 E-EXP-002`.
- (e) The answer stays `200` with the document. The file name, `Content-Disposition`, and the `VersionExported` entry do not change. An `Approved` version gets no `RETIRED_` prefix.
- (f) A v2 document checks every parameter of the version, not only the parameters of its scope. The whole-version hash check of ADR-0109 (w) uses the same set.
- (g) The JSON Schema files of v1 and v2 keep the rule that a `Retired` document has `loadable` `false`. They drop the rule that an `Approved` document has `loadable` `true`.
- (h) `schema_version` stays `"1"` and `"2"`. The fields, their types, and the hashed body do not change, and a consumer already checks `loadable`.
- (i) The check uses the rules at export time. A later rule change can change `loadable` of the same version. The hash stays the same.
- (j) The check runs in the blocking task of the hash compare, after the compare and before the facts of the document. It works only on rows that the export already read.
- (k) The check therefore waits for no I/O and no lock inside the export transaction, as `reliability.md` section 7.4 requires. `timeout.export` bounds the blocking task.
- (l) The version page of an `Approved` version runs the same check. When a part breaks a rule, the export area of ADR-0052 (e) shows a notice with the code of each broken rule.
- (m) The export link names that notice in `aria-describedby`. To run the check, the version page of an `Approved` version reads the values of every parameter. This amends ADR-0052 (e) and ADR-0108 (b).

## Consequences

### Positive

- A transfer client that follows `api.md` section 6.3 does not load an affected version.
- The audit and rollback use of an affected version stays possible.

### Negative

- An `Approved` document can have `loadable` `false`. A consumer that reads only `version.status` misses this.
- Each export parses every stored value and limit and computes the layout size of each parameter with an S7 address.
- The version page of an `Approved` version reads every value of the version. The page then reads as much as an export.

### Risks and mitigations

- Risk: a consumer checks the document with an older copy of the schema file and refuses an `Approved` document with `loadable` `false`. Mitigation: the refusal stops the load, which is the intent of the flag.

## Related

- ADR-0019 (export contract, `loadable`), ADR-0109 (transfer routes, v2 document), ADR-0009 (immutable snapshot)
- ADR-0052 (export link and notice), ADR-0108 (version page reads), ADR-0120 (rule sets)
- ADR-0028 (stored check), ADR-0114, ADR-0117 (the new value rules), ADR-0116 (the new address rules), ADR-0118 (stored values that break a newer rule)
- `api.md` sections 2.7, 6.1, 6.3, 6.4, `reliability.md` section 7.4
- `overview.md` section 7: R03, R12, R14, R31
