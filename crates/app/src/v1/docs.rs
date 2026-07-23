//! `GET /docs` — the human-readable reference for the public `/v1` API, served
//! by the server itself so it always describes the running version.
//!
//! Deliberately a single static, server-rendered HTML page (no docs framework,
//! no client JS): the surface is small and stable, and a plain axum route sits
//! beside `/_source` and `/health` outside the rate limiter, so reading the docs
//! never spends a caller's API budget. Kept in step with the code by hand — the
//! same discipline the `/v1/sql/schema` notes and the dashboard copy follow.

use axum::http::header;
use axum::response::{IntoResponse, Response};

/// Render the docs page. No state, no arguments: the content is a constant.
pub async fn page() -> Response {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], PAGE).into_response()
}

const PAGE: &str = r####"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>tender-db API reference</title>
<style>
  :root {
    color-scheme: light dark;
    --bg: #ffffff; --fg: #1a1a1a; --muted: #5a5a5a; --line: #e2e2e2;
    --card: #f7f7f8; --code-bg: #f0f0f2; --accent: #1f5fbf; --pill: #eef2fb;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #16171a; --fg: #e8e8ea; --muted: #9a9aa2; --line: #2c2d31;
      --card: #1e1f23; --code-bg: #24252a; --accent: #6ea8fe; --pill: #23293a;
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; background: var(--bg); color: var(--fg);
    font: 16px/1.6 system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
  }
  .wrap { max-width: 52rem; margin: 0 auto; padding: 2rem 1.25rem 5rem; }
  header.top { border-bottom: 1px solid var(--line); padding-bottom: 1rem; margin-bottom: 1.5rem; }
  header.top nav { font-size: .95rem; }
  header.top nav a { margin-right: 1rem; }
  h1 { font-size: 1.9rem; margin: .2rem 0; }
  h2 { font-size: 1.3rem; margin: 2.4rem 0 .6rem; padding-top: .4rem; border-top: 1px solid var(--line); }
  h3 { font-size: 1.05rem; margin: 1.4rem 0 .4rem; }
  a { color: var(--accent); text-decoration: none; }
  a:hover { text-decoration: underline; }
  p, li { color: var(--fg); }
  .muted { color: var(--muted); }
  code { background: var(--code-bg); padding: .1rem .35rem; border-radius: 4px; font-size: .88em; }
  pre {
    background: var(--code-bg); padding: .9rem 1rem; border-radius: 8px;
    overflow-x: auto; font-size: .85rem; line-height: 1.5;
  }
  pre code { background: none; padding: 0; }
  .method {
    display: inline-block; font-weight: 600; font-size: .78rem; letter-spacing: .03em;
    padding: .1rem .45rem; border-radius: 4px; background: var(--pill); color: var(--accent);
    margin-right: .5rem; vertical-align: middle;
  }
  .ep { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: .95rem; }
  table { border-collapse: collapse; width: 100%; margin: .6rem 0; font-size: .9rem; }
  th, td { text-align: left; padding: .4rem .6rem; border-bottom: 1px solid var(--line); vertical-align: top; }
  th { color: var(--muted); font-weight: 600; }
  .card { background: var(--card); border: 1px solid var(--line); border-radius: 8px; padding: .3rem 1rem; margin: 1rem 0; }
  .toc { columns: 2; column-gap: 2rem; font-size: .92rem; }
  @media (max-width: 34rem) { .toc { columns: 1; } }
  footer { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--line); font-size: .88rem; color: var(--muted); }
</style>
</head>
<body>
<div class="wrap">
<header class="top">
  <h1>tender-db API</h1>
  <p class="muted">Public procurement data — REST, live feeds, read-only SQL and webhooks. This page documents the API served at this host.</p>
  <nav>
    <a href="/">Dashboard</a>
    <a href="/v1">Service info</a>
    <a href="/v1/sql/schema">SQL schema</a>
    <a href="/_source">Source</a>
  </nav>
</header>

<p>Base URL: <code>https://tenders.zebreus.click</code>. All responses are JSON
(SSE excepted). The historical backfill is ongoing, so collection counts grow
over time. Domain terms (Tender, Lot, Bid, Notice, Source, Organization) are
defined in the project's <code>CONTEXT.md</code>.</p>

<div class="card"><nav class="toc">
  <a href="#conventions">Conventions</a><br>
  <a href="#collections">Collections</a><br>
  <a href="#filters">Filters &amp; pagination</a><br>
  <a href="#detail">Tender detail</a><br>
  <a href="#changes">Change feed (poll)</a><br>
  <a href="#sse">Live feed (SSE)</a><br>
  <a href="#event">Event schema</a><br>
  <a href="#sql">SQL endpoint</a><br>
  <a href="#webhooks">Webhooks</a><br>
  <a href="#accounts">Accounts &amp; tokens</a><br>
  <a href="#meta">Service &amp; licence</a><br>
</nav></div>

<h2 id="conventions">Conventions</h2>
<ul>
  <li><strong>JSON</strong> everywhere but SSE. Money is <code>{"cents": 1234, "currency": "EUR"}</code> (integer minor units — never a float). Timestamps are ISO 8601; a source that published a date only yields a date only, never an invented time.</li>
  <li>The change <strong>cursor is an opaque string</strong>. Compare cursors for equality and pass them back verbatim; do not parse or do arithmetic on them.</li>
  <li><strong>Auth</strong> (SQL + webhooks): <code>Authorization: Bearer tdb_…</code>. Create tokens on the <a href="/account">dashboard</a>.</li>
  <li><strong>Errors</strong> share one shape: <code>{"error": {"status": 404, "message": "no such tender"}}</code> with the matching HTTP status.</li>
  <li><strong>Rate limits</strong>: ~10 req/s per client (burst 50) across <code>/v1</code>; live streams capped at 5 per client; SQL has its own limits (below). Behind the proxy the client is keyed by <code>X-Forwarded-For</code>.</li>
</ul>

<h2 id="collections">Collections</h2>
<p>Four collection endpoints. Each returns a page of rows and — with
<code>Accept: text/event-stream</code> — becomes a live subscription (<a href="#sse">SSE</a>).</p>
<table>
  <tr><th>Endpoint</th><th>Returns</th></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/tenders</td><td>Tenders (current version of each), newest matching first.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/tenders/{id}</td><td>One Tender in full — see <a href="#detail">detail</a>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/lots</td><td>Lots (subdivisions of Tenders).</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/organizations</td><td>Canonical Organizations (buyers, bidders, winners).</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/organizations/{id}</td><td>One Organization by id — the counterpart of a detail's <code>parties[].organization_id</code>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices</td><td>Raw import records. No canonical change rows, so an SSE subscription here is a snapshot then silence.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices/{id}</td><td>One Notice by id — the counterpart of a version's <code>caused_by_notice_id</code>.</td></tr>
</table>
<pre><code>curl -s "https://tenders.zebreus.click/v1/tenders?limit=2"</code></pre>
<p>Envelope: <code>{"items": [ … ], "next_cursor": "1234"|null, "more": true|false}</code>.</p>
<p>Tender rows echo the <code>cpv</code> (CPV codes) and <code>country</code>
(NUTS place codes) they carry, so you can see why a row matched a
<code>cpv</code>/<code>country</code> filter.</p>

<h2 id="filters">Filters &amp; pagination</h2>
<p>All collections accept the same filter parameters — a subscription is a
collection query plus its filters:</p>
<table>
  <tr><th>Param</th><th>Meaning</th></tr>
  <tr><td class="ep">source</td><td>Source key, e.g. <code>ted</code>.</td></tr>
  <tr><td class="ep">country</td><td>ISO-3166 alpha-3 country, e.g. <code>DEU</code>.</td></tr>
  <tr><td class="ep">cpv</td><td>CPV code prefix, e.g. <code>45</code> (construction).</td></tr>
  <tr><td class="ep">buyer</td><td>Organization id that is the buyer.</td></tr>
  <tr><td class="ep">winner</td><td>Organization id that won at least one Lot.</td></tr>
  <tr><td class="ep">status</td><td><code>open</code> or <code>closed</code> (by submission deadline).</td></tr>
  <tr><td class="ep">min_value / max_value</td><td>Value in <strong>cents</strong>.</td></tr>
  <tr><td class="ep">kind</td><td>Tender/Lot kind flag.</td></tr>
  <tr><td class="ep">tender</td><td>Restrict Lots to one Tender id; on <code>/v1/notices</code>, list the Notices that caused that Tender's versions.</td></tr>
  <tr><td class="ep">limit</td><td>Page size, default 100, max 500.</td></tr>
  <tr><td class="ep">cursor</td><td>Opaque page position — pass back the previous page's <code>next_cursor</code>.</td></tr>
</table>
<p>An unknown or misspelled query parameter is rejected with <code>400</code>
rather than silently ignored, so a typo (<code>cvp</code> for <code>cpv</code>)
never reads as "everything matched".</p>
<p>Paginate by following <code>next_cursor</code> until <code>more</code> is false:</p>
<pre><code>curl -s "https://tenders.zebreus.click/v1/tenders?country=DEU&amp;status=open&amp;limit=50"
curl -s "https://tenders.zebreus.click/v1/tenders?country=DEU&amp;status=open&amp;limit=50&amp;cursor=14327"</code></pre>

<h2 id="detail">Tender detail</h2>
<p><code class="ep">GET /v1/tenders/{id}</code> returns the current version of a
Tender plus its satellites: <code>lots</code> count and <code>lot_details</code>,
<code>texts</code>, <code>amounts</code>, <code>dates</code>,
<code>classifications</code>, <code>parties</code>, <code>lot_results</code>
(award decisions, accumulating across framework/DPS rounds), <code>bids</code>,
<code>contracts</code>, and <code>versions</code> — each version naming the
<code>caused_by_notice_id</code> that produced it (the ADR-0001 traceability
chain). A missing id is <code>404</code>.</p>
<pre><code>curl -s https://tenders.zebreus.click/v1/tenders/14327</code></pre>

<h2 id="changes">Change feed — poll</h2>
<p><code class="ep">GET /v1/changes?since={cursor}</code> returns everything the
canonical layer learned after <code>since</code> (start at <code>0</code>). Same
events as SSE, without holding a connection open. Optional
<code>entity=tender|lot|organization</code> narrows the stream.</p>
<pre><code>curl -s "https://tenders.zebreus.click/v1/changes?since=0&amp;limit=100"</code></pre>
<p>Response: <code>{"events": [ … ], "last_cursor": "193055", "more": false}</code>.
Loop, passing <code>last_cursor</code> as the next <code>since</code>, until
<code>more</code> is false; then poll periodically for new ones. The cursor is
<em>learn order</em> (ingestion), independent of a notice's publication date —
historical backfill and live updates share one monotonic sequence, which is what
makes out-of-order ingestion harmless. Sort by <code>published_at</code> if you
want domain-time order.</p>

<h2 id="sse">Live feed — Server-Sent Events</h2>
<p>Send <code>Accept: text/event-stream</code> to any collection endpoint (with
any filters). The protocol:</p>
<ol>
  <li><strong>Snapshot</strong> — one <code>added</code> event per row currently matching your filter, read in a single consistent transaction.</li>
  <li>A <code>live</code> marker carrying the snapshot's cursor.</li>
  <li><strong>Diff</strong> — <code>change</code> events forever after, each re-evaluating your filter against the old and new version of the entity: a row moving <em>into</em> your filter is <code>added</code>, out of it <code>removed</code>, changed-within it <code>changed</code>.</li>
</ol>
<p>Each event's SSE <code>id</code> is the change cursor. On reconnect the browser
<code>EventSource</code> replays it as <code>Last-Event-ID</code>; for curl and
scripts pass it as that header or as <code>?cursor=</code>. Resuming skips the
snapshot and delivers exactly what you missed. A cursor below the log's retained
horizon gets a <code>reset</code> event (<code>{"reason":"cursor_expired"}</code>)
meaning "re-snapshot". Add <code>?include_data=true</code> to embed each entity's
current JSON in its event.</p>
<pre><code># -N disables curl's buffering so events arrive as they happen
curl -N -H "Accept: text/event-stream" \
  "https://tenders.zebreus.click/v1/tenders?country=DEU"

# resume from the last cursor you processed
curl -N -H "Accept: text/event-stream" -H "Last-Event-ID: 193000" \
  https://tenders.zebreus.click/v1/tenders</code></pre>
<p class="muted">Streams are capped at 5 per client. Notices have no live diffs
(snapshot then silence). A 15 s keep-alive comment holds the connection open.</p>

<h2 id="event">Event schema</h2>
<p>One shape across SSE <code>change</code> events, poll items and webhook
batches, so you can move between transports without reparsing:</p>
<pre><code>{
  "cursor":  "193055",          // opaque string; the SSE id
  "op":      "added|changed|removed",
  "entity":  "tender|lot|organization",
  "id":       14327,             // entity id
  "version":  3,                 // canonical version seq (null for notices)
  "changed_at": "2026-07-19T22:21:32Z"   // poll only
  // "data": { … }               // only with ?include_data=true on SSE
}</code></pre>

<h2 id="sql">SQL endpoint</h2>
<p><code class="ep">POST /v1/sql</code> (token required) runs <strong>one
read-only <code>SELECT</code></strong> against the public schema. Send the SQL as
the raw request body — it never travels in the URL, so it stays out of access
logs. Discover the queryable tables and views at
<a href="/v1/sql/schema"><code class="ep">GET /v1/sql/schema</code></a> (public,
no token); the main entry points are the current-state views
<code>v_tenders</code>, <code>v_lots</code>, <code>v_lot_results</code> and
<code>v_organizations</code>.</p>
<pre><code>curl -s -X POST https://tenders.zebreus.click/v1/sql \
  -H "Authorization: Bearer tdb_…" \
  --data 'SELECT source, count(*) FROM v_tenders GROUP BY source'</code></pre>
<p>Rules:</p>
<ul>
  <li>Exactly one statement, and it must be a bare <code>SELECT</code> — no writes, PRAGMA, ATTACH, EXPLAIN, CTE-wrapped writes or multi-statement bodies.</li>
  <li>The queryable surface is a positive allow-list: the <code>v_*</code> views and the public business tables (canonical, notice, quarantine, changes). Account, webhook and operator tables — and the raw-fetch registry, whose paths are server infrastructure — are never queryable, and a table not on the list is denied by default.</li>
  <li><strong>Time columns are epoch seconds in SQL</strong>, not ISO — unlike the REST responses above. <code>WHERE published_at LIKE '2012%'</code> matches nothing; use <code>strftime(published_at,'unixepoch')</code>. Each timestamp column is flagged in <a href="/v1/sql/schema">the schema</a>, which also carries per-table notes, enum vocabularies and worked examples.</li>
  <li><strong>Backfill in progress:</strong> the canonical <code>v_*</code> layer currently holds only projected tenders (2026 forward, until the historical backfill is projected), so a <code>v_*</code> query scoped to earlier years may return nothing yet; the <code>notice_*</code> and <code>quarantine</code> layers already hold the full imported history.</li>
  <li>Result caps: 10 000 rows / 10 MB — a capped response carries <code>"truncated": true</code>.</li>
  <li>Limits per token: 2 concurrent queries, 300 per hour, 10 s per query. Over-limit is <code>429</code> with <code>Retry-After</code>; any query past the time limit — a slow scan or a heavy aggregate alike — is <code>408</code>, and its server-side work is abandoned so it never holds a slot past the cap.</li>
  <li>Dialect gaps (Turso): no <code>WITH RECURSIVE</code>; window functions are partial (<code>row_number</code> and aggregate <code>OVER</code> work; <code>rank</code>/<code>lead</code>/<code>lag</code> and custom frames do not). A dialect or column error comes back as <code>400</code> with the engine's message.</li>
</ul>
<p>Response: <code>{"columns": [ … ], "rows": [[ … ]], "row_count": N, "truncated": false}</code>.</p>

<h2 id="webhooks">Webhooks</h2>
<p>Account holders register https URLs that receive change batches as signed
POSTs. Manage them on the <a href="/account">dashboard</a> or over the API
(token required); every endpoint is scoped to your account.</p>
<table>
  <tr><th>Endpoint</th><th>Does</th></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/webhooks</td><td>List your endpoints (no secrets).</td></tr>
  <tr><td class="ep"><span class="method">POST</span>/v1/webhooks</td><td>Register <code>{"url": "https://…"}</code>. Returns the signing secret <strong>once</strong>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/webhooks/{id}</td><td>One endpoint with its recent delivery attempts.</td></tr>
  <tr><td class="ep"><span class="method">DELETE</span>/v1/webhooks/{id}</td><td>Remove it.</td></tr>
  <tr><td class="ep"><span class="method">POST</span>/v1/webhooks/{id}/disable</td><td>Pause delivery.</td></tr>
  <tr><td class="ep"><span class="method">POST</span>/v1/webhooks/{id}/enable</td><td>Resume. Body <code>{"from_now": true}</code> drops the backlog; default replays what was missed.</td></tr>
</table>
<pre><code>curl -s -X POST https://tenders.zebreus.click/v1/webhooks \
  -H "Authorization: Bearer tdb_…" \
  -H "content-type: application/json" \
  -d '{"url":"https://example.com/hooks/tenders"}'</code></pre>
<h3>Delivery &amp; verification</h3>
<ul>
  <li>Each POST body is <code>{"cursor_from": "…", "cursor": "…", "events": [ … ]}</code> using the <a href="#event">event schema</a>.</li>
  <li><a href="https://www.standardwebhooks.com/">Standard Webhooks</a> headers: <code>webhook-id</code>, <code>webhook-timestamp</code>, <code>webhook-signature</code>. The signature is <code>v1,&lt;base64 HMAC-SHA256&gt;</code> over <code>{id}.{timestamp}.{body}</code>, keyed by your secret (the base64 body after the <code>whsec_</code> prefix). Verify it and check the timestamp to reject replays.</li>
  <li>Delivery is <strong>at-least-once</strong> and advances only on a <code>2xx</code>; the change log is the queue, so a recovered endpoint automatically catches up — there is no separate outbox.</li>
  <li>Retries back off 30 s → 2 m → 10 m → 1 h → 4 h → 12 h → daily. After ~3 days of continuous failure the endpoint auto-disables; re-enable it from the dashboard or the API.</li>
  <li>Only https URLs resolving to publicly-routable addresses are accepted (SSRF guard).</li>
</ul>

<h2 id="accounts">Accounts &amp; tokens</h2>
<p>Accounts are username + password only, created on the
<a href="/account">dashboard</a>. There is no email, so there is no password
reset — lose the password and you lose the account and its tokens. API tokens
(<code>tdb_…</code>) are shown once at creation. Check a token with:</p>
<pre><code>curl -s https://tenders.zebreus.click/v1/me -H "Authorization: Bearer tdb_…"</code></pre>

<h2 id="meta">Service &amp; licence</h2>
<table>
  <tr><td class="ep"><span class="method">GET</span>/v1</td><td>Service info: version, revision, current cursor, endpoint list, source offer.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/health</td><td>Readiness probe (process up, database answers).</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/_source</td><td>AGPL §13 corresponding-source offer for the running revision.</td></tr>
</table>
<p>tender-db is free software under <strong>AGPL-3.0-or-later</strong>. The
running server offers the source of its exact revision at
<a href="/_source">/_source</a>.</p>

<footer>
  tender-db · <a href="/">Dashboard</a> · <a href="/v1">Service info</a> ·
  <a href="/_source">Source (AGPL-3.0-or-later)</a>
</footer>
</div>
</body>
</html>
"####;
