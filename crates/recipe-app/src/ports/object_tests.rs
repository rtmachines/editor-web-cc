//! Each port of design D6 must be usable as a trait object (task 3.1).
//!
//! A port that is not object safe fails to compile here. The stubs live in this module only.
//! The real in-memory adapters belong to task 3.10.

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::export::Sha256;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::{
    AlertCondition, AlertWebhook, AuditChainReads, BackupDirectory, BackupFault, BatchOutcome,
    ChainHead, ChainPosition, ChainSeq, DiskFault, DiskSpace, FreeSpace, MaintenanceStore,
    NotReadyCause, OperationsReads, OperationsSnapshot, ReadOnlySwitch, Readiness, ReadinessProbe,
    ReplicationState, RoleCheck, SnapshotWindows, StateValue, TimeStateKey, WebhookFault,
};
use crate::error::StorageError;

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

fn instant(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

#[test]
fn every_port_is_send_and_sync() {
    assert_send_sync::<dyn MaintenanceStore>();
    assert_send_sync::<dyn AuditChainReads>();
    assert_send_sync::<dyn OperationsReads>();
    assert_send_sync::<dyn ReadinessProbe>();
    assert_send_sync::<dyn DiskSpace>();
    assert_send_sync::<dyn BackupDirectory>();
    assert_send_sync::<dyn AlertWebhook>();
    assert_send_sync::<dyn ReadOnlySwitch>();
}

struct MaintenanceStub;

#[async_trait]
impl MaintenanceStore for MaintenanceStub {
    async fn delete_expired_sessions(
        &self,
        absolute_timeout: Duration,
    ) -> Result<u64, StorageError> {
        Ok(absolute_timeout.as_secs())
    }

    async fn purge_login_attempts(
        &self,
        _retention: Duration,
        batch: u32,
    ) -> Result<u64, StorageError> {
        Ok(u64::from(batch))
    }

    async fn purge_security_events(&self) -> Result<u64, StorageError> {
        Ok(0)
    }

    async fn record_state(&self, value: &StateValue) -> Result<(), StorageError> {
        match value {
            StateValue::HeartbeatLastAt(_) => Err(StorageError::Unavailable),
            StateValue::AuditLastCheckSeq(_) | StateValue::AuditLastCheckAt(_) => Ok(()),
        }
    }
}

#[tokio::test]
async fn maintenance_store_works_as_a_boxed_trait_object() {
    let store: Box<dyn MaintenanceStore> = Box::new(MaintenanceStub);
    assert_eq!(
        store
            .delete_expired_sessions(Duration::from_secs(7))
            .await
            .unwrap(),
        7
    );
    assert_eq!(
        store
            .purge_login_attempts(Duration::from_secs(1), 5000)
            .await
            .unwrap(),
        5000
    );
    assert_eq!(store.purge_security_events().await.unwrap(), 0);
    let beat = StateValue::HeartbeatLastAt(instant(5));
    let error = store.record_state(&beat).await.unwrap_err();
    assert!(matches!(error, StorageError::Unavailable));
    let seq = StateValue::AuditLastCheckSeq(ChainSeq::from(41));
    store.record_state(&seq).await.unwrap();
    let checked = StateValue::AuditLastCheckAt(instant(6));
    store.record_state(&checked).await.unwrap();
}

struct ChainStub;

#[async_trait]
impl AuditChainReads for ChainStub {
    async fn head(&self) -> Result<ChainHead, StorageError> {
        Ok(ChainHead {
            position: ChainPosition::START,
            bounds: None,
            anchor: ChainSeq::ZERO,
        })
    }

    async fn check_batch(
        &self,
        after: &ChainPosition,
        head: ChainSeq,
        _batch: u32,
        _bytes: u64,
    ) -> Result<BatchOutcome, StorageError> {
        if after.seq >= head {
            return Ok(BatchOutcome::Clean {
                rows: 0,
                last: None,
            });
        }
        Ok(BatchOutcome::Broken {
            first_break: ChainSeq::from(head.get()),
            last_clean: after.seq,
        })
    }
}

#[tokio::test]
async fn audit_chain_reads_works_as_a_shared_trait_object() {
    let chain: Arc<dyn AuditChainReads> = Arc::new(ChainStub);
    let head = chain.head().await.unwrap().position;
    assert_eq!(head, ChainPosition::START);
    let after = ChainPosition {
        seq: ChainSeq::from(3),
        hash: Some(Sha256::from([7_u8; 32])),
    };
    let done = chain
        .check_batch(&after, ChainSeq::from(3), 10_000, 8_388_608)
        .await
        .unwrap();
    assert_eq!(
        done,
        BatchOutcome::Clean {
            rows: 0,
            last: None
        }
    );
    let broken = chain
        .check_batch(&head, ChainSeq::from(9), 10_000, 8_388_608)
        .await
        .unwrap();
    assert_eq!(
        broken,
        BatchOutcome::Broken {
            first_break: ChainSeq::from(9),
            last_clean: ChainSeq::ZERO
        }
    );
}

#[test]
fn the_start_of_a_chain_is_sequence_zero_without_a_hash() {
    assert_eq!(ChainPosition::START.seq.get(), 0);
    assert_eq!(ChainPosition::START.hash, None);
}

struct OperationsStub;

#[async_trait]
impl OperationsReads for OperationsStub {
    async fn snapshot(
        &self,
        windows: &SnapshotWindows,
    ) -> Result<OperationsSnapshot, StorageError> {
        Ok(OperationsSnapshot {
            database_now: instant(1_000),
            replication: ReplicationState {
                connected: true,
                lag_bytes: Some(windows.failed_logins.as_secs()),
            },
            failed_logins: 4,
            security_alerts: 0,
            audit_chain_broken_at: Some(instant(900)),
            audit_last_check_at: None,
            first_setting_at: Some(instant(10)),
        })
    }

    async fn database_now(&self) -> Result<Timestamp, StorageError> {
        Ok(instant(2_000))
    }

    async fn role_check(&self) -> Result<RoleCheck, StorageError> {
        Ok(RoleCheck {
            current_user: "recipe_app".to_owned(),
            is_superuser: false,
            owns_relations: false,
        })
    }

    async fn state(&self, key: TimeStateKey) -> Result<Option<Timestamp>, StorageError> {
        Ok((key == TimeStateKey::AuditLastCheckAt).then(|| instant(500)))
    }
}

#[tokio::test]
async fn operations_reads_works_as_a_shared_trait_object() {
    let reads: Arc<dyn OperationsReads> = Arc::new(OperationsStub);
    let windows = SnapshotWindows {
        failed_logins: Duration::from_secs(900),
        security: Duration::from_secs(900),
    };
    let snapshot = reads.snapshot(&windows).await.unwrap();
    assert_eq!(snapshot.database_now, instant(1_000));
    assert_eq!(snapshot.replication.lag_bytes, Some(900));
    assert_eq!(snapshot.failed_logins, 4);
    assert_eq!(snapshot.first_setting_at, Some(instant(10)));
    assert_eq!(snapshot.audit_chain_broken_at, Some(instant(900)));
    assert_eq!(reads.database_now().await.unwrap(), instant(2_000));
    assert!(reads.role_check().await.unwrap().is_allowed());
    let checked = reads.state(TimeStateKey::AuditLastCheckAt).await.unwrap();
    assert_eq!(checked, Some(instant(500)));
    let beat = reads.state(TimeStateKey::HeartbeatLastAt).await.unwrap();
    assert_eq!(beat, None);
}

#[test]
fn a_role_check_allows_only_the_plain_application_role() {
    let plain = RoleCheck {
        current_user: "recipe_app".to_owned(),
        is_superuser: false,
        owns_relations: false,
    };
    assert!(plain.is_allowed());
    let wrong_name = RoleCheck {
        current_user: "recipe_migrator".to_owned(),
        ..plain.clone()
    };
    assert!(!wrong_name.is_allowed());
    let superuser = RoleCheck {
        is_superuser: true,
        ..plain.clone()
    };
    assert!(!superuser.is_allowed());
    let owner = RoleCheck {
        owns_relations: true,
        ..plain
    };
    assert!(!owner.is_allowed());
}

struct ProbeStub;

#[async_trait]
impl ReadinessProbe for ProbeStub {
    async fn check(&self, timeout: Duration) -> Readiness {
        if timeout.is_zero() {
            Readiness::NotReady(NotReadyCause::TimedOut)
        } else {
            Readiness::Ready
        }
    }
}

#[tokio::test]
async fn readiness_probe_works_as_a_boxed_trait_object() {
    let probe: Box<dyn ReadinessProbe> = Box::new(ProbeStub);
    let ready = probe.check(Duration::from_secs(2)).await;
    assert_eq!(ready, Readiness::Ready);
    assert!(ready.is_ready());
    let late = probe.check(Duration::ZERO).await;
    assert_eq!(late, Readiness::NotReady(NotReadyCause::TimedOut));
    assert!(!late.is_ready());
}

struct DiskStub;

#[async_trait]
impl DiskSpace for DiskStub {
    async fn free_space(&self, path: &Path) -> Result<FreeSpace, DiskFault> {
        if path == Path::new("/missing") {
            return Err(DiskFault::Unsupported);
        }
        Ok(FreeSpace {
            available_blocks: 15,
            total_blocks: 100,
        })
    }
}

#[tokio::test]
async fn disk_space_works_as_a_shared_trait_object() {
    let disk: Arc<dyn DiskSpace> = Arc::new(DiskStub);
    let space = disk.free_space(Path::new("/pgdata")).await.unwrap();
    assert_eq!((space.available_blocks, space.total_blocks), (15, 100));
    let fault = disk.free_space(Path::new("/missing")).await.unwrap_err();
    assert!(matches!(fault, DiskFault::Unsupported));
}

#[test]
fn each_disk_fault_has_its_display_text() {
    assert_eq!(
        DiskFault::Unsupported.to_string(),
        "The free disk space cannot be read on this system."
    );
    let unreadable = DiskFault::Unreadable(io::Error::from(io::ErrorKind::PermissionDenied));
    assert_eq!(
        unreadable.to_string(),
        "The free disk space cannot be read."
    );
}

#[test]
fn the_backup_fault_has_its_display_text_without_a_path() {
    let unreadable = BackupFault::Unreadable(io::Error::from(io::ErrorKind::NotFound));
    assert_eq!(
        unreadable.to_string(),
        "The backup directory cannot be read."
    );
}

struct BackupStub {
    newest: Result<Option<Timestamp>, io::ErrorKind>,
}

#[async_trait]
impl BackupDirectory for BackupStub {
    async fn newest_complete_set(&self) -> Result<Option<Timestamp>, BackupFault> {
        self.newest
            .map_err(|kind| BackupFault::Unreadable(io::Error::from(kind)))
    }
}

#[tokio::test]
async fn backup_directory_separates_no_set_from_an_unreadable_directory() {
    let none: Box<dyn BackupDirectory> = Box::new(BackupStub { newest: Ok(None) });
    assert_eq!(none.newest_complete_set().await.unwrap(), None);
    let one: Box<dyn BackupDirectory> = Box::new(BackupStub {
        newest: Ok(Some(instant(77))),
    });
    assert_eq!(one.newest_complete_set().await.unwrap(), Some(instant(77)));
    let broken: Box<dyn BackupDirectory> = Box::new(BackupStub {
        newest: Err(io::ErrorKind::PermissionDenied),
    });
    let fault = broken.newest_complete_set().await.unwrap_err();
    assert!(matches!(
        fault,
        BackupFault::Unreadable(ref source) if source.kind() == io::ErrorKind::PermissionDenied
    ));
}

struct WebhookStub;

#[async_trait]
impl AlertWebhook for WebhookStub {
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>> {
        conditions
            .iter()
            .map(|condition| match condition {
                AlertCondition::NotReady => Err(WebhookFault::Timeout),
                _ => Ok(()),
            })
            .collect()
    }
}

#[tokio::test]
async fn alert_webhook_answers_once_per_condition_in_order() {
    let webhook: Arc<dyn AlertWebhook> = Arc::new(WebhookStub);
    let results = webhook
        .send_all(&[AlertCondition::Panics, AlertCondition::NotReady])
        .await;
    assert_eq!(results.len(), 2);
    assert!(results.first().is_some_and(Result::is_ok));
    assert!(matches!(results.get(1), Some(Err(WebhookFault::Timeout))));
    assert!(webhook.send_all(&[]).await.is_empty());
}

struct SwitchStub(std::sync::atomic::AtomicBool);

impl ReadOnlySwitch for SwitchStub {
    fn set(&self, on: bool) {
        self.0.store(on, std::sync::atomic::Ordering::Relaxed);
    }

    fn is_on(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[test]
fn read_only_switch_works_as_a_shared_trait_object() {
    let switch: Arc<dyn ReadOnlySwitch> = Arc::new(SwitchStub(false.into()));
    assert!(!switch.is_on());
    switch.set(true);
    assert!(switch.is_on());
}
