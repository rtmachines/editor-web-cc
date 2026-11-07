# ADR-0138: One line for a changed form body

Status: Accepted. Amends ADR-0041 (a) (the list of bad fields) and ADR-0039 (b) (the answer to a bad key)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a conflict between two rules of the form answer. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as finding #11.

These facts shape the decision:

- ADR-0041 (a) says that `recipe-web` collects every bad field of a form and answers once with all of them.
- ADR-0039 (b) says that a key outside the fields of the route, a malformed leaf path, a `[*]` segment, or a duplicate key gives `422 E-WEB-008`.
- `api.md` §2.6 states both rules in one list item. It does not say which rule applies when a body breaks both.
- The reader of the parameter forms stops at the first bad key. It answers one line `E-WEB-008` and does not check the other fields.
- Thus the body `tag=bad tag&bogus=1&value[*]=2` gives one line, and not a line for each of the three faults.
- The reader of the UDT form and the `serde` readers of the other forms also stop at the first fault of the body.
- The HTML of the editor sets each key, each hidden field, and each button value of a form. A user types only the values of the inputs.
- The parameter reader also stops at some bad hidden fields and button values. These are the leaf path of a limit row, the flag `fields`, `limit_mode`, and `value_type`. The add, edit, and rebind forms put a bad `row_version`, `from_type_id`, or `to_type_id` into the list of ADR-0041 (a). They do not stop the reader.
- The value form and the limits form read `row_version` before any other field. A bad `row_version` gives one line, and the reader checks no other field.
- ADR-0041 has two stages. First `recipe-web` lists the bad fields that it reads (ADR-0041 (a)). Only a body without such a field reaches the command, which lists the faults of the value and the limits (ADR-0041 (b)).

The project owner decided on 2026-11-06 (fix plan, question Q-F, option (1)). A body with such a key is a changed request. It gives one line and lists no other bad field.

## Decision drivers

- A user of the editor HTML sees every fault of the typed values at once (ADR-0041).
- A body that the editor HTML cannot send gets a short answer. The server does not check the rest of a changed request.
- One rule in the documents, the specs, and the code.
- No new error code and no change of the code.

## Considered options

### Option A: One line for a changed body

- Pros: the code does not change. The reader stops early, so a changed request costs little work. The answer names no field that the HTML did not send.
- Cons: a hand-made request with several faults needs one request per fault.

### Option B: List every bad key with the other faults

- Pros: one rule for every body. Every fault gets a line.
- Cons: the reader must collect the faults of the keys. Then it must check the fields of a body that it cannot trust. Several readers change. A user of the editor HTML gets no benefit.

## Decision

The project uses **Option A**. The items below are the rules.

- (a) A changed form body gives `422 E-WEB-008` with one line. The line has the label `Form`. A fault of (c) or (d) may give the label of its own field. The answer lists no other bad field and no fault of the value or the limits.
  - The line does not name the key or the fault. Its text depends on the class of the fault.
  - The text `reason-form` is "the form has a missing field, an unknown field, or a wrong content type" (`i18n/en.ftl`, `pl.ftl`).
  - The parameter reader gives it for an unknown key, a plain key or the CSRF token twice, and more pairs than the bound.
  - The UDT reader gives it for an unknown key, a key twice, and a malformed index. Each `serde` reader gives it for every fault of its form.
  - The parameter reader gives the text `reason-value-key` for a faulty value key or member field, and `reason-limit-row` for a faulty limit key.
- (b) These faults make a body a changed body:
  - a key outside the fields of the route, for example a limit key on a route without limits
  - a key twice, also the CSRF token, a value key, a limit field, or a member field
  - a malformed key, for example a bad leaf path, a bad row index, or a bad parameter key in a rebind field
  - a `[*]` segment in a value key or in a rebind field
  - more pairs than the bound of the parameter reader
  - in a `serde` form, a value that the struct cannot read into the type of its field, for example `row_version=x` in the approve form
- (c) The parameter reader also gives one line for some malformed hidden fields and button values. These are the leaf path of a limit row, a limit row with a bound and no leaf path, and the flag `fields` other than `1`. They are also a malformed `limit_mode` or `value_type`.
  - A bad `row_version` of the add form or the edit form is not a fault of (c). It gives a line in the list of (f).
  - A bad `row_version` of the value form or the limits form is not a fault of (c). It gives one line, and the reader checks no other field.
  - A bad `row_version`, `from_type_id`, or `to_type_id` of the rebind form is not a fault of (c). It gives a line in the list of (f). The two type rows share one line.
- (d) The UDT form also gives one line for a member index of `MAX_MEMBERS` or more and for a flag `add` other than `1`. ADR-0037 (k) gives the keys of its member rows.
- (e) The rules apply to every form body that `recipe-web` reads:
  - the forms of `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, and `RebindTypeRevision`
  - the UDT form
  - each form that `recipe-web` reads into a `serde` struct with `deny_unknown_fields`
- (f) The two stages of ADR-0041 (a) and (b) stay for a body without a fault of (b) to (d). That body also has no missing field of (j).
  - Stage 1: `recipe-web` checks the fields that it reads. One answer lists every bad field of this stage. Examples are the definition fields, a bad maximum length of a limit row, and the member rows of the UDT form. The add, edit, and rebind forms also list a bad `row_version`.
  - Stage 2: only a body without a bad field of stage 1 reaches the command. One answer then lists every fault of the leaves and the limit rows that the command finds (ADR-0041 (b)).
  - Thus a line of stage 1 and a fault of the command never share one answer.
  - `POST /login` keeps one line per answer, and `E-IDN-003` names no field (ADR-0132 (e)).
  - The own password form and the users forms keep the lines of ADR-0132 (a) to (d).
- (g) An HTMX answer of (a) holds no out-of-band form, so the browser keeps the form with its texts. A request without HTMX gets the full error page (ADR-0047).
- (h) After a fault of (b) to (d), the command does not run and writes nothing.
- (i) In a parameter value field, a well-formed leaf path that the type does not hold is not a changed body. It gives `422 E-RCP-005` in the list of stage 2 of (f). This ADR keeps the other contracts of such a path:
  - A limit row with such a path gives `422 E-RCP-017` in the list of stage 2.
  - A rebind field with such a path gives one line `422 E-WEB-008` and no out-of-band form.
- (j) This ADR does not change the rules of a query string (ADR-0050 (e), ADR-0051). It does not change the answer to a missing field of a `serde` form (`reliability.md` §4.2).

## Consequences

### Positive

- `api.md` §2.6, ADR-0039, and ADR-0041 give one answer for every body.
- The code does not change. Tests pin the rule.

### Negative

- A hand-made request with several faults gets one line per answer.
- The text of that line does not name the fault. A plain key twice and the pair bound give the text of an unknown key.

### Risks and mitigations

- Risk: a future template sends a key that the reader does not know. Then every save of that form gives one line `E-WEB-008` with no field line. Mitigation: the HTTP tests of each form send the body of its template and expect a success.
- Risk: a later change makes the reader collect bad keys. Mitigation: a pinning test of each reader expects one line for a body with a bad key and a bad field.

## Related

- ADR-0039 (b), amended by this ADR: the answer to a bad key has one line.
- ADR-0041 (a), amended by this ADR: the list covers a body without a fault of (b) to (d).
- ADR-0037 (k) (indexed fields), ADR-0046 (form query before the fields), ADR-0050 (e) (a line with no link), ADR-0132 (own password and users forms)
- `api.md` §2.6, §5
- `reliability.md` §1.2 (`E-WEB-008`)
- `overview.md` §7: R36, R38, R39
