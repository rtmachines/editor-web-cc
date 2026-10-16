//! The rows of the audit list and of the recipe history (ADR-0051 (e)).
//!
//! The module solves one problem: the history page, the audit list, and the entry page show the
//! time, the actor, the action, and the object of an entry in one form. One view and one set of
//! word functions give that form, so the pages cannot disagree. Every word comes from the bundles.

use std::fmt::Display;

use recipe_app::ports::{AuditEntityType, AuditRow, EntityLabel};
use recipe_domain::audit::AuditAction;

use crate::pages::render::instant;
use crate::pages::texts::Texts;

/// One row of the table of audit entries, in the UI language.
pub(crate) struct AuditRowView {
    /// The address of the entry page, or `None` for a role without an audit permission.
    pub(crate) href: Option<String>,
    /// The text of the link: the word Entry and the id.
    pub(crate) link_text: String,
    /// The commit time in UTC.
    pub(crate) time: String,
    /// The login of the actor, or the word for an entry without an actor.
    pub(crate) actor: String,
    /// The word of the action.
    pub(crate) action: String,
    /// The word of the entity type and the label of the entity.
    pub(crate) entity: String,
}

impl AuditRowView {
    /// Build the view of `row`. The row links to its entry page when `linked` is true.
    pub(crate) fn new(t: Texts<'_>, row: &AuditRow, linked: bool) -> Self {
        Self {
            href: linked.then(|| format!("/audit/{}", row.id)),
            link_text: t.with_arg("audit-entry-link", "id", row.id),
            time: instant(&row.occurred_at),
            actor: row.actor.as_ref().map_or_else(
                || t.get("audit-actor-system"),
                std::string::ToString::to_string,
            ),
            action: action_word(t, row.action),
            entity: entity_text(t, row),
        }
    }
}

/// Return the text of `name` in kebab case, for example `VersionApproved` gives `version-approved`.
fn kebab(name: &str) -> String {
    let mut text = String::with_capacity(name.len());
    for (index, letter) in name.chars().enumerate() {
        if letter.is_ascii_uppercase() {
            if index > 0 {
                text.push('-');
            }
            text.push(letter.to_ascii_lowercase());
        } else {
            text.push(letter);
        }
    }
    text
}

/// Return the word of an action in the UI language.
pub(crate) fn action_word(t: Texts<'_>, action: AuditAction) -> String {
    t.get(&format!("audit-action-{}", kebab(action.as_str())))
}

/// Return the word of an entity type in the UI language.
pub(crate) fn entity_word(t: Texts<'_>, entity_type: AuditEntityType) -> String {
    t.get(&format!(
        "audit-entity-{}",
        entity_type.as_sql().replace('_', "-")
    ))
}

/// Return the label of an entity from its current row. A missing row gives the short id.
pub(crate) fn label_text(label: &EntityLabel, short_id: &str) -> String {
    match label {
        EntityLabel::Recipe(reference) => reference.to_string(),
        EntityLabel::Version(number) => number.to_string(),
        EntityLabel::Parameter { tag, version } => format!("{tag} ({version})"),
        EntityLabel::Machine(code) => code.to_string(),
        EntityLabel::Unit(path) => path.to_string(),
        EntityLabel::Type { name, revision } => format!("{name}@{revision}"),
        EntityLabel::User(login) => login.to_string(),
        EntityLabel::Setting(key) => key.to_string(),
        EntityLabel::Missing => short_id.to_owned(),
    }
}

/// Return the text of a stored state or detail: the pretty JSON with sorted keys.
///
/// The value is a JSON value. The alternate form `{:#}` of its `Display` gives the pretty text,
/// and askama escapes that text in the `pre` element. A time leaf shows as its stored integer
/// count. This is an exception to A43 (ADR-0051 (f)).
pub(crate) fn state_text<T: Display>(state: Option<&T>) -> Option<String> {
    state.map(|value| format!("{value:#}"))
}

/// Return the word of the entity type of `row` and its label (ADR-0051 (e)).
pub(crate) fn entity_text(t: Texts<'_>, row: &AuditRow) -> String {
    format!(
        "{} {}",
        entity_word(t, row.entity_type),
        label_text(&row.label, &row.entity_id.short())
    )
}

#[cfg(test)]
mod tests;
