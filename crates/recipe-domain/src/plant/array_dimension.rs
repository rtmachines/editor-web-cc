//! Dimensions of an `ARRAY` type (`domain-model.md` section 2.1, Assumption A38).

use std::fmt;

use super::limits::MAX_DIMENSIONS;

/// Largest number of elements of one dimension: `upper - lower < 65536`.
const MAX_SPAN: i64 = 65_536;

/// One dimension of an array, such as `1..8` (`ARRAY[1..8] OF INT`).
///
/// The rules are `lower <= upper` and `upper - lower < 65536`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayDimension {
    lower: i32,
    upper: i32,
    count: u32,
}

/// Reason why a dimension or a list of dimensions is not valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayDimensionReason {
    /// The lower bound is greater than the upper bound.
    Order,
    /// The dimension has 65536 or more elements.
    Span,
    /// The list has no dimension or more than three dimensions.
    Count,
}

impl fmt::Display for ArrayDimensionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Order => f.write_str("the lower bound is greater than the upper bound"),
            Self::Span => write!(f, "a dimension has {MAX_SPAN} or more elements"),
            Self::Count => write!(f, "an array has 1 to {MAX_DIMENSIONS} dimensions"),
        }
    }
}

impl ArrayDimension {
    /// Parse one dimension from its bounds.
    ///
    /// # Errors
    ///
    /// Returns [`ArrayDimensionReason::Order`] when `lower > upper`.
    /// Returns [`ArrayDimensionReason::Span`] when the dimension has 65536 or more elements.
    pub fn parse(lower: i32, upper: i32) -> Result<Self, ArrayDimensionReason> {
        let span = i64::from(upper)
            .checked_sub(i64::from(lower))
            .ok_or(ArrayDimensionReason::Span)?;
        if span < 0 {
            return Err(ArrayDimensionReason::Order);
        }
        if span >= MAX_SPAN {
            return Err(ArrayDimensionReason::Span);
        }
        let count = span
            .checked_add(1)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(ArrayDimensionReason::Span)?;
        Ok(Self {
            lower,
            upper,
            count,
        })
    }

    /// Return the lower bound.
    #[must_use]
    pub const fn lower(self) -> i32 {
        self.lower
    }

    /// Return the upper bound.
    #[must_use]
    pub const fn upper(self) -> i32 {
        self.upper
    }

    /// Return the number of elements, from 1 to 65536.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.count
    }
}

impl fmt::Display for ArrayDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.lower, self.upper)
    }
}

/// The dimensions of an array, from one to three (Assumption A38).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayDimensions(Vec<ArrayDimension>);

impl ArrayDimensions {
    /// Parse the dimensions of an array.
    ///
    /// # Errors
    ///
    /// Returns [`ArrayDimensionReason::Count`] for zero dimensions or more than three.
    pub fn parse(dimensions: Vec<ArrayDimension>) -> Result<Self, ArrayDimensionReason> {
        if dimensions.is_empty() || dimensions.len() > MAX_DIMENSIONS {
            return Err(ArrayDimensionReason::Count);
        }
        Ok(Self(dimensions))
    }

    /// Iterate over the dimensions in their order.
    pub fn iter(&self) -> std::slice::Iter<'_, ArrayDimension> {
        self.0.iter()
    }

    /// Return the number of dimensions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return `true` when the list has no dimension. A parsed list is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return the number of elements of the array, the product of the counts.
    ///
    /// The result is `None` only when the product overflows `u64`. Three
    /// dimensions of at most 65536 elements cannot overflow it.
    #[must_use]
    pub fn element_count(&self) -> Option<u64> {
        self.0.iter().try_fold(1_u64, |product, dimension| {
            product.checked_mul(u64::from(dimension.count()))
        })
    }
}

impl fmt::Display for ArrayDimensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        f.write_str(&text.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dimension(lower: i32, upper: i32) -> ArrayDimension {
        ArrayDimension::parse(lower, upper).expect("test dimension is valid")
    }

    #[test]
    fn reversed_bounds_fail_with_order() {
        assert_eq!(
            ArrayDimension::parse(5, 1),
            Err(ArrayDimensionReason::Order)
        );
    }

    #[test]
    fn span_of_65536_fails_with_span() {
        assert_eq!(
            ArrayDimension::parse(0, 65_536),
            Err(ArrayDimensionReason::Span)
        );
    }

    #[test]
    fn span_of_65535_passes_with_65536_elements() {
        let parsed = ArrayDimension::parse(0, 65_535);
        assert_eq!(parsed.map(ArrayDimension::count), Ok(65_536));
    }

    #[test]
    fn negative_span_of_65535_passes() {
        let parsed = ArrayDimension::parse(-2_147_483_648, -2_147_418_113);
        assert_eq!(parsed.map(ArrayDimension::count), Ok(65_536));
    }

    #[test]
    fn full_i32_range_fails_with_span_without_overflow() {
        assert_eq!(
            ArrayDimension::parse(-2_147_483_648, 2_147_483_647),
            Err(ArrayDimensionReason::Span)
        );
    }

    #[test]
    fn single_element_dimension_has_one_element() {
        assert_eq!(
            ArrayDimension::parse(7, 7).map(ArrayDimension::count),
            Ok(1)
        );
    }

    #[test]
    fn bounds_are_returned_unchanged() {
        let parsed = ArrayDimension::parse(1, 8);
        assert_eq!(parsed.map(ArrayDimension::lower), Ok(1));
        assert_eq!(parsed.map(ArrayDimension::upper), Ok(8));
    }

    #[test]
    fn dimension_displays_its_bounds() {
        assert_eq!(
            ArrayDimension::parse(1, 8).map(|d| d.to_string()),
            Ok("1..8".to_owned())
        );
    }

    #[test]
    fn one_to_three_dimensions_pass() {
        for count in 1..=3 {
            let list = vec![dimension(1, 2); count];
            assert!(ArrayDimensions::parse(list).is_ok(), "{count}");
        }
    }

    #[test]
    fn zero_dimensions_fail_with_count() {
        assert_eq!(
            ArrayDimensions::parse(Vec::new()),
            Err(ArrayDimensionReason::Count)
        );
    }

    #[test]
    fn four_dimensions_fail_with_count() {
        let list = vec![dimension(1, 2); 4];
        assert_eq!(
            ArrayDimensions::parse(list),
            Err(ArrayDimensionReason::Count)
        );
    }

    #[test]
    fn two_dimensions_have_six_elements() {
        let list = vec![dimension(1, 2), dimension(1, 3)];
        let dimensions = ArrayDimensions::parse(list);
        assert_eq!(
            dimensions.as_ref().map(ArrayDimensions::element_count),
            Ok(Some(6))
        );
        assert_eq!(dimensions.as_ref().map(ArrayDimensions::len), Ok(2));
    }

    #[test]
    fn three_maximal_dimensions_have_two_to_the_48_elements() {
        let list = vec![dimension(0, 65_535); 3];
        let dimensions = ArrayDimensions::parse(list);
        assert_eq!(
            dimensions.as_ref().map(ArrayDimensions::element_count),
            Ok(Some(281_474_976_710_656))
        );
    }

    #[test]
    fn dimensions_display_with_commas() {
        let list = vec![dimension(1, 2), dimension(1, 3)];
        let dimensions = ArrayDimensions::parse(list);
        assert_eq!(
            dimensions.map(|d| d.to_string()),
            Ok("1..2,1..3".to_owned())
        );
    }

    #[test]
    fn iter_yields_the_dimensions_in_order() {
        let list = vec![dimension(1, 2), dimension(5, 9)];
        let dimensions = ArrayDimensions::parse(list.clone());
        let yielded: Vec<ArrayDimension> = dimensions
            .as_ref()
            .map_or(Vec::new(), |d| d.iter().copied().collect());
        assert_eq!(yielded, list);
    }
}
