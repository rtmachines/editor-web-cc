//! Line ending checks: the `.gitattributes` rules of the files that the images copy.

use crate::files::{PGTOOLS_DOCKERFILE, check_line_endings};
use crate::inputs::{GITATTRIBUTES_FILE, Inputs, Mutation, assert_mutations_fail};

const LINE_ENDINGS: [Mutation; 8] = [
    // A file that a COPY line names needs its own rule.
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-code.expected text eol=lf\n",
        "",
        "restore-code.expected",
    ),
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-roles.expected text eol=lf\n",
        "",
        "restore-roles.expected",
    ),
    // The rule needs both attributes.
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-code.expected text eol=lf",
        "deploy/pgtools/restore-code.expected eol=lf",
        "restore-code.expected",
    ),
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-code.expected text eol=lf",
        "deploy/pgtools/restore-code.expected text eol=crlf",
        "restore-code.expected",
    ),
    // A pattern for the files of a directory source covers each file of it.
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/bin/* text eol=lf\n",
        "",
        "deploy/pgtools/bin/restore-test",
    ),
    // A basename pattern covers a file of a directory and a file of the other context.
    (
        GITATTRIBUTES_FILE,
        "*.sh text eol=lf\n",
        "",
        "cluster-access.sh",
    ),
    (
        GITATTRIBUTES_FILE,
        "crontab.* text eol=lf\n",
        "",
        "crontab.walarchive",
    ),
    // The two allow lists need a rule.
    (
        GITATTRIBUTES_FILE,
        "deploy/db/.dockerignore text eol=lf\n",
        "",
        "deploy/db/.dockerignore",
    ),
];

#[test]
fn a_copy_source_without_a_line_ending_rule_is_a_fault() {
    assert_mutations_fail(&LINE_ENDINGS);
}

#[test]
fn a_copy_source_that_does_not_exist_is_a_fault() {
    let base = Inputs::load();
    let changed = base
        .text(PGTOOLS_DOCKERFILE)
        .replace("restore-code.sql", "missing-file.sql");
    let report = check_line_endings(&changed, base.text(GITATTRIBUTES_FILE), base.listing());
    assert!(
        report.has_fault_for("missing-file.sql"),
        "{}",
        report.message()
    );
}
