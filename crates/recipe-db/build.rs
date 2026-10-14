//! Build script of `recipe-db`.
//!
//! `sqlx::migrate!` embeds the directory `migrations/` at compile time. Cargo does not see a
//! new or changed file there, so this script tells it to build the crate again.

use std::io::Write as _;

fn main() {
    // A failed write to stdout ends the build step with the missing line. Nothing else can react.
    let _ = writeln!(std::io::stdout(), "cargo:rerun-if-changed=../../migrations");
}
