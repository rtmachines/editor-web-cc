//! Tests of the free-space adapter.
//!
//! The Unix tests call `statvfs` on the temporary directory of the host. They do not run on
//! Windows. The CI job on Linux runs them. The Windows test checks the value `Unsupported`.

use recipe_app::ports::{DiskFault, DiskSpace};

use super::SystemDiskSpace;

#[cfg(unix)]
#[tokio::test]
async fn free_space_of_the_temporary_directory_is_consistent() {
    let space = SystemDiskSpace
        .free_space(&std::env::temp_dir())
        .await
        .unwrap();

    assert!(space.total_blocks > 0, "a file system has blocks");
    assert!(space.available_blocks <= space.total_blocks);
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_path_is_unreadable() {
    let missing = std::env::temp_dir().join("recipe-server-disk-missing-path");

    let fault = SystemDiskSpace.free_space(&missing).await.unwrap_err();

    assert!(matches!(fault, DiskFault::Unreadable(_)), "got {fault:?}");
}

#[cfg(unix)]
#[tokio::test]
async fn the_text_of_an_unreadable_fault_names_no_path() {
    let marker = "recipe-server-disk-secret-name";
    let missing = std::env::temp_dir().join(marker);

    let fault = SystemDiskSpace.free_space(&missing).await.unwrap_err();

    // The `Display` text is a fixed sentence. The `Debug` text shows the wrapped error.
    // It shows the marker when the adapter puts the path into that error.
    assert!(matches!(fault, DiskFault::Unreadable(_)), "got {fault:?}");
    assert!(!fault.to_string().contains(marker));
    assert!(!format!("{fault:?}").contains(marker), "got {fault:?}");
}

#[cfg(not(unix))]
#[tokio::test]
async fn a_system_without_statvfs_gives_unsupported() {
    let fault = SystemDiskSpace
        .free_space(&std::env::temp_dir())
        .await
        .unwrap_err();

    assert!(matches!(fault, DiskFault::Unsupported), "got {fault:?}");
}

#[cfg(not(unix))]
#[tokio::test]
async fn a_system_without_statvfs_gives_unsupported_also_for_a_missing_path() {
    let fault = SystemDiskSpace
        .free_space(&std::env::temp_dir().join("recipe-server-disk-missing-path"))
        .await
        .unwrap_err();

    assert!(matches!(fault, DiskFault::Unsupported), "got {fault:?}");
}
