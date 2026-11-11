# ADR-0155: Record the PLC credential key file in app_setting

Status: Accepted. Amends ADR-0062 (a) and (c) (the list of the recorded keys) and ADR-0154 (the key file of the PLC password)
Date: 2026-11-11
Deciders: project owner

## Context

ADR-0062 (a) gives the 18 keys that `serve` reads and records in `app_setting`. The list holds the paths of the key files `security.login_hmac_key_file` and `security.csrf_signing_key_file`.

ADR-0154 (g) and (ha) added the Docker secret `plc_credential_key` and the key `security.plc_credential_key_file`. `serve` reads the key, but ADR-0154 did not add it to the list of ADR-0062 (a). `app_setting` therefore records no path of the PLC key file.

These facts shape the decision:

- `reliability.md` 7.12 compares each recorded key at startup. A difference gives one audit entry `SettingChanged` with the old and the new value.
- A missing row counts as a difference with the old value `null` (`reliability.md` 7.12, ADR-0079).
- `app_setting` has no check on the key names. A new key needs no migration (`data-model.md` 3.17).
- A test comment in `recipe-server` records the gap between ADR-0062 (a) and ADR-0154.

The project owner decided on 2026-11-11 to record the key like the other two key files.

## Decision drivers

- Every setting change has an audit entry (R20, A34).
- The three key files of `serve` have one audit trail.
- A secret key never appears in a table, an audit entry, or a log line (ADR-0154 (h), `reliability.md` 3.4).

## Considered options

### Option A: Record the path of the PLC key file

- Pros: a change of the path has an audit entry, like the other two key files.
- Cons: one more `SettingChanged` entry on the first start after the upgrade.

### Option B: Keep the key out of `app_setting`

- Pros: no change.
- Cons: a change of the path of the PLC key file has no audit entry. Two lists of `serve` keys stay different.

## Decision

The project uses **Option A**. It amends ADR-0062 (a) and (c) and ADR-0154.

- (a) `serve` reads and records 19 keys. The 19th key is `security.plc_credential_key_file`. This changes the count 18 of ADR-0062 (a) and (c) to 19.
- (b) The key gets this row in the table of ADR-0062 (a):

| Key | Environment name | Canonical text in `app_setting` |
|---|---|---|
| `security.plc_credential_key_file` | `RECIPE_SECURITY__PLC_CREDENTIAL_KEY_FILE` | The path |

- (c) `app_setting` stores the path of the file, never the AES key. A new AES key in a file at the same path therefore gives no `SettingChanged` entry. The same rule applies to the other two key files.
- (d) `CompareSettings` treats the key like every other recorded key (`reliability.md` 7.12). A changed path gives one `SettingChanged` entry with the old and the new path.
- (e) The first start of a release with this ADR finds no row of `security.plc_credential_key_file`. `CompareSettings` counts the missing row as a difference with the old value `null`. It writes one `SettingChanged` entry with the old value `null` and the new path. Then it inserts the row.
- (f) That entry has `aggregate_type = 'app_setting'` and `actor = None`. It shares the `request_id` of the other entries of the same start. It also occurs in read-only mode (ADR-0061 (e)).
- (g) The rules of ADR-0154 (g), (h), and (ha) on the key file stay. This ADR adds no error code, no migration, and no new setting.

## Consequences

### Positive

- A change of the path of the PLC key file has an audit entry with the old and the new path.
- The three key files of `serve` have one rule.

### Negative

- The first start after the upgrade writes one more `SettingChanged` entry, with the old value `null`.
- `reliability.md` 7.12, `data-model.md` 3.17, and the runbook change their count of the recorded keys to 19.

### Risks and mitigations

- Risk: an operator reads the entry with the old value `null` as an unknown change. Mitigation: the runbook names the entry in the upgrade section.

## Related

- ADR-0022 (stored settings), ADR-0061 (e) (read-only mode at startup), ADR-0062 (a) and (c), ADR-0079 (statements of the startup comparison), ADR-0154 (g), (h), and (ha)
- `docs/architecture/reliability.md` 7.11 and 7.12, `docs/architecture/data-model.md` 3.17
- `overview.md` §7: R20, R31. §9: A34
