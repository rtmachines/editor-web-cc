//! Faults found by a check. A fault names the file, the key, the expected value, and the found value.

use std::collections::BTreeSet;
use std::fmt;

use crate::canon::{Bare, Canon, canon_whole};

/// One difference between a file of `deploy/` and a document.
pub struct Fault {
    pub file: String,
    pub key: String,
    pub expected: String,
    pub found: String,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: key {}: expected {}, found {}",
            self.file, self.key, self.expected, self.found
        )
    }
}

/// The result of a group of checks: the faults and the keys of `reliability.md` 7.11 that a check read.
#[derive(Default)]
pub struct Report {
    pub faults: Vec<Fault>,
    pub covered: BTreeSet<String>,
}

fn contains_name(text: &str, part: &str) -> bool {
    text.match_indices(part).any(|(start, _)| {
        let before = text.get(..start).and_then(|head| head.chars().last());
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
    })
}

/// What a check expects: the canonical value and the text to show in a fault.
pub type Expected = Option<(Canon, String)>;

impl Report {
    /// Record one fault.
    pub fn fail(
        &mut self,
        file: &str,
        key: &str,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) {
        self.faults.push(Fault {
            file: file.to_owned(),
            key: key.to_owned(),
            expected: expected.into(),
            found: found.into(),
        });
    }

    /// Mark a key of the table as read by a check.
    pub fn cover(&mut self, table_key: &str) {
        self.covered.insert(table_key.to_owned());
    }

    /// Join the faults and the covered keys of another report.
    pub fn absorb(&mut self, other: Report) {
        self.faults.extend(other.faults);
        self.covered.extend(other.covered);
    }

    /// Compare a found text with an expected value after both go to their canonical form.
    pub fn compare(
        &mut self,
        file: &str,
        label: &str,
        expected: Expected,
        found: Option<&str>,
        bare: Bare,
    ) {
        let Some((want, shown)) = expected else {
            self.fail(file, label, "a value in the document", "no value");
            return;
        };
        let Some(text) = found else {
            self.fail(file, label, shown, "missing");
            return;
        };
        if canon_whole(text, bare).as_ref() != Some(&want) {
            self.fail(file, label, shown, text);
        }
    }

    /// Compare a found list with an expected list.
    pub fn compare_list(
        &mut self,
        file: &str,
        label: &str,
        expected: &[String],
        found: Option<Vec<String>>,
    ) {
        match found {
            Some(list) if list == expected => {}
            Some(list) => self.fail(file, label, format!("{expected:?}"), format!("{list:?}")),
            None => self.fail(file, label, format!("{expected:?}"), "missing"),
        }
    }

    /// True when a fault has a key or a file that contains `part` as a whole name.
    /// The text before `part` is empty or ends in a mark other than a letter, a digit, `_`, or `-`.
    /// So `migrate.mem_limit` does not match `backup-premigrate.mem_limit`.
    pub fn has_fault_for(&self, part: &str) -> bool {
        self.faults
            .iter()
            .any(|fault| contains_name(&fault.key, part) || contains_name(&fault.file, part))
    }

    /// All faults, one for each line.
    pub fn message(&self) -> String {
        let lines: Vec<String> = self.faults.iter().map(ToString::to_string).collect();
        lines.join("\n")
    }
}
