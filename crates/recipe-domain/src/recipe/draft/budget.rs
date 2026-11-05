//! The size budgets of a version (`reliability.md` section 7.11, design D11).

use crate::recipe::{Budget, RecipeError};

/// The largest number of parameters in one version (`reliability.md` section 7.11).
pub const PARAMETERS_PER_VERSION: u64 = 4_000;

/// The largest number of scalar leaves in one version (`reliability.md` section 7.11).
pub const LEAVES_PER_VERSION: u64 = 262_144;

/// The largest number of limit rows in one version (`reliability.md` section 7.11).
pub const LIMIT_ROWS_PER_VERSION: u64 = 32_768;

/// The counts of a version that the size budgets check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BudgetCounts {
    /// The number of parameters of the version.
    pub parameters: u64,
    /// The number of scalar leaves of the version.
    pub leaves: u64,
    /// The number of limit rows of the version.
    pub limit_rows: u64,
}

/// Check the counts of a version against the size budgets.
///
/// The check runs in the order parameters, leaves, and limit rows.
/// The first count above its limit gives `E-RCP-021`. A count equal to its limit passes.
///
/// # Errors
///
/// Return [`RecipeError::SizeBudgetExceeded`] for the first count above its limit.
pub const fn check_budgets(counts: BudgetCounts) -> Result<(), RecipeError> {
    if counts.parameters > PARAMETERS_PER_VERSION {
        return Err(exceeded(
            Budget::Parameters,
            PARAMETERS_PER_VERSION,
            counts.parameters,
        ));
    }
    if counts.leaves > LEAVES_PER_VERSION {
        return Err(exceeded(Budget::Leaves, LEAVES_PER_VERSION, counts.leaves));
    }
    if counts.limit_rows > LIMIT_ROWS_PER_VERSION {
        return Err(exceeded(
            Budget::LimitRows,
            LIMIT_ROWS_PER_VERSION,
            counts.limit_rows,
        ));
    }
    Ok(())
}

/// Build the budget error of one count.
const fn exceeded(budget: Budget, limit: u64, actual: u64) -> RecipeError {
    RecipeError::SizeBudgetExceeded {
        budget,
        limit,
        actual,
    }
}

#[cfg(test)]
mod tests;
