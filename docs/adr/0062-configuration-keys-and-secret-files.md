# ADR-0062: Configuration keys of serve, their delivery, and the secret files

Status: Accepted. Amended by a note: item (k) does not apply to `serve`. Item (o) has an exception for the key files of `security.login_hmac_key_file` and `security.csrf_signing_key_file`. Each of these two files holds a key ring (ADR-0035)
Date: 2026-10-23
Deciders: project owner

## Context

ADR-0031 gives `migrate` a TOML file and environment values with the prefix `RECIPE_`, read by `figment`. Step P4.1 wires `serve`, and the plan found these gaps:

- `reliability.md` section 7.11 says that the configuration file uses the key in the first column. `serve` uses most of these keys. No document says which keys an operator can set.
- Section 7.12 stores the effective value of every key that the server reads from its configuration. Without a fixed list and a fixed text of each value, a new release can write a false `SettingChanged`.
- A21 makes the session timeouts configuration values.
- The ports 8080 and 9000 are code defaults. An integration test on a host with a busy port then fails.
- Compose ignores `uid`, `gid`, and `mode` of a file secret. Only the host file decides who reads it. No document gives the owner, the mode, or the place of a secret file.
- The operator prompt of step P4.1 names "the first administrator from the environment". `reliability.md` section 5 reads the password from stdin or from a secret file.

## Decision drivers

- A password never appears in a configuration file, an environment value, or a log line (ADR-0031, section 3.4).
- Every setting change has an audit entry, and no false entry appears (A34).
- Each key of the configuration is a row of section 7.11.
- One delivery path for Compose, CI, and a local run (R34).

## Considered options

### Option A: A list of 18 operator keys, environment values in `compose.yaml`, secrets as files

- Pros: a short list with a fixed text for each value. The other values stay equal to section 7.11.
- Cons: a site cannot tune a code default without a new release.

### Option B: Every key of section 7.11 as a configuration key

- Pros: each value can change without a new release.
- Cons: most rows of section 7.11 become rows of `app_setting`. A typing error in a limit can weaken a safety rule.

### Option C: A TOML file through Compose `configs` as the only delivery

- Pros: one file holds the whole configuration.
- Cons: the file must exist on the host next to the secrets. CI and the tests need a second path.

## Decision

The project uses **Option A**.

Keys:

- (a) `serve` reads and records the 18 keys of the table below. Every other value of section 7.11 that `serve` uses is a code default. A unit test compares each default with section 7.11.

| Key | Environment name | Canonical text in `app_setting` |
|---|---|---|
| `db.host`, `db.port`, `db.name`, `db.app_user` | `RECIPE_DB__HOST`, `RECIPE_DB__PORT`, `RECIPE_DB__NAME`, `RECIPE_DB__APP_USER` | The given text. The port as a decimal number |
| `secrets.app_password_file` | `RECIPE_SECRETS__APP_PASSWORD_FILE` | The path |
| `security.login_hmac_key_file`, `security.csrf_signing_key_file` | `RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE`, `RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE` | The path |
| `http.trusted_proxies` | `RECIPE_HTTP__TRUSTED_PROXIES`, a comma list | The CIDR texts joined by `,` in the given order |
| `http.public_port`, `http.internal_port` | `RECIPE_HTTP__PUBLIC_PORT`, `RECIPE_HTTP__INTERNAL_PORT` | The port as a decimal number |
| `approval.separation_of_duties` | `RECIPE_APPROVAL__SEPARATION_OF_DUTIES` | `true` or `false` |
| `alerts.webhook_url_file`, `alerts.webhook_ca_file` | `RECIPE_ALERTS__WEBHOOK_URL_FILE`, `RECIPE_ALERTS__WEBHOOK_CA_FILE` | The path, or empty |
| `alerts.webhook_url` | none. ADR-0063 derives it from the file | `sha256:` and 64 hex characters, or empty |
| `session.secure` | `RECIPE_SESSION__SECURE` | `true` or `false` |
| `session.idle_timeout`, `session.absolute_timeout` | `RECIPE_SESSION__IDLE_TIMEOUT`, `RECIPE_SESSION__ABSOLUTE_TIMEOUT`, in whole seconds | Whole seconds and `s`, for example `1800s` |
| `i18n.translation_langs` | `RECIPE_I18N__TRANSLATION_LANGS`, a comma list | The tags joined by `,` |

- (b) `session.secure` with the value `false` stops the start without the feature `dev`, as section 7.11 says.
- (c) `migrate` and `create-admin` keep their own key lists and `deny_unknown_fields`. The keys of `migrate` stay out of `app_setting`. This narrows ADR-0022 and `reliability.md` section 7.12: `app_setting` stores the 18 keys of `serve` only.
- (d) The subcommand `healthcheck` reads `RECIPE_HTTP__INTERNAL_PORT` and no other value (ADR-0059).
- (e) `http.public_port` and `http.internal_port` become configuration keys with the defaults 8080 and 9000. A test can then use free ports.
- (f) The sentence of section 7.11 on the configuration file changes. A key of the first column is a configuration key only when this ADR or ADR-0031 lists it.

Delivery:

- (g) `compose.yaml` gives `app` the `RECIPE_*` environment values. No value holds a secret.
- (h) A secret reaches a process only as a file. A key with the suffix `_file` names that file.
- (i) The file `deploy/.env` holds only the eight values of the Compose interpolation. It holds no password. The values are these:
  - `RECIPE_VERSION`, `RECIPE_BACKUP_DIR`, `RECIPE_WALARCHIVE_DIR`, and `RECIPE_SECRETS_DIR`
  - `COMPOSE_SUBNET`, `COMPOSE_IP_RANGE`, `PROXY_ADDRESS`, and `SITE_ADDRESS`
  ADR-0065 gives the values of the four network variables. Item (l) gives the rule of `RECIPE_SECRETS_DIR`.
- (j) The application never reads `deploy/.env`. The sentence "There is no env-file option" of section 7.1 therefore stays true.
- (k) The TOML file of ADR-0031 stays an option for a local run.

Secret files on the host:

- (l) The secret files live in a directory outside the repository. The directory belongs to `root` and has the mode `0700`. `RECIPE_SECRETS_DIR` names this directory on the host. `compose.yaml` names each secret file as `${RECIPE_SECRETS_DIR:?}/<name>`. CI sets the value to a directory in `$RUNNER_TEMP`.
- (m) Each file with one reader has the mode `0400` and the uid of its reader.
- (n) `app_password` and `migrator_password` have two readers, uid 999 in `db` and uid 65532 in `app` or `migrate`. Each of the two files gets a POSIX ACL for both uids with the mask `r--`. Its mode then shows `0440`, and the owning group gets no right.
- (o) The runbook makes each value with `openssl rand -hex 32`. A secret file holds one line, as ADR-0031 says.
- (p) `.gitignore` gets the lines `/deploy/.env` and `/deploy/secrets/`. The line `/deploy/secrets/` only guards against a mistake, because the secret files live outside the repository.

First administrator:

- (q) The text "the first administrator from the environment" of the operator prompt yields to `reliability.md` section 5. CI runs `create-admin --password-file`.

Settings:

- (r) The row `http.public_port` / `http.internal_port` of section 7.11 keeps the values 8080 / 9000. Its text says that both are configuration keys and that `app_setting` records them.

## Consequences

### Positive

- A new release writes no false `SettingChanged`, because each recorded value has one canonical text.
- No password reaches a configuration file, an environment value, or `deploy/.env`.
- The integration test of `serve` runs on free ports.

### Negative

- A site cannot change a code default without a new release.
- The host preparation needs the modes, the owners, and two ACLs. Compose does not check them.

### Risks and mitigations

- Risk: a secret file with a wrong mode is readable by another uid. Mitigation: the runbook gives the commands for the directory, the modes, and the ACLs, and a check of each file.

## Related

- ADR-0022 (stored settings). This ADR amends it (item (c)).
- ADR-0031 (configuration and secret files), ADR-0035, ADR-0059, ADR-0063, ADR-0065
- `docs/architecture/reliability.md` section 5, section 7.1, section 7.11 (the column "Used by" and the sentence on the configuration file), and section 7.12
- Baseline requirements: operations "Secrets as files", "Startup settings comparison", and "Configuration of the migrator"
- `overview.md` section 7: R34. Section 9: A21, A34
