# ADR-0137: No U+0000 in a text value, and the unit of a string length

Status: Accepted. Amends ADR-0028 (value and text rules) and ADR-0022 (range of a `CHAR` value)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found two faults in the text rules. `docs/plan/review/2026-11-06-plc-type-review.md` holds them as #1 and #12.

These facts shape the decision:

- `domain-model.md` §2.1 gives a `CHAR` value the range `U+0000..=U+00FF` and a `WCHAR` value the range `U+0000..=U+FFFF`. ADR-0022 gives the same `CHAR` range.
- `ParameterValue::parse` accepts `U+0000` in a `CHAR`, `WCHAR`, `STRING`, or `WSTRING` leaf. A form text `value=%00` gives the same value.
- PostgreSQL `jsonb` refuses `U+0000` in a string. The write fails with `unsupported Unicode escape sequence`. The review checked this in a `postgres:16.15-bookworm` container.
- PostgreSQL also cannot store `U+0000` in a `TEXT` column (ADR-0035).
- Thus the write statement of the command fails. The user gets a storage error, and not the field line of `E-RCP-005`.
- IEC 61131-3 permits the value 0 in a `CHAR` or a `WCHAR`. An S7 `STRING` or `WSTRING` holds its length in its header, so a zero byte can be data on the PLC.
- A `STRING` or `WSTRING` of CODESYS and of TwinCAT 3 ends at its first zero.
- `domain-model.md` §2.1 calls `StringLength` "the number of characters". `reliability.md` §4.4 calls it a "byte length" in Latin-1 or in UTF-16 units.
- The code counts characters for `STRING` and UTF-16 code units for `WSTRING`. The value check and the `max_length` check use the same count.
- A `WSTRING(1)` value "😀" has one character and two UTF-16 units. The two documents give different answers for it. The code refuses it.

The project owner decided on 2026-11-06 (fix plan, question Q-E). The owner kept the decision on 2026-11-07. The domain refuses `U+0000` with `E-RCP-005`. The reason is the storage, because PostgreSQL cannot hold `U+0000` in `jsonb` or in `TEXT`.

## Decision drivers

- A value that the domain accepts is a value that storage can hold (R31, R38).
- Validation runs before the first write of the command. A bad value gives its field line (ADR-0041).
- One length has one unit in every document and in the code.
- The rule does not change a stored row or a content hash.

## Considered options

### Option A: The domain refuses U+0000

- Pros: one contract from the parse to the storage. The user gets `E-RCP-005` under the field. No new error code.
- Cons: a user cannot type `U+0000` in a text value. The domain range is narrower than the vendor range.

### Option B: The domain stores an escaped form

- Pros: the domain accepts every Unicode scalar, also the value 0 of a `CHAR` and a zero byte of an S7 `STRING`.
- Cons: the stored text, the export, and the content hash differ from the value. A transfer client must decode the text.

### Option C: Keep the rule and map the PostgreSQL error

- Pros: no change of the value rules.
- Cons: the write statement fails with SQLSTATE `22P05` and aborts the transaction. `map_error` gives `StorageError::Other`, so the answer has no field line. The documents still give a range that storage cannot hold.

## Decision

The project uses **Option A**. The rule is a deliberate narrowing of the vendor range, in the pattern of ADR-0115. Option B stays the path for a later ADR. The items below are the rules.

- (a) A `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value holds no `U+0000`. A `CHAR` value is one Unicode scalar in `U+0001..=U+00FF`. A `WCHAR` value is one Unicode scalar in `U+0001..=U+FFFF`.
- (b) A `STRING` character is in `U+0001..=U+00FF`. A `WSTRING` character is any Unicode scalar except `U+0000`.
- (c) A value with `U+0000` gives `E-RCP-005` with the expected form of its type. The check is part of the leaf conversion of `ParameterValue`, so it runs before the first write of the command. One exception: `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` can insert a new derived type (`EnsureDerivedType`) before the check. A refused command commits no change.
- (d) Every path that builds a value applies the rule. These are a JSON value, the text of a form field, and a new leaf of `RebindTypeRevision`.
- (e) A process limit holds no text, because `max_length` is a number. The rule therefore applies only to a value. The `Text` rules of `reliability.md` §4.3 already refuse a control character in a name or a description.
- (f) This ADR changes no other character of a text value.
- (g) The rule is in the current and in the earlier rule set of ADR-0120. PostgreSQL never stored `U+0000` in `jsonb`, so the rule affects no stored row. ADR-0120 needs no change.
- (h) A `STRING(n)` length counts characters. Each character is one Latin-1 byte, so the count equals the byte count of the PLC.
- (i) A `WSTRING(n)` length counts UTF-16 code units. A character outside the Basic Multilingual Plane counts 2.
- (j) The units of (h) and (i) apply to the value check, to `max_length` of a process limit, and to the range table of `domain-model.md` §2.1.
- (k) The expected form of a `CHAR` or `WCHAR` fault starts at `U+0001`. The expected form of a `STRING` or `WSTRING` fault names the rule "without `U+0000`".

## Consequences

### Positive

- A value with `U+0000` gives `422 E-RCP-005` with a field line, and not a storage error.
- `domain-model.md`, `reliability.md`, and the code use one unit for a string length.

### Negative

- A user cannot store `U+0000` in a text value.
- A `CHAR` or `WCHAR` value cannot hold the value 0 on any platform.
- An S7 `STRING` or `WSTRING` value cannot hold a zero byte.
- The UI phrases of three expected forms change in the bundles `pl` and `en`.

### Risks and mitigations

- Risk: a recipe needs the value 0 in a `CHAR` or a zero byte in an S7 `STRING`. Mitigation: the editor never stored such a value, because PostgreSQL refuses it. A later ADR can add the escaped form of Option B.
- Risk: a test generator draws `U+0000` for a valid value. Mitigation: the property tests of `ParameterValue` draw from `U+0001`.

## Related

- ADR-0028 (value and text rules), ADR-0022 (leaf classes, `CHAR` range), ADR-0035 (no `U+0000` in a text column)
- ADR-0041 (field lines), ADR-0115 (ranges narrower than the vendor range), ADR-0120 (earlier rule set)
- `domain-model.md` §2.1 (`StringLength`, `LeafClass`)
- `reliability.md` §1.2 (`E-RCP-005`), §4.3, §4.4
- `overview.md` §7: R11, R12, R31, R38
