use crate::fault::Report;
use crate::lines::code_lines;
use crate::table::Table;

/// The cron prefix `minute hour * * weekday` and the zone word of a time such as `Sunday 03:30 UTC`.
fn cron_schedule(raw: &str) -> Option<(String, String)> {
    let cleaned = raw.replace('`', "");
    let mut words = cleaned.split_whitespace();
    let first = words.next()?;
    let (weekday, time) = if first == "Sunday" {
        ("0", words.next()?)
    } else {
        ("*", first)
    };
    let (hour, minute) = time.split_once(':')?;
    let (hour, minute): (u32, u32) = (hour.parse().ok()?, minute.parse().ok()?);
    let zone = words
        .next()?
        .trim_matches(|c: char| !(c.is_alphanumeric() || c == '/' || c == '_'));
    // A zone word has letters, `/`, and `_` only, so `ADR-0067` after a time is no zone.
    let is_zone = !zone.is_empty()
        && zone
            .chars()
            .all(|c| c.is_alphabetic() || c == '/' || c == '_');
    is_zone.then(|| (format!("{minute} {hour} * * {weekday}"), zone.to_owned()))
}

/// A crontab file runs each command at the time of its key in 7.11, in the zone of the document.
/// The file holds those jobs and no other job, each once. `reliability.md` 7.11, ADR-0066.
pub fn check_crontab(file: &str, text: &str, entries: &[(&str, &str)], table: &Table) -> Report {
    let mut report = Report::default();
    let lines: Vec<&str> = code_lines(text).into_iter().map(str::trim).collect();
    let mut wanted: Vec<String> = Vec::new();
    let mut zones: Vec<String> = Vec::new();
    for (key, command) in entries {
        report.cover(key);
        let Some((prefix, zone)) = table.raw(key).and_then(cron_schedule) else {
            report.fail(file, key, "a time and a zone in 7.11", "no time or no zone");
            continue;
        };
        let want = format!("{prefix} {command}");
        if !lines.contains(&want.as_str()) {
            let found = lines
                .iter()
                .find(|line| line.ends_with(command))
                .copied()
                .unwrap_or("no line");
            report.fail(file, &format!("{key} ({command})"), want.as_str(), found);
        }
        wanted.push(want);
        let zone_line = format!("CRON_TZ={zone}");
        if !zones.contains(&zone_line) {
            zones.push(zone_line);
        }
    }
    check_cron_zones(&mut report, file, &lines, &zones);
    check_cron_extra_jobs(&mut report, file, &lines, wanted);
    report
}

/// The file sets the zone once per expected zone. A later `CRON_TZ=` would change the zone below it.
fn check_cron_zones(report: &mut Report, file: &str, lines: &[&str], want: &[String]) {
    let found: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.starts_with("CRON_TZ="))
        .collect();
    if found != want {
        report.fail(file, "CRON_TZ", format!("{want:?}"), format!("{found:?}"));
    }
}

/// Every code line except the zone lines is an expected job. Each expected job matches one line.
fn check_cron_extra_jobs(report: &mut Report, file: &str, lines: &[&str], wanted: Vec<String>) {
    let mut jobs: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.starts_with("CRON_TZ="))
        .collect();
    for want in &wanted {
        if let Some(at) = jobs.iter().position(|line| *line == want.as_str()) {
            jobs.remove(at);
        }
    }
    for extra in jobs {
        report.fail(file, "unexpected cron job", "no other job", extra);
    }
}
