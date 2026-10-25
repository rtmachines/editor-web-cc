# ADR-0031: Configuration of the migrator and the secret files of the roles

Status: Accepted. Amended by a note: the sentence of this ADR that `serve` uses the same file format does not apply. The note in ADR-0062 drops the TOML file for `serve`
Date: 2026-10-13
Deciders: project owner

## Context

Step P2.4 adds the subcommand `recipe-server migrate` and the script `deploy/db/initdb/01-roles.sh`. The documents leave four points open:

- `reliability.md` section 7.11 names the settings of the migrator (`migrate.lock_timeout`, `migrate.retries`, `migrate.retry_pause`), but no key for the host, the port, the database, or the user.
- Section 7.11 names `secrets.replicator_password_file` only. `data-model.md` section 5.1 reads four passwords: `migrator_password`, `app_password`, `backup_password`, and `replicator_password`.
- No document gives the format or the location of the configuration file.
- `reliability.md` section 7.1 says that the application reads secrets through `*_FILE` variables only.

## Decision drivers

- The service `migrate` connects as `recipe_migrator` with a password from a secret file (ADR-0020).
- A password never appears in a configuration file, an environment value, or a log line (`reliability.md` sections 3.4 and 7.1).
- Each key of the configuration file is a row of section 7.11.
- No new crate for a feature that a few lines can do.

## Considered options

### Option A: A TOML file with keys of section 7.11, read by `figment`

- Pros: `overview.md` section 5 already lists `figment` for `recipe-server`. One format for every later subcommand. An environment value can override a file value.
- Cons: section 7.11 gets new rows.

### Option B: Environment values only

- Pros: no file.
- Cons: `serve` later needs many values. A second mechanism follows then.

### Option C: Defer the subcommand to P4.1

- Pros: no decision now.
- Cons: the storage tests cannot run the real subcommand, and the operations requirement "Forward-only migrations" stays without a binary.

## Decision

The project uses **Option A**.

- `recipe-server migrate [--config <path>]` reads a TOML file. `figment` merges the file and the environment values with the prefix `RECIPE_`. A double underscore separates the parts of a key, for example `RECIPE_DB__HOST`.
- The parser of the arguments is hand-written. Three subcommands need no parser crate.
- Section 7.11 gets the rows `db.host`, `db.port`, `db.name`, `db.migrator_user`, `secrets.migrator_password_file`, `secrets.app_password_file`, and `secrets.backup_password_file`.
- `migrate` reads the password of `recipe_migrator` only from the file of `secrets.migrator_password_file`. A missing file gives one `ERROR` line and exit code 1 before a connection.
- `01-roles.sh` reads the four password files of section 7.11 inside `psql`. A password never appears in the argument list of a process.
- Before the role statements, `01-roles.sh` sets `log_statement = 'none'` and `log_min_error_statement = 'panic'` in its session. A failed statement then writes no password into the server log.
- A secret file must hold exactly one line. A file that is empty, holds a carriage return, or holds a second line stops `01-roles.sh` and `recipe-server migrate` with an error. Both readers then use the same password.

## Consequences

### Positive

- `migrate` runs in CI and in the storage tests with the same configuration as in Compose.
- Each password has one secret file and one key.

### Negative

- Section 7.11 has seven more rows.
- `serve` must use the same file format when a later step wires it.

### Risks and mitigations

- Risk: an operator puts a password into the TOML file. Mitigation: no key of section 7.11 holds a password. The file holds only the path of the secret file.

## Related

- `reliability.md` sections 3.4, 5, 7.1, and 7.11
- `data-model.md` section 5.1
- `overview.md` section 5
- ADR-0020 (deployment and migrations)
