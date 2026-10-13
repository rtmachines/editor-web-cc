//! Identifiers of the Recipe Management context (`domain-model.md` section 2.2).

use std::fmt;

use uuid::Uuid;

/// Stable identity of a recipe (domain-model.md section 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecipeId(Uuid);

impl From<Uuid> for RecipeId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl RecipeId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for RecipeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable identity of a recipe version (domain-model.md section 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecipeVersionId(Uuid);

impl From<Uuid> for RecipeVersionId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl RecipeVersionId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for RecipeVersionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable identity of a parameter (domain-model.md section 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(Uuid);

impl From<Uuid> for ParameterId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl ParameterId {
    /// Return the UUID of this identifier.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for ParameterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Key of a parameter in a URL (domain-model.md section 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterKey(Uuid);

/// Reason why a text is not a parameter key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKeyReason {
    /// The text is not a UUID.
    Format,
}

impl fmt::Display for ParameterKeyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str("a parameter key is a UUID"),
        }
    }
}

impl From<Uuid> for ParameterKey {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl ParameterKey {
    /// Parse a parameter key from the text of a URL.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterKeyReason::Format`] when the text is not a UUID.
    pub fn parse(text: &str) -> Result<Self, ParameterKeyReason> {
        Uuid::parse_str(text)
            .map(Self)
            .map_err(|_| ParameterKeyReason::Format)
    }

    /// Return the UUID of this key.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for ParameterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn recipe_id_from_uuid_keeps_the_uuid() {
        let raw = Uuid::from_u128(101);
        assert_eq!(RecipeId::from(raw).as_uuid(), &raw);
    }

    #[test]
    fn recipe_id_display_is_the_hyphenated_uuid() {
        let raw = Uuid::from_u128(102);
        assert_eq!(RecipeId::from(raw).to_string(), raw.to_string());
    }

    #[test]
    fn recipe_version_id_from_uuid_keeps_the_uuid() {
        let raw = Uuid::from_u128(103);
        assert_eq!(RecipeVersionId::from(raw).as_uuid(), &raw);
    }

    #[test]
    fn recipe_version_id_display_is_the_hyphenated_uuid() {
        let raw = Uuid::from_u128(104);
        assert_eq!(RecipeVersionId::from(raw).to_string(), raw.to_string());
    }

    #[test]
    fn parameter_id_from_uuid_keeps_the_uuid() {
        let raw = Uuid::from_u128(105);
        assert_eq!(ParameterId::from(raw).as_uuid(), &raw);
    }

    #[test]
    fn parameter_id_display_is_the_hyphenated_uuid() {
        let raw = Uuid::from_u128(106);
        assert_eq!(ParameterId::from(raw).to_string(), raw.to_string());
    }

    #[test]
    fn parameter_key_from_uuid_keeps_the_uuid() {
        let raw = Uuid::from_u128(107);
        assert_eq!(ParameterKey::from(raw).as_uuid(), &raw);
    }

    #[test]
    fn parameter_key_display_is_the_hyphenated_uuid() {
        let raw = Uuid::from_u128(108);
        assert_eq!(ParameterKey::from(raw).to_string(), raw.to_string());
    }

    #[test]
    fn parameter_key_parse_of_a_uuid_text_keeps_the_uuid() {
        let raw = Uuid::from_u128(109);
        let parsed = ParameterKey::parse(&raw.to_string());
        assert_eq!(parsed.map(|key| *key.as_uuid()), Ok(raw));
    }

    #[test]
    fn parameter_key_parse_of_not_a_uuid_gives_format() {
        assert_eq!(
            ParameterKey::parse("not-a-uuid"),
            Err(ParameterKeyReason::Format)
        );
    }
}
