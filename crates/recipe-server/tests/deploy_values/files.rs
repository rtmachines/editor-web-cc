//! Checks of the values in `deploy/.env.example`, the Caddyfiles, the crontab files, and the pgtools scripts.

#[path = "files/caddy.rs"]
mod caddy;
#[path = "files/crontab.rs"]
mod crontab;
#[path = "files/docker.rs"]
mod docker;
#[path = "files/env.rs"]
mod env;
#[path = "files/line_endings.rs"]
mod line_endings;
#[path = "files/scripts.rs"]
mod scripts;

pub use caddy::check_caddyfile;
pub use crontab::check_crontab;
pub use docker::{
    DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST, PGTOOLS_DOCKERFILE, check_context_lists,
    check_library_copies,
};
pub use env::{check_env_example, parse_env};
pub use line_endings::check_line_endings;
pub use scripts::{check_no_fsync, check_scratch_settings, check_script_assignments};
