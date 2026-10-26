//! Scanner for the operators in the body of a function (ADR-0098 (f)).
//!
//! A function of `audit_owner` runs as `audit_owner`, or a trigger runs it with the rights of the
//! session. A bare operator is resolved through the `search_path`, so the scanner requires the
//! form `OPERATOR(pg_catalog.<op>)` for each operator. The scan reads `prosrc`, the body only.
//!
//! The scanner skips what is no operator lookup:
//!
//! - comments, string literals, quoted identifiers, and dollar-quoted text
//! - `::`, `:=`, and `=>`
//! - the `=` of `GET DIAGNOSTICS`
//! - the `=` of an assignment after `SET` or `USING`, such as `SET a = 1` or `RAISE ... USING x = 1`
//! - the `*` of `count(*)`, `t.*`, and `SELECT *`
//! - each group `OPERATOR(pg_catalog.…)`
//!
//! These words and phrases count as operators, because the parser resolves each one to an operator
//! through the `search_path`:
//!
//! - `IN`, `LIKE`, `ILIKE`, `BETWEEN`, and `SIMILAR`
//! - `IS DISTINCT FROM` and `IS NOT DISTINCT FROM`, which look up `=`
//! - `NULLIF(a, b)`, which looks up `=`
//! - `CASE x WHEN`, the simple form of `CASE`, which looks up `=`. A `CASE` that starts with
//!   `WHEN` is no hit. A comment between `CASE` and the next word makes a false hit.
//!
//! `GROUP BY`, `ORDER BY`, `DISTINCT`, `UNION`, `GREATEST`, and `min()` use the default btree
//! operator class of the type, not a name in the `search_path`. They are no hit. The qualified
//! form of `ORDER BY x USING <op>` is out of scope, because the `<op>` is an operator hit.

const OPERATOR_CHARACTERS: &str = "+-*/<>=~!@#%^&|`?";
/// A character in an operator that stops the removal of a trailing `+` or `-`.
const SPECIAL_CHARACTERS: &str = "~!@#%^&|`?";
const WORD_OPERATORS: [&str; 6] = ["in", "like", "ilike", "between", "similar", "nullif"];
/// A word that opens a list of `name = value` items.
const ASSIGNMENT_WORDS: [&str; 2] = ["set", "using"];
/// A word that ends such a list.
const LIST_END_WORDS: [&str; 5] = ["where", "from", "returning", "select", "on"];
/// A word before a `*` that means "all columns".
const STAR_WORDS: [&str; 4] = ["select", "returning", "distinct", "all"];
const QUALIFIED_PREFIX: &str = "pg_catalog.";

/// What the scanner saw last, apart from white space and comments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Previous {
    #[default]
    Other,
    Word(String),
    Open,
    Comma,
    Dot,
}

#[derive(Default)]
struct Scan {
    chars: Vec<char>,
    position: usize,
    line: usize,
    hits: Vec<String>,
    previous: Previous,
    /// The word before `previous`, when `previous` is a word.
    word_before: Option<String>,
    depth: usize,
    /// True inside `SET` or `USING`, as long as no list-end word came.
    in_list: bool,
    list_depth: usize,
    /// True when the next `=` at the depth of the list is an assignment.
    assignment_next: bool,
    in_diagnostics: bool,
}

fn is_word_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

impl Scan {
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars
            .get(self.position.saturating_add(offset))
            .copied()
    }

    fn advance(&mut self) {
        if self.peek_at(0) == Some('\n') {
            self.line = self.line.saturating_add(1);
        }
        self.position = self.position.saturating_add(1);
    }

    fn hit(&mut self, what: &str) {
        self.hits.push(format!("{what} at line {}", self.line));
    }

    /// Skip a `--` comment. The cursor stands on the first dash.
    fn skip_line_comment(&mut self) {
        while self.peek_at(0).is_some_and(|ch| ch != '\n') {
            self.advance();
        }
    }

    /// Skip a nested `/* ... */` comment. The cursor stands on the slash.
    fn skip_block_comment(&mut self) {
        let mut depth: usize = 0;
        while let Some(ch) = self.peek_at(0) {
            match (ch, self.peek_at(1)) {
                ('/', Some('*')) => {
                    depth = depth.saturating_add(1);
                    self.advance();
                }
                ('*', Some('/')) => {
                    depth = depth.saturating_sub(1);
                    self.advance();
                    if depth == 0 {
                        self.advance();
                        return;
                    }
                }
                _ => {}
            }
            self.advance();
        }
        self.hit("unterminated comment");
    }

    /// Skip a quoted text. The cursor stands on the opening quote. A doubled quote is no end.
    /// With `backslash`, a backslash hides the next character, as in `E'...'`.
    fn skip_quoted(&mut self, quote: char, backslash: bool) {
        self.advance();
        while let Some(ch) = self.peek_at(0) {
            if backslash && ch == '\\' {
                self.advance();
            } else if ch == quote {
                if self.peek_at(1) == Some(quote) {
                    self.advance();
                } else {
                    self.advance();
                    return;
                }
            }
            self.advance();
        }
        self.hit("unterminated literal");
    }

    /// True when the quote at the cursor opens an escape string `E'...'`.
    fn is_escape_string(&self) -> bool {
        let Some(before) = self.position.checked_sub(1).and_then(|i| self.chars.get(i)) else {
            return false;
        };
        let word_before = self
            .position
            .checked_sub(2)
            .and_then(|i| self.chars.get(i))
            .is_some_and(|ch| is_word_char(*ch));
        matches!(before, 'e' | 'E') && !word_before
    }

    /// Skip `$tag$ ... $tag$` or a parameter such as `$1`. The cursor stands on the dollar.
    fn skip_dollar(&mut self) {
        let mut end = self.position.saturating_add(1);
        while self
            .chars
            .get(end)
            .is_some_and(|ch| is_word_char(*ch) && *ch != '$')
        {
            end = end.saturating_add(1);
        }
        let tag_is_valid = self
            .chars
            .get(self.position.saturating_add(1))
            .is_none_or(|ch| !ch.is_ascii_digit());
        if self.chars.get(end) != Some(&'$') || !tag_is_valid {
            self.advance();
            while self.peek_at(0).is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance();
            }
            return;
        }
        let delimiter: String = self
            .chars
            .get(self.position..=end)
            .unwrap_or_default()
            .iter()
            .collect();
        let rest: String = self
            .chars
            .get(self.position..)
            .unwrap_or_default()
            .iter()
            .collect();
        let closing = rest
            .get(delimiter.len()..)
            .and_then(|tail| tail.find(&delimiter));
        let Some(offset) = closing else {
            self.hit("unterminated literal");
            self.position = self.chars.len();
            return;
        };
        let skipped = rest
            .get(
                ..delimiter
                    .len()
                    .saturating_add(offset)
                    .saturating_add(delimiter.len()),
            )
            .unwrap_or_default();
        for _ in skipped.chars() {
            self.advance();
        }
    }

    fn read_word(&mut self) -> String {
        let mut word = String::new();
        while let Some(ch) = self.peek_at(0).filter(|ch| is_word_char(*ch)) {
            word.push(ch);
            self.advance();
        }
        word
    }

    fn skip_white_space(&mut self) {
        while self.peek_at(0).is_some_and(char::is_whitespace) {
            self.advance();
        }
    }

    /// Read `( ... )` after the word `OPERATOR`. Return false when no group follows.
    fn read_operator_group(&mut self) -> bool {
        let saved = (self.position, self.line);
        self.skip_white_space();
        if self.peek_at(0) != Some('(') {
            (self.position, self.line) = saved;
            return false;
        }
        let line = self.line;
        self.advance();
        let mut content = String::new();
        while let Some(ch) = self.peek_at(0) {
            self.advance();
            if ch == ')' {
                if !content.trim_start().starts_with(QUALIFIED_PREFIX) {
                    self.hits
                        .push(format!("OPERATOR({content}) at line {line}"));
                }
                return true;
            }
            content.push(ch);
        }
        self.hit("unterminated OPERATOR group");
        true
    }

    /// The hit of the word `DISTINCT` after `IS` or `IS NOT`. `SELECT DISTINCT` is no hit.
    fn distinct_hit(&self) -> Option<&'static str> {
        match (&self.previous, &self.word_before) {
            (Previous::Word(word), _) if word == "is" => Some("IS DISTINCT FROM"),
            (Previous::Word(word), Some(before)) if word == "not" && before == "is" => {
                Some("IS NOT DISTINCT FROM")
            }
            _ => None,
        }
    }

    /// True when the word after the cursor is not `WHEN`. Then `CASE` has an operand.
    fn case_has_operand(&mut self) -> bool {
        let saved = (self.position, self.line);
        self.skip_white_space();
        let next = self.read_word();
        (self.position, self.line) = saved;
        !next.eq_ignore_ascii_case("when")
    }

    fn word(&mut self) {
        let word = self.read_word();
        let lower = word.to_ascii_lowercase();
        if lower == "operator" && self.read_operator_group() {
            self.previous = Previous::Other;
            return;
        }
        if WORD_OPERATORS.contains(&lower.as_str()) {
            self.hit(&word.to_ascii_uppercase());
        }
        if lower == "distinct"
            && let Some(what) = self.distinct_hit()
        {
            self.hit(what);
        }
        let after_end = self.previous == Previous::Word("end".to_owned());
        if lower == "case" && !after_end && self.case_has_operand() {
            self.hit("CASE x WHEN");
        }
        if lower == "diagnostics" {
            self.in_diagnostics = true;
        }
        if ASSIGNMENT_WORDS.contains(&lower.as_str()) {
            self.in_list = true;
            self.list_depth = self.depth;
            self.assignment_next = true;
        }
        if LIST_END_WORDS.contains(&lower.as_str()) {
            self.in_list = false;
            self.assignment_next = false;
        }
        self.word_before = match &self.previous {
            Previous::Word(before) => Some(before.clone()),
            _ => None,
        };
        self.previous = Previous::Word(lower);
    }

    fn number(&mut self) {
        while let Some(ch) = self.peek_at(0) {
            let exponent_sign = matches!(ch, 'e' | 'E')
                && matches!(self.peek_at(1), Some('+' | '-'))
                && self.peek_at(2).is_some_and(|digit| digit.is_ascii_digit());
            if exponent_sign {
                self.advance();
                self.advance();
            } else if ch.is_ascii_alphanumeric()
                || ch == '_'
                || (ch == '.' && self.peek_at(1) != Some('.'))
            {
                self.advance();
            } else {
                break;
            }
        }
        self.previous = Previous::Other;
    }

    /// The text of an operator that starts at the cursor. The run ends before `--` and `/*`.
    /// A trailing `+` or `-` goes back to the input, unless the run holds a special character.
    fn read_operator(&mut self) -> String {
        let mut run = String::new();
        while let Some(ch) = self
            .peek_at(0)
            .filter(|ch| OPERATOR_CHARACTERS.contains(*ch))
        {
            let comment_start =
                matches!((ch, self.peek_at(1)), ('-', Some('-')) | ('/', Some('*')));
            if comment_start {
                break;
            }
            run.push(ch);
            self.advance();
        }
        let has_special = run.chars().any(|ch| SPECIAL_CHARACTERS.contains(ch));
        while run.chars().count() > 1 && !has_special && run.ends_with(['+', '-']) {
            run.pop();
            self.position = self.position.saturating_sub(1);
        }
        run
    }

    fn is_assignment(&self) -> bool {
        self.in_diagnostics
            || (self.in_list && self.assignment_next && self.depth == self.list_depth)
    }

    fn is_all_columns(&self) -> bool {
        match &self.previous {
            Previous::Open | Previous::Dot | Previous::Comma => true,
            Previous::Word(word) => STAR_WORDS.contains(&word.as_str()),
            Previous::Other => false,
        }
    }

    fn operator(&mut self) {
        let run = self.read_operator();
        if run.is_empty() {
            self.advance();
            return;
        }
        match run.as_str() {
            "=>" => {}
            "=" if self.is_assignment() => self.assignment_next = false,
            "*" if self.is_all_columns() => {}
            _ => self.hit(&run),
        }
        self.previous = Previous::Other;
    }

    fn punctuation(&mut self, ch: char) {
        match ch {
            '(' => {
                self.depth = self.depth.saturating_add(1);
                self.previous = Previous::Open;
            }
            ')' => {
                self.depth = self.depth.saturating_sub(1);
                self.previous = Previous::Other;
            }
            ',' => {
                self.assignment_next = self.in_list && self.depth == self.list_depth;
                self.previous = Previous::Comma;
            }
            '.' => self.previous = Previous::Dot,
            ';' => {
                self.in_list = false;
                self.assignment_next = false;
                self.in_diagnostics = false;
                self.previous = Previous::Other;
            }
            _ => self.previous = Previous::Other,
        }
        self.advance();
    }

    fn step(&mut self, ch: char) {
        match (ch, self.peek_at(1)) {
            ('-', Some('-')) => self.skip_line_comment(),
            ('/', Some('*')) => self.skip_block_comment(),
            ('\'', _) => {
                let escapes = self.is_escape_string();
                self.skip_quoted('\'', escapes);
                self.previous = Previous::Other;
            }
            ('"', _) => {
                self.skip_quoted('"', false);
                self.previous = Previous::Other;
            }
            ('$', _) => {
                self.skip_dollar();
                self.previous = Previous::Other;
            }
            (':', Some(':' | '=')) => {
                self.advance();
                self.advance();
                self.previous = Previous::Other;
            }
            _ if ch.is_whitespace() => self.advance(),
            _ if is_word_start(ch) => self.word(),
            _ if ch.is_ascii_digit() => self.number(),
            _ if OPERATOR_CHARACTERS.contains(ch) => self.operator(),
            _ => self.punctuation(ch),
        }
    }
}

/// The operators of a function body that carry no `OPERATOR(pg_catalog.…)` form. Each entry is
/// the operator and its line, such as `+ at line 3`. A scanner fault, such as an open literal,
/// is one entry too.
pub fn unqualified_operators(body: &str) -> Vec<String> {
    let mut scan = Scan {
        chars: body.chars().collect(),
        line: 1,
        ..Scan::default()
    };
    while let Some(ch) = scan.peek_at(0) {
        scan.step(ch);
    }
    scan.hits
}

/// The body of the first `CREATE FUNCTION <name>(` or `CREATE OR REPLACE FUNCTION <name>(` of a
/// migration text: the text between the first two `$$` after it. This is the `prosrc` of the
/// function.
pub fn function_body(migration: &str, name: &str) -> String {
    let start = ["CREATE FUNCTION", "CREATE OR REPLACE FUNCTION"]
        .iter()
        .find_map(|keywords| migration.find(&format!("{keywords} {name}(")))
        .expect("the text holds the function");
    let after = migration.get(start..).unwrap();
    let open = after.find("$$").unwrap().saturating_add(2);
    let tail = after.get(open..).unwrap();
    tail.get(..tail.find("$$").unwrap()).unwrap().to_owned()
}

#[cfg(test)]
mod tests {
    use super::{function_body, unqualified_operators};
    use crate::support::migration_texts;

    fn migration(file: &str) -> String {
        migration_texts()
            .into_iter()
            .find(|(name, _)| name == file)
            .expect("the migration file exists")
            .1
    }

    #[test]
    fn the_scanner_finds_no_hit_in_replication_state_and_the_guard_of_0007() {
        let operations = migration("0006_operations.sql");
        let replaced = migration("0007_audit_definer_operators.sql");

        let state = unqualified_operators(&function_body(&operations, "replication_state"));
        let trigger = unqualified_operators(&function_body(&replaced, "security_event_guard"));

        assert_eq!(state, Vec::<String>::new());
        assert_eq!(trigger, Vec::<String>::new());
    }

    #[test]
    fn the_scanner_finds_the_two_distinct_forms_of_the_guard_of_0005() {
        let guard = migration("0005_security_event_guard.sql");

        let hits = unqualified_operators(&function_body(&guard, "security_event_guard"));

        assert_eq!(hits.len(), 2, "{hits:?}");
        assert!(
            hits.iter().all(|hit| hit.starts_with("IS DISTINCT FROM ")),
            "{hits:?}"
        );
    }

    #[test]
    fn the_scanner_finds_the_bare_operators_of_the_shipped_bodies_of_0002() {
        let triggers = migration("0002_triggers.sql");

        let chain = unqualified_operators(&function_body(&triggers, "audit_hash_chain"));
        let purge = unqualified_operators(&function_body(&triggers, "security_event_purge"));

        assert_eq!(chain.len(), 2, "{chain:?}");
        assert!(chain.iter().any(|hit| hit.starts_with("+ ")), "{chain:?}");
        assert!(chain.iter().any(|hit| hit.starts_with("|| ")), "{chain:?}");
        assert_eq!(purge.len(), 3, "{purge:?}");
        for operator in ["IN ", "< ", "- "] {
            assert!(
                purge.iter().any(|hit| hit.starts_with(operator)),
                "{purge:?}"
            );
        }
    }

    #[test]
    fn the_scanner_finds_each_kind_of_bare_operator_with_its_line() {
        let cases = [
            ("x := a + 1;", "+ at line 1"),
            ("x := a || b;", "|| at line 1"),
            ("x := a <> b;", "<> at line 1"),
            ("\n\nIF a = b THEN NULL; END IF;", "= at line 3"),
            ("WHERE id IN (1, 2)", "IN at line 1"),
            ("WHERE name like 'a%'", "LIKE at line 1"),
            ("WHERE n BETWEEN 1 AND 2", "BETWEEN at line 1"),
            ("x := -a;", "- at line 1"),
            (
                "x := a OPERATOR(public.+) b;",
                "OPERATOR(public.+) at line 1",
            ),
            ("x := a OPERATOR(+) b;", "OPERATOR(+) at line 1"),
            ("x := a=-b;", "= at line 1"),
            ("x := a IS DISTINCT FROM b;", "IS DISTINCT FROM at line 1"),
            (
                "x := a\n  is   not\n distinct from b;",
                "IS NOT DISTINCT FROM at line 3",
            ),
            ("x := NULLIF(a, b);", "NULLIF at line 1"),
            ("x := nullif (a, b);", "NULLIF at line 1"),
            ("x := CASE a WHEN 1 THEN 2 END;", "CASE x WHEN at line 1"),
            ("x := CASE\n(a) WHEN 1 THEN 2 END;", "CASE x WHEN at line 1"),
        ];

        for (body, expected) in cases {
            let hits = unqualified_operators(body);
            assert!(hits.iter().any(|hit| hit == expected), "{body}: {hits:?}");
        }
    }

    #[test]
    fn the_scanner_reports_an_open_literal_and_an_open_comment_as_a_fault() {
        for body in [
            "x := 'open",
            "/* open",
            "x := $a$ open",
            "x := a OPERATOR(pg_catalog.+",
        ] {
            assert_eq!(unqualified_operators(body).len(), 1, "{body}");
        }
    }

    #[test]
    fn the_scanner_skips_what_is_no_operator_lookup() {
        let skipped = [
            "x := 1;",
            "x := a::text;",
            "-- a + b\nx := 1;",
            "/* a /* + */ || */ x := 1;",
            "x := 'a + b || ''c'' - d';",
            "x := E'a \\' + b';",
            "x := \"a+b\";",
            "x := $t$ a + b $t$;",
            "x := f(a => 1);",
            "x := $1;",
            "GET DIAGNOSTICS v = ROW_COUNT, w = PG_CONTEXT;",
            "UPDATE t SET a = 1, b = f(x, y) WHERE c OPERATOR(pg_catalog.=) 1;",
            "UPDATE t SET a = 1 FROM u WHERE t.id OPERATOR(pg_catalog.=) u.id;",
            "RAISE EXCEPTION 'x' USING ERRCODE = 'check_violation', DETAIL = 'y';",
            "SELECT count(*), t.* FROM t;",
            "SELECT * FROM t;",
            "x := a OPERATOR(pg_catalog.*) 2 OPERATOR ( pg_catalog.+ ) 1e-3;",
            "x := 1..2;",
            "SELECT DISTINCT a FROM t;",
            "x := CASE WHEN a OPERATOR(pg_catalog.=) b THEN 1 END;",
            "CASE\n WHEN a THEN NULL; END CASE;",
            "x := a IS NOT NULL;",
        ];

        for body in skipped {
            assert_eq!(unqualified_operators(body), Vec::<String>::new(), "{body}");
        }
    }

    #[test]
    fn the_scanner_does_not_skip_an_equals_sign_inside_a_set_clause_value() {
        let hits = unqualified_operators(
            "UPDATE t SET a = (b = c), d = 1 WHERE e OPERATOR(pg_catalog.=) 1;",
        );

        assert_eq!(hits, ["= at line 1".to_owned()]);
    }
}
