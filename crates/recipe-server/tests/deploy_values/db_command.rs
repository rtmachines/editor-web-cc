//! The `command:` list of `db` as a closed grammar.
//!
//! PostgreSQL also accepts the long form `--name=value`, and a later argument overrides an earlier one.
//! So the list may hold only the item `postgres` first and the pairs `-c` and `name=value`.
//! Every other item is a fault, and so is a second pair for one setting.

use std::collections::BTreeMap;

use crate::fault::Report;

const EXPECTED_GRAMMAR: &str = "only `postgres` first and `-c name=value` pairs";

/// The setting that an item names: the name of a long form `--name=value`, or the item itself.
fn setting_of(item: &str) -> &str {
    item.strip_prefix("--")
        .and_then(|long| long.split_once('='))
        .map_or(item, |(name, _)| name)
}

/// The `name=value` pairs of the `command:` list, each after its `-c`.
/// A fault names the setting of every item outside the grammar and of every repeated setting.
pub fn command_settings<'a>(
    file: &str,
    items: &[&'a str],
    report: &mut Report,
) -> BTreeMap<&'a str, &'a str> {
    let rest = match items.split_first() {
        Some((&"postgres", rest)) => rest,
        _ => items,
    };
    let mut settings = BTreeMap::new();
    let mut iter = rest.iter().copied();
    while let Some(item) = iter.next() {
        if item != "-c" {
            let key = format!("db.command {} (item outside the pairs)", setting_of(item));
            report.fail(file, &key, EXPECTED_GRAMMAR, format!("the item {item}"));
            continue;
        }
        let next = iter.next();
        let Some((name, value)) = next.and_then(|pair| pair.split_once('=')) else {
            let found =
                next.map_or_else(|| "nothing".to_owned(), |pair| format!("the item {pair}"));
            report.fail(file, "db.command -c (pair)", "`-c name=value`", found);
            continue;
        };
        if settings.insert(name, value).is_some() {
            let key = format!("db.command {name} (second -c pair)");
            report.fail(file, &key, "one `-c` pair for the setting", "a second pair");
        }
    }
    settings
}
