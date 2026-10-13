//! Instant value object of the domain.

use time::OffsetDateTime;

/// An instant of the domain, stored with its UTC offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(OffsetDateTime);

impl From<OffsetDateTime> for Timestamp {
    fn from(value: OffsetDateTime) -> Self {
        Self(value)
    }
}

impl Timestamp {
    /// Return the instant of this value.
    #[must_use]
    pub const fn as_offset_date_time(&self) -> &OffsetDateTime {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn timestamp_round_trips_an_offset_date_time() {
        let instant = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(Timestamp::from(instant).as_offset_date_time(), &instant);
    }
}
