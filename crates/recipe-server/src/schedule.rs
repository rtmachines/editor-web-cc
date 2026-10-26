//! The schedules of the supervised jobs (design D6, ADR-0060 (i), `reliability.md` section 2.4).
//!
//! A schedule answers one question: how long must the supervisor wait before the next run, and
//! which kind of run follows (ADR-0088 (b))?
//! The answer is a pure function of two values. These values are the time since the start of the
//! supervisor and the time of day in UTC. A test therefore needs no runtime and no clock.
//!
//! A schedule has one or more triggers:
//!
//! - `Every` waits one interval after the end of the last run. A late run therefore gives no
//!   burst of catch-up runs.
//! - `Daily` waits for the next time of day in UTC (`audit.check_time`).
//! - `Once` waits for a delay since the start and then fires one time (`audit.catchup_delay`).
//!
//! All triggers of one job live in one schedule, so the supervisor runs them in one loop. Two
//! runs of one job therefore never overlap. A trigger that is due while a run is open fires when
//! that run ends.

use std::fmt;
use std::time::Duration;

use recipe_app::jobs::RunKind;

/// The seconds of one UTC day.
const DAY_SECONDS: u64 = 86_400;

/// The seconds of one UTC day, as a signed number for the Unix time.
const DAY_SECONDS_SIGNED: i64 = 86_400;

/// The length of one hour.
const HOUR: Duration = Duration::from_secs(3_600);

/// The length of one minute.
const MINUTE: Duration = Duration::from_secs(60);

/// The largest count of nanoseconds in a second.
const MAX_SUBSECOND_NANOS: u32 = 999_999_999;

/// A value of a schedule that the constructor refuses.
///
/// The type has no `code()` and does not implement `std::error::Error`. It is a reason type like
/// `ProbeFault`. The caller wraps the fault with `anyhow!` and names the job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleFault {
    /// An interval is zero. The job would run in a burst.
    ZeroInterval,
    /// The hour of a time of day is above 23.
    Hour,
    /// The minute of a time of day is above 59.
    Minute,
}

impl fmt::Display for ScheduleFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ZeroInterval => "An interval of a schedule must be longer than zero.",
            Self::Hour => "The hour of a time of day must be from 0 to 23.",
            Self::Minute => "The minute of a time of day must be from 0 to 59.",
        })
    }
}

/// A time of day in UTC, to the minute.
///
/// The type solves one problem: a daily schedule must never hold an hour or a minute that does not
/// exist. The only constructor is `parse`, so code does not check the value again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay(Duration);

impl TimeOfDay {
    /// Make a time of day from an hour and a minute in UTC.
    ///
    /// # Errors
    ///
    /// Returns `ScheduleFault::Hour` for an hour above 23. Returns `ScheduleFault::Minute` for a
    /// minute above 59.
    pub fn parse(hour: u8, minute: u8) -> Result<Self, ScheduleFault> {
        if hour > 23 {
            return Err(ScheduleFault::Hour);
        }
        if minute > 59 {
            return Err(ScheduleFault::Minute);
        }
        // The checks above keep every step below one day, so a `None` cannot occur.
        let hours = HOUR
            .checked_mul(u32::from(hour))
            .ok_or(ScheduleFault::Hour)?;
        let minutes = MINUTE
            .checked_mul(u32::from(minute))
            .ok_or(ScheduleFault::Minute)?;
        let since_midnight = hours.checked_add(minutes).ok_or(ScheduleFault::Hour)?;
        Ok(Self(since_midnight))
    }

    /// The wait from `since_midnight` to the next occurrence of this time.
    ///
    /// At the exact time, the next occurrence is one day later, so a run never repeats at once.
    fn delay_from(self, since_midnight: Duration) -> Duration {
        let day = Duration::from_secs(DAY_SECONDS);
        match self.0.checked_sub(since_midnight) {
            Some(wait) if !wait.is_zero() => wait,
            // `since_midnight` is below one day, so these steps cannot overflow. The fallback
            // of a full day is the safe answer if they ever do.
            _ => day
                .checked_add(self.0)
                .and_then(|next| next.checked_sub(since_midnight))
                .unwrap_or(day),
        }
    }
}

/// The two times that a schedule needs to pick the next wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    since_start: Duration,
    since_midnight: Duration,
}

impl Moment {
    /// Make a moment.
    ///
    /// `since_start` is the time since the supervisor started the job. `unix_seconds` and
    /// `subsecond_nanos` are the wall clock in UTC. The constructor keeps only the time of day.
    #[must_use]
    pub fn new(since_start: Duration, unix_seconds: i64, subsecond_nanos: u32) -> Self {
        // `rem_euclid` of a positive number is never negative. The fallback of midnight cannot
        // occur and only avoids a panic path.
        let seconds = u64::try_from(unix_seconds.rem_euclid(DAY_SECONDS_SIGNED)).unwrap_or(0);
        let nanos = subsecond_nanos.min(MAX_SUBSECOND_NANOS);
        Self {
            since_start,
            since_midnight: Duration::new(seconds, nanos),
        }
    }
}

/// One rule of a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    /// Fire once, `at` after the start of the supervisor.
    Once { at: Duration, fired: bool },
    /// Fire one interval after the end of the last run.
    Every(Duration),
    /// Fire at the next occurrence of a time of day in UTC.
    ///
    /// The trigger keeps the due occurrence as an offset `due` since the start. An occurrence that
    /// passes while another trigger runs is overdue and fires at once after that run. Several
    /// passed occurrences give one run.
    ///
    /// `fire` clears `due` when this trigger fires. The schedule arms the trigger again after the
    /// run. A run of this trigger that lasts 24 hours or more skips the occurrence that passes
    /// during it.
    Daily {
        time: TimeOfDay,
        due: Option<Duration>,
    },
}

impl Trigger {
    /// Fix the due occurrence of a `Daily` trigger that has none.
    fn arm(&mut self, now: Moment) {
        if let Self::Daily { time, due } = self
            && due.is_none()
        {
            let wait = time.delay_from(now.since_midnight);
            // A process cannot run long enough for the sum to overflow `Duration`.
            *due = Some(now.since_start.checked_add(wait).unwrap_or(Duration::MAX));
        }
    }

    /// Forget the due occurrence of a `Daily` trigger and spend a `Once` trigger.
    fn fire(&mut self) {
        match self {
            Self::Once { fired, .. } => *fired = true,
            Self::Daily { due, .. } => *due = None,
            Self::Every(_) => {}
        }
    }

    /// The wait until this trigger fires, or `None` when it fired for good.
    fn delay(&self, now: Moment) -> Option<Duration> {
        match self {
            // An overdue trigger fires at once, so its wait is zero.
            Self::Once { at, fired } => {
                (!fired).then(|| at.checked_sub(now.since_start).unwrap_or(Duration::ZERO))
            }
            Self::Every(interval) => Some(*interval),
            Self::Daily { due, .. } => {
                let due = due.unwrap_or(now.since_start);
                Some(due.checked_sub(now.since_start).unwrap_or(Duration::ZERO))
            }
        }
    }

    /// The kind of the run that this trigger fires.
    const fn kind(&self) -> RunKind {
        match self {
            Self::Once { .. } => RunKind::CatchUp,
            Self::Every(_) | Self::Daily { .. } => RunKind::Scheduled,
        }
    }
}

/// The schedule of one job.
///
/// The type solves one problem: the intervals, the nightly time, and the catch-up of the jobs
/// follow different rules, and the supervisor must run them in one loop without overlap. A
/// schedule keeps which one-shot trigger already fired, so the supervisor asks it for the next
/// wait before the first run and after every run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schedule {
    triggers: Vec<Trigger>,
}

impl Schedule {
    /// Run one interval after the start, then one interval after the end of each run.
    ///
    /// # Errors
    ///
    /// Returns `ScheduleFault::ZeroInterval` for an interval of zero.
    pub fn every(interval: Duration) -> Result<Self, ScheduleFault> {
        if interval.is_zero() {
            return Err(ScheduleFault::ZeroInterval);
        }
        Ok(Self {
            triggers: vec![Trigger::Every(interval)],
        })
    }

    /// Run at once, then one interval after the end of each run.
    ///
    /// # Errors
    ///
    /// Returns `ScheduleFault::ZeroInterval` for an interval of zero.
    pub fn now_then_every(interval: Duration) -> Result<Self, ScheduleFault> {
        Ok(Self::every(interval)?.with_once_after(Duration::ZERO))
    }

    /// Run every day at a time of day in UTC.
    #[must_use]
    pub fn daily_at(time: TimeOfDay) -> Self {
        Self {
            triggers: vec![Trigger::Daily { time, due: None }],
        }
    }

    /// Run one time, `delay` after the start.
    #[must_use]
    pub fn once_after(delay: Duration) -> Self {
        Self::default().with_once_after(delay)
    }

    /// Add one run, `delay` after the start, to this schedule.
    ///
    /// The run shares the loop of the other triggers. It waits for an open run to end.
    #[must_use]
    pub fn with_once_after(mut self, delay: Duration) -> Self {
        self.triggers.push(Trigger::Once {
            at: delay,
            fired: false,
        });
        self
    }

    /// The wait before the next run and the kind of that run, or `None` when no trigger can fire
    /// again.
    ///
    /// The caller calls this function before the first wait and after each run ends. The
    /// trigger with the shortest wait wins. A one-shot trigger that wins is spent and gives
    /// `RunKind::CatchUp`.
    /// When a one-shot trigger and another trigger have the same wait, the other trigger wins and
    /// the run is `RunKind::Scheduled`. The one-shot trigger then stays pending.
    ///
    /// A daily trigger keeps its due occurrence until it wins. Two calls without a win of the
    /// daily trigger therefore leave the occurrence unchanged. An overdue occurrence has the
    /// wait zero.
    pub fn next_delay(&mut self, now: Moment) -> Option<(Duration, RunKind)> {
        for trigger in &mut self.triggers {
            trigger.arm(now);
        }
        let (index, delay, kind) = self
            .triggers
            .iter()
            .enumerate()
            .filter_map(|(index, trigger)| {
                trigger
                    .delay(now)
                    .map(|delay| (index, delay, trigger.kind()))
            })
            .min_by_key(|&(_, delay, kind)| (delay, kind == RunKind::CatchUp))?;
        if let Some(trigger) = self.triggers.get_mut(index) {
            trigger.fire();
        }
        Some((delay, kind))
    }
}

#[cfg(test)]
mod tests;
