# ADR-0022: Approval policy, audit tamper evidence and draft clarifications

Status: Accepted. Amended by ADR-0029 (failed record of a Forbidden event) and ADR-0035 (column grant and guard trigger of security_event). Amended by ADR-0062 (`app_setting` stores the 18 keys of `serve` only) and ADR-0063 (the fingerprint of the webhook URL in `app_setting`). Amended by ADR-0086 (bounds and anchor of the audit chain check). Amended by ADR-0096 (the retry sentence of Risks and mitigations)
Date: 2026-10-12
Deciders: project owner

## Context

ADR-0013 rejected a four-eyes rule. A high finding of the security review showed the consequence. One person at an engineer workstation can widen a process limit, move the value inside it and approve within a minute. No second person sees the change. A process limit protects the process only when the person who sets the value cannot also move the limit alone.

The traceability review (Pass A, item 8) showed that the draft says a parameter "must have ... a description ... and process limits", while the documents made both optional.

The security review also found that the audit log had no tamper evidence. The architecture review (F19) found that the audit table could not store several required entries (`PasswordReset` without state, `LoginFailed` for an unknown login, `Forbidden` before a transaction).

The owner accepted five decisions. Separation of duties is a configuration flag. The audit log has a hash chain in the database. A unit path has at most five levels. Descriptions and limits are required. S7-1200 and S7-1500 parameters can have an address.

## Decision drivers

- A safety-relevant change needs a second person or an explicit plant decision to waive it (R27, R31).
- The draft wording "must have" for descriptions and limits (R08, R09).
- Every mutating operation is recorded and the record is tamper-evident (R20).
- Security events on the failure path must survive the rollback of the failed command (R35).

## Considered options

### Option A: Keep "any Administrator or ProcessEngineer approves" (ADR-0013)

- Pros: simple.
- Cons: a single careless or malicious user controls the whole path.

### Option B: Separation of duties as a configuration flag, default on

- Pros: the rule exists in code, and a plant with one engineer can switch it off with an audited change.
- Cons: a plant with one engineer needs an Administrator to approve.

### Option C: Password re-entry as an electronic signature

- Pros: strongest attribution.
- Cons: a second password prompt on every approval. Kept as an option for a later ADR.

## Decision

The project uses **Option B**.

Approval:

- `approval.separation_of_duties = true` by default. The approver cannot be `created_by` of the draft and cannot appear in `draft_editor` for that draft. The error is `E-RCP-015 ApproverEditedDraft`, 409. The rule runs in `Recipe::approve()`. The database stores the editors in `draft_editor`. No trigger enforces the rule (I16).
- The approve page shows the diff against the current Approved version. Changed limits are highlighted. The approve form carries the `row_version` of the draft.
- At approval every numeric, string, duration, and instant leaf has a process limit. BOOL, CHAR, and WCHAR leaves have none. The error is `E-RCP-016 LimitMissing`, 422, with the list of leaves.
- Leaf classes (amends ADR-0007). Every primitive has a `LeafClass` (`domain-model.md` 2.1): numeric, string, bool, char, duration, or instant. `data-model.md` 7 gives the class of each type.
- A duration or instant value is an integer count of the base unit of its type (Assumption A43). The domain stores, hashes, and limits this count as decimal text. `ProcessLimit.min` and `max` use the same unit.
- The UI accepts and shows IEC literals such as `T#1d2h3m4s5ms` or `DT#2026-10-12-12:30:00`. The parser `IecLiteral` of `recipe-domain` converts them. It is own code, not an external crate.
- A char value is one Unicode scalar. A CHAR value is in `U+0000..=U+00FF`.
- A limit with `leaf_path` `[*]` applies to every element of one array level. The domain rejects a `[*]` row and an index row for the same array level. This amends ADR-0007.
- `ApproveVersion` returns `E-RCP-024 TypeRevisionOutdated` for an outdated parameter unless the request carries `allow_outdated = true`. The audit entry records the flag (ADR-0018, Assumption A29).
- `description_en` is required and non-empty for every parameter. `name_en` was already required. This amends ADR-0012.
- `ApproveVersion` computes `content_sha256` (ADR-0019).

Settings:

- Table `app_setting (key, value, updated_at, updated_by)` stores the effective value of every configuration key. It also holds three state keys: the two keys of the audit chain check and `heartbeat.last_at`. It has an `id` column for the audit entry.
- At startup the server compares the configuration with the stored values. For each difference it writes the audit action `SettingChanged` with the old and the new value, and it updates the row (`reliability.md` 7.12).
- A change of `approval.separation_of_duties` is therefore in the audit log, which has no time limit and a hash chain.

Audit tamper evidence (amends ADR-0005):

- `audit_log` gets `chain_seq`, `prev_hash`, and `entry_hash`. The table `audit_chain_head` holds one row with `last_seq` and `last_hash`.
- A BEFORE INSERT trigger function owned by `audit_owner` (NOLOGIN, `SECURITY DEFINER`) sets the three columns. It runs with `search_path = pg_catalog, public, pg_temp` and schema-qualifies every name.
- The function locks the head row `FOR UPDATE`, sets `chain_seq = last_seq + 1`, and sets `entry_hash = SHA-256(prev_hash || canonical)`. Then it moves the head. The row lock gives all audit inserts one order.
- `canonical` is the JSON text of a constant column list. Timestamps are epoch microseconds, so the text does not depend on the session time zone.
- A nightly job recomputes every hash in `chain_seq` order with `TimeZone = 'UTC'`. First it reads the head. Then it checks the rows up to that head and carries the last hash forward.
- A batch ends at `audit.check_batch` rows or at `audit.check_batch_bytes` of state, whichever comes first. Each batch is one statement in its own short transaction, so no long snapshot holds back VACUUM.
- Each batch runs with `SET LOCAL statement_timeout` = `audit.check_statement_timeout`. On a break the job writes a `security_event` row of kind `audit_chain_broken`, logs `E-AUD-002`, and raises an alert.
- After each completed check the job records `audit.last_check_seq` and `audit.last_check_at` in `app_setting`. The job `alerts` raises an alert when `audit.last_check_at` is older than `audit.max_check_age` (`reliability.md` 3.7).
- Migration `0003` grants `CREATE` on the schema `public` to `audit_owner` for the ownership transfer and revokes it at once.
- `audit_log` also gets `aggregate_type`, `aggregate_id`, `client_ip`, and `session_ref`. Five parameter actions store a diff. Every other action stores the full row (ADR-0005).
- Off-host shipping is a site option described in the runbook, not a requirement.

Security events:

- A new table `security_event` records `login_failed`, `login_rejected_inactive`, `login_throttled`, `csrf_rejected`, `forbidden`, `unlock`, and `audit_chain_broken`. It stores `login_hmac` for unknown logins and for every `login_throttled` event, `user_id` for known ones, `client_ip`, `user_agent`, `request_id` and a small `detail`. The HMAC key is the Docker secret `login_hmac_key` (`reliability.md` 7.1).
- The port `SecurityEventSink` writes the row in its own short transaction. A failed command does not roll it back.
- Rows of the kinds `login_failed`, `login_throttled`, and `csrf_rejected` aggregate per `(client_ip, kind, minute)`. One row holds a `count`, and later events of that minute increment it. A login flood therefore adds a bounded number of rows (`data-model.md` 3.8).
- The table is append-only for `recipe_app`, except the column grant `UPDATE (count)`. Rows are kept 400 days.
- The `SECURITY DEFINER` function `security_event_purge()`, owned by `audit_owner`, deletes older rows. One call deletes at most `security_event.purge_batch` rows in one statement. The supervised job `cleanup` calls it again while a call deletes a full batch.
- The retention and the batch size are constants inside the function.
- `audit_log.entity_id` stays `NOT NULL`. Every business audit entry has a small non-null state object.

Clarifications of the draft, recorded as Assumptions:

- A30: a unit path has 1 to 5 segments. The draft example `U1.2.4.5.6` has five segments and the text says "up to five levels". The earlier documents said six.
- A31: an S7 address is required on S7-300, optional on S7-1200 and S7-1500, and not allowed on CODESYS and TwinCAT 3. One CHECK enforces the three cases. The export writes the stored value when present. The draft requires an address only for S7-300 and does not forbid it elsewhere. Non-optimized data blocks use absolute addresses.
- A33: authentication and authorization failures go to `security_event`. A setting change is the audit action `SettingChanged`.
- A34: separation of duties on approval is on by default. A plant can turn it off through an audited configuration value.
- A36: at approval every numeric, string, duration, and instant leaf has a process limit. BOOL, CHAR, and WCHAR leaves have none. `description_en` is never empty.
- A43: a time value is an integer count of a base unit. `domain-model.md` 2.1 gives the unit and the range of each type.

## Consequences

### Positive

- A limit change and its approval always involve two people, unless the plant decides otherwise in an audited way.
- An edited audit row breaks the chain and is detected the next night.
- Failed logins and forbidden requests are visible with their source address.

### Negative

- `draft_editor` adds one insert to every draft command.
- A plant with one engineer must give the Administrator the approval work or switch the flag off.

### Risks and mitigations

- Risk: the row lock on `audit_chain_head` serializes all audit inserts. Mitigation: each command makes one or more small inserts. The soak test measures the throughput. A SERIALIZABLE command that meets a concurrent head update retries once.

## Related

- Amends ADR-0005, ADR-0007, ADR-0012, and ADR-0013. ADR-0014 (security_event writers), ADR-0016 (draft_editor), ADR-0018 (outdated parameters), ADR-0019 (hash at approval).
- `domain-model.md` sections 2.2, 2.4, 3.
- `data-model.md` sections 3.8, 3.11, 3.12, 3.15, 3.16, 3.17, 4.9, 4.12, 7.
- `api.md` section 3.
- Review findings: security M8 and two high findings. Architecture F19. Traceability Pass A, items 7, 8, 12, and 16. Traceability C6.
