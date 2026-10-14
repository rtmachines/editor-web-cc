use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{NewPassword, Password, PasswordHash, ThrottlePolicy};

use super::{Argon2Hasher, Argon2Params, HashFailure, PasswordHasher};
use crate::error::AppError;
use crate::identity::IdentitySettings;

const RELIABILITY_DOC: &str = include_str!("../../../../../docs/architecture/reliability.md");

const GOOD_PASSWORD: &str = "tulip-orbit-lantern-77";
const OTHER_PASSWORD: &str = "tulip-orbit-lantern-78";

/// Small parameters keep the tests fast. The default parameters cost about 50 ms for each hash.
fn small_params() -> Argon2Params {
    Argon2Params {
        m_cost_kib: 64,
        t_cost: 1,
        p_cost: 1,
    }
}

fn fast_hasher() -> Argon2Hasher {
    Argon2Hasher::new(small_params()).unwrap()
}

fn new_password(text: &str) -> NewPassword {
    NewPassword::parse(text).unwrap()
}

fn typed(text: &str) -> Password {
    Password::parse(text).unwrap()
}

// ---- Argon2Hasher ----

#[test]
fn a_hash_with_the_default_parameters_has_the_documented_prefix() {
    let hasher = Argon2Hasher::new(Argon2Params::default()).unwrap();

    let hash = hasher.hash(&new_password(GOOD_PASSWORD)).unwrap();

    assert!(
        hash.as_str().starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
        "the prefix of a hash is wrong"
    );
    assert!(hasher.verify(&typed(GOOD_PASSWORD), Some(&hash)));
}

#[test]
fn the_right_password_passes() {
    let hasher = fast_hasher();
    let hash = hasher.hash(&new_password(GOOD_PASSWORD)).unwrap();

    assert!(hasher.verify(&typed(GOOD_PASSWORD), Some(&hash)));
}

#[test]
fn a_wrong_password_fails() {
    let hasher = fast_hasher();
    let hash = hasher.hash(&new_password(GOOD_PASSWORD)).unwrap();

    assert!(!hasher.verify(&typed(OTHER_PASSWORD), Some(&hash)));
}

#[test]
fn a_check_without_a_hash_runs_against_the_dummy_hash_and_fails() {
    let hasher = fast_hasher();

    assert!(!hasher.verify(&typed(GOOD_PASSWORD), None));
    assert!(!hasher.verify(&typed("x"), None));
}

#[test]
fn the_dummy_hash_has_the_parameters_of_a_real_hash() {
    let hasher = fast_hasher();
    let real = hasher.hash(&new_password(GOOD_PASSWORD)).unwrap();

    let real_prefix = real.as_str().rsplit_once('$').unwrap().0;
    let real_params = real_prefix.rsplit_once('$').unwrap().0;
    assert!(
        hasher.dummy_hash().as_str().starts_with(real_params),
        "the dummy hash must cost the same as a real hash"
    );
}

#[test]
fn two_hashes_of_one_password_differ_by_their_salt() {
    let hasher = fast_hasher();
    let password = new_password(GOOD_PASSWORD);

    let first = hasher.hash(&password).unwrap();
    let second = hasher.hash(&password).unwrap();

    assert_ne!(first, second);
}

#[test]
fn a_stored_hash_that_argon2_cannot_read_fails_the_check() {
    let hasher = fast_hasher();
    let broken = PasswordHash::parse("$argon2id$not-a-real-hash").unwrap();

    assert!(!hasher.verify(&typed(GOOD_PASSWORD), Some(&broken)));
}

#[test]
fn parameters_that_argon2_refuses_give_a_hash_failure() {
    let too_little_memory = Argon2Params {
        m_cost_kib: 1,
        t_cost: 1,
        p_cost: 1,
    };

    let error = Argon2Hasher::new(too_little_memory).unwrap_err();

    assert_eq!(error, HashFailure::Parameters);
}

#[test]
fn the_debug_of_the_hasher_shows_no_secret() {
    let hasher = fast_hasher();
    let hash = hasher.hash(&new_password(GOOD_PASSWORD)).unwrap();

    let shown = format!("{hasher:?}");

    assert!(!shown.contains("$argon2"), "{shown}");
    assert!(!shown.contains(hasher.dummy_hash().as_str()), "{shown}");
    assert!(!shown.contains(hash.as_str()), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
}

// ---- HashFailure ----

#[test]
fn a_hash_failure_gives_e_web_011() {
    for failure in [
        HashFailure::Parameters,
        HashFailure::Random,
        HashFailure::Compute,
    ] {
        let error = AppError::from(failure);

        assert!(
            matches!(
                error,
                AppError::TaskFailed {
                    stage: crate::error::TaskStage::PasswordHash
                }
            ),
            "{failure:?}"
        );
        assert_eq!(error.code(), ErrorCode::WEB_011);
    }
}

#[test]
fn the_text_of_a_hash_failure_holds_no_password() {
    for failure in [
        HashFailure::Parameters,
        HashFailure::Random,
        HashFailure::Compute,
    ] {
        let shown = format!("{failure} {failure:?}");

        assert!(!shown.contains(GOOD_PASSWORD));
        assert!(!shown.contains("$argon2"));
    }
}

// ---- IdentitySettings and reliability.md section 7.11 ----

/// Return the value cell of the row of section 7.11 whose key cell starts with `key`.
fn settings_cell(key: &str) -> &'static str {
    let (_, table) = RELIABILITY_DOC.split_once("### 7.11 Settings").unwrap();
    let marker = format!("| `{key}");
    let row = table
        .lines()
        .find(|line| line.starts_with(&marker))
        .unwrap();
    row.split('|').nth(2).unwrap().trim()
}

fn number_at(cell: &str, index: usize) -> u64 {
    cell.split(" / ")
        .nth(index)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .trim_end_matches('.')
        .parse()
        .unwrap()
}

fn number_after(cell: &str, word: &str) -> u64 {
    let mut words = cell.split_whitespace();
    words.find(|candidate| *candidate == word).unwrap();
    words.next().unwrap().parse().unwrap()
}

fn seconds(value: u64, unit_seconds: u64) -> Duration {
    Duration::from_secs(value.checked_mul(unit_seconds).unwrap())
}

#[test]
fn the_default_argon2_parameters_equal_the_table() {
    let cell = settings_cell("argon2.m_cost");

    let params = Argon2Params::default();

    assert!(cell.contains("MiB"), "{cell}");
    let mib = number_at(cell, 0);
    assert_eq!(u64::from(params.m_cost_kib), mib.checked_mul(1024).unwrap());
    assert_eq!(u64::from(params.t_cost), number_at(cell, 1));
    assert_eq!(u64::from(params.p_cost), number_at(cell, 2));
}

#[test]
fn the_default_settings_equal_the_table() {
    let settings = IdentitySettings::default();

    let permits = settings_cell("argon2.permits");
    assert_eq!(
        u64::try_from(settings.argon2_permits).unwrap(),
        number_at(permits, 0)
    );
    assert!(permits.contains("2 s"), "{permits}");
    assert_eq!(
        settings.argon2_acquire_timeout,
        seconds(number_at(permits, 1), 1)
    );

    assert!(settings_cell("session.idle_timeout").ends_with("min"));
    assert_eq!(
        settings.session_idle_timeout,
        seconds(number_at(settings_cell("session.idle_timeout"), 0), 60)
    );
    assert!(settings_cell("session.absolute_timeout").ends_with(" h"));
    assert_eq!(
        settings.session_absolute_timeout,
        seconds(
            number_at(settings_cell("session.absolute_timeout"), 0),
            3600
        )
    );
    assert_eq!(
        u64::from(settings.page_users),
        number_at(settings_cell("page.users"), 0)
    );
    assert_eq!(
        u64::from(settings.unlock_rows),
        number_at(settings_cell("audit.unlock_rows"), 0)
    );
    assert_eq!(settings.argon2, Argon2Params::default());
}

#[test]
fn the_default_throttle_equals_the_table() {
    let policy = IdentitySettings::default().throttle;

    let delay = settings_cell("login.delay_after_failures");
    assert_eq!(u64::from(policy.free_failures), number_at(delay, 0));
    assert_eq!(policy.window, seconds(number_after(delay, "in"), 60));
    let formula = settings_cell("login.delay_formula");
    assert_eq!(policy.delay_cap, seconds(number_after(formula, "cap"), 1));
    assert_eq!(
        u64::from(policy.address_budget),
        number_at(settings_cell("login.ip_budget"), 0)
    );
    assert_eq!(policy, ThrottlePolicy::default());
}
