//! Small builders of fixture rows (design D14).
//!
//! A fixture connects as a table owner and obeys every trigger of `data-model.md` section 4.
//! It uses the platform of its machine. A parameter goes into a live draft, a type belongs to
//! the platform of the machine, and a unit belongs to the machine of the recipe.
//! The builders take a name from a counter, so two fixtures in one database never collide.

use std::sync::atomic::{AtomicU32, Ordering};

use sqlx::PgConnection;
use uuid::Uuid;

/// The platform of the default fixtures.
pub const TWIN_CAT: &str = "TwinCat3";
/// Every platform value of `data-model.md` section 1.1.
pub const ALL_PLATFORMS: [&str; 5] = ["CodesysIec", "TwinCat3", "S7_1200", "S7_1500", "S7_300"];
/// A hash that satisfies `recipe_version_hash_format`.
const FIXTURE_HASH: &str = "a";

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(1);

/// Return a number that no other fixture of this test binary uses.
fn next_number() -> u32 {
    NEXT_NUMBER.fetch_add(1, Ordering::Relaxed)
}

/// Insert an administrator with a hash that matches `app_user_hash_format`.
pub async fn user(conn: &mut PgConnection) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO app_user (login, password_hash, role) \
         VALUES ($1, '$argon2id$fixture', 'administrator') RETURNING id",
    )
    .bind(format!("user{}", next_number()))
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a live machine on a platform.
pub async fn machine(conn: &mut PgConnection, platform: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO machine (code, name, plc_platform) VALUES ($1, 'machine', $2) RETURNING id",
    )
    .bind(format!("M{}", next_number()))
    .bind(platform)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a unit. The parent unit of a nested path must exist.
pub async fn unit(conn: &mut PgConnection, machine: Uuid, path: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO unit (machine_id, path, name) VALUES ($1, $2, 'unit') RETURNING id",
    )
    .bind(machine)
    .bind(path)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Return the global primitive type of a name on every platform.
///
/// The seed migration inserts the real rows later. The insert does nothing when the row exists,
/// and the select finds the row of the seed.
pub async fn primitive(conn: &mut PgConnection, name: &str) -> Uuid {
    primitive_on(conn, name, &ALL_PLATFORMS).await
}

/// Return the global primitive type of a name. A new row exists on the listed platforms only.
pub async fn primitive_on(conn: &mut PgConnection, name: &str, platforms: &[&str]) -> Uuid {
    let platforms: Vec<String> = platforms
        .iter()
        .map(|platform| (*platform).to_owned())
        .collect();
    sqlx::query(
        "INSERT INTO plc_type (kind, name, platforms, size_bytes, leaf_class) \
         VALUES ('primitive', $1, $2, 4, 'numeric') ON CONFLICT DO NOTHING",
    )
    .bind(name)
    .bind(platforms)
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query_scalar(
        "SELECT id FROM plc_type \
         WHERE name = $1 AND machine_id IS NULL AND string_length IS NULL",
    )
    .bind(name)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a `STRING(length)` type that belongs to one machine and to its platform.
pub async fn machine_string(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    length: i16,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO plc_type \
             (kind, name, platforms, size_bytes, string_length, leaf_class, machine_id) \
         VALUES ('primitive', 'STRING', ARRAY[$1], 12, $2, 'string', $3) RETURNING id",
    )
    .bind(platform)
    .bind(length)
    .bind(machine)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a struct type of one machine and one platform.
///
/// The struct needs a member before the transaction commits (`E-PLT-036`). Call `member` in the
/// same transaction.
pub async fn struct_type(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    name: &str,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, machine_id) \
         VALUES ('struct', $1, ARRAY[$2], $3) RETURNING id",
    )
    .bind(name)
    .bind(platform)
    .bind(machine)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a member of a struct. The member name is `m` and the ordinal.
pub async fn member(conn: &mut PgConnection, struct_type: Uuid, ordinal: i16, member_type: Uuid) {
    sqlx::query(
        "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
         VALUES ($1, $2, 'm' || $2::text, $3)",
    )
    .bind(struct_type)
    .bind(ordinal)
    .bind(member_type)
    .execute(conn)
    .await
    .unwrap();
}

/// Insert a struct with one member in one transaction and return its id.
pub async fn complete_struct(
    conn: &mut PgConnection,
    machine: Uuid,
    platform: &str,
    name: &str,
    member_type: Uuid,
) -> Uuid {
    let mut tx = sqlx::Connection::begin(&mut *conn).await.unwrap();
    let id = struct_type(&mut tx, machine, platform, name).await;
    member(&mut tx, id, 1, member_type).await;
    tx.commit().await.unwrap();
    id
}

/// Insert a recipe of a machine. The platform must be the platform of the machine.
pub async fn recipe(conn: &mut PgConnection, machine: Uuid, platform: &str, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO recipe (machine_id, plc_platform, reference_number, name, created_by) \
         VALUES ($1, $2, $3, 'recipe', $4) RETURNING id",
    )
    .bind(machine)
    .bind(platform)
    .bind(format!("REF{}", next_number()))
    .bind(user)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a live draft version.
pub async fn draft_version(
    conn: &mut PgConnection,
    recipe: Uuid,
    platform: &str,
    number: i32,
    user: Uuid,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO recipe_version (recipe_id, plc_platform, number, change_note, created_by) \
         VALUES ($1, $2, $3, 'fixture', $4) RETURNING id",
    )
    .bind(recipe)
    .bind(platform)
    .bind(number)
    .bind(user)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Approve a draft with the allowed transition `draft` to `approved`.
pub async fn approve(conn: &mut PgConnection, version: Uuid, approver: Uuid) {
    sqlx::query(
        "UPDATE recipe_version SET status = 'approved', approved_by = $2, approved_at = now(), \
             content_sha256 = repeat($3, 64) WHERE id = $1",
    )
    .bind(version)
    .bind(approver)
    .bind(FIXTURE_HASH)
    .execute(conn)
    .await
    .unwrap();
}

/// Retire an approved version with the allowed transition `approved` to `retired`.
pub async fn retire(conn: &mut PgConnection, version: Uuid) {
    sqlx::query("UPDATE recipe_version SET status = 'retired', retired_at = now() WHERE id = $1")
        .bind(version)
        .execute(conn)
        .await
        .unwrap();
}

/// Insert a parameter without an S7 address. The value is the JSON number 0.
pub async fn parameter(
    conn: &mut PgConnection,
    version: Uuid,
    platform: &str,
    unit: Uuid,
    plc_type: Uuid,
    tag: &str,
) -> Uuid {
    parameter_with_address(conn, version, platform, unit, plc_type, tag, None).await
}

/// Insert a parameter with an optional S7 address.
pub async fn parameter_with_address(
    conn: &mut PgConnection,
    version: Uuid,
    platform: &str,
    unit: Uuid,
    plc_type: Uuid,
    tag: &str,
    s7_address: Option<&str>,
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO parameter \
             (recipe_version_id, plc_platform, unit_id, tag, name_en, description_en, \
              plc_type_id, s7_address, value) \
         VALUES ($1, $2, $3, $4, 'name', 'description', $5, $6, '0'::jsonb) RETURNING id",
    )
    .bind(version)
    .bind(platform)
    .bind(unit)
    .bind(tag)
    .bind(plc_type)
    .bind(s7_address)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a numeric limit from 0 to 10 for a leaf of a parameter.
pub async fn limit(conn: &mut PgConnection, parameter: Uuid, version: Uuid, leaf_path: &str) {
    sqlx::query(
        "INSERT INTO parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, max_value) \
         VALUES ($1, $2, $3, 0, 10)",
    )
    .bind(parameter)
    .bind(version)
    .bind(leaf_path)
    .execute(conn)
    .await
    .unwrap();
}

/// Insert a translation of a parameter.
pub async fn translation(conn: &mut PgConnection, parameter: Uuid, version: Uuid, lang: &str) {
    sqlx::query(
        "INSERT INTO parameter_translation (parameter_id, recipe_version_id, lang, name) \
         VALUES ($1, $2, $3, 'nazwa')",
    )
    .bind(parameter)
    .bind(version)
    .bind(lang)
    .execute(conn)
    .await
    .unwrap();
}

/// A machine with a unit, a recipe, a live draft, and the type `DINT`.
#[derive(Debug, Clone, Copy)]
pub struct Draft {
    /// The user who created the recipe and the draft.
    pub user: Uuid,
    /// The machine on the platform `TwinCat3`.
    pub machine: Uuid,
    /// The unit `U1` of the machine.
    pub unit: Uuid,
    /// The recipe of the machine.
    pub recipe: Uuid,
    /// The live draft, version number 1.
    pub version: Uuid,
    /// The global primitive `DINT`.
    pub dint: Uuid,
}

impl Draft {
    /// Insert a complete plant for a `TwinCat3` draft.
    pub async fn create(conn: &mut PgConnection) -> Self {
        let user = user(conn).await;
        let machine = machine(conn, TWIN_CAT).await;
        let unit = unit(conn, machine, "U1").await;
        let recipe = recipe(conn, machine, TWIN_CAT, user).await;
        let version = draft_version(conn, recipe, TWIN_CAT, 1, user).await;
        let dint = primitive(conn, "DINT").await;
        Self {
            user,
            machine,
            unit,
            recipe,
            version,
            dint,
        }
    }

    /// Insert a parameter of the type `DINT` into the draft.
    pub async fn parameter(&self, conn: &mut PgConnection, tag: &str) -> Uuid {
        parameter(conn, self.version, TWIN_CAT, self.unit, self.dint, tag).await
    }

    /// Insert a second recipe on the same machine with its own live draft.
    pub async fn other_draft(&self, conn: &mut PgConnection) -> Uuid {
        let recipe = recipe(conn, self.machine, TWIN_CAT, self.user).await;
        draft_version(conn, recipe, TWIN_CAT, 1, self.user).await
    }
}
