//! The diff of the translations of one parameter (ADR-0050 (c)).

use std::collections::BTreeSet;

use super::rows::DiffPart;
use crate::recipe::{LangCode, Parameter};

/// Return one row for each language whose translation differs, in the order of the codes.
///
/// A language of one side only gives a row with an empty other side.
pub(super) fn translation_rows(old: &Parameter, new: &Parameter) -> Vec<DiffPart> {
    let langs: BTreeSet<&LangCode> = old
        .translations()
        .keys()
        .chain(new.translations().keys())
        .collect();
    langs
        .into_iter()
        .filter_map(|lang| {
            let old = old.translations().get(lang);
            let new = new.translations().get(lang);
            (old != new).then(|| DiffPart::Translation {
                lang: lang.clone(),
                old: old.cloned(),
                new: new.cloned(),
            })
        })
        .collect()
}
