# ADR-0167: `search_path` of the audit guard function

Status: Accepted. Amends ADR-0098 (the `search_path` rule of `data-model.md` §1.1)
Date: 2026-11-14
Deciders: project owner

## Context

`data-model.md` §1.1 says that every function sets `search_path = pg_catalog, public, pg_temp`. §1.1 cites ADR-0022 and ADR-0098 for the rule. `.claude/rules/sql.md` repeats the rule and cites ADR-0098.

The event-trigger function `audit_guard.audit_objects_dropped()` does not follow this rule:

1. `deploy/db/initdb/01-roles.sh` creates the function with `SET search_path = pg_catalog, pg_temp`. `data-model.md` §5.1 shows the same statement.
2. The superuser creates the function and owns it (ADR-0034). The line of the function in `deploy/pgtools/restore-code.expected` names the owner `postgres`.
3. The function is not `SECURITY DEFINER`. It runs as the role that runs the drop, for example `recipe_migrator`.
4. `recipe_migrator` owns the schema `public` through `pg_database_owner` (ADR-0034, ADR-0097). It can therefore create an object in `public`.
5. The body uses operators such as `IN` and `<>` without a schema. The operator scan of ADR-0098 (f) reads only the functions that `audit_owner` owns, so it does not read this body.

The full review of 2026-11-13 reports the conflict between §1.1 and §5.1 as F-16. The code has no defect. A maintainer or a lint that applies §1.1 can add `public` to the function and so contradict §5.1.

## Decision drivers

- No role except `audit_owner` drops an audit object (R20, I13).
- An unqualified name in the body of the guard must not reach an object that a login role can create.
- Each rule lives in one document, and that document names its exceptions (`.claude/rules/docs.md`).

## Considered options

### Option A: Keep the narrow path and name the exception

- Pros: no migration and no change of `01-roles.sh`. No unqualified name of the body reaches `public`. The pinned SHA-256 of the function stays.
- Cons: the general rule gets one exception.

### Option B: Add `public` to the path of the guard

- Pros: every function follows the general rule.
- Cons: an unqualified name of the body can then reach an object in `public`. The superuser must replace the function on every existing volume (`reliability.md` §7.10). The SHA-256 in `restore-code.expected` changes.

## Decision

The project uses **Option A** (fix plan of 2026-11-13, §1).

- (a) `audit_guard.audit_objects_dropped()` keeps `SET search_path = pg_catalog, pg_temp`. No migration follows, and `01-roles.sh` does not change.
- (b) The function is the one exception to the `search_path` rule of `data-model.md` §1.1. §1.1 and `.claude/rules/sql.md` name it, with a link to `data-model.md` §5.1.
- (c) Every other function keeps `search_path = pg_catalog, public, pg_temp`. The rules of ADR-0098 (e) for a `SECURITY DEFINER` function do not change.
- (d) The decisions (a) to (j) of ADR-0098 do not change. This ADR adds one exception to the §1.1 rule that `.claude/rules/sql.md` cites with ADR-0098.

## Consequences

### Positive

- §1.1, §5.1, and `.claude/rules/sql.md` agree.
- A name in the body of the guard resolves only in `pg_catalog` and `pg_temp`.

### Negative

- A reader of the general rule must also read its exception.

### Risks and mitigations

- Risk: a later change adds `public` to the guard. Mitigation: the test `the_guard_function_of_section_5_1_equals_the_function_of_the_init_script` (`crates/recipe-db/tests/storage/grants/audit_hardening.rs`) keeps §5.1 and `01-roles.sh` equal. The upgrade check of `reliability.md` §7.10 compares the SHA-256 of the function with `restore-code.expected`.
- Risk: a later function copies the narrow path without a reason. Mitigation: §1.1 names one function. A second exception needs a new ADR (`CLAUDE.md`, "Source of truth and change rule").

## Related

- ADR-0034 (event trigger of the superuser), ADR-0097 (schema-free names in the drop guard), ADR-0098 (qualified names of the definer functions).
- `data-model.md` §1.1 and §5.1, `reliability.md` §7.10, `.claude/rules/sql.md`.
- `deploy/db/initdb/01-roles.sh`, `deploy/pgtools/restore-code.expected`, `crates/recipe-db/tests/storage/grants/audit_hardening.rs`.
- `docs/plan/review/2026-11-13-full-repo-review.md` §4 (F-16), `docs/plan/review/2026-11-13-fix-plan.md` §1.
- `overview.md` §7: R20. `domain-model.md` §3: I13.
