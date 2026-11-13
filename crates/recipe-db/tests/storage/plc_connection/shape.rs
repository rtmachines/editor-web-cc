//! Check tests of the table `plc_connection`: the shape, the pairs of login and password, and
//! the credentials of each mode (design D1, tasks 4.1 and 4.3).

use super::*;

#[tokio::test]
async fn plc_connection_s7_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = insert_s7(&mut conn, machine, S7_1500, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn plc_connection_iec_row_with_valid_values_inserts() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_iec(&mut conn, machine, TWIN_CAT, None).await;

    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn plc_connection_row_starts_at_row_version_1() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;
    insert_s7(&mut conn, machine, S7_1500, None).await.unwrap();

    let version: i64 =
        sqlx::query_scalar("SELECT row_version FROM plc_connection WHERE machine_id = $1")
            .bind(machine)
            .fetch_one(&mut conn)
            .await
            .unwrap();

    assert_eq!(version, 1);
}

#[tokio::test]
async fn plc_connection_row_of_another_platform_violates_the_machine_platform_key() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = insert_iec(&mut conn, machine, TWIN_CAT, None).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "plc_connection_machine_platform",
    );
}

#[tokio::test]
async fn plc_connection_second_row_for_one_machine_violates_the_machine_unique_key() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;
    insert_s7(&mut conn, machine, S7_1500, None).await.unwrap();

    let result = insert_s7(&mut conn, machine, S7_1500, None).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_connection_machine_unique");
}

#[tokio::test]
async fn plc_connection_s7_row_with_a_port_violates_the_shape_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = insert_s7(&mut conn, machine, S7_1500, Some(102)).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_connection_shape");
}

#[tokio::test]
async fn plc_connection_iec_row_with_a_rack_violates_the_shape_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;

    let result = insert_iec(&mut conn, machine, TWIN_CAT, Some(0)).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_connection_shape");
}

#[tokio::test]
async fn plc_connection_s7_row_without_a_rack_violates_the_shape_check() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, S7_1500).await;

    let result = sqlx::query(
        "INSERT INTO plc_connection (machine_id, plc_platform, host, slot, db_number) \
         VALUES ($1, $2, '192.168.1.20', 2, 10)",
    )
    .bind(machine)
    .bind(S7_1500)
    .execute(&mut conn)
    .await;

    assert_constraint(result, CHECK_VIOLATION, "plc_connection_shape");
}

// ---------------------------------------------------------------------------------------------
// Task 4.3: the check `plc_connection_credentials`. Login and sealed password come as a pair.
// The mode decides whether the pair is needed, permitted, or refused.
// ---------------------------------------------------------------------------------------------

/// Insert an IEC record for a new machine of `platform` with an explicit login and sealed
/// password, so that a test can break the pair.
async fn credential_case(
    conn: &mut PgConnection,
    platform: &str,
    mode: &str,
    login: Option<&str>,
    sealed: Option<&[u8]>,
) -> Result<Uuid, sqlx::Error> {
    let machine = fixtures::machine(conn, platform).await;
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, port, address, comm_mode, login, password_sealed) \
         VALUES ($1, $2, '192.168.1.20', 4840, '5.12.34.56.1.1', $3, $4, $5) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(mode)
    .bind(login)
    .bind(sealed)
    .fetch_one(conn)
    .await
}

/// Insert an S7 record for a new machine of `platform` with an explicit login and sealed password.
async fn s7_credential_case(
    conn: &mut PgConnection,
    platform: &str,
    login: Option<&str>,
    sealed: Option<&[u8]>,
) -> Result<Uuid, sqlx::Error> {
    let machine = fixtures::machine(conn, platform).await;
    sqlx::query_scalar(
        "INSERT INTO plc_connection \
             (machine_id, plc_platform, host, rack, slot, db_number, login, password_sealed) \
         VALUES ($1, $2, '192.168.1.20', 0, 2, 10, $3, $4) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(login)
    .bind(sealed)
    .fetch_one(conn)
    .await
}

const CREDENTIALS: &str = "plc_connection_credentials";
const SEALED: &[u8] = &[1_u8];

#[tokio::test]
async fn plc_connection_login_without_a_ciphertext_violates_the_credentials_check() {
    let mut conn = connection().await;

    for (platform, mode) in [
        (CODESYS, "OpcUa"),
        (CODESYS, "CodesysV3"),
        (TWIN_CAT, "Ads"),
    ] {
        let result = credential_case(&mut conn, platform, mode, Some("operator"), None).await;
        assert_check(&format!("login only, {mode}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_ciphertext_without_a_login_violates_the_credentials_check() {
    let mut conn = connection().await;

    for (platform, mode) in [
        (CODESYS, "OpcUa"),
        (CODESYS, "CodesysV3"),
        (TWIN_CAT, "Ads"),
    ] {
        let result = credential_case(&mut conn, platform, mode, None, Some(SEALED)).await;
        assert_check(&format!("ciphertext only, {mode}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_mode_that_needs_credentials_without_a_login_violates_the_credentials_check()
{
    let mut conn = connection().await;

    for mode in ["OpcUa", "CodesysV3"] {
        let result = credential_case(&mut conn, CODESYS, mode, None, None).await;
        assert_check(&format!("no pair, {mode}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_modbus_row_with_a_login_violates_the_credentials_check() {
    let mut conn = connection().await;

    for platform in [CODESYS, TWIN_CAT] {
        let result = credential_case(
            &mut conn,
            platform,
            "ModbusTcp",
            Some("operator"),
            Some(SEALED),
        )
        .await;
        assert_check(&format!("ModbusTcp on {platform}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_s7_row_with_a_login_violates_the_credentials_check() {
    let mut conn = connection().await;

    for platform in [S7_300, S7_1200, S7_1500] {
        let result = s7_credential_case(&mut conn, platform, Some("operator"), Some(SEALED)).await;
        assert_check(&format!("S7 login on {platform}"), result, CREDENTIALS);
    }
}

#[tokio::test]
async fn plc_connection_row_with_the_pair_that_its_mode_permits_inserts() {
    let mut conn = connection().await;
    let pair = (Some("operator"), Some(SEALED));
    let cases = [
        (CODESYS, "OpcUa", pair),
        (CODESYS, "CodesysV3", pair),
        (CODESYS, "ModbusTcp", (None, None)),
        (TWIN_CAT, "OpcUa", pair),
        (TWIN_CAT, "Ads", pair),
        (TWIN_CAT, "Ads", (None, None)),
        (TWIN_CAT, "ModbusTcp", (None, None)),
    ];

    for (platform, mode, (login, sealed)) in cases {
        let result = credential_case(&mut conn, platform, mode, login, sealed).await;
        assert_accepted(&format!("{platform} {mode} login {login:?}"), result);
    }
    for platform in [S7_300, S7_1200, S7_1500] {
        let result = s7_credential_case(&mut conn, platform, None, None).await;
        assert_accepted(&format!("S7 without a login on {platform}"), result);
    }
}
