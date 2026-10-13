//! Translations of a parameter and the removal of a parameter (design D6).

use super::{Draft, ParameterChange};
use crate::recipe::{LangCode, ParameterKey, RecipeError, Translation};

impl Draft {
    /// Add the translation of a parameter or replace the translation of the same language.
    ///
    /// A parameter has one translation for each language (ADR-0012). The name and the
    /// description of `translation` already passed their length limits. The method needs no
    /// type tree, so it also works for a parameter whose type did not resolve.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` when the draft has no parameter with this key.
    pub fn upsert_translation(
        &mut self,
        key: ParameterKey,
        translation: Translation,
    ) -> Result<ParameterChange, RecipeError> {
        let old = self.parameter(key)?.clone();
        let new = old.with_translation(translation);
        self.parameters.insert(key, new.clone());
        Ok(ParameterChange::new(Some(old), Some(new)))
    }

    /// Remove the translation of one language from a parameter.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` when the draft has no parameter with this key. Returns `E-RCP-050`
    /// when the parameter has no translation for `lang`.
    pub fn remove_translation(
        &mut self,
        key: ParameterKey,
        lang: &LangCode,
    ) -> Result<ParameterChange, RecipeError> {
        let old = self.parameter(key)?.clone();
        let Some(new) = old.without_translation(lang) else {
            return Err(RecipeError::TranslationNotFound {
                reference: self.reference.clone(),
                number: self.number,
                tag: old.tag().clone(),
                lang: lang.clone(),
            });
        };
        self.parameters.insert(key, new.clone());
        Ok(ParameterChange::new(Some(old), Some(new)))
    }

    /// Remove a parameter with its limits and its translations.
    ///
    /// The method needs no type tree, so it also removes a parameter whose type did not
    /// resolve.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` when the draft has no parameter with this key.
    pub fn remove_parameter(&mut self, key: ParameterKey) -> Result<ParameterChange, RecipeError> {
        let old = self.parameter(key)?.clone();
        self.parameters.remove(&key);
        Ok(ParameterChange::new(Some(old), None))
    }
}

#[cfg(test)]
mod tests;
