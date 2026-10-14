//! Tests of the configuration and of the argument parser.
//!
//! The environment of the process is global, so no unit test sets an environment value. The
//! tests pass a provider in its place. The integration test `tests/migrate.rs` sets real
//! `RECIPE_` values on a child process.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use figment::Figment;
use figment::providers::Serialized;

use super::args::{self, Command};
use super::{
    ENV_KEYS, SecretFault, Settings, app_connection, load_with, migrator_connection, secret_line,
};

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A temporary directory of one test. The drop removes it.
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-config-{}-{number}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A provider without any value. It stands for an environment without `RECIPE_` values.
fn no_environment() -> Figment {
    Figment::new()
}

fn words(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn config_reads_the_toml_file() {
    let dir = Sandbox::new();
    let path = dir.write(
        "recipe.toml",
        "[db]\nhost = \"database\"\nport = 6543\nname = \"plant\"\nmigrator_user = \"owner\"\n\
         [secrets]\nmigrator_password_file = '/run/x/pw'\n",
    );

    let settings = load_with(Some(&path), no_environment()).unwrap();

    assert_eq!(settings.db.host, "database");
    assert_eq!(settings.db.port, 6543);
    assert_eq!(settings.db.name, "plant");
    assert_eq!(settings.db.migrator_user, "owner");
    assert_eq!(
        settings.secrets.migrator_password_file,
        PathBuf::from("/run/x/pw")
    );
}

#[test]
fn config_environment_value_overrides_a_file_value() {
    let dir = Sandbox::new();
    let path = dir.write("recipe.toml", "[db]\nhost = \"from-file\"\nport = 6543\n");
    let environment = Figment::new()
        .merge(Serialized::default("db.host", "from-env"))
        .merge(Serialized::default(
            "secrets.migrator_password_file",
            "/env/secret",
        ));

    let settings = load_with(Some(&path), environment).unwrap();

    assert_eq!(settings.db.host, "from-env");
    assert_eq!(settings.db.port, 6543);
    assert_eq!(
        settings.secrets.migrator_password_file,
        PathBuf::from("/env/secret")
    );
}

#[test]
fn config_defaults_equal_the_table_of_reliability_7_11() {
    let settings = load_with(None, no_environment()).unwrap();

    assert_eq!(settings.db.host, "db");
    assert_eq!(settings.db.port, 5432);
    assert_eq!(settings.db.name, "recipe");
    assert_eq!(settings.db.migrator_user, "recipe_migrator");
    assert_eq!(
        settings.secrets.migrator_password_file,
        PathBuf::from("/run/secrets/migrator_password")
    );
}

#[test]
fn config_missing_file_gives_an_error() {
    let dir = Sandbox::new();
    let path = dir.path("absent.toml");

    let error = load_with(Some(&path), no_environment()).unwrap_err();

    assert!(format!("{error:#}").contains("absent.toml"), "{error:#}");
}

#[test]
fn config_malformed_file_gives_an_error() {
    let dir = Sandbox::new();
    let path = dir.write("recipe.toml", "[db\nhost =");

    let error = load_with(Some(&path), no_environment()).unwrap_err();

    assert!(!format!("{error:#}").is_empty());
}

fn settings_with_secret(path: &Path) -> Settings {
    let mut settings = Settings::default();
    settings.secrets.migrator_password_file = path.to_path_buf();
    settings
}

#[test]
fn config_missing_secret_file_gives_an_error_before_a_connection() {
    let dir = Sandbox::new();
    let settings = settings_with_secret(&dir.path("absent_secret"));

    let error = migrator_connection(&settings).unwrap_err();

    let text = format!("{error:#}");
    assert!(text.contains("secrets.migrator_password_file"), "{text}");
    assert!(text.contains("absent_secret"), "{text}");
}

#[test]
fn config_secret_may_end_with_one_line_feed() {
    for (content, expected) in [("secret-1\n", "secret-1"), ("secret-2", "secret-2")] {
        assert_eq!(secret_line(content), Ok(expected));
    }
    let dir = Sandbox::new();
    let settings = settings_with_secret(&dir.write("secret", "secret-3\n"));

    assert!(migrator_connection(&settings).is_ok());
}

#[test]
fn config_secret_line_names_the_fault() {
    assert_eq!(secret_line(""), Err(SecretFault::Empty));
    assert_eq!(secret_line("\n"), Err(SecretFault::Empty));
    assert_eq!(secret_line("a\r\n"), Err(SecretFault::CarriageReturn));
    assert_eq!(secret_line("a\nb"), Err(SecretFault::SeveralLines));
    assert_eq!(secret_line("\n\n"), Err(SecretFault::SeveralLines));
}

#[test]
fn config_secret_with_a_carriage_return_gives_an_error_without_the_content() {
    let dir = Sandbox::new();
    for content in [
        "topsecretvalue\r\n",
        "topsecretvalue\r",
        "top\rsecretvalue\n",
    ] {
        let settings = settings_with_secret(&dir.write("secret", content));

        let error = migrator_connection(&settings).unwrap_err();

        let text = format!("{error:#}");
        assert!(text.contains("carriage return"), "{text}");
        assert!(text.contains("secrets.migrator_password_file"), "{text}");
        assert!(!text.contains("topsecretvalue"), "{text}");
    }
}

#[test]
fn config_secret_with_a_second_line_gives_an_error_without_the_content() {
    let dir = Sandbox::new();
    for content in [
        "topsecretvalue\nsecondline\n",
        "topsecretvalue\n\n",
        "\ntopsecretvalue",
    ] {
        let settings = settings_with_secret(&dir.write("secret", content));

        let error = migrator_connection(&settings).unwrap_err();

        let text = format!("{error:#}");
        assert!(text.contains("more than one line"), "{text}");
        assert!(text.contains("secrets.migrator_password_file"), "{text}");
        assert!(!text.contains("topsecretvalue"), "{text}");
        assert!(!text.contains("secondline"), "{text}");
    }
}

#[test]
fn config_secret_file_without_content_gives_an_error() {
    let dir = Sandbox::new();
    let settings = settings_with_secret(&dir.write("secret", ""));

    let error = migrator_connection(&settings).unwrap_err();

    assert!(format!("{error:#}").contains("is empty"), "{error:#}");
}

#[test]
fn config_unknown_key_in_the_file_gives_an_error_that_names_the_key() {
    let dir = Sandbox::new();
    for text in [
        "[db]\nhots = \"typo\"\n",
        "[secrets]\nmigrator_password = \"x\"\n",
        "[other]\nkey = 1\n",
        "unknown_top_level = 1\n",
    ] {
        let path = dir.write("recipe.toml", text);

        let error = load_with(Some(&path), no_environment()).unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("unknown field"), "{message}");
    }
}

#[test]
fn config_every_environment_key_is_a_key_of_the_settings() {
    // The environment provider reads only `ENV_KEYS`. A key that the settings reject would
    // stop the program at the first start. The integration test `tests/migrate.rs` sets
    // real unrelated `RECIPE_` values on a child process and expects exit code 0.
    for key in ENV_KEYS {
        let environment = Figment::new().merge(Serialized::default(&key.replace("__", "."), "x"));

        let result = load_with(None, environment);

        let message = result.err().map(|error| format!("{error:#}"));
        assert!(
            !message.is_some_and(|text| text.contains("unknown field")),
            "{key}"
        );
    }
    let unknown = Figment::new().merge(Serialized::default("db.unknown", "x"));
    let error = load_with(None, unknown).unwrap_err();
    assert!(format!("{error:#}").contains("unknown field"), "{error:#}");
}

#[test]
fn config_connection_takes_its_values_from_the_settings_and_hides_the_secret() {
    let dir = Sandbox::new();
    let secret = dir.write("secret", "very-secret-text\n");
    let settings = settings_with_secret(&secret);

    let connection = migrator_connection(&settings).unwrap();

    let debug = format!("{connection:?}");
    assert!(!debug.contains("very-secret-text"), "{debug}");
    assert_eq!(connection.host, "db");
    assert_eq!(connection.user, "recipe_migrator");
    assert_eq!(connection.database, "recipe");
    assert_eq!(connection.port, 5432);
}

#[test]
fn config_empty_secret_file_gives_an_error_without_the_content() {
    let dir = Sandbox::new();
    let secret = dir.write("secret", "\n");
    let settings = settings_with_secret(&secret);

    let error = migrator_connection(&settings).unwrap_err();

    assert!(format!("{error:#}").contains("is empty"), "{error:#}");
}

#[test]
fn config_migrate_without_option_has_no_file() {
    let command = args::parse(&words(&["migrate"])).unwrap();

    assert_eq!(command, Command::Migrate { config: None });
}

#[test]
fn config_migrate_takes_the_path_of_the_file() {
    let command = args::parse(&words(&["migrate", "--config", "/etc/recipe.toml"])).unwrap();

    assert_eq!(
        command,
        Command::Migrate {
            config: Some(PathBuf::from("/etc/recipe.toml"))
        }
    );
}

#[test]
fn config_option_without_a_path_gives_an_error() {
    let error = args::parse(&words(&["migrate", "--config"])).unwrap_err();

    assert!(error.to_string().contains("--config"), "{error}");
}

#[test]
fn config_repeated_option_gives_an_error() {
    let error = args::parse(&words(&["migrate", "--config", "a", "--config", "b"])).unwrap_err();

    assert!(error.to_string().contains("--config"), "{error}");
}

#[test]
fn config_unknown_subcommand_gives_an_error() {
    let error = args::parse(&words(&["frobnicate"])).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("argument 1 is not known"), "{message}");
    assert!(!message.contains("frobnicate"), "{message}");
}

#[test]
fn config_missing_subcommand_gives_an_error() {
    let error = args::parse(&[]).unwrap_err();

    assert!(error.to_string().contains("subcommand"), "{error}");
}

#[test]
fn config_unknown_option_gives_an_error() {
    let error = args::parse(&words(&["migrate", "--verbose"])).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("argument 2 is not known"), "{message}");
    assert!(!message.contains("--verbose"), "{message}");
}

#[test]
fn config_unknown_argument_never_appears_in_the_error() {
    const SECRET: &str = "S3cret-Passw0rd-x";
    for args in [
        &["create-admin", "--login", "admin", SECRET][..],
        &["create-admin", SECRET][..],
        &["migrate", SECRET][..],
        &["serve", SECRET][..],
        &[SECRET][..],
    ] {
        let error = args::parse(&words(args)).unwrap_err();

        let message = format!("{error:#}");
        assert!(message.contains("is not known"), "{args:?}: {message}");
        assert!(!message.contains(SECRET), "{args:?}: {message}");
        assert!(!format!("{error:?}").contains(SECRET), "{args:?}");
    }
    let error = args::parse(&words(&["create-admin", "--login", "admin", SECRET])).unwrap_err();
    assert!(
        format!("{error:#}").contains("argument 4 is not known"),
        "{error:#}"
    );
}

#[test]
fn config_serve_parses_but_takes_no_options() {
    assert_eq!(args::parse(&words(&["serve"])).unwrap(), Command::Serve);
    assert!(args::parse(&words(&["serve", "--config", "x"])).is_err());
}

#[test]
fn config_create_admin_requires_the_login() {
    let error = args::parse(&words(&["create-admin"])).unwrap_err();

    assert!(error.to_string().contains("--login"), "{error}");
    let error = args::parse(&words(&["create-admin", "--config", "x"])).unwrap_err();
    assert!(error.to_string().contains("--login"), "{error}");
}

#[test]
fn config_create_admin_takes_login_password_file_and_config() {
    let command = args::parse(&words(&["create-admin", "--login", "Boss.One"])).unwrap();
    assert!(
        matches!(
            &command,
            Command::CreateAdmin { login, password_file: None, config: None }
                if login.as_str() == "boss.one"
        ),
        "{command:?}"
    );

    let command = args::parse(&words(&[
        "create-admin",
        "--config",
        "/etc/recipe.toml",
        "--password-file",
        "/run/pw",
        "--login",
        "admin",
    ]))
    .unwrap();
    assert!(
        matches!(
            &command,
            Command::CreateAdmin { login, password_file: Some(file), config: Some(path) }
                if login.as_str() == "admin"
                    && file == Path::new("/run/pw")
                    && path == Path::new("/etc/recipe.toml")
        ),
        "{command:?}"
    );
}

#[test]
fn config_create_admin_rejects_unknown_repeated_and_incomplete_options() {
    for bad in [
        &["create-admin", "--login", "admin", "--verbose"][..],
        &["create-admin", "--login", "admin", "--login", "other"][..],
        &["create-admin", "--login"][..],
        &["create-admin", "--login", "admin", "--password-file"][..],
        &[
            "create-admin",
            "--login",
            "admin",
            "--password-file",
            "a",
            "--password-file",
            "b",
        ][..],
        &[
            "create-admin",
            "--login",
            "admin",
            "--config",
            "a",
            "--config",
            "b",
        ][..],
        &["create-admin", "--login", "no"][..],
        &["create-admin", "--login", "has space"][..],
    ] {
        assert!(args::parse(&words(bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn config_app_user_defaults_to_recipe_app_and_its_secret_file_to_the_table() {
    let settings = load_with(None, no_environment()).unwrap();

    assert_eq!(settings.db.app_user, "recipe_app");
    assert_eq!(
        settings.secrets.app_password_file,
        PathBuf::from("/run/secrets/app_password")
    );
}

#[test]
fn config_app_user_comes_from_the_file_and_the_environment_overrides_it() {
    let dir = Sandbox::new();
    let path = dir.write(
        "recipe.toml",
        "[db]\napp_user = \"from_file\"\n[secrets]\napp_password_file = '/file/app'\n",
    );
    let from_file = load_with(Some(&path), no_environment()).unwrap();
    assert_eq!(from_file.db.app_user, "from_file");
    assert_eq!(
        from_file.secrets.app_password_file,
        PathBuf::from("/file/app")
    );

    let environment = Figment::new().merge(Serialized::default("db.app_user", "from_env"));
    let settings = load_with(Some(&path), environment).unwrap();
    assert_eq!(settings.db.app_user, "from_env");
}

#[test]
fn config_environment_names_include_the_app_keys() {
    assert!(ENV_KEYS.contains(&"db__app_user"));
    assert!(ENV_KEYS.contains(&"secrets__app_password_file"));
}

#[test]
fn config_app_connection_uses_the_app_user_and_its_secret_file() {
    let dir = Sandbox::new();
    let secret = dir.write("app_secret", "app-secret-text\n");
    let mut settings = Settings::default();
    settings.secrets.app_password_file = secret;

    let connection = app_connection(&settings).unwrap();

    assert_eq!(connection.user, "recipe_app");
    assert_eq!(connection.database, "recipe");
    let debug = format!("{connection:?}");
    assert!(!debug.contains("app-secret-text"), "{debug}");
}

#[test]
fn config_app_secret_fault_names_the_key_and_hides_the_content() {
    let dir = Sandbox::new();
    let mut settings = Settings::default();
    settings.secrets.app_password_file = dir.write("app_secret", "topsecretvalue\nsecondline\n");

    let error = app_connection(&settings).unwrap_err();

    let text = format!("{error:#}");
    assert!(text.contains("secrets.app_password_file"), "{text}");
    assert!(text.contains("more than one line"), "{text}");
    assert!(!text.contains("topsecretvalue"), "{text}");
}
