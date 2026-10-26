# ADR-0098: Insert columns of `audit_log` and qualified operators of two definer functions

Status: Proposed
Date: 2026-10-25
Deciders: project owner

## Context

The security audit of step P4.2 found two weak points in the audit objects of `data-model.md` sections 4 and 5.

Insert grant of `audit_log`:

1. `0003_grants.sql` grants `INSERT` on the whole table `audit_log` to `recipe_app` (`data-model.md` section 5.2).
2. `recipe_app` can therefore set `occurred_at`. With `OVERRIDING SYSTEM VALUE`, it can also set `id`.
3. Trigger 4.9 hashes the row as it gets it. A backdated entry is therefore a valid link of the chain, and the nightly check passes.
4. `0005_security_event_guard.sql` already grants `INSERT` on `security_event` only on the columns without a default (ADR-0035).

Operators of two definer functions:

1. `audit_hash_chain()` and `security_event_purge()` are `SECURITY DEFINER` functions of `audit_owner` in `0002_triggers.sql`.
2. They use the operators `+`, `||`, `<`, `-`, and `IN` without a schema, and the types `bytea` and `interval` without a schema.
3. `data-model.md` section 4 and `.claude/rules/sql.md` require that a `SECURITY DEFINER` function qualifies every name with its schema. Section 1.1 names only tables and functions. `audit_canonical` and `replication_state` also qualify their operators.
4. No login role holds `CREATE` on `public` today. A later grant would let a planted operator run as `audit_owner`.

`0002` and `0003` shipped, so their text stays (`data-model.md` section 8).

## Decision drivers

- An entry of the audit log holds the time of the database, not a time of the client (R20, I13).
- A definer function runs only built-in operators and types (`data-model.md` section 4).
- A change of an audit object uses the five steps of `data-model.md` section 5.1.

## Considered options

### Option A: Two forward migrations with the five steps

- Pros: each migration holds one topic. The existing hashes stay valid, because the results of the functions do not change.
- Cons: two more files.

### Option B: Set `occurred_at` in trigger 4.9

- Pros: one function sets the time.
- Cons: the body of the hash function changes. The grant still lets `recipe_app` set `id`.

### Option C: Accept both points

- Pros: no change.
- Cons: a holder of the `recipe_app` credential can add backdated history with a valid hash.

## Decision

The project uses **Option A**.

- (a) `migrations/0007_audit_definer_operators.sql` runs the five steps of `data-model.md` section 5.1. Step 3 runs `CREATE OR REPLACE FUNCTION` for `audit_hash_chain()` and `security_event_purge()`.
  - Each operator becomes `OPERATOR(pg_catalog.<op>)`. `IN (subquery)` becomes `OPERATOR(pg_catalog.=) ANY (subquery)`.
  - Every `OPERATOR(...)` form has the same precedence and groups from left to right. The migration therefore keeps the grouping of each expression with parentheses.
  - The condition of `security_event_purge()` becomes `e.occurred_at OPERATOR(pg_catalog.<) (pg_catalog.now() OPERATOR(pg_catalog.-) '400 days'::pg_catalog.interval)`.
  - A type with a normal name, such as `bytea` or `text`, gets the schema `pg_catalog`.
  - A type keyword of the SQL standard, such as `BIGINT`, `INTEGER`, `BOOLEAN`, or the literal form `INTERVAL '400 days'`, takes no schema. The parser resolves it without the search path.
  - Each `CREATE OR REPLACE FUNCTION` repeats `SECURITY DEFINER` and `SET search_path = pg_catalog, public, pg_temp`. Without them, the statement removes both attributes.
  - The results do not change, so no hash of an existing entry changes.
- (b) `migrations/0008_audit_log_insert_columns.sql` runs the five steps. Step 3 revokes `INSERT` on `audit_log` from `recipe_app`. Then it grants `INSERT` on eleven columns.
  - The columns are `actor_user_id`, `action`, `entity_type`, `entity_id`, `aggregate_type`, `aggregate_id`, `before_state`, `after_state`, `request_id`, `client_ip`, and `session_ref`.
  - These columns are the columns of the insert of `recipe-db` (`audit_insert.rs`).
- (c) The grant holds no `id`, `occurred_at`, `chain_seq`, `prev_hash`, or `entry_hash`. The database sets `id` and `occurred_at`. Trigger 4.9 sets the three chain columns. An insert of `recipe_app` that names another column fails with `42501`.
- (d) `crates/recipe-db/tests/grants.expected`, `data-model.md` sections 5.2 and 8, and `restore-code.expected` change in the same change.
- (e) The rule of `data-model.md` section 1.1 for a `SECURITY DEFINER` function also names operators and types. Section 1.1 then agrees with section 4 and `.claude/rules/sql.md`. The new text is: "A `SECURITY DEFINER` function also schema-qualifies every table, function, operator, and type name. A type keyword of the SQL standard, such as `BIGINT` or `INTERVAL`, needs no schema (ADR-0022, ADR-0098)."
- (f) A storage test scans the body of every function that `audit_owner` owns, not only the definer functions. `audit_hash_chain` calls `audit_canonical` as `audit_owner`.
  - The scan reads `prosrc`, the body only, not `pg_get_functiondef`.
  - It skips comments, string literals, `$$`, `::`, `:=`, `=>`, the `=` of `GET DIAGNOSTICS`, and each `OPERATOR(pg_catalog.…)` group.
  - It fails when an operator outside `OPERATOR(pg_catalog.…)` remains.
  - A unit test feeds the scanner the bodies of `replication_state` and `security_event_guard`. It expects no hit.
- (g) After `0007`, a storage test reads `pg_proc` of `audit_hash_chain()` and `security_event_purge()`. It requires `prosecdef`, `proconfig = {search_path=pg_catalog, public, pg_temp}`, the owner `audit_owner`, and the ACL of before the migration.
- (h) The existing tests of the retention boundary and of the batch limit of `security_event_purge()` stay. A test of the qualified names alone accepts a wrong grouping.
- (i) The tests of the insert columns change as follows:
  - The test `recipe_app_cannot_forge_the_chain_columns_of_an_insert` (`hash_chain.rs`) becomes a `42501` test. It requires no new entry and an unchanged chain head.
  - A separate test inserts forged chain columns as the superuser. Trigger 4.9 overrides them, as the old test showed for `recipe_app`.
  - Each of the five columns `id`, `occurred_at`, `chain_seq`, `prev_hash`, and `entry_hash` has one `42501` case for `recipe_app`.
  - The positive test of a normal insert of the application stays.

Residual risk:

- (j) A holder of the `recipe_app` credential can still choose `actor_user_id`, `request_id`, `client_ip`, and `session_ref`. A grant cannot stop this, because the application writes these values.

## Consequences

### Positive

- `occurred_at` and `id` of an entry come from the database.
- The two definer functions follow the rule of `data-model.md` section 4.

### Negative

- Two more migrations. Each one runs `SET ROLE audit_owner` and `RESET ROLE`.

### Risks and mitigations

- Risk: a test or a tool inserts `occurred_at` as `recipe_app`. Mitigation: the grant test and the storage tests run as `recipe_app` and show each such insert.
- Risk: a later definer function uses an unqualified operator. Mitigation: the storage test of (f).
- Risk: a replacement of a function loses `SECURITY DEFINER`. Then `audit_hash_chain()` runs as `recipe_app`, and every command fails. Mitigation: the test of (g).
- Risk: an `OPERATOR(...)` form changes the grouping of an expression. Mitigation: the parentheses of (a) and the tests of (h).

## Related

- ADR-0022 (hash chain), ADR-0034 (audit objects), ADR-0035 (column grant of `security_event`).
- `data-model.md` sections 1.1, 4.9, 4.12, 5.1, 5.2, 5.3, and 8.
- `migrations/0002_triggers.sql`, `migrations/0003_grants.sql`, `migrations/0005_security_event_guard.sql`.
- `crates/recipe-db/src/audit_insert.rs`, `crates/recipe-db/tests/grants.expected`.
- Baseline requirement: `audit` "Append-only audit log for recipe_app".
- `overview.md` section 7: R20. `domain-model.md` section 3: I13.
