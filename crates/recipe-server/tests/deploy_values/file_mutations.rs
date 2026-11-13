//! Proof that each group of checks on the Caddyfiles, the crontab files, and the pgtools scripts can fail.
//!
//! Each case changes one copy of a text and asserts a fault. No file of `deploy/` changes.

#[path = "file_mutations/caddy.rs"]
mod caddy;
#[path = "file_mutations/crontab.rs"]
mod crontab;
#[path = "file_mutations/docker.rs"]
mod docker;
#[path = "file_mutations/line_endings.rs"]
mod line_endings;
#[path = "file_mutations/scripts.rs"]
mod scripts;
#[path = "file_mutations/volumes.rs"]
mod volumes;
