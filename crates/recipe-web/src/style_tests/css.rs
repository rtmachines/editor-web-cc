//! Test helpers: a strict reader for the flat CSS format of `app.css` and for the tables of
//! `design-system.md` (design D4).
//!
//! The reader accepts a flat list of rules, `@font-face` blocks, and one level of `@media`. It
//! fails on every other construct, so a test never passes because the reader skipped a rule.

/// One rule of the style sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Rule {
    /// The text after `@media`, or `None` for a rule at the top level.
    pub(super) media: Option<String>,
    /// The comma-separated selectors with collapsed white space. `@font-face` for a font block.
    pub(super) selectors: Vec<String>,
    /// The declarations in source order, with collapsed white space in the values.
    pub(super) decls: Vec<(String, String)>,
}

impl Rule {
    /// Return the value of the last declaration with this property name.
    pub(super) fn value(&self, property: &str) -> Option<&str> {
        self.decls
            .iter()
            .rev()
            .find(|(name, _value)| name == property)
            .map(|(_name, value)| value.as_str())
    }
}

/// Collapse every run of white space into one space and trim the ends.
pub(super) fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove the comments of a style sheet.
fn strip_comments(css: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = css;
    while let Some((before, after)) = rest.split_once("/*") {
        out.push_str(before);
        let (_comment, tail) = after
            .split_once("*/")
            .ok_or_else(|| "comment without end".to_owned())?;
        out.push(' ');
        rest = tail;
    }
    out.push_str(rest);
    Ok(out)
}

/// Split the declarations of one block.
fn parse_decls(body: &str) -> Result<Vec<(String, String)>, String> {
    let mut decls = Vec::new();
    for part in body.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, value) = part
            .split_once(':')
            .ok_or_else(|| format!("declaration without colon: {part}"))?;
        decls.push((name.trim().to_owned(), collapse(value)));
    }
    Ok(decls)
}

/// Return the text before the brace that closes a block, and the text after it.
fn split_block(after_open: &str) -> Result<(&str, &str), String> {
    let mut depth = 1_u32;
    for (index, ch) in after_open.char_indices() {
        match ch {
            '{' => depth = depth.checked_add(1).ok_or("depth overflow")?,
            '}' => {
                depth = depth.checked_sub(1).ok_or("depth underflow")?;
                if depth == 0 {
                    let (body, tail) = after_open.split_at(index);
                    let tail = tail.strip_prefix('}').ok_or("missing closing brace")?;
                    return Ok((body, tail));
                }
            }
            _ => {}
        }
    }
    Err("block without end".to_owned())
}

/// Read the rules of one level.
fn parse_level(text: &str, media: Option<&str>, out: &mut Vec<Rule>) -> Result<(), String> {
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (head, after) = rest
            .split_once('{')
            .ok_or_else(|| format!("rule without block: {rest}"))?;
        if head.contains('}') {
            return Err(format!("stray closing brace before: {head}"));
        }
        let (body, tail) = split_block(after)?;
        let head = collapse(head);
        if let Some(condition) = head.strip_prefix("@media") {
            if media.is_some() {
                return Err("nested @media".to_owned());
            }
            parse_level(body, Some(condition.trim()), out)?;
        } else if head.starts_with('@') && head != "@font-face" {
            return Err(format!("unsupported at-rule: {head}"));
        } else {
            if body.contains('{') {
                return Err(format!("nested block in: {head}"));
            }
            out.push(Rule {
                media: media.map(str::to_owned),
                selectors: head.split(',').map(collapse).collect(),
                decls: parse_decls(body)?,
            });
        }
        rest = tail.trim();
    }
    Ok(())
}

/// Read a flat style sheet. It fails on a construct that the format does not allow.
pub(super) fn parse_sheet(css: &str) -> Result<Vec<Rule>, String> {
    let clean = strip_comments(css)?;
    if clean.contains("@import") {
        return Err("@import".to_owned());
    }
    let mut rules = Vec::new();
    parse_level(&clean, None, &mut rules)?;
    Ok(rules)
}

/// Return the rows of every table whose header cells equal `header`, as trimmed cells without
/// the backticks. A row with fewer cells than the header is an error.
pub(super) fn parse_tables(markdown: &str, header: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let wanted = format!("| {} |", header.join(" | "));
    let mut rows = Vec::new();
    let mut inside = false;
    let mut skipped_separator = false;
    for line in markdown.lines() {
        if line.trim_end() == wanted {
            inside = true;
            skipped_separator = false;
        } else if inside && !skipped_separator {
            skipped_separator = true;
        } else if inside && line.starts_with('|') {
            let cells: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect();
            if cells.len() != header.len() {
                return Err(format!("bad row: {line}"));
            }
            rows.push(cells);
        } else {
            inside = false;
        }
    }
    Ok(rows)
}

/// The named colors of CSS Color Level 4, in lower case.
const NAMED_COLORS: [&str; 148] = [
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

/// Return true when a word is a named color of CSS, in any case.
pub(super) fn is_named_color(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    NAMED_COLORS.contains(&lower.as_str())
}

/// Return the relative luminance of a color `#RRGGBB` (WCAG 2.2).
pub(super) fn luminance(hex: &str) -> Result<f64, String> {
    let digits = hex
        .strip_prefix('#')
        .filter(|digits| digits.len() == 6)
        .ok_or_else(|| format!("not a #RRGGBB color: {hex}"))?;
    let mut linear = Vec::new();
    for index in [0_usize, 2, 4] {
        let end = index.checked_add(2).ok_or("index overflow")?;
        let pair = digits.get(index..end).ok_or("short color")?;
        let value = u8::from_str_radix(pair, 16).map_err(|error| error.to_string())?;
        let channel = f64::from(value) / 255.0;
        linear.push(if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        });
    }
    match linear.as_slice() {
        [r, g, b] => Ok(0.2126 * r + 0.7152 * g + 0.0722 * b),
        _ => Err("channel count".to_owned()),
    }
}

/// Return the WCAG contrast ratio of two colors. The divisor is checked first.
pub(super) fn contrast_ratio(first: &str, second: &str) -> Result<f64, String> {
    let (a, b) = (luminance(first)?, luminance(second)?);
    let (light, dark) = if a >= b { (a, b) } else { (b, a) };
    let divisor = dark + 0.05;
    if divisor <= 0.0 {
        return Err("zero divisor".to_owned());
    }
    Ok((light + 0.05) / divisor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_reader_reads_rules_font_faces_and_one_media_level() {
        let css = "/* c */ a, b  c { x: 1;  y : 2 3 } @font-face { z: q } \
                   @media (forced-colors: active) { a { x: 9 } }";
        let rules = parse_sheet(css).unwrap();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].selectors, vec!["a", "b c"]);
        assert_eq!(rules[0].value("y"), Some("2 3"));
        assert_eq!(rules[1].selectors, vec!["@font-face"]);
        assert_eq!(rules[2].media.as_deref(), Some("(forced-colors: active)"));
    }

    #[test]
    fn css_reader_rejects_import_nesting_and_other_at_rules() {
        assert!(parse_sheet("@import url(x.css);").is_err());
        assert!(parse_sheet("a { b { c: d } }").is_err());
        assert!(parse_sheet("@media x { @media y { a { b: c } } }").is_err());
        assert!(parse_sheet("@keyframes k { a { b: c } }").is_err());
        assert!(parse_sheet("a { b }").is_err());
        assert!(parse_sheet("a { b: c ").is_err());
    }

    #[test]
    fn css_reader_rejects_a_stray_closing_brace() {
        assert!(parse_sheet("a { b: c } }").is_err());
        assert!(parse_sheet("a { b: c }} d { e: f }").is_err());
        assert!(parse_sheet("} a { b: c }").is_err());
        assert!(parse_sheet("@media x { a { b: c } } }").is_err());
        assert!(parse_sheet("a { b: c } d { e: f }").is_ok());
    }

    #[test]
    fn table_reader_returns_the_rows_of_the_matching_header_only() {
        let doc = "| Token | Value | Use |\n|---|---|---|\n| `--a` | `1` | x. y |\n\
                   | `--b` | `2 3` | z |\n\ntext\n\n| Other | Value |\n|---|---|\n| q | r |\n";
        let rows = parse_tables(doc, &["Token", "Value", "Use"]).unwrap();
        assert_eq!(
            rows,
            vec![vec!["--a", "1", "x. y"], vec!["--b", "2 3", "z"]]
        );
        assert!(parse_tables("| A | B |\n|---|---|\n| x |\n", &["A", "B"]).is_err());
    }

    #[test]
    fn contrast_of_black_on_white_is_21_and_of_equal_colors_is_1() {
        let ratio = contrast_ratio("#000000", "#FFFFFF").unwrap();
        assert!((ratio - 21.0).abs() < 1e-9, "{ratio}");
        let same = contrast_ratio("#4A5361", "#4A5361").unwrap();
        assert!((same - 1.0).abs() < 1e-9, "{same}");
        assert!(luminance("red").is_err());
    }
}
