//! A random secret and a temporary directory for the tests that run the built binary.

use std::hash::{BuildHasher, Hasher, RandomState};
use std::ops::Deref;
use std::path::{Path, PathBuf};

const SECRET_WORDS: usize = 4;
const DIRECTORY_PREFIX: &str = "recipe-server-test";

/// A random hexadecimal secret of 64 characters.
///
/// `RandomState` takes its keys from the operating system, so no new crate is needed.
pub fn random_secret() -> String {
    (0..SECRET_WORDS)
        .map(|_| format!("{:016x}", RandomState::new().build_hasher().finish()))
        .collect()
}

/// A temporary directory that the drop removes.
///
/// The type derefs to its path, so a test joins a file name with `dir.join(name)`.
pub struct TempDir(PathBuf);

impl TempDir {
    /// Make a directory with a random name. `create_dir` fails when the path exists.
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "{DIRECTORY_PREFIX}-{}-{}",
            std::process::id(),
            random_secret()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    /// Write a file into the directory and return its path.
    pub fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }
}

impl Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
