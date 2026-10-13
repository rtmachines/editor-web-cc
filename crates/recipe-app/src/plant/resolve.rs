//! Step 2 of design D10 for the commands that start from a machine code.

use recipe_domain::plant::{Machine, MachineCode, MachineId};

use crate::context::AppContext;
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked};

/// Accept the locked machine only when it is live.
///
/// The function solves one problem: the URL step resolves a live machine, but a concurrent
/// command can archive it before the lock. The command then gives the result of the URL
/// step, `E-WEB-005` (`api.md` section 2.2). Commands whose row lists `E-PLT-010` do not use it.
///
/// # Errors
///
/// Returns `AppError::NotFound` (`E-WEB-005`) when the lock found no machine or an archived one.
pub(super) fn require_live(
    locked: Option<Locked<Machine>>,
    code: MachineCode,
) -> Result<Locked<Machine>, AppError> {
    locked
        .filter(|locked| locked.state.is_live())
        .ok_or(AppError::NotFound {
            entity: Missing::Machine(code),
        })
}

/// Resolve the live machine with `code` in a `ReadOnly` transaction.
///
/// The function solves one problem: a URL names a machine by its code, but a lock needs its
/// identifier. The transaction takes no lock and ends before the function returns. The caller
/// never runs a domain method on the state of this read (design D10).
///
/// # Errors
///
/// Returns `AppError::NotFound` (`E-WEB-005`) when no live machine has the code.
/// Returns `AppError::Storage` when the read fails.
pub(super) async fn live_machine_id(
    ctx: &AppContext,
    code: MachineCode,
) -> Result<MachineId, AppError> {
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    let machine = tx.plant().machine_by_code(code).await?;
    machine
        .map(|machine| machine.id())
        .ok_or(AppError::NotFound {
            entity: Missing::Machine(code),
        })
}
