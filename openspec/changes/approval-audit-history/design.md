## Context

Step P3.4 adds the approval preview, the lifecycle actions, the diff of two versions, the recipe history, the audit pages, and the security event page. The commands `ApproveVersion`, `RetireVersion`, `DiscardDraft`, `DeleteRecipe`, and `RestoreRecipe` exist in `recipe-app`. The queries `GetApprovalPreview`, `DiffVersions`, `GetRecipeHistory`, `ListAuditEntries`, `GetAuditEntry`, and `ListSecurityEvents` have only their `Operation` variants and matrix rows.

Normative sources:

- `api.md` sections 1, 2.4, 2.5, 2.9, 3, 4, 5, 7.3, 9,
- `reliability.md` sections 1.2, 2, 4.4, 7.4, 7.11,
- `domain-model.md` sections 2, 5, 6, 7, and `data-model.md` sections 3.8, 3.15, 5, 6, 10,
- `design-system.md` sections 8, 9, 10,
- ADR-0049, ADR-0050, ADR-0051.

The binding decisions of the archived designs of steps P3.1 to P3.3 stay valid.

## Goals / Non-Goals

Goals:

- The pages and routes of ADR-0049, ADR-0050, and ADR-0051 in `pl` and `en`.
- Read ports with an in-memory adapter and a `sqlx` adapter that give the same rows.
- A pure diff in `recipe-domain`, with the bounds of ADR-0050.
- The tests of section D10, including the full approval scenario on PostgreSQL.

Non-Goals:

- Export (step P3.5), a toast after the `303` answers, a time zone other than UTC, the load test (step P4.2).
- A new index, a migration, a grant, or a dependency.

## Decisions

### D1. Answers of the user (2026-10-16)

Every answer took the recommended option. The ADR of each answer:

| Question | Answer | ADR |
|---|---|---|
| Variant of the shotgun | V3, the changes grouped by kind, limits first | ADR-0050 (d) |
| a. Confirmation | The preview is the review step with the blockers. Danger disclosures for retire, discard, delete. `HX-Push-Url` after approve and retire | ADR-0049 |
| b. Diff | All parts, pure function in `recipe-domain`, SQL change flags, one page of pairs, first approval shows every parameter | ADR-0050 |
| c. Sizes | `page.audit`, `page.history`, `page.security_events` 100, `page.diff` 50, `diff.leaf_rows` 1 000, `diff.permits` 2 | ADR-0050 (m), ADR-0051 (k) |
| d. Security events | In P3.4. `login` by the user and by the HMAC under the current key of the ring (`KeyRing::sign`) | ADR-0051 (h) |
| e. Audit entry | Full formatted JSON, no cut. A ProcessEngineer gets `404 E-WEB-005` outside the scope | ADR-0051 (c), (f) |
| f. Filters | `aggregate` is an aggregate id from a link | ADR-0051 (d) |
| g. Restore | A form in each deleted row of `show_deleted` | ADR-0049 (h) |
| h. Scenario test | `recipe-db` test through the use cases with PostgreSQL | D10 |
| Isolation | `RepeatableRead` for the preview and the diff | ADR-0050 (f) |
| Navigation | Fifth link "Audit" after "Users" | ADR-0051 (j) |
| Limit color | Error tokens with the icon `diff-limit` | ADR-0050 (k) |
| Plan review | Every recommendation of the plan review, also task 11.3 without `[hard]` | this design |
| Unknown `actor` | A well-formed `actor` that names no user gives the empty list. A malformed `actor` gives `422 E-WEB-008` | ADR-0051 (d) |

Orchestrator decisions: `api.md` section 2 keeps its answers, so a toast after a `303` is a follow-up. `conflict.html` keeps the link to the version page (`api.md` section 4).

### D2. Module layout

`recipe-domain`:

- `src/recipe/diff.rs` and `src/recipe/diff/` (`pairs.rs`, `definition.rs`, `values.rs`, `limits.rs`, `translations.rs`, `rows.rs`, tests). The public items are `DiffPair` (with `match_by_key`), `DiffRows`, `DiffRow`, `DiffKind` (with `parse` and `as_str`), `DiffKindReason`, `DiffPart`, `DiffBounds`, `DefinitionChange`, and `diff_pairs`. A type shows as its `PlcTypeId`, and `recipe-app` writes the type text.
- `src/recipe/root/approve.rs`: the function `breaks_separation_of_duties` with the creator, the editors, the approver, and the policy. `Recipe::approve` calls it with the creator and the editors of the token.
- `src/recipe/draft/approval.rs`: `Draft::check_approver`. It calls the same function and runs before `check_for_approval` (ADR-0049 (c)).

`recipe-app`:

- `src/ports/audit_reads.rs` (port `AuditReads`), `src/ports/diff_reads.rs` (port `DiffReads`), accessors in `ports/unit_of_work.rs` and `ports/mod.rs`.
- `src/memory/audit/`: the read model of the entries and the events, with the aggregation of the events.
- `memory/core/store.rs` (ids, times, and the sink), `memory/sink.rs` (id and time of each event), `memory/recipe/diff_reads.rs`.
- The store owns the sink. `events()` stays a flat list of the recorded events. Only the read model aggregates (ADR-0051 (a)).
- `src/audit/queries/` (`list.rs`, `entry.rs`, `history.rs`, `security_events.rs`, `scope.rs`, tests). `security_events.rs` builds the filter of the security events.
- `src/recipe/queries/diff.rs` (`DiffVersions`) and `approval_preview.rs` (`GetApprovalPreview`).
- `recipe/approve_version.rs` returns `ApproveVersionOutput { outcome, retired }`. `retired` is the number of the retired version. `ApprovalOutcome` of the domain does not change.
- `src/blocking.rs`: `PageSizes` gets `audit`, `history`, `security_events`, `diff`. `DiffSettings` holds `leaf_rows` and the number `permits`. `TimeoutStage::Diff` and `TaskStage::Diff`.
- `src/context.rs`: `AppContext::new` builds the semaphore of `diff.permits` from the settings. Every clone of the context shares it (ADR-0050 (g)).

`recipe-db`:

- `src/transaction/audit_reads.rs`, `security_event_reads.rs`, `diff_reads.rs`, accessors in `src/transaction.rs`.
- `tests/storage/contract/audit_reads/`, `contract/diff_reads/`, `tests/storage/scenario/approve_retire.rs`.

`recipe-web`:

- `i18n/en-approval.ftl` and `i18n/pl-approval.ftl`: the second resource file of each language. `src/i18n.rs` loads both files into one bundle, so each file stays below 800 lines.
- `src/error/values.rs`: the key `task-stage-diff` of `TaskStage::Diff`.
- `src/pages/approval.rs` and `src/pages/approval/` (preview page, approve `POST`, rv, blockers).
- `src/pages/diff_view.rs` (the view of the diff sections, shared by the preview and the diff page), `templates/components/diff.html`.
- `src/pages/versions/actions.rs` (retire, discard), `src/pages/recipes/lifecycle.rs` (delete, restore).
- `src/pages/diff.rs` (diff page), `src/pages/history.rs`, `src/pages/audit.rs` and `src/pages/audit/` (list, entry, filter values), `src/pages/security_events.rs`.
- Templates `approve.html`, `diff.html`, `history.html`, `audit.html`, `audit-entry.html`, `security-events.html`. Components `danger.html` (a new macro with `row_version`), `pager.html` (a new cursor macro), `icons.html` (four icons).

### D3. Approval preview

`GetApprovalPreview` (ADR-0049 (c), ADR-0050 (f), (g), (h)) runs these steps:

1. `authorize` with `VersionApprove` (ADR-0013).
2. Start one deadline of `timeout.validation`, then take a `diff.permits` permit. The wait for the permit and the blocking task of step 4 share this deadline (ADR-0050 (g)).
3. One `RepeatableRead` transaction reads the root (no replay rule) and runs `check_approvable`. It reads the stored draft, the machine, the live units, the type rows, the current Approved version, and the editors. It reads the change flags, the counts, and the pairs of the requested page. The new side of a pair comes from the loaded draft.
4. The transaction ends. One blocking task with the rest of the deadline runs three steps in this order:
   1. `Draft::check_approver` for the current user with `ctx.settings.approval_policy`,
   2. `diff_pairs`,
   3. `check_for_approval` with `allow_outdated = true`. It consumes the draft, so it runs last.
5. The result holds the permit, the `row_version` of the snapshot, the retired number, the summary, and the rows of the page. It also holds the blockers, the outdated parameters, and the flag "may approve". `recipe-web` drops the permit after the render.

The step order of the errors equals `ApproveVersion`: `E-WEB-005`, `E-RCP-012`, `E-RCP-007`. A blocker is data of the result, not an error. `E-RCP-015` and the faults of `check_for_approval` can show together.

When the review of the page fails, for example on a type that does not resolve, the checks of step 4 still run. The review is the outdated list and the diff of the page. With a blocker, the result holds the blockers, no diff (`None`), and no outdated parameters. Without a blocker, the query returns the error of the review. The blockers therefore always equal the errors of `ApproveVersion` on the same state. A page without a diff shows the header and the blockers only, with no summary and no section.

The result also holds the key of each parameter tag that a blocker of the draft names (`blocker_keys`). The web layer links each blocker line to the parameter page with that key (ADR-0049 (c)). A line without a tag, such as `E-RCP-008`, has no link.

The page shows the header, then the blockers, the notice of `E-RCP-015`, and the notice of a changed draft. The summary and the sections follow (ADR-0049 (a)).

- The faults of `check_for_approval` stand in one notice with the `h2` of `approve-blockers-heading` and a plain list. That `h2` shows only when such a fault exists.
- `E-RCP-015` stands in its own notice with its code, its sentence, and `approve-blocker-editor-hint` (ADR-0049 (c)).
- Both notices use the macro `notice_block` of `components/alert.html`. It has the markup of `notice` and holds a block, because `notice` holds one text only.

The approve form follows the `h2` of `approve-form-heading` (user decision on the review of 10.1):

- The notice of the retired version stands in the form, before its fields.
- The outdated parameters stand in one `fieldset` of the form. Its `legend` is `approve-outdated-legend`. The list, the checkbox `allow_outdated`, its hint, and the error line of `E-RCP-024` follow in one `.field` wrapper.
- A page without a form shows no retire notice and no outdated list. A page with `diff` set to `None` therefore shows the header and the blockers only.

`rv`: the web layer reads `rv` as an optional positive integer (`422 E-WEB-008` for another text). When `rv` differs from the snapshot, the page shows `approve-changed-draft` and no form.

- A bad `kind` or `rv` gives the reason `FieldReason::Query` with the label `field-kind` or `field-rv`, as the filters of the audit list do.
- `rv` stands on each link to another page of the preview: a page of a section, a section with `kind`, and the first page. A link to a parameter page or to the version page has no `rv`, because those pages take no `rv` (ADR-0050 (h)).

### D4. Diff reads and rows

- `DiffReads::change_flags(old, new)` returns one row per key. `old` is an `Option`. It is `None` for a first approval, and then every key is only in the new version. The row holds these fields:
  - the flags `definition`, `value`, `limits`, `translations`, `only_old`, and `only_new`,
  - the order fields of the new side: unit path, ordinal, and tag. A key only in the old version takes the order fields of the old side,
  - the row counts of the limits, the definitions, and the translations.
- The SQL compares the JSONB values with `IS DISTINCT FROM`. It compares the child rows with a full outer join on the key and the leaf path or the language tag.
- A pair with a changed `plc_type_id` sets the flag `value`. It sets the flag `limits` when one side has a limit row. `diff_pairs` then gives every leaf and every limit row of both sides (ADR-0050 (c)).
- Order of the rows of one parameter (ADR-0050 (d)):
  - Limit rows: the preorder position of the first covered leaf. A row of the old side only takes its position in the old tree. A `[*]` row comes first at the same position.
  - Value rows: first the leaves of the new tree in preorder. Then the leaves of the old side only, in the preorder of the old tree.
  - At a mode change of an array of a struct, the rows of the two sides can interleave. Example: old `[1].setpoint`, `[2].setpoint`, new `[*].setpoint`, `[*].ramp` give `[*].setpoint`, `[1].setpoint`, `[*].ramp`, `[2].setpoint`.
- With a changed `plc_type_id`, the limit row count of a pair is the number of distinct leaf paths of both sides. The contract test of D2 holds this case.
- `DiffReads::pairs(old, new, keys)` returns the stored parameters of the given keys, one list for each side. `old` and `new` are each an `Option`. A side of `None` reads no row. The preview sends `new = None`, because the new side comes from the loaded draft (D3).
- The use case picks the entries of the page from the flags. An entry is one key in one section, so a key in two sections counts twice. The entries come in the order of the sections, then in the order of the parameter table.
  - Without `kind`, page `n` holds the entries `n × page.diff` to `(n + 1) × page.diff − 1` of that order. Page 0 therefore fills the sections in their order up to `page.diff` entries.
  - With `kind`, page `n` holds the entries of that section in the same range.
  - The use case reads the pairs of the distinct keys of these entries only (ADR-0050 (e)).
  - The page result names each section that has an entry outside the page. The web layer links each of these sections to its own pages with `kind`. This includes a section that the page does not show at all.
- `diff_pairs` gives the rows of each part. A value row holds the effective limit of the new side for its leaf (a `[*]` row or the element row). A parameter with more rows than `diff.leaf_rows` in one section gives the first rows and the count of the others.
- The web view shows one table row for each row of `diff_pairs` (user decision on the review of 10.1). A limit row shows each side as one limit text (`diff-limit-range`, `diff-limit-min-only`, `diff-limit-max-only`, `diff-limit-max-length`). A translation row shows the name and the description of each side for one language. The counts of the summary therefore equal the rows of the tables.
- The memory adapter computes the same flags from its rows. The contract test compares both adapters on one case list.
- In a `RepeatableRead` transaction, the memory adapter reads the draft from the state of the transaction start (ADR-0050 (f), which amends ADR-0042 (e)).

### D5. Audit read port and adapters

- `AuditReads::entries(filter, scope, before, limit)`, `entry(id, scope)`, `history(recipe, before, limit)`, `security_events(filter, before, limit)`, `aggregate_label(aggregate_type, aggregate_id)`. Each list returns `limit` rows and a flag "older rows exist". The SQL reads `limit + 1` rows.
- `ListAuditEntries` with the filter `aggregate` reads the label of the aggregate after a page with rows. The type comes from the `aggregate_type` of the newest row. An empty page reads no label, and the page shows the short id (ADR-0051 (d)).
- Only a `struct` row gives the label of a type. A derived type (`ARRAY`, `STRING(n)`, `WSTRING(n)`) and a primitive give the type word and the short id.
- `scope` is `All` or `Recipes`. `Recipes` adds `aggregate_type = 'recipe'`. The audit list and the entry include the entries of a deleted recipe (ADR-0051 (c)).
- The `sqlx` adapter builds the statement with `sqlx::QueryBuilder` and binds each value. It never joins a value into the SQL text.
- A list row holds the id, the time, the actor login (a join on `app_user`), the action, and the entity type. It also holds the entity id, the aggregate type, the aggregate id, and the label. No state column.
- Labels: a join on the current row of the entity. The adapter reads no `before_state` for a label. A row that no longer exists gives the word of its type and the short id (ADR-0051 (e)).
- `actor`: the adapter looks up the user id of the login. No user gives the empty list without a further read (ADR-0051 (d)).
- The memory adapter keeps the entries in commit order. The id is the position plus 1. The store owns the sink, and the sink keeps the id and the clock time of each event. The read model aggregates the events.

### D6. Filter values

`recipe-web` parses the query of `/audit` and `/security-events` into the input types of `recipe-app` (ADR-0051 (d), (h)).

- A malformed value gives `422 E-WEB-008` through `query_values` (`pages/path_values.rs`).
- `actor` parses as a `Login`, and `login` as a `TypedLogin`. A well-formed `actor` that names no user gives the empty list.
- A date parses as `YYYY-MM-DD`. The parser accepts the years 1 to 9999.
- The web layer converts `to` into the start of the next day. The date 9999-12-31 has no next day, so that `to` sets no upper bound. A test covers it.
- No new dependency: the `time` feature `parsing` stays off.
- `recipe-app` owns the day type `UtcDay` (one `parse`) and the function that makes a `TimeRange` from the two days. `recipe-web` calls both and does not use `time`.

The filter form is a boosted `GET` form with literal `hx-*` attributes (ADR-0051 (i)). A `422` answer goes into `#error-panel`, and the form keeps its values. D8 holds the label keys of the filter fields.

### D7. Pages and fragments

| Page | Template | HTMX answer |
|---|---|---|
| Approval preview | `approve.html` with `components/diff.html` | content of `main` (ADR-0037) |
| Approve `POST` success | `version.html` | content of `main`, `HX-Push-Url`, `HX-Trigger`, `aria-describedby="toast"` on the `h1` (ADR-0049 (e)) |
| Retire `POST` success | `version.html` | content of `main`, `HX-Push-Url`, `HX-Trigger`, `aria-describedby="toast"` on the `h1` |
| Discard, delete, restore success | - | `303` (boosted form follows it) |
| Diff page | `diff.html` with `components/diff.html` | content of `main` |
| History | `history.html` | content of `main` |
| Audit list, entry, security events | `audit.html`, `audit-entry.html`, `security-events.html` | content of `main` |

Every action form is a boosted form with this literal `hx-*` set: `hx-boost="true" hx-target="#content" hx-push-url="true" hx-sync="closest form:drop" hx-disabled-elt="find button"`. Its `action` holds the URL with user data (ADR-0037 (f)). It holds the indicator of `form-in-progress` (ADR-0049 (j)).

The filter forms of `/audit` and `/security-events` send `GET` with `hx-boost="true" hx-target="#content" hx-push-url="true"`. Their `action` is the literal path of the page.

The blockers of the preview use the macro `notice` of `components/alert.html`, not `field-errors.html` (ADR-0049 (c)).

### D8. Texts

Every key exists in `i18n/en-approval.ftl` and `i18n/pl-approval.ftl` (task 2.2). The table holds every new key. The error messages of the codes of D9 already exist, except `error-E-EXP-004`. Each new Polish text uses the word "szkic" of the key `status-draft`.

The label key of a field is `field-` and the field name with `_` replaced by `-` (`design-system.md` section 10.5). `field-login` and `field-page` exist. The table holds the other field keys of the filters, of the preview, and of the approve form.

| Key | en | pl |
|---|---|---|
| `nav-audit` | Audit | Audyt |
| `version-actions-heading` | Actions | Akcje |
| `version-approve-link` | Approve version { $number } | Zatwierdź wersję { $number } |
| `version-compare-approved` | Compare with the approved version { $number } | Porównaj z zatwierdzoną wersją { $number } |
| `version-compare-base` | Compare with the base version { $number } | Porównaj z wersją bazową { $number } |
| `recipe-history-link` | History | Historia |
| `retire-summary` | Retire this version | Wycofaj tę wersję |
| `retire-notice` | The recipe then has no approved version. The export of the approved version stops until the next approval. | Receptura nie będzie wtedy mieć wersji zatwierdzonej. Eksport wersji zatwierdzonej przestanie działać do następnego zatwierdzenia. |
| `retire-button` | Retire version { $number } | Wycofaj wersję { $number } |
| `discard-summary` | Discard this draft | Odrzuć ten szkic |
| `discard-notice` | Nobody can edit or approve this draft again. | Nikt nie będzie mógł już edytować ani zatwierdzić tego szkicu. |
| `discard-button` | Discard draft { $number } | Odrzuć szkic { $number } |
| `delete-recipe-summary` | Delete this recipe | Usuń tę recepturę |
| `delete-recipe-notice` | Only an Administrator can restore the recipe. The system also discards the draft of the recipe, if one exists. | Tylko Administrator może przywrócić recepturę. System odrzuci też szkic receptury, jeśli istnieje. |
| `delete-recipe-button` | Delete recipe { $reference } | Usuń recepturę { $reference } |
| `restore-recipe-button` | Restore { $reference } | Przywróć { $reference } |
| `form-in-progress` | The server processes the request. | Serwer przetwarza żądanie. |
| `toast-version-approved` | Version { $number } is approved. | Wersja { $number } jest zatwierdzona. |
| `toast-version-approved-retired` | Version { $number } is approved. Version { $retired } is retired. | Wersja { $number } jest zatwierdzona. Wersja { $retired } jest wycofana. |
| `version-already-approved` | Version { $number } is already approved. | Wersja { $number } jest już zatwierdzona. |
| `toast-version-retired` | Version { $number } is retired. | Wersja { $number } jest wycofana. |
| `approve-title` | Approval of version { $number } of recipe { $reference } | Zatwierdzenie wersji { $number } receptury { $reference } |
| `approve-retires-notice` | The approval of version { $number } retires version { $retired }. | Zatwierdzenie wersji { $number } wycofa wersję { $retired }. |
| `approve-first-notice` | Recipe { $reference } has no approved version. Every parameter is new. | Receptura { $reference } nie ma wersji zatwierdzonej. Każdy parametr jest nowy. |
| `approve-go-to-form` | Go to the approval form | Przejdź do formularza zatwierdzenia |
| `approve-form-heading` | Approval | Zatwierdzenie |
| `approve-outdated-legend` | Outdated parameters: { $count } | Nieaktualne parametry: { $count } |
| `approve-allow-outdated` | Approve with the outdated parameters | Zatwierdź mimo nieaktualnych parametrów |
| `approve-allow-outdated-hint` | The audit log records this choice. | Wpis audytu zapisze tę decyzję. |
| `approve-button` | Approve version { $number } | Zatwierdź wersję { $number } |
| `approve-back` | Back to version { $number } | Wróć do wersji { $number } |
| `approve-blockers-heading` | The approval is not possible | Zatwierdzenie nie jest możliwe |
| `approve-blocker-editor-hint` | Another user with the permission must approve this draft. | Ten szkic musi zatwierdzić inny użytkownik z uprawnieniem. |
| `approve-changed-draft` | The draft changed during the review. Start the review again. | Szkic zmienił się w trakcie przeglądu. Zacznij przegląd od nowa. |
| `approve-first-page` | Start the review again | Zacznij przegląd od nowa |
| `field-allow-outdated` | Approval with outdated parameters | Zatwierdzenie mimo nieaktualnych parametrów |
| `field-rv` | Change number of the review | Numer zmiany przeglądu |
| `field-kind` | Kind | Rodzaj |
| `field-aggregate` | Aggregate | Agregat |
| `field-entity-type` | Object type | Typ obiektu |
| `field-entity-id` | Object | Obiekt |
| `field-actor` | User login | Login użytkownika |
| `field-action` | Action | Akcja |
| `field-from` | From | Od |
| `field-to` | To | Do |
| `field-before` | Position in the list | Pozycja na liście |
| `field-client-ip` | Client address | Adres klienta |
| `task-stage-diff` | diff | porównanie wersji |
| `diff-title` | Changes of version { $new } of recipe { $reference } against version { $old } | Zmiany wersji { $new } receptury { $reference } względem wersji { $old } |
| `diff-summary-heading` | Summary of the changes | Podsumowanie zmian |
| `diff-pager-label` | Pages of the section { $section } | Strony sekcji { $section } |
| `diff-section-limits` | Process limits | Limity procesowe |
| `diff-section-values` | Values | Wartości |
| `diff-section-definitions` | Definitions | Definicje |
| `diff-section-translations` | Translations | Tłumaczenia |
| `diff-section-added` | Added parameters | Dodane parametry |
| `diff-section-removed` | Parameters only in version { $old } | Parametry tylko w wersji { $old } |
| `diff-count-parameters` | Parameters: { $count } | Parametry: { $count } |
| `diff-count-rows` | Rows: { $count } | Wiersze: { $count } |
| `diff-unchanged` | Parameters without a change: { $count } | Parametry bez zmian: { $count } |
| `diff-caption` | { $section }: { $count } | { $section }: { $count } |
| `diff-col-parameter` | Parameter | Parametr |
| `diff-col-unit` | Unit | Jednostka |
| `diff-col-path` | Path | Ścieżka |
| `diff-col-bound` | Bound | Granica |
| `diff-col-version` | Version { $number } | Wersja { $number } |
| `diff-col-limit` | Limit in version { $number } | Limit w wersji { $number } |
| `diff-col-field` | Field | Pole |
| `diff-col-lang` | Language | Język |
| `diff-col-name` | Name | Nazwa |
| `diff-col-type` | Type | Typ |
| `diff-col-value` | Value | Wartość |
| `diff-col-state` | Change | Zmiana |
| `diff-bound-min` | Minimum | Minimum |
| `diff-bound-max` | Maximum | Maksimum |
| `diff-bound-max-length` | Maximum length | Maksymalna długość |
| `diff-field-tag` | Tag | Znacznik |
| `diff-field-unit` | Unit | Jednostka |
| `diff-field-type` | Type | Typ |
| `diff-field-s7` | S7 address | Adres S7 |
| `diff-field-ordinal` | Order | Kolejność |
| `diff-field-name-en` | English name | Nazwa angielska |
| `diff-field-description-en` | English description | Opis angielski |
| `diff-field-tr-name` | Name | Nazwa |
| `diff-field-tr-description` | Description | Opis |
| `diff-none` | none | brak |
| `diff-limit-range` | { $min } to { $max } | { $min } do { $max } |
| `diff-limit-min-only` | at least { $min } | co najmniej { $min } |
| `diff-limit-max-only` | at most { $max } | najwyżej { $max } |
| `diff-limit-max-length` | Maximum length: { $max_length } | Maksymalna długość: { $max_length } |
| `diff-leaf-count` | Leaves: { $count } | Liście: { $count } |
| `diff-more-rows` | More rows: { $count } | Więcej wierszy: { $count } |
| `diff-open-version` | Parameter in version { $number } | Parametr w wersji { $number } |
| `diff-empty` | Versions { $new } and { $old } have the same parameters. | Wersje { $new } i { $old } mają te same parametry. |
| `diff-state-limit` | Limit changed | Zmieniono limit |
| `diff-state-value` | Value changed | Zmieniono wartość |
| `diff-state-definition` | Definition changed | Zmieniono definicję |
| `diff-state-translation` | Translation changed | Zmieniono tłumaczenie |
| `diff-state-added` | Added | Dodano |
| `diff-state-removed` | Only in the old version | Tylko w starej wersji |
| `history-title` | History of recipe { $reference } | Historia receptury { $reference } |
| `history-caption` | Entries of the history | Wpisy historii |
| `history-empty` | The recipe has no entries. | Receptura nie ma wpisów. |
| `audit-col-time` | Time | Czas |
| `audit-col-actor` | User | Użytkownik |
| `audit-col-action` | Action | Akcja |
| `audit-col-entity` | Object | Obiekt |
| `audit-actor-system` | system | system |
| `audit-entry-link` | Entry { $id } | Wpis { $id } |
| `audit-whole-recipe` | Entries of the whole recipe | Wpisy całej receptury |
| `audit-title` | Audit log | Dziennik audytu |
| `audit-filter-heading` | Filters | Filtry |
| `audit-filter-entity-type` | Object type | Typ obiektu |
| `audit-filter-any-type` | Every type | Każdy typ |
| `audit-filter-action` | Action | Akcja |
| `audit-filter-any-action` | Every action | Każda akcja |
| `audit-filter-actor` | User login | Login użytkownika |
| `audit-filter-from` | From (UTC, YYYY-MM-DD) | Od (UTC, RRRR-MM-DD) |
| `audit-filter-to` | To (UTC, YYYY-MM-DD) | Do (UTC, RRRR-MM-DD) |
| `audit-filter-submit` | Show | Pokaż |
| `audit-filter-clear` | Clear the filters | Wyczyść filtry |
| `audit-filter-aggregate` | Aggregate: { $type } { $label } | Agregat: { $type } { $label } |
| `audit-filter-aggregate-id` | Aggregate: { $label } | Agregat: { $label } |
| `audit-filter-aggregate-clear` | Show every aggregate | Pokaż każdy agregat |
| `audit-filter-object` | Object: { $type } { $id } | Obiekt: { $type } { $id } |
| `audit-filter-object-clear` | Show every object | Pokaż każdy obiekt |
| `audit-group-plant` | Plant model | Model zakładu |
| `audit-group-recipe` | Recipes | Receptury |
| `audit-group-identity` | Users and sessions | Użytkownicy i sesje |
| `audit-group-export` | Export | Eksport |
| `audit-group-settings` | Settings | Ustawienia |
| `audit-caption` | Audit entries | Wpisy audytu |
| `audit-empty` | No entry matches the filters. | Żaden wpis nie pasuje do filtrów. |
| `audit-newest` | Newest entries | Najnowsze wpisy |
| `audit-older` | Older entries | Starsze wpisy |
| `audit-no-older` | No older entries. | Brak starszych wpisów. |
| `audit-security-link` | Security events | Zdarzenia bezpieczeństwa |
| `audit-entry-title` | Audit entry { $id } | Wpis audytu { $id } |
| `audit-entry-time` | Time | Czas |
| `audit-entry-actor` | User | Użytkownik |
| `audit-entry-action` | Action | Akcja |
| `audit-entry-entity-type` | Object type | Typ obiektu |
| `audit-entry-entity` | Object | Obiekt |
| `audit-entry-aggregate` | Aggregate | Agregat |
| `audit-entry-request` | Request id | Id żądania |
| `audit-entry-client` | Client address | Adres klienta |
| `audit-entry-session` | Session reference | Referencja sesji |
| `audit-entry-chain` | Position in the chain | Pozycja w łańcuchu |
| `audit-entry-before` | State before | Stan przed |
| `audit-entry-after` | State after | Stan po |
| `audit-entry-no-state` | No state. | Brak stanu. |
| `audit-entry-object-link` | Entries of this object | Wpisy tego obiektu |
| `audit-entry-aggregate-link` | Entries of the whole aggregate | Wpisy całego agregatu |
| `audit-entity-machine` | Machine | Maszyna |
| `audit-entity-unit` | Unit | Jednostka |
| `audit-entity-plc-type` | PLC type | Typ PLC |
| `audit-entity-recipe` | Recipe | Receptura |
| `audit-entity-recipe-version` | Version | Wersja |
| `audit-entity-parameter` | Parameter | Parametr |
| `audit-entity-app-user` | User | Użytkownik |
| `audit-entity-app-setting` | Setting | Ustawienie |
| `audit-action-machine-created` | Machine created | Utworzono maszynę |
| `audit-action-machine-renamed` | Machine renamed | Zmieniono nazwę maszyny |
| `audit-action-machine-archived` | Machine archived | Zarchiwizowano maszynę |
| `audit-action-unit-added` | Unit added | Dodano jednostkę |
| `audit-action-unit-renamed` | Unit renamed | Zmieniono nazwę jednostki |
| `audit-action-unit-archived` | Unit archived | Zarchiwizowano jednostkę |
| `audit-action-udt-defined` | Struct defined | Zdefiniowano strukturę |
| `audit-action-derived-type-created` | Derived type created | Utworzono typ pochodny |
| `audit-action-recipe-created` | Recipe created | Utworzono recepturę |
| `audit-action-recipe-renamed` | Recipe renamed | Zmieniono nazwę receptury |
| `audit-action-recipe-deleted` | Recipe deleted | Usunięto recepturę |
| `audit-action-recipe-restored` | Recipe restored | Przywrócono recepturę |
| `audit-action-version-created` | Version created | Utworzono wersję |
| `audit-action-draft-discarded` | Draft discarded | Odrzucono szkic |
| `audit-action-version-approved` | Version approved | Zatwierdzono wersję |
| `audit-action-version-retired` | Version retired | Wycofano wersję |
| `audit-action-parameter-added` | Parameter added | Dodano parametr |
| `audit-action-parameter-definition-updated` | Parameter definition changed | Zmieniono definicję parametru |
| `audit-action-parameter-value-set` | Value set | Ustawiono wartość |
| `audit-action-limits-replaced` | Limits replaced | Zastąpiono limity |
| `audit-action-translation-upserted` | Translation saved | Zapisano tłumaczenie |
| `audit-action-translation-removed` | Translation deleted | Usunięto tłumaczenie |
| `audit-action-parameter-removed` | Parameter deleted | Usunięto parametr |
| `audit-action-type-revision-rebound` | Type revision rebound | Przepięto rewizję typu |
| `audit-action-user-logged-in` | User logged in | Zalogowano użytkownika |
| `audit-action-user-logged-out` | User logged out | Wylogowano użytkownika |
| `audit-action-user-created` | User created | Utworzono użytkownika |
| `audit-action-user-role-set` | Role set | Ustawiono rolę |
| `audit-action-user-deactivated` | User deactivated | Dezaktywowano użytkownika |
| `audit-action-user-activated` | User activated | Aktywowano użytkownika |
| `audit-action-password-reset` | Password reset | Zresetowano hasło |
| `audit-action-own-password-changed` | Own password changed | Zmieniono własne hasło |
| `audit-action-login-unlocked` | Login unlocked | Odblokowano logowanie |
| `audit-action-version-exported` | Version exported | Wyeksportowano wersję |
| `audit-action-setting-changed` | Setting changed | Zmieniono ustawienie |
| `security-title` | Security events | Zdarzenia bezpieczeństwa |
| `security-caption` | Security events | Zdarzenia bezpieczeństwa |
| `security-empty` | No event matches the filters. | Żadne zdarzenie nie pasuje do filtrów. |
| `security-filter-kind` | Kind | Rodzaj |
| `security-filter-any-kind` | Every kind | Każdy rodzaj |
| `security-filter-login` | Login | Login |
| `security-filter-client-ip` | Client address | Adres klienta |
| `security-col-count` | Count | Liczba |
| `security-col-agent` | User agent | Przeglądarka |
| `security-col-request` | Request id | Id żądania |
| `security-col-detail` | Detail | Szczegóły |
| `security-login-unknown` | unknown login | nieznany login |
| `security-kind-login-failed` | Failed login | Nieudane logowanie |
| `security-kind-login-rejected-inactive` | Login of an inactive user | Logowanie nieaktywnego użytkownika |
| `security-kind-login-throttled` | Login delayed | Opóźnione logowanie |
| `security-kind-csrf-rejected` | Request without a valid token | Żądanie bez ważnego tokenu |
| `security-kind-forbidden` | Access refused | Odmowa dostępu |
| `security-kind-unlock` | Login unlocked | Odblokowanie logowania |
| `security-kind-audit-chain-broken` | Audit chain broken | Przerwany łańcuch audytu |
| `error-E-EXP-004` | The server cannot create the export document. Contact the administrator. Reference: { $request_id }. | Serwer nie może utworzyć dokumentu eksportu. Skontaktuj się z administratorem. Referencja: { $request_id }. |
| `missing-audit-entry` | Audit entry { $id } | Wpis audytu { $id } |
| `missing-malformed-audit-entry` | Audit entry | Wpis audytu |

The keys `missing-audit-entry` and `missing-malformed-audit-entry` name the entity of `E-WEB-005` for `Missing::AuditEntry` and `Missing::MalformedAuditEntry`. `GetAuditEntry` returns `AppError::NotFound` for an entry that does not exist and for an entry outside the scope of the user.

The English text of `task-stage-diff` equals the `Display` text of `TaskStage::Diff` (`error/value_tests.rs`).

### D9. Codes of the new routes

`api.md` sections 2.4, 2.5, and 2.9 give the codes. The routes also return the common codes of `api.md` section 2. Each code has a message in both bundles (D8). The HTMX error answers follow `api.md` section 5.3 and ADR-0047 (b).

| Route | Codes |
|---|---|
| `POST /recipes/{ref}/delete` | `404 E-WEB-005`, `409 E-RCP-010`, `409 E-RCP-026`, `409 E-RCP-012` |
| `POST /recipes/{ref}/restore` | `404 E-WEB-005`, `409 E-RCP-025`, `409 E-PLT-010` |
| `GET .../approve` | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `422 E-WEB-008` (`kind`, `page`, `rv`), `503 E-WEB-009`, `500 E-WEB-011` |
| `POST .../approve` | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `409 E-RCP-015`, `409 E-WEB-001`, `422 E-RCP-008`, `422 E-RCP-016`, `422 E-RCP-020`, `422 E-RCP-022`, `422 E-RCP-023`, `422 E-RCP-024`, `500 E-EXP-004` |
| `POST .../retire` | `404 E-WEB-005`, `409 E-RCP-009`, `409 E-RCP-012`, `409 E-WEB-001` |
| `POST .../discard` | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `409 E-WEB-001` |
| `GET .../diff/{m}` | `404 E-WEB-005`, `409 E-RCP-012`, `422 E-WEB-008`, `503 E-WEB-009`, `500 E-WEB-011` |
| `GET /audit`, `/audit/{id}`, `/security-events`, `/recipes/{ref}/history` | `403 E-WEB-002`, `404 E-WEB-005`, `409 E-RCP-012` (history), `422 E-WEB-008` |

### D10. Test plan of the required states

| State | Test |
|---|---|
| Every route x six principals (no session, Operator, ProcessEngineer, Administrator, ProcessEngineer with `must_change_password`, inactive user) | W3, W2, W4: the status of each pair in `routes/tests.rs`, in the wave of the route. H2: one `forbidden` event for each `403` |
| `RecipeDelete` conditions | W3: PE with Approved gives `E-RCP-010`, PE with only Retired gives `E-RCP-026`, Admin with only Retired gives `303`, Admin with Approved gives `E-RCP-010` |
| Scope of A20 | A1 and D1: PE gets only recipe entries, also the entries of a deleted recipe. A filter outside the scope gives an empty list, and an entry outside the scope gives `NotFound`. W3: an Operator gets `403` on `/audit`. A PE sees no `chain_seq`, `client_ip`, or `session_ref` on the entry page |
| Approval success | W2: one `VersionApproved` entry with `content_sha256`, the toast with both numbers, `aria-describedby="toast"` on the `h1`, `HX-Push-Url` |
| AlreadyApproved | W2: a second `POST` with the same `row_version` gives `200`, the toast `version-already-approved`, and no second entry |
| `E-RCP-015` | X1: `breaks_separation_of_duties` and `Draft::check_approver`. W2: the creator and an editor of the draft, on the preview together with a line of `E-RCP-016`, and on the `POST` |
| Lists of `E-RCP-016` and `E-RCP-020` | W2: one line per entry on the preview and in the `POST` answer |
| `E-RCP-024` and `allow_outdated` | W2: without the flag `422 E-RCP-024`, with the error line under the checkbox and a summary link to it. With the flag, the audit entry holds `allow_outdated: true` |
| Two sessions | W2: the second approval with the same `row_version` gives AlreadyApproved. A draft edit between the preview and the `POST` gives `409 E-WEB-001` with `conflict.html` |
| Retire, discard, delete, restore codes | W2 and W3: one HTMX test per code with `HX-Retarget` and `HX-Push-Url: false` |
| Busy action forms | W2 and W3: each action form has `hx-disabled-elt="find button"` and the indicator of `form-in-progress` |
| Danger macro | W1: the id `danger-notice` shows once, the notice has no role, and the button names it in `aria-describedby` |
| Preview structure | W2: the blockers and the notices stand right after the header, with no `role="alert"`. The first summary item links to the `h2` of the form. Only the first row of a parameter links its tag |
| Diff parts | X1: added, only old, changed definition, value in a struct in a struct. Also limit `[*]`, limit of an element, mode change, type change, translation, canonical text, IEC literal |
| Diff bounds | X1: `diff.leaf_rows` cut and count. V1: at most `page.diff` keys per page over all sections, the permit taken before the transaction and held in the result, the permit wait gives `E-WEB-009` |
| SQL flags equal the domain diff | D2: the contract cases on memory and PostgreSQL, also a first approval with `old` set to `None`. A PostgreSQL case commits a draft command on a second connection between two reads of the query |
| Paged preview `rv` | W2: a later page with another `row_version` shows the notice and no form |
| History and audit | A1: cursor, newest first, each filter, the day boundary 23:59 and 00:00 UTC with fixed times of the memory clock. D1: the SQL predicate of `from` and `to` with dates relative to the commit time of the seed |
| Filter values | W3: a malformed value gives `422 E-WEB-008` in `#error-panel` with the field label, and the form stays. An unknown `actor` gives the empty list. `to=9999-12-31` sets no upper bound. No state on the history, links only for audit roles |
| Before and after | W3: escaped JSON (the text `&#60;script&#62;`), time leaves as stored counts, no `password_hash`, no `entry_hash`, no `prev_hash`, the XSS corpus (W5) |
| Security events | A1, D1: `login` by user and by HMAC under the current key. An event under an older key does not match. No `login_hmac` in the result or the page |
| Texts | W1: the loader reads the second resource file of each language. H1: each key of D8 in `pl` and `en`. Each code of D9 has a message |
| Every new page in `pl` and `en`, CSP, no inline script or style or handler, literal `hx-*`, no token in a URL | W5: the sweep |
| Structure of every new page | W5: one `h1`, no skipped heading level, a `caption` and `th scope` in each table, a label for each input. Every id that an `aria-*` attribute names exists. No `required`, `pattern`, or `placeholder` |
| Read-only mode | W5: `503 E-WEB-006` for the five commands, the queries stay available |
| CSRF | W5: a missing token gives `403 E-WEB-007` on each of the five `POST` routes |
| Full scenario | D3: create, edit as engineer, approve as admin, version 2 based on 1, approve (version 1 retired), retire version 2. One transaction per command (one `request_id` and one `occurred_at` per command), `chain_seq` without gaps, `content_sha256` set, the hash chain recomputes |

Checks that only a browser can do. The QA agent of step 7 runs them in `pl`. Step P4.2 turns each row into a Playwright test (ADR-0048 (f)).

| Id | Page and state | Widths (CSS px) | Expected number |
|---|---|---|---|
| B1 | Approval preview of `R-100` v2 with the limits section | 320, 481, 560, 640, 768, 1280 | `scrollWidth` equals `clientWidth` of the page at each width |
| B2 | Focus on the approve button after Tab from the checkbox | 320, 768, 1280 | 15 of 15 sample points of the focused button visible |
| B3 | Retire disclosure by keyboard: Tab to `summary`, Enter, Tab, Enter | 320, 1280 | 1 `POST` request, then the focus on the `h1` of the new content |
| B4 | Double click on the approve button | 1280 | 1 `POST` request in the network log, 1 `VersionApproved` entry |
| B5 | Audit entry with a long state | 320 | `scrollWidth` equals `clientWidth` of the page and of the `pre` element |
| B6 | Diff page with an array of 1 000 changed elements | 320, 1280 | 0 sticky elements, 1 000 rows in the section, page width as B1 |
| B7 | Audit filter by keyboard: select an action in an `optgroup`, Tab to the submit button, Enter | 320 | the URL holds the action and no `before` |
| B8 | Cursor pager of the audit list | 320 | each link at least 44 x 44 px, each gap at least 8 px |
| B9 | Navigation of an Administrator with the link "Audit" | 320 | `scrollWidth` equals `clientWidth` of the page |
| B10 | Restore button in a row of `show_deleted` | 320 | target at least 44 x 44 px |
| B11 | Approval of `R-500` v2 by keyboard: Tab to the approve button, Enter | 320, 1280 | 1 `POST` request, the focus on the `h1`, the URL `/recipes/R-500/versions/2`, a toast with the numbers 2 and 1 |
| B12 | Version page after the approval toast, focus on the `h1` | 320 | 0 px overlap of the focused `h1` and the toast, with the `scroll-padding` of the page |
| B13 | Limits frame of the preview by keyboard: Tab into the frame, ArrowRight | 320 | `scrollLeft` of the frame greater than 0 |
| B14 | Diff page of `R-500` with forced colors on | 1280 | a visible border of at least 1 px on the limits frame and on each state badge |
| B15 | Security event page with a long `detail` | 320 | the detail column at least 120 CSS px wide, and `scrollWidth` equals `clientWidth` of the page |

### D11. Known changes of test expectations

The implementer reports each change with this reason. Any other change needs its own reason.

| Test | Old expectation | New expectation | Reason |
|---|---|---|---|
| Navigation tests of the top bar | four links | five links for Administrator and ProcessEngineer | ADR-0051 (j) |
| `routes::tests` (`expected_routes`, the matrix, the area lists, the coverage test) | the routes of P3.3 | also the eleven routes of D9. The agent that adds a route changes the file in its wave | ADR-0049, ADR-0050, ADR-0051 |
| Tests with a literal of `PageSizes` | five fields | nine fields | ADR-0050 (m), ADR-0051 (k) |
| Tests with a literal of `AppSettings` (`blocking/tests.rs`) | no diff settings | `DiffSettings` with `leaf_rows` and `permits` | ADR-0050 (g) |
| Test doubles of `Transaction` (`recipe/queries/count_tests.rs`, `export/tests.rs`, `plant/short_rows.rs`, `memory/core/transaction.rs`) | the accessors of P3.3 | also `audit_reads` and `diff_reads` | ADR-0050 (f), ADR-0051 (a) |
| Tests of the output of `ApproveVersion` | `ApprovalOutcome` with ids | `ApproveVersionOutput` with `outcome` and `retired`. The RED step is the compile error on the new field | ADR-0049 (e) |
| Tests of the stage words of `TaskStage` (`error/value_tests.rs`) | four stages | five stages with `Diff` | ADR-0050 (g) |
| Tests of the bundle sources (`i18n.rs`) | one resource file per language | two resource files per language | D2 |
| Tests that count the forms of the version page or the recipe page | no action form | the action forms of ADR-0049 for the allowed roles | ADR-0049 (i) |
| The sweep and XSS page lists | the pages of P3.3 | also the new `GET` pages | D7 |
| Tests that build `MemorySink::default()` next to a store | a sink of their own | the sink of the store. `events()` stays a flat list | ADR-0051 (a) |
| `diff/value_tests.rs` `change_of_the_limit_mode` | `[1]` to `[8]`, then `[*]` | `[*]` first, then `[1]` to `[8]` | ADR-0050 (d), ADR-0039 (k) |
| `diff/value_tests.rs` `a_type_change_compares_the_leaves_by_leaf_path` | only the leaves with a changed text | every leaf of both sides | ADR-0050 (c) |
| `diff/tests.rs` `speed_row` | the type label `INT` | the type id of `INT` | ADR-0050 (c), full type text in `recipe-app` |
| The endpoint tables of the recipe security and XSS tests | each command route has a body and a field case | the delete route and the restore route send only the CSRF token. They need no body and no field case | ADR-0049 (g) and (h), these two forms hold no `row_version` and no text field |
| `pages::recipe_sweep_tests` (`TOKEN_ONLY_ROUTES`, `statuses_of`) | the delete route and the restore route | also the approve, retire, and discard routes. `statuses_of` also reads the want `Status` | ADR-0049 (b), (f). These forms hold no text field, so the XSS table has no case for them |
| `approval::preview_tests` (limit row and first-row link) | one row per bound with "Minimum" and the cells `0` and `10`, the tag `ADDED` at least twice in the limits section | one row per limit row with "0 to 3000" and "10 to 3000", the tag `ADDED` once | D4, one table row for each row of `diff_pairs` (user decision on the review of 10.1) |
| `pages::recipe_sweep_tests` (stale answers, `stale_statuses`) | `409` for each route | `409`, except a route whose matrix row wants a status. That route keeps its status | The approve route answers `422 E-RCP-016` before its row check (`ApproveVersion` step order) |
| `routes::recipe_access_tests` (`REFUSED_ROUTES`) | 14 routes | 19 routes with the delete, restore, approve, retire, and discard routes | ADR-0049 (b), (f) to (h). The Operator has none of the permissions of these five routes |
| The status matrix of `POST .../approve` (`versions/route_tests.rs` `approve_row`) | none | the ProcessEngineer who edited the draft gets `422 E-RCP-016` before `409 E-RCP-015` | `ApproveVersion` runs `check_for_approval` before `Recipe::approve`, which checks the approver. `domain-model.md` §5 gives no order |
| `approval::approve_tests` (list of missing limits) | every line of the list | the first 20 lines and the line of the other errors | ADR-0041 (d) |
| The fallback tests of the preview (`approval_preview_parity_tests.rs`) | an empty diff with the blockers | no diff (`None`) and no outdated parameters with the blockers | D3, no diff with a blocker |
| `pages::audit::list_tests` (the text of an empty aggregate page) | `Aggregate:` with two spaces before the short id | one space, from the key `audit-filter-aggregate-id` | D8 |
| The diff page tests with `partial` | the field `partial` | the field `cut`, which also names a section with no entry on the page | D4 |
| `approval::preview_state_tests` (draft blocker of `R-100` version 3) | no summary and no section | the blockers before the summary, the diff, and no link to the form | D3. A review that does not fail gives a diff, also next to a blocker |
| `approval::preview_tests` (the form of the preview) | the locator `<form method="post" action=` | the locator `<form id="approve-fields" method="post" action=` | ADR-0041 (e). The answer of `E-RCP-024` swaps the form out of band by its id |
| `approval::preview_page_tests` (cut sections) | the first link with `kind` | the link with `kind`, `rv`, and the text of `diff-pager-label` | D3, design-system 10.13. The first link with `kind` is the summary link of the section |

### D12. Waves

Classes: T1-O `opus` max, T1-C `opus` high, T2 `opus` xhigh, T3 `sonnet` high, T4 `haiku` medium. No agent uses `fable`.

| Wave | Agent | Class | Model | Effort | Tasks | File set |
|---|---|---|---|---|---|---|
| 1, step 1 | W1 | T3 | sonnet | high | 2.1 | `recipe-web`: `src/i18n.rs`, `i18n/en-approval.ftl`, `i18n/pl-approval.ftl` (new), `src/i18n/resource_tests.rs` (new) |
| 1, step 2 | H1 | T4 | haiku | medium | 2.2 `[mechanical]` | `recipe-web`: `i18n/en-approval.ftl`, `i18n/pl-approval.ftl`, `src/i18n/approval_texts_tests.rs`, the `mod` line in `src/i18n.rs` |
| 1, step 3 | A1 | T1-C | opus | high | 4.1 to 4.4 `[hard]` | `recipe-app`: `src/ports/audit_reads.rs`, `ports/mod.rs`, `ports/unit_of_work.rs`, `src/memory/audit/`, `memory/mod.rs`, `memory/core/store.rs`, `memory/core/transaction.rs`, `memory/sink.rs` and its tests, `src/audit/queries/`, `src/audit/mod.rs`, `src/blocking.rs` and the literals of `PageSizes` in tests, `src/lib.rs`, `src/auth/first_statement.rs`. The test doubles of `Transaction`: `src/recipe/queries/count_tests.rs`, `src/export/tests.rs`, `src/plant/short_rows.rs`. The sink sites: `src/memory/fixtures/seed.rs`, `src/identity/fixtures.rs`, `src/auth/matrix.rs`. `recipe-web`: the sink sites `src/test_support.rs`, `tests/web/support.rs`, `examples/dev_server.rs`. `recipe-db`: the accessor stub `audit_reads` in `src/transaction.rs` only |
| 1, step 3b | A1b | T1-C | opus | high | 4.2, 4.3 `[hard]` (A1 stopped at its context limit) | the `recipe-app` files of A1 for 4.2 and 4.3, also `src/error/missing.rs` (`Missing::AuditEntry`, `Missing::MalformedAuditEntry`), `src/recipe/queries/mod.rs` and `src/recipe/mod.rs` (`pub(crate)` on `read_recipe` and `require_visible` only). `recipe-web`: `src/error/values.rs`, `src/error/value_tests.rs`, the two `missing-*` keys at the end of `i18n/en-approval.ftl` and `pl-approval.ftl` |
| 1, step 3 | W1 | T3 | sonnet | high | 9.1 to 9.4 | `recipe-web`: `templates/test/` (the test templates of the new macros), `templates/components/danger.html`, `pager.html`, `icons.html`, `static/app.css`, `static/components.css`, `src/style_tests.rs`, `src/style_tests/approval_rules.rs`, `src/components_tests.rs`, `src/components_tests/approval.rs`. The navigation in `src/layout.rs` and `src/layout/tests.rs`, the navigation expectations in `src/error/tests.rs` and `src/i18n/tests.rs` (D11, no new line), `src/settings.rs`, `src/test_support/recipes.rs`, `src/test_support/recipes/` |
| 2 | D1 | T1-C | opus | high | 5.1, 5.2 `[hard]` | `recipe-db`: `src/transaction.rs`, `src/transaction/audit_reads.rs`, `security_event_reads.rs`, `src/unit_of_work.rs`, `tests/storage/contract/mod.rs`, `tests/storage/contract/harness.rs`, `memory_harness.rs`, `postgres_harness.rs`, `tests/storage/contract/audit_reads/`, `tests/grants.expected` (read only) |
| 2 | X1 | T1-C | opus | high | 3.1 to 3.3 `[hard]` | `recipe-domain`: `src/recipe/diff.rs`, `src/recipe/diff/`, `src/recipe/root/approve.rs` and its tests, `src/recipe/draft/approval.rs` and its tests, `src/recipe/mod.rs`, `src/lib.rs` |
| 3 | V1 | T1-C | opus | high | 6.1 to 6.4 `[hard]` | `recipe-app`: `src/ports/diff_reads.rs`, `ports/mod.rs`, `ports/unit_of_work.rs`, `src/memory/core/transaction.rs`, `src/memory/recipe/diff_reads.rs`, `memory/recipe/mod.rs`, `memory/mod.rs`. The queries `src/recipe/queries/diff.rs`, `approval_preview.rs`, their tests, `recipe/queries/mod.rs`, `queries/source_tests.rs`, `queries/count_tests.rs`. The command `src/recipe/approve_version.rs`, `approve_version/`, `src/recipe/mod.rs`. Also `src/blocking.rs`, `src/blocking/tests.rs`, the literals of `PageSizes` and `AppSettings` in tests (`src/plant/queries/usage_tests.rs`, `machines_tests.rs`, `src/audit/queries/test_world.rs`), `src/context.rs`, `src/error/` (stages), `src/lib.rs`, `src/auth/first_statement.rs`, `src/export/tests.rs`, `src/plant/short_rows.rs`. The callers of `ApproveVersionOutput`: `src/memory/fixtures/scenarios.rs`, `src/audit/coverage.rs` (no new line). `recipe-db`: the accessor stub `diff_reads` in `src/transaction.rs`, `tests/storage/race/approve.rs`, `tests/storage/atomic/recipe.rs`. `recipe-web`: `src/error/values.rs`, `src/error/value_tests.rs`, the callers of the `ApproveVersion` output |
| 3b | V1b | T1-C | opus | high | 6.2 to 6.4 `[hard]` (V1 stopped at its context limit after 6.1) | the files of V1 |
| 3b | W3b | T3 | sonnet | high | 11.4 and the open items of W3 (W3 stopped at its context limit after 11.3) | the files of W3. Also `src/error/view.rs` (`INPUT_FIELDS`), `src/pages/sweep_tests.rs`, `src/pages/recipes/route_tests.rs`, `src/pages/recipe_security_tests.rs`, `src/pages/recipe_sweep_tests.rs`, `src/pages/plant_xss_tests.rs`. `recipe-app`: `src/audit/queries/scope.rs`, `src/audit/queries/day.rs` (new), `src/audit/queries/mod.rs`, `src/audit/mod.rs` |
| 3d | V1d | T1-C | opus | high | the fixes of VW3a (6.2, 6.3) | the files of V1. Also the doc comments of `src/memory/draft/mod.rs` and `src/ports/draft.rs` |
| 3c | W3c | T3 | sonnet | high | the open items of W3b (W3b stopped at its context limit after 11.4) | the files of W3b |
| 3f | V1e | T1-C | opus | high | the doubts of V1d: no diff with a blocker (D3), the `diff.leaf_rows` test of the preview, the shared type trees of `outdated_parameters` (VW3a-10) | `recipe-app`: `src/recipe/queries/approval_preview.rs` and its three test files, `src/recipe/queries/outdated.rs`, `outdated/tests.rs`, `src/recipe/queries/versions.rs` |
| 3h | V1f | T1-C | opus | high | the fixes of VW3c-2 to VW3c-6 | `recipe-app`: `src/blocking.rs`, `src/blocking/tests.rs`, `src/memory/core/transaction.rs` (doc comment only), `src/recipe/queries/diff.rs`, `diff_tests.rs`, `approval_preview.rs` (doc comments only), `approval_preview_tests.rs`, `approval_preview_gate_tests.rs` |
| 3g | W3e | T3 | sonnet | high | the fixes of VW3b-3 to VW3b-6, the removal of the unused `empty_page` | the files of W3b. Also `recipe-web` `src/pages/render.rs` and `src/pages/parameters/fields.rs` (visibility of the reused helpers only), `src/pages/history/tests.rs`, `src/pages/recipes/lifecycle_tests.rs`. `recipe-app`: `src/recipe/queries/diff_page.rs` (the removal only) |
| 3e | W3d | T3 | sonnet | high | the open items of W3c (W3c stopped at its context limit), the names `DayReason` and `DayRangeReason` | the files of W3b |
| 3 | W3 | T3 | sonnet | high | 11.1 to 11.4 | `recipe-web`: `templates/components/danger.html`, `templates/test/`, `src/components_tests/approval.rs`, `src/pages/page_tests.rs`, `src/pages/recipes/lifecycle.rs` and tests, `src/pages/recipes.rs`, `templates/recipe.html`, `recipes.html`, `src/pages/history.rs`, `src/pages/audit.rs`, `src/pages/audit/`, `src/pages/security_events.rs`, templates `history.html`, `audit.html`, `audit-entry.html`, `security-events.html`, `src/pages.rs`, `src/routes.rs`, `src/routes/matrix.rs`, `src/routes/tests.rs`, `src/pages/path_values.rs`, `i18n/en-approval.ftl`, `i18n/pl-approval.ftl` |
| 4 | W2 | T1-C | opus | high | 10.1 to 10.4 `[hard]` | `recipe-web`: `src/pages/approval.rs`, `src/pages/approval/`, `src/pages/diff_view.rs`, `templates/components/diff.html`, `templates/approve.html`, `src/pages/versions.rs`, `src/pages/versions/` (also `actions.rs`, `page.rs`), `templates/version.html`, `src/pages/draft_reply.rs`, `src/pages.rs`, `src/routes.rs`, `src/routes/matrix.rs`, `src/routes/tests.rs`, `i18n/en-approval.ftl`, `i18n/pl-approval.ftl`, `src/i18n/approval_texts_tests.rs`. The test data `src/test_support/recipes.rs` and `src/test_support/recipes/` (new seeds and new helpers only, for example `Fixture::with_app_settings`). `templates/components/heading.html` (a new macro only) |
| 4b | V1g | T1-C | opus | high | `blocker_keys` of the preview (D3), found by W2 | `recipe-app`: `src/recipe/queries/approval_preview.rs` and its three test files |
| 4c | W2b | T1-C | opus | high | 10.1 `[hard]` (W2 stopped at its context limit before the first test, user decision: three agents in a row) | the files of W2 |
| 4d | W2c | T1-C | opus | high | 10.2, 10.4 `[hard]` | the files of W2 |
| 4e | W2d | T1-C | opus | high | 10.3 `[hard]`. Also the three entries that W2c2 needs outside its set | the files of W2, except the files of W2e. Also `recipe-web` `src/pages/recipe_sweep_tests.rs` (the entries of the approve, retire, and discard routes only), `src/routes/recipe_access_tests.rs` (`REFUSED_ROUTES` only) |
| 4 | D2 | T1-C | opus | high | 7.1 `[hard]` | `recipe-db`: `src/transaction.rs`, `src/transaction/diff_reads.rs`, `tests/storage/contract/mod.rs`, `tests/storage/contract/diff_reads/`. `recipe-app`: `src/memory/recipe/diff_reads.rs` only for a fault that the contract test shows |
| 4g | W2b2 | T1-C | opus | high | the GREEN step of 10.1 `[hard]` (W2b wrote the RED tests and stopped at its context limit) | the files of W2. Also `recipe-web` `templates/components/alert.html` (the new macro `notice_block` only) |
| 4h | W2c2 | T1-C | opus | high | the GREEN step of 10.2 and 10.4 `[hard]` (W2c wrote the RED tests and stopped at its context limit). Also the form layout of D3 (VW4b1-3, VW4b1-4, VW4b1-6), because the line of `E-RCP-024` goes into that form | the files of W2. Also `recipe-web` `src/error/lists.rs` (the method `FieldError::with_subject` only), `src/error.rs` (the export of `line_view` only) |
| 4i | W2e | T1-C | opus | high | the fixes of VW4b1-1, VW4b1-2, VW4b1-5, VW4b1-7, VW4b1-9, and VW4b1-11 after W2c2. Also the preview test with `diff` set to `None` and the removal of the workaround in `blockers.rs`. User decision: the row model of D4 | `recipe-web`: `src/pages/approval/preview.rs`, `blockers.rs`, the 10.1 test files, `src/pages/diff_view.rs`, `templates/components/diff.html`, `templates/approve.html`, `i18n/en-approval.ftl`, `i18n/pl-approval.ftl`, `src/i18n/approval_texts_tests.rs`, `static/components.css` (a gap rule only). `src/error.rs`: only the removal of the temporary `expect` attribute of `line_view`. It runs next to W2d, which changes no file of this set |
| 4f | D2b | T1-C | opus | high | the fixes of VW4a-1 to VW4a-6 and VW4a-9 (user decision: all three groups) | the files of D2. Also `recipe-db` `src/transaction/parameter_rows.rs` (the visibility of `group` and an accessor `parameter_id` only), `tests/storage/contract/harness.rs`, `memory_harness.rs`, `postgres_harness.rs` (new helpers only). `recipe-app`: `src/ports/diff_reads.rs` (doc comment only) |
| 5 | W4a | T3 | sonnet | high | 12.1 | `recipe-web`: `src/pages/diff.rs` and tests, `templates/diff.html`, `src/pages.rs`, `src/routes.rs`, `src/routes/matrix.rs`, `src/routes/tests.rs`, `src/pages/versions.rs` (the route row only), `src/pages/versions/route_tests.rs`. Also `src/pages/diff_view.rs` and `templates/components/diff.html` for the empty state of `diff-empty` only (VW4b1-17) |
| 5 | W4b | T3 | sonnet | high | 12.2, after W4a | `recipe-web`: `templates/version.html` (the compare links), `src/pages/versions/page.rs`, a new test file in `src/pages/versions/` with its `mod` line in `src/pages/versions.rs` |
| 5 | D3 | T1-C | opus | high | 8.1 `[hard]`, next to W4a | `recipe-db`: `tests/storage/scenario/`, the `mod` line of the storage test root, `tests/storage/support/` (new helpers only) |
| 6 | W5a | T3 | sonnet | high | 13.1 | `recipe-web`: `src/pages/approval_xss_tests.rs`, its `mod` line in `src/pages.rs`, `tests/web/xss-corpus.txt` (read only) |
| 6 | W5b | T3 | sonnet | high | 13.2, after W5a | `recipe-web`: `src/pages/approval_sweep_tests.rs`, its `mod` line in `src/pages.rs` |
| 6 | W5c | T3 | sonnet | high | 13.3, after W5b | `recipe-web`: `src/pages/approval_security_tests.rs`, its `mod` line in `src/pages.rs` |
| 6 | H2 | T4 | haiku | medium | 14.1 `[mechanical]`, next to W5a | `recipe-web`: `src/routes/approval_access_tests.rs`, one `mod` line in `src/routes.rs` |

Rules of the waves:

- At most two agents that run cargo work at the same time. A shared file has one owner in each wave. The owner is the agent whose set names the file. Another agent reports the entry that it needs.
- Wave 1 runs in three steps: W1 task 2.1, then H1 task 2.2, then A1 and W1 tasks 9.1 to 9.4. Each step is a wave of its own for the owner rule.
- A wave starts after the green CI of the wave before it. After a wave with `[hard]` tasks, it also waits for a verifier report (T2, `opus` xhigh, read only, no cargo) without CRITICAL and HIGH.
- The `recipe-db` accessor stubs of A1 and V1 return an error of the storage layer. The workspace therefore compiles until D1 and D2 write the SQL.
- The agent that adds a route (W3, W2, W4) adds its rows to `src/routes/tests.rs` in its wave. H2 adds only the checks of the `forbidden` events.
- W3 does not edit `version.html`. W2 owns it in wave 4, and W4 in wave 5.
- W3 adds the macro of the delete form to `components/danger.html`. The macro has no `row_version`. It has the literal `hx-*` set of D7, the indicator, and the notice `danger-notice`.
- Task 11.1 maps the existing codes `E-RCP-010`, `E-RCP-026`, and `E-RCP-012`. It changes no permission and no shape of `WebError`, so it is not `[hard]`.
- A T3 task that fails twice goes to T1-C with both reports. A red CI that one fix does not make green goes to a T1-O diagnosis (read only).

### D13. Test data

`test_support/recipes.rs` (W1) adds:

- `R-500` on `M7`: version 1 Approved, created by `anowak` and approved by `admin`. Draft version 2 is based on 1 and edited by `anowak`. It has a changed limit `[*]`, a changed value in a struct in a struct, one added parameter, and one parameter only in version 1.
- `R-600` on `M7`: version 1 Retired, no Approved version.
- `R-700`: deleted, on the archived machine `M12`.
- A second ProcessEngineer `jsmith` who edited nothing.

The QA seed of step 7 extends `target/tools/seed_recipes.py` in the scratchpad with the same flow over HTTP.

## Risks / Trade-offs

- [The preview reads the whole draft for the checks] → as `ApproveVersion` does. The permit of `diff.permits` bounds the parallel reads. The load test of step P4.2 measures it.
- [A filter without an index reads many rows] → `db.statement_timeout` gives `503 E-DB-002`. The load test of step P4.2 measures the filters without an index, the scope of a ProcessEngineer, and the HMAC filter.
- [The SQL flags and the domain diff disagree] → the contract test of D2.
- [Six waves with shared files] → one owner per wave (D12).

## Migration Plan

No migration, no grant, no index. The settings of `reliability.md` section 7.11 get defaults in `recipe-app` and `recipe-web`.

## Open Questions

None. Each later doubt goes to the orchestrator, who records it here.
