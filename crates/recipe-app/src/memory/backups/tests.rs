//! Tests of the in-memory `BackupDirectory` (task 3.10, design D6, test row T14).

use super::MemoryBackupDirectory;
use crate::memory::testkit::at;
use crate::ports::{BackupDirectory as _, BackupFault};

#[tokio::test]
async fn a_new_directory_holds_no_complete_set() {
    let directory = MemoryBackupDirectory::default();
    assert_eq!(directory.newest_complete_set().await.unwrap(), None);
}

#[tokio::test]
async fn the_directory_returns_the_time_of_the_newest_set() {
    let directory = MemoryBackupDirectory::default();
    directory.set_newest(Some(at(100)));
    assert_eq!(
        directory.newest_complete_set().await.unwrap(),
        Some(at(100))
    );
    directory.set_newest(None);
    assert_eq!(directory.newest_complete_set().await.unwrap(), None);
}

#[tokio::test]
async fn an_unreadable_directory_is_a_fault_and_not_none() {
    let directory = MemoryBackupDirectory::default();
    directory.set_unreadable();
    let fault = directory.newest_complete_set().await.unwrap_err();
    assert!(matches!(fault, BackupFault::Unreadable(_)));
    assert_eq!(fault.to_string(), "The backup directory cannot be read.");
}

#[tokio::test]
async fn set_newest_ends_the_unreadable_state() {
    let directory = MemoryBackupDirectory::default();
    directory.set_unreadable();
    directory.set_newest(Some(at(5)));
    assert_eq!(directory.newest_complete_set().await.unwrap(), Some(at(5)));
}

#[tokio::test]
async fn a_fault_on_the_second_call_hits_that_call_only() {
    let directory = MemoryBackupDirectory::default();
    directory.set_newest(Some(at(5)));
    directory.fail_on_call(2);
    directory.newest_complete_set().await.unwrap();
    assert!(directory.newest_complete_set().await.is_err());
    directory.newest_complete_set().await.unwrap();
    assert_eq!(directory.calls(), 3);
}

#[tokio::test]
async fn a_fault_from_a_call_on_lasts_until_it_is_cleared() {
    let directory = MemoryBackupDirectory::default();
    directory.fail_from_call(1);
    assert!(directory.newest_complete_set().await.is_err());
    assert!(directory.newest_complete_set().await.is_err());
    directory.clear_faults();
    assert_eq!(directory.newest_complete_set().await.unwrap(), None);
}

#[tokio::test]
async fn a_clone_shares_the_state() {
    let directory = MemoryBackupDirectory::default();
    let copy = directory.clone();
    copy.set_newest(Some(at(1)));
    assert_eq!(directory.newest_complete_set().await.unwrap(), Some(at(1)));
}
