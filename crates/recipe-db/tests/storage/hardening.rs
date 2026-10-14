//! Hardening of the test harness itself (review S2, R13, S10).
//!
//! The container must not accept a login from the network with a fixed password. The temporary
//! directories must have names that another process cannot guess.

use std::process::Command;

use crate::support::{Role, TempMigrations, server};

/// The literal password that the harness used before the hardening.
const OLD_SUPERUSER_PASSWORD: &str = "test-superuser-password";
const MIN_PASSWORD_LENGTH: usize = 32;
const UUID_SIMPLE_LENGTH: usize = 32;
const LOOPBACK_PREFIX: &str = "127.0.0.1:";

#[test]
fn the_superuser_password_is_random_for_each_run() {
    let password = Role::Superuser.password();

    assert_ne!(password, OLD_SUPERUSER_PASSWORD);
    assert!(
        password.len() >= MIN_PASSWORD_LENGTH,
        "the password is too short: {} bytes",
        password.len()
    );
    assert_eq!(password, Role::Superuser.password());
}

#[test]
fn the_published_port_is_bound_to_the_loopback_address_only() {
    let output = Command::new("docker")
        .args(["port", &server().container_id, "5432/tcp"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout).into_owned();

    assert!(output.status.success(), "docker port failed: {text}");
    let lines: Vec<&str> = text.lines().collect();
    assert!(!lines.is_empty(), "docker port lists no binding");
    for line in lines {
        assert!(
            line.starts_with(LOOPBACK_PREFIX),
            "the port is bound to more than the loopback address: {line}"
        );
    }
}

#[test]
fn a_temporary_directory_has_a_random_name() {
    let first = TempMigrations::new(&[]);
    let second = TempMigrations::new(&[]);

    let name_of = |directory: &TempMigrations| {
        directory
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    };
    for name in [name_of(&first), name_of(&second)] {
        let suffix: Vec<char> = name.chars().rev().take(UUID_SIMPLE_LENGTH).collect();
        assert!(
            suffix.len() == UUID_SIMPLE_LENGTH && suffix.iter().all(char::is_ascii_hexdigit),
            "the name has no random suffix: {name}"
        );
    }
    assert_ne!(first.path(), second.path());
}
