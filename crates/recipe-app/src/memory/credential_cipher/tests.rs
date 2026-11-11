//! Tests of the in-memory `CredentialCipher` (task 5.3, design D5).

use recipe_domain::plant::{MachineId, PlcPassword};
use uuid::Uuid;

use super::{CipherCall, MemoryCredentialCipher};
use crate::ports::{CipherFault, CredentialCipher as _, SealedSecret};

const PASSWORD_TEXT: &str = "hunter2-hunter2";

fn machine(n: u128) -> MachineId {
    MachineId::from(Uuid::from_u128(n))
}

fn password() -> PlcPassword {
    PlcPassword::parse(PASSWORD_TEXT).unwrap()
}

#[test]
fn a_sealed_password_opens_on_the_same_machine() {
    let cipher = MemoryCredentialCipher::default();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let opened = cipher.open(machine(1), &sealed).unwrap();
    assert_eq!(opened.expose(), PASSWORD_TEXT);
}

#[test]
fn the_sealed_form_has_the_version_byte_and_hides_the_password_text() {
    let cipher = MemoryCredentialCipher::default();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    assert_eq!(sealed.as_bytes().first(), Some(&1));
    let needle = PASSWORD_TEXT.as_bytes();
    assert!(
        !sealed
            .as_bytes()
            .windows(needle.len())
            .any(|window| window == needle)
    );
}

#[test]
fn a_sealed_password_does_not_open_on_another_machine() {
    let cipher = MemoryCredentialCipher::default();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    let fault = cipher.open(machine(2), &sealed).unwrap_err();
    assert_eq!(fault, CipherFault::Authentication);
}

#[test]
fn a_malformed_secret_does_not_open() {
    let cipher = MemoryCredentialCipher::default();
    for bytes in [vec![], vec![1], vec![9; 40]] {
        let fault = cipher
            .open(machine(1), &SealedSecret::from_bytes(bytes))
            .unwrap_err();
        assert_eq!(fault, CipherFault::Format);
    }
}

#[test]
fn the_adapter_records_each_call_in_order() {
    let cipher = MemoryCredentialCipher::default();
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    cipher.open(machine(1), &sealed).unwrap();
    cipher.seal(machine(2), &password()).unwrap();
    assert_eq!(
        cipher.calls(),
        vec![
            CipherCall::Seal(machine(1)),
            CipherCall::Open(machine(1)),
            CipherCall::Seal(machine(2)),
        ]
    );
    assert_eq!(cipher.seal_count(), 2);
    assert_eq!(cipher.open_count(), 1);
}

#[test]
fn a_planned_seal_fault_fires_once_on_its_call_number() {
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal_on_call(2, CipherFault::Cipher);
    assert!(cipher.seal(machine(1), &password()).is_ok());
    assert_eq!(
        cipher.seal(machine(1), &password()).unwrap_err(),
        CipherFault::Cipher
    );
    assert!(cipher.seal(machine(1), &password()).is_ok());
}

#[test]
fn a_planned_open_fault_leaves_seal_alone() {
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_open(CipherFault::Authentication);
    let sealed = cipher.seal(machine(1), &password()).unwrap();
    assert_eq!(
        cipher.open(machine(1), &sealed).unwrap_err(),
        CipherFault::Authentication
    );
}

#[test]
fn a_failed_call_is_recorded_too() {
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal(CipherFault::Random);
    cipher.seal(machine(1), &password()).unwrap_err();
    assert_eq!(cipher.calls(), vec![CipherCall::Seal(machine(1))]);
}

#[test]
fn clear_faults_stops_the_faults() {
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal(CipherFault::Random);
    cipher.clear_faults();
    assert!(cipher.seal(machine(1), &password()).is_ok());
}

#[test]
fn a_clone_shares_the_calls() {
    let cipher = MemoryCredentialCipher::default();
    let copy = cipher.clone();
    copy.seal(machine(1), &password()).unwrap();
    assert_eq!(cipher.calls().len(), 1);
}
