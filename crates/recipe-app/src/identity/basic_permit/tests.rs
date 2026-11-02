//! Tests of the Basic permit (task 3.7, design D2 step 1, ADR-0109 (n)).

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Password, TypedLogin};
use uuid::Uuid;

use crate::auth::RequestMeta;
use crate::error::{AppError, TimeoutStage};
use crate::identity::fixtures::IdentityFixture;
use crate::identity::{Login, LoginInput};

const PASSWORD: &str = "correct-horse-battery";

fn login_input() -> LoginInput {
    LoginInput {
        login: TypedLogin::parse("anna").unwrap(),
        password: Password::parse(PASSWORD).unwrap(),
        client_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)),
        old_session: None,
    }
}

fn meta() -> RequestMeta {
    RequestMeta::new(RequestId::from(Uuid::from_u128(0x77)), None, None, None)
}

#[tokio::test(start_paused = true)]
async fn a_second_basic_check_times_out_after_the_acquire_timeout() {
    let fixture = IdentityFixture::new();
    let timeout = fixture.context.settings().argon2_acquire_timeout;
    let first = fixture.context.acquire_basic().await.unwrap();

    let started = tokio::time::Instant::now();
    let error = fixture.context.acquire_basic().await.err().unwrap();

    assert!(started.elapsed() >= timeout);
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Argon2Permit
        }
    ));
    drop(first);
}

#[tokio::test(start_paused = true)]
async fn a_dropped_basic_check_lets_the_next_basic_check_in() {
    let fixture = IdentityFixture::new();
    let first = fixture.context.acquire_basic().await.unwrap();

    drop(first);

    assert!(fixture.context.acquire_basic().await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn a_login_still_gets_an_argon2_permit_while_a_basic_check_holds_its_permits() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna(PASSWORD, false);
    let held = fixture.context.acquire_basic().await.unwrap();
    let total = fixture.context.settings().argon2_permits;
    assert_eq!(
        fixture.context.gate().available_permits(),
        total.checked_sub(1).unwrap()
    );

    let outcome = Login::new(fixture.context.clone())
        .execute(&meta(), login_input())
        .await;

    assert!(outcome.is_ok(), "{:?}", outcome.err());
    drop(held);
    assert_eq!(fixture.context.gate().available_permits(), total);
}

#[tokio::test(start_paused = true)]
async fn the_basic_gate_uses_the_acquire_timeout_for_the_hash_permit_too() {
    let fixture = IdentityFixture::new();
    let timeout = fixture.context.settings().argon2_acquire_timeout;
    let total = fixture.context.settings().argon2_permits;
    let mut hash_permits = Vec::new();
    for _ in 0..total {
        hash_permits.push(fixture.context.gate().acquire().await.unwrap());
    }

    let started = tokio::time::Instant::now();
    let error = fixture.context.acquire_basic().await.err().unwrap();

    assert!(started.elapsed() >= timeout);
    assert_eq!(error.code(), ErrorCode::WEB_009);
    drop(hash_permits);
    // The failed call must give the Basic permit back.
    assert!(fixture.context.acquire_basic().await.is_ok());
}
