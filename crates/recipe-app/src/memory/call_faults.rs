//! Plan of the faults of one in-memory adapter, by method and by call number (task 3.10).

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// One planned fault: the method, the first call that fails, and whether later calls fail too.
#[derive(Clone, Debug)]
struct Plan<K, F> {
    /// The method that the plan targets.
    method: K,
    /// The number of the first call that fails. The first call of a method has the number 1.
    first_call: u64,
    /// True when every call from `first_call` on fails. False when only that call fails.
    repeats: bool,
    /// The fault that a failing call returns.
    fault: F,
}

impl<K: Copy + Eq, F> Plan<K, F> {
    /// Return true when this plan makes call number `call` of `method` fail.
    fn matches(&self, method: K, call: u64) -> bool {
        if self.method != method || self.first_call == 0 {
            return false;
        }
        if self.repeats {
            call >= self.first_call
        } else {
            call == self.first_call
        }
    }
}

/// The faults of one adapter and the number of calls of each method.
///
/// The type solves one problem: a job test must fail the n-th call of one method and keep the
/// calls before it whole. The adapters of the ports use one plan each. `K` names a method and
/// `F` is the fault that the adapter turns into its own error.
#[derive(Clone, Debug)]
pub(crate) struct CallFaults<K, F> {
    /// The number of calls that each method received.
    counts: HashMap<K, u64>,
    /// The planned faults. The first plan that matches wins.
    plans: Vec<Plan<K, F>>,
}

impl<K, F> Default for CallFaults<K, F> {
    fn default() -> Self {
        Self {
            counts: HashMap::new(),
            plans: Vec::new(),
        }
    }
}

impl<K: Copy + Eq + Hash, F: Clone> CallFaults<K, F> {
    /// Make call number `call` of `method` fail with `fault`. Later calls succeed.
    ///
    /// A call number of 0 never fires.
    pub(crate) fn fail_on_call(&mut self, method: K, call: u64, fault: F) {
        self.push(method, call, false, fault);
    }

    /// Make call number `call` of `method` and every later call fail with `fault`.
    ///
    /// A call number of 0 never fires.
    pub(crate) fn fail_from_call(&mut self, method: K, call: u64, fault: F) {
        self.push(method, call, true, fault);
    }

    /// Drop every plan. The counters keep their values.
    pub(crate) fn clear(&mut self) {
        self.plans.clear();
    }

    /// Return the number of calls that `method` received.
    pub(crate) fn calls(&self, method: K) -> u64 {
        self.counts.get(&method).copied().unwrap_or(0)
    }

    /// Count one call of `method` and return the fault of that call, or `None`.
    pub(crate) fn next(&mut self, method: K) -> Option<F> {
        let counter = self.counts.entry(method).or_insert(0);
        // A counter that a test reads stops at the largest value and never wraps.
        *counter = counter.saturating_add(1);
        let call = *counter;
        self.plans
            .iter()
            .find(|plan| plan.matches(method, call))
            .map(|plan| plan.fault.clone())
    }

    /// Add one plan at the end of the list.
    fn push(&mut self, method: K, first_call: u64, repeats: bool, fault: F) {
        self.plans.push(Plan {
            method,
            first_call,
            repeats,
            fault,
        });
    }
}

#[cfg(test)]
mod tests;
