//! Tests of the in-memory `DiskSpace` (task 3.10, design D6, test rows T14).

use std::path::{Path, PathBuf};

use super::{DiskFailure, DiskReading, MemoryDiskSpace};
use crate::ports::{DiskFault, DiskSpace as _, FreeSpace};

fn space(available: u64, total: u64) -> FreeSpace {
    FreeSpace {
        available_blocks: available,
        total_blocks: total,
    }
}

fn pgdata() -> &'static Path {
    Path::new("/var/lib/postgresql/data")
}

fn backups() -> &'static Path {
    Path::new("/backups")
}

#[tokio::test]
async fn each_path_has_its_own_value() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Space(space(5, 100)));
    disk.set(backups(), DiskReading::Space(space(40, 100)));
    assert_eq!(disk.free_space(pgdata()).await.unwrap(), space(5, 100));
    assert_eq!(disk.free_space(backups()).await.unwrap(), space(40, 100));
}

#[tokio::test]
async fn set_replaces_the_value_of_a_path() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Space(space(5, 100)));
    disk.set(pgdata(), DiskReading::Space(space(50, 100)));
    assert_eq!(disk.free_space(pgdata()).await.unwrap(), space(50, 100));
}

#[tokio::test]
async fn a_path_can_be_unsupported() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Failure(DiskFailure::Unsupported));
    let fault = disk.free_space(pgdata()).await.unwrap_err();
    assert!(matches!(fault, DiskFault::Unsupported));
}

#[tokio::test]
async fn a_path_can_be_unreadable() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Failure(DiskFailure::Unreadable));
    let fault = disk.free_space(pgdata()).await.unwrap_err();
    assert!(matches!(fault, DiskFault::Unreadable(_)));
    assert_eq!(fault.to_string(), "The free disk space cannot be read.");
}

#[tokio::test]
async fn a_path_without_a_value_is_unreadable() {
    let disk = MemoryDiskSpace::default();
    let fault = disk.free_space(pgdata()).await.unwrap_err();
    assert!(matches!(fault, DiskFault::Unreadable(_)));
}

#[tokio::test]
async fn the_adapter_records_each_queried_path_in_order() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Space(space(1, 2)));
    disk.free_space(backups()).await.unwrap_err();
    disk.free_space(pgdata()).await.unwrap();
    assert_eq!(
        disk.queries(),
        vec![PathBuf::from("/backups"), pgdata().to_path_buf()]
    );
    assert_eq!(disk.calls(), 2);
}

#[tokio::test]
async fn a_fault_on_the_second_call_hits_one_call_whatever_the_path() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Space(space(5, 100)));
    disk.set(backups(), DiskReading::Space(space(40, 100)));
    disk.fail_on_call(2, DiskFailure::Unreadable);
    disk.free_space(pgdata()).await.unwrap();
    let fault = disk.free_space(backups()).await.unwrap_err();
    assert!(matches!(fault, DiskFault::Unreadable(_)));
    disk.free_space(backups()).await.unwrap();
}

#[tokio::test]
async fn a_fault_from_a_call_on_can_be_unsupported_and_clears() {
    let disk = MemoryDiskSpace::default();
    disk.set(pgdata(), DiskReading::Space(space(5, 100)));
    disk.fail_from_call(1, DiskFailure::Unsupported);
    let fault = disk.free_space(pgdata()).await.unwrap_err();
    assert!(matches!(fault, DiskFault::Unsupported));
    disk.clear_faults();
    disk.free_space(pgdata()).await.unwrap();
}

#[tokio::test]
async fn a_clone_shares_the_values() {
    let disk = MemoryDiskSpace::default();
    let copy = disk.clone();
    copy.set(pgdata(), DiskReading::Space(space(9, 10)));
    assert_eq!(disk.free_space(pgdata()).await.unwrap(), space(9, 10));
}
