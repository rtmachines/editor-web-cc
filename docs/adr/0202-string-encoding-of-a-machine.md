# ADR-0202: STRING encoding of a machine

Status: Accepted. Amends ADR-0181 (b) (the bytes of a `STRING` character on a `Utf8` machine). Amends ADR-0137 (b) and (h) (the characters of a `STRING` value and the unit of a `STRING(n)` length on a `Utf8` machine). Amends ADR-0028 (the rule of a `STRING` character outside Latin-1). Amends ADR-0109 (v) (the scope hashed body). Amends ADR-0119 (c) (`loadable` of a v1 document of a `Utf8` machine). Amended by ADR-0210 (item (f): LF and CR at command input only)
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ03 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- ADR-0137 (h) counts a `STRING(n)` length in characters, and each character is one Latin-1 byte. ADR-0181 (b) tells the transfer client to write each character as one Latin-1 byte.
- The reference file says that both vendors document Latin-1 and UTF-8 `STRING` encodings, and that the capacity counts bytes (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:161` to `:167`).
- On a UTF-8 project, `"éééé"` needs 8 bytes and does not fit `STRING(4)`. The editor accepts it, because it counts 4 characters.
- On a UTF-8 project, the editor refuses a valid value such as `"Ł"` (`U+0141`).
- ADR-0181 considered a setting for each machine as Option B. Its Siemens context showed no need for it.
- `CHAR` does not exist on `CodesysIec` and `TwinCat3` (`data-model.md` §7).
- `CreateRecipe`, `RestoreRecipe`, and `CreateVersion` lock the machine row `FOR SHARE` (`data-model.md` §6).

The project owner decided on 2026-11-19:

1. Add a `STRING` encoding to each machine (Option B of ADR-0181), with the values `Latin1` and `Utf8`.
2. Every S7 machine and every stored machine is `Latin1`.
3. The encoding is set when the machine is created. It can change only while the machine has no recipe version.
4. The export v2 gets the optional field `machine.string_encoding`. The v2 document holds it only for a `Utf8` machine. The scope hashed body holds it too. The system computes it at each export and does not store it.
5. The v1 document and the v1 schema file do not change. A v1 document of a `Utf8` machine has `loadable` `false`, because a v1 consumer assumes Latin-1.

## Decision drivers

- An exported value fits the PLC type, and the PLC gets the approved bytes (R03, R11, R12).
- The domain checks all external input (R38).
- A stored value and a stored hash never change without the user (`reliability.md` §4.4).
- Each refusal uses a code of `reliability.md` §1.2 (R36).

## Considered options

### Option A: Keep Latin-1 and add an Assumption

- Pros: no code change.
- Cons: a plant with a UTF-8 project cannot use the editor for `STRING` values.

### Option B: An encoding for each machine

- Pros: each machine follows the encoding of its PLC project.
- Cons: a new column, a new command, a new export field, and a second unit of the `STRING` length.

### Option C: Only `U+0001..=U+007F` in a `STRING` on these platforms

- Pros: Latin-1 and UTF-8 give the same bytes. No new setting.
- Cons: no `é` and no `ü` in a `STRING` on these platforms.

## Decision

The project uses **Option B**.

- (a) A machine has the field `string_encoding` of the value object `StringEncoding`. The values are `Latin1` and `Utf8`.
- (b) A machine on `S7_300`, `S7_1200`, or `S7_1500` is always `Latin1`. `CreateMachine` and `SetMachineStringEncoding` give `E-PLT-053 StringEncodingNotOnPlatform` for `Utf8` on an S7 platform.
- (c) `CreateMachine` takes the field `string_encoding`. The form proposes `Latin1`. Every stored machine gets `Latin1` through the column default. Thus no stored value changes.
- (d) The new command `SetMachineStringEncoding` changes the encoding of a live machine. It needs `MachineManage`. It locks `machine FOR NO KEY UPDATE` first. Then it requires that no recipe of the machine has a `recipe_version` row.
  - (d1) Each row counts: every status, a discarded draft, and a version of a deleted recipe.
  - (d2) A machine with a version gives `E-PLT-054 StringEncodingFrozen`. The lookup of the machine runs before the transaction, as in `SetMachineDataBlock`. A missing or archived machine there gives `404 E-WEB-005`, because no live machine has the code. A machine that is archived after the lookup gives `409 E-PLT-010 MachineArchived` under the lock.
  - (d3) The lock conflicts with the `FOR SHARE` lock of `CreateRecipe`, `RestoreRecipe`, and `CreateVersion`. Thus no version appears between the check and the commit. No trigger repeats the check.
  - (d4) The command writes the audit entry `MachineStringEncodingSet` with the full `machine` row in both states, as `MachineDataBlockSet`.
- (e) A `Latin1` machine keeps ADR-0137 (h) and ADR-0181. A `STRING` character is in `U+0001..=U+00FF`, and the length counts characters.
- (f) On a `Utf8` machine, a `STRING` character is any Unicode scalar except `U+0000`. A `STRING(n)` length counts UTF-8 bytes. A value with more bytes gives `E-RCP-005` with the expected form in bytes. This item amends ADR-0137 (b) and (h). It also amends the rule of ADR-0028 that a `STRING` character outside Latin-1 gives `E-RCP-005`. That rule applies to a `Latin1` machine only.
- (g) The unit of item (f) also applies to `max_length` of a process limit and to the `StringLength` table (ADR-0137 (j)). The range `1..=255` of ADR-0141 does not change. On both encodings, `n` is the payload capacity in bytes.
- (h) `CHAR` does not exist on `CodesysIec` and `TwinCat3`. Thus a `Utf8` machine has no `CHAR` leaf. If a later ADR adds `CHAR` there, a `CHAR` value on a `Utf8` machine is in `U+0001..=U+007F`.
- (i) On a `Utf8` machine, the transfer client writes each `STRING` character as its UTF-8 bytes. ADR-0181 (b) applies to a `Latin1` machine only. This item amends ADR-0181 (b).
- (j) The v2 document holds `machine.string_encoding` only for a `Utf8` machine. It sits directly after `machine.plc_platform`. For a `Latin1` machine the field is absent.
  - (j1) When the field is present, the scope hashed body holds it too. The system reads it from the machine row at each export and does not store it. This item amends ADR-0109 (v).
  - (j2) This ADR adds no byte to a v1 document or a v2 document of a `Latin1` machine. It changes no stored `content_sha256` and no stored `scope_sha256`, also in the stored `VersionExported` entries. Only ADR-0201 (i) can change `loadable` of a stored version, as ADR-0180 (j) does on S7. Every stored machine is `Latin1` (item (c)).
  - (j3) The v2 schema gets an optional property `string_encoding` on `machine`. Its enum holds the value `Utf8` only. The schema permits the property only when `machine.plc_platform` is `CodesysIec` or `TwinCat3`. A JSON Schema `if` and `then` rule, or an equal rule, holds this condition. Thus a document of an S7 machine with the property fails the schema.
  - (j4) An old v2 consumer checks documents against the old schema. It refuses the document of a `Utf8` machine and so fails closed.
  - (j5) This item amends the rule of the v2 schema description that a new field needs `schema_version` `"3"`, for this one optional field. This is no breaking change under ADR-0019. Each existing document and each existing hash stays valid. Only a `Utf8` machine gives a new document, and no such machine exists before this ADR.
- (k) The v1 document, the v1 schema file, the v1 hashed body, and `content_sha256` do not change. Thus no stored hash changes.
- (l) A v1 document of a `Utf8` machine has `loadable` `false`. This item amends ADR-0119 (c). The version page of such a version shows a notice that the v1 document is not loadable.
- (m) The value check takes the encoding of the machine. The snapshot check, `ApproveVersion`, the export check, and the version page use the same rule. The rule has no branch of `RuleSet`, because the encoding cannot change after the first version.
- (n) Storage. The column `machine.string_encoding` is `TEXT NOT NULL DEFAULT 'Latin1'`. One `CHECK` permits the two values. A second `CHECK` permits `Utf8` only on `CodesysIec` and `TwinCat3`. `recipe_app` gets `UPDATE (string_encoding)` on `machine`.
- (o) The migration adds the column and both checks as `NOT VALID`. A second migration runs `VALIDATE CONSTRAINT` on both checks, as `0013` and `0015` do (ADR-0157).
- (p) The form of a new machine and the page of a machine show the encoding. The page offers the change only while item (d) permits it.
- (q) Rollback. The release before this ADR cannot read the encoding. These facts limit a rollback of the image only on the newer schema:
  - (q1) The previous release does not read `machine.string_encoding`. Its v1 document of a `Utf8` machine can have `loadable` `true`, and no document holds the encoding. A transfer client then writes Latin-1 bytes (ADR-0181 (b)) to a `Utf8` PLC.
  - (q2) The previous release reads an unknown audit action as corrupt (`crates/recipe-db/src/transaction/audit_reads.rs`). It does not know the action `MachineStringEncodingSet`. An entry with this action stays in `audit_log` also after a change back to `Latin1`.
  - (q3) The path "Roll back on the newer schema" changes only the image. The operator uses it only when the database holds 0 machines with `Utf8`. The database must also hold 0 entries of `MachineStringEncodingSet`. The operator runs the query of the runbook to count both.
  - (q4) If a count is not 0, the operator keeps the new image and rolls forward. The other path restores the `_premigrate` set. The restore loses every write since that dump.

## Consequences

### Positive

- A `STRING` value on a `Utf8` machine fits its byte capacity, and the PLC gets the approved bytes.
- A plant with a UTF-8 project can enter characters outside Latin-1.
- No stored value, no stored hash, and no v1 field changes.

### Negative

- A new column, a new command, a new route, two new codes, and a new audit action.
- A plant with a `Utf8` machine must use the export v2. A v1 document of such a machine is never loadable.
- An old v2 consumer refuses the v2 document of a `Utf8` machine.

### Risks and mitigations

- Risk: an old v2 consumer refuses the document of a `Utf8` machine, because its schema does not know the field. The owner decided this on 2026-11-19. The refusal fails closed. This ADR adds no byte to a `Latin1` document (item (j2)). Mitigation: the runbook tells the plant to update the consumer before it sets `Utf8`.
- Risk: a rollback of the image only sends Latin-1 bytes to a `Utf8` PLC, or breaks the audit read. Mitigation: item (q) and the runbook section "Roll back on the newer schema".
- Risk: a plant sets an encoding that differs from the PLC project. Mitigation: the runbook names the check before a plant adds a `CodesysIec` or `TwinCat3` machine.

## Related

- ADR-0019 (export contract), ADR-0028 (value rules), ADR-0109 (export document v2), ADR-0119 (export check), ADR-0137 (text rules)
- ADR-0141 (string length on these platforms), ADR-0156 (data block number of a machine), ADR-0181 (Latin-1 bytes)
- `domain-model.md` §2.1 (`Machine`, `StringLength`), §5.1, `data-model.md` §3.1, §5.2, §6, `api.md` §2.2, §3.3, §6.1, §6.4, `reliability.md` §1.2, §4.4, `docs/ops/runbook.md` ("Roll back on the newer schema")
- `overview.md` §7: R03, R11, R12, R36, R38
