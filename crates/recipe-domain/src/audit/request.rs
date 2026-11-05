//! Request identifier and User-Agent value of the audit context (`data-model.md` 3.8).

use std::fmt;

use uuid::Uuid;

/// Number of characters that a User-Agent header keeps (`http.user_agent_max`, `reliability.md` 7.11).
const USER_AGENT_MAX_CHARS: usize = 256;

/// Identifier of one HTTP request. The audit entries and the security events store it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(Uuid);

impl RequestId {
    /// Return the UUID of the request.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl From<Uuid> for RequestId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The User-Agent header of a request, cut to its first 256 characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserAgent(String);

impl UserAgent {
    /// Return the kept text of the header.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for UserAgent {
    fn from(value: &str) -> Self {
        Self(value.chars().take(USER_AGENT_MAX_CHARS).collect())
    }
}

#[cfg(test)]
mod tests;
