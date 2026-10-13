//! Size limits of the Plant Model (`reliability.md` section 7.11).

/// Largest number of members of one struct (`type.max_members`).
pub const MAX_MEMBERS: usize = 1024;

/// Largest number of scalar leaves of one parameter (`budget.leaves_per_parameter`).
pub const MAX_LEAVES: u32 = 16_384;

/// Largest number of JSON levels of one value (`json.max_depth`).
pub const MAX_DEPTH: u32 = 32;

/// Largest number of dimensions of an array and of indexes in one index segment
/// (`type.max_dimensions`, `reliability.md` section 7.11, Assumption A38).
pub const MAX_DIMENSIONS: usize = 3;
