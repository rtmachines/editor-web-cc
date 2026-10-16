# Tasks

The table "Waves" of design D12 gives the agent, the class, the model, the effort, and the file set of each task. Each code task starts with a failing test. Task 8.1 is a test task only.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0049, ADR-0050, ADR-0051, the index rows, and the decision map. Add the "Amended by" notes of ADR-0036, ADR-0037, and ADR-0042. Check: `ste-lint.py` on the scope of ADR-0023 reports 0 hard violations.
- [x] 1.2 (docs/architecture) Put the decisions into `api.md`, `reliability.md`, `design-system.md`, and `domain-model.md`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.3 (.claude/rules) Add the rules of ADR-0049 and ADR-0050 to `web.md` and `sql.md`. Check: `ste-lint.py` on the two files reports 0 hard violations.
- [ ] 1.4 (CHANGELOG.md) Add the features of this change under "Added" and "Changed". Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 2. Texts

- [x] 2.1 (recipe-web) Write a failing test in `src/i18n/resource_tests.rs` that reads one key of `i18n/en-approval.ftl` and one key of `i18n/pl-approval.ftl` from the bundles. Then load the second resource file of each language in `src/i18n.rs`.
- [x] 2.2 (recipe-web) [mechanical] Write a failing test in `src/i18n/approval_texts_tests.rs` that reads each key of design D8 in `en` and `pl`. Then copy the table D8 into `i18n/en-approval.ftl` and `i18n/pl-approval.ftl`.

## 3. Domain diff and the rule of the approver

- [x] 3.1 (recipe-domain) [hard] Write failing tests of `breaks_separation_of_duties` and `Draft::check_approver` for the creator, an editor, another user, and the policy off. Then add both. `Recipe::approve` calls the function with the data of the token, and its tests stay green.
- [x] 3.2 (recipe-domain) [hard] Write failing tests of `diff_pairs` for an added parameter and a parameter only in the old version. Add tests for a changed definition field and a changed translation. Then add the module `recipe/diff` (ADR-0050).
- [x] 3.3 (recipe-domain) [hard] Write failing tests for a value in a struct in a struct, a `[*]` limit, and an element limit. Add a test for a change of the limit mode. Add tests for a type change, the IEC literal, and the cut at `leaf_rows`. Then write the rows.

## 4. Audit read port and queries

- [x] 4.1 (recipe-app) [hard] Write failing tests of the in-memory read model: ids in commit order and commit times. Add tests of the security events with the id and the clock time of the sink of the store. Add tests of the aggregation per minute in the read model, while `events()` stays a flat list. Then add the port `AuditReads`, the adapter, and the sink sites of design D12.
- [x] 4.2 (recipe-app) [hard] Write failing tests of `ListAuditEntries` and `GetAuditEntry`. Cover the permission check, the cursor, the order, each filter, and the scope of A20 with the entries of a deleted recipe. Cover the day boundary 23:59 and 00:00 UTC and an `actor` that names no user. Then write both queries.
- [x] 4.3 (recipe-app) [hard] Write failing tests of `GetRecipeHistory` without states and on a deleted recipe. Write failing tests of `ListSecurityEvents` with `login` by user and by the HMAC under the current key. An event under an older key does not match. Then write both queries.
- [x] 4.4 (recipe-app) Write failing tests of the defaults `page.audit`, `page.history`, and `page.security_events`. Then add the fields of `PageSizes`.

## 5. Audit SQL

- [x] 5.1 (recipe-db) [hard] Write failing contract cases of the audit list on both adapters. Cover the cursor, every filter, and the scope of A20. Cover the labels and the type word with the short id of a row that no longer exists. Then write the SQL with `QueryBuilder`.
- [x] 5.2 (recipe-db) [hard] Write failing contract cases of one entry, the history, and the security events with the HMAC filter. Add a PostgreSQL case of `from` and `to` with dates relative to the commit time of the seed. Then write the SQL. The grant test stays green.

## 6. Diff and preview queries

- [x] 6.1 (recipe-app) [hard] Write failing tests of the port `DiffReads` on the memory adapter. Cover the change flags per key and part, a first approval with `old` set to `None`, the counts, and the pairs of each side. Then add the port and the adapter.
- [x] 6.2 (recipe-app) [hard] Write failing tests of `DiffVersions`. Cover the permission check, `RepeatableRead`, the sections, and at most `page.diff` keys per page over all sections. Cover the permit before the transaction, the permit in the result, and the timeout. Then write the query and the semaphore of `AppContext`.
- [x] 6.3 (recipe-app) [hard] Write failing tests of `GetApprovalPreview`. It writes nothing, and its blockers equal the errors of `ApproveVersion`. Cover `E-RCP-015` together with `E-RCP-016`, the outdated list, the `row_version` of the snapshot, and the retired number. Then write the query.
- [x] 6.4 (recipe-app) [hard] Write a failing test that `ApproveVersion` returns `ApproveVersionOutput` with the number of the retired version. The RED step is the compile error on the new field. Then change the output and its callers of design D12.

## 7. Diff SQL

- [ ] 7.1 (recipe-db) [hard] Write failing contract cases of `DiffReads` on both adapters. Each case compares the flags with the rows of `diff_pairs`. Add a first approval and a PostgreSQL case with a draft command between two reads. Then write the SQL of the flags, the counts, and the pairs.

## 8. Full scenario

- [ ] 8.1 (recipe-db) [hard] Write a scenario test through the use cases with PostgreSQL: create, edit, approve, version 2, approve, retire. The code exists after wave 4, so a failure of this test shows a fault of an earlier task. Assert one transaction per command, `content_sha256`, and the hash chain.

## 9. Web foundation

- [x] 9.1 (recipe-web) Write failing component tests of the `danger` macro with `row_version` and the notice id `danger-notice`. Add tests of the cursor pager and the four diff icons. Then add the macros and the icons.
- [x] 9.2 (recipe-web) Write failing style tests of the diff tables, the limits frame, and the `pre` element (design-system 10.13, 10.14). Then add the rules.
- [x] 9.3 (recipe-web) Write failing tests of the navigation link "Audit" for each role and of the settings of the new pages. Then add both.
- [x] 9.4 (recipe-web) Write failing tests of the test data of design D13. Then extend `test_support/recipes.rs`.

## 10. Approval and version actions

- [ ] 10.1 (recipe-web) [hard] Write failing tests of the approval preview: sections, blockers, the outdated checkbox, `rv`, and the codes of D9. Cover the place of the blockers, the notice without `role="alert"`, the link to the form, and the tag link of the first row. Then write the page and the shared view of the diff.
- [ ] 10.2 (recipe-web) [hard] Write failing tests of the approve `POST`: success, `HX-Push-Url`, `aria-describedby="toast"`, and AlreadyApproved. Add tests of `E-RCP-015`, the lists, `E-RCP-024` under the checkbox, and `allow_outdated`. Then write the route and its rows in `routes/tests.rs`.
- [ ] 10.3 (recipe-web) [hard] Write failing tests of the action area of the version page. Add tests of the retire and discard forms with their codes and `hx-disabled-elt`. Then write them.
- [ ] 10.4 (recipe-web) [hard] Write failing tests with two sessions. The same `row_version` gives AlreadyApproved. An edit in between gives `409 E-WEB-001`. Then close the gaps.

## 11. Recipe lifecycle, history, and audit pages

- [x] 11.1 (recipe-web) Write failing tests of the delete form, the restore form in the list, `hx-disabled-elt`, and each of their codes. Then write both routes and their rows in `routes/tests.rs`.
- [x] 11.2 (recipe-web) Write failing tests of the history page in three roles and of a deleted recipe. Then write the page.
- [x] 11.3 (recipe-web) Write failing tests of the audit list: filter values, `422 E-WEB-008` in `#error-panel` with the field label, and the boosted filter form. Cover the scope of the selects, the cursor pager, an unknown `actor`, and `to=9999-12-31`. Cover the label of the filter `aggregate` and the short id of an empty page. Then write the page.
- [x] 11.4 (recipe-web) Write failing tests of the audit entry page and the security event page. Cover `chain_seq`, `client_ip`, and `session_ref` only for `AuditView`, and time leaves as stored counts. Then write both pages.

## 12. Diff page

- [ ] 12.1 (recipe-web) Write failing tests of the diff page: sections, `kind`, `page`, the empty state, and a deleted recipe. Cover at most `page.diff` parameters over all sections. Then write the page and its rows in `routes/tests.rs`.
- [ ] 12.2 (recipe-web) Write failing tests of the compare links of the version page. Then add the links.

## 13. Security of the new pages

- [ ] 13.1 (recipe-web) Write the XSS tests for `change_note`, the filters, `actor`, and `q`. Add the XSS tests for `before`, `after`, `detail`, and `user_agent`. Each payload stays escaped.
- [ ] 13.2 (recipe-web) Write the sweep of every new page in `pl` and `en`. Check the CSP, the literal `hx-*`, and no token in a URL. Check that no page has an inline script, an inline style, or a handler. Check one `h1`, no skipped heading level, and a `caption` and `th scope` in each table. Check a label for each input, an existing id for each `aria-*` reference, and no `required`, `pattern`, or `placeholder`.
- [ ] 13.3 (recipe-web) Write the tests of the read-only mode and of a missing CSRF token for the five `POST` routes.

## 14. Routes and roles

- [ ] 14.1 (recipe-web) [mechanical] Write one test per new route and principal of design D10 that gets `403`. Each test checks one `forbidden` event. The route agents keep the status matrix of `routes/tests.rs`.

## 15. Verification

- [ ] 15.1 (recipe-web) Run the browser checks B1 to B15 of design D10 in `pl`. Record the numbers in the progress file.
- [ ] 15.2 (workspace) Run every command of the section "Local checks" of `CONTRIBUTING.md` and `openspec validate approval-audit-history --strict`. All pass.

## Workflow follow-up

- Archive the change after the code review, the fixes, and the verification.
- Run `openspec validate --specs --strict` on the merged specs.
