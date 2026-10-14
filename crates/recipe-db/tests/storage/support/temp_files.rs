//! A temporary directory with migration files that a test writes at run time.

use std::path::{Path, PathBuf};

use uuid::Uuid;

/// A directory that holds migration files. The drop removes it.
#[derive(Debug)]
pub struct TempMigrations {
    directory: PathBuf,
}

impl TempMigrations {
    /// Write the files `(name, sql)` into a new directory.
    pub fn new(files: &[(&str, &str)]) -> Self {
        // The name is random (review R13, S10). `create_dir` fails when the path exists, so the
        // test never writes into a directory that another process made.
        let directory = std::env::temp_dir().join(format!(
            "recipe-db-migrations-{}-{}",
            std::process::id(),
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&directory).expect("the directory is new");
        for (name, sql) in files {
            std::fs::write(directory.join(name), sql).expect("the file is written");
        }
        Self { directory }
    }

    /// The path of the directory.
    pub fn path(&self) -> &Path {
        &self.directory
    }
}

impl Drop for TempMigrations {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
