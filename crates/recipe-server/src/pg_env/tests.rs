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
