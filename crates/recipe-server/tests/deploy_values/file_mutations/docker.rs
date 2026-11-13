//! Context checks: the allow lists and the Dockerfile COPY sources of the pgtools and db images.

use crate::files::{
    DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST, PGTOOLS_DOCKERFILE, check_context_lists,
    check_library_copies,
};
use crate::inputs::Inputs;

/// One change of an allow list or of the Dockerfile: the file, the text to replace, the new text,
/// and the file that the fault must name.
type ContextCase = (&'static str, &'static str, &'static str, &'static str);

const CONTEXT_LISTS: [ContextCase; 9] = [
    // The directory source `bin/` needs its entry.
    (PGTOOLS_CONTEXT_LIST, "!bin\n", "", PGTOOLS_CONTEXT_LIST),
    // A suite of tests/ is mounted, not copied. The list never admits it.
    (
        PGTOOLS_CONTEXT_LIST,
        "!bin\n",
        "!bin\n!tests\n",
        PGTOOLS_CONTEXT_LIST,
    ),
    // A file in lib/ needs its own path. A whole directory admits too much.
    (
        PGTOOLS_CONTEXT_LIST,
        "!lib/restore-steps.sh\n",
        "!lib\n",
        PGTOOLS_CONTEXT_LIST,
    ),
    // One list admits each source.
    (
        PGTOOLS_CONTEXT_LIST,
        "!bin\n",
        "!bin\n!cluster-access.sh\n",
        PGTOOLS_CONTEXT_LIST,
    ),
    (
        DB_CONTEXT_LIST,
        "!cluster-access.sh\n",
        "!cluster-access.sh\n!bin\n",
        DB_CONTEXT_LIST,
    ),
    // The first line of code excludes everything.
    (DB_CONTEXT_LIST, "*\n", "!x\n", DB_CONTEXT_LIST),
    // A new COPY source without an entry, and an entry without a source.
    (
        PGTOOLS_DOCKERFILE,
        "COPY --chmod=0644 --from=db cluster-access.sh /etc/pgtools/cluster-access.sh",
        "COPY --chmod=0644 --from=db cluster-access.sh other.sh /etc/pgtools/",
        DB_CONTEXT_LIST,
    ),
    (
        PGTOOLS_DOCKERFILE,
        "crontab.backup crontab.walarchive",
        "crontab.backup",
        PGTOOLS_CONTEXT_LIST,
    ),
    // A COPY from the db context moves its source from one list to the other.
    (
        PGTOOLS_DOCKERFILE,
        "--from=db cluster-access.sh",
        "cluster-access.sh",
        DB_CONTEXT_LIST,
    ),
];

#[test]
fn a_context_list_that_differs_from_the_copy_sources_is_a_fault() {
    let base = Inputs::load();
    let files = [PGTOOLS_DOCKERFILE, DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST];
    let texts = files.map(|path| base.text(path).to_owned());
    for (file, from, to, named) in CONTEXT_LISTS {
        let mut changed = texts.clone();
        let index = files.iter().position(|path| *path == file).unwrap();
        assert!(
            changed[index].contains(from),
            "{file} does not hold {from:?}"
        );
        changed[index] = changed[index].replace(from, to);
        let report = check_context_lists(&changed[0], &changed[1], &changed[2]);
        assert!(
            report.has_fault_for(named),
            "no fault for {named:?} after {file}: {from:?} -> {to:?}\nfaults:\n{}",
            report.message()
        );
    }
}

#[test]
fn a_wildcard_source_or_an_unknown_from_is_a_fault() {
    let base = Inputs::load();
    let dockerfile = base.text(PGTOOLS_DOCKERFILE);
    let (db, pgtools) = (base.text(DB_CONTEXT_LIST), base.text(PGTOOLS_CONTEXT_LIST));
    for (from, to) in [
        ("crontab.backup crontab.walarchive", "crontab.* "),
        ("--from=db", "--from=other"),
    ] {
        let changed = dockerfile.replace(from, to);
        assert_ne!(changed, dockerfile, "the Dockerfile does not hold {from:?}");
        let report = check_context_lists(&changed, db, pgtools);
        assert!(
            report.has_fault_for(PGTOOLS_DOCKERFILE),
            "{from:?}: {}",
            report.message()
        );
    }
}

#[test]
fn a_copy_from_a_build_stage_needs_no_entry() {
    let base = Inputs::load();
    let dockerfile = format!(
        "FROM busybox AS tools\n{}\nCOPY --from=tools /bin/tool /usr/local/bin/tool\n",
        base.text(PGTOOLS_DOCKERFILE)
    );
    let report = check_context_lists(
        &dockerfile,
        base.text(DB_CONTEXT_LIST),
        base.text(PGTOOLS_CONTEXT_LIST),
    );
    assert!(report.faults.is_empty(), "{}", report.message());
}

/// One change of the Dockerfile or of the allow list of the pgtools context, and the fault that follows.
/// The fault must name the library file.
const LIBRARY_COPIES: [ContextCase; 5] = [
    // The Dockerfile drops the library, and the allow list drops it too. Only this check sees it.
    (
        PGTOOLS_DOCKERFILE,
        " lib/recovery-steps.sh",
        "",
        "lib/recovery-steps.sh",
    ),
    // The Dockerfile copies the library, and the allow list does not admit it.
    (
        PGTOOLS_CONTEXT_LIST,
        "!lib/recovery-steps.sh\n",
        "",
        "lib/recovery-steps.sh",
    ),
    // The library lands outside the directory of the other library files.
    (
        PGTOOLS_DOCKERFILE,
        " lib/wal-identity.sh lib/check-result.sh /etc/pgtools/",
        " lib/check-result.sh /etc/pgtools/\nCOPY --chmod=0644 lib/wal-identity.sh /usr/local/lib/",
        "lib/wal-identity.sh",
    ),
    // The new library lands outside that directory too.
    (
        PGTOOLS_DOCKERFILE,
        " lib/check-result.sh /etc/pgtools/",
        " /etc/pgtools/\nCOPY --chmod=0644 lib/check-result.sh /usr/local/lib/",
        "lib/check-result.sh",
    ),
    // A COPY from a stage does not count as a copy of the library.
    (
        PGTOOLS_DOCKERFILE,
        "COPY --chmod=0644 lib/globals-grammar.sh lib/recovery-steps.sh",
        "COPY --chmod=0644 --from=db lib/recovery-steps.sh /x/\nCOPY --chmod=0644 lib/globals-grammar.sh",
        "lib/recovery-steps.sh",
    ),
];

#[test]
fn a_library_file_that_the_pgtools_image_does_not_copy_or_admit_is_a_fault() {
    let base = Inputs::load();
    for (file, from, to, named) in LIBRARY_COPIES {
        let (mut dockerfile, mut list) = (
            base.text(PGTOOLS_DOCKERFILE).to_owned(),
            base.text(PGTOOLS_CONTEXT_LIST).to_owned(),
        );
        let target = if file == PGTOOLS_DOCKERFILE {
            &mut dockerfile
        } else {
            &mut list
        };
        assert!(target.contains(from), "{file} does not hold {from:?}");
        *target = target.replace(from, to);
        // The first case drops the library from the allow list too, so no other check sees it.
        if file == PGTOOLS_DOCKERFILE && from == " lib/recovery-steps.sh" {
            list = list.replace("!lib/recovery-steps.sh\n", "");
        }
        let report = check_library_copies(&dockerfile, &list, base.listing());
        assert!(
            report.has_fault_for(named),
            "no fault for {named:?} after {file}: {from:?} -> {to:?}
faults:
{}",
            report.message()
        );
    }
}

#[test]
fn the_pgtools_image_copies_and_admits_every_library_file() {
    let base = Inputs::load();
    let report = check_library_copies(
        base.text(PGTOOLS_DOCKERFILE),
        base.text(PGTOOLS_CONTEXT_LIST),
        base.listing(),
    );
    assert!(report.faults.is_empty(), "{}", report.message());
}
