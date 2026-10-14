//! A small SQL tokenizer for the static check of the migration files (design D11).
//!
//! It skips `--` comments and nested `/* ... */` comments. It reads single-quoted strings with
//! `''`, escape strings `E'...'` with a backslash, double-quoted identifiers, and dollar-quoted
//! bodies. It splits the statements on `;` outside of those forms.

/// One token of a statement. Punctuation is not a token.
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    /// A run of letters, digits, and underscores, or the content of a quoted identifier.
    Word(String),
    /// The content of a string literal or a dollar-quoted body.
    Literal(String),
}

impl Token {
    /// The text of the token in lower case. A literal gives its content.
    pub fn lowercase(&self) -> String {
        match self {
            Token::Word(text) | Token::Literal(text) => text.to_ascii_lowercase(),
        }
    }

    /// The text for a message. A literal hides its content.
    pub fn shown(&self) -> String {
        match self {
            Token::Word(text) => text.clone(),
            Token::Literal(_) => "<literal>".to_owned(),
        }
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// The length of the dollar-quote delimiter at the start of the text, or `None`.
/// A delimiter is `$`, an optional tag of identifier characters, and `$`. `$1` is no delimiter.
fn delimiter_len(text: &str) -> Option<usize> {
    let rest = text.strip_prefix('$')?;
    let tag_len = rest
        .find(|ch: char| !is_word_char(ch))
        .unwrap_or(rest.len());
    let tag = rest.get(..tag_len)?;
    let starts_with_digit = tag.chars().next().is_some_and(|ch| ch.is_ascii_digit());
    (rest.get(tag_len..)?.starts_with('$') && !starts_with_digit).then(|| tag_len.saturating_add(2))
}

/// The cursor over the text. Every method moves it forward and never past the end.
struct Cursor<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        self.text.get(self.position..).unwrap_or_default()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn advance(&mut self, bytes: usize) {
        self.position = self.position.saturating_add(bytes).min(self.text.len());
    }

    /// Take the characters while the test holds.
    fn take_while(&mut self, test: impl Fn(char) -> bool) -> &'a str {
        let rest = self.rest();
        let length = rest.find(|ch: char| !test(ch)).unwrap_or(rest.len());
        self.advance(length);
        rest.get(..length).unwrap_or_default()
    }

    /// Skip a block comment that starts at the cursor. Block comments nest. Return false when
    /// the text ends inside the comment.
    fn skip_block_comment(&mut self) -> bool {
        let mut depth = 0_u32;
        loop {
            let rest = self.rest();
            if rest.starts_with("/*") {
                depth = depth.saturating_add(1);
                self.advance(2);
            } else if rest.starts_with("*/") {
                depth = depth.saturating_sub(1);
                self.advance(2);
                if depth == 0 {
                    return true;
                }
            } else if let Some(ch) = self.peek() {
                self.advance(ch.len_utf8());
            } else {
                return false;
            }
        }
    }

    /// Read a single-quoted string after the opening quote. `''` is a quote. With
    /// `backslash`, `\x` takes the next character as plain text. Return `None` when the text
    /// ends inside the string.
    fn read_quoted(&mut self, quote: char, backslash: bool) -> Option<String> {
        let mut content = String::new();
        while let Some(ch) = self.peek() {
            self.advance(ch.len_utf8());
            if ch == quote {
                if self.peek() == Some(quote) {
                    self.advance(quote.len_utf8());
                    content.push(quote);
                } else {
                    return Some(content);
                }
            } else if ch == '\\' && backslash {
                let escaped = self.peek()?;
                self.advance(escaped.len_utf8());
                content.push(escaped);
            } else {
                content.push(ch);
            }
        }
        None
    }

    /// Read a dollar-quoted body when the cursor stands on a delimiter.
    /// `Ok(None)` means no delimiter. `Err(())` means the text ends inside the body.
    fn read_dollar_body(&mut self) -> Result<Option<String>, ()> {
        let Some(length) = delimiter_len(self.rest()) else {
            return Ok(None);
        };
        let delimiter = self.rest().get(..length).unwrap_or_default();
        let after_open = self.rest().get(length..).unwrap_or_default();
        let end = after_open.find(delimiter).ok_or(())?;
        let body = after_open.get(..end).unwrap_or_default().to_owned();
        self.advance(length.saturating_add(end).saturating_add(length));
        Ok(Some(body))
    }
}

/// Split a SQL text into statements of tokens. The flag is false when the text ends inside a
/// literal, a quoted identifier, or a comment. Then the last statement may be incomplete.
pub fn split_statements(text: &str) -> (Vec<Vec<Token>>, bool) {
    let mut cursor = Cursor { text, position: 0 };
    let mut statements = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    while let Some(ch) = cursor.peek() {
        let rest = cursor.rest();
        let step = if rest.starts_with("--") {
            cursor.take_while(|c| c != '\n');
            Some(())
        } else if rest.starts_with("/*") {
            cursor.skip_block_comment().then_some(())
        } else if ch == ';' {
            cursor.advance(1);
            statements.push(std::mem::take(&mut current));
            Some(())
        } else if ch == '\'' {
            cursor.advance(1);
            cursor
                .read_quoted('\'', false)
                .map(|content| current.push(Token::Literal(content)))
        } else if ch == '"' {
            cursor.advance(1);
            cursor
                .read_quoted('"', false)
                .map(|content| current.push(Token::Word(content)))
        } else if ch == '$' {
            match cursor.read_dollar_body() {
                Ok(Some(body)) => {
                    current.push(Token::Literal(body));
                    Some(())
                }
                Ok(None) => {
                    cursor.advance(1);
                    Some(())
                }
                Err(()) => None,
            }
        } else if is_word_char(ch) {
            let word = cursor.take_while(is_word_char);
            let escape_string = word.eq_ignore_ascii_case("e") && cursor.peek() == Some('\'');
            if escape_string {
                cursor.advance(1);
                cursor
                    .read_quoted('\'', true)
                    .map(|content| current.push(Token::Literal(content)))
            } else {
                current.push(Token::Word(word.to_owned()));
                Some(())
            }
        } else {
            cursor.advance(ch.len_utf8());
            Some(())
        };
        if step.is_none() {
            statements.push(current);
            return (statements, false);
        }
    }
    statements.push(current);
    statements.retain(|statement| !statement.is_empty());
    (statements, true)
}
