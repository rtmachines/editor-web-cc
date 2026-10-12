# ADR-0013: Fixed role-based permissions checked in each use case

Status: Accepted. Amended by ADR-0022 (separation of duties, Forbidden recorded in security_event)
Date: 2026-10-12
Deciders: project owner

## Context

The draft defines three roles: administrator, process engineer, and operator. It says that an administrator or process engineer can create, edit, and delete recipes "when their permissions allow the operation". It says that an operator can only view and that "an authorized user approves a recipe version". It also says that permissions must be enforced "for every recipe management operation".

The phrase "when their permissions allow" can mean a fixed set per role, or per-user overrides. The draft does not say who is "authorized" to approve.

## Decision drivers

- Every recipe operation must check a permission (R28). The check must be impossible to skip.
- The matrix must be readable by an auditor in one page.
- Fewer rules mean fewer gaps.
- The draft does not ask for per-user configuration or a four-eyes rule.

## Considered options

### Option A: Fixed RBAC, role maps to a permission set in code

- Pros: the matrix is a constant. One test per (operation, role) pair covers it. No admin UI for permissions. No data that can be misconfigured.
- Cons: a plant that needs a different split must change code and redeploy.

### Option B: RBAC with per-user overrides stored in the database

- Pros: a plant can tune access without a release.
- Cons: a permission UI, a data model, audit of permission changes, and a test matrix that grows with every override. No requirement asks for it.

### Option C: Fixed RBAC plus a four-eyes rule for approval

The author of a draft cannot approve it.

- Pros: a stronger control for a critical process.
- Cons: a one-person plant cannot approve anything. The draft does not ask for it.

## Decision

We use Option A. ADR-0022 amends this decision: it adds Option C as the configuration flag `approval.separation_of_duties`, on by default.

- `Permission` is an enum in `recipe-domain`. `Role::permissions()` returns a constant set. The full matrix (operation x role) is in `api.md` section 3.
- `Administrator` and `ProcessEngineer` hold `VersionApprove` (Assumption A5).
- `Administrator` and `ProcessEngineer` manage machines, units, and UDTs. Only `Administrator` manages users (Assumption A9).
- `Operator` holds `RecipeView` and `VersionExport` only (Assumption A4).
- `ProcessEngineer` can delete a recipe only when it has no `Approved` and no `Retired` version. `Administrator` can delete any recipe without an `Approved` version (ADR-0021).
- The permission check is the first statement of every use case in `recipe-app`. It takes the `CurrentUser` and returns `Err(Forbidden { operation })`.
- `recipe-web` does not make authorization decisions. It only hides UI elements that the role cannot use. The hidden element is a convenience, not a control.
- An integration test enumerates every use case and every role and asserts the expected `Ok` or `Forbidden` outcome. A new use case without a row in the test fails the build.
- A `Forbidden` result is written to `security_event` (kind `forbidden`) in its own transaction, with the user and the operation (amended by ADR-0022).
- At least one active `Administrator` must exist. `SetUserRole` and `DeactivateUser` return `Err(LastAdministrator)` otherwise.

## Consequences

### Positive

- The matrix is one table in `api.md` and one constant in code. They are compared by a test.
- No permission data can be wrong in the database.
- A review can see the check at the top of each use case.

### Negative

- A change of the matrix is a code change and a release.
- The HTTP layer and the use case both know about roles, once for display and once for control.

### Risks and mitigations

- Risk: a route calls a repository directly and skips the use case. Mitigation: repositories are not visible to `recipe-web`. Only `recipe-app` use cases are public.
- Risk: a plant needs the four-eyes rule. Mitigation: Option C adds one condition in `ApproveVersion` and needs the `draft_editor` table (ADR-0022).

## Related

- ADR-0014 (who the `CurrentUser` is)
- ADR-0022 (amendment: separation of duties, `Forbidden` in `security_event`)
- ADR-0021 (delete rules per role)
- `domain-model.md` section 2.3
- `api.md` section 3 (permission matrix)
