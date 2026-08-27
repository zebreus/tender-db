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
    <a href="/v1/openapi.json">OpenAPI</a>
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
  <a href="#ordering">Ordering tenders</a><br>
  <a href="#lookups">Lookups by real-world key</a><br>
  <a href="#detail">Tender detail</a><br>
  <a href="#notice-content">Notice content</a><br>
  <a href="#changes">Change feed (poll)</a><br>
  <a href="#sse">Live feed (SSE)</a><br>
  <a href="#event">Event schema</a><br>
  <a href="#sql">SQL endpoint</a><br>
  <a href="#webhooks">Webhooks</a><br>
  <a href="#accounts">Accounts &amp; tokens</a><br>
  <a href="#performance">Performance</a><br>
  <a href="#caveats">Data caveats</a><br>
  <a href="#meta">Service &amp; licence</a><br>
</nav></div>

<h2 id="conventions">Conventions</h2>
<ul>
  <li><strong>JSON</strong> everywhere but SSE. Money is <code>{"cents": 1234, "currency": "EUR"}</code> (integer minor units — never a float). Timestamps are ISO 8601; a source that published a date only yields a date only, never an invented time.</li>
  <li>The change <strong>cursor is an opaque string</strong>. Compare cursors for equality and pass them back verbatim; do not parse or do arithmetic on them.</li>
  <li><strong>Auth</strong> (SQL + webhooks): <code>Authorization: Bearer tdb_…</code>. Create tokens on the <a href="/account">dashboard</a>.</li>
  <li><strong>Errors</strong> share one shape: <code>{"error": {"status": 404, "message": "no such tender"}}</code> with the matching HTTP status.</li>
  <li><strong>Rate limits</strong>: ~10 req/s per client (burst 50) across <code>/v1</code>; live streams capped at 5 per client; SQL has its own limits (below). Behind the proxy the client is keyed by <code>X-Forwarded-For</code>.</li>
  <li><strong>CORS</strong>: every endpoint that needs no token is callable from browser JavaScript on any origin (<code>Access-Control-Allow-Origin: *</code>), SSE resume preflights included — build a client-side app directly against the API. The token-gated endpoints (SQL, webhooks, <code>/v1/me</code>) are not CORS-open; call them server-side.</li>
</ul>

<h2 id="collections">Collections</h2>
<p>Four collection endpoints. Each returns a page of rows and — with
<code>Accept: text/event-stream</code> — becomes a live subscription (<a href="#sse">SSE</a>).</p>
<table>
  <tr><th>Endpoint</th><th>Returns</th></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/tenders</td><td>Tenders (current version of each), ascending id by default — see <a href="#ordering">ordering</a> for newest-first and closes-soon.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/tenders/{id}</td><td>One Tender in full — see <a href="#detail">detail</a>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/lots</td><td>Lots (subdivisions of Tenders).</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/organizations</td><td>Canonical Organizations (buyers, bidders, winners).</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/organizations/{id}</td><td>One Organization by id — the counterpart of a detail's <code>parties[].organization_id</code>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices</td><td>Raw import records. No canonical change rows, so an SSE subscription here is a snapshot then silence.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices/{id}</td><td>One Notice by id — the counterpart of a version's <code>caused_by_notice_id</code>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices/{id}/content</td><td>Everything the parser extracted from that Notice — see <a href="#notice-content">notice content</a>.</td></tr>
</table>
<pre><code>curl -s "https://tenders.zebreus.click/v1/tenders?limit=2"</code></pre>
<p>Envelope: <code>{"items": [ … ], "next_cursor": "1234"|null, "more": true|false, "ignored_filters": []}</code>.
<code>ignored_filters</code> names any filter you sent that this collection does not
apply (see below) — an empty array means every filter applied.</p>
<p>Tender rows echo the <code>cpv</code> (CPV codes) and <code>country</code>
(NUTS place codes) they carry, so you can see why a row matched a
<code>cpv</code>/<code>country</code> filter.</p>

<h2 id="filters">Filters &amp; pagination</h2>
<p>Every collection accepts the same filter vocabulary — a subscription is a
collection query plus its filters — but each applies only the subset that is
meaningful to it (see <a href="#applies">which filters apply where</a> below):</p>
<table>
  <tr><th>Param</th><th>Meaning</th></tr>
  <tr><td class="ep">source</td><td>Source key, e.g. <code>ted</code>.</td></tr>
  <tr><td class="ep">country</td><td>A <strong>NUTS place-code prefix</strong> matched against the tender's places. At the country level NUTS is ISO-3166 <strong>alpha-2</strong>, so Germany is <code>DE</code> (not <code>DEU</code>); a longer prefix narrows to a region, e.g. <code>DE1</code> (Baden-Württemberg) or <code>DEB35</code> (a specific place).</td></tr>
  <tr><td class="ep">cpv</td><td>CPV code prefix, e.g. <code>45</code> (construction).</td></tr>
  <tr><td class="ep">buyer</td><td>Organization id that is the buyer.</td></tr>
  <tr><td class="ep">winner</td><td>Organization id that won at least one Lot.</td></tr>
  <tr><td class="ep">bidder</td><td>Organization id that submitted a bid on at least one Lot — won or not, a superset of <code>winner</code>.</td></tr>
  <tr><td class="ep">status</td><td><code>open</code> or <code>closed</code> (by submission deadline).</td></tr>
  <tr><td class="ep"><code>min_value</code> / <code>max_value</code></td><td>Value in <strong>EUR cents</strong>, compared against the tender's highest amount converted to EUR at its publication date (the derived <code>eur_cents</code> — see <a href="#caveats">caveats</a>). A tender with no convertible amount never matches a value bound.</td></tr>
  <tr><td class="ep">currency</td><td>ISO&nbsp;4217 code, case-insensitive (e.g. <code>EUR</code>, <code>sek</code>) — Tenders/Lots whose current version publishes at least one amount in that currency, <em>as published</em>.</td></tr>
  <tr><td class="ep">kind</td><td>Tender/Lot kind flag; on <code>/v1/organizations</code>, the identifier scheme (e.g. <code>VAT</code>).</td></tr>
  <tr><td class="ep">tender</td><td>Restrict Lots to one Tender id; on <code>/v1/notices</code>, list the Notices that caused that Tender's versions.</td></tr>
  <tr><td class="ep">publication_id</td><td>The official notice number a source prints on its notices (e.g. a TED OJS number) — exact match. On <code>/v1/notices</code> the notice itself; on <code>/v1/tenders</code> the tender it caused. See <a href="#lookups">lookups</a>.</td></tr>
  <tr><td class="ep">identifier</td><td>An Organization's official identifier <em>value</em> (e.g. a VAT number); pair with <code>kind</code> for the scheme. See <a href="#lookups">lookups</a>.</td></tr>
  <tr><td class="ep">name_prefix</td><td>Organization-name prefix, Unicode case-insensitive (<code>mü</code> matches <code>MÜLLER</code>); switches the list to name order. Must not be empty. See <a href="#lookups">lookups</a>.</td></tr>
  <tr><td class="ep"><code>published_after</code><br><code>published_before</code></td><td>Bound Tenders by their current version's publication time. Unix seconds or RFC 3339; a single bound implies <code>sort=published_at</code>. See <a href="#ordering">ordering</a>.</td></tr>
  <tr><td class="ep"><code>deadline_after</code><br><code>deadline_before</code></td><td>Bound Tenders by submission deadline (rows without one never match). A single bound implies <code>sort=deadline</code>.</td></tr>
  <tr><td class="ep">sort</td><td>Tenders only: <code>id</code> (default), <code>published_at</code> or <code>deadline</code>. See <a href="#ordering">ordering</a>.</td></tr>
  <tr><td class="ep">order</td><td><code>asc</code> | <code>desc</code>. Defaults per sort: <code>published_at</code> newest-first, <code>deadline</code> soonest-first, <code>id</code> ascending (its only direction).</td></tr>
  <tr><td class="ep">limit</td><td>Page size, default 100, max 1000.</td></tr>
  <tr><td class="ep">cursor</td><td>Opaque page position — pass back the previous page's <code>next_cursor</code>, to the same query shape (a cursor is specific to its <code>sort</code>).</td></tr>
</table>
<p>An unknown or misspelled query parameter is rejected with <code>400</code>
rather than silently ignored, so a typo (<code>cvp</code> for <code>cpv</code>)
never reads as "everything matched".</p>

<h3 id="applies">Which filters apply where</h3>
<p>A filter that has no meaning for a collection is <em>accepted but not applied</em>
— <code>cpv</code> on <code>/v1/organizations</code>, say, does not narrow anything,
because an Organization carries no CPV. So that an unfiltered page can never look
filtered, every list response names the filters it dropped in
<code>ignored_filters</code>; an empty array means all of them applied. The full map:</p>
<table>
  <tr><th>Collection</th><th>Applies</th><th>Accepted but ignored</th></tr>
  <tr><td class="ep">/v1/tenders</td><td>source, country, cpv, buyer, winner, bidder, status, min_value, max_value, currency, kind, publication_id, published_after/_before, deadline_after/_before (+ sort/order)</td><td>tender, identifier, name_prefix</td></tr>
  <tr><td class="ep">/v1/lots</td><td>source, country, cpv, buyer, winner, bidder, status, min_value, max_value, currency, kind, tender</td><td>publication_id, identifier, name_prefix, the date bounds</td></tr>
  <tr><td class="ep">/v1/organizations</td><td>country, kind, buyer, identifier, name_prefix</td><td>source, cpv, winner, bidder, status, min_value, max_value, currency, tender, publication_id, the date bounds</td></tr>
  <tr><td class="ep">/v1/notices</td><td>source, kind, publication_id, tender</td><td>country, cpv, buyer, winner, bidder, status, min_value, max_value, currency, identifier, name_prefix, the date bounds</td></tr>
</table>
<p>So <code>GET /v1/notices?country=DE</code> returns
<em>every</em> notice with <code>"ignored_filters": ["country"]</code> in the
envelope — not the German ones, and the field says so.</p>

<p>Paginate by following <code>next_cursor</code> until <code>more</code> is false:</p>
<pre><code>curl -s "https://tenders.zebreus.click/v1/tenders?country=DE&amp;status=open&amp;limit=50"
curl -s "https://tenders.zebreus.click/v1/tenders?country=DE&amp;status=open&amp;limit=50&amp;cursor=14327"</code></pre>

<h2 id="ordering">Ordering tenders</h2>
<p>Every collection lists in ascending id by default — a stable keyset order for
pagination, not a domain order. <code>/v1/tenders</code> additionally sorts by
domain time:</p>
<table>
  <tr><th>Query</th><th>Returns</th></tr>
  <tr><td class="ep">?sort=published_at</td><td>Newest first (default <code>order=desc</code>) — "what's new". <code>order=asc</code> for oldest-first.</td></tr>
  <tr><td class="ep">?sort=deadline</td><td>Soonest submission deadline first (default <code>order=asc</code>) — "closes soon". Only tenders that <em>have</em> a deadline appear; pair with <code>status=open</code> and <code>deadline_after</code> for still-open ones.</td></tr>
  <tr><td class="ep">?sort=id</td><td>The default ascending list, named explicitly. Descending id is not supported — use <code>sort=published_at</code> for newest-first.</td></tr>
</table>
<p>The date <strong>bounds imply their ordering</strong>: a
<code>published_after</code>/<code>published_before</code> bound alone implies
<code>sort=published_at</code>, a <code>deadline_after</code>/<code>deadline_before</code>
bound implies <code>sort=deadline</code>. Bounds on <em>both</em> columns need an
explicit <code>sort</code> to pick the ordering, else <code>400</code>. Instants
are unix seconds or RFC 3339 (the format the API itself serves; an unencoded
<code>+01:00</code> offset pasted into a URL works). All of this composes with the
other filters, and pagination is unchanged: follow <code>next_cursor</code>, back
into the <em>same</em> query shape — a cursor is specific to its sort. Sorted
reads are REST-only; an SSE subscription snapshots in id order and then follows
the change log, so <code>sort</code>/<code>order</code> on a stream is
<code>400</code>.</p>
<pre><code># the five newest tenders
curl -s "https://tenders.zebreus.click/v1/tenders?sort=published_at&amp;limit=5"

# open German tenders closing soonest
curl -s "https://tenders.zebreus.click/v1/tenders?sort=deadline&amp;status=open&amp;country=DE&amp;deadline_after=1786910000"

# everything published since August 1st, newest first (bound implies the sort)
curl -s "https://tenders.zebreus.click/v1/tenders?published_after=2026-08-01T00:00:00Z"</code></pre>

<h2 id="lookups">Lookups by real-world key</h2>
<p>The keys a consumer actually holds — an official notice number, a VAT number,
a company name — resolve directly, without knowing any internal id:</p>
<table>
  <tr><th>You hold</th><th>Query</th></tr>
  <tr><td>An official notice number</td><td class="ep">GET /v1/notices?publication_id=123456-2026</td></tr>
  <tr><td>An organization's official identifier (VAT &amp; co.)</td><td class="ep">GET /v1/organizations?identifier=RO42283735&amp;kind=VAT</td></tr>
  <tr><td>An organization's name</td><td class="ep">GET /v1/organizations?name_prefix=müller</td></tr>
</table>
<ul>
  <li><code>publication_id</code> is an exact match on the number the source printed on the notice; pair with <code>source=</code> if the same number could exist in two sources. An unknown number is an empty page, not a <code>404</code>. The same number on <code>/v1/tenders</code> resolves the <em>tender</em> it caused — through any of its versions, so a corrigendum's number still finds the procedure. From the notice, <code>/v1/notices/{id}/content</code> gives its parsed payload and a tender detail's <code>versions[].caused_by_notice_id</code> links back the other way.</li>
  <li><code>identifier</code> matches the official identifier <em>value</em>; <code>kind</code> names its scheme. This is the front door to participation history: resolve the VAT to a canonical org id, then ask <code>/v1/tenders?buyer=</code>, <code>?winner=</code> or <code>?bidder=</code> with it.</li>
  <li><code>name_prefix</code> is a prefix match on the organization's name, case-insensitive across the whole of Unicode (<code>müller</code>, <code>MÜLLER</code> and <code>Müller</code> all match), and switches the response to <strong>name order</strong> (id order otherwise breaks name-ordered pagination). It composes with <code>country=</code>/<code>kind=</code>; an empty prefix is <code>400</code>.</li>
</ul>
<pre><code># VAT → canonical org → everything they ever bid on
curl -s "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735"
curl -s "https://tenders.zebreus.click/v1/tenders?bidder=2"</code></pre>

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

<h2 id="notice-content">Notice content</h2>
<p><code class="ep">GET /v1/notices/{id}/content</code> returns <em>everything</em>
the parser extracted from one Notice — the full section tree with every typed
field value, verbatim from the parse layer (source field ids like TED business
terms, not canonical projections). Use it when the projected tender is not
enough: to see a field the canonical layer does not model, or to check what a
quarantined notice did yield. A held or unparsed notice returns its metadata
row via <code>/v1/notices/{id}</code> but an empty <code>sections</code> array
here; an unknown id is <code>404</code>.</p>
<pre><code>{
  "notice_id": 14327,
  "sections": [
    {
      "section_id": 1, "kind": "root", "parent_section_id": null,
      "values": [
        {"type": "text",   "field_id": "BT-21",  "ordinal": 0, "lang": "deu", "value": "…"},
        {"type": "amount", "field_id": "BT-27",  "ordinal": 0, "cents": 1200000, "currency": "EUR"},
        {"type": "date",   "field_id": "BT-131", "ordinal": 0, "value": "2026-09-01T10:00:00+02:00"}
      ]
    }
  ]
}</code></pre>
<p class="muted">Value types: <code>text</code>, <code>code</code>,
<code>classification</code>, <code>amount</code>, <code>date</code>,
<code>integer</code>, <code>number</code>, <code>id</code> — each carrying its
own fields (see the <a href="/v1/openapi.json">OpenAPI schema</a>). Values sort
by <code>(field_id, ordinal)</code> within their section.</p>

<h2 id="changes">Change feed — poll</h2>
<p><code class="ep">GET /v1/changes?since={cursor}</code> returns everything the
canonical layer learned after <code>since</code> (start at <code>0</code>). Same
events as SSE, without holding a connection open. Optional
<code>entity=tender|lot|organization</code> narrows the stream.</p>
<pre><code>curl -s "https://tenders.zebreus.click/v1/changes?since=0&amp;limit=100"</code></pre>
<p>Response: <code>{"events": [ … ], "last_cursor": "193055", "more": false,
"generation": 3}</code>.
Loop, passing <code>last_cursor</code> as the next <code>since</code>, until
<code>more</code> is false; then poll periodically for new ones. The cursor is
<em>learn order</em> (ingestion), independent of a notice's publication date —
historical backfill and live updates share one monotonic sequence, which is what
makes out-of-order ingestion harmless. Sort by <code>published_at</code> if you
want domain-time order.</p>
<p><strong>Store <code>generation</code> beside your cursor.</strong> It moves
only when the dataset is rebuilt from scratch (rare, operator-initiated). Events
across a rebuild do not compose: entity ids are reissued and your cursor indexes
a feed that no longer exists. When the generation you read differs from the one
you stored, drop your local state, re-fetch the collections you mirror, store the
new generation, and continue polling from that response's
<code>last_cursor</code>. <code>GET /v1</code> also reports the current
generation.</p>

<h2 id="sse">Live feed — Server-Sent Events</h2>
<p>Send <code>Accept: text/event-stream</code> to any collection endpoint (with
any filters). The protocol:</p>
<ol>
  <li><strong>Snapshot</strong> — one <code>added</code> event per row currently matching your filter, read in a single consistent transaction.</li>
  <li>A <code>live</code> marker carrying the snapshot's cursor.</li>
  <li><strong>Diff</strong> — <code>change</code> events forever after, each re-evaluating your filter against the old and new version of the entity: a row moving <em>into</em> your filter is <code>added</code>, out of it <code>removed</code>, changed-within it <code>changed</code>.</li>
</ol>
<p>Each event's SSE <code>id</code> is an opaque resume token
(generation-qualified cursor, e.g. <code>3:193055</code>). On reconnect the
browser <code>EventSource</code> replays it as <code>Last-Event-ID</code>; for
curl and scripts pass it as that header or as <code>?cursor=</code>, verbatim.
Resuming skips the snapshot and delivers exactly what you missed. A
<code>reset</code> event means your token cannot resume and you must drop local
state and re-subscribe fresh: <code>{"reason":"cursor_expired"}</code> (the log's
retained horizon passed your token) or <code>{"reason":"feed_rebuilt"}</code>
(the dataset was rebuilt — same event a poll client detects as a
<code>generation</code> change). Add <code>?include_data=true</code> to embed
each entity's current JSON in its event.</p>
<pre><code># -N disables curl's buffering so events arrive as they happen
curl -N -H "Accept: text/event-stream" \
  "https://tenders.zebreus.click/v1/tenders?country=DE"

# resume from the last event id you processed, verbatim
curl -N -H "Accept: text/event-stream" -H "Last-Event-ID: 3:193000" \
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
no token). The <code>v_*</code> views — <code>v_tenders</code>, <code>v_lots</code>,
<code>v_lot_results</code>, <code>v_organizations</code>, the convenience views
<code>v_tender_buyers</code>, <code>v_awards</code>,
<code>v_tender_classifications</code>, <code>v_tender_amounts</code>,
<code>v_tender_dates</code>, <code>v_tender_notices</code>, and
<code>v_fetches</code> for path-free provenance — show the current version of each
row and are the readable way to <em>see</em> the shape of the data.</p>
<p><strong>But do not filter a view.</strong> A <code>WHERE</code> on a view is
applied only after the whole view has been built, so even
<code>WHERE id = 12345</code> reads the entire corpus and hits the time limit. For
anything filtered, query the base tables and take the current version through
<code>tenders.current_seq</code> — measured at 17 ms for the point read below,
against a view that cannot answer it at all.</p>
<pre><code>curl -s -X POST https://tenders.zebreus.click/v1/sql \
  -H "Authorization: Bearer tdb_…" \
  --data 'SELECT t.id, t.current_title AS title, v.publication_id
            FROM tenders t
            JOIN tender_versions v
              ON v.tender_id = t.id AND v.seq = t.current_seq
           WHERE t.id = 12345'</code></pre>
<p>Rules:</p>
<ul>
  <li>Exactly one statement, and it must be a bare <code>SELECT</code> — no writes, PRAGMA, ATTACH, EXPLAIN, CTE-wrapped writes or multi-statement bodies.</li>
  <li>The queryable surface is a positive allow-list: the <code>v_*</code> views and the public business tables (canonical, notice, quarantine, changes). Account, webhook and operator tables — and the raw-fetch registry, whose paths are server infrastructure — are never queryable, and a table not on the list is denied by default.</li>
  <li><strong>Time columns are epoch seconds in SQL</strong>, not ISO — unlike the REST responses above. <code>WHERE published_at LIKE '2012%'</code> matches nothing. Put the FORMAT FIRST: <code>strftime('%Y', published_at, 'unixepoch')</code>. The reversed order, <code>strftime(published_at,'unixepoch')</code>, returns <code>NULL</code> for every row and raises no error — so it silently collapses a histogram into one empty bucket. Each timestamp column is flagged in <a href="/v1/sql/schema">the schema</a>, which also carries per-table notes, enum vocabularies and worked examples.</li>
  <li><strong>Coverage:</strong> the canonical layer holds the full imported history — 7.9M Tenders, 1993 to today (1993 alone has 49k). An earlier version of this page warned that only 2026 forward was projected; that backfill has long since completed.</li>
  <li>Result caps: 10 000 rows / 10 MB — a capped response carries <code>"truncated": true</code>.</li>
  <li>Limits per token: 2 concurrent queries, 300 per hour, 10 s per query. Over-limit is <code>429</code> with <code>Retry-After</code>; any query past the time limit — a slow scan or a heavy aggregate alike — is <code>408</code>, and its server-side work is abandoned so it never holds a slot past the cap. A <code>503</code> is different and means the query never ran: the backend had no capacity, so retry it unchanged rather than rewriting it.</li>
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
  <li><strong>Feed rebuild.</strong> Every body carries <code>generation</code> (see the <a href="#changes">change feed</a>). When the dataset is rebuilt, the next delivery to each endpoint is a reset notice — an empty batch marked <code>{"reset": "feed_rebuilt", "generation": N, "events": []}</code>, signed like any other — telling you your mirrored state no longer composes: drop it, re-fetch the collections you mirror, and resume from the <code>cursor</code> it carries. You receive this even if no events are flowing, so a rebuild is never silent.</li>
</ul>

<h2 id="accounts">Accounts &amp; tokens</h2>
<p>Accounts are username + password only, created on the
<a href="/account">dashboard</a>. There is no email, so there is no password
reset — lose the password and you lose the account and its tokens. API tokens
(<code>tdb_…</code>) are shown once at creation. Check a token with:</p>
<pre><code>curl -s https://tenders.zebreus.click/v1/me -H "Authorization: Bearer tdb_…"</code></pre>

<h2 id="performance">Performance</h2>
<p>How the API responds by <strong>query shape</strong>. The rule of thumb: anything
reachable by id or a small page is index-served and returns in single-digit to tens of
milliseconds; a filter on a <em>selective</em> value is the only thing that can be slow,
and it is deliberately kept from affecting anything else.</p>
<p class="muted">Median of 5 warm server-side samples, measured 2026-08-15 against the
live corpus (≈7.9M tenders / 14.3M notices; the external-key lookups and sorts
2026-08-16, ≈24.6M organizations). Absolute numbers drift as the corpus grows
and the hardware changes — the <em>shape</em> is the durable part, not the exact
milliseconds.</p>

<div class="card" style="padding:1rem 1rem .5rem;">
<svg viewBox="0 0 620 288" style="width:100%;height:auto;" role="img"
     aria-label="Median response time by query type, in milliseconds">
  <!-- gridlines + axis -->
  <g stroke="var(--line)" stroke-width="1">
    <line x1="210" y1="15" x2="210" y2="250"/>
    <line x1="321" y1="15" x2="321" y2="250"/>
    <line x1="432" y1="15" x2="432" y2="250"/>
    <line x1="544" y1="15" x2="544" y2="250"/>
  </g>
  <g fill="var(--muted)" font-size="11" text-anchor="middle" font-family="system-ui,sans-serif">
    <text x="210" y="264">0</text>
    <text x="321" y="264">20</text>
    <text x="432" y="264">40</text>
    <text x="544" y="264">60</text>
    <text x="415" y="277" fill="var(--muted)">median response time (ms)</text>
  </g>
  <!-- bars -->
  <g font-family="ui-monospace,Menlo,monospace" font-size="11.5">
    <g fill="var(--muted)" text-anchor="end">
      <text x="203" y="34">metadata (/health, /docs)</text>
      <text x="203" y="64">SQL &mdash; bounded SELECT</text>
      <text x="203" y="94">list /v1/notices</text>
      <text x="203" y="124">point /v1/&hellip;/{id}</text>
      <text x="203" y="154">list /v1/tenders</text>
      <text x="203" y="184">filter ?country=DE</text>
      <text x="203" y="214">filter ?cpv=45</text>
      <text x="203" y="244">list /v1/organizations</text>
    </g>
    <g>
      <rect x="210" y="22" width="3"   height="15" rx="2" fill="var(--accent)"/>
      <rect x="210" y="52" width="6"   height="15" rx="2" fill="var(--accent)"/>
      <rect x="210" y="82" width="6"   height="15" rx="2" fill="var(--accent)"/>
      <rect x="210" y="112" width="11"  height="15" rx="2" fill="var(--accent)"/>
      <rect x="210" y="142" width="106" height="15" rx="2" fill="var(--accent)"/>
      <rect x="210" y="172" width="217" height="15" rx="2" fill="#d98a2b"/>
      <rect x="210" y="202" width="262" height="15" rx="2" fill="#d98a2b"/>
      <rect x="210" y="232" width="362" height="15" rx="2" fill="var(--accent)"/>
    </g>
    <g fill="var(--fg)" text-anchor="start">
      <text x="219"  y="34">0.5</text>
      <text x="222"  y="64">1.0</text>
      <text x="222"  y="94">1.1</text>
      <text x="227"  y="124">1.9</text>
      <text x="322"  y="154">19</text>
      <text x="433"  y="184">39</text>
      <text x="478"  y="214">47</text>
      <text x="578"  y="244">65</text>
    </g>
  </g>
</svg>
<p class="muted" style="margin:.2rem 0 .4rem;font-size:.82rem;">
  <span style="color:var(--accent);">&#9632;</span> main reader pool &nbsp;
  <span style="color:#d98a2b;">&#9632;</span> isolated pool (filterable reads that can walk) &mdash;
  a sparse filter (e.g. <code>?buyer=&lt;rare org&gt;</code>) can walk the whole corpus and is
  <strong>off-scale</strong>: it returns slowly or <code>503</code> under load, never blocking the bars above.
</p>
</div>

<table>
  <tr><th>Query shape</th><th>Example</th><th>Median</th><th>Pool</th></tr>
  <tr><td>Point lookup</td><td class="ep">GET /v1/{collection}/{id}</td><td>&lt;1&ndash;2 ms</td><td>main</td></tr>
  <tr><td>Small list</td><td class="ep">GET /v1/notices, /v1/lots</td><td>~1 ms</td><td>main</td></tr>
  <tr><td>Tender list (page)</td><td class="ep">GET /v1/tenders?limit=50</td><td>~19 ms</td><td>main</td></tr>
  <tr><td>Organization list</td><td class="ep">GET /v1/organizations?limit=50</td><td>~65 ms</td><td>main</td></tr>
  <tr><td>Lookup by external key</td><td class="ep">?publication_id=&hellip;, ?identifier=&hellip;, ?name_prefix=&hellip;</td><td>1&ndash;8 ms</td><td>main</td></tr>
  <tr><td>Ordered tender list</td><td class="ep">?sort=published_at, ?sort=deadline</td><td>2&ndash;13 ms</td><td>main</td></tr>
  <tr><td>Filter, common value</td><td class="ep">?country=DE, ?cpv=45</td><td>~40 ms</td><td>isolated</td></tr>
  <tr><td>Filter, absent value</td><td class="ep">?country=ZZ</td><td>&lt;1 ms*</td><td>isolated</td></tr>
  <tr><td>Filter, sparse value</td><td class="ep">?buyer=&lt;rare&gt;, ?winner=&lt;rare&gt;, ?bidder=&lt;rare&gt;</td><td>walks &rarr; up to a full scan; 503 under load</td><td>isolated</td></tr>
  <tr><td>Change feed</td><td class="ep">GET /v1/changes?since=0</td><td>&lt;1 ms</td><td>main</td></tr>
  <tr><td>SQL (bounded)</td><td class="ep">POST /v1/sql (indexed SELECT)</td><td>~1 ms</td><td>isolated, 10 s cap</td></tr>
  <tr><td>Metadata</td><td class="ep">/v1, /docs, /v1/openapi.json, /health</td><td>&lt;1 ms</td><td>&mdash;</td></tr>
</table>
<p class="muted" style="font-size:.85rem;">* an absent filter value short-circuits to an empty page.</p>

<h3>Why the shape looks like this</h3>
<ul>
  <li>Everything reachable by id or a small page is <strong>index-served</strong>, so it is sub-millisecond to tens of milliseconds regardless of corpus size. The organization list is the heaviest &ldquo;fast&rdquo; read because it counts each row's mentions.</li>
  <li>Filterable collection reads run on a <strong>separate isolated reader pool</strong>. A filter on a common value fills its page quickly; a filter on a <em>selective</em> value can walk the whole corpus, so it is kept off the main pool &mdash; it may be slow or return <code>503</code> under contention, but it <strong>never slows point lookups, indexed lists, or other clients</strong>. (Measured: main-pool reads stayed under 18 ms while a walking filter ran.) A <code>name_prefix</code> search paired with <code>country</code>/<code>kind</code> is one of these walking shapes; alone it is index-served and fast.</li>
  <li>For a fast, predictable read, filter on a value you expect to be common, keep <code>limit</code> modest, and paginate with the returned <code>next_cursor</code>. Ascending id is the default order everywhere (a stable keyset order for pagination); the tender <a href="#ordering">sorts</a> and the org <a href="#lookups">name search</a> ride their own indexes, so they are equally page-cheap at any depth.</li>
  <li><code>/v1/sql</code> is bounded by design: one <code>SELECT</code>, a 10-second cap, and its own runtime, so an expensive query returns <code>408</code> instead of degrading the REST surface.</li>
  <li>Rate limit: ~10 requests/second sustained, burst 50, per client &mdash; page within that.</li>
</ul>

<h2 id="caveats">Data caveats</h2>
<p>The corpus is served as published. Where the source is wrong, odd, or silent,
tender-db keeps the published value and documents the pattern here rather than
&ldquo;fixing&rdquo; data underneath you. These are the measured patterns a consumer
should know about; the <a href="/">dashboard</a> carries the live per-era quality
rates and the quarantine resolution ledger.</p>

<h3>Coverage varies by era</h3>
<ul>
  <li>The corpus spans 1993&ndash;today across fundamentally different source formats
  (a plain-text era, several structured-form generations, eForms). Field coverage
  differs by era &mdash; the dashboard's per-era panel is the live truth, and some
  historical extraction is still being actively backfilled, so old notices can
  <em>gain</em> content (arriving on the <a href="#changes">change feed</a> as changes).</li>
  <li>Award winners exist only where the source names them. In some dialects the
  publisher is largely silent &mdash; e.g. eForms sdk-0.1 (a 2022 pilot), where ~87%
  of award notices name nobody; where a winner <em>is</em> published it is resolved.
  Winner-coverage numbers on the dashboard exclude publisher silence from the
  denominator rather than reporting it as extraction failure.</li>
</ul>

<h3>Amounts</h3>
<ul>
  <li>Amounts are integer <strong>cents</strong> plus a currency code. A published amount
  with more than two fraction digits (real practice: unit-price mills, float artifacts)
  is rounded half-away-from-zero to the cent &mdash; error &le; half a cent; the archived
  notice keeps the original lexical value.</li>
  <li><strong>Negative amounts are source-published</strong>, kept as published. A known
  shape is the <code>-1.00</code> publisher sentinel.</li>
  <li><strong>Zero often means &ldquo;no value given&rdquo;</strong>, not a free tender:
  measured at 2.7&ndash;8.9% of EUR amounts depending on era. Filter zeros out of
  aggregates unless you specifically want them.</li>
  <li><code>tax_basis</code> is <code>incl</code>, <code>excl</code>, or NULL &mdash; NULL
  means the source did not say, and the incl/excl mix is era-biased; do not compare raw
  sums across eras without checking it.</li>
  <li>Served values are <strong>as published</strong> &mdash; 26 currency codes occur,
  including pre-euro national currencies, retired codes, and occasional codelist leaks
  (e.g. <code>OP_DATPRO</code>); nothing this API returns is converted. A derived
  EUR-at-publication-date column lives <em>beside</em> the published values
  (<code>eur_cents</code> via <a href="#sql">/v1/sql</a>; official ECB/ECU daily series
  plus the irrevocable euro conversion rates; NULL where no official rate resolves)
  and is what <code>min_value</code>/<code>max_value</code> compare against &mdash;
  see the filter table and CHANGELOG.md in the repository.</li>
  <li>Astronomical garbage magnitudes (10<sup>50</sup>-class) are quarantined at
  ingestion and never enter the corpus.</li>
</ul>

<h3>Dates</h3>
<ul>
  <li>Placeholder instants occur at ~55 per 100k dates: year-0000, 1899-12-31
  (spreadsheet epoch), year-2100 &mdash; published values, kept.</li>
  <li>Deadlines <em>before</em> the publication date are a stable 0.2&ndash;0.3%
  source background across two decades; treat &ldquo;deadline &lt; published_at&rdquo;
  as published noise, not a data-loss signal.</li>
</ul>

<h3>Codes and identities</h3>
<ul>
  <li>CPV-2003 and CPV-2008 classifications coexist (era-dependent); no cross-era
  mapping is applied. NUTS carries occasional pseudo-codes.</li>
  <li>Organizations are aggregated by identifier where the source publishes one, else
  by (name, country); mentions without either stay <em>provisional</em> single-mention
  organizations. The <code>provisional</code> flag on
  <code>/v1/organizations</code> tells you which kind you are looking at.</li>
</ul>

<p class="muted">Ingestion is strict by design: a notice the parser cannot fully and
faithfully represent is held in quarantine &mdash; whole, diagnosed, and disclosed on
the dashboard &mdash; rather than partially parsed. Current outstanding holds are a
few hundred members out of 2.4M ever held, each with a documented verdict.</p>

<h2 id="meta">Service &amp; licence</h2>
<table>
  <tr><td class="ep"><span class="method">GET</span>/v1</td><td>Service info: version, revision, current cursor, endpoint list, source offer.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/openapi.json</td><td>This API as an <a href="/v1/openapi.json">OpenAPI 3.0 document</a> — machine-readable, CORS-enabled, for client generators and API tooling.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/health</td><td>Liveness probe (process up; does not query the DB &mdash; see /health/deep).</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/metrics</td><td>Operational gauges in Prometheus text format (cursor, RSS, disk/WAL, per-job durations, quarantine counts). An operator surface, not part of the data API.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/_source</td><td>AGPL §13 corresponding-source offer for the running revision.</td></tr>
</table>
<p>Browse and try the API interactively in
<a href="https://petstore.swagger.io/?url=https%3A%2F%2Ftenders.zebreus.click%2Fv1%2Fopenapi.json">Swagger UI</a>
or <a href="https://redocly.github.io/redoc/?url=https%3A%2F%2Ftenders.zebreus.click%2Fv1%2Fopenapi.json">Redoc</a>
— both are the projects' hosted viewers, loading the spec straight from this
server.</p>
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
