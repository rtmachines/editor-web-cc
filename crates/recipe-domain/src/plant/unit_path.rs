//! Unit path of the Plant Model context (`domain-model.md` section 2.1).

use std::fmt;

use super::PlantError;

/// Largest number of segments of a unit path.
const MAX_SEGMENTS: usize = 5;

/// Largest value of one segment of a unit path.
const MAX_SEGMENT: u8 = 64;

/// Parse one segment: ASCII digits, no leading zero, value `1..=64`.
fn parse_segment(text: &str) -> Option<u8> {
    let well_formed = (1..=2).contains(&text.len())
        && text.bytes().all(|b| b.is_ascii_digit())
        && !text.starts_with('0');
    if !well_formed {
        return None;
    }
    text.parse::<u8>()
        .ok()
        .filter(|number| (1..=MAX_SEGMENT).contains(number))
}

/// Hierarchical path of a unit, such as `U1.2.4` (`domain-model.md` section 2.1).
///
/// The grammar is `U` followed by 1 to 5 segments. Each segment is a number
/// in `1..=64`, and the segments are joined by dots.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitPath {
    segments: Vec<u8>,
}

impl UnitPath {
    /// Parse a unit path such as `U1.2.4`.
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::InvalidUnitPath`] when the text or a segment is malformed.
    /// Returns [`PlantError::UnitDepthExceeded`] when a valid path has more than five segments.
    pub fn parse(text: &str) -> Result<Self, PlantError> {
        let invalid = || PlantError::InvalidUnitPath {
            text: text.to_owned(),
        };
        let body = text.strip_prefix('U').ok_or_else(invalid)?;
        let segments = body
            .split('.')
            .map(parse_segment)
            .collect::<Option<Vec<u8>>>()
            .ok_or_else(invalid)?;
        if segments.len() > MAX_SEGMENTS {
            return Err(PlantError::UnitDepthExceeded {
                path: text.to_owned(),
            });
        }
        Ok(Self { segments })
    }

    /// Parse the path of a new unit from its parent and one segment.
    ///
    /// Without a parent the new path has one segment. The check of one segment
    /// stops a text such as `1.2` from adding a deeper path.
    ///
    /// # Errors
    ///
    /// Returns [`PlantError::InvalidUnitPath`] with the joined text when `segment`
    /// is not exactly one valid segment.
    /// Returns [`PlantError::UnitDepthExceeded`] when the new path has six segments.
    pub(crate) fn child(parent: Option<&UnitPath>, segment: &str) -> Result<Self, PlantError> {
        let joined = match parent {
            Some(parent) => format!("{parent}.{segment}"),
            None => format!("U{segment}"),
        };
        if parse_segment(segment).is_none() {
            return Err(PlantError::InvalidUnitPath { text: joined });
        }
        Self::parse(&joined)
    }

    /// Return the path of the first main unit, `U1`.
    #[must_use]
    pub(crate) fn first_main() -> Self {
        Self { segments: vec![1] }
    }

    /// Return the parent path, or `None` for a path with one segment.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let (_, rest) = self.segments.split_last()?;
        if rest.is_empty() {
            return None;
        }
        Some(Self {
            segments: rest.to_vec(),
        })
    }

    /// Return the number of segments, from 1 to 5.
    #[must_use]
    pub fn depth(&self) -> u8 {
        u8::try_from(self.segments.len()).unwrap_or(u8::MAX)
    }

    /// Return the segments in order.
    pub fn segments(&self) -> impl Iterator<Item = u8> + '_ {
        self.segments.iter().copied()
    }

    /// Return true when `other` is a strict descendant of this path.
    #[must_use]
    pub fn is_ancestor_of(&self, other: &UnitPath) -> bool {
        other.segments.len() > self.segments.len() && other.segments.starts_with(&self.segments)
    }

    /// Return true when this path equals `ancestor` or lies below it.
    ///
    /// The function compares whole segments and not text. This stops `U12` from
    /// matching the scope `U1`. It solves the prefix error of a text comparison
    /// in the unit filter of a scoped export (ADR-0109).
    #[must_use]
    pub fn is_self_or_descendant_of(&self, ancestor: &UnitPath) -> bool {
        self.segments.starts_with(&ancestor.segments)
    }
}

impl fmt::Display for UnitPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined = self
            .segments
            .iter()
            .map(u8::to_string)
            .collect::<Vec<String>>()
            .join(".");
        write!(f, "U{joined}")
    }
}

#[cfg(test)]
mod tests;
