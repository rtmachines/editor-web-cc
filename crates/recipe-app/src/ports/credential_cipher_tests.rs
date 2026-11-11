//! Tests of the port `CredentialCipher` and of `seal_password` (task 5.3, design D5).

use std::sync::Arc;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{MachineId, PlcPassword};
use uuid::Uuid;

use super::{CipherFault, CredentialCipher, SealedSecret, seal_password};
use crate::error::{AppError, TaskStage};
use crate::memory::MemoryCredentialCipher;

const PASSWORD_TEXT: &str = "s3cret-Pa55";

fn machine() -> MachineId {
    MachineId::from(Uuid::from_u128(7))
}

fn password() -> PlcPassword {
    PlcPassword::parse(PASSWORD_TEXT).unwrap()
}

#[test]
fn the_port_is_usable_as_a_trait_object() {
    fn assert_send_sync<T: Send + Sync + ?Sized>() {}
    assert_send_sync::<dyn CredentialCipher>();
    let cipher: Arc<dyn CredentialCipher> = Arc::new(MemoryCredentialCipher::default());
    assert!(cipher.seal(machine(), &password()).is_ok());
}

#[test]
fn a_sealed_secret_hides_its_bytes_in_debug() {
    let sealed = SealedSecret::from_bytes(vec![1, 2, 3]);
    assert_eq!(format!("{sealed:?}"), "SealedSecret(<redacted>)");
    assert_eq!(sealed.as_bytes(), &[1, 2, 3]);
}

#[test]
fn a_cipher_fault_texts_name_no_secret() {
    for fault in [
        CipherFault::Random,
        CipherFault::Cipher,
        CipherFault::Format,
        CipherFault::Authentication,
    ] {
        let text = format!("{fault} {fault:?}");
        assert!(!text.is_empty());
        assert!(!text.contains(PASSWORD_TEXT));
    }
}

#[test]
fn seal_password_returns_the_sealed_secret_of_the_cipher() {
    let cipher = MemoryCredentialCipher::default();
    let sealed = seal_password(&cipher, machine(), &password()).unwrap();
    let opened = cipher.open(machine(), &sealed).unwrap();
    assert_eq!(opened.expose(), PASSWORD_TEXT);
}

#[test]
fn a_failed_seal_gives_task_failed_with_the_token_stage() {
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal(CipherFault::Random);
    let error = seal_password(&cipher, machine(), &password()).unwrap_err();
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Token
        }
    ));
    assert_eq!(error.code(), ErrorCode::WEB_011);
}

#[test]
fn every_fault_of_a_seal_gives_the_same_error() {
    for fault in [CipherFault::Random, CipherFault::Cipher] {
        let cipher = MemoryCredentialCipher::default();
        cipher.fail_seal(fault);
        let error = seal_password(&cipher, machine(), &password()).unwrap_err();
        assert!(matches!(
            error,
            AppError::TaskFailed {
                stage: TaskStage::Token
            }
        ));
    }
}

#[test]
fn the_error_of_a_failed_seal_holds_no_password() {
    let cipher = MemoryCredentialCipher::default();
    cipher.fail_seal(CipherFault::Cipher);
    let error = seal_password(&cipher, machine(), &password()).unwrap_err();
    let text = format!("{error} {error:?}");
    assert!(!text.contains(PASSWORD_TEXT));
}
