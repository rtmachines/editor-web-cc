# Traceability and consistency review — 2026-10-12

Reviewer: independent agent (Opus), read-only. Scope: general-draft.txt, docs/architecture (5), docs/adr (13 + README), docs/plan/implementation-prompts.md. Method: line-by-line comparison plus scripts for table column counts, every N/A/I/R reference, every section reference, sentence length and semicolons.

The coverage claim in overview.md §7 is not correct: 12 rows are wrong or only partly true (R07, R08, R14, R16, R19, R23, R29, R31, R35, R36, R40, R42), and draft lines 102–104 have no row at all.

## Pass A — Traceability (draft → docs)

Type: (1) no mechanism, (2) partial, (3) the docs reinterpret the draft.

| # | file:line | Finding | Fix |
|---|---|---|---|
| A1 | draft:102 | (1) "Use a design pattern only when it improves…" — no R row. | Add R row; CONTRIBUTING rule: each pattern names the problem it solves in its doc comment. |
| A2 | draft:103 | (1) "Use models tailored… Use subagents." — no R row; only the plan has it. | Add R row pointing to the plan, or mark as process requirement. |
| A3 | draft:104 | (1) "Communicate with the chat user in Polish." — no R row. | Add R row pointing to the plan. |
| A4 | draft:104 | (2) "English for documentation, specifications, and code comments" — R42 covers only code comments. | Make R42 cover all three. |
| A5 | draft:11-12 | (2) "Divide the production line into machines" — no production-line concept; "one installation = one line" not written. | Add Assumption A25. |
| A6 | draft:12-13 | (2) "one or more units" — `ArchiveUnit` can archive the last active main unit. | Add error `LastMainUnit` with an E-PLT code. |
| A7 | draft:14 | (3) Draft: "up to five levels, such as `U1.2.4.5.6`" — the example has **5 segments**. Docs: "1 to 6 segments". N2/N3 not defined anywhere. | Decide 5 or 6 segments; write as A-number quoting the draft. |
| A8 | draft:18-19, 21 | (3) Draft: parameter "**must have** … a description … and process limits"; docs make both optional (limits "zero or more", `description_en DEFAULT ''`). R08 claims full coverage. | Require `length(description_en) >= 1`; require at least one limit per numeric/string leaf at approval; or record both as Assumptions. |
| A9 | draft:18 | (3) "a unique identifier" — docs have `id`, `key` (unique per version), `tag` (unique per version); none named as the identifier. | State in R08 that `key` is the identifier and its scope. |
| A10 | draft:24 | (2) IEC types: no `LTIME_OF_DAY`/`LTOD`, no `LDATE`, no derived types (ENUM, subrange, alias). | Add rows or list exclusions in A3. |
| A11 | draft:24-25 | (2) Siemens: `LTOD` (S7-1500) missing; reliability.md:278 uses `TOD`/`DT` names not in the matrix. | Add `LTOD`; one name per type. |
| A12 | draft:28 | (3) Draft requires an address for S7-300; docs **forbid** one on other platforms (non-optimized DBs on S7-1200/1500 use absolute addresses). R14 and I5 quote a one-way CHECK; the DDL is two-way. | Allow optional address on S7_1200/S7_1500 or record the ban as Assumption; correct the CHECK text in R14 and I5. |
| A13 | draft:33 | (3) "alphanumeric reference number" — docs allow `_` and `-` and fold case; examples use `R-100`. | Record as Assumption, or allow only `[A-Z0-9]`. |
| A14 | draft:34-35, 51-52 | (3) Docs add `Discarded` and manual "retire (Administrator)"; source "N11" does not exist. | Record discard and retire-without-replacement as A-numbers. |
| A15 | draft:38-39 | (3) Draft: a UDT change "**creates** a new recipe version"; docs: a UDT change creates only a type revision, the engineer edits by hand. | Record the "explicit draft" reading as Assumption, or let `DefineUdt` with `supersedes` create a Draft for each recipe using the old revision. |
| A16 | draft:39 | (2) "Record all changes" gaps: `audit_log_has_state` rejects "PasswordReset, no state"; `LoginFailed`/`Forbidden` rows lost on rollback; 7 commands without audit action; ArchiveUnit action unnamed; seed migration audit unspecified. | Fix the CHECK; "commit then return error" rule; complete data-model §6. |
| A17 | draft:46-47 | (2) Delete "when their permissions allow" — four documents give different conditions (B31). | One rule. |
| A18 | draft:52-53 | (2) Enforcement defined only for commands; queries and GET pages have a permission in the table but no place that checks it. | Define query use cases that check the permission first; add to the matrix test. |
| A19 | draft:57 | (2) data-model §6 omits 7 of 24 commands; failed-login path contradicts "everything or nothing" (B51). | Complete the table; commit-on-error rule. |
| A20 | draft:59, 62 | (3) Draft: "must not crash", "no code paths that can panic"; docs: overflow-checks panic in release, `main` may use `expect`, panic hook calls abort. | Remove the `expect` exception; record abort-and-restart and overflow-checks as deliberate deviations (Assumptions) for the author to accept. |
| A21 | draft:64 | (3) "anyhow for application errors" — docs use thiserror in `recipe-app`, anyhow only in the binary. | Record "application = binary crate" as Assumption. |
| A22 | draft:67-68 | (2) Errors logged only at the HTTP boundary; background jobs (session cleanup, login_attempt cleanup, disk check, restore test, clock check) have no error rule; domain-model §5 lists only "typical errors". | Each job logs each `Err` once at its loop boundary with a `job` field; complete error lists. |
| A23 | draft:68-69 | (2) ~16 error names have no code (B43); E-EXP-001 lacks {reference}/{number}, E-IDN-002 lacks {login}, E-RCP-014 lacks version. | Code for each variant; identifying values in messages. |
| A24 | draft:70 | (2) Unvalidated inputs: `POST /lang` redirects to `Referer` (open redirect); `login_attempt.login` has no length CHECK; env input for first admin; 3 grammars for LangCode and LeafPath. | Same-origin path only; CHECK; one grammar each. |
| A25 | draft:72 | (2) UI/UX only has interaction rules; design left to plan P3.1. | Mark R39 partial. |
| A26 | draft:72-73 | (2) STE structural rule violations (Pass D). | See Pass D. |
| A27 | draft:77-100 | (2) overview §8 covers 7 of 22 principles; the other 15 have no statement. | One line per principle (applied / not applied: no problem). |

## Pass B — Contradictions between documents

### References
| # | Where | Finding | Fix |
|---|---|---|---|
| B1 | overview:133,136; domain-model:45,59,118,124; data-model:441,461; adr/0003:9; adr/0009:39; api:189 | N1–N5, N7, N11 are cited but no document defines an N-list. | Add "Draft interpretation notes N1–N12" or convert to A-numbers. |
| B2 | overview:216; reliability:80 | A24 and E-WEB-006 point to "reliability.md 7.9"; the disk policy is §7.8. | Change to 7.8. |
| B3 | overview:207 | A15 cites ADR-0001 for axum 0.8 / sqlx 0.8; ADR-0001 names only HTMX 2. | Add versions to ADR-0001 or cite correctly. |
| B4 | adr/0005:46 | Cites A8 for auditing login/failed login/logout; A8 covers export only. | Add an Assumption for auth events. |
| B5 | adr/0004:13 | Cites R24 for revocation; R24 is "operator is read-only". | Cite R28/R21. |
| B6 | api:187 | "R9" vs "R09". | Two digits. |
| B7 | domain-model:19 | Cites ADR-0010 for the module/context split. | Point to overview §4–§5. |
| B8 | api:150 | Cites ADR-0003 for "proxy does not forward health routes"; no such rule there. | Move the rule into ADR-0003 or reliability §7. |
| B9 | api:172 | Claims a last-administrator constraint trigger; data-model §4 has none. | Remove claim or add trigger 4.7. |
| B10 | reliability:137 | Cites §4.3 for the depth bound; it is §4.4. | Fix. |
| B11 | data-model:748 | Says Docker secrets are in reliability §7; only adr/0003:53 mentions them. | Cite ADR-0003. |
| B12 | api:381 | "Argon2id with the parameters in reliability.md" — only "about 250 ms" given. | Add m, t, p values. |
| B13 | domain-model:194; overview:143 | I5/R14 CHECK text is one-way; DDL is two-way `(plc_platform='S7_300') = (s7_address IS NOT NULL)`. | Copy the DDL text (or change the DDL per A12). |
| B14 | domain-model:203 | I14 says no DELETE grant on `parameter`; data-model grants it and RemoveParameter needs it. | Remove `parameter` from I14. |
| B15 | domain-model:202; plan:214 | I13 says "Audit role" (none exists; role is `recipe_app`) and "a use case without audit does not compile" — `Vec<AuditEntry>` can be empty. | Non-empty type; name the role. |
| B16 | data-model:646-671; adr/0010:38 | "parameter unit is in the recipe's machine" has a trigger but no invariant number. | Add I15. |
| B17 | domain-model:193 | I4 says DB mechanism "same as I3", but trigger 4.1 protects only parameter rows; non-draft `recipe_version` rows can be updated. | Transition trigger (see PostgreSQL H1). |

### Tables, columns, grants, grammars
| # | Where | Finding | Fix |
|---|---|---|---|
| B18 | adr/0011:38 | Says DELETE granted only on session; data-model grants DELETE on parameter, parameter_limit, parameter_translation, login_attempt. | Supersede ADR-0011. |
| B19 | adr/0011:40 | Says unique constraints are partial (reference reusable); DDL, R16 and I11 say plain UNIQUE. | Decide; if partial, `{ref}` URLs must handle reuse. |
| B20 | adr/0011:42 | Says an Administrator can restore a recipe; no command, endpoint, permission or §6 row exists. | Add `RestoreRecipe` or remove the sentence. |
| B21 | adr/0006:46 | "The application does not insert primitive rows" — it does for STRING(n)/WSTRING(n). | Add the exception. |
| B22 | adr/0005:66 | "No other indexes" besides 2; DDL adds `audit_log_actor_idx`. | Align. |
| B23 | domain-model:7 vs data-model:7 | Both claim their names are mandatory; names differ (actor/before/after vs actor_user_id/before_state/after_state; key vs parameter_key; supersedes vs supersedes_type_id). | Rust names from domain-model, SQL names from data-model; mapping table. |
| B24 | reliability:259 vs DDL | 4000 vs 2000 characters; unit name no max vs 200. | One number each. |
| B25 | reliability:260; data-model:524; overview:209; adr/0012:61 | LangCode has 3–4 definitions. | One grammar. |
| B26 | domain-model:79; data-model:493; reliability:264 | LeafPath has 3 grammars. | One grammar. |
| B27 | reliability:278 | `TOD`/`DT` vs matrix `TIME_OF_DAY`/`DATE_AND_TIME`. | Matrix names. |
| B28 | domain-model:104 | `row_version: u64` vs BIGINT (i64). | i64 or newtype. |
| B29 | api:71; domain-model:103 vs data-model:810 | Version 1 change_note from user vs fixed "Initial version". | User value. |

### Permissions
| # | Where | Finding | Fix |
|---|---|---|---|
| B31 | overview:152; api:185; adr/0011:36; adr/0013:47; domain-model:253 | Recipe-delete rule differs in every document (ProcessEngineer: no Approved / no Approved and no Retired; Administrator: always / never with Approved). No error code for "blocked by Retired". | Use the ADR text everywhere; add error + code. |
| B32 | plan:215; adr/0013:43; api:171; domain-model:166 | `Permission` enum placed in recipe-app (plan) vs recipe-domain (ADR, api); check location ambiguous. | Enum in recipe-domain, check in recipe-app. |
| B33 | reliability:221 | Audit reader "Administrator" contradicts A20. | Match A20. |
| B34 | adr/0013:51 | `Forbidden` written to audit log, but check runs before any transaction; no write path. | Define the write path or remove. |
| B35 | overview:27 | Export "pull by API" vs adr/0004:58 "machine client cannot use the export until Option C". | "by user". |
| B80 | api:69 | `show_deleted=1` for every role vs adr/0011:41 Administrator only. | Choose. |

### Error codes and HTTP statuses
| # | Where | Finding | Fix |
|---|---|---|---|
| B36 | reliability:59 vs api:81,235 | RecipeDeleted 410 vs 409. | Choose. |
| B37 | reliability:55 vs api:83,235 | DraftHasNoParameters 422 vs 409. | Choose. |
| B38 | reliability:43 vs api:48 | ParentNotFound 422 vs 404. | Choose. |
| B39 | reliability:45-46 vs api:59,237 | TypeCycle, MemberTypeNotOnPlatform 422 vs 409 vs 422. | 422. |
| B40 | reliability:67 vs api:111-112,256; data-model:823 | E-EXP-001 409 vs 404; `VersionNotApproved` in two enums. | One status; rename one variant. |
| B41 | data-model:795 vs reliability:31,68 | Retry: 3 attempts/50·n ms/409 vs 3 retries/50-100-200 ms/`Storage`/503. | One rule. |
| B42 | domain-model:256 vs data-model:822; api:34 | `UserInactive` vs `LoginRejected`; api lists no 403. | One name; add 403. |
| B43 | many | No code for: MachineArchived, MachineHasDraft, UnitArchived, UnitReferencedByDraft, ValidationFailed, LeafPathNotFound, MinGreaterThanMax, LimitNotApplicable, ConcurrencyRetryExhausted, LoginRejected, 429 rate limit, 403 CSRF, 422 password policy, WebError::BadRequest, WebError::Timeout; E-WEB-006 missing from table 1.2; DuplicateTag missing from api.md. | Complete table 1.2. |
| B44 | reliability:18,21 vs 66,70-74,78,30 | Variant ownership inconsistent (ConcurrentEdit/Forbidden/NotFound in WebError table vs AppError list; AuditWriteFailed, DbError::Constraint not in the lists; triggers raise 23514 which maps to CheckViolation). | One owner per variant; mapping order. |
| B45 | api:129,34 vs 254 | Wrong current password 401 → HX-Redirect to /login from the password-change form. | 422. |
| B46 | reliability:78,47 | Trigger codes → 409 but domain twin of E-PLT-020 (E-RCP-013) is 422; E-PLT-010 duplicates PlatformLocked (E-PLT-007) and no command can raise it. | Align; add a platform-change use case or drop E-PLT-007. |
| B47 | api:227-240 | Status table lacks 413, 410, 503 read-only. | Add rows. |

### Session, timeouts, rate limit
| # | Where | Finding | Fix |
|---|---|---|---|
| B48 | adr/0004:40 vs api:355,357 | SameSite Lax vs Strict; 128 vs 256-bit id (tower-sessions uses 128). | Strict, 128 bits. |
| B49 | data-model:343 vs api:366 | Cleanup 10 vs 5 minutes. | Choose. |
| B50 | — | 30 min idle, 12 h absolute, 5/15 min, 12–128 chars: consistent. | — |
| B51 | data-model:822; adr/0004:43 vs data-model:788 | Failed login inserts rows, but `Err` rolls the transaction back → counter never increases. | Commit on InvalidCredentials, then return the error. |

### Use cases, audit actions, behaviour
| # | Where | Finding | Fix |
|---|---|---|---|
| B52 | data-model:804-823 | No rows for RenameMachine, ArchiveMachine, RenameUnit, RenameRecipe, ActivateUser, ChangeOwnPassword, Logout. R29 says complete. | Add rows. |
| B53 | api:100-103 | Routes call "UpsertParameter (limits part)/(translation part)" incl. a translation DELETE; no such commands; data-model replaces all limits/translations in one call. | SetLimits, UpsertTranslation, RemoveTranslation. |
| B54 | data-model:812 vs api:97 | Upsert `ON CONFLICT (recipe_version_id, parameter_key)` but the route identifies by `{tag}` and sends no key; no explicit create route. | Explicit key in form, or upsert by tag. |
| B55 | api:341 vs data-model:823 | `VersionExported` vs `RecipeVersionExported`; ArchiveUnit action unnamed; `RecipeDeleted` is both action and error. | AuditAction list in domain-model §2.4. |
| B56 | domain-model:168; api:127 vs data-model:821 | ResetPassword "ends all sessions" vs "except the current one" (which belongs to the admin). | Remove "except the current one". |
| B57 | domain-model:240 vs api:46 | ArchiveMachine blocked by `MachineHasDraft` vs "has recipes that are not deleted". | Choose. |
| B58 | domain-model:50,243 vs api:51 | ArchiveUnit cascade + blocked only by draft vs blocked by sub-units or any parameter. | Choose. |
| B59 | overview:181 vs data-model:685-687; adr/0008:52; domain-model:279-282; plan:334 | `Ok(AlreadyApproved)` "when row_version matches" — the trigger increments row_version so it never matches; `ApprovalOutcome` has no such variant; plan P3.4 test depends on it. | Rewrite: approved with row_version == expected+1 → AlreadyApproved; or 409. |
| B60 | api:214 vs data-model:690 | App sets `row_version = row_version + 1` vs "application never sets row_version". | Data-model text. |
| B61 | adr/0008:42 vs api:216 vs plan:316 | Conflict fragment content differs (values / row_versions + editor + time / differences). | Choose. |
| B62 | api:215 vs adr/0001:46; plan:65,318 | "only client-side script" vs ≤ 50 lines of tree JS allowed. | List permitted scripts. |
| B63 | adr/0002:40 (+ domain-model:17,299; overview:9, A4; data-model:823) vs api:112,182,274; plan:349 | Export "never serializes a Retired version" vs Retired exported by number. | Choose; supersede ADR-0002 if Retired allowed. |
| B64 | adr/0012:39; adr/0002:43 vs api:294,277-289 | Translations included vs excluded; per-parameter type tree vs shared `types[]`; name_en absent in ADR. | Align ADRs with api.md §6. |
| B65 | api:338 | "Immutable for (ref, n)" but `version.status` changes and (B19) reference may be reused. | Rewrite. |
| B66 | domain-model:269,195 vs overview:69,77,103 | Domain method takes `&TypeCatalog` (an I/O port in recipe-app); recipe-domain cannot depend on it. | Pass resolved `TypeTree` and platform. |
| B67 | reliability:239,243 vs api:13,97,105 | "1 MiB JSON body", "a JSON route" — no JSON request route exists; 64 KiB form limit cannot hold 65 536 leaves. | JSON route or bigger limit for that route. |
| B68 | reliability:301 vs data-model:430-431 | Test "two concurrent approvals of different drafts" impossible (one live draft per recipe). | "Two approvals of the same draft". |
| B69 | api:13,147-150 vs reliability:315,328,330 | Health: text/plain vs JSON; DB only vs DB+disk; proxy does not forward vs proxy healthcheck through proxy; readyz on disk <5 % + proxy 503 stops all queries (contradicts A24). | Align; keep disk out of the proxy routing decision. |
| B70 | overview:216 vs data-model:823 | A24 exports keep working, but export writes an audit row. | Allow audit inserts in read-only mode or block exports. |

### Operations and migrations
| # | Where | Finding | Fix |
|---|---|---|---|
| B71 | adr/0003:52, data-model:875 vs reliability:285; overview:37-42,49 | Who runs migrations (app vs one-shot service); service count 4 vs 5 vs no migrate in C4; restart policy. | Supersede ADR-0003 with the reliability §5/§7 model. |
| B72 | data-model:870-874 vs reliability:282 vs plan:232-233,87 | Migration names, split (3 vs 5), folder. | Choose one. |
| B73 | data-model:874 vs reliability:290, plan:233 | First admin: hash from env in SQL vs password in Rust subcommand; SQL cannot read env. | Seed in Rust. |
| B74 | reliability:289,376 vs 408 | Automatic `_premigrate` dump vs manual `_preupgrade`; no Compose mechanism orders backup before migrate. | One mechanism. |
| B75 | adr/0003:71 vs reliability:378 | Restore tested per release vs weekly. | State both. |
| B76 | reliability:304 vs 389; plan:386 | Soak memory ceiling 256 vs 512 MiB. | Align. |
| B77 | plan:235 vs reliability:30 | SQLSTATE lists differ. | Reliability list. |
| B78 | reliability:369 vs data-model:809-820 | "Only approval uses SERIALIZABLE" vs six commands. | Reference data-model §6 (or PostgreSQL H4). |
| B79 | overview:160 vs adr/0010:53 | "No denormalized write paths" vs `plc_platform` in 3 tables. | "Protected by composite FK". |

## Pass C — Internal consistency (20)

| # | Where | Finding | Fix |
|---|---|---|---|
| C1 | reliability:254,264 | Table rows break: unescaped `\|` inside code spans in regexes. | Escape. |
| C2 | reliability:39-74 | No E-WEB-006 row; trigger codes only as ranges. | Add rows. |
| C3 | api:328 | Example `"s7_address": "DB1.DBD2"` on a Struct parameter; rule says first byte with B. | `DB1.DBB2`. |
| C4 | api:338 | "Only exported_at and exported_by change" but `version.status` changes. | Add status. |
| C5 | data-model:609 (585-587) | Cascade trigger bug (= PostgreSQL C1). | Return OLD when no parent, or delete children first. |
| C6 | data-model:547,541 | `audit_log_has_state` rejects PasswordReset/Logout/Forbidden; `entity_id NOT NULL` cannot hold LoginFailed for unknown login. | Relax CHECK; nullable entity_id or login entity. |
| C7 | data-model:258-262 | BOOL "1 bit" but `size_bytes NOT NULL` in bytes; STRING(n) size not stated. | BOOL=0 or 1; STRING(n)=n+2. |
| C8 | data-model:245,276,887 | Array rows need an identifier-shaped unique name; no rule; example "ARRAY[1..3] OF INT" breaks the pattern. | Name rule (= PostgreSQL C2 fix). |
| C9 | data-model:748,751,872 | 0001 "creates roles" but connects as recipe_migrator. | Roles in db init script. |
| C10 | data-model:15,18 | Own conventions broken: session/login_attempt/audit_log lack `created_at`; login_attempt has no PK; no CHECK on login/session.id. | List exceptions or fix. |
| C11 | data-model:28-151 | ERD cardinalities wrong (`app_user ||--o{ audit_log` → `\|o--o{`; `machine ||--o{ plc_type` → `\|o--o{`); session/login_attempt relations without FK; created_by/approved_by missing. Mermaid blocks parse (need Mermaid ≥ 10). | Correct. |
| C12 | reliability:393 | `pg_stat_file` cannot report free space. | statvfs / sidecar. |
| C13 | reliability:327,316 | "Docker restarts the container on failure" false for unhealthy; app healthcheck targets /readyz not /healthz. | Rewrite; autoheal if needed. |
| C14 | domain-model:48 vs 247; data-model:811 | "Draft cannot reference archived unit" but CreateVersion copies without checks; empty draft without `based_on` undefined. | Add checks; define empty-draft case. |
| C15 | api:21 vs 34,371,388 | "Every POST carries the CSRF token" but POST /login has no session yet; `POST /lang` missing from §2 tables. | State the login exception and mitigation; add /lang row. |
| C16 | api:221 vs 84 | "Approved version is immutable and has no conflict" but RetireVersion takes row_version and returns ConcurrentEdit. | Rewrite. |
| C17 | domain-model:142 vs data-model:829 | S7 width rule covers only 1/2/4-byte types; 8-byte (DATE_AND_TIME on S7-300) and STRING not covered. | Complete the grammar. |
| C18 | plan:417 vs 422; plan:345 | Flowchart P2.4 → P2.5 serial vs text parallel; "endpointy api.md §6" should be §2.7. | Align. |
| C19 | reliability:76 | Code registry in `recipe-domain/src/error_codes.md` cannot see WEB/DB/AUD codes of other crates. | Shared registry; workspace-level test. |
| C20 | reliability:374 vs 370 | Backup at "02:00 plant time" but containers use UTC. | Set TZ or say UTC. |

## Pass D — ASD-STE100 (15 worst)

No semicolons in English prose. Long sentences and passives, with rewrites:

| # | Where | Finding | Rewrite |
|---|---|---|---|
| D1 | adr/0013:9 | 44 words, four clauses. | "The draft gives four rules. An administrator or a process engineer can create, edit, and delete recipes "when their permissions allow the operation". An operator can only view. "An authorized user approves a recipe version". The application must enforce permissions "for every recipe management operation"." |
| D2 | reliability:402 | 40 words, list in one sentence. | "Send an alert when one of these conditions occurs:" + bullets. |
| D3 | data-model:829 | 36 words + 4-noun cluster. | ""S7 size" is the size of the type on S7 in bytes. This size sets the width letter of an S7-300 address: `X` for a bit, `B` for 1 byte, `W` for 2 bytes, `D` for 4 bytes. A larger type uses `B` and the address of its first byte." |
| D4 | reliability:378 | 34 words, four actions. | "A restore test runs each week. The script restores the latest dump into a scratch database. It counts the rows in each table and compares the counts with the source. It writes the result to the event log." |
| D5 | api:19 | 33 words, six definitions. | Two sentences + a 2-column table. |
| D6 | adr/0001:36 | 33 words. | "Cons: two languages and two toolchains. The client state must stay the same as the server state. The same screens need more code. The UI alone needs a versioned JSON API." |
| D7 | domain-model:71 | 32 words. | "The application does not create primitive types. There is one exception. `DefineUdt` and `UpsertParameter` create a machine-scoped `STRING(n)` or `WSTRING(n)` row when they need it." |
| D8 | adr/0002:43 | 30 words. | Split into two sentences (document header fields / per-parameter fields). |
| D9 | adr/0010:30 | 30 words. | "Cons: one version can contain more than one platform. The address rule of each parameter must find its machine. The export must be split per PLC. A conflict on one machine blocks the whole line." |
| D10 | reliability:393 | 28 words, chain of passives. | "`/readyz` reports the free space of the `pgdata` volume. …" (see C12). |
| D11 | adr/0013:51 | Passive where the actor matters. | "The use case writes one audit entry for each `Forbidden` result. …" |
| D12 | api:131 | Passive. | "The `DeactivateUser` use case deletes all sessions of the user in the same transaction." |
| D13 | adr/0003:71 | Passive, no actor. | "The operations team tests a restore before each release. The weekly automatic test also tests a restore." |
| D14 | adr/0004:43 | Passive; actor matters (B51). | "The `Login` use case writes a `LoginFailed` audit entry without the password. It commits this entry before it returns the error." |
| D15 | domain-model:61 | 4-noun cluster "S7 address width check". | "The domain uses this value to check the width of an S7 address." |

## Verdict
Not yet. The scope is complete and most of it traces to the draft, but about 12 blocking contradictions would become conflicting SHALL statements in the specs: B31, B63/B64, B71/B72, B18/B19, B36–B44, B59, B1, A7, A8, C5, C6 and B51. Fix them first in one docs-only change, with superseding ADRs for ADR-0002, ADR-0003, ADR-0011 and ADR-0012: complete reliability §1.2, data-model §4 and §6, overview §7 and §9, and define N1–N12. After that change, starting OpenSpec P1.1 is safe; the rest of Pass B and all of Pass C can become review comments during the spec work.
