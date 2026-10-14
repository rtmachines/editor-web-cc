//! The insert of the audit entries of one command (design D3, `data-model.md` section 3.15).

use recipe_app::ports::NonEmpty;
use recipe_domain::audit::AuditEntry;
use sqlx::PgConnection;
use uuid::Uuid;

/// One insert for all entries. `WITH ORDINALITY` and `ORDER BY` keep the order of the list.
///
/// The statement omits `chain_seq`, `prev_hash`, and `entry_hash`. Trigger 4.9 sets them in the
/// order of the rows. `client_ip` travels as text and the cast `::inet` reads it, so no network
/// crate enters the dependency graph (design D6).
const INSERT_AUDIT_ENTRIES: &str = "\
INSERT INTO audit_log (actor_user_id, action, entity_type, entity_id, aggregate_type, \
    aggregate_id, before_state, after_state, request_id, client_ip, session_ref)
SELECT t.actor_user_id, t.action, t.entity_type, t.entity_id, t.aggregate_type, \
    t.aggregate_id, t.before_state, t.after_state, t.request_id, t.client_ip::inet, t.session_ref
FROM unnest($1::uuid[], $2::text[], $3::text[], $4::uuid[], $5::text[], $6::uuid[], \
        $7::jsonb[], $8::jsonb[], $9::uuid[], $10::text[], $11::text[])
    WITH ORDINALITY AS t(actor_user_id, action, entity_type, entity_id, aggregate_type, \
        aggregate_id, before_state, after_state, request_id, client_ip, session_ref, ordinality)
ORDER BY t.ordinality";

/// Write the entries in the current transaction, in the order of the list.
pub(crate) async fn insert_entries(
    connection: &mut PgConnection,
    entries: &NonEmpty<AuditEntry>,
) -> Result<(), sqlx::Error> {
    let actors: Vec<Option<Uuid>> = column(entries, |e| e.actor().map(|user| *user.as_uuid()));
    let actions: Vec<&str> = column(entries, |e| e.action().as_str());
    let entity_types: Vec<&str> = column(entries, |e| e.target().entity_type());
    let entity_ids: Vec<Uuid> = column(entries, |e| *e.target().entity_id());
    let aggregate_types: Vec<&str> = column(entries, |e| e.target().aggregate_type().as_sql());
    let aggregate_ids: Vec<Uuid> = column(entries, |e| *e.target().aggregate_id());
    let before_states = column(entries, |e| e.before().cloned());
    let after_states = column(entries, |e| e.after().cloned());
    let request_ids: Vec<Uuid> = column(entries, |e| *e.request_id().as_uuid());
    let client_ips: Vec<Option<String>> = column(entries, |e| {
        e.client_ip().map(|address| address.to_string())
    });
    let session_refs: Vec<Option<&str>> =
        column(entries, |e| e.session_ref().map(|session| session.as_str()));
    sqlx::query(INSERT_AUDIT_ENTRIES)
        .bind(actors)
        .bind(actions)
        .bind(entity_types)
        .bind(entity_ids)
        .bind(aggregate_types)
        .bind(aggregate_ids)
        .bind(before_states)
        .bind(after_states)
        .bind(request_ids)
        .bind(client_ips)
        .bind(session_refs)
        .execute(connection)
        .await
        .map(|_| ())
}

/// Collect one column of the statement from the entries, in list order.
fn column<'a, T>(entries: &'a NonEmpty<AuditEntry>, read: impl Fn(&'a AuditEntry) -> T) -> Vec<T> {
    entries.iter().map(read).collect()
}
