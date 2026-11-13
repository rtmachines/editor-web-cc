use std::collections::BTreeMap;

use crate::fault::Report;
use crate::lines::code_lines;

const ENV_FILE: &str = "deploy/.env.example";

/// The nine values of the Compose interpolation (`reliability.md` 7.1, "Secrets", ADR-0103 (d), and design D9).
/// The file holds no password, key, or token.
const ENV_KEYS: [&str; 9] = [
    "COMPOSE_IP_RANGE",
    "COMPOSE_SUBNET",
    "PROXY_ADDRESS",
    "RECIPE_BACKUP_DIR",
    "RECIPE_PGDATA_VOLUME",
    "RECIPE_SECRETS_DIR",
    "RECIPE_VERSION",
    "RECIPE_WALARCHIVE_DIR",
    "SITE_ADDRESS",
];

const VOLUME_KEY: &str = "RECIPE_PGDATA_VOLUME";

const VOLUME_PREFIX: &str = "recipe-editor_pgdata_";

/// The addresses of design D9 (the row `deploy/.env.example`) and of `reliability.md` 7.11 (`http.trusted_proxies`).
const ENV_ADDRESSES: [(&str, &str); 3] = [
    ("COMPOSE_SUBNET", "172.28.0.0/16"),
    ("COMPOSE_IP_RANGE", "172.28.1.0/24"),
    ("PROXY_ADDRESS", "172.28.0.2"),
];

/// The `NAME=value` lines of an env file.
pub fn parse_env(text: &str) -> BTreeMap<&str, &str> {
    code_lines(text)
        .into_iter()
        .filter_map(|line| line.trim().split_once('='))
        .collect()
}

/// The file holds exactly the nine values, its header counts them, and the three addresses equal design D9.
/// The volume name has the form of ADR-0103 (e).
pub fn check_env_example(text: &str) -> Report {
    let mut report = Report::default();
    let env = parse_env(text);
    check_env_header(&mut report, text);
    check_volume_name(&mut report, &env);
    let found: Vec<String> = env.keys().map(ToString::to_string).collect();
    let want: Vec<String> = ENV_KEYS.map(String::from).to_vec();
    report.compare_list(ENV_FILE, "the names of the values", &want, Some(found));
    for (name, value) in ENV_ADDRESSES {
        let found = env.get(name).copied();
        if found != Some(value) {
            report.fail(ENV_FILE, name, value, found.unwrap_or("missing"));
        }
    }
    report
}

/// The header line `# The file holds <count> values and no password, key, or token.` names the count of `ENV_KEYS`.
fn check_env_header(report: &mut Report, text: &str) {
    let header = text
        .lines()
        .find(|line| line.starts_with("# The file holds "))
        .unwrap_or("missing");
    if !header.starts_with("# The file holds nine values ") {
        report.fail(
            ENV_FILE,
            "header count",
            "# The file holds nine values",
            header,
        );
    }
}

/// `RECIPE_PGDATA_VOLUME` is `recipe-editor_pgdata_<YYYYMMDD_HHMMSS>` (ADR-0103 (e)).
fn check_volume_name(report: &mut Report, env: &BTreeMap<&str, &str>) {
    let found = env.get(VOLUME_KEY).copied().unwrap_or("missing");
    let stamp = found.strip_prefix(VOLUME_PREFIX).unwrap_or("");
    let is_stamp = stamp.split_once('_').is_some_and(|(date, time)| {
        date.len() == 8
            && time.len() == 6
            && date.chars().chain(time.chars()).all(|c| c.is_ascii_digit())
    });
    if !is_stamp {
        report.fail(
            ENV_FILE,
            VOLUME_KEY,
            "recipe-editor_pgdata_<YYYYMMDD_HHMMSS>",
            found,
        );
    }
}
