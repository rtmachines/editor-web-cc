//! Tests of step 6: the failure paths of the build of the Argon2id hasher.

use std::sync::Arc;

use anyhow::anyhow;
use recipe_app::error::{AppError, TaskStage};
use recipe_app::identity::PasswordHasher;

use super::build_hasher;
use crate::serve::host::HasherBuild;

/// A refused build. The error comes from a call, so no lint sees a literal `Err`.
fn refused() -> Result<Arc<dyn PasswordHasher>, String> {
    Err("the build refuses".to_owned())
}

/// A build that panics. `clippy::panic` also applies in tests, so `expect` starts the panic.
fn panicking_build() -> Arc<dyn PasswordHasher> {
    refused().expect("the build must panic")
}

#[tokio::test]
async fn a_panic_in_the_hasher_build_gives_the_stage_password_hash() {
    let build: HasherBuild = Box::new(|| Ok(panicking_build()));

    let error = build_hasher(build)
        .await
        .err()
        .expect("the panic must fail the build");

    assert!(
        matches!(
            error.downcast_ref::<AppError>(),
            Some(AppError::TaskFailed {
                stage: TaskStage::PasswordHash
            })
        ),
        "{error:#}"
    );
}

#[tokio::test]
async fn a_refusal_of_the_hasher_build_keeps_its_cause_and_is_no_task_failure() {
    let build: HasherBuild = Box::new(|| Err(anyhow!("the memory cost is out of range")));

    let error = build_hasher(build)
        .await
        .err()
        .expect("the refusal must fail the build");

    assert!(error.downcast_ref::<AppError>().is_none(), "{error:#}");
    assert_eq!(
        format!("{error:#}"),
        "the password hasher cannot start: the memory cost is out of range"
    );
}
