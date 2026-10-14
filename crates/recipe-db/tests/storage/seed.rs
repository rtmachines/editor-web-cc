//! Tests of the delta requirement "Stable identifiers of the seed types" (`plant-model`) and of
//! the seed migration `0004_seed_plc_types.sql` (`data-model.md` section 7, design D11).

use std::collections::BTreeSet;

use recipe_db::migrate;
use recipe_db::settings::MigrateSettings;
use recipe_domain::plant::{
    LeafClass, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, StringKind,
    StringLength,
};
use uuid::Uuid;

use crate::support::{Role, TestDatabase};

/// The platforms of a row, in the order of the column list of `0004_seed_plc_types.sql`.
const ALL: &[&str] = &["CodesysIec", "TwinCat3", "S7_1200", "S7_1500", "S7_300"];
const NO_S7_300: &[&str] = &["CodesysIec", "TwinCat3", "S7_1200", "S7_1500"];
const CT_1500: &[&str] = &["CodesysIec", "TwinCat3", "S7_1500"];
const CT_1500_300: &[&str] = &["CodesysIec", "TwinCat3", "S7_1500", "S7_300"];
const CT: &[&str] = &["CodesysIec", "TwinCat3"];
const S7_1200_1500: &[&str] = &["S7_1200", "S7_1500"];
const S7_1200_1500_300: &[&str] = &["S7_1200", "S7_1500", "S7_300"];
const S7_1500_300: &[&str] = &["S7_1500", "S7_300"];

/// One row of the seed, transcribed from the matrix of `data-model.md` section 7.
///
/// The identifier is the row number of the matrix in hexadecimal (design D11). The rows
/// `STRING` and `WSTRING` with 80 characters take `1e` and `1f`.
struct SeedRow {
    id: &'static str,
    name: &'static str,
    platforms: &'static [&'static str],
    s7_size: i32,
    string_length: Option<i16>,
    class: &'static str,
}

/// The 31 seed rows, in the order of the identifiers.
const SEED: [SeedRow; 31] = [
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000001",
        name: "BOOL",
        platforms: ALL,
        s7_size: 1,
        string_length: None,
        class: "bool",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000002",
        name: "BYTE",
        platforms: ALL,
        s7_size: 1,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000003",
        name: "WORD",
        platforms: ALL,
        s7_size: 2,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000004",
        name: "DWORD",
        platforms: ALL,
        s7_size: 4,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000005",
        name: "LWORD",
        platforms: CT_1500,
        s7_size: 8,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000006",
        name: "SINT",
        platforms: NO_S7_300,
        s7_size: 1,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000007",
        name: "USINT",
        platforms: NO_S7_300,
        s7_size: 1,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000008",
        name: "INT",
        platforms: ALL,
        s7_size: 2,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000009",
        name: "UINT",
        platforms: NO_S7_300,
        s7_size: 2,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000000a",
        name: "DINT",
        platforms: ALL,
        s7_size: 4,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000000b",
        name: "UDINT",
        platforms: NO_S7_300,
        s7_size: 4,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000000c",
        name: "LINT",
        platforms: CT_1500,
        s7_size: 8,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000000d",
        name: "ULINT",
        platforms: CT_1500,
        s7_size: 8,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000000e",
        name: "REAL",
        platforms: ALL,
        s7_size: 4,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000000f",
        name: "LREAL",
        platforms: NO_S7_300,
        s7_size: 8,
        string_length: None,
        class: "numeric",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000010",
        name: "STRING",
        platforms: S7_1200_1500_300,
        s7_size: 256,
        string_length: Some(254),
        class: "string",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000011",
        name: "WSTRING",
        platforms: S7_1200_1500,
        s7_size: 512,
        string_length: Some(254),
        class: "string",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000012",
        name: "CHAR",
        platforms: S7_1200_1500_300,
        s7_size: 1,
        string_length: None,
        class: "char",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000013",
        name: "WCHAR",
        platforms: S7_1200_1500,
        s7_size: 2,
        string_length: None,
        class: "char",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000014",
        name: "TIME",
        platforms: ALL,
        s7_size: 4,
        string_length: None,
        class: "duration",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000015",
        name: "LTIME",
        platforms: CT_1500,
        s7_size: 8,
        string_length: None,
        class: "duration",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000016",
        name: "DATE",
        platforms: ALL,
        s7_size: 2,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000017",
        name: "LDATE",
        platforms: CT,
        s7_size: 8,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000018",
        name: "TIME_OF_DAY",
        platforms: ALL,
        s7_size: 4,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-000000000019",
        name: "LTOD",
        platforms: CT_1500,
        s7_size: 8,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000001a",
        name: "DATE_AND_TIME",
        platforms: CT_1500_300,
        s7_size: 8,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000001b",
        name: "LDT",
        platforms: CT_1500,
        s7_size: 8,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000001c",
        name: "DTL",
        platforms: S7_1200_1500,
        s7_size: 12,
        string_length: None,
        class: "instant",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000001d",
        name: "S5TIME",
        platforms: S7_1500_300,
        s7_size: 2,
        string_length: None,
        class: "duration",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000001e",
        name: "STRING",
        platforms: CT,
        s7_size: 82,
        string_length: Some(80),
        class: "string",
    },
    SeedRow {
        id: "5eed0000-0000-4000-8000-00000000001f",
        name: "WSTRING",
        platforms: CT,
        s7_size: 164,
        string_length: Some(80),
        class: "string",
    },
];

/// One global primitive row: id, name, platforms, S7 size, string length, and leaf class.
type DbRow = (
    Uuid,
    Option<String>,
    Vec<String>,
    Option<i32>,
    Option<i16>,
    Option<String>,
);

/// Read every global primitive row, ordered by identifier.
async fn global_primitive_rows(database: &TestDatabase) -> Vec<DbRow> {
    let mut conn = database.connect(Role::Migrator).await;
    sqlx::query_as::<_, DbRow>(
        "SELECT id, name, platforms, size_bytes, string_length, leaf_class \
         FROM plc_type WHERE machine_id IS NULL AND kind = 'primitive' ORDER BY id",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn there_are_31_global_primitive_rows() {
    let database = TestDatabase::create().await;

    let rows = global_primitive_rows(&database).await;

    assert_eq!(SEED.len(), 31);
    assert_eq!(rows.len(), 31);
}

#[tokio::test]
async fn each_seed_row_equals_its_row_of_the_matrix() {
    let database = TestDatabase::create().await;
    let rows = global_primitive_rows(&database).await;
    assert_eq!(rows.len(), SEED.len());

    for seed in &SEED {
        let id = Uuid::parse_str(seed.id).unwrap();
        assert!(rows.iter().any(|row| row.0 == id), "no row has {}", seed.id);
        let row = rows.iter().find(|row| row.0 == id).unwrap();
        let platforms: Vec<&str> = row.2.iter().map(String::as_str).collect();
        assert_eq!(row.1.as_deref(), Some(seed.name), "name of {}", seed.id);
        assert_eq!(platforms, seed.platforms, "platforms of {}", seed.name);
        assert_eq!(row.3, Some(seed.s7_size), "S7 size of {}", seed.name);
        assert_eq!(row.4, seed.string_length, "string length of {}", seed.name);
        assert_eq!(row.5.as_deref(), Some(seed.class), "class of {}", seed.name);
    }
}

#[tokio::test]
async fn each_seed_row_converts_into_a_domain_type_of_its_size_and_class() {
    let database = TestDatabase::create().await;
    let rows = global_primitive_rows(&database).await;
    assert_eq!(rows.len(), SEED.len());

    for row in &rows {
        let name = row.1.as_deref().unwrap();
        let primitive = Primitive::parse(name).unwrap();
        let leaf_class = LeafClass::parse(row.5.as_deref().unwrap()).unwrap();
        let size_bytes = u32::try_from(row.3.unwrap()).unwrap();
        let string_length = row.4.map(|length| u16::try_from(length).unwrap());
        let platforms: BTreeSet<PlcPlatform> = row
            .2
            .iter()
            .map(|text| PlcPlatform::parse(text).unwrap())
            .collect();
        let record = PlcTypeRecord {
            id: PlcTypeId::from(row.0),
            machine: None,
            platforms: platforms.clone(),
            shape: RecordShape::Primitive {
                primitive,
                leaf_class,
                size_bytes,
                string_length,
            },
        };

        let plc_type = PlcType::try_from(record.clone()).unwrap();

        assert_eq!(plc_type.shape(), &record.shape, "{name}");
        assert_eq!(leaf_class, primitive.leaf_class(), "class of {name}");
        match string_length {
            None => {
                assert_eq!(
                    primitive.fixed_size_bytes(),
                    Some(size_bytes),
                    "size of {name}"
                );
            }
            Some(length) => {
                let kind = if primitive == Primitive::Wstring {
                    StringKind::Wstring
                } else {
                    StringKind::String
                };
                for platform in &platforms {
                    let parsed = StringLength::parse(kind, *platform, u32::from(length)).unwrap();
                    assert_eq!(
                        parsed.size_bytes(kind),
                        Some(size_bytes),
                        "size of {name}({length}) on {platform}"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn a_second_run_of_the_migrator_changes_no_seed_row() {
    let database = TestDatabase::create().await;
    let before = global_primitive_rows(&database).await;
    assert_eq!(before.len(), 31);

    migrate(
        &database.target(Role::Migrator),
        &MigrateSettings::default(),
    )
    .await
    .unwrap();

    let after = global_primitive_rows(&database).await;
    assert_eq!(after, before);
}
