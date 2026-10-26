//! A random secret and a temporary directory for the tests that run the built binary.

use std::hash::{BuildHasher, Hasher, RandomState};
use std::ops::Deref;
use std::path::{Path, PathBuf};

const SECRET_WORDS: usize = 4;
/// Only the owner may enter a temporary directory. The secret files of a test live in it.
#[cfg(unix)]
const DIRECTORY_MODE: u32 = 0o700;
/// Only the owner may read or write a temporary file.
#[cfg(unix)]
const FILE_MODE: u32 = 0o600;
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
        create_directory(&path);
        Self(path)
    }

    /// Write a file into the directory and return its path.
    pub fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        write_file(&path, content);
        path
    }
}

/// Create a new directory that only the owner may enter. The call fails when the path exists.
#[cfg(unix)]
fn create_directory(path: &Path) {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new()
        .mode(DIRECTORY_MODE)
        .create(path)
        .unwrap();
}

/// Create a new directory. The call fails when the path exists.
#[cfg(not(unix))]
fn create_directory(path: &Path) {
    std::fs::create_dir(path).unwrap();
}

/// Write a file that only the owner may read or write. An existing file gets the new content.
#[cfg(unix)]
fn write_file(path: &Path, content: &str) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

/// Write a file. An existing file gets the new content.
#[cfg(not(unix))]
fn write_file(path: &Path, content: &str) {
    std::fs::write(path, content).unwrap();
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
