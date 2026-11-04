# ADR-0118: Stored values that break a newer rule

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

ADR-0114, ADR-0116, and ADR-0117 add rules for values, limits, and S7 addresses. A row that a user stored before these rules can break them. ADR-0120 keeps such rows in drafts and in copies. The examples are a `DATE` count from 72 684 to 72 840, a `REAL` number with 8 significant digits, and a `DINT` at `DB1.DBD65534`.

Several reads check a stored value or limit against its type:

- the parameter table (ADR-0108 (a)),
- the parameter page with its summary and its form,
- the diff of two versions (ADR-0050 (c)),
- the approval preview with its diff of the draft against its base (ADR-0049).

These reads give `500 E-DB-008` for every fault. A user then cannot open the page that fixes the value. After a fix in a new draft, the approval preview of that draft still fails on its affected base. The text of a time leaf comes from a literal of the current range, so an affected `DATE` has no text.

## Decision drivers

- The user sees each part that breaks a rule before the approval and the export (R12, R38).
- Approved versions are immutable (ADR-0009).
- A storage defect stays visible as a defect (`reliability.md` section 1.1).

## Considered options

### Option A: Show the stored part with a field line

- Pros: every page opens. The user sees the cause next to the part and can change it.
- Cons: a read must tell a rule fault from a storage defect.

### Option B: Keep the page failure

- Pros: no change of the reads.
- Cons: a draft with an affected value has no page that fixes it. An affected approved version cannot be read.

### Option C: Hold the release until a scan finds no affected row

- Pros: no affected row reaches a page.
- Cons: a scan needs the domain walk for leaves inside a struct or an array. A row can still come back from a restore of an older backup.

## Decision

The project uses **Option A**.

- (a) A read that shows a stored value, limit, or S7 address shows the stored data unchanged. The reads are the parameter table, the parameter page, the diff of two versions, and the approval preview with its diff.
- (b) The read checks each part that it shows with the current rule set and with the earlier rule set of ADR-0120.
- (c) A part that passes the earlier set and fails the current set gets a field line. The line holds the code of the broken rule, and the page answer stays `200`.
- (d) A part that fails the earlier set is a storage defect. The read gives `500 E-DB-008`, as ADR-0050 (c) and ADR-0108 (a) say.
- (e) The codes of a field line are `E-RCP-005` for a value and `E-RCP-006` for a limit. `E-RCP-055` marks too many significant digits. `E-RCP-056` and `E-RCP-057` mark an address.
- (f) The line uses the markup of a field line of ADR-0041. It stands under the leaf of a value or a limit, or under the field `s7_address`.
- (g) The read builds the text of the part with the earlier set. A `DATE` count 72 700 shows as `D#2169-01-17`. The read builds no value object of the current set for such a part.
- (h) The approval preview lists the faults of the draft as blockers (ADR-0049). Its diff shows the field lines on both sides. A fault on the side of the base does not stop the preview.
- (i) The lines appear in a draft, an approved version, and a retired version. In a draft, the form holds the stored text, and the user can change or remove the part.

Items (a) to (i) are one decision: how a read shows a stored part that breaks a newer rule. ADR-0120 holds the commands, and ADR-0116 (e) holds the address checks of a command.

## Consequences

### Positive

- A rule change locks no page.
- The user finds each affected part on the page that shows it.
- A storage defect still stops the read and writes one log line.

### Negative

- The reads get a second result kind next to the text of a part.
- A read runs the checks of two rule sets.

### Risks and mitigations

- Risk: an affected approved version reaches a PLC. Mitigation: ADR-0119 sets `loadable` to `false` for such a version.
- Risk: a read takes a storage defect for a rule fault. Mitigation: item (d) uses the earlier set as the border, and a test covers a `DATE` count above 72 840.

## Related

- ADR-0041 (field lines), ADR-0049 (approval preview), ADR-0050 (diff), ADR-0108 (parameter table), ADR-0009 (immutable snapshot)
- ADR-0114, ADR-0116, ADR-0117 (the new rules), ADR-0119 (export), ADR-0120 (earlier rule set)
- `api.md` sections 2.5, 2.6
- `overview.md` section 7: R12, R38
