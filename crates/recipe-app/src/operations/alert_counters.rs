//! In-process counters of the job `alerts` (`reliability.md` section 3.7).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The two counters that the job `alerts` reads.
///
/// The type solves one problem: the panic hook and the export flow run outside the job `alerts`.
/// They must report to the job without a lock and without a port.
/// One counter holds the panic events. The other holds the results `E-EXP-002`.
/// A clone shares both counters (design D4, ADR-0060 (z)).
/// The panic hook and the export flow each hold a clone.
/// The hook calls `record_panic`. The export flow calls `record_export_hash_mismatch`.
///
/// Every access uses `Ordering::Relaxed`. A counter publishes no other memory, and the job only
/// needs the value of the counter itself. A stale read is safe, because the job reads again at
/// its next run.
#[derive(Clone, Debug, Default)]
pub struct AlertCounters {
    panics: Arc<AtomicU64>,
    export_hash_mismatches: Arc<AtomicU64>,
}

impl AlertCounters {
    /// Count one panic event.
    pub fn record_panic(&self) {
        self.panics.fetch_add(1, Ordering::Relaxed);
    }

    /// Count one export result `E-EXP-002`. `ExportVersion` calls it.
    pub fn record_export_hash_mismatch(&self) {
        self.export_hash_mismatches.fetch_add(1, Ordering::Relaxed);
    }

    /// Return the number of panic events since the start.
    #[must_use]
    pub fn panics(&self) -> u64 {
        self.panics.load(Ordering::Relaxed)
    }

    /// Return the number of results `E-EXP-002` since the start.
    #[must_use]
    pub fn export_hash_mismatches(&self) -> u64 {
        self.export_hash_mismatches.load(Ordering::Relaxed)
    }
}
