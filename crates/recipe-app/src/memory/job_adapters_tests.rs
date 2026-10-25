//! The in-memory adapters of the job ports work as trait objects (task 3.10, design D6).
//!
//! A job holds each port as an `Arc<dyn Port>`. The test proves that every adapter fits.

use std::sync::Arc;
use std::time::Duration;

use super::testkit::store;
use super::{
    MemoryAuditChain, MemoryBackupDirectory, MemoryDiskSpace, MemoryMaintenance, MemoryOperations,
    MemoryReadOnly, MemoryReadiness, MemoryWebhook,
};
use crate::ports::{
    AlertCondition, AlertWebhook, AuditChainReads, BackupDirectory, ChainPosition, DiskSpace,
    MaintenanceStore, OperationsReads, ReadOnlySwitch, Readiness, ReadinessProbe,
};

#[tokio::test]
async fn every_adapter_works_as_a_trait_object() {
    let store = store();
    let maintenance: Arc<dyn MaintenanceStore> = Arc::new(MemoryMaintenance::new(store.clone()));
    let chain: Arc<dyn AuditChainReads> = Arc::new(MemoryAuditChain::new(store.clone()));
    let operations: Arc<dyn OperationsReads> = Arc::new(MemoryOperations::new(store));
    let readiness: Arc<dyn ReadinessProbe> = Arc::new(MemoryReadiness::default());
    let disk: Arc<dyn DiskSpace> = Arc::new(MemoryDiskSpace::default());
    let backups: Arc<dyn BackupDirectory> = Arc::new(MemoryBackupDirectory::default());
    let webhook: Arc<dyn AlertWebhook> = Arc::new(MemoryWebhook::default());
    let switch: Arc<dyn ReadOnlySwitch> = Arc::new(MemoryReadOnly::default());

    assert_eq!(maintenance.purge_security_events().await.unwrap(), 0);
    assert_eq!(chain.head().await.unwrap().position, ChainPosition::START);
    operations.database_now().await.unwrap();
    assert_eq!(
        readiness.check(Duration::from_secs(1)).await,
        Readiness::Ready
    );
    assert!(disk.free_space("/".as_ref()).await.is_err());
    assert_eq!(backups.newest_complete_set().await.unwrap(), None);
    assert_eq!(webhook.send_all(&[AlertCondition::Panics]).await.len(), 1);
    assert!(!switch.is_on());
}
