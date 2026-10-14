//! In-memory `AttemptRepository`.

use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::identity::TypedLogin;

use super::before;
use crate::error::StorageError;
use crate::memory::MemoryTransaction;
use crate::ports::{AttemptId, AttemptRepository, AttemptRow, DeletedAttempts, LoginThrottleState};

#[async_trait]
impl AttemptRepository for MemoryTransaction {
    async fn mark_succeeded(&mut self, id: AttemptId) -> Result<(), StorageError> {
        if let Some(row) = self.state_mut().attempts.get_mut(&id) {
            row.succeeded = true;
        }
        Ok(())
    }

    async fn delete_for_login(
        &mut self,
        login: &TypedLogin,
        keep: u32,
    ) -> Result<DeletedAttempts, StorageError> {
        let attempts = &mut self.state_mut().attempts;
        let ids: Vec<AttemptId> = attempts
            .iter()
            .filter(|(_id, row)| row.login == *login)
            .map(|(id, _row)| *id)
            .collect();
        let mut gone: Vec<AttemptRow> = ids.iter().filter_map(|id| attempts.remove(id)).collect();
        let count =
            u64::try_from(gone.len()).map_err(|_| StorageError::invariant("too many attempts"))?;
        gone.sort_by(|left, right| {
            (right.attempted_at, right.id).cmp(&(left.attempted_at, left.id))
        });
        gone.truncate(usize::try_from(keep).unwrap_or(usize::MAX));
        Ok(DeletedAttempts {
            newest: gone,
            count,
        })
    }

    async fn throttle_state(
        &mut self,
        login: &TypedLogin,
        window: Duration,
    ) -> Result<LoginThrottleState, StorageError> {
        let start = before(self.now(), window);
        let times = self
            .state()
            .attempts
            .values()
            .filter(|row| row.login == *login && !row.succeeded && row.attempted_at > start)
            .map(|row| row.attempted_at);
        let (failures, last_failure) = times.fold((0_usize, None), |(count, last), time| {
            (count.saturating_add(1), last.max(Some(time)))
        });
        Ok(LoginThrottleState {
            failures: u32::try_from(failures).unwrap_or(u32::MAX),
            last_failure,
        })
    }
}
