# ADR-0049: Approval preview and lifecycle actions

Status: Accepted. Amended by ADR-0052 (export link in the action area). Amended by ADR-0057 (no `HX-*` header on the answer of the backstop panic layer in (k))
Date: 2026-10-16
Deciders: project owner

## Context

Step P3.4 adds the pages that approve a draft, retire a version, discard a draft, delete a recipe, and restore it. The commands `ApproveVersion`, `RetireVersion`, `DiscardDraft`, `DeleteRecipe`, and `RestoreRecipe` exist in `recipe-app`. No route and no page uses them.

`api.md` sections 2.4 and 2.5 give the routes, the requests, and the answers. ADR-0022 asks for an approve page with the diff to the current Approved version and a form with the `row_version` of the draft. ADR-0018 asks the page to list the outdated parameters and to offer the flag `allow_outdated`.

The documents leave these points open:

- How the user accepts an action. A modal `dialog` needs the script call `showModal()`, and `api.md` section 9 permits no new script.
- Whether the preview shows the blockers of an approval before the `POST`. Today `E-RCP-015` and the list of `E-RCP-016` come only after the click.
- Where the restore action stands. ADR-0043 (e) gives the pages of a deleted recipe no form.
- Which URL the browser shows after an HTMX approval, because the answer `200` comes from the `POST` route.
- Whether the forms of these actions are edit forms. The web-ui spec "Version page" and design D7 of step P3.3 give the edit forms only to a live `Draft` and to `ParameterEdit`.

## Decision drivers

- A second person sees every change before the approval (ADR-0022, R27).
- A user sees the reason that blocks an approval before the user sends it (R39).
- No new script, no inline style, and each `hx-*` value is a literal (`api.md` section 9, ADR-0037).
- A danger action follows `design-system.md` section 10.5.
- No change of a binding rule: `api.md` section 2 keeps its answers, and ADR-0043 (e) keeps the pages of a deleted recipe without a form.

## Considered options

### Option A: The preview page is the review step, danger disclosures for the other actions

- Pros: no script. The approver reads the diff and the blockers on the page that holds the form. The other actions use the existing pattern of section 10.5.
- Cons: the preview reads the whole draft for the checks, as the approval does.

### Option B: A `details` element for the approval too

- Pros: one pattern for every action.
- Cons: the approval needs the diff and the outdated list. A disclosure on the version page cannot hold them.

### Option C: A modal `dialog`

- Pros: a known pattern.
- Cons: it needs a new script (`showModal()`), which `api.md` section 9 forbids.

For the blockers, the options are a check on the preview or the errors of the `POST` only.

For the restore action, the options are:

- a form in each deleted row of the recipe list,
- a form on the page of the deleted recipe,
- both.

The second and the third option change ADR-0043 (e).

## Decision

The project uses **Option A**, the blockers on the preview, and the restore form in the recipe list.

- (a) `GET /recipes/{ref}/versions/{n}/approve` is the review step of the approval. The page holds the header of the draft, the diff of ADR-0050, the blockers, and the approve form.
  - The approve form holds the outdated parameters of point (b). When an Approved version exists, a notice in the form says that the approval retires that version and names its number.
  - A page without the approve form shows no outdated list and no notice of the retired version. The user cannot approve from that page.
  - The blockers, the notice of `E-RCP-015`, and the notice of a changed draft (ADR-0050 (h)) stand right after the header. A keyboard user therefore reaches them before the diff.
  - Without a blocker, the first item of the diff summary links to the `h2` of the approve form. Its text is the key `approve-go-to-form`.
  - The page has no `details` element and no dialog for the approval.
- (b) The approve form sends `POST` to the same path. It holds the hidden fields `csrf_token` and `row_version` and the attribute `hx-sync="closest form:drop"`.
  - `row_version` is the row version of the draft in the snapshot of the first preview page (ADR-0050 (h)).
  - The checkbox `allow_outdated` with the value `true` shows only when the draft has an outdated parameter. Its label says that the audit entry records the choice.
  - A `422 E-RCP-024` answer shows its error line under the checkbox `allow_outdated` (ADR-0041), when the preview offers the approve form.
  - When the preview offers no approve form, the answer holds the line in the summary and swaps no form (point (a)).
  - Only the answer with the form links the summary line to the checkbox. The label key of the field is `field-allow-outdated`.
  - The form has one primary button with the text "Approve version {n}".
- (c) `GetApprovalPreview` runs the checks of the approval and writes nothing. It uses the same domain functions as `ApproveVersion`.
  - It calls `Recipe::check_approvable`, then `Draft::check_approver`, then `Draft::check_for_approval` with `allow_outdated = true`. The outdated parameters have their own list (point (b)).
  - `Draft::check_approver` checks the separation of duties for the current user with `ctx.settings.approval_policy`. It runs before `check_for_approval`, so the page can show `E-RCP-015` together with the other blockers.
  - `recipe-domain` gets one public function for this rule. It takes the creator, the editors, the approver, and the policy. `Draft::check_approver` and `Recipe::approve` call it, so the rule has one place. `Recipe::approve` passes the creator and the editors of the approval token.
  - A blocker of the separation of duties shows as a notice with the code `E-RCP-015` and its sentence.
  - The errors of `check_for_approval` (`E-RCP-008`, `E-RCP-016`, `E-RCP-020`, `E-RCP-022`, `E-RCP-023`) show as a list with one line per entry. Each line names the parameter and links to its parameter page.
  - The blockers stand in one notice of the macro `notice` of `components/alert.html`. The notice holds the `h2` of the key `approve-blockers-heading` and a plain list. It has no `role="alert"`, because the page answers a `GET`. The page does not use `field-errors.html`.
  - The list shows at most 20 lines, the bound of the error list of ADR-0041 (d). The sentence `errors-more` follows and counts the other lines.
  - When a blocker exists, the page shows no approve form. The `POST` runs every check again.
- (d) The preview answers `409 E-RCP-007` for a version that is not a live `Draft`. It answers `409 E-RCP-012` to every role with `VersionApprove` for a deleted recipe, as a form route of `api.md` section 2.4 does. A role without `VersionApprove` gets `403 E-WEB-002`, because the permission check comes first (ADR-0013).
- (e) A success of the approval answers `200` with the version page of the approved version (`api.md` section 2.5).
  - The toast names the approved version and, when one exists, the retired version. `ApproveVersion` returns the number of the retired version.
  - An `AlreadyApproved` result answers the same page with the toast "Version {n} is already approved." It writes no audit entry.
  - An HTMX answer sets `HX-Push-Url` to the path of the version page. A reload then reads the version page and does not send the `POST` again. The header is a response header, not an `hx-*` attribute (ADR-0037 (f)).
  - When the answer holds a toast, the `h1` of the new content has `aria-describedby="toast"`. A screen reader then reads the toast with the heading after the swap.
- (f) The retire action stands on the page of an Approved version. The discard action stands on the page of a live `Draft`. Each shows only when the matrix row of its command allows the role.
  - Each action is one `details` element of `design-system.md` section 10.5. Its `summary` names the action and the version. A notice sentence states the effect, and the button names it in `aria-describedby`.
  - The notice has the fixed id `danger-notice` and no role. A page holds at most one danger action, so the id is unique.
  - The form holds `csrf_token`, `row_version` of `VersionDetail`, and `hx-sync="closest form:drop"`. The button names the object, for example "Retire version 6".
  - A retire success answers `200` with the version page and sets `HX-Push-Url` as in point (e). A discard success answers `303` to `/recipes/{ref}` (`api.md` section 2.5).
  - The discard answer sets no `HX-Trigger`. The browser follows the `303` before HTMX reads a header, and the discarded draft has no page with a `row_version` field.
- (g) The delete action stands on the recipe page in one `details` element of section 10.5. It shows to each role with `RecipeDelete`.
  - The form holds `csrf_token` and `hx-sync="closest form:drop"`. It has no `row_version`, because `DeleteRecipe` takes none (`api.md` section 2.4).
  - The page does not copy the delete rule of the domain. The server answers `409 E-RCP-010` or `409 E-RCP-026`.
- (h) The restore action stands in each deleted row of the list `/recipes?show_deleted=1`. The list needs `RecipeRestore`, so only the Administrator sees it.
  - The form holds `csrf_token` and `hx-sync="closest form:drop"`. Its secondary button names the reference number. A restore loses no data, so it is not a danger action.
  - The delete stays a danger action of section 10.5, because the user who deletes cannot always undo it. Only an Administrator can restore a recipe.
  - The row also links to the page of the deleted recipe. That page keeps no form (ADR-0043 (e)).
- (i) The approve link and the forms of points (f) to (h) are action forms, not edit forms.
  - The rule "only a live `Draft` and a user with `ParameterEdit` get the forms" names the edit forms of parameters. It comes from `api.md` section 2.5 and design D7 of step P3.3.
  - An action form shows when the matrix row of its command allows the role and the state of the version permits the transition.
  - The version page gets an action area with these items:
    - the link "Approve" for a live `Draft` and a role that the row of `ApproveVersion` allows,
    - the danger actions of point (f),
    - the links "Compare with the approved version {number}" and "Compare with the base version {number}", each with the number of the linked version,
    - the link "History".
  - The links of this list stand in one row of links after the header `dl` of the version, with no heading. The recipe page uses the same row.
  - A compare link opens the diff with the lower version number as the old side (ADR-0050 (i)). The page shows no link to its own version and one link for one version.
  - The danger actions stand at the end of `main`, as on the recipe, machine, and unit pages. The version page puts the `h2` "Actions" before them. The content of the version therefore keeps its own headings.
- (j) Every form of the five actions has `hx-sync="closest form:drop"`. `api.md` section 4 rule 7 names the approve and save forms. This point extends the rule to the other action forms.
  - Each of these forms holds an indicator with the class `htmx-indicator` and the text of the key `form-in-progress`. The user therefore sees that the request runs.
  - The forms have no `hx-disabled-elt`. A disabled button loses the focus, and `design-system.md` section 10.6 keeps the focus where it is. `hx-sync` already drops a second request.
- (k) Every error of the five `POST` routes follows `api.md` section 5.3 and ADR-0047 (b): `HX-Retarget`, `HX-Reswap`, `HX-Push-Url: false`, and an empty `#toast` out of band.
- (l) The five `POST` routes have no form query of ADR-0046 (c). Each route reads its body before its command, as the routes of the Risks section of ADR-0046 do.
  - A user without the permission who sends a malformed body gets `422 E-WEB-008` and writes no `forbidden` event.
  - The answer holds no data of the recipe and no form, because the route reads no data before the command.
  - A well-formed body of such a user gets `403 E-WEB-002` and one `forbidden` event from the command.
- (m) The toast of point (e) names two versions, so a view below 48rem can show it in three lines. This point amends ADR-0047 (c).
  - Below 48rem, `app.css` gives `html` a scroll padding that covers a toast of four lines and the gap.
  - The rule is `calc(var(--space-3) * 2 + var(--edge) * 2 + var(--text-base) * var(--leading) * 4 + var(--space-4))`, which is 144 px.
  - From 48rem, `html` keeps `calc(var(--target) + var(--space-4))`, because the toast then needs one line.
  - A focused element therefore does not hide under the toast (`design-system.md` section 10.7, WCAG 2.4.11).
  - A toast with a long code or a long unit path can need more than four lines. A later step handles that toast.

## Consequences

### Positive

- The approver sees the diff, the outdated parameters, and each blocker before the click.
- No new script. Every action uses a pattern of `design-system.md` section 10.5 or a plain form.
- A reload after an approval shows the version page and sends no second `POST`.

### Negative

- The preview reads the whole draft and the type trees for the checks, as `ApproveVersion` does.
- The page of a deleted recipe has no restore button. The Administrator restores from the list.

### Risks and mitigations

- Risk: the draft changes between the preview and the `POST`. Mitigation: the form sends `row_version`, and the `POST` runs every check again. A stale form gets `409 E-WEB-001` (`api.md` section 4) when the draft passes the first check before the transaction. A fault of that first check gives its own code first (`data-model.md` section 6).
- Risk: the security events miss an attempt with a malformed body (point (l)). Mitigation: the answer holds no data, and a well-formed attempt writes one `forbidden` event. A later decision can add a check before the body, as ADR-0046 says for its other routes.
- Risk: the preview and the approval disagree on a rule. Mitigation: both call the same domain functions. A test compares the blockers of the preview with the errors of the `POST` for each code of point (c).

## Related

- ADR-0018, ADR-0022, ADR-0028, ADR-0037 (f), ADR-0041, ADR-0043 (e), ADR-0047, ADR-0050, ADR-0051
- ADR-0046, amended by this ADR in (l): the five `POST` routes keep the order of the routes of its Risks section
- `docs/architecture/api.md` sections 2.4, 2.5, 4, 5.3
- `docs/architecture/design-system.md` section 10.5
- `overview.md` section 7: R27, R39. Section 9: A5, A28, A29, A34
