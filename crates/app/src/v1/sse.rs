//! Live subscriptions: snapshot, then diffs, resumable by cursor.
//!
//! The protocol (docs/architecture.md "SSE", docs/research/api-layer.md §2):
//!
//! 1. **Subscribe first** — take the doorbell receiver before reading anything,
//!    so a change committed during the snapshot cannot be missed.
//! 2. **Snapshot** in one read transaction, capturing `N = MAX(cursor)` *inside*
//!    it; stream the matching set as `added` events, then a `live` marker.
//! 3. **Diff** on `cursor > N`, forever. Because the cursor is strictly
//!    monotonic and the query is strictly `>`, nothing can be emitted twice or
//!    skipped — no locking required.
//!
//! A resuming client (`Last-Event-ID`, or `?cursor=` for curl) skips step 2 and
//! gets exactly what it missed. A cursor below the log's horizon cannot be
//! served that way and gets a `reset` event instead, which means "re-snapshot"
//! (Firestore's expired-token semantics).

use super::{ApiError, ApiResult, AppState, Collection, Item, Params, StreamSlot, json, read_items};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_core::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use store::read::{self, Filter, Scope};
use store::{Change, Readers};
use tokio::sync::watch;

/// Idle comment interval. Proxies and NATs drop quiet connections; 15 s is
/// safely inside nginx's default 60 s read timeout.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// How many change rows one diff pass reads. A client that falls behind simply
/// reads bigger batches later — the log is the buffer, so it cannot lose data.
const DIFF_BATCH: i64 = 500;

/// Rows per snapshot page, inside the snapshot's read transaction.
const SNAPSHOT_PAGE: i64 = 500;

pub async fn subscribe(
    collection: Collection,
    state: AppState,
    headers: HeaderMap,
    params: Params,
    filter: Filter,
) -> ApiResult {
    let key = super::client_key(&headers);
    let Some(slot) = state.claim_stream(&key) else {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "too many live streams from this client".into(),
        ));
    };
    let resume = resume_cursor(&headers, &params);
    let include_data = params.include_data.unwrap_or(false);
    // Step 1: subscribe to the doorbell *before* anything is read. From here on
    // no committed change can slip past the snapshot unnoticed.
    let mut doorbell = state.cursor.clone();
    doorbell.mark_unchanged();
    let stream =
        events(collection, state.readers.clone(), filter, resume, include_data, slot, doorbell);

    Ok((
        [
            // nginx buffers proxied responses by default, which would hold an
            // SSE stream indefinitely; this opts out per-response.
            (header::CACHE_CONTROL, "no-store"),
            (axum::http::HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE)),
    )
        .into_response())
}

/// Where a reconnecting client left off. `EventSource` replays the last `id:`
/// it saw in `Last-Event-ID`; `?cursor=` is the same thing for clients that are
/// not `EventSource` (curl, scripts).
fn resume_cursor(headers: &HeaderMap, params: &Params) -> Option<i64> {
    headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
        .or_else(|| params.cursor.as_deref().and_then(|c| c.parse().ok()))
}

#[allow(clippy::too_many_arguments)]
fn events(
    collection: Collection,
    readers: Arc<Readers>,
    filter: Filter,
    resume: Option<i64>,
    include_data: bool,
    slot: StreamSlot,
    mut doorbell: watch::Receiver<i64>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        // The slot lives exactly as long as the stream: when the client goes
        // away, this generator is dropped and the budget is released.
        let _slot = slot;

        let started = match start(collection, &readers, &filter, resume).await {
            Ok(started) => started,
            Err(e) => {
                yield Ok(error_event(&e));
                return;
            }
        };
        for event in started.initial {
            yield Ok(event);
        }
        let mut cursor = started.cursor;
        loop {
            // Drain first, wait second. A resuming client's backlog was
            // committed before it connected, so no doorbell will ever ring for
            // it — reading before blocking is what makes resume work at all.
            // After a fresh snapshot the drain finds nothing, which is correct
            // and costs one indexed query.
            loop {
                let batch = match diff(collection, &readers, &filter, cursor, include_data).await {
                    Ok(batch) => batch,
                    Err(e) => {
                        yield Ok(error_event(&e));
                        return;
                    }
                };
                let Some(last) = batch.last_cursor else { break };
                for event in batch.events {
                    yield Ok(event);
                }
                cursor = last;
                if !batch.more {
                    break;
                }
            }
            // Then wait for the writer to ring. A closed channel means the
            // database handle is gone — the process is shutting down.
            if doorbell.changed().await.is_err() {
                return;
            }
        }
    }
}

struct Started {
    initial: Vec<Event>,
    cursor: i64,
}

/// Step 2: the snapshot, or — for a resuming client — the decision to skip it.
async fn start(
    collection: Collection,
    readers: &Arc<Readers>,
    filter: &Filter,
    resume: Option<i64>,
) -> Result<Started, store::turso::Error> {
    let reader = readers.get().await?;

    if let Some(from) = resume {
        let oldest = read::oldest_cursor(&reader).await?;
        // The log is append-only and never renumbered, so "below the horizon"
        // can only happen if it was pruned — but the path exists either way,
        // and a client that invents a cursor gets an honest answer.
        if oldest > 0 && from < oldest - 1 {
            let event = Event::default()
                .event("reset")
                .json_data(serde_json::json!({ "reason": "cursor_expired" }))
                .expect("a literal object always serialises");
            return Ok(Started { initial: vec![event], cursor: 0 });
        }
        return Ok(Started { initial: Vec::new(), cursor: from });
    }

    // One read transaction: the cursor and the rows come from the same
    // consistent snapshot of the database (WAL readers see a stable view). If this
    // future is cancelled (client disconnects) between BEGIN and COMMIT — likely,
    // since `collect_snapshot` paginates a whole collection — the reader is dropped
    // mid-transaction; the pool discards such a connection rather than returning it
    // with an open snapshot that would pin the WAL forever (store::read Drop, issue 53).
    reader.execute("BEGIN", ()).await?;
    let snapshot = collect_snapshot(collection, &reader, filter).await;
    let _ = reader.execute("COMMIT", ()).await;
    let (cursor, items) = snapshot?;

    let mut initial: Vec<Event> = items
        .iter()
        .map(|item| {
            entity_event(collection, cursor, "added", item.id, None, Some(&item.json), true)
        })
        .collect();
    initial.push(
        Event::default()
            .event("live")
            .id(cursor.to_string())
            .json_data(serde_json::json!({ "cursor": cursor.to_string() }))
            .expect("a literal object always serialises"),
    );
    Ok(Started { initial, cursor })
}

async fn collect_snapshot(
    collection: Collection,
    reader: &store::Reader,
    filter: &Filter,
) -> Result<(i64, Vec<Item>), store::turso::Error> {
    let cursor = read::latest_cursor(reader).await?;
    let mut items = Vec::new();
    let mut after = 0;
    loop {
        let scope = Scope::Page { after, limit: SNAPSHOT_PAGE };
        let page = read_items(collection, reader, filter, scope).await?;
        let Some(last) = page.last() else { break };
        after = last.id;
        let full = page.len() as i64 == SNAPSHOT_PAGE;
        items.extend(page);
        if !full {
            break;
        }
    }
    Ok((cursor, items))
}

struct Batch {
    events: Vec<Event>,
    last_cursor: Option<i64>,
    more: bool,
}

/// Step 3: classify each change past `cursor` against this subscription's
/// predicate, by evaluating it on both the old and the new version of the
/// entity (docs/architecture.md). That is what turns a global change log into a
/// *filtered* added/changed/removed feed.
async fn diff(
    collection: Collection,
    readers: &Arc<Readers>,
    filter: &Filter,
    cursor: i64,
    include_data: bool,
) -> Result<Batch, store::turso::Error> {
    let reader = readers.get().await?;
    let Some(kind) = collection.entity_kind() else {
        // Notices have no canonical change rows; the subscription stays open
        // and silent rather than pretending otherwise.
        return Ok(Batch { events: Vec::new(), last_cursor: None, more: false });
    };
    let rows = read::changes_since(&reader, cursor, DIFF_BATCH, Some(kind)).await?;
    let more = rows.len() as i64 == DIFF_BATCH;
    let last_cursor = rows.last().map(|c| c.cursor);

    let mut events = Vec::new();
    for change in &rows {
        let seq = change.version_seq.unwrap_or(0);
        let new = read_items(collection, &reader, filter, Scope::At { id: change.entity_id, seq })
            .await?
            .pop();
        let old = match seq > 1 {
            true => read_items(
                collection,
                &reader,
                filter,
                Scope::At { id: change.entity_id, seq: seq - 1 },
            )
            .await?
            .pop(),
            false => None,
        };
        // The subscription's own view of what happened, which is not always the
        // log's: a Tender that changed *out of* the filtered set is a removal
        // for this client, and one that changed *into* it is an addition.
        let op = match (old.is_some(), new.is_some()) {
            (false, true) => "added",
            (true, true) => "changed",
            (true, false) => "removed",
            (false, false) => continue,
        };
        events.push(entity_event(
            collection,
            change.cursor,
            op,
            change.entity_id,
            change.version_seq,
            new.as_ref().map(|i| &i.json),
            include_data,
        ));
    }
    Ok(Batch { events, last_cursor, more })
}

/// One change as JSON — the same object for SSE `data:`, poll items and (later)
/// webhook bodies, so a client can move between transports without reparsing.
fn entity_event(
    collection: Collection,
    cursor: i64,
    op: &str,
    id: i64,
    version: Option<i64>,
    data: Option<&serde_json::Value>,
    include_data: bool,
) -> Event {
    let mut body = serde_json::json!({
        "cursor": cursor.to_string(),
        "op": op,
        "entity": collection.entity_kind().unwrap_or("notice"),
        "id": id,
        "version": version,
    });
    if include_data {
        let map = body.as_object_mut().expect("just built as an object");
        map.insert("data".into(), data.cloned().unwrap_or(serde_json::Value::Null));
    }
    Event::default()
        .event("change")
        // The cursor is the SSE id, which is what makes Last-Event-ID resume
        // exact rather than approximate.
        .id(cursor.to_string())
        .json_data(body)
        .expect("a literal object always serialises")
}

/// A change-log row as the poll endpoint returns it.
pub fn change_event(change: &Change) -> serde_json::Value {
    serde_json::json!({
        "cursor": change.cursor.to_string(),
        "op": change.op,
        "entity": change.entity_kind,
        "id": change.entity_id,
        "version": change.version_seq,
        "changed_at": json::instant(change.changed_at),
    })
}

fn error_event(e: &store::turso::Error) -> Event {
    Event::default()
        .event("error")
        .json_data(serde_json::json!({ "message": e.to_string() }))
        .expect("a literal object always serialises")
}

