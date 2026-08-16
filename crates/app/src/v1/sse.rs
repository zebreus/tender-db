//! Live subscriptions: snapshot, then diffs, resumable by cursor.
//!
//! The protocol (docs/architecture.md "SSE", docs/research/api-layer.md §2):
//!
//! 1. **Subscribe first** — take the doorbell receiver before reading anything,
//!    so a change committed during the snapshot cannot be missed.
//! 2. **Snapshot**: capture `N = MAX(cursor)` first, then stream the matching
//!    set as `added` events in keyset pages — a pooled reader per page, never
//!    a transaction held across pages — then a `live` marker. Pages read after
//!    a concurrent commit may already contain its rows, whose change rows are
//!    `> N` and so are emitted again by step 3: the snapshot is at-least-once
//!    per entity under concurrent writes, and the diff stream is what makes
//!    the client's final state exact. Paging instead of accumulating is
//!    deliberate (issue 55): a whole-collection snapshot held one reader for
//!    minutes and buffered millions of events in memory, which let a handful
//!    of anonymous subscriptions take the box down (incident 2026-08-09); now
//!    memory is bounded by one page and every yield is a cancellation point,
//!    so a vanished client stops costing anything at the next page.
//! 3. **Diff** on `cursor > N`, forever. Because the cursor is strictly
//!    monotonic and the query is strictly `>`, nothing past `live` can be
//!    emitted twice or skipped — no locking required.
//!
//! A resuming client (`Last-Event-ID`, or `?cursor=` for curl) skips step 2 and
//! gets exactly what it missed. That promise is only sound because snapshot
//! events carry NO SSE id: the first resumable position a client can ever hold
//! is the `live` marker, so a stream that dies mid-snapshot reconnects with no
//! `Last-Event-ID` and re-snapshots, rather than "resuming" past the snapshot's
//! own undelivered remainder. A cursor below the log's horizon cannot be
//! served either and gets a `reset` event instead, which means "re-snapshot"
//! (Firestore's expired-token semantics).

use super::{ApiError, ApiResult, AppState, Collection, Params, StreamSlot, json, read_items};
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

/// Rows per snapshot page — also the memory bound: this many items is all a
/// subscription ever buffers. `AppState.snapshot_page` carries it so tests can
/// shrink it to exercise multi-page snapshots on small fixtures.
pub(crate) const SNAPSHOT_PAGE: i64 = 500;

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
    let stream = events(
        collection,
        state.readers.clone(),
        state.isolated.clone(),
        filter,
        resume,
        include_data,
        // Clamped: 0 would snapshot nothing, and a negative LIMIT means
        // UNLIMITED in SQLite — one page holding the whole collection, the
        // exact incident this page size exists to prevent.
        state.snapshot_page.max(1),
        slot,
        doorbell,
    );

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

/// A resume token: the feed generation the client last saw (if its token
/// carried one) and the cursor. Diff/live event ids are `<generation>:<cursor>`
/// (issue 46), so an `EventSource` reconnect proves which generation its state
/// belongs to; a bare integer (legacy id, or a hand-built `?cursor=`) proves
/// nothing and is treated as the current generation — the documented resume
/// token is the event id, verbatim.
#[derive(Clone, Copy)]
struct Resume {
    generation: Option<i64>,
    cursor: i64,
}

/// Where a reconnecting client left off. `EventSource` replays the last `id:`
/// it saw in `Last-Event-ID`; `?cursor=` is the same thing for clients that are
/// not `EventSource` (curl, scripts).
fn resume_cursor(headers: &HeaderMap, params: &Params) -> Option<Resume> {
    let token = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .or_else(|| params.cursor.clone())?;
    let token = token.trim();
    match token.split_once(':') {
        Some((generation, cursor)) => Some(Resume {
            generation: Some(generation.parse().ok()?),
            cursor: cursor.parse().ok()?,
        }),
        None => Some(Resume { generation: None, cursor: token.parse().ok()? }),
    }
}

#[allow(clippy::too_many_arguments)]
fn events(
    collection: Collection,
    readers: Arc<Readers>,
    isolated: Arc<super::isolate::IsolatedReads>,
    filter: Filter,
    resume: Option<Resume>,
    include_data: bool,
    snapshot_page: i64,
    slot: StreamSlot,
    mut doorbell: watch::Receiver<i64>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        // The slot lives exactly as long as the stream: when the client goes
        // away, this generator is dropped and the budget is released.
        let _slot = slot;

        let started = match start(&readers, resume).await {
            Ok(started) => started,
            Err(e) => {
                yield Ok(error_event(&e));
                return;
            }
        };
        let (mut cursor, generation) = match started {
            Started::Resume { initial, cursor, generation } => {
                for event in initial {
                    yield Ok(event);
                }
                (cursor, generation)
            }
            // The snapshot streams here, in the generator, so the client's
            // disconnect drops it at the next yield and each page's reader
            // goes back to the pool before anything is sent.
            Started::Snapshot { cursor, generation } => {
                let mut after = 0;
                loop {
                    let scope = Scope::Page { after, limit: snapshot_page };
                    // A filter shape that CAN walk pages on the isolated
                    // runtime and pool, exactly as the list endpoint routes it
                    // (issue 120) — a connected subscriber paging a walk-shaped
                    // snapshot must never pin main-pool readers page after page
                    // (issue 163). `Shed` ends the stream like any other
                    // snapshot failure: the client re-subscribes and
                    // re-snapshots under the isolated pool's own admission.
                    let result = if store::read::walks(collection.into(), &filter) {
                        match isolated.read(collection, filter.clone(), scope).await {
                            Ok(result) => result,
                            Err(super::isolate::Shed) => {
                                let shed = "too many expensive filtered reads in flight; retry shortly";
                                yield Ok(error_event(&shed));
                                return;
                            }
                        }
                    } else {
                        match readers.get().await {
                            Ok(reader) => read_items(collection, &reader, &filter, scope).await,
                            Err(e) => Err(e),
                        }
                    };
                    let page = match result {
                        Ok(page) => page,
                        Err(e) => {
                            yield Ok(error_event(&e));
                            return;
                        }
                    };
                    let Some(last) = page.last() else { break };
                    after = last.id;
                    let full = page.len() as i64 == snapshot_page;
                    for item in &page {
                        // Deliberately NO SSE id on snapshot events: an
                        // EventSource that loses the stream mid-snapshot must
                        // reconnect with no Last-Event-ID and re-snapshot.
                        // Stamping these with the boundary cursor would make
                        // that reconnect a "resume from N" that silently skips
                        // the rest of the snapshot — entities the client would
                        // then never see. `live` is the first resumable point.
                        yield Ok(entity_event(
                            collection,
                            cursor,
                            "added",
                            item.id,
                            None,
                            Some(&item.json),
                            true,
                        ));
                    }
                    if !full {
                        break;
                    }
                }
                yield Ok(live_event(cursor, generation));
                (cursor, generation)
            }
        };
        loop {
            // Drain first, wait second. A resuming client's backlog was
            // committed before it connected, so no doorbell will ever ring for
            // it — reading before blocking is what makes resume work at all.
            // After a fresh snapshot the drain finds nothing, which is correct
            // and costs one indexed query.
            loop {
                let batch = match diff(collection, &readers, &filter, cursor, generation, include_data).await {
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

enum Started {
    /// A resuming client: replay `initial` (a `reset`, or nothing) and diff
    /// from `cursor`.
    Resume { initial: Vec<Event>, cursor: i64, generation: i64 },
    /// A fresh client: stream the snapshot (in the generator, page by page),
    /// then diff from `cursor` — the log position captured *before* the first
    /// page, so a change landing mid-snapshot is re-delivered by the diff
    /// rather than lost.
    Snapshot { cursor: i64, generation: i64 },
}

/// Step 2's decision: where this subscription starts. One pooled read, no
/// transaction — the heavy part (the snapshot itself) happens lazily in the
/// stream so it can be cancelled and never buffers more than a page.
async fn start(readers: &Arc<Readers>, resume: Option<Resume>) -> Result<Started, store::turso::Error> {
    let reader = readers.get().await?;
    let generation = read::feed_generation(&reader).await?;

    if let Some(Resume { generation: from_generation, cursor: from }) = resume {
        // A cursor from another generation resumes NOTHING: the feed it indexed
        // was wiped, its entity ids were reissued, and an in-range replay would
        // silently merge two worlds (issue 46). The reset means "drop state and
        // re-subscribe fresh".
        if from_generation.is_some_and(|g| g != generation) {
            return Ok(Started::Resume {
                initial: vec![reset_event("feed_rebuilt", generation)],
                cursor: 0,
                generation,
            });
        }
        let oldest = read::oldest_cursor(&reader).await?;
        // The log is append-only and never renumbered, so "below the horizon"
        // can only happen if it was pruned — but the path exists either way,
        // and a client that invents a cursor gets an honest answer.
        if oldest > 0 && from < oldest - 1 {
            return Ok(Started::Resume {
                initial: vec![reset_event("cursor_expired", generation)],
                cursor: 0,
                generation,
            });
        }
        // Ahead of the head: only a stale cursor carried across an unsignalled
        // wipe can produce this (a bare-integer token from a pre-rebuild feed).
        // Same answer as any other incomposable resume.
        if from > read::latest_cursor(&reader).await? {
            return Ok(Started::Resume {
                initial: vec![reset_event("cursor_ahead", generation)],
                cursor: 0,
                generation,
            });
        }
        return Ok(Started::Resume { initial: Vec::new(), cursor: from, generation });
    }

    Ok(Started::Snapshot { cursor: read::latest_cursor(&reader).await?, generation })
}

/// "Your state does not compose with this feed — drop it and re-subscribe."
fn reset_event(reason: &str, generation: i64) -> Event {
    Event::default()
        .event("reset")
        .json_data(serde_json::json!({ "reason": reason, "generation": generation }))
        .expect("a literal object always serialises")
}

/// The end-of-snapshot marker: "you are caught up to `cursor`".
fn live_event(cursor: i64, generation: i64) -> Event {
    Event::default()
        .event("live")
        .id(resume_token(generation, cursor))
        .json_data(serde_json::json!({
            "cursor": cursor.to_string(),
            "generation": generation,
        }))
        .expect("a literal object always serialises")
}

/// The resume token diff/live events carry as their SSE id (issue 46):
/// generation-qualified, so a reconnect proves which feed the cursor indexes.
fn resume_token(generation: i64, cursor: i64) -> String {
    format!("{generation}:{cursor}")
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
    generation: i64,
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
            // Both probes can miss legitimately (the entity never matched the
            // filter on either side of the change) — but retirement also lands
            // here, because `retire_chunk_tx` hard-deletes the versions before
            // this loop ever probes them (`version_seq` NULL → seq 0 → both
            // scopes empty). The rows are gone, so the filter cannot be
            // evaluated against what the subscriber saw; the log row's own op
            // is the only witness. Over-deliver `removed` — clients treat it
            // as an idempotent delete — rather than let a subscriber keep a
            // ghost of a retired Tender forever (issue 164).
            (false, false) if change.op == "removed" => "removed",
            (false, false) => continue,
        };
        events.push(
            entity_event(
                collection,
                change.cursor,
                op,
                change.entity_id,
                change.version_seq,
                new.as_ref().map(|i| &i.json),
                include_data,
            )
            // The generation-qualified cursor as the SSE id is what makes
            // Last-Event-ID resume exact — for DIFF events only. Snapshot
            // events stay id-less so an interrupted snapshot re-snapshots
            // instead of "resuming" past its own missing remainder.
            .id(resume_token(generation, change.cursor)),
        );
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
    // No `.id()` here: whether an event is a resume point is the caller's
    // call. Diff events get the cursor as their SSE id; snapshot events must
    // not carry one (see the snapshot loop).
    Event::default()
        .event("change")
        .json_data(body)
        .expect("a literal object always serialises")
}

/// The entity kinds the public change feed carries (issue 211). The projection
/// also writes `lot_result`, `bid` and `contract` change rows for the canonical
/// layer, but those are not a public-feed concern: the SSE diff loop never emits
/// them (it drives off `Collection::entity_kind`, which covers only these three),
/// they are absent from the published `ChangeEvent.entity` enum, and their ids
/// resolve to no REST endpoint. The poll feed and webhook delivery filter to these
/// so all three transports carry the same documented, resolvable events.
pub(crate) const PUBLIC_CHANGE_KINDS: [&str; 3] = ["tender", "lot", "organization"];

/// Whether a raw `changes.entity_kind` belongs on the public change feed.
pub(crate) fn is_public_change_kind(kind: &str) -> bool {
    PUBLIC_CHANGE_KINDS.contains(&kind)
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

fn error_event(e: &dyn std::fmt::Display) -> Event {
    Event::default()
        .event("error")
        .json_data(serde_json::json!({ "message": e.to_string() }))
        .expect("a literal object always serialises")
}

