//! In-memory read model of the audit views (ADR-0051 (a), design D5).
//!
//! The read model gives each committed audit entry the id of its position in commit order plus 1
//! and its commit time. It gives each event of the sink of the store an id and the time of the
//! store clock. Only this read model aggregates the events per minute.
//!
//! A read sees one state. It reads the entries and events that were committed when its
//! transaction began, with the labels and the users of the state of that `begin`.

mod entries;
mod events;
mod labels;

use async_trait::async_trait;
use recipe_domain::audit::AggregateType;
use recipe_domain::recipe::RecipeId;

use self::entries::{Numbered, actor_of, matches, numbered, record, row, window};
use self::events::{aggregate, event_row, filtered};
use self::labels::label;
use crate::error::StorageError;
use crate::memory::MemoryTransaction;
use crate::ports::{
    AggregateId, AuditEntityType, AuditEntryId, AuditFilter, AuditReads, AuditRecord, AuditRow,
    AuditScope, CursorPage, EntityId, EntityLabel, SecurityEventFilter, SecurityEventId,
    SecurityEventRow,
};

impl MemoryTransaction {
    /// Return the committed entries newest first, with an id below `before`.
    fn newest_entries(
        &self,
        before: Option<AuditEntryId>,
    ) -> Result<impl Iterator<Item = Numbered>, StorageError> {
        let committed = numbered(self.audit_at_begin())?;
        Ok(committed
            .into_iter()
            .rev()
            .filter(move |numbered| before.is_none_or(|before| numbered.id < before)))
    }
}

#[async_trait]
impl AuditReads for MemoryTransaction {
    async fn entries(
        &mut self,
        filter: &AuditFilter,
        scope: AuditScope,
        before: Option<AuditEntryId>,
        limit: u32,
    ) -> Result<CursorPage<AuditRow>, StorageError> {
        let state = self.state();
        let Some(actor) = actor_of(state, filter) else {
            return Ok(CursorPage::empty());
        };
        let rows = self
            .newest_entries(before)?
            .filter(|numbered| matches(numbered, filter, scope, actor))
            .take(window(limit)?)
            .map(|numbered| row(state, &numbered))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CursorPage::cut(rows, limit))
    }

    async fn entry(
        &mut self,
        id: AuditEntryId,
        scope: AuditScope,
    ) -> Result<Option<AuditRecord>, StorageError> {
        let found = self
            .newest_entries(None)?
            .find(|numbered| numbered.id == id)
            .filter(|numbered| scope.admits(numbered.entry.target().aggregate_type()));
        found
            .map(|numbered| record(self.state(), &numbered))
            .transpose()
    }

    async fn history(
        &mut self,
        recipe: RecipeId,
        before: Option<AuditEntryId>,
        limit: u32,
    ) -> Result<CursorPage<AuditRow>, StorageError> {
        let filter = AuditFilter {
            aggregate: Some(AggregateId::from(*recipe.as_uuid())),
            ..AuditFilter::default()
        };
        self.entries(&filter, AuditScope::Recipes, before, limit)
            .await
    }

    async fn security_events(
        &mut self,
        filter: &SecurityEventFilter,
        before: Option<SecurityEventId>,
        limit: u32,
    ) -> Result<CursorPage<SecurityEventRow>, StorageError> {
        let state = self.state();
        let rows = aggregate(self.events_at_begin())?;
        let rows = filtered(state, rows, filter)
            .into_iter()
            .rev()
            .filter(|row| before.is_none_or(|before| row.id < before))
            .take(window(limit)?)
            .map(|row| event_row(state, row))
            .collect();
        Ok(CursorPage::cut(rows, limit))
    }

    async fn aggregate_label(
        &mut self,
        aggregate_type: AggregateType,
        id: AggregateId,
    ) -> Result<EntityLabel, StorageError> {
        let root = EntityId::from(*id.as_uuid());
        Ok(label(
            self.state(),
            AuditEntityType::from(aggregate_type),
            root,
        ))
    }
}

#[cfg(test)]
mod tests;
