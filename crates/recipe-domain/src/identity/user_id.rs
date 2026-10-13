//! Identifier of a user (`reliability.md` section 1.3, design D2).

use std::fmt;

use uuid::Uuid;

/// Stable identity of a user (`reliability.md` section 1.3, design D2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(Uuid);

impl From<Uuid> for UserId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl UserId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn user_id_from_uuid_keeps_the_uuid() {
        let raw = Uuid::from_u128(201);
        assert_eq!(UserId::from(raw).as_uuid(), &raw);
    }

    #[test]
    fn user_id_display_is_the_hyphenated_uuid() {
        let raw = Uuid::from_u128(202);
        assert_eq!(UserId::from(raw).to_string(), raw.to_string());
    }
}
