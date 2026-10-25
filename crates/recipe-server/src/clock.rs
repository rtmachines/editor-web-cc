//! Adapter of the port `Clock` for the system clock (design D3).

use recipe_app::ports::Clock;
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

/// The clock of the host, read in UTC.
///
/// The type solves one problem: the use cases read time through the port `Clock`, so the server
/// needs one adapter that reads the real clock. The job `clock_check` compares this clock with
/// the clock of the database (`reliability.md` section 7.9).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from(OffsetDateTime::now_utc())
    }
}

#[cfg(test)]
mod tests;
