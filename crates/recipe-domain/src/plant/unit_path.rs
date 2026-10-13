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
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn parse_ok(text: &str) -> UnitPath {
        UnitPath::parse(text).unwrap()
    }

    /// Return the rejected text when `text` gives `InvalidUnitPath`.
    fn invalid_text(text: &str) -> Option<String> {
        match UnitPath::parse(text) {
            Err(PlantError::InvalidUnitPath { text }) => Some(text),
            _ => None,
        }
    }

    #[test]
    fn valid_paths_pass_and_display_equals_the_input() {
        for text in ["U1", "U64", "U1.2.4.5.6", "U64.64.64.64.64", "U1.2.4"] {
            assert_eq!(parse_ok(text).to_string(), text);
        }
    }

    #[test]
    fn segment_above_64_gives_invalid_text() {
        assert_eq!(invalid_text("U65"), Some("U65".to_owned()));
        assert_eq!(invalid_text("U1.65"), Some("U1.65".to_owned()));
    }

    #[test]
    fn malformed_texts_give_invalid_text() {
        for text in [
            "U0", "U01", "U1..2", "U1.", "1.2", "u1", "U", "U1.2a", "U 1", "",
        ] {
            assert_eq!(invalid_text(text), Some(text.to_owned()));
        }
    }

    #[test]
    fn six_valid_segments_give_depth_error() {
        assert!(matches!(
            UnitPath::parse("U1.2.3.4.5.6"),
            Err(PlantError::UnitDepthExceeded { path }) if path == "U1.2.3.4.5.6"
        ));
    }

    #[test]
    fn six_segments_with_a_bad_last_segment_give_invalid_text() {
        assert_eq!(
            invalid_text("U1.2.3.4.5.65"),
            Some("U1.2.3.4.5.65".to_owned())
        );
    }

    #[test]
    fn parent_of_three_segments_is_two_segments() {
        assert_eq!(parse_ok("U1.2.4").parent(), Some(parse_ok("U1.2")));
    }

    #[test]
    fn parent_of_one_segment_is_none() {
        assert_eq!(parse_ok("U1").parent(), None);
    }

    #[test]
    fn depth_counts_the_segments() {
        assert_eq!(parse_ok("U1").depth(), 1);
        assert_eq!(parse_ok("U1.2.3.4.5").depth(), 5);
    }

    #[test]
    fn segments_iterate_in_order() {
        let values: Vec<u8> = parse_ok("U1.10.64").segments().collect();
        assert_eq!(values, vec![1, 10, 64]);
    }

    #[test]
    fn ancestor_is_a_strict_prefix() {
        let one = parse_ok("U1");
        assert!(one.is_ancestor_of(&parse_ok("U1.2.3")));
        assert!(!one.is_ancestor_of(&parse_ok("U12")));
        assert!(!one.is_ancestor_of(&one));
        assert!(!parse_ok("U1.2").is_ancestor_of(&parse_ok("U1.3.4")));
        assert!(!parse_ok("U1.2.3").is_ancestor_of(&parse_ok("U1.2")));
    }

    #[test]
    fn order_is_numeric_per_segment() {
        let mut paths = [
            parse_ok("U10"),
            parse_ok("U2"),
            parse_ok("U1.10"),
            parse_ok("U1.2"),
            parse_ok("U1"),
        ];
        paths.sort();
        let texts: Vec<String> = paths.iter().map(ToString::to_string).collect();
        assert_eq!(texts, vec!["U1", "U1.2", "U1.10", "U2", "U10"]);
    }

    #[test]
    fn first_main_is_u1() {
        assert_eq!(UnitPath::first_main().to_string(), "U1");
        assert_eq!(UnitPath::first_main(), parse_ok("U1"));
    }

    fn valid_segments(range: std::ops::RangeInclusive<usize>) -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(1u8..=64, range)
    }

    fn join_text(segments: &[u8]) -> String {
        let joined: Vec<String> = segments.iter().map(ToString::to_string).collect();
        format!("U{}", joined.join("."))
    }

    proptest! {
        #[test]
        fn valid_text_round_trips(segments in valid_segments(1..=5)) {
            let text = join_text(&segments);
            let path = UnitPath::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(path.to_string(), text);
            prop_assert_eq!(usize::from(path.depth()), segments.len());
        }

        #[test]
        fn six_to_eight_valid_segments_give_depth_error(segments in valid_segments(6..=8)) {
            let text = join_text(&segments);
            let depth_error = matches!(
                UnitPath::parse(&text),
                Err(PlantError::UnitDepthExceeded { .. })
            );
            prop_assert!(depth_error);
        }
    }
}
