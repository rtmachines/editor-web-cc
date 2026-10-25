//! Refusal of environment values for the database client (ADR-0069 (b), (c)).
//!
//! The driver `sqlx` reads twelve `PG*` values when it builds connect options. One of them,
//! `PGOPTIONS`, can change every session of a role, and no setter clears it. The subcommands
//! that open a database connection therefore refuse to start when such a variable exists.

use anyhow::{Result, bail};

/// The prefix of the environment names that the PostgreSQL client libraries read.
const PREFIX: &str = "PG";

/// Return the first name that starts with `PG`.
///
/// The function takes only names, so a test needs no process environment. Windows reads an
/// environment name without regard to case, so the prefix check ignores case there.
pub fn find_pg_name<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    names.into_iter().find(|name| {
        name.get(..PREFIX.len()).is_some_and(|start| {
            if cfg!(windows) {
                start.eq_ignore_ascii_case(PREFIX)
            } else {
                start == PREFIX
            }
        })
    })
}

/// Refuse to continue when the process environment holds a `PG*` variable.
///
/// The error names the variable and never its value. A name that is not valid UTF-8 cannot
/// be one of the names that the driver reads, so the check skips it.
///
/// # Errors
///
/// Returns an error that names the first variable whose name starts with `PG`.
pub fn refuse_pg_variables() -> Result<()> {
    let names: Vec<String> = std::env::vars_os()
        .filter_map(|(name, _value)| name.into_string().ok())
        .collect();
    match find_pg_name(names.iter().map(String::as_str)) {
        Some(name) => bail!(
            "the environment variable {name} is set. The server reads no variable whose name starts with {PREFIX}. Remove it"
        ),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::find_pg_name;

    #[test]
    fn a_name_that_starts_with_pg_is_found() {
        let names = ["PATH", "HOME", "PGOPTIONS", "PGPASSWORD"];

        assert_eq!(find_pg_name(names), Some("PGOPTIONS"));
    }

    #[test]
    fn every_libpq_name_is_found() {
        let libpq = [
            "PGHOST",
            "PGHOSTADDR",
            "PGPORT",
            "PGDATABASE",
            "PGUSER",
            "PGPASSWORD",
            "PGPASSFILE",
            "PGSSLMODE",
            "PGSSLROOTCERT",
            "PGSSLCERT",
            "PGSSLKEY",
            "PGAPPNAME",
            "PGOPTIONS",
        ];

        for name in libpq {
            assert_eq!(find_pg_name([name]), Some(name), "{name}");
        }
    }

    #[test]
    fn names_without_the_prefix_pass() {
        let names = ["PATH", "HOME", "RECIPE_DB__HOST", "XPGHOST", "P", ""];

        assert_eq!(find_pg_name(names), None);
    }

    #[test]
    fn an_empty_environment_passes() {
        assert_eq!(find_pg_name(std::iter::empty::<&str>()), None);
    }

    /// Windows reads an environment name without regard to case. Other systems do not.
    #[test]
    fn the_case_rule_follows_the_system() {
        let found = find_pg_name(["pgoptions"]);

        assert_eq!(found.is_some(), cfg!(windows));
    }
}
