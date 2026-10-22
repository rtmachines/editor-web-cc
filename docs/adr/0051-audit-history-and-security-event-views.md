# ADR-0051: Audit, history, and security event views

Status: Accepted
Date: 2026-10-16
Deciders: project owner

## Context

`api.md` section 2.9 has four routes: the audit list `/audit`, one audit entry `/audit/{id}`, the recipe history `/recipes/{ref}/history`, and the security events `/security-events`. The queries `ListAuditEntries`, `GetAuditEntry`, `GetRecipeHistory`, and `ListSecurityEvents` have their `Operation` variants and matrix rows in `recipe-app`, but no use case, no read port, and no adapter.

The in-memory adapter keeps the audit entries without an identifier and without a read interface. Its security event sink keeps no time, no identifier, and no aggregation per minute. `recipe-db` reads `audit_log` only for the last editor of a version and never reads `security_event`.

The documents leave these points open:

- the meaning of the filter `aggregate`, and the grammar of `actor`, `from`, `to`, and `before`,
- the page sizes of the three lists,
- the answer to a ProcessEngineer for an entry outside the scope of A20,
- the filter `login` of the security events, because an unknown login is stored only as `login_hmac`,
- how the user reaches the audit pages, because `design-system.md` section 10.2 fixes four navigation links,
- how the page shows `before` and `after` without `|safe`.

`audit_log` has the indexes `(aggregate_id, id)`, `(entity_type, entity_id, id)`, `(actor_user_id, id)`, `(action, id)`, and a BRIN index on `occurred_at` (`data-model.md` section 3.15). A new index on an audit table needs the five steps of `data-model.md` section 5.1 as `audit_owner`.

## Decision drivers

- Each role sees only its scope of A20. The permission check comes first (ADR-0013, R28).
- Each list query uses an existing index and reads no state column (R30).
- No secret shows: no password hash, no session token, no cookie value, and no `login_hmac` (R20, A33).
- A user text of an entry shows escaped. No template uses `|safe` (`api.md` section 9).
- The in-memory adapter and PostgreSQL give the same rows in the same order.

## Considered options

### Option A: Read ports with a cursor on the entry id, filters from links and fixed lists

- Pros: every list reads one statement on an existing index. The filter values are fixed lists or values that a link sets.
- Cons: the user cannot type an aggregate. The link sets it.

### Option B: The aggregate as a type in a select

- Pros: a simple select.
- Cons: no index on `aggregate_type`. A rare type scans the primary key until `db.statement_timeout`.

### Option C: Offset pages

- Pros: page numbers, as the other lists have.
- Cons: a new entry moves every row of the next page. The audit log grows without an end, so an offset scan gets slower.

## Decision

The project uses **Option A**.

- (a) `recipe-app` gets the read port of the audit views. It has five methods:
  - the audit list,
  - one audit entry,
  - the recipe history,
  - the security event list,
  - the label of one aggregate for the filter `aggregate` of point (d).
  Each list takes a cursor `before` and a page size. It returns at most one page and the information that an older row exists.
  - The in-memory adapter keeps a read model with an identifier per entry: the position in the commit order plus 1. It keeps the commit time of each entry.
  - The in-memory store owns the security event sink. The sink keeps an identifier and the clock time of each event. Its method `events()` keeps the flat list of the recorded events.
  - Only the read model aggregates the events by `data-model.md` section 3.8. One row then holds the events of one client address, kind, and minute of `login_failed`, `login_throttled`, and `csrf_rejected`.
  - A contract test runs the same cases on both adapters.
- (b) Each query reads in `Isolation::ReadOnly` (ADR-0042 (c)). Each list is one statement with `id < $before`, the filters, `ORDER BY id DESC`, and `LIMIT page + 1`. The audit list and the history read no `before_state` and no `after_state` column. The security event list reads `detail` for point (h).
  - The SQL text comes from a builder that adds only the predicates of the sent filters. It has no predicate of the form `$n IS NULL OR`.
  - No migration and no new index.
- (c) The scope of A20. Each query calls `authorize` first (ADR-0013).
  - A role with `AuditView` sees every entry. A role with only `AuditViewRecipes` sees the entries with `aggregate_type = 'recipe'`. The query adds this predicate.
  - A filter value of `entity_type` or `action` outside the scope of the role gives the empty list without a read.
  - `GetAuditEntry` of an entry outside the scope gives `404 E-WEB-005`. The answer does not show that the entry exists.
  - The audit list and `GetAuditEntry` include the entries of a deleted recipe for a role with `AuditView` or `AuditViewRecipes`. The recipe history of point (g) keeps `409 E-RCP-012` for every role except the Administrator.
- (d) The filters of `/audit`:
  - `aggregate` is the id of an aggregate (a UUID). A link sets it: "Entries of the whole recipe" on the history page, and "Entries of the whole aggregate" on an entry page. The filter form keeps it as a hidden field and shows a link that clears it.
  - With the filter `aggregate`, the page names the aggregate by the word of its type and its label of point (e). The type comes from the `aggregate_type` of the newest row. An empty list names the aggregate by its short id of point (e).
  - `entity_type` is one of the eight entity types of `data-model.md` section 10. `entity_id` is a UUID and needs `entity_type`. The link "Entries of this object" of an entry page sets both. The filter form keeps both as hidden fields and shows a link that clears them (key `audit-filter-object-clear`).
  - `action` is one of the 35 actions of `data-model.md` section 10. The select groups them by context with `optgroup`. A ProcessEngineer sees only the entity types and the actions of the recipe scope.
  - `actor` is a `Login`. A malformed `actor` gives `422 E-WEB-008`. A well-formed `actor` that names no user gives the empty list.
  - This is an exception to the rule of a query value of `api.md` section 1. A `404` would show which logins exist (A42).
  - `from` and `to` are dates `YYYY-MM-DD` in UTC. Both days are inside the range. `to` before `from` gives `422 E-WEB-008`. The UI shows every time in UTC (`design-system.md` section 5).
  - `before` is the id of an entry, a positive integer.
  - A value outside a fixed list, a malformed UUID, date, or integer, and `entity_id` without `entity_type` give `422 E-WEB-008`. An unknown key gives `422 E-WEB-008`. An empty field means no filter.
- (e) A row of the audit list links to `/audit/{id}`. It shows these fields:
  - the time,
  - the actor login, or the word "system" for an entry without an actor,
  - the action and the entity type,
  - the label of the entity.
  The label comes from the current rows. The list reads no stored state for a label. A row that no longer exists shows the word of its entity type and the short id. The short id is the first eight characters of the entity id. The label of each entity type is:
  - a recipe: the reference number,
  - a version: the version number,
  - a parameter: the tag and the version number,
  - a machine: the machine code. A unit: the unit path. A type: the name and the revision,
  - a user: the login. A setting: the key.
- (f) The page `/audit/{id}` shows the fields of point (e), the request id, and the aggregate.
  - A role with `AuditView` also sees the client address, the session reference, and the chain position (`chain_seq`). A role with only `AuditViewRecipes` does not see them. The gaps of `chain_seq` would otherwise show the exact number of the entries outside its scope.
  - `before` and `after` show as the text of `format!("{:#}", value)` with sorted keys inside a `pre` element. Askama escapes the text. No template uses `|safe`.
  - The text shows in full. The `pre` element wraps long lines (`pre-wrap`, `overflow-wrap: anywhere`).
  - The JSON shows the stored record. A time leaf therefore shows its integer count of the base unit, not an IEC literal. This is an exception to A43 and `domain-model.md` section 2.1, because the page shows the record of the hash chain.
  - The page shows no `entry_hash` and no `prev_hash`. A row image never holds `password_hash` (`data-model.md` section 10).
- (g) The recipe history shows the entries with the `aggregate_id` of the recipe, newest first, without `before` and `after`. A row has the shape of point (e).
  - For a role with `AuditView` or `AuditViewRecipes`, each row links to `/audit/{id}`. The Operator sees no link.
  - For a deleted recipe, the Administrator reads the history, and every other role gets `409 E-RCP-012`. This extends the rule of `api.md` section 2.4 to the history route.
  - The recipe page and the version page link to the history.
- (h) The filters of `/security-events` are `kind`, `login`, `client_ip`, `from`, `to`, and `before`. They follow the grammar of point (d).
  - `kind` is one of the seven kinds of `data-model.md` section 3.8. `client_ip` is an IP address.
  - `login` is a `TypedLogin`. A malformed value gives `422 E-WEB-008`.
  - `login` matches the events of the account with that login. It also matches the events whose `login_hmac` equals the HMAC of the typed login under the current key of the login key ring (`KeyRing::sign`). An event under an older key does not match.
  - An aggregated row keeps the subject of its first event (`data-model.md` section 3.8). The filter `login` therefore matches a row by that first event only. The filter `client_ip` finds every attempt of an address, and the page states this limit next to the filter `login`.
  - A row shows the time, the kind, the count, the client address, the user agent, and the request id.
  - A row also shows the login of the user or the words "unknown login", and `detail` as formatted JSON.
  - Askama escapes each text. `login_hmac` never shows.
- (i) The pager of a cursor list has these parts:
  - the link "Newest entries" without `before`,
  - the link "Older entries" with `before` set to the id of the last row, when an older row exists,
  - the text "No older entries" on the last page.
  The filter form sends `GET` with no token and no `before`. A change of a filter therefore starts at the newest entry.
  - The filter form has the literal attributes `hx-boost="true"`, `hx-target="#content"`, and `hx-push-url="true"`. Its `action` is the literal path of the page.
  - A `422 E-WEB-008` answer goes into `#error-panel` (`api.md` section 5.3), so the form keeps its values. Each line names the field with the label key of `design-system.md` section 10.5. `design-system.md` gets the section 10.12 for the page pager and the cursor pager.
- (j) The navigation gets a fifth link "Audit" after "Users". It shows when the matrix row of `ListAuditEntries` allows the role. The order becomes recipes, machines, users, audit, own profile. This amends the order of ADR-0037, the query list of ADR-0036 (i), and `design-system.md` section 10.2.
  - The audit page shows the link "Security events" when the row of `ListSecurityEvents` allows the role.
- (k) `reliability.md` section 7.11 gets the rows `page.audit`, `page.history`, and `page.security_events`, each 100 rows.

## Consequences

### Positive

- Each list reads one statement and no state column.
- A ProcessEngineer cannot read an entry outside the recipe scope. `GET /audit/{id}` of such an entry gives the same answer as an absent entry. The entry page shows `chain_seq`, `client_ip`, and `session_ref` only to a role with `AuditView`.
- The actor filter does not show which logins exist (A42).
- The pages show every stored state in full and escaped.

### Negative

- A user sets `aggregate` and `entity_id` only through a link.
- A typing error in `actor` gives the empty list and no error. The user cannot tell a typing error from a user without entries.
- The HMAC filter scans the rows of the other filters, because `login_hmac` has no index.
- The scope of a ProcessEngineer adds the predicate on `aggregate_type`, which has no index.
- The entry ids in the links and the cursor have gaps. A ProcessEngineer can count the entries outside the recipe scope between two visible entries. The ProcessEngineer cannot read their content, type, or actor (A20).

### Risks and mitigations

- Risk: a filter combination without a matching index reads many rows. Mitigation: `db.statement_timeout` stops the statement with `503 E-DB-002`. On a large audit log, the load test of step P4.2 measures three cases. They are the filters without an index, the scope of a ProcessEngineer, and the HMAC filter.
- Risk: an entry state holds user text with markup. Mitigation: askama escapes the text, and the XSS corpus runs through `before`, `after`, `detail`, and `user_agent`.
- Risk: the in-memory adapter and PostgreSQL order the rows differently. Mitigation: point (a), the contract test.

## Related

- ADR-0005, ADR-0013, ADR-0022, ADR-0035, ADR-0042, ADR-0049, ADR-0050
- ADR-0036 (amended by this ADR in the query list of (i)) and ADR-0037 (amended by this ADR in the navigation order)
- `docs/architecture/api.md` sections 1, 2.4, and 2.9
- `docs/architecture/data-model.md` sections 3.8, 3.15, 5.1, and 10
- `docs/architecture/design-system.md` sections 5, 10.2, and 10.12
- `overview.md` section 7: R20, R24, R28, R35. Section 9: A20, A33, A42, A43
