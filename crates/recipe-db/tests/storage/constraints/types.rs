//! Tests of `plc_type` and `plc_type_member` (`data-model.md` sections 3.3 and 3.4, task 7.2).
//!
//! A violating row starts from a valid value and changes one field. The named constraint is then
//! the only one that fails. A member goes into the transaction that creates its struct, because
//! trigger `plc_type_immutable` closes a committed struct (`E-PLT-034`).

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures::{self, ALL_PLATFORMS, TWIN_CAT};
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint,
};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "plc_type_pkey",
    "plc_type_kind_value",
    "plc_type_name_present",
    "plc_type_name_format",
    "plc_type_platforms_value",
    "plc_type_revision_positive",
    "plc_type_string_length_required",
    "plc_type_string_length_range",
    "plc_type_leaf_class_present",
    "plc_type_leaf_class_value",
    "plc_type_leaf_class_string",
    "plc_type_primitive_shape",
    "plc_type_array_shape",
    "plc_type_struct_shape",
    "plc_type_supersedes_only_struct",
    "plc_type_named_unique",
    "plc_type_string_unique",
    "plc_type_array_unique",
    "plc_type_element_type_id_fkey",
    "plc_type_machine_id_fkey",
    "plc_type_supersedes_type_id_fkey",
    "plc_type_member_pkey",
    "plc_type_member_ordinal_range",
    "plc_type_member_name_format",
    "plc_type_member_name_unique",
    "plc_type_member_no_self",
    "plc_type_member_struct_type_id_fkey",
    "plc_type_member_member_type_id_fkey",
];

/// The columns of one `plc_type` row.
#[derive(Debug, Clone)]
struct TypeRow {
    kind: &'static str,
    name: Option<&'static str>,
    platforms: Vec<String>,
    size_bytes: Option<i32>,
    string_length: Option<i16>,
    leaf_class: Option<&'static str>,
    element_type_id: Option<Uuid>,
    dim_lower: Option<Vec<i32>>,
    dim_upper: Option<Vec<i32>>,
    machine_id: Option<Uuid>,
    revision: i32,
    supersedes_type_id: Option<Uuid>,
}

impl TypeRow {
    /// A row of the given kind with no optional column set, on the platform `TwinCat3`.
    fn blank(kind: &'static str) -> Self {
        Self {
            kind,
            name: None,
            platforms: vec![TWIN_CAT.to_owned()],
            size_bytes: None,
            string_length: None,
            leaf_class: None,
            element_type_id: None,
            dim_lower: None,
            dim_upper: None,
            machine_id: None,
            revision: 1,
            supersedes_type_id: None,
        }
    }

    /// A global primitive of 4 bytes and class `numeric`, on every platform.
    fn primitive(name: &'static str) -> Self {
        Self {
            name: Some(name),
            platforms: ALL_PLATFORMS
                .iter()
                .map(|platform| (*platform).to_owned())
                .collect(),
            size_bytes: Some(4),
            leaf_class: Some("numeric"),
            ..Self::blank("primitive")
        }
    }

    /// A `STRING(20)` of one machine.
    fn machine_string(machine: Uuid) -> Self {
        Self {
            name: Some("STRING"),
            size_bytes: Some(12),
            string_length: Some(20),
            leaf_class: Some("string"),
            machine_id: Some(machine),
            ..Self::blank("primitive")
        }
    }

    /// A struct of one machine.
    fn struct_of(machine: Uuid, name: &'static str) -> Self {
        Self {
            name: Some(name),
            machine_id: Some(machine),
            ..Self::blank("struct")
        }
    }

    /// An array of one machine with the given element type and bounds.
    fn array(machine: Uuid, element: Uuid, lower: Vec<i32>, upper: Vec<i32>) -> Self {
        Self {
            element_type_id: Some(element),
            dim_lower: Some(lower),
            dim_upper: Some(upper),
            machine_id: Some(machine),
            ..Self::blank("array")
        }
    }
}

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

/// Return a `TwinCat3` machine and the seed type `DINT`.
async fn machine_and_dint(conn: &mut PgConnection) -> (Uuid, Uuid) {
    let machine = fixtures::machine(conn, TWIN_CAT).await;
    let dint = fixtures::primitive(conn, "DINT").await;
    (machine, dint)
}

/// Insert a `plc_type` row and return its id.
async fn insert(conn: &mut PgConnection, row: &TypeRow) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, size_bytes, string_length, leaf_class, \
             element_type_id, dim_lower, dim_upper, machine_id, revision, supersedes_type_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING id",
    )
    .bind(row.kind)
    .bind(row.name)
    .bind(row.platforms.clone())
    .bind(row.size_bytes)
    .bind(row.string_length)
    .bind(row.leaf_class)
    .bind(row.element_type_id)
    .bind(row.dim_lower.clone())
    .bind(row.dim_upper.clone())
    .bind(row.machine_id)
    .bind(row.revision)
    .bind(row.supersedes_type_id)
    .fetch_one(conn)
    .await
}

/// Insert a member row of a struct.
async fn insert_member(
    conn: &mut PgConnection,
    struct_type: Uuid,
    ordinal: i16,
    name: &str,
    member_type: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(struct_type)
    .bind(ordinal)
    .bind(name)
    .bind(member_type)
    .execute(conn)
    .await
    .map(|_| ())
}

// ---- plc_type: keys and checks ----

#[tokio::test]
async fn plc_type_rows_of_each_kind_insert() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;

    insert(&mut conn, &TypeRow::primitive("TEST_PRIM"))
        .await
        .unwrap();
    insert(&mut conn, &TypeRow::machine_string(machine))
        .await
        .unwrap();
    insert(&mut conn, &TypeRow::array(machine, dint, vec![1], vec![3]))
        .await
        .unwrap();
    fixtures::complete_struct(&mut conn, machine, TWIN_CAT, "TEST_STRUCT", dint).await;
}

#[tokio::test]
async fn plc_type_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = connection().await;
    let id = insert(&mut conn, &TypeRow::primitive("TEST_PRIM"))
        .await
        .unwrap();

    let result = sqlx::query(
        "INSERT INTO plc_type (id, kind, name, platforms, size_bytes, leaf_class) \
         VALUES ($1, 'primitive', 'TEST_OTHER', ARRAY['TwinCat3'], 4, 'numeric')",
    )
    .bind(id)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_pkey");
}

#[tokio::test]
async fn plc_type_kind_value_rejects_an_unknown_kind() {
    let mut conn = connection().await;
    let row = TypeRow {
        kind: "enum",
        leaf_class: None,
        ..TypeRow::primitive("TEST_ENUM")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_kind_value");
}

#[tokio::test]
async fn plc_type_name_present_rejects_a_named_array() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow {
        name: Some("TEST_ARR"),
        ..TypeRow::array(machine, dint, vec![1], vec![3])
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_name_present");
}

#[tokio::test]
async fn plc_type_name_format_rejects_a_name_with_a_space() {
    let mut conn = connection().await;

    let result = insert(&mut conn, &TypeRow::primitive("bad name")).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_name_format");
}

#[tokio::test]
async fn plc_type_platforms_value_rejects_an_empty_platform_list() {
    let mut conn = connection().await;
    let row = TypeRow {
        platforms: Vec::new(),
        ..TypeRow::primitive("TEST_PLAT")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_platforms_value");
}

#[tokio::test]
async fn plc_type_revision_positive_rejects_revision_zero() {
    let mut conn = connection().await;
    let row = TypeRow {
        revision: 0,
        ..TypeRow::primitive("TEST_REV")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_revision_positive");
}

#[tokio::test]
async fn plc_type_string_length_required_rejects_a_string_without_a_length() {
    let mut conn = connection().await;
    let row = TypeRow {
        name: Some("STRING"),
        ..TypeRow::primitive("TEST_STR")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_string_length_required");
}

#[tokio::test]
async fn plc_type_string_length_range_rejects_a_length_above_16382() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let row = TypeRow {
        string_length: Some(16383),
        ..TypeRow::machine_string(machine)
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_string_length_range");
}

#[tokio::test]
async fn plc_type_leaf_class_present_rejects_a_primitive_without_a_leaf_class() {
    let mut conn = connection().await;
    let row = TypeRow {
        leaf_class: None,
        ..TypeRow::primitive("TEST_LEAF")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_leaf_class_present");
}

#[tokio::test]
async fn plc_type_leaf_class_value_rejects_an_unknown_leaf_class() {
    let mut conn = connection().await;
    let row = TypeRow {
        leaf_class: Some("integer"),
        ..TypeRow::primitive("TEST_LCV")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_leaf_class_value");
}

#[tokio::test]
async fn plc_type_leaf_class_string_rejects_class_string_without_a_length() {
    let mut conn = connection().await;
    let row = TypeRow {
        leaf_class: Some("string"),
        ..TypeRow::primitive("TEST_LCS")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_leaf_class_string");
}

#[tokio::test]
async fn plc_type_primitive_shape_rejects_an_element_type() {
    let mut conn = connection().await;
    let dint = fixtures::primitive(&mut conn, "DINT").await;
    let row = TypeRow {
        element_type_id: Some(dint),
        ..TypeRow::primitive("TEST_ELEM")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_primitive_shape");
}

#[tokio::test]
async fn plc_type_primitive_shape_rejects_a_size_below_one_byte() {
    let mut conn = connection().await;
    let row = TypeRow {
        size_bytes: Some(0),
        ..TypeRow::primitive("TEST_SIZE")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_primitive_shape");
}

#[tokio::test]
async fn plc_type_struct_shape_rejects_a_struct_without_a_machine() {
    let mut conn = connection().await;
    let (machine, _dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow {
        machine_id: None,
        ..TypeRow::struct_of(machine, "TEST_NOMACHINE")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_struct_shape");
}

#[tokio::test]
async fn plc_type_supersedes_only_struct_rejects_a_primitive_that_supersedes() {
    let mut conn = connection().await;
    let dint = fixtures::primitive(&mut conn, "DINT").await;
    let row = TypeRow {
        supersedes_type_id: Some(dint),
        ..TypeRow::primitive("TEST_SUP")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_supersedes_only_struct");
}

// ---- dims_valid and plc_type_array_shape ----

/// The lower bounds, the upper bounds, and the expected result of one `dims_valid` case.
type DimsCase = (Option<Vec<i32>>, Option<Vec<i32>>, bool);

#[tokio::test]
async fn dims_valid_accepts_one_to_three_dimensions_with_a_span_below_65536() {
    let mut conn = connection().await;
    let cases: [DimsCase; 9] = [
        (Some(vec![1]), Some(vec![3]), true),
        (Some(vec![0]), Some(vec![65535]), true),
        (Some(vec![1, 1, 1]), Some(vec![2, 2, 2]), true),
        (Some(vec![0]), Some(vec![65536]), false),
        (Some(vec![1, 1, 1, 1]), Some(vec![2, 2, 2, 2]), false),
        (Some(vec![3]), Some(vec![1]), false),
        (Some(Vec::new()), Some(Vec::new()), false),
        (None, None, false),
        (Some(vec![1]), None, false),
    ];

    for (lower, upper, expected) in cases {
        let valid: bool = sqlx::query_scalar("SELECT dims_valid($1, $2)")
            .bind(lower.clone())
            .bind(upper.clone())
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(valid, expected, "lower {lower:?}, upper {upper:?}");
    }
}

#[tokio::test]
async fn plc_type_array_shape_rejects_null_bounds() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow {
        dim_lower: None,
        dim_upper: None,
        ..TypeRow::array(machine, dint, vec![1], vec![3])
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_array_shape");
}

#[tokio::test]
async fn plc_type_array_shape_rejects_empty_bounds() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow::array(machine, dint, Vec::new(), Vec::new());

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_array_shape");
}

#[tokio::test]
async fn plc_type_array_shape_rejects_more_than_three_dimensions() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow::array(machine, dint, vec![1, 1, 1, 1], vec![2, 2, 2, 2]);

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_array_shape");
}

#[tokio::test]
async fn plc_type_array_shape_rejects_a_lower_bound_above_the_upper_bound() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow::array(machine, dint, vec![5], vec![3]);

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_array_shape");
}

#[tokio::test]
async fn plc_type_array_shape_rejects_a_span_of_65536() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow::array(machine, dint, vec![0], vec![65536]);

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_array_shape");
}

// ---- plc_type: unique indexes ----

#[tokio::test]
async fn plc_type_named_unique_rejects_a_second_global_primitive_of_one_name() {
    let mut conn = connection().await;
    insert(&mut conn, &TypeRow::primitive("TEST_DUP"))
        .await
        .unwrap();

    let result = insert(&mut conn, &TypeRow::primitive("TEST_DUP")).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_named_unique");
}

#[tokio::test]
async fn plc_type_string_unique_rejects_a_second_string_of_one_length_in_a_machine() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    insert(&mut conn, &TypeRow::machine_string(machine))
        .await
        .unwrap();

    let result = insert(&mut conn, &TypeRow::machine_string(machine)).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_string_unique");
}

#[tokio::test]
async fn plc_type_array_unique_rejects_a_second_array_with_one_element_and_bounds() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow::array(machine, dint, vec![1], vec![3]);
    insert(&mut conn, &row).await.unwrap();

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_array_unique");
}

// ---- plc_type: foreign keys ----

#[tokio::test]
async fn plc_type_element_type_id_fkey_rejects_an_unknown_element() {
    let mut conn = connection().await;
    let (machine, _dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow::array(machine, Uuid::nil(), vec![1], vec![3]);

    let result = insert(&mut conn, &row).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "plc_type_element_type_id_fkey",
    );
}

#[tokio::test]
async fn plc_type_machine_id_fkey_rejects_an_unknown_machine() {
    let mut conn = connection().await;
    let (machine, _dint) = machine_and_dint(&mut conn).await;
    let row = TypeRow {
        machine_id: Some(Uuid::nil()),
        ..TypeRow::struct_of(machine, "TEST_FK")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "plc_type_machine_id_fkey");
}

#[tokio::test]
async fn plc_type_supersedes_type_id_fkey_rejects_an_unknown_type() {
    // Only a struct may supersede. A primitive would fail the CHECK before the foreign key.
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let row = TypeRow {
        supersedes_type_id: Some(Uuid::nil()),
        ..TypeRow::struct_of(machine, "TEST_SFK")
    };

    let result = insert(&mut conn, &row).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "plc_type_supersedes_type_id_fkey",
    );
}

// ---- plc_type_member ----

#[tokio::test]
async fn plc_type_member_row_inserts_in_the_transaction_of_its_struct() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;

    let parent = fixtures::complete_struct(&mut conn, machine, TWIN_CAT, "TEST_OK", dint).await;

    let members: i64 =
        sqlx::query_scalar("SELECT count(*) FROM plc_type_member WHERE struct_type_id = $1")
            .bind(parent)
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(members, 1);
}

#[tokio::test]
async fn plc_type_member_pkey_rejects_two_members_with_one_ordinal() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let parent = fixtures::struct_type(&mut tx, machine, TWIN_CAT, "TEST_PK").await;
    insert_member(&mut tx, parent, 1, "first", dint)
        .await
        .unwrap();

    let result = insert_member(&mut tx, parent, 1, "second", dint).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_member_pkey");
}

#[tokio::test]
async fn plc_type_member_ordinal_range_rejects_ordinal_zero() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let parent = fixtures::struct_type(&mut tx, machine, TWIN_CAT, "TEST_ORD").await;

    let result = insert_member(&mut tx, parent, 0, "zero", dint).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_member_ordinal_range");
}

#[tokio::test]
async fn plc_type_member_name_format_rejects_a_name_with_a_space() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let parent = fixtures::struct_type(&mut tx, machine, TWIN_CAT, "TEST_NAME").await;

    let result = insert_member(&mut tx, parent, 1, "bad name", dint).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_member_name_format");
}

#[tokio::test]
async fn plc_type_member_name_unique_rejects_two_members_with_one_name() {
    let mut conn = connection().await;
    let (machine, dint) = machine_and_dint(&mut conn).await;
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let parent = fixtures::struct_type(&mut tx, machine, TWIN_CAT, "TEST_DUPNAME").await;
    insert_member(&mut tx, parent, 1, "same", dint)
        .await
        .unwrap();

    let result = insert_member(&mut tx, parent, 2, "same", dint).await;

    assert_constraint(result, UNIQUE_VIOLATION, "plc_type_member_name_unique");
}

#[tokio::test]
async fn plc_type_member_no_self_rejects_a_struct_as_its_own_member() {
    let mut conn = connection().await;
    let (machine, _dint) = machine_and_dint(&mut conn).await;
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let parent = fixtures::struct_type(&mut tx, machine, TWIN_CAT, "TEST_SELF").await;

    let result = insert_member(&mut tx, parent, 1, "self_ref", parent).await;

    assert_constraint(result, CHECK_VIOLATION, "plc_type_member_no_self");
}

#[tokio::test]
async fn plc_type_member_struct_type_id_fkey_rejects_an_unknown_struct() {
    let mut conn = connection().await;
    let dint = fixtures::primitive(&mut conn, "DINT").await;

    let result = insert_member(&mut conn, Uuid::nil(), 1, "orphan", dint).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "plc_type_member_struct_type_id_fkey",
    );
}

#[tokio::test]
async fn plc_type_member_member_type_id_fkey_rejects_an_unknown_member_type() {
    let mut conn = connection().await;
    let (machine, _dint) = machine_and_dint(&mut conn).await;
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let parent = fixtures::struct_type(&mut tx, machine, TWIN_CAT, "TEST_MFK").await;

    let result = insert_member(&mut tx, parent, 1, "member", Uuid::nil()).await;

    assert_constraint(
        result,
        FOREIGN_KEY_VIOLATION,
        "plc_type_member_member_type_id_fkey",
    );
}
