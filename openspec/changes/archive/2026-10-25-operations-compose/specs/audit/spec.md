# Spec Delta

## MODIFIED Requirements

### Requirement: Chain break
The system SHALL write `E-AUD-002` to the event log and a security event of the kind `audit_chain_broken` when the job `audit_chain_check` finds a break. The job SHALL stop at the break. The job SHALL write the security event through the security event sink in its own transaction. The row SHALL have no client address and the `request_id` of the job run (R20, A33, I13). The job SHALL also report a break for a row outside 1 to the head, and for a head below `audit.last_check_seq`. The job SHALL never lower `audit.last_check_seq` (R20). The job SHALL report a head with a positive `last_seq` and no hash as a break (R20).

#### Scenario: Entry hash does not match
- **GIVEN** an audit entry whose stored hash does not match its content
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job writes `E-AUD-002` to the event log
- **AND** the system writes the security event `audit_chain_broken` without a client address
- **AND** the job stops

#### Scenario: Break found by the job
- **GIVEN** an audit entry whose `entry_hash` does not match its row
- **WHEN** the job `audit_chain_check` runs
- **THEN** the table `security_event` holds one row of the kind `audit_chain_broken` with an empty `client_ip`
- **AND** the row holds the `request_id` of the job run

#### Scenario: Row outside the chain range
- **GIVEN** `audit_log` holds a row with `chain_seq` 0 that a superuser inserted with the triggers off
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job reports `E-AUD-002` for entry 0

#### Scenario: Head set back
- **GIVEN** `audit.last_check_seq` is 10 and `audit_log` holds the rows 1 to 8 with a head of 8
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job reports `E-AUD-002` for entry 9
- **AND** `audit.last_check_seq` stays 10

#### Scenario: Gap in the chain
- **GIVEN** `audit.last_check_seq` is 0 and `audit_log` holds the sound rows 1 and 2
- **AND** `audit_log` holds a row 100 that links to row 2, with a head of 100
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job reports `E-AUD-002` for entry 100
- **AND** `audit.last_check_seq` becomes 2

#### Scenario: Head without a hash
- **GIVEN** the rows 1 to 5 and a head with `last_seq` 5 and no `last_hash`
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job reports `E-AUD-002` for entry 5
- **AND** the system writes the security event `audit_chain_broken`

## ADDED Requirements

### Requirement: Catch-up audit chain check
When `app` starts and `audit.last_check_at` is missing or older than 24 hours, the job `audit_chain_check` SHALL run one check 10 minutes after the start. The nightly check at 03:00 UTC SHALL stay. The nightly check and the catch-up check SHALL NOT run at the same time. The catch-up check SHALL skip when a newer completed check exists at its start (R20, R30).

#### Scenario: Missed nightly check
- **GIVEN** a stored `audit.last_check_at` 30 hours old
- **WHEN** the service `app` starts at 10:00 UTC
- **THEN** the job `audit_chain_check` runs at 10:10 UTC
- **AND** the job records a new `audit.last_check_at` in `app_setting`

#### Scenario: Recent check
- **GIVEN** a stored `audit.last_check_at` 2 hours old
- **WHEN** the service `app` starts
- **THEN** the job `audit_chain_check` runs at the next 03:00 UTC only

#### Scenario: Nightly check before the catch-up
- **GIVEN** a stored `audit.last_check_at` 30 hours old
- **WHEN** the service `app` starts at 02:55 UTC and the nightly check completes before 03:05 UTC
- **THEN** the job `audit_chain_check` runs no catch-up check at 03:05 UTC

### Requirement: Fingerprint of the webhook URL
The stored value of `alerts.webhook_url` and each `SettingChanged` entry of that key SHALL hold the text `sha256:` and 64 lower-case hex characters of the URL. When no URL is set, they SHALL hold the empty text. They SHALL NOT hold the URL (R20, A34).

#### Scenario: New webhook URL
- **GIVEN** the stored empty value of `alerts.webhook_url`
- **WHEN** the service `app` starts with a webhook secret file that holds a URL with a token
- **THEN** the audit log holds one `SettingChanged` entry whose new value starts with `sha256:`
- **AND** no audit entry, no `app_setting` row, and no log line holds the URL
