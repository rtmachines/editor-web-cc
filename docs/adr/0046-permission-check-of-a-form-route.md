# ADR-0046: Permission check of a form route

Status: Accepted. Amended by ADR-0049 (the approve, retire, discard, delete, and restore routes keep the order of the Risks section in (l))
Date: 2026-10-15
Deciders: project owner

## Context

ADR-0013 puts the permission check into the first statement of every use case. The HTTP layer only authenticates (`api.md` section 3.2). ADR-0041 (a) lets `recipe-web` read every field of a form before it calls a use case.

The code review of step P3.3 found that this order gives a wrong answer to a user without the permission:

- An Operator sends a rename of a recipe with an empty name. The route reads the fields first and answers `422 E-WEB-008`. The answer holds the rename form out of band, with the inputs of the form.
- A translation route answers `422` and an editable form to an Operator. The value routes and the rebind route answer `422` from the reader of the fields.
- These answers write no `forbidden` event, so the security events miss the attempt (ADR-0022).
- An unknown field gives the same `422`, because each request struct has `deny_unknown_fields`.

A route can run its form query before it reads the fields. The form query checks the permission first. But one form query can serve more than one command:

- `GetRecipeForm` checks `RecipeCreate`. A rename needs `RecipeEdit`, so a rename route cannot run `GetRecipeForm` as it is.
- The `forbidden` event of a form query names the form query, not the command that the user tried.

`api.md` section 2.6 gives `GetLimits` to `GET .../parameters/{key}/limits`. The route runs `GetParameter` in its place.

## Decision drivers

- A user without the permission of a route gets `403 E-WEB-002` and one `forbidden` event (R28, ADR-0013, ADR-0022).
- A user without the permission sees no form and no text of a field.
- The permission check stays the first statement of a use case in `recipe-app`. `recipe-web` makes no authorization decision (ADR-0013).
- Each `hx-*` attribute holds a literal, and the answers of ADR-0041 do not change for a user with the permission (ADR-0037).

## Considered options

### Option A: The form query with a purpose runs before the fields

- Pros: the form query already has the data of the out-of-band form of ADR-0041 (e). One query serves the check and the form of an error answer.
- Cons: the input of each form query gets a purpose. A write route reads the store once more before its command.

### Option B: A use case that only checks a permission

- Pros: no read of data before the command.
- Cons: a new use case that the matrix of `api.md` section 3 does not hold. An error answer still needs the form query for its out-of-band form.

### Option C: The fields first, as today

- Pros: no change.
- Cons: a user without the permission gets `422` and a form, and the security events miss the attempt.

## Decision

The project uses **Option A**. This ADR amends ADR-0041 (a) and refines ADR-0013 for the routes of the web layer.

- (a) A route of point (c) runs a use case that checks the permission of the route before the route reads its fields.
  - A write route runs the form query of its command first. Then it reads the fields of the body, and then it runs the command.
  - The route parses its path values before the form query, because the form query needs them. A bad path value keeps its code of `api.md` section 1.
  - A user without the permission gets `403 E-WEB-002` and one `forbidden` event. The user never gets `422` and never gets a form.
  - This also holds for a field that the form does not know.
- (b) A form query serves one or more commands. Its input names the command that it serves, the purpose.
  - The form query checks the permission of that command. The `forbidden` event names that command.
  - `authorize` stays the first statement of the form query (ADR-0013). It takes the operation of the purpose.
  - The page route of a form, such as `GET /recipes/new`, runs the form query with the purpose of the command that the form sends.
- (c) The form queries serve these commands. Each permission comes from the matrix of `api.md` section 3.3.

  | Form query | Purpose | Permission |
  |---|---|---|
  | `GetMachineForm` | `CreateMachine`, `RenameMachine` | `MachineManage` |
  | `GetUnitForm` | `AddUnit`, `RenameUnit` | `UnitManage` |
  | `GetUdtForm` | `DefineUdt` (a new type and a new revision) | `UdtManage` |
  | `GetRecipeForm` | `CreateRecipe` | `RecipeCreate` |
  | `GetRecipeForm` | `RenameRecipe` | `RecipeEdit` |
  | `GetVersionForm` | `CreateVersion` | `VersionCreate` |
  | `GetParameterForm` | `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, `RemoveParameter`, `RebindTypeRevision` | `ParameterEdit` |

- (d) `GET .../parameters/{key}/limits` runs `GetLimits`, as `api.md` section 2.6 says. `GetLimits` returns the type tree that the limits grid needs.

## Consequences

### Positive

- A user without the permission of a form route gets `403 E-WEB-002` and sees no form.
- Each refused attempt writes one `forbidden` event that names the command.
- The check of a route stays in `recipe-app`, and the matrix of `api.md` section 3.3 stays the only source of a permission.

### Negative

- Each write route of point (c) reads the store once more before its command.
- An error of the form query, such as `E-RCP-002` or `E-RCP-012`, comes before the field errors. The command gives the same code, so the user gets the same error.
- The source tests of the first statement must accept the operation of a purpose in place of `Self::OPERATION`.

### Risks and mitigations

- Risk: a new form route reads its fields first. Mitigation: a route test sends a bad field as an Operator to each route of point (c) and expects `403 E-WEB-002` and one event.
- Risk: the archive routes of `api.md` section 2.2 and the routes of section 2.8 have no purpose in point (c). Mitigation: they keep their order until a later decision. The routes of the users and of the own password keep their error answers until step P4.2, as ADR-0041 (g) says.

## Related

- ADR-0013, refined by this ADR: the check of a form route runs in its form query.
- ADR-0041, amended by this ADR in (a): the order of the fields and the use case.
- ADR-0022, ADR-0037, ADR-0039
- `api.md` sections 2 and 3
- `overview.md` section 7: R28
