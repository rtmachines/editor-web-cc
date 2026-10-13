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
mod tests {
    use crate::error_codes::ErrorCode;
    use crate::recipe::{Budget, RecipeError};

    use super::check_budgets;
    use super::{BudgetCounts, LEAVES_PER_VERSION, LIMIT_ROWS_PER_VERSION, PARAMETERS_PER_VERSION};

    fn counts(parameters: u64, leaves: u64, limit_rows: u64) -> BudgetCounts {
        BudgetCounts {
            parameters,
            leaves,
            limit_rows,
        }
    }

    fn exceeded(input: BudgetCounts) -> Option<(Budget, u64, u64)> {
        match check_budgets(input) {
            Err(RecipeError::SizeBudgetExceeded {
                budget,
                limit,
                actual,
            }) => Some((budget, limit, actual)),
            _ => None,
        }
    }

    #[test]
    fn counts_at_the_limits_pass_together() {
        let input = counts(
            PARAMETERS_PER_VERSION,
            LEAVES_PER_VERSION,
            LIMIT_ROWS_PER_VERSION,
        );
        assert!(check_budgets(input).is_ok());
    }

    #[test]
    fn one_parameter_over_the_limit_gives_parameters_budget() {
        let actual = PARAMETERS_PER_VERSION.checked_add(1).unwrap();
        let input = counts(actual, 0, 0);
        assert_eq!(
            exceeded(input),
            Some((Budget::Parameters, PARAMETERS_PER_VERSION, actual))
        );
    }

    #[test]
    fn one_leaf_over_the_limit_gives_leaves_budget() {
        let actual = LEAVES_PER_VERSION.checked_add(1).unwrap();
        let input = counts(0, actual, 0);
        assert_eq!(
            exceeded(input),
            Some((Budget::Leaves, LEAVES_PER_VERSION, actual))
        );
    }

    #[test]
    fn one_limit_row_over_the_limit_gives_limit_rows_budget() {
        let actual = LIMIT_ROWS_PER_VERSION.checked_add(1).unwrap();
        let input = counts(0, 0, actual);
        assert_eq!(
            exceeded(input),
            Some((Budget::LimitRows, LIMIT_ROWS_PER_VERSION, actual))
        );
    }

    #[test]
    fn parameters_budget_wins_when_two_counts_are_over() {
        let parameters = PARAMETERS_PER_VERSION.checked_add(1).unwrap();
        let leaves = LEAVES_PER_VERSION.checked_add(1).unwrap();
        let input = counts(parameters, leaves, 0);
        assert_eq!(
            exceeded(input),
            Some((Budget::Parameters, PARAMETERS_PER_VERSION, parameters))
        );
    }

    #[test]
    fn leaves_budget_wins_over_limit_rows() {
        let leaves = LEAVES_PER_VERSION.checked_add(1).unwrap();
        let limit_rows = LIMIT_ROWS_PER_VERSION.checked_add(1).unwrap();
        let input = counts(0, leaves, limit_rows);
        assert_eq!(
            exceeded(input),
            Some((Budget::Leaves, LEAVES_PER_VERSION, leaves))
        );
    }

    #[test]
    fn budget_error_has_code_e_rcp_021_and_its_message() {
        let actual = PARAMETERS_PER_VERSION.checked_add(1).unwrap();
        let report = check_budgets(counts(actual, 0, 0))
            .err()
            .map(|error| (error.code(), error.to_string()));
        assert_eq!(
            report,
            Some((
                ErrorCode::RCP_021,
                "The version exceeds the limit of 4000 parameters. Current count: 4001.".to_owned()
            ))
        );
    }
}
