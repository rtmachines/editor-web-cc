//! The opener of transactions (design D3).

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{Isolation, Transaction, UnitOfWork};
use sqlx::PgPool;

use crate::error::map_error;
use crate::transaction::PgTransaction;

/// Return the `BEGIN` statement of an isolation.
///
/// The function solves one problem: the isolation of a command is a rule of its row in
/// `data-model.md` section 6, so the first statement of the transaction must carry it.
/// `ReadOnly` runs at `READ COMMITTED` and rejects every write. `RepeatableRead` gives every
/// statement of a draft query one snapshot and also rejects every write (ADR-0042).
const fn begin_statement(isolation: Isolation) -> &'static str {
    match isolation {
        Isolation::ReadCommitted => "BEGIN ISOLATION LEVEL READ COMMITTED",
        Isolation::ReadOnly => "BEGIN ISOLATION LEVEL READ COMMITTED READ ONLY",
        Isolation::Serializable => "BEGIN ISOLATION LEVEL SERIALIZABLE",
        Isolation::RepeatableRead => "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
    }
}

/// The `UnitOfWork` of `recipe-db`. It opens one transaction on one pooled connection.
pub(crate) struct PgUnitOfWork {
    pool: PgPool,
}

impl PgUnitOfWork {
    pub(crate) const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWork for PgUnitOfWork {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        let tx = self
            .pool
            .begin_with(begin_statement(isolation))
            .await
            .map_err(map_error)?;
        Ok(Box::new(PgTransaction::new(tx)))
    }
}

#[cfg(test)]
mod tests {
    use recipe_app::ports::Isolation;

    use super::begin_statement;

    #[test]
    fn each_isolation_begins_with_its_level_and_its_access_mode() {
        let expected = [
            (
                Isolation::ReadCommitted,
                "BEGIN ISOLATION LEVEL READ COMMITTED",
            ),
            (
                Isolation::ReadOnly,
                "BEGIN ISOLATION LEVEL READ COMMITTED READ ONLY",
            ),
            (
                Isolation::Serializable,
                "BEGIN ISOLATION LEVEL SERIALIZABLE",
            ),
            (
                Isolation::RepeatableRead,
                "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
            ),
        ];
        for (isolation, statement) in expected {
            assert_eq!(begin_statement(isolation), statement, "{isolation:?}");
        }
    }
}
