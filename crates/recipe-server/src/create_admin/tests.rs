//! Tests of the password source of `create-admin` (design D17, ADR-0031).

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use recipe_domain::identity::{Login, NewPassword};

use super::{CreateAdminInput, StdinInput, read_password};

const PASSWORD: &str = "Zq7-plant-recipe-ok";

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A temporary file of one test. The drop removes it.
struct TempFile(PathBuf);

impl TempFile {
    fn new(content: &str) -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-create-admin-{}-{number}",
            std::process::id()
        ));
        std::fs::write(&path, content).unwrap();
        Self(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        // A leftover file in the temporary folder does no harm.
        let _ = std::fs::remove_file(&self.0);
    }
}

fn stdin_of(text: &str) -> StdinInput<Cursor<Vec<u8>>> {
    StdinInput::new(Cursor::new(text.as_bytes().to_vec()), false)
}

fn terminal() -> StdinInput<Cursor<Vec<u8>>> {
    StdinInput::new(Cursor::new(format!("{PASSWORD}\n").into_bytes()), true)
}

#[test]
fn create_admin_stdin_and_password_file_give_the_same_password() {
    let file = TempFile::new(&format!("{PASSWORD}\n"));

    let from_file = read_password(Some(&file.0), &mut stdin_of("ignored\n")).unwrap();
    let from_stdin = read_password(None, &mut stdin_of(&format!("{PASSWORD}\n"))).unwrap();
    let without_line_feed = read_password(None, &mut stdin_of(PASSWORD)).unwrap();

    assert_eq!(from_file.as_str(), PASSWORD);
    assert_eq!(from_stdin.as_str(), from_file.as_str());
    assert_eq!(without_line_feed.as_str(), from_file.as_str());
}

#[test]
fn create_admin_empty_input_fails() {
    let file = TempFile::new("");

    let from_file = read_password(Some(&file.0), &mut stdin_of("")).unwrap_err();
    let from_stdin = read_password(None, &mut stdin_of("\n")).unwrap_err();

    assert!(
        format!("{from_file:#}").contains("is empty"),
        "{from_file:#}"
    );
    assert!(
        format!("{from_stdin:#}").contains("is empty"),
        "{from_stdin:#}"
    );
}

#[test]
fn create_admin_two_lines_fail_and_the_error_holds_no_line() {
    let text = format!("{PASSWORD}\nsecondlinetext\n");
    let file = TempFile::new(&text);

    for error in [
        read_password(Some(&file.0), &mut stdin_of("")).unwrap_err(),
        read_password(None, &mut stdin_of(&text)).unwrap_err(),
    ] {
        let message = format!("{error:#}");
        assert!(message.contains("more than one line"), "{message}");
        assert!(!message.contains(PASSWORD), "{message}");
        assert!(!message.contains("secondlinetext"), "{message}");
    }
}

#[test]
fn create_admin_carriage_return_fails() {
    let text = format!("{PASSWORD}\r\n");
    let file = TempFile::new(&text);

    for error in [
        read_password(Some(&file.0), &mut stdin_of("")).unwrap_err(),
        read_password(None, &mut stdin_of(&text)).unwrap_err(),
    ] {
        let message = format!("{error:#}");
        assert!(message.contains("carriage return"), "{message}");
        assert!(!message.contains(PASSWORD), "{message}");
    }
}

#[test]
fn create_admin_terminal_stdin_is_refused_without_reading() {
    let error = read_password(None, &mut terminal()).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("terminal"), "{message}");
    assert!(!message.contains(PASSWORD), "{message}");
}

#[test]
fn create_admin_password_file_does_not_need_stdin_to_be_a_pipe() {
    let file = TempFile::new(&format!("{PASSWORD}\n"));

    let password = read_password(Some(&file.0), &mut terminal()).unwrap();

    assert_eq!(password.as_str(), PASSWORD);
}

#[test]
fn create_admin_missing_password_file_names_the_path() {
    let path = std::env::temp_dir().join("recipe-server-absent-password-file");

    let error = read_password(Some(&path), &mut stdin_of("")).unwrap_err();

    assert!(
        format!("{error:#}").contains("recipe-server-absent-password-file"),
        "{error:#}"
    );
}

#[test]
fn create_admin_oversized_stdin_fails() {
    let text = "a".repeat(100_000);

    let error = read_password(None, &mut stdin_of(&text)).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("too long"), "{message}");
    assert!(!message.contains("aaaa"), "{message}");
}

#[test]
fn create_admin_password_file_of_1025_bytes_gives_the_size_error() {
    let text = "a".repeat(1025);
    let file = TempFile::new(&text);

    let error = read_password(Some(&file.0), &mut stdin_of("")).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("too long"), "{message}");
    assert!(!message.contains("aaaa"), "{message}");
}

#[test]
fn create_admin_huge_password_file_is_read_only_up_to_the_limit() {
    let file = TempFile::new(&"a".repeat(200_000));

    let error = read_password(Some(&file.0), &mut stdin_of("")).unwrap_err();

    assert!(format!("{error:#}").contains("too long"), "{error:#}");
}

#[test]
fn create_admin_password_file_of_exactly_1024_bytes_passes_the_size_check() {
    let mut text = "Zq7-".to_owned();
    text.push_str(&"a".repeat(1020));
    let file = TempFile::new(&text);

    let error = read_password(Some(&file.0), &mut stdin_of("")).unwrap_err();

    // The size check passes, so the policy rejects the 1024 characters (E-IDN-006).
    assert!(format!("{error:#}").contains("E-IDN-006"), "{error:#}");
}

#[test]
fn create_admin_short_password_gives_the_code_e_idn_006_without_the_text() {
    let eleven = "Zq7-plant-o";
    assert_eq!(eleven.chars().count(), 11);

    let error = read_password(None, &mut stdin_of(&format!("{eleven}\n"))).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("E-IDN-006"), "{message}");
    assert!(!message.contains(eleven), "{message}");
}

#[test]
fn create_admin_debug_hides_the_password() {
    let input = CreateAdminInput {
        login: Login::parse("admin").unwrap(),
        password: NewPassword::parse(PASSWORD).unwrap(),
    };

    let debug = format!("{input:?}");

    assert!(debug.contains("[redacted]"), "{debug}");
    assert!(debug.contains("admin"), "{debug}");
    assert!(!debug.contains(PASSWORD), "{debug}");
}
