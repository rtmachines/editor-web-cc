# ADR-0026: Platform ranges of DATE and DATE_AND_TIME

Status: Accepted
Date: 2026-10-13
Deciders: project owner

## Context

The base-unit table of `domain-model.md` section 2.1 gives the range of the count of each time type. For two types it says only "The range of the platform":

- `DATE` on every platform,
- `DATE_AND_TIME` on `CodesysIec` and `TwinCat3`.

The parser `IecLiteral` rejects a count outside the range (`domain-model.md` section 2.1). The plan review of step P2.1 found no number for these rows in a normative document. An implementer would have to take the values from a vendor manual without a record.

The table gives the S7 range of `DATE_AND_TIME` as "1990-01-01 to 2089-12-31". It does not say whether the last day is complete.

## Decision drivers

- A limit and a value check use the same range in every place (R09, R38).
- The range must be the range of the PLC, so that an exported value loads into the controller (R03, R11, R12).
- Every range in code has a row in a normative document.

## Considered options

### Option A: Write the vendor ranges into the table

- Pros: one source for the code and the tests. The owner checks each value once.
- Cons: a vendor change needs a new ADR.

### Option B: Derive the range from the storage width

For example, an unsigned 32-bit count of seconds on CODESYS.

- Pros: no vendor lookup.
- Cons: the rule is not in a document. The storage width of S7 `DATE` gives an upper day that differs between S7-300 and the other S7 platforms.

## Decision

The project uses **Option A**. The owner checked the values against the vendor manuals as of the design date.

| Type | Platform | Range |
|---|---|---|
| `DATE` | `CodesysIec`, `TwinCat3` | 1970-01-01 to 2106-02-07 |
| `DATE` | `S7_1200`, `S7_1500` | 1990-01-01 to 2169-06-06 |
| `DATE` | `S7_300` | 1990-01-01 to 2168-12-31 |
| `DATE_AND_TIME` | `CodesysIec`, `TwinCat3` | 1970-01-01-00:00:00 to 2106-02-07-06:28:15 |
| `DATE_AND_TIME` | `S7_1500`, `S7_300` | 1990-01-01-00:00:00 to 2089-12-31-23:59:59 |

Both ends are inside the range. The counts follow the base units of Assumption A43: days for `DATE` and seconds for `DATE_AND_TIME`, from 1970-01-01.

## Consequences

### Positive

- The parser, the limit check, and the tests use one table.
- The S7 range of `DATE_AND_TIME` includes the complete last day.

### Negative

- The table holds vendor facts. A new platform version can change them.

### Risks and mitigations

- Risk: a vendor manual changes a range. Mitigation: a new ADR changes the row, and one row of code and one test follow.

## Related

- `domain-model.md` section 2.1 (base-unit table, `IecLiteral`)
- `data-model.md` section 7 (primitive matrix)
- `overview.md` section 9 (A43)
- ADR-0022 (time values)
