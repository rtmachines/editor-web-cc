//! The text of an error and of every source behind it.

use std::error::Error;

/// Join the `Display` text of an error and of each error in its `source` chain.
///
/// A secret must appear in none of these texts, because the log writes the whole chain.
pub fn display_chain(error: &dyn Error) -> String {
    let mut texts = vec![error.to_string()];
    let mut next = error.source();
    while let Some(source) = next {
        texts.push(source.to_string());
        next = source.source();
    }
    texts.join(" | ")
}
