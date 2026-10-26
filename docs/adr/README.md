# Architecture Decision Records

This directory holds the Architecture Decision Records (ADRs) of the recipe editor. Each ADR records one decision, the options that were considered, and the consequences.

Language: English, ASD-STE100 structural rules. Requirements that are not in `general-draft.txt` are marked "Assumption".

ADR-0014 to ADR-0022 come from the design review of 2026-10-12. They replace or amend earlier decisions. A superseded ADR stays in the directory for history. Read the superseding ADR first.

## Index

| ADR | Title | Status | Date |
|---|---|---|---|
| [0001](0001-frontend-rust-ssr-htmx.md) | Frontend with Rust server-side rendering and HTMX | Accepted. Amended by ADR-0015 (row-version.js) and ADR-0039 (whole value in one form) | 2026-10-12 |
| [0002](0002-export-contract-json.md) | Export contract as a versioned JSON document | Superseded by ADR-0019 | 2026-10-12 |
| [0003](0003-deployment-docker-compose.md) | Deployment with Docker Compose on one on-premises host | Superseded by ADR-0020 | 2026-10-12 |
| [0004](0004-auth-server-sessions.md) | Authentication with server-side sessions in PostgreSQL | Superseded by ADR-0014 | 2026-10-12 |
| [0005](0005-audit-log-all-mutations.md) | Append-only audit log for every mutating operation | Accepted. Amended by ADR-0022 (hash chain, security_event table, diff state, indexes) and ADR-0029 (child rows and source of a full row) | 2026-10-12 |
| [0006](0006-plc-type-catalog-relational.md) | Relational PLC type catalog with immutable UDT revisions | Accepted. Amended by ADR-0017 (derived types) | 2026-10-12 |
| [0007](0007-process-limits-per-leaf.md) | Process limits per scalar leaf of a parameter | Accepted. Amended by ADR-0022 (limits required at approval, `[*]` leaf path, leaf classes) and ADR-0018 (limits at rebind) | 2026-10-12 |
| [0008](0008-optimistic-concurrency-row-version.md) | Optimistic concurrency with a row version on the draft | Accepted. Amended by ADR-0015 (locking model, idempotent approve) and ADR-0028 (row version check of version commands in the domain) | 2026-10-12 |
| [0009](0009-version-snapshot-immutable.md) | A recipe version is a full immutable snapshot | Accepted | 2026-10-12 |
| [0010](0010-recipe-scoped-to-one-machine.md) | A recipe belongs to exactly one machine | Accepted | 2026-10-12 |
| [0011](0011-soft-delete-everywhere.md) | Soft delete for recipes, versions, machines, and users | Superseded by ADR-0021 | 2026-10-12 |
| [0012](0012-translations-table-and-fluent.md) | Parameter translations in a table and UI text with Fluent | Accepted. Amended by ADR-0019 (translations are not exported) and ADR-0022 (description_en required) | 2026-10-12 |
| [0013](0013-rbac-fixed-permissions.md) | Fixed role-based permissions checked in each use case | Accepted. Amended by ADR-0022 (separation of duties, Forbidden recorded in security_event) and ADR-0029 (failed record of a Forbidden event) | 2026-10-12 |
| [0014](0014-session-identity-model.md) | Session and identity model with an owned session table | Accepted. Amended by a note: the idle and absolute limits are a security control (essential exception of WCAG 2.2.1 Timing Adjustable). The UI shows no expiry warning or extension. Amended by ADR-0035 (throttle of ChangeOwnPassword, credential check at the commit, key format, pre-session cookie, create-admin) | 2026-10-12 |
| [0015](0015-concurrency-locking-model.md) | Concurrency and locking model | Accepted | 2026-10-12 |
| [0016](0016-aggregate-boundaries.md) | Aggregate boundaries and command split | Accepted. Amended by ADR-0028 (version header, approval token, version command row check) | 2026-10-12 |
| [0017](0017-derived-type-identity.md) | Identity of derived PLC types and S7-300 layout | Accepted | 2026-10-12 |
| [0018](0018-udt-revision-rebind.md) | UDT revisions and the rebind of recipe drafts | Accepted | 2026-10-12 |
| [0019](0019-export-integrity-and-contract.md) | Export integrity and the export format | Accepted | 2026-10-12 |
| [0020](0020-deployment-migrations-backup.md) | Deployment, migrations, backup and failure policy | Accepted. Amended by ADR-0033 (options of the globals dump). Amended by ADR-0057 (two panic layers) and ADR-0059 (status codes and readiness rule of the health endpoints). Amended by ADR-0061 (`SettingChanged` at the start in read-only mode) and ADR-0065 (no re-encryption to the upstream). Amended by ADR-0066 (retention of every file set) and ADR-0069 (`replication_state()` and the whole `pg_hba.conf`). Amended by ADR-0073 (read-only mode on a failed measurement of the free space). Amended by ADR-0082 (`--no-deps` in the upgrade and the rollback of `app`). Amended by ADR-0083 (the image of `backup` and `walarchive` in the upgrade and the rollback) | 2026-10-12 |
| [0021](0021-soft-delete-and-delete-rules.md) | Soft delete, unique keys and delete rules | Accepted | 2026-10-12 |
| [0022](0022-approval-policy.md) | Approval policy, audit tamper evidence and draft clarifications | Accepted. Amended by ADR-0029 (failed record of a Forbidden event) and ADR-0035 (column grant and guard trigger of security_event). Amended by ADR-0062 (`app_setting` stores the 18 keys of `serve` only) and ADR-0063 (the fingerprint of the webhook URL in `app_setting`). Amended by ADR-0086 (bounds and anchor of the audit chain check) | 2026-10-12 |
| [0023](0023-ste-lint-scope-and-location.md) | Scope and location of the STE lint in CI | Accepted | 2026-10-12 |
| [0024](0024-error-codes-member-names-s7-width.md) | Error codes for duplicate member names and the S7 address width | Accepted | 2026-10-13 |
| [0025](0025-parse-reasons-of-value-objects.md) | Parse reasons of value objects without an own error code | Accepted | 2026-10-13 |
| [0026](0026-ranges-of-date-and-date-and-time.md) | Platform ranges of DATE and DATE_AND_TIME | Accepted | 2026-10-13 |
| [0027](0027-error-lists-and-draft-errors.md) | Error lists and the error of a draft command | Accepted. Amended by ADR-0041 (lists of a value and of limits) | 2026-10-13 |
| [0028](0028-version-header-and-approval-token.md) | Version header, approval token, and draft clarifications | Accepted. Amended by ADR-0029 (the approval token owns the draft) | 2026-10-13 |
| [0029](0029-application-layer-clarifications.md) | Approval token ownership, audit row images, and application clarifications | Accepted. Amended by ADR-0035 (dependencies of recipe-app, row image of LoginUnlocked), ADR-0039 (transport of `new_member_values`), and ADR-0056 (scope of `timeout.export`) | 2026-10-13 |
| [0030](0030-guard-of-a-version-status-write.md) | Guard of a version status write | Accepted | 2026-10-13 |
| [0031](0031-migrator-configuration-and-secret-files.md) | Configuration of the migrator and the secret files of the roles | Accepted. Amended by a note in ADR-0062 (no TOML file for `serve`) | 2026-10-13 |
| [0032](0032-default-databases-closed-to-public.md) | Default databases closed to PUBLIC | Accepted | 2026-10-13 |
| [0033](0033-globals-dump-of-the-backup.md) | Globals dump of the backup | Accepted | 2026-10-14 |
| [0034](0034-hardening-of-the-audit-objects.md) | Hardening of the audit objects | Accepted. Amended by ADR-0035 (security_event_guard in the guard list). Amended by ADR-0069 (`public.replication_state()` in the guard list) | 2026-10-14 |
| [0035](0035-identity-sessions-clarifications.md) | Identity, sessions, and the guard of security events | Accepted | 2026-10-14 |
| [0036](0036-design-system-and-ui-foundation.md) | Design system and UI foundation | Accepted. Amended by ADR-0037 (style sheets, navigation link, whole unit tree) and ADR-0039 (inputs and groups in the leaf grid). Amended by ADR-0041 (alert of `field-errors.html`) and ADR-0047 (refused form page, empty toast). Amended by ADR-0051 (query list of the navigation). Amended by ADR-0057 (no `HX-Reswap` on the answer of the backstop panic layer) and ADR-0071 (read-only banner before `#error-panel`) | 2026-10-14 |
| [0037](0037-plant-model-pages.md) | Pages of the plant model | Accepted. Amended by ADR-0038 (datalist of a new revision, browser test of `E-PLT-005`) and ADR-0051 (navigation link "Audit"). Amended by ADR-0057 (no `HX-Push-Url` on the answer of the backstop panic layer) | 2026-10-14 |
| [0038](0038-struct-cycle-across-revisions.md) | Struct cycle across revisions | Accepted | 2026-10-15 |
| [0039](0039-parameter-value-form.md) | Form of a parameter value | Accepted. Amended by ADR-0045 (groups of a nested array) | 2026-10-15 |
| [0040](0040-post-routes-for-translations-and-removal.md) | POST routes for translations and for the removal of a parameter | Accepted | 2026-10-15 |
| [0041](0041-error-lists-of-a-form.md) | Error lists of a form | Accepted. Amended by ADR-0046 (form query before the fields) and ADR-0050 (a line with no link for a query value without an input) | 2026-10-15 |
| [0042](0042-snapshot-reads-of-draft-queries.md) | Snapshot reads of draft queries | Accepted. Amended by ADR-0050 (approval preview and diff in one snapshot, draft read of the in-memory adapter) | 2026-10-15 |
| [0043](0043-recipe-editor-pages.md) | Pages of the recipe editor | Accepted. Amended by ADR-0044 (bound of the rebind forms) | 2026-10-15 |
| [0044](0044-bound-of-the-rebind-forms.md) | Bound of the rebind forms | Accepted | 2026-10-15 |
| [0045](0045-nested-arrays-inside-a-group.md) | Nested arrays inside a group | Accepted | 2026-10-15 |
| [0046](0046-permission-check-of-a-form-route.md) | Permission check of a form route | Accepted. Amended by ADR-0049 (the five action routes read the body first) | 2026-10-15 |
| [0047](0047-refused-form-page-and-place-of-the-toast.md) | Refused form page and place of the toast | Accepted. Amended by ADR-0049 (m) (scroll padding of a toast of several lines). Amended by ADR-0057 (no empty `#toast` on the answer of the backstop panic layer) | 2026-10-15 |
| [0048](0048-browser-regression-suite-with-playwright.md) | Browser regression suite with Playwright | Accepted | 2026-10-16 |
| [0049](0049-approval-preview-and-lifecycle-actions.md) | Approval preview and lifecycle actions | Accepted. Amended by ADR-0052 (export link in the action area). Amended by ADR-0057 (no `HX-*` header on the answer of the backstop panic layer) | 2026-10-16 |
| [0050](0050-diff-of-two-versions.md) | Diff of two versions | Accepted | 2026-10-16 |
| [0051](0051-audit-history-and-security-event-views.md) | Audit, history, and security event views | Accepted | 2026-10-16 |
| [0052](0052-export-link-on-the-version-page.md) | Export link on the version page | Accepted | 2026-10-22 |
| [0053](0053-error-answers-of-the-export-routes.md) | Error answers of the export routes | Accepted. Amended by ADR-0057 (text answer of the backstop panic layer) | 2026-10-22 |
| [0054](0054-get-only-on-the-export-routes.md) | Only GET on the export routes | Accepted | 2026-10-22 |
| [0055](0055-navigation-guard-of-the-export-routes.md) | Navigation guard of the export routes | Accepted | 2026-10-22 |
| [0056](0056-bound-on-the-parallel-exports.md) | Bound on the parallel exports | Accepted | 2026-10-23 |
| [0057](0057-panic-containment-layers.md) | Panic containment layers | Accepted. Amended by ADR-0092 (item (h): a sixth place, the `JoinSet` of the webhook calls) | 2026-10-23 |
| [0058](0058-http-serving-loop-and-request-timeouts.md) | HTTP serving loop and request timeouts | Accepted. Amended by ADR-0072 (export routes outside the request timeout). Amended by ADR-0085 (the pause `http.accept_error_pause` after an accept fault). Amended by ADR-0087 (the drain of 0 s of the internal listener, the abort without a wait). Amended by ADR-0088 (the three numbers of the stop `WARN` line). Amended by ADR-0089 (c) (item (o): the stop `WARN` line only when a count is above 0) | 2026-10-23 |
| [0059](0059-health-probe-readiness-and-health-router.md) | Health probe subcommand, readiness rule, and health router layers | Accepted | 2026-10-23 |
| [0060](0060-supervised-jobs-ports-and-library-target.md) | Supervised jobs, their ports, the catch-up, and the library target | Accepted. Amended by ADR-0073 (read faults of the jobs and the audit chain alert). Amended by ADR-0088 (the method `run_once(request_id, kind)`, the one loop of `cleanup` and its due times). Amended by ADR-0089 (d) (item (q): the `Err` of a failed snapshot of `alerts` at `WARN`) | 2026-10-23 |
| [0061](0061-start-order-of-serve.md) | Start order of serve | Accepted. Amended by ADR-0073 (a failed measurement of the free space in step 6 of the start). Amended by ADR-0080 (a `RUST_LOG` without a directive, the stderr text after the `ERROR` line). Amended by ADR-0089 (b) (step 6 makes the supervisor and runs the first run of `disk_check` through it). Amended by a note (item (j)). | 2026-10-23 |
| [0062](0062-configuration-keys-and-secret-files.md) | Configuration keys of serve, their delivery, and the secret files | Accepted. Amended by a note (no TOML file for `serve`, key files hold a key ring) | 2026-10-23 |
| [0063](0063-webhook-of-the-alerts.md) | Webhook of the alerts | Accepted. Amended by ADR-0075 (names of the alert conditions in the body). Amended by a note (items (f) and (n)). A mandatory filter of the log layer rejects every event and span of the HTTP client targets of `hyper_util` below `info`. The filter ignores `RUST_LOG`, also span and field directives. `build_filter` in `recipe-server` keeps fixed `info` directives for the same targets. No `RUST_LOG` value shows the host or the address of the webhook. | 2026-10-23 |
| [0064](0064-images-and-their-distribution.md) | Images and their distribution | Accepted | 2026-10-23 |
| [0065](0065-proxy-tls-and-default-trusted-proxy.md) | TLS of the proxy and the default trusted proxy | Accepted. Amended by a note on the risk of `PROXY_ADDRESS`. The check reads the event `login_failed` of a login with a wrong password in the security events. A successful login is an audit entry (A33) | 2026-10-23 |
| [0066](0066-backup-disks-file-sets-and-schedule.md) | Backup disks, file sets, and schedule | Accepted. Amended by ADR-0074 (names of the files on the backup disks) | 2026-10-23 |
| [0067](0067-restore-test-in-the-backup-service.md) | Restore test in the backup service | Accepted. Amended by ADR-0076 (name list and trigger lines of the restore test). Amended by ADR-0083 (the restore test in the upgrade and the rollback, the image of the schema). Amended by ADR-0084 (item (k) and the first risk: the code of a generated column in the restore of `pre-data`) | 2026-10-23 |
| [0068](0068-wal-archive-and-base-backups.md) | WAL archive and base backups | Accepted. Amended by ADR-0077 (WAL cutoff, marker entry and base backup of `pitr-check`). Amended by ADR-0081 (CPU limit of the temporary container of `pitr-check`). Amended by ADR-0099 (order of the gap procedure, target directory of the moved segments) | 2026-10-23 |
| [0069](0069-hardening-of-the-database-access.md) | Hardening of the database access | Accepted | 2026-10-23 |
| [0070](0070-container-limits-networks-healthchecks-and-stack-test.md) | Container limits, networks, healthchecks, and the stack test | Accepted. Amended by ADR-0081 (the temporary container of `pitr-check` in `container.small_cpus`). Amended by ADR-0087 (the term `shutdown.internal_drain` of the first sum rule of (v)). Amended by a note (item (u)). Amended by ADR-0094 (item (w): the trigger rule of the job `compose`). | 2026-10-23 |
| [0071](0071-read-only-banner.md) | Read-only banner | Accepted. Amended by ADR-0073 (text of the read-only banner) | 2026-10-23 |
| [0072](0072-export-routes-outside-the-request-timeout.md) | Export routes outside the request timeout | Accepted | 2026-10-23 |
| [0073](0073-read-faults-and-the-audit-chain-alert.md) | Read faults of the jobs and the audit chain alert | Accepted. Amended by ADR-0075 (field `volume` of the `low_disk` line). Amended by ADR-0086 (an empty chain table keeps the anchor `A`) | 2026-10-23 |
| [0074](0074-names-on-the-backup-disks.md) | Names of the files on the backup disks | Accepted | 2026-10-23 |
| [0075](0075-names-of-the-alert-conditions.md) | Names of the alert conditions | Accepted. Amended by ADR-0093 (b) | 2026-10-23 |
| [0076](0076-name-list-of-the-restore-test.md) | Name list of the restore test | Accepted. Amended by ADR-0078 (enabled state of a trigger, refused kinds of entries) | 2026-10-23 |
| [0077](0077-recovery-check-wal-cutoff-and-scratch-constants.md) | Marker entry of the recovery check, WAL cutoff, and scratch constants | Accepted. Amended by a note on (g). The row `pitr_check.ready_poll` is the connection check of the scratch cluster, not the archive poll that goes away (`reliability.md` 7.11) | 2026-10-24 |
| [0078](0078-trigger-state-and-refused-kinds-of-the-restore-test.md) | Trigger state and refused kinds of the restore test | Accepted. Amended by ADR-0084 (d) (a mitigation of the `DEFAULT` risk) | 2026-10-24 |
| [0079](0079-row-of-the-startup-settings-comparison.md) | Row of the startup settings comparison | Accepted | 2026-10-24 |
| [0080](0080-blank-rust-log-and-the-exit-text-of-main.md) | Blank RUST_LOG and the exit text of main | Accepted | 2026-10-24 |
| [0081](0081-cpu-limit-of-the-recovery-check.md) | CPU limit of the recovery check | Accepted | 2026-10-24 |
| [0082](0082-upgrade-and-rollback-without-dependencies.md) | Upgrade and rollback without the dependencies of app | Accepted. Amended by ADR-0083 (the previous tag reaches only `app` in step 6) | 2026-10-24 |
| [0083](0083-pgtools-image-in-the-upgrade-and-the-rollback.md) | Image of `backup` and `walarchive` in the upgrade and the rollback | Accepted. Amended by a note (VX11-4: reconciles the normative wording) | 2026-10-24 |
| [0084](0084-generated-column-code-in-the-pre-data-restore.md) | Code of a generated column in the restore of `pre-data` | Accepted | 2026-10-24 |
| [0085](0085-pause-after-an-accept-fault.md) | Pause after an accept fault | Accepted | 2026-10-24 |
| [0086](0086-bounds-and-anchor-of-the-audit-chain-check.md) | Bounds and anchor of the audit chain check | Accepted. Amended by a note (item (h) changes ADR-0073 for the anchor of an empty chain table). Amended by ADR-0100 (the audit chain check after a restore: wait for the next run of the job) | 2026-10-24 |
| [0087](0087-drain-of-the-internal-listener.md) | Drain of the internal listener | Accepted. Amended by ADR-0088 (the cut count of the jobs as the third number of the `WARN` line of the stop) | 2026-10-24 |
| [0088](0088-run-interface-of-the-jobs-and-cut-count.md) | Run interface of the jobs and the cut count of the jobs | Accepted. Amended by ADR-0089 (b) (items (a) and (d): the first run of `disk_check` inside the span `job`). Amended by ADR-0089 (c) (item (e): the stop line only for a cut) | 2026-10-24 |
| [0089](0089-job-errors-first-disk-run-and-stop-line.md) | Errors of the jobs, the first run of `disk_check`, and the stop line | Accepted. Amended by ADR-0090 (answers of two open items: a `NULL` `sent_lsn` and `CloseOutcome::TimedOut`). Amended by a note (section 7.3) | 2026-10-24 |
| [0090](0090-null-sent-lsn-and-pool-close-timeout.md) | A `NULL` `sent_lsn` and a pool close that reaches its limit | Accepted | 2026-10-24 |
| [0091](0091-readiness-cache-of-the-health-router.md) | Readiness cache of the health router | Accepted. Amended by a note (item (e)). | 2026-10-24 |
| [0092](0092-webhook-call-panic-isolation.md) | Panic isolation of one webhook call | Accepted | 2026-10-25 |
| [0093](0093-stored-times-and-totals-of-the-alerts-job.md) | Stored times and counter totals of the job `alerts` | Accepted | 2026-10-25 |
| [0094](0094-trigger-of-the-ci-stack-test.md) | Trigger of the CI stack test | Accepted | 2026-10-25 |
| [0095](0095-role-grammar-of-the-globals-file.md) | Role grammar of the globals file in the restore test | Accepted | 2026-10-25 |
| [0096](0096-guard-lock-of-the-last-administrator.md) | Guard lock of the last Administrator | Accepted. Amended by a note on (c). `run_admin_change` reads the actor row again after it takes the guard lock. Both commands refuse with `E-WEB-002` (`AppError::Forbidden`) when the actor lost the permission or the active flag. The refusal writes only a `forbidden` security event. | 2026-10-25 |
| [0097](0097-schema-free-names-in-the-drop-guard.md) | Names without a schema in the drop guard of the audit objects | Accepted | 2026-10-25 |
| [0098](0098-audit-log-insert-columns-and-qualified-operators.md) | Insert columns of `audit_log` and qualified operators of two definer functions | Accepted | 2026-10-25 |
| [0099](0099-stream-before-the-replacement-base-backup.md) | Stream before the replacement base backup after a WAL gap | Accepted | 2026-10-26 |
| [0100](0100-wait-for-the-audit-chain-job-after-a-restore.md) | Wait for the scheduled audit chain check after a restore | Accepted. Amended by ADR-0102 (the restore procedure ends after the operator reads and accepts the result of the next completed run) | 2026-10-26 |
| [0101](0101-start-a-container-for-the-storage-tests.md) | Start a PostgreSQL container for the storage tests | Accepted | 2026-10-26 |
| [0102](0102-end-the-restore-after-the-audit-chain-result.md) | End the restore after the result of the audit chain check | Accepted | 2026-10-26 |

## Decision map

| Topic | Current ADR |
|---|---|
| Frontend | 0001 + 0015 (`row-version.js`) + 0036 (design system) + 0037 (`hx-boost` forms, style sheets) + 0039 (value form) + 0040 (POST routes) |
| Design system, accessibility, UI layout | 0036 + 0037 (whole unit tree, archived state, navigation link) + 0039 (leaf grid) + 0043 (deleted state) + 0045 (nested arrays inside a group). Diff: 0050 (states and icons). Audit: 0051 (pagers, navigation link) |
| Error page, page of a refused form, toast | 0036 + 0047 (refused form page in the session layout, empty toast after an error, sticky toast) + 0049 (m) (scroll padding below 48rem) |
| Pages of machines, units, and UDT types | 0037 + 0038 (datalist of a new revision) + 0041 (error lists) |
| Permission check and fields of a form route | 0013 + 0041 + 0046 (form query with a purpose, check before the fields) + 0049 (l) (the five action routes read the body first) |
| Pages of recipes, versions, and parameters | 0039 (value form, leaf grid) + 0040 (POST routes) + 0041 (error lists) + 0043 (lists, deleted recipe, translations, rebind, conflict) + 0044 (rebind bound) |
| Approval preview and lifecycle actions | 0049 (preview with blockers, danger disclosures, restore in the list) + 0052 (export link in the action area) |
| Diff of two versions | 0050 (domain diff, sections by kind, pages, SQL change flags) |
| Audit, history, and security event pages | 0051 (read port, cursor, filters, scope of A20, formatted JSON) |
| Isolation of queries | 0042 + 0050 (approval preview and diff in one snapshot) |
| Export | 0019 + 0052 (link on the version page) + 0053 (JSON errors, session redirects) + 0054 (only `GET`) + 0055 (guard) + 0056 (permits). Also 0072 (no request timeout) |
| Deployment, migrations, backup, panic policy, health | 0020 + 0031 (migrator configuration, secret files) + 0032 (default databases) + 0033 (globals dump). Server: 0057 (panic layers) + 0058 (accept loop, timeouts, stop). Also 0092 (the sixth place that catches a panic: the `JoinSet` of the webhook calls). Also 0085 (pause after an accept fault) + 0087 (drain of 0 s of the internal listener, abort without a wait). Also 0088 (e) (cut count of the jobs in the `WARN` line of the stop). Also 0089 (c) (that line only for a cut). Also 0090 (b) (a `WARN` line and the exit code 0 after a pool close that reaches its limit). Also 0059 (health) + 0060 (jobs, catch-up, library target) + 0061 (start order). Also 0091 (readiness cache with one probe call at a time, the flag `draining`). Also 0080 (a `RUST_LOG` without a directive, the stderr text of `main`). Jobs: 0073 (read faults, audit chain alert, run state, state key forms) + 0088 (span `job`, `RunKind`, `run_once(request_id, kind)`, one loop of `cleanup`). Also 0089 (`AppError::Audit`, `run_at_start` for the first run of `disk_check`, a failed snapshot of `alerts` at `WARN`). Also 0090 (a) (a row of `walarchive` without a `sent_lsn` makes the condition `replication` active). Also 0093 (a stored time ahead by more than `alerts.interval` keeps its condition active, the `E-EXP-002` total since the previous run). Configuration: 0062 (keys of `serve`, secret files) + 0063 (webhook) + 0075 (names of the alert conditions). Images and Compose: 0064 (images) + 0065 (TLS of the proxy, networks `edge` and `egress`) + 0070 (limits, network `backend`, healthchecks, stack test). Also 0094 (trigger of the stack test: `workflow_dispatch` and a push to `main`). Also 0081 (CPU limit of `pitr-check`). Upgrade and rollback: 0082 (`--no-deps` in steps 4 and 6) + 0083 (the image of `backup` and `walarchive` follows the schema). Backup and database: 0066 (disks, file sets, retention) + 0067 (restore test) + 0076 (name list and trigger lines). Also 0078 (trigger state, refused kinds) + 0084 (code of a generated column in `pre-data`, migration rule). Also 0068 (WAL archive) + 0069 (database access) + 0077 (WAL cutoff, recovery check, scratch constants). Also 0099 (stream before the replacement base backup, target of the moved segments). Also 0100 (no on-demand audit chain check after a restore, wait for the next run of the job). Also 0101 (storage tests start their own container, no `DATABASE_URL`). Also 0102 (the restore procedure ends after the result of the next completed run of `audit_chain_check`). Files on the backup disks: 0074 (marker, temp, and set names) |
| Read-only banner | 0071 (notice at the start of `main`) + 0073 (banner text for a low or unknown free space) |
| Sessions, login, passwords, bootstrap | 0014 + 0035 (throttle of ChangeOwnPassword, credential check at the commit, key format, pre-session cookie, Origin, client address, create-admin) + 0065 (default trusted proxy) |
| Audit log | 0005 + 0022 + 0029 (child rows and source of a full row) + 0034 (audit hardening) + 0035 (security_event guard, LoginUnlocked image). Also 0086 (bounds and anchor of the audit chain check) |
| PLC types, UDT revisions, derived types, S7-300 layout | 0006 + 0017 + 0018 + 0024 (member names, S7 width) + 0038 (struct cycle across revisions) |
| Process limits | 0007 + 0018 (rebind) + 0022 (completeness at approval, `[*]`, leaf classes, time values) + 0026 (ranges of `DATE`, `DATE_AND_TIME`) |
| Errors of value objects without an own code | 0025 + 0073 (`DiskFault` and `BackupFault` as reasons) |
| Error lists, errors of draft commands | 0027 + 0041 (every fault of values and limits, form error lists) + 0047 (page of a refused form without HTMX) + 0050 (e) |
| Concurrency | 0008 + 0015 + 0028 (row version check of version commands) + 0030 (guard of a version status write) + 0035 (lock failure cause) |
| Versions and aggregates | 0009 + 0016 + 0028 (version header, approval token, draft load, value and text clarifications) + 0029 (token owns the draft, two command clarifications) |
| Recipe scope | 0010 |
| Soft delete and delete rules | 0021 |
| Translations | 0012 + 0043 (fixed list of language tags) |
| Permissions and approval | 0013 + 0022 + 0028 (approval token, `Approver`) + 0029 (failed record of a Forbidden event) + 0046 (form queries) |
| Separation of duties on the preview | 0049 (one domain function for the preview and the approval) |
| Dependencies of the application layer | 0029 + 0035 (Argon2, random bytes, semaphore) |
| Stored settings, security event retention | 0022 + 0062 (the 18 keys of `serve`) + 0063 (fingerprint of the webhook URL) + 0079 (locks and statements of the startup comparison) |
| Role grammar of the globals file in the restore test | 0095 (role grammar of the globals file, check before the restore step runs it) |
| Last administrator guard | 0096 (guard lock of the last Administrator, `READ COMMITTED` with the lock first) |
| Drop guard of the audit objects | 0097 (names without a schema in the drop guard) |
| Audit log insert and definer functions | 0098 (insert columns of `audit_log`, qualified operators of two definer functions) |
| STE lint scope and script location in CI | 0023 |
| Browser tests | 0048 (Playwright regression suite on the dev server, smoke suite on the Compose stack) |

## How to add an ADR

1. Copy the shape of an existing ADR: Status, Date, Deciders, Context, Decision drivers, Considered options, Decision, Consequences, Related.
2. Use the next free number and a short slug: `NNNN-<slug>.md`.
3. An accepted ADR changes only by a Status line and an "Amended by" note. To replace it, write a new ADR that supersedes it and set the old status to "Superseded by ADR-NNNN". For a partial change, set "Accepted. Amended by ADR-NNNN (scope)".
4. Add a row to the index and update the decision map.

## Status values

- Proposed: under discussion.
- Accepted: decision made.
- Accepted. Amended by ADR-NNNN (scope): still valid, one part replaced by a later ADR.
- Superseded by ADR-NNNN: replaced in full. Kept for history.
- Rejected: considered, not adopted.
