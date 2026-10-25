//! In-memory `AlertWebhook` (task 3.10, design D6).

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::call_faults::CallFaults;
use crate::ports::{AlertCondition, AlertWebhook, WebhookFault};

/// The values that an adapter holds.
#[derive(Debug, Default)]
struct Inner {
    /// The conditions of each call of `send_all`, in call order.
    calls: Vec<Vec<AlertCondition>>,
    /// The planned faults and the number of sends of each condition.
    faults: CallFaults<AlertCondition, WebhookFault>,
}

/// In-memory adapter of [`AlertWebhook`].
///
/// The type solves one problem: the tests of the job `alerts` must see which conditions it sends
/// and must fail the send of one condition. The "method" of a fault is the condition. Its call
/// number counts the sends of that condition over all calls of `send_all`. A new adapter
/// succeeds for every condition. A clone shares the record, the counters, and the faults.
#[derive(Clone, Debug, Default)]
pub struct MemoryWebhook {
    /// The values of the adapter.
    inner: Arc<Mutex<Inner>>,
}

impl MemoryWebhook {
    /// Return the conditions of each call of `send_all`, in call order.
    #[must_use]
    pub fn calls(&self) -> Vec<Vec<AlertCondition>> {
        self.inner.lock().calls.clone()
    }

    /// Return the number of sends of `condition`, over all calls.
    #[must_use]
    pub fn sent(&self, condition: AlertCondition) -> u64 {
        self.inner.lock().faults.calls(condition)
    }

    /// Make every send of `condition` fail with `fault`.
    pub fn fail_condition(&self, condition: AlertCondition, fault: WebhookFault) {
        self.fail_from_call(condition, 1, fault);
    }

    /// Make send number `call` of `condition` fail with `fault`. Later sends succeed.
    pub fn fail_on_call(&self, condition: AlertCondition, call: u64, fault: WebhookFault) {
        self.inner
            .lock()
            .faults
            .fail_on_call(condition, call, fault);
    }

    /// Make send number `call` of `condition` and every later send fail with `fault`.
    pub fn fail_from_call(&self, condition: AlertCondition, call: u64, fault: WebhookFault) {
        self.inner
            .lock()
            .faults
            .fail_from_call(condition, call, fault);
    }

    /// Stop every planned fault. The counters keep their values.
    pub fn clear_faults(&self) {
        self.inner.lock().faults.clear();
    }
}

#[async_trait]
impl AlertWebhook for MemoryWebhook {
    async fn send_all(&self, conditions: &[AlertCondition]) -> Vec<Result<(), WebhookFault>> {
        let mut inner = self.inner.lock();
        inner.calls.push(conditions.to_vec());
        conditions
            .iter()
            .map(|condition| match inner.faults.next(*condition) {
                Some(fault) => Err(fault),
                None => Ok(()),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
