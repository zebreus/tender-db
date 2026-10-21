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
  <a href="#data-sources">Data sources &amp; reuse terms</a><br>
  <a href="#meta">Service &amp; licence</a><br>
</nav></div>

<h2 id="conventions">Conventions</h2>
<ul>
  <li><strong>JSON</strong> everywhere but SSE. Money is <code>{"cents": 1234, "currency": "EUR"}</code> (integer minor units — never a float). Timestamps are ISO 8601 in the offset the source published. A source that published a date only yields a date only: a German portal's 2026-09-05 serves as <code>2026-09-05</code>, not <code>2026-09-04T22:00:00Z</code> (issue 367 unit 3). Rows stamped before that offset/precision pair was stored render the UTC instant until <code>repair-notice-instants</code> fills them in.</li>
  <li>The change <strong>cursor is an opaque string</strong>. Compare cursors for equality and pass them back verbatim; do not parse or do arithmetic on them.</li>
  <li><strong>Auth</strong> (SQL + webhooks): <code>Authorization: Bearer tdb_…</code>. Create tokens on the <a href="/account">dashboard</a>.</li>
  <li><strong>Errors</strong> share one shape: <code>{"error": {"status": 404, "message": "no such tender"}}</code> with the matching HTTP status &mdash; <em>every</em> status, including a <code>405</code> for a wrong method on a path that exists (which carries an <code>Allow</code> header beside the envelope).</li>
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
  <tr><td class="ep"><span class="method">GET</span>/v1/organizations/{id}</td><td>One Organization by id — the counterpart of a detail's <code>parties[].organization_id</code>. An id that a merge folded into another organization answers <code>308</code> to the survivor.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices</td><td>Raw import records. No canonical change rows, so an SSE subscription here is a snapshot then silence.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices/{id}</td><td>One Notice by id — the counterpart of a version's <code>caused_by_notice_id</code>.</td></tr>
  <tr><td class="ep"><span class="method">GET</span>/v1/notices/{id}/content</td><td>Everything the parser extracted from that Notice — see <a href="#notice-content">notice content</a>.</td></tr>
</table>
<pre><code>curl -s "https://tenders.zebreus.click/v1/tenders?limit=2"</code></pre>
<p>Envelope: <code>{"items": [ … ], "next_cursor": "1234"|null, "more": true|false, "ignored_filters": []}</code>.
<code>ignored_filters</code> names any filter you sent that this collection does not
apply (see below) — an empty array means every filter applied.</p>
<p>Organization ids are stable until two rows turn out to be one organization and
are merged. The merged-away id keeps working. <code>GET /v1/organizations/{id}</code>
answers <code>308</code> with <code>Location</code> and <code>"merged_into": &lt;id&gt;</code>,
and a <code>buyer</code>, <code>winner</code> or <code>bidder</code> filter on it is applied
to the survivor, with the page naming the rewrite in
<code>"resolved_filters": {"winner": {"asked": …, "merged_into": …}}</code>, present only
when something was rewritten. Store the survivor's id. Chains of merges are followed up to
8 hops. The merged-away organization's identifier keeps working too: the survivor lists
it in <code>merged_identifiers</code>, <code>?identifier=</code> finds the survivor by it,
and the page names the merge as
<code>"resolved_filters": {"identifier": {"asked": "…", "merged_into": [&lt;id&gt;, …]}}</code>
(see <a href="#lookups">lookups</a>). The one exception is a number a review found was
never the organization's own (<code>identifier_status: register_mismatch</code>, or
<code>related_entity</code> for a parent's, subsidiary's or sister company's number): when
that organization is re-keyed onto its own number, the old number — in any spelling — stops
answering it rather than following it. A publication number answered at another stored
spelling of it is named the same way, as
<code>"resolved_filters": {"publication_id": {"asked": "…", "matched": ["…"]}}</code>
(see <a href="#lookups">lookups</a>).</p>
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
  <tr><td class="ep">country</td><td>A <strong>NUTS place-code prefix</strong> matched against the tender's places. At the country level NUTS is ISO-3166 <strong>alpha-2</strong>, so Germany is <code>DE</code> (not <code>DEU</code>); a longer prefix narrows to a region, e.g. <code>DE1</code> (Baden-Württemberg) or <code>DEB35</code> (a specific place). <strong>NUTS is not ISO everywhere</strong>: Greece is <code>EL</code> and the United Kingdom is <code>UK</code> &mdash; <code>GB</code> is not a NUTS code at all and matches nothing, and <code>GR</code> is Greece's pre-2013 NUTS spelling, which the corpus still holds on notices published under it. A tender whose notices give the place of performance only as a country (eForms BT-5141; D&Ouml;E's realized-location country), and whose version carries no NUTS place code, is served that country's <strong>NUTS level-0 code</strong>, so <code>LT</code> finds it and <code>LT0</code> does not (Greece <code>EL</code>, the United Kingdom <code>UK</code>). Only the EU-27, the United Kingdom, Iceland, Liechtenstein, Norway, Switzerland, Albania, Montenegro, North Macedonia, Serbia and T&uuml;rkiye are served this way: a place in any other country, e.g. Egypt or the USA, carries none, and so does a place given as an overseas region's own ISO code (e.g. R&eacute;union, &Aring;land). The places are the <strong>place of performance</strong> only, in every era (issue 546): a legacy TED (r2.0.8, r2.0.9) notice's buyer and winner address regions are not a tender's places, and <code>?country=</code> does not match them. They stay on the notice layer, in <code>notice_classifications</code> under field ids <code>TED-CA_CE_NUTS</code> (buyer) and <code>TED-TENDERER_NUTS</code> (winner), and as <code>TED-NUTS</code> in the notice's address sections, readable through <code>/v1/sql</code>. A legacy tender none of whose notices publishes a place of performance (e.g. an F12/F13 design contest) serves none: no fallback to an address. A later notice that publishes only addresses (e.g. an F14 corrigendum) keeps the place an earlier one published. Until the issue-546 refold has run, a legacy tender folded before this change may still serve its buyer's or winner's region, and <code>?country=</code> may still find it by one.</td></tr>
  <tr><td class="ep">cpv</td><td>CPV code prefix, e.g. <code>45</code> (construction).</td></tr>
  <tr><td class="ep">buyer</td><td>Organization id that is the buyer.</td></tr>
  <tr><td class="ep">winner</td><td>Organization id that won at least one Lot &mdash; excluding awards where the winner is the tender's own buyer (<code>is_buyer</code>, see the <a href="#tender-detail-buyer">tender detail</a>).</td></tr>
  <tr><td class="ep">bidder</td><td>Organization id named as a tenderer on a published bid (TED eForms, eForms-DE), won or not. On FTS, an award supplier only: FTS publishes no losing bidders. The pre-eForms TED formats (text, internal-ojs, r2.0.8, r2.0.9) and DÖE sdk-0.1 publish winners without bids, so a win there appears under <code>winner</code> only. To find every tender where an organization is named as a tenderer or a winner, query both; on the formats without bids, an award to the tender's own buyer is in neither (issue 531).</td></tr>
  <tr><td class="ep">status</td><td><code>open</code> or <code>closed</code>, by submission deadline <strong>and by what the newest notice says</strong> (issue 533). <code>open</code>: a submission deadline in the future, and the procedure not closed by its newest notice. <code>closed</code>: everything else &mdash; no deadline in the future, or a newest notice that published results when every result the procedure has published is a no-award (all <code>clos-nw</code>: a cancellation, an FTS UK12 termination) and together they decide every lot of kind <code>Lot</code> (lots groups and parts need no result; a Tender-scoped result decides no lot, so it closes only a Tender with no <code>Lot</code>-kind lot), because a result notice usually publishes no deadline and the Tender carries its contract notice's (see <a href="#caveats">caveats &rarr; Dates</a>). Not where the Tender declares an open-ended scheme, whose no-award result reports one round: a dynamic purchasing system, procedure <code>oth-single</code>/<code>oth-mult</code>, a qualification-system (eForms 15) or dynamic-market (UK13&ndash;UK16) notice, or a current deadline more than a year after the publication of the first notice that published a deadline (an admission window; an explicit r2.0.9 <em>discontinued</em>/<em>unsuccessful</em> no-award closes even then). A winner, or a decision the notice withheld, never closes a Tender here; a later notice that publishes no results of its own (a re-launch contract notice, but also e.g. a corrigendum) reopens it. On <code>/v1/lots</code> a lot is closed with its Tender: a no-award for some lots of a Tender that stays open does not close those lots. The deadline may be the procedure's rather than the lot's &mdash; see <a href="#caveats">caveats &rarr; Dates</a>. <code>sort=deadline</code> and <code>deadline_after</code>/<code>deadline_before</code> still read the deadline alone.</td></tr>
  <tr><td class="ep"><code>min_value</code> / <code>max_value</code></td><td>Value in <strong>EUR cents</strong>, compared against the TENDER's head value in EUR: its elected headline figure converted to EUR at its publication date or, where <code>value_derivation</code> is set, the sum of its lots converted the same way (on <code>/v1/lots</code> too: the bound reads the Tender's value, not the lot's own <code>value</code>) (the derived <code>eur_cents</code> — see <a href="#caveats">caveats</a>). A tender with no convertible amount matches no value bound unless its lot awards are summed (<code>lot-sum-awards</code>, only where a total the buyer stated vouches for the sum, past the two guards issue 551 B6 put on that total: see <a href="#caveats">caveats</a>): then the bound compares that sum.</td></tr>
  <tr><td class="ep">currency</td><td>ISO&nbsp;4217 code, case-insensitive (e.g. <code>EUR</code>, <code>sek</code>) — Tenders/Lots whose current version publishes at least one amount in that currency, <em>as published</em> &mdash; any amount row, including the partner-only award-notice framework values (issue 506).</td></tr>
  <tr><td class="ep">procedure_type</td><td>The <strong>procedure type</strong> (eForms BT-105, <code>procurement-procedure-type</code>): one or more codes, comma-separated, case-insensitive, at most 10 &mdash; <code>open</code>, <code>restricted</code>, <code>neg-w-call</code>, <code>neg-wo-call</code>, <code>comp-dial</code>, <code>innovation</code>, <code>oth-single</code>, <code>oth-mult</code>, and the German national codes as published (eForms-DE&rsquo;s below-threshold <code>us-*</code>, DÖE sdk-0.1&rsquo;s <code>de-*</code>; not cross-walked onto the EU list). Exact match on the current version&rsquo;s procedure type &mdash; the latest <em>mapped</em> type a notice of the Tender stated; lots inherit their Tender&rsquo;s. The pre-eForms TED eras fold the form&rsquo;s own procedure checkbox where it names one type, else the <code>PR</code> code through a closed table (1 open, 2/3 restricted, 4 neg-w-call, T neg-wo-call); any other value or a silent notice folds nothing, so the earlier type stays in place (it can predate the latest notice), and a Tender no notice typed matches no code. E.g. <code>procedure_type=neg-wo-call</code> for awards without a prior call &mdash; complete for eForms, <em>not</em> before it: a legacy award is typed only when its form ticked a without-call box, and FTS&rsquo;s Procurement Act routes (<em>Direct award</em>, <em>Competitive flexible procedure</em>, &hellip;) are not mapped. Tenders/Lots.</td></tr>
  <tr><td class="ep">lang</td><td>Preferred language for the <em>picked</em> text values (the <code>title</code> on tenders, lots and the detail header): ISO&nbsp;639 code, case-insensitive (<code>de</code> and <code>DEU</code> both work). Fallback chain: requested &rarr; English &rarr; the notice's original language &rarr; any labelled &rarr; unlabelled. A <em>selector</em>, not a filter &mdash; it changes which title a row serves, never which rows match, so it is never reported in <code>ignored_filters</code>. The detail's <code>texts</code> array always carries every stored language variant regardless.</td></tr>
  <tr><td class="ep">kind</td><td>Tender/Lot kind flag &mdash; on <code>/v1/lots</code> one of <code>Lot</code>, <code>LotsGroup</code>, <code>Part</code> (any letter case; anything else is a 400); on <code>/v1/organizations</code>, the identifier scheme (e.g. <code>VAT</code>).</td></tr>
  <tr><td class="ep">tender</td><td>Restrict Lots to one Tender id; on <code>/v1/notices</code>, list the Notices that caused that Tender's versions.</td></tr>
  <tr><td class="ep">publication_id</td><td>The official notice number a source prints on its notices. A TED number is found in any of its spellings &mdash; TED's own unpadded one, the 6 or 8 digits the archive stores, the OJ S form &mdash; on TED notices only; another source's number (FTS, DÖE) matches as that source stores it. On <code>/v1/notices</code> the notice itself; on <code>/v1/tenders</code> the tender it caused. See <a href="#lookups">lookups</a>.</td></tr>
  <tr><td class="ep">identifier</td><td>An Organization's official identifier <em>value</em> (e.g. a VAT number); pair with <code>kind</code> for the scheme. See <a href="#lookups">lookups</a>.</td></tr>
  <tr><td class="ep">name_prefix</td><td>Organization-name prefix, Unicode case-insensitive (<code>mü</code> matches <code>MÜLLER</code>); switches the list to name order. It matches the organization's served name only: after a head refresh renames an organization, its former spelling no longer matches it. Must not be empty. See <a href="#lookups">lookups</a>.</td></tr>
  <tr><td class="ep"><code>published_after</code><br><code>published_before</code></td><td>Bound Tenders by their current version's publication time. Unix seconds, RFC 3339, a bare date (midnight UTC) or the literal <code>now</code>; <code>now</code> is accepted on pages only: an event stream refuses it with <code>400</code> (see <a href="#sse">Live feed</a>). A single bound implies <code>sort=published_at</code> (pages only; a stream snapshots in id order). See <a href="#ordering">ordering</a>.</td></tr>
  <tr><td class="ep"><code>deadline_after</code><br><code>deadline_before</code></td><td>Bound Tenders by submission deadline (rows without one never match). Same formats as the published pair; <code>now</code> is accepted on pages only: an event stream refuses it with <code>400</code> (see <a href="#sse">Live feed</a>). A single bound implies <code>sort=deadline</code> (pages only; a stream snapshots in id order).</td></tr>
  <tr><td class="ep">sort</td><td>Tenders only: <code>id</code> (default), <code>published_at</code> or <code>deadline</code>. See <a href="#ordering">ordering</a>.</td></tr>
  <tr><td class="ep">order</td><td><code>asc</code> | <code>desc</code>. Defaults per sort: <code>published_at</code> newest-first, <code>deadline</code> soonest-first, <code>id</code> ascending (its only direction).</td></tr>
  <tr><td class="ep">limit</td><td>Page size, default 100. Must be 1&ndash;1000 inclusive &mdash; a value outside that range is a <code>400</code>, not silently clamped.</td></tr>
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
  <tr><td class="ep">/v1/tenders</td><td>source, country, cpv, buyer, winner, bidder, status, min_value, max_value, currency, procedure_type, kind, publication_id, published_after/_before, deadline_after/_before (+ sort/order)</td><td>tender, identifier, name_prefix</td></tr>
  <tr><td class="ep">/v1/lots</td><td>source, country, cpv, buyer, winner, bidder, status, min_value, max_value, currency, procedure_type, kind, tender</td><td>publication_id, identifier, name_prefix, the date bounds</td></tr>
  <tr><td class="ep">/v1/organizations</td><td>country, kind, buyer, identifier, name_prefix</td><td>source, cpv, winner, bidder, status, min_value, max_value, currency, procedure_type, tender, publication_id, the date bounds</td></tr>
  <tr><td class="ep">/v1/notices</td><td>source, kind, publication_id, tender</td><td>country, cpv, buyer, winner, bidder, status, min_value, max_value, currency, procedure_type, identifier, name_prefix, the date bounds</td></tr>
</table>
<p>So <code>GET /v1/notices?country=DE</code> returns
<em>every</em> notice with <code>"ignored_filters": ["country"]</code> in the
envelope — not the German ones, and the field says so.</p>

<p>Paginate by following <code>next_cursor</code> until <code>more</code> is false. A page
can be short &mdash; even empty &mdash; while <code>more</code> is true: a filtered read that
has to walk the id order examines one bounded band of ids per page, and its cursor is the last
id <em>examined</em>, so following it never re-reads a range and always terminates. A
<code>publication_id</code> page does the same when it narrows its number's rows after the
read (by source, or by <code>source</code>/<code>kind</code> and the other filters): its cursor
is the last row examined. Page length is not an end-of-results signal; <code>more</code> is. One stream pages in a different order: a
lots read seeded by an organization (<code>winner</code> or <code>bidder</code>) is served in
<em>(tender, lot)</em> order &mdash; the order its participation index provides, so a page costs a
page however many lots the organization has &mdash; and its cursor is compound. Pass it back
verbatim like any other; it is not a lot id, and a cursor from another query shape restarts the
walk from the first page.</p>
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
<p>On a page the date <strong>bounds imply their ordering</strong>: a
<code>published_after</code>/<code>published_before</code> bound alone implies
<code>sort=published_at</code>, a <code>deadline_after</code>/<code>deadline_before</code>
bound implies <code>sort=deadline</code>. Bounds on <em>both</em> columns need an
explicit <code>sort</code> to pick the ordering, else <code>400</code> (pages only; a
stream snapshots in id order). Instants
are unix seconds, RFC 3339 (the format the API itself serves; an unencoded
<code>+01:00</code> offset pasted into a URL works), a bare <code>YYYY-MM-DD</code>
(the form a date-only publication is served in; taken as that day's midnight UTC),
or the literal <code>now</code> &mdash; which is what makes
<code>?deadline_after=now&amp;sort=deadline&amp;order=asc</code> the
&ldquo;closes soon&rdquo; query without the caller computing a timestamp that
goes stale the moment it is saved. No other word is accepted;
<code>today</code> would need a timezone this API has no notion of. <code>now</code> is
accepted on pages only: an event stream refuses it with <code>400</code>, because a resume
would resolve it again (see <a href="#sse">Live feed</a>). All of this composes with the
other filters, and pagination is unchanged: follow <code>next_cursor</code>, back
into the <em>same</em> query shape — a cursor is specific to its sort. Sorted
reads are REST-only; an SSE subscription snapshots in the list's own order (ascending id; the
organization-seeded lots stream in its <em>(tender, lot)</em> order) and then follows
the change log, whatever its date bounds. On a <code>/v1/tenders</code> stream,
<code>sort</code> unset or <code>id</code> and <code>order</code> unset or <code>asc</code>
are accepted and mean that id order, with or without date bounds (bounds on both columns
included); any other <code>sort</code> (<code>published_at</code>, <code>deadline</code>) or
<code>order=desc</code> is <code>400</code>. The other collections refuse
<code>sort</code>/<code>order</code> on a stream as on a page.</p>
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
  <tr><td>An official notice number</td><td class="ep">GET /v1/notices?publication_id=79997-2017</td></tr>
  <tr><td>An organization's official identifier (VAT &amp; co.)</td><td class="ep">GET /v1/organizations?identifier=RO42283735&amp;kind=VAT</td></tr>
  <tr><td>An organization's name</td><td class="ep">GET /v1/organizations?name_prefix=müller</td></tr>
</table>
<ul>
  <li><code>publication_id</code> takes a TED number as TED prints it, unpadded (<code>79997-2017</code>), at the 6 or 8 digits the archive stores (<code>079997-2017</code>, <code>00079997-2017</code>), or in the OJ S form (<code>2017/S 044-079997</code>); surrounding whitespace is ignored. The stored width follows the notice's format, not its year — unpadded in the 1993–2010 text era, 6 digits in the XML era (2011–2024), 8 in eForms (2022 on) — and a TED number is one notice per year whichever width you send. Those widths answer TED notices only. Other sources match their own number exactly: a UK Find a Tender (FTS) number is its 6-digit release id, <code>083685-2026</code> (also asked as <code>2026/S 000-083685</code>: issue 000 is no TED issue, so that form looks up the FTS spelling only), and a DÖE number is <code>&lt;uuid or number&gt;-&lt;version&gt;</code>, version suffix included. FTS numbers from 2021 on coincide with TED's 6-digit ones, so <code>083685-2026</code> answers FTS's notice and TED's <code>00083685-2026</code> alike — pair with <code>source=</code>. When the page matched a stored spelling other than the one sent, it names it in <code>"resolved_filters": {"publication_id": {"asked": "79997-2017", "matched": ["079997-2017"]}}</code>. An unknown number is an empty page, not a <code>404</code>. The same number on <code>/v1/tenders</code> resolves the <em>tender</em> it caused — through any of its versions, so a corrigendum's number still finds the procedure. From the notice, <code>/v1/notices/{id}/content</code> gives its parsed payload and a tender detail's <code>versions[].caused_by_notice_id</code> links back the other way.</li>
  <li><code>identifier</code> matches the official identifier <em>value</em>; <code>kind</code> names its scheme. This is the front door to participation history: resolve the identifier to a canonical org id, then ask <code>/v1/tenders?buyer=</code>, <code>?winner=</code> or <code>?bidder=</code> with it. An identifier can resolve to MORE THAN ONE canonical org — the identity index is deliberately not unique, and a shared VAT (an Organschaft) or an unfolded duplicate both occur — so take every id the lookup returns rather than the first (issue 329). It also finds an identifier a merge folded into an organization — a PPON whose organization was merged into the company-number one, another spelling of one register key: the survivor lists those in <code>merged_identifiers</code>, and a page that reached an organization that way says so in <code>"resolved_filters": {"identifier": {"asked": "GBPPONPWYP8439MZWY", "merged_into": [5718658]}}</code> (an array, for the same reason). With <code>kind</code>, the kind of the identifier that matched is what must agree. The value is matched as stored (normalised: upper-case letters and digits, no separators), so <code>GB-PPON-PWYP-8439-MZWY</code> is asked as <code>GBPPONPWYP8439MZWY</code>. A v4 UUID in <code>identifier</code> (<code>identifier_kind: "national"</code>) is a submission platform's record key, not a register number. Some German and Swiss platforms mint a fresh one per notice, so one body can appear as many organizations with one or two mentions each; aggregate such rows by name. Where one notice names a body twice under two such keys, the keys are dropped and its mentions join the body's name-keyed organization (issue 530).</li>
  <li><code>name_prefix</code> is a prefix match on the organization's name, case-insensitive across the whole of Unicode (<code>müller</code>, <code>MÜLLER</code> and <code>Müller</code> all match); runs of whitespace and a trailing <code>.</code>, <code>,</code> or <code>;</code> do not matter (<code>ACME Ltd.</code> finds <code>ACME Ltd</code>), with one exception: an organization keyed before issue 432 while it had an identifier, and never renamed since (a reverted rename restores that key), keeps its plain lower-cased name as its key, so a prefix that crosses whitespace other than one plain space in its name (a doubled space, a tab, a no-break space) does not reach it (a shorter prefix does). It matches the organization's served <code>name</code> only: after a head refresh renames an organization (issue 524), its former spelling no longer matches it. The rename is an <code>organization</code> row with op <code>changed</code> on <code>/v1/changes</code>, and a party's <code>mention_name</code> on the Tender detail keeps the spelling its notice printed. It switches the response to <strong>name order</strong> (id order otherwise breaks name-ordered pagination). It composes with <code>country=</code>/<code>kind=</code>; an empty prefix is <code>400</code>.</li>
</ul>
<pre><code># VAT → canonical org → the tenders naming it as a winner, then as a tenderer
# (ask both: a source without published bids serves its wins under winner only)
curl -s "https://tenders.zebreus.click/v1/organizations?identifier=RO42283735"
curl -s "https://tenders.zebreus.click/v1/tenders?winner=2"
curl -s "https://tenders.zebreus.click/v1/tenders?bidder=2"</code></pre>

<h2 id="detail">Tender detail</h2>
<p><code class="ep">GET /v1/tenders/{id}</code> returns the current version of a
Tender plus its satellites: <code>lots</code> count and <code>lot_details</code>,
<code>texts</code>, <code>amounts</code>, <code>dates</code>,
<code>classifications</code> (schemes <code>cpv</code>, <code>nuts</code> and <code>nature</code>
&mdash; the contract nature, <code>works</code>/<code>supplies</code>/<code>services</code>, in one
vocabulary across eras, plus <code>combined</code> where a pre-eForms notice published it; and
<code>procedure</code> &mdash; the procedure type, Tender scope only, the value the row&rsquo;s
<code>procedure_type</code> serves), <code>parties</code>, <code>lot_results</code>
(award decisions, accumulating across framework/DPS rounds), <code>bids</code>,
<code>contracts</code>, and <code>versions</code> — each version naming the
<code>caused_by_notice_id</code> that produced it (the ADR-0001 traceability
chain). A missing id is <code>404</code>.</p>
<p><strong>A party names its organization twice.</strong> Each <code>parties[]</code>
entry, and each bid's <code>parties[]</code> entry, carries <code>organization_name</code>,
the organization's <em>head</em> name &mdash; one name for every tender the organization
appears on, the <code>name</code> <code>/v1/organizations</code> serves &mdash; and
<code>mention_name</code>, the name the party's anchoring notice
published for it, served whether or not the two agree (null only when that notice published
no name). The anchoring notice is the version's own, or an earlier one of the chain when a
later notice republished no party in that role: parties carry forward role by role. The two
names differ for ordinary reasons: a trading name, a rename, another language.
They also differ for a bad one: a party binds to an organization by its official
identifier before its name, so a notice that published another organization's identifier
on a party puts that party &mdash; and its <code>organization_id</code> &mdash; on the
other organization, whose head then names it. <code>mention_name</code> still says what
the notice said (issue 456). For an organization with an identifier the head is
<em>elected</em> from the names its notice mentions published (issue 524): the operator's
<code>refresh-org-names</code> job serves a fit spelling of the winning cluster, its
whitespace collapsed (trimmed, every run one space). The votes are the mentions that
published a name (one with no name, a blank one or a non-name one does not vote), clustered
as one name across casing, diacritics, <code>the</code> and a legal form. The head changes
only when a cluster other than its own holds at least 55% of the votes, at least 3 votes and
at least twice the head's; or when the head's own cluster holds at most 1 vote and a unique
top cluster holds at least 3 votes and at least 25%; or when the head is empty or not a name
(a placeholder, a blank, a void-lot phrase) and some mention's name votes, whatever script
that name is written in. Even then it stays when the change would swap one legal-form family
for another (<code>AG</code> to <code>Sàrl</code>) while the head's own cluster holds at least
2 votes, drop a consortium mark unless the new name keeps a word besides its legal forms and
every such word is a word of the head, fail the NHS ODS
name check the old head passed, or, on the 25% arm only, serve an initialism of the head;
when the identifier is a placeholder (<code>KEINEANGABE</code>, <code>UNBEKANNT</code> or
<code>NICHTVORHANDEN</code> with any digits, or all zeros);
when no spelling of the winning cluster is fit to serve; when the identifier carries a
<code>wrong</code> or unapplied <code>related</code> review verdict; on the 55% arm, while
the organization's newest procedures still use the current head (a rename: the head names at
least 2 procedures and two thirds of the organization's newest as many), or while a notice
printed the current head within 3 years (1,095 days) before the job's plan; on the 55% and 25%
arms, while the current head is the only one of the organization's names printed on its newest
publication day; all three unless the head is unfit to serve for a reason other than being all lower
case, or carries a standalone number the other name lacks while the other name drops at most
one other word of it (a number the other name prints joined to a neighbouring word,
<code>SMD 3</code> against <code>Smd3</code>, is the name's own); a head unfit only through a
line break or a tab counts as unfit only when the other name's words lie inside one of its lines
or are exactly those of a run of its whole lines, and a head held this way is instead served as
its own words on one line when every break falls mid-phrase (the line before ends in an unspaced
hyphen or a joining word such as <code>und</code>, <code>of</code> or <code>z</code>, or the next
line starts lower case, and the line before does not end in a legal form, a comma, a semicolon or
a colon) and the name on one line does not itself carry such a standalone number, unless that
change would trip the legal-form, consortium, NHS or placeholder check above;
on the 55% arm, while a name
other than the winning one and the current head's also has votes and no notice printed the
winning name (any spelling of it) within a year (365 days) before the job's plan (an undated one
counts as not printed), unless one of those three holds was lifted for the head; on the 55% and
25% arms, when the change would serve a review body's name (a procurement review chamber or court) for a head
that is not one, a name led by a contracting organ (<code>Junta de Gobierno …</code>,
<code>Alcaldía …</code>) for a head that is not, the head's own words plus a code
(<code>(976)</code>), a name that adds a number after a comma or semicolon (an appended
address; not for a Czech or Slovak school), an ALL-CAPS respelling of a mixed-case head's
same words, the same words printed twice (<code>X - X</code>) for a head that is not, or the
head with its seat dropped (a tail of 2 or 3 words led by <code>w</code>, <code>we</code>,
<code>v</code>, <code>ve</code> or <code>vo</code>: <code>Ośrodek Pomocy Społecznej w
Kluczborku</code> to <code>Ośrodek Pomocy Społecznej</code>); and, on the 55% and 25% arms only, when the head is in another script than its
mentions (TED's English rendering). So the head
can change with no new notice, and every tender the organization is on shows the new name at
once. The rename arrives as an <code>organization</code> <code>changed</code> event, not a
Tender one: the parties join the organization when read. Where the job has not renamed an
organization, and always for one without an identifier (whose name is its matching key), the
head is the first spelling a notice published. <code>lot_results[].winners[]</code> carry
the head only: a
winner has no anchor to the notice's party block. In eForms the winning bid's
<code>parties[]</code> normally carry its published name. The detail's <code>parties[]</code>
(role <code>winner</code> or <code>Tenderer</code>) carry it only while the version's parties
still come from that round's notice &mdash; decisions accumulate, parties are replaced &mdash;
so an earlier round's winner, in a legacy era with no bids, can be named nowhere on the
detail; its published name is then in <code>organization_mentions</code> under the
decision's <code>notice_id</code>.</p>
<p id="tender-detail-buyer"><strong>A buyer named as its own winner is flagged, not
dropped.</strong> Some award notices name one organization as buyer <em>and</em> as
contractor &mdash; the same party, or the same name up to case and accents. A winner of that
shape carries <code>"is_buyer": true</code> on <code>lot_results[].winners[]</code>, and the
detail's <code>parties[]</code> entry for it (role <code>winner</code> or
<code>Tenderer</code>) carries the same key; it is absent everywhere else. The award is
published data and stays as published: in the notices read by hand it was either a publisher
repeating its own block in the contractor slot or a genuine in-house award to the
authority's own service, and a notice alone cannot tell the two apart. What the flag changes
is counting: <code>?winner=</code> does not count such an award as a win, and supplier
statistics on <code>/v1/sql</code> should filter <code>winner_is_buyer IS NULL</code>
(issue 484). The flag is written when a tender is folded, so a tender not re-folded since
2026-10 reads unflagged until it is.</p>
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
<p><strong>To ask whether a notice is held, read <code>parse_state</code>, not
the presence of <code>quarantine</code>.</strong> The <code>quarantine</code>
object on <code>/v1/notices/{id}</code> is the notice&rsquo;s hold
<em>history</em>: the row is kept after a member is reclaimed, so the reclaim
campaign stays auditable, and reclaimed is what happened to most of them. A
notice with <code>parse_state: "parsed"</code> and a non-null
<code>quarantine</code> whose <code>reprocessed_at</code> is set was held once,
was fixed, and is fully served &mdash; its content is right here. Only
<code>parse_state: "quarantined"</code> means held now.</p>
<pre><code>{
  "notice_id": 23555356,
  "sections": [
    {
      "section_id": "PROCEDURE", "kind": "Notice", "parent_section_id": null,
      "values": [ … ]
    },
    {
      "section_id": "LOT-0001", "kind": "Lot", "parent_section_id": "PROCEDURE",
      "values": [
        {"type": "text",    "field_id": "BT-21-Lot",  "ordinal": 0, "lang": "DEU", "value": "…"},
        {"type": "integer", "field_id": "BT-115-Lot", "ordinal": 0, "value": 1},
        {"type": "id",      "field_id": "BT-01(c)-Procedure", "ordinal": 0,
         "scheme": null, "is_ref": false, "value": "vgv"}
      ]
    }
  ]
}</code></pre>
<p class="muted">Section ids are the source&rsquo;s own
(<code>PROCEDURE</code>, <code>LOT-0001</code>, <code>ORG-0003</code>&hellip;),
not integers, and <code>kind</code> is the parser&rsquo;s section vocabulary
(<code>Notice</code>, <code>Lot</code>, <code>Organization</code>&hellip;);
<code>lang</code> is the source&rsquo;s own tag, uppercase ISO 639-2/T for
eForms. The <code>notice_id</code> above is illustrative: entity ids are scoped
to the feed&rsquo;s <code>generation</code> and are reissued by a rebuild, so
take one from <code>/v1/notices</code> rather than copying a literal.</p>
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
"generation": 3, "ignored_filters": []}</code>.
<strong>Only <code>entity</code> narrows this feed.</strong> The collection
filters (<code>country</code>, <code>cpv</code>, <code>source</code>,
<code>status</code>, <code>min_value</code>&hellip;) parse but are not applied
here, and each one you send is named back in <code>ignored_filters</code> rather
than silently dropped. To follow a filtered subset, subscribe to the collection
endpoint with <code>Accept: text/event-stream</code>, which does apply them.
Loop, passing <code>last_cursor</code> as the next <code>since</code>, until
<code>more</code> is false; then poll periodically for new ones. The resume
parameter is <code>since</code>: a <code>cursor</code> (the list routes'
spelling) is answered <code>400</code> rather than silently replaying the log
from the start. A page with
<code>more: false</code> has read to the end, and its <code>last_cursor</code>
is the feed head &mdash; the value <code>GET /v1</code> reports &mdash; even
when the log's newest rows are kinds this feed does not carry.
<code>limit</code> counts events: every page but the last is full.
If you ever send a cursor this feed did not issue — one past the head, or one
from before a rebuild — the answer carries a <code>reset</code> field
(<code>cursor_ahead</code> or <code>cursor_expired</code>) with an empty
<code>events</code> array and <code>last_cursor: "0"</code>: drop your state,
re-snapshot the collections, and resume from <code>0</code>. That is the same
verdict the SSE transport gives for the same cursor, so the two halves never
disagree about where you are. The cursor is
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
any filters). Two exceptions answer <code>400</code> to a stream request rather than
quietly returning JSON:
<code>/v1/notices?tender=&lt;id&gt;</code>, which is a lookup rather than a
subscription (subscribe to <code>/v1/notices</code> without
<code>tender</code> if you need a live feed); and, on <code>/v1/tenders</code>, a date
bound spelled as the literal <code>now</code> (pass an absolute timestamp; see resuming,
below). The protocol:</p>
<ol>
  <li><strong>Snapshot</strong> — one <code>added</code> event per row currently matching your filter, read page by page with no transaction held across the pages, so it is not a point-in-time view. Under concurrent writes it is <strong>at-least-once</strong>: an entity written during the snapshot can arrive as <code>added</code> and again as a <code>change</code> after <code>live</code>, and one that left your filter before its page was read is not sent, while its <code>removed</code> is. Apply <code>added</code> as an upsert by id and <code>removed</code> as an idempotent delete — a <code>removed</code> can name an entity you never received; the diff stream then makes your final state exact, with known exceptions: on <code>/v1/lots</code>, a filter that reads the lot's Tender (<code>country</code>, <code>cpv</code>, <code>buyer</code>, <code>winner</code>, <code>bidder</code>, <code>procedure_type</code>, <code>currency</code>, <code>status</code>) is checked again only when a change row is written for the lot itself, so a version that moves only such a Tender fact (a second award notice naming a winner, the lot's own facts unchanged) can leave you holding a lot the filter now excludes, or not send you one it now admits (issue 565); and under <code>min_value</code>/<code>max_value</code>, a version since superseded is read as a superset on both sides, whether a later version was written in the same fold or the stream resumes, or lags, past it, so a <code>changed</code> or an <code>added</code> can name an entity outside the bound, on a live stream as well as a resumed one, until a later change probes the current version (issue 561).</li>
  <li>A <code>live</code> marker carrying the snapshot's cursor.</li>
  <li><strong>Diff</strong> — <code>change</code> events forever after, each re-evaluating your filter against the old and new version of the entity: a row moving <em>into</em> your filter is <code>added</code>, out of it <code>removed</code>, changed-within it <code>changed</code>. Apply <code>changed</code> as an upsert by id too (re-read the entity, or subscribe with <code>include_data</code>): under <code>min_value</code>/<code>max_value</code>, <code>source</code> or (on <code>/v1/tenders</code>) <code>kind</code> a <code>changed</code> can name an entity you never received where <code>added</code> was due &mdash; an earlier version's value is not stored and is read as a superset (issue 561), and when the fold re-labels a Tender's <code>source</code> or <code>kind</code> (a DÖE Tender turns <code>ted</code> when its TED twin arrives) both the version's own change and the <code>version: null</code> correction that follows read the Tender's current label on both sides, so a subscriber to the new label hears <code>changed</code> (or <code>added</code>, for an entity the version before did not hold) and one to the old label <code>removed</code> (issue 562). On <code>/v1/lots</code> a lot <code>changed</code> can arrive when only its Tender's value or closure moved: a lot's <code>min_value</code>/<code>max_value</code> bound and its <code>status</code> read its Tender, so a version that moves either is the lot's change too (issues 560 and 533).</li>
</ol>
<p>Each event's SSE <code>id</code> is an opaque resume token
(generation-qualified cursor, e.g. <code>3:193055</code>). On reconnect the
browser <code>EventSource</code> replays it as <code>Last-Event-ID</code>; for
curl and scripts pass it as that header or as <code>?cursor=</code>, verbatim.
Resuming skips the snapshot and delivers exactly what you missed. That is why a
<code>/v1/tenders</code> stream refuses a date bound spelled as the literal <code>now</code>
with <code>400</code>: a stream resolves <code>now</code> when its connection opens, so a resume
would resolve it again and check a later bound than the one your state was built under, and a
Tender that crossed the moving bound between the two connects would not be sent the move. Pass
an absolute timestamp instead (unix seconds, RFC 3339 or a bare date); pages accept
<code>now</code> as before (issue 563). In place of <code>deadline_after=now</code>,
<code>status=open</code> selects tenders still open, but <code>status</code> also reads the clock
when the stream connects, and has no absolute spelling: no event marks a deadline passing, so drop
a Tender from a <code>status=open</code> state once its deadline has passed (after a resume, a
change that leaves such a Tender closed is not sent; a version that reopens it arrives as
<code>added</code>). A
<code>reset</code> event means your token cannot resume and you must drop local
state and re-subscribe fresh: <code>{"reason":"cursor_expired"}</code> (the log's
retained horizon passed your token), <code>{"reason":"cursor_ahead"}</code> (your
token is past the feed's head, so this feed never issued it) or
<code>{"reason":"feed_rebuilt"}</code> (the dataset was rebuilt — same event a poll
client detects as a <code>generation</code> change). Add <code>?include_data=true</code> to embed
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
  "version":  3,                 // the version seq it is at; null if no version moved
  "changed_at": "2026-07-19T22:21:32Z"   // poll only
  // "data": { … }               // only with ?include_data=true on SSE
}</code></pre>
<p><strong><code>version: null</code></strong> marks a change that is not a version
transition: every <code>organization</code> event; an entity <code>removed</code>
because it left its Tender; and a <code>tender</code> or <code>lot</code>
<code>changed</code> whose existing versions were rewritten in place (an organization
merge re-pointing its parties; a re-derivation correcting what the fold stored, which
announces only the Tenders and lots whose reading moved; or the fold re-labelling a Tender's
<code>source</code> or <code>kind</code>, as when a DÖE Tender turns <code>ted</code>, which
announces the Tender and every lot of its current version). Re-read the entity by id and
upsert it &mdash; you may not hold it yet.</p>

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
<p><strong>Values.</strong> A Tender's headline value is
<code>tenders.current_value_eur_cents</code> (indexed). Where it is a derived lot sum (issue
551: <code>'lot-sum-vouched'</code>, or <code>'lot-sum-awards'</code> with the stated total that
vouches for it in the detail's <code>vouched_by</code>, served again since issue 551 B6 after its
switch-off on 19 October 2026),
<code>tenders.current_value_derivation</code> says so and
<code>current_value_cents</code> / <code>current_value_currency</code> hold the sum as money
(no amount row carries it; NULL on every elected head). A lot's is stored per version
on <code>tender_version_lots</code> &mdash; <code>value_cents</code>,
<code>value_currency</code>, <code>value_eur_cents</code> &mdash; and it is the figure
<code>/v1/lots</code> serves, chosen by the same rule (issue 490). The raw
<code>tender_version_amounts</code> rows carry every published figure, including the
placeholders and scale slips that rule refuses, so a <code>MAX()</code> over them is
not the lot's value. For a range or a top-N over lot values, query the base table
(<code>v_lots</code> carries the columns but, like every view, cannot be filtered),
keep <code>value_eur_cents IS NOT NULL</code> in the <code>WHERE</code> literally (the
index serves only that form), and put the lot table FIRST with a
<code>CROSS JOIN</code> &mdash; the one join on this page where the version table
drives, because its value index is what narrows the read:</p>
<pre><code>SELECT vl.tender_id, vl.lot_id, vl.value_cents, vl.value_currency
  FROM tender_version_lots vl
  CROSS JOIN tenders t ON t.id = vl.tender_id AND t.current_seq = vl.seq
 WHERE vl.value_eur_cents IS NOT NULL AND vl.value_eur_cents &gt;= 1000000000
 ORDER BY vl.value_eur_cents DESC LIMIT 20</code></pre>
<p>Two things the lot value is not. It is chosen by the largest PUBLISHED figure, so on
a lot published in two currencies <code>value_eur_cents</code> is the EUR of that figure,
not necessarily the lot's largest EUR amount. And it is chosen from the lot's own
amounts, never from award results: a lot whose only figure is an awarded value (most of
TED's 2021 r2.0.9 era) has no lot value &mdash; its award is in
<code>v_lot_results.awarded_cents</code>. Nor from the award notice's framework values: an
eForms award notice's framework maximum and re-estimate per lot result (BT-709, BT-660) are
kept in <code>amounts</code> at their lot as <code>result_framework_maximum</code> and
<code>result_framework_reestimate</code>, and act only as partners of the scale rules
(issue 506).</p>
<p>Rules:</p>
<ul>
  <li>Exactly one statement, and it must be a bare <code>SELECT</code> — no writes, PRAGMA, ATTACH, EXPLAIN, CTE-wrapped writes or multi-statement bodies.</li>
  <li>A few functions are refused (<code>400</code>) because one call can build a result larger than the endpoint's memory bound before any row is returned: the unbounded accumulating aggregates (<code>group_concat</code>, <code>string_agg</code>, <code>json_group_array</code>/<code>_object</code>, <code>array_agg</code>, <code>median</code>, <code>mode</code>, <code>percentile</code>) and the byte-amplifying scalars (<code>zeroblob</code>, <code>randomblob</code>, <code>replace</code>, <code>repeat</code>, <code>hex</code>, <code>quote</code>, <code>printf</code>/<code>format</code>). Aggregate with <code>count</code>/<code>sum</code>/<code>avg</code>/<code>min</code>/<code>max</code> and page with <code>LIMIT</code> instead (issue 426).</li>
  <li>The queryable surface is a positive allow-list: the <code>v_*</code> views and the public business tables (canonical, notice, quarantine, changes). Account, webhook and operator tables — and the raw-fetch registry, whose paths are server infrastructure — are never queryable, and a table not on the list is denied by default.</li>
  <li><strong>Time columns are epoch seconds in SQL</strong>, not ISO — unlike the REST responses above. <code>WHERE published_at LIKE '2012%'</code> matches nothing. Put the FORMAT FIRST: <code>strftime('%Y', published_at, 'unixepoch')</code>. The reversed order, <code>strftime(published_at,'unixepoch')</code>, returns <code>NULL</code> for every row and raises no error — so it silently collapses a histogram into one empty bucket. Each timestamp column is flagged in <a href="/v1/sql/schema">the schema</a>, which also carries per-table notes, enum vocabularies and worked examples.</li>
  <li><strong>Coverage:</strong> the canonical layer holds the full imported history — 7.9M Tenders, 1993 to today (1993 alone has 49k). An earlier version of this page warned that only 2026 forward was projected; that backfill has long since completed.</li>
  <li>Result caps: 10 000 rows / 10 MB — a capped response carries <code>"truncated": true</code>.</li>
  <li>Limits per token: 4 concurrent queries, 960 per hour, 15 s per query. Over-limit is <code>429</code> with <code>Retry-After</code>; any query past the time limit — a slow scan or a heavy aggregate alike — is <code>408</code>. The query is interrupted at the limit (issue 425), so an over-limit query does not keep running after its <code>408</code>. A <code>503</code> is different and means the query never ran: the backend had no capacity, so retry it unchanged rather than rewriting it.</li>
  <li>Dialect gaps (Turso): window functions are partial (<code>row_number</code> and aggregate <code>OVER</code> work; <code>rank</code>/<code>lead</code>/<code>lag</code> and custom frames do not). A dialect or column error comes back as <code>400</code> with the engine's message.</li>
  <li>Refused whatever the engine supports, because their work grows with the data's fan-out rather than with the query: <code>WITH RECURSIVE</code>, any CTE that reads its own name or a later sibling (a newer engine runs those recursively even without the keyword), and <code>EXCLUDE</code> window frames.</li>
</ul>
<p>Response: <code>{"columns": [ … ], "rows": [[ … ]], "row_count": N, "truncated": false}</code>.</p>
<p><strong>What you may build on</strong> (ADR-0015): the allow-listed table and
view <em>names</em>, their existing columns, and this envelope are the contract —
new columns and tables appear without notice, but nothing is renamed or removed
without an entry in the repository's <code>CHANGELOG.md</code>. The SQL dialect
itself is described, not promised: it is whatever the current embedded engine
implements (the gaps above), and a representative set of query shapes is tested
against every build so a dialect change is caught before it ships. The
<code>currency_rates</code> table carries the full EUR-pivot rate series behind
<code>eur_cents</code> (ECB daily 1999&rarr;, the daily ECU series 1993&ndash;1998
&mdash; data CC&nbsp;BY&nbsp;4.0, Source: Eurostat &mdash; and the irrevocable
euro conversion rates), so you can convert published amounts your own way.</p>

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
  <tr><td>Filter, org reverse-lookup</td><td class="ep">?buyer=&lt;org&gt;, ?winner=&lt;org&gt;, ?bidder=&lt;org&gt;</td><td>seed-driven on both collections: ~0.5 s for an ordinary org; a few seconds for the most prolific (a distributor over 334k lots: ~3&ndash;4 s warm, longer cold) &mdash; the cost is the org&rsquo;s lot count, not the page (issue 388)</td><td>isolated</td></tr>
  <tr><td>Change feed</td><td class="ep">GET /v1/changes?since=0</td><td>&lt;1 ms</td><td>main</td></tr>
  <tr><td>SQL (bounded)</td><td class="ep">POST /v1/sql (indexed SELECT)</td><td>~1 ms</td><td>isolated, 15 s cap</td></tr>
  <tr><td>Metadata</td><td class="ep">/v1, /docs, /v1/openapi.json, /health</td><td>&lt;1 ms</td><td>&mdash;</td></tr>
</table>
<p class="muted" style="font-size:.85rem;">* an absent filter value short-circuits to an empty page. Every isolation-routed filter has a reachability probe and the compiler will not accept one without it (issue 371). A value that is PRESENT but rare is a different case and still walks: <code>?currency=DEM</code> answers in ~6 s, since a probe can only prove that nothing matches, never that a match is near. The shapes a probe cannot bound at all &mdash; a <code>country</code> over five characters, a <code>cpv</code> that is not one to eight digits &mdash; are refused at the API with a 400 instead of walking (issue 117).</p>

<h3>Why the shape looks like this</h3>
<ul>
  <li>Everything reachable by id or a small page is <strong>index-served</strong>, so it is sub-millisecond to tens of milliseconds regardless of corpus size. The organization list is the heaviest &ldquo;fast&rdquo; read because it counts each row's mentions.</li>
  <li>Filterable collection reads run on a <strong>separate isolated reader pool</strong>. A filter on a common value fills its page quickly; a filter on a <em>selective</em> value can walk the whole corpus, so it is kept off the main pool &mdash; it may be slow or return <code>503</code> under contention, but it <strong>never slows point lookups, indexed lists, or other clients</strong>. No read runs longer than 25 s: one that would is interrupted there and answered <code>503</code> saying so &mdash; narrow the filters rather than repeating it, since the same request costs as much again (issue 120). (Measured: main-pool reads stayed under 18 ms while a walking filter ran.) A <code>name_prefix</code> search paired with <code>country</code>/<code>kind</code> is one of these walking shapes; alone it is index-served and fast.</li>
  <li>For a fast, predictable read, filter on a value you expect to be common, keep <code>limit</code> modest, and paginate with the returned <code>next_cursor</code>. Ascending id is the default order everywhere (a stable keyset order for pagination), except the organization-seeded lots stream (<code>winner</code>/<code>bidder</code>), which pages in (tender, lot) order off its participation index; the tender <a href="#ordering">sorts</a> and the org <a href="#lookups">name search</a> ride their own indexes, so they are equally page-cheap at any depth.</li>
  <li><code>/v1/sql</code> is bounded by design: one <code>SELECT</code>, a 15-second cap, and its own runtime, so an expensive query returns <code>408</code> instead of degrading the REST surface.</li>
  <li>Rate limit: ~10 requests/second sustained, burst 50, per client &mdash; page within that.</li>
</ul>

<h2 id="caveats">Data caveats</h2>
<p>The corpus is served as published. Where the source is wrong, odd, or silent,
tender-db keeps the published value and documents the pattern here rather than
&ldquo;fixing&rdquo; data underneath you, except where a caveat below names a decode
(titles and descriptions, issue 538). These are the measured patterns a consumer
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
  <li><strong>Whether <code>lots</code>, <code>notice_subtype</code> or
  <code>original_lang</code> is empty follows the format</strong> (issue 535).
  <code>notice_subtype</code> is the eForms subtype (OPP-070; TED eForms, eForms-DE) or,
  on FTS, the UK form code, else the release tags joined with <code>+</code>, and null
  when the release has neither. It is null for the pre-eForms TED formats (text,
  internal-ojs, r2.0.8, r2.0.9) and DÖE sdk-0.1, which publish no subtype. Their own
  document type is kept out of this column (issue 235). It is in
  <code>/v1/notices/{id}/content</code> and in <code>notice_codes</code> on
  <code>/v1/sql</code>, field <code>TED-TD_DOCUMENT_TYPE</code> (r2.0.8, r2.0.9),
  <code>TXT-TD</code> (text), <code>TED-NAT_NOTICE</code> (internal-ojs) or
  <code>SDK01-NoticeTypeCode</code> (DÖE sdk-0.1). In this bullet, r2.0.8 includes the
  R2.0.7 files filed under that profile. A single-lot procurement serves <code>lots: 1</code> on
  r2.0.9 (its object description is a Lot) and on eForms. It serves <code>lots: 0</code>
  on the text era (no lot structure), on internal-ojs 2008 without a <code>LOTS</code>
  section, and on r2.0.8 unless a contract notice published a lot annex: an r2.0.8 award
  notice carries no lot section, divided or not. An r2.0.8 award takes the lot its contract
  notice printed under the same number (issue 568). It stays on the Tender
  (<code>lot_results[].lot</code> null) where the lots are lettered and the awards numbered,
  where the award prints no number, where only the award prints the number or two lots print
  it, where a lot was added only in a corrigendum's prose, where an earlier notice kept a lot at
  its position but dropped its number or printed another (a renumbering that cannot be joined,
  below), and where an award notice prints one number on all of its two or more awards while
  the Tender's lots print several numbers. An r2.0.8 award whose own title names several lots,
  names another lot than both its printed number and the lot that number finds (by that lot's
  own title or description), or carries another lot's title, or that prints several lot
  numbers, stays on the Tender (issue 568): a title only ever keeps an award off a lot, never
  puts it on one. Where two or more awards of one notice are titled with another lot that way,
  that notice does not number its awards by the contract notice's lots, and every award of it
  not confirmed by its own title (one naming exactly the number the award prints, as a lot and
  not as a position within one) stays on the Tender too. None of this applies on a Tender whose
  lots print fewer than two numbers. An r2.0.8 award's lot is inferred from the number it
  prints against the contract notice's lots; a small recorded residual (about 2 % of such
  Tenders) keeps an award on another lot. An r2.0.9 award takes the lot section of its own
  notice that prints its <code>LOT_NO</code> (else <code>LOT_NUMBER</code>), an exact match
  first, then the one lot whose number matches once normalised (<code>Lot 01</code> is
  <code>1</code>) (issue 567); an r2.0.9 award printing a list of numbers (<code>18,19</code>),
  a number two lot sections print, or no number where the sections' numbers are not their
  positions stays on the Tender. A pre-eForms TED notice's lot section joins the Tender lot an
  earlier notice printed under the same number, and its awards follow (issue 570): the first
  notice that publishes lots keys them, and those keys never move. A section with no number
  keeps its position, and so does every section of a notice where two of its sections name one
  lot, or where one prints a number no earlier lot prints at a position a lot with another
  number holds (<code>1</code> against <code>1-4</code>, <code>2.</code> against
  <code>2</code>), and of every notice after an eForms notice published lots. FTS makes a
  Lot only from a release's <code>tender.lots</code>, so an FTS Tender serves 0 until one
  of its releases lists a lot; award, contract and tenderUpdate releases often list none.
  A Tender that is only an r2.0.9 F14 corrigendum, or an eForms prior-information notice
  with no lot or part, has no Lot either. None is invented for them, so they have no row
  on <code>/v1/lots</code>: find them on <code>/v1/tenders</code>.
  <code>original_lang</code> is null where the notice states no language, as many DÖE
  sdk-0.1 notices do not. FTS serves the release's language, English when the release
  names none.</li>
  <li>About 2,650 award notices name the <strong>buyer as its own contractor</strong>
  (census, 2026-10): served as published with <code>is_buyer: true</code> on the winner,
  and left out of <code>?winner=</code> (see the <a href="#tender-detail-buyer">tender
  detail</a>). <strong>The flag is written when a tender is folded, and the corpus has not
  been re-folded since it was introduced:</strong> until the next corpus-wide re-fold only
  tenders a daily update rewrote carry it, and the rest read unflagged and count as wins,
  exactly as before. An in-house supplier under its own name (a municipality's road
  service, an Eigenbetrieb) is not flagged, even when it shares the authority's
  identifier &mdash; but a pre-2004 text-era notice that lists it as
  <code>&lt;Authority&gt;, &lt;Unit&gt;</code> publishes no separate name for it: the
  winner is read as the authority's name and is flagged as the authority's own award.
  Withheld names (<code>N/A</code>, <code>Confidential</code>) on both sides are not
  taken as the same party.</li>
</ul>

<h3>Texts</h3>
<ul>
  <li><strong>Titles and descriptions are served with a publisher&rsquo;s extra escaping
  decoded</strong> (issue 538). Some publishers escape a text two or three times before it
  reaches the notice (about 1&ndash;2&nbsp;% of TED eForms Tenders, a few per thousand in TED&rsquo;s
  r2.0.8 and r2.0.9 XML formats, about none elsewhere), so after the notice&rsquo;s own
  decoding it still read <code>modalit&amp;#224;</code>, <code>&amp;amp;quot;offre&amp;amp;quot;</code>
  or <code>&amp;amp;lt;br/&amp;amp;gt;</code>. In every <code>title</code> and
  <code>description</code> (Tender and lot: the list rows&rsquo; <code>title</code>, the detail&rsquo;s
  <code>texts[]</code>, and on <a href="#sql">/v1/sql</a> <code>tender_version_texts</code> and
  <code>tenders.current_title</code>) numeric character references (<code>&amp;#224;</code>,
  <code>&amp;#x2019;</code>) and the five XML names (<code>&amp;amp;</code>, <code>&amp;lt;</code>,
  <code>&amp;gt;</code>, <code>&amp;quot;</code>, <code>&amp;apos;</code>, in any letter case) are
  decoded, up to three levels deep. An escaped <code>&lt;br&gt;</code>, <code>&lt;br/&gt;</code> or
  <code>&lt;br /&gt;</code> becomes a line break, and an escaped private-use symbol (U+E000&ndash;U+F8FF,
  a Symbol-font bullet such as <code>&amp;#61655;</code>) becomes <code>&bull;</code>. Nothing else
  changes: other markup, other entity names (<code>&amp;nbsp;</code>), a reference to a control
  character other than tab, line feed and carriage return, and text mangled at the source
  (<code>modalit&agrave;;</code>) are served as published. The notice keeps the text as parsed
  (<code>/v1/notices/{id}/content</code>, <code>notice_texts</code>). The other text fields and organization names are not decoded: a
  name can still read <code>D&amp;apos;ACTION</code> or <code>Gore &amp;Amp; Associ&eacute;s</code>.
  Until the issue-538 refold reaches it, a Tender folded before the change serves its old text; the
  decoded text then arrives on the <a href="#changes">change feed</a> as a change.</li>
</ul>

<h3>Amounts</h3>
<ul>
  <li>Amounts are integer <strong>cents</strong> plus a currency code. A published amount
  with more than two fraction digits (real practice: unit-price mills, float artifacts)
  is rounded half-away-from-zero to the cent &mdash; error &le; half a cent; the archived
  notice keeps the original lexical value.</li>
  <li><strong>An amount can carry a <code>quality</code> key</strong>, and then its
  <code>value</code> is <code>null</code>: the row exists, but its number is not a figure
  to use. <code>"withheld"</code> &mdash; the notice suppressed the field and the stored
  number is the eForms <code>-1</code> placeholder. The same column is <code>quality</code>
  on <code>v_tender_amounts</code> through <a href="#sql">/v1/sql</a>; filter
  <code>quality IS NULL</code> before aggregating.</li>
  <li><strong>A legacy amount is read from its printed text when its machine attribute
  is that text scaled by an exact even power of ten.</strong> TED's 2011 generator wrote
  <code>FMTVAL</code> attributes such as 4.97&times;10<sup>16</sup> beside a printed
  <code>49 700</code>; the printed figure is the amount and is served like any other.
  Any other disagreement between the two keeps the attribute, as before.</li>
  <li><strong>Negative amounts are source-published</strong>, kept as published, and they
  are not one thing. About 15,500 are exactly <code>-1.00</code>, the eForms SDK's marker
  for a figure the buyer withheld. A much smaller set carries ordinary magnitudes
  (&minus;&euro;151m on a Troms&oslash; bank agreement, &minus;&euro;28m on waste
  treatment, scrap metal, waste paper, land leasing) and reads as
  <strong>revenue-side</strong>: the supplier pays the authority, and the minus sign is the
  publisher saying so in a field with no sign convention. These are ordinary award notices
  &mdash; subtypes 29 and 30, no concession marker &mdash; so nothing but the sign and the
  subject matter distinguishes them, and this API does not model a direction (issue 376).
  A negative <code>value</code> is therefore a figure to read, not a defect to discard.</li>
  <li><strong>Zero often means &ldquo;no value given&rdquo;</strong>, not a free tender:
  measured at 2.7&ndash;8.9% of EUR amounts depending on era. Filter zeros out of
  aggregates unless you specifically want them. Published zeros are kept in
  <code>amounts</code> as published; the derived EUR column the value filters
  compare treats a 0 as an absence and elects nothing from it, so a Tender whose
  only figure is 0 has <em>no known value</em> rather than a value of zero
  (24,647 Tenders; 11,793 of the zero rows sit on <code>result_value</code> and
  1,408 on <code>framework_maximum</code>, fields where 0 cannot be a price). The derived
  column also declines a conversion that ROUNDS to zero &mdash; a published CZK 0.10 or
  HUF 1.48 is a real figure worth less than half a euro cent, and &euro;0.00 is not what it
  is worth &mdash; so a zero never appears in it for any reason.</li>
  <li><strong>A very small value may be a per-unit rate</strong>, not the contract's worth.
  Three tenders titled &ldquo;Taxi Vehicles&rdquo; publish &pound;8.57 into both
  <code>estimated_value</code> and <code>result_value</code>; that is a fare, honestly
  published into a field with no per-unit marker. 8,951 Tenders serve a value at or under
  &euro;10. <strong>These are NOT filtered</strong>, unlike the placeholder classes above:
  a unit rate is a real figure, the source carries no field that distinguishes it from a
  total, and the band has no cliff to cut at (2,892 at or under &euro;0.10, 1,304 to
  &euro;1, 4,755 to &euro;10, then 30,191 more to &euro;1,000). Treat an implausibly small
  value as unreliable rather than as the contract's size.</li>
  <li><strong>A published <code>0.01</code> or <code>1.00</code> is usually a token</strong>,
  not a price &mdash; the largest placeholder class in the corpus. 59,000 Tenders published
  exactly one minor unit and 53,000 exactly one major unit, and the next value down carries
  36&times; fewer; ten currencies each spike at exactly one unit. Most of it sits on
  <code>result_value</code>, on ordinary award notices for works and services. Kept in
  <code>amounts</code> as published; the derived EUR column does not elect it, so these
  Tenders have <em>no known value</em>. <code>0.10</code> and <code>2.00</code> are a tail
  rather than a convention and ARE elected.</li>
  <li><code>tax_basis</code> is <code>incl</code>, <code>excl</code>, or NULL &mdash; NULL
  means the source did not say, and the incl/excl mix is era-biased; do not compare raw
  sums across eras without checking it.</li>
  <li>Served values are <strong>as published</strong> &mdash; 26 currency codes occur,
  including pre-euro national currencies, retired codes, and occasional codelist leaks
  (e.g. <code>OP_DATPRO</code>); nothing this API returns is converted (a <code>value</code>
  marked by <code>value_derivation</code> is a sum of the lots' published figures, added up in
  their own currency, not a conversion). A derived
  EUR-at-publication-date column lives <em>beside</em> the published values
  (<code>eur_cents</code> via <a href="#sql">/v1/sql</a>; official ECB/ECU daily series
  plus the irrevocable euro conversion rates; NULL where no official rate resolves)
  and is what <code>min_value</code>/<code>max_value</code> compare against &mdash;
  see the filter table and CHANGELOG.md in the repository.</li>
  <li>Only amounts that overflow the stored integer are refused at ingestion, so
  <code>amounts</code> carries every figure a notice published, however
  implausible. A Tender's <code>value</code> is the ONE figure elected from
  them, and it is still the publisher's own &mdash; never a corrected one &mdash;
  except where <code>value_derivation</code> says it is a lot sum (below). One
  scale slip is refused when the Tender itself holds the evidence: a figure worth
  &euro;1bn or more that is exactly 1,000&times; (or 10,000&times;, a
  million&times;&hellip;) another figure of the same Tender, in the same
  currency. Exactly 100&times; another figure is refused too, unless the big
  figure is a framework total: a procedure-level figure over two or more lots,
  whose smaller partner is a lot figure, and whose lots add up to between a tenth
  of it and all of it. A lot's own figure 100&times; another is kept only when that
  other figure only ever appears as a figure of other lots of the Tender, and the
  notice states an accepted procedure total at least as large. A lot's figure is also
  refused when it is exactly 100&times; to 1,000,000&times; what the notice's accepted
  procedure total leaves after its other lots that state the same field, where that
  remainder is at least a hundredth of the total. A procedure-level figure of
  &euro;1bn or more is refused with no partner at all when it is 100&times; to
  1,000,000&times; a sum of the notice's own lots, two or more of them, to within a cent
  per lot (issue 537): their latest awards and their award-notice framework values always
  count, their estimates and framework maxima only when another procedure-level figure of
  the Tender sits at the lots' scale; a framework total kept as above still stands, and a
  lots group restating the refused figure falls with it. A part carried from a prior
  information notice into a contract notice with lots is not elected while the
  notice has any other accepted figure; it stays on the part's own row. The refused figure stays in <code>amounts</code> and the election
  falls to the next one &mdash; which, on a Tender with several lots, would be one lot's figure or
  nothing. So where no procedure-level figure is accepted, one case serves the SUM of the head
  version's lots instead, in the lots' own currency, marked by <code>value_derivation</code>
  (issue 551): <code>lot-sum-vouched</code>, when the refused figure is exactly 10<sup>k</sup>
  times that sum (to a cent per lot) &mdash; the buyer's own total with its decimal separator
  lost vouches for it (each lot's latest award, else its estimates or framework maxima at the
  same k; the award notice's framework values, BT-709/BT-660, are never summed), when that sum is
  above the figure the election would serve without it. No amount row is invented:
  <code>amounts</code> keeps what was published, and the bounds and <code>sort</code> compare the
  sum's EUR value. A second case, <code>lot-sum-awards</code>, adds up each lot's latest award on a
  Tender that publishes no procedure-level figure at all (only placeholders) and whose value would
  otherwise be an award's copy or nothing, and only where a total the buyer itself stated vouches
  for the sum: some notice of the Tender states a total in the sum's currency <strong>within 1&nbsp;%
  of it</strong>, at any size (an award notice's total, BT-161, BT-118 or BT-1118, or TED
  <code>VAL_TOTAL</code>; a procedure estimate, BT-27 or TED <code>VAL_ESTIMATED_TOTAL</code>; either
  end of a TED <code>VAL_RANGE_TOTAL</code>; never one lot's own award), and
  <code>value_derivation.vouched_by</code> names it (<code>field</code>, <code>notice_id</code>,
  <code>cents</code>). Path W, as issue 551 calls it, was withdrawn in October 2026 (unit 4a): read
  against the notices after its first rollout, the sum was wrong on 9 of a uniform 50 of the 15,352
  Tenders it had changed and on 96 of the 229 it served at &euro;100m or more &mdash; framework
  ceilings or per-supplier maxima added up across lots that share one envelope, one lot counted
  under two keys, a sum farther from the notice's stated total than the figure it replaced, a
  &times;10<sup>k</sup> award slip. It was <strong>re-enabled with its guards</strong> (issue 551 unit
  4b, 18 October 2026): beside the earlier ones (no figure repeated on two or more lots at a tenth of the largest lot
  or more, nor on four or more lots and half of them, no lot 1, 10, 100 or 1,000 times the others,
  every award in one currency and on a lot the notice types, no lot number under two keys, no award
  or Tender-level copy refused as a scale slip or over &euro;100bn, lot estimates within 10&times; of
  the awards, the sum above today's figure), ten refusal families (R1&ndash;R10)
  keep the Tender's one figure when a lot was won by two or more separate tenders; when the awards'
  published lot numbers collide, list several lots or are missing across notices; when a notice's
  own total is below the lots it awards; when an award is 10&times; its lot's estimate, 10&times; or
  a tenth of an earlier award on the same lot, or 10&times; the next largest award; when an award
  reads as a closed lot, a unit rate, a ceiling copied per supplier or the procedure total booked on
  one lot; when a lot is restated under a second key or an award was resolved through another lot's
  tender; and, R10, when no stated total vouches for the sum. Measured before it was switched on, 29
  of a fresh uniform 30 of these sums read right against their notices, and all 20 at &euro;100m or
  more did. It was <strong>switched off again</strong> on 19 October 2026, when its rollout changed
  about twice as many Tenders as projected, past the stop rule set before it ran; read against
  their notices, 59 of 61 of the sums it had served read right, one neutral and one wrong, vouched
  by a single-lot award notice's own total that the sum did not include. It is <strong>served again
  with two guards on the vouching total</strong> (issue 551 B6): a total from a notice whose round
  has exactly one result does not vouch, and an award notice's total, or either end of a
  <code>VAL_RANGE_TOTAL</code>, vouches only if that notice has a result on at least one of the
  summed lots (a result that closes the lot or awards nothing counts). The two refuse 35 of the
  3,502 sums served before (of 31 of those read, 4 were wrong), and a fresh sample of 30 of the
  sums they keep all read right. So a Tender with several lots, no
  procedure figure and no vouching total serves ONE figure (an award's copy, or null), and one
  whose only figures are lot <em>estimates</em> or
  framework maxima still serves ONE lot's figure, never their sum (summed estimates moved 3 of 20
  adjudicated heads the wrong way: sums insured, a &times;1000 slip on every lot, a pseudo-lot
  restating the whole). A figure with neither such a partner nor such a lot sum is served as published,
  so a very large single value can still be a typo: in a sample of the residual
  &euro;10bn-and-up band (issue 471, October 2026) roughly a third were. Check
  the notice before quoting one.</li>
  <li><strong><code>min_value</code>/<code>max_value</code> compare the head's EUR
  value</strong>: the elected figure's derived EUR value or, where
  <code>value_derivation</code> is set, the lot sum's (above). The election skips five
  classes: negative amounts (mostly the SDK's withheld marker, ~15,500 rows &mdash;
  but see above, a few are revenue-side contracts and this bound loses them too),
  exactly zero (an absence, not a price &mdash; see the zero bullet above), exactly
  one minor or one major unit (a typed token, ~112,000 Tenders &mdash; see the bullet
  above), a run of
  nine or more identical digits, which is a form-width maximum rather than a
  figure (&euro;999,999,999.99 on street cleaning in a town of 47,000), and
  anything above &euro;100bn EUR-equivalent. A Tender whose only published
  amounts fall in those classes has <em>no known value</em> unless its lot
  awards are summed (<code>lot-sum-awards</code>, above, where a stated total vouches for the
  sum): otherwise its <code>value</code> is null and NEITHER bound returns it (issue 366).</li>
</ul>

<h3>Dates</h3>
<ul>
  <li>Placeholder instants occur at ~55 per 100k dates: year-0000, 1899-12-31
  (spreadsheet epoch), year-2100 &mdash; published values, kept. A Tender's
  headline <code>submission_deadline</code> (and <code>status</code>,
  <code>sort=deadline</code>, <code>deadline_after</code>/<code>_before</code>)
  ignores a deadline before 1990-01-01 or more than ten years past the notice's
  publication: a year-0016 or <code>1970-01-01</code> deadline is a typo or an
  epoch default, not a date. A Lot's <code>submission_deadline</code> and
  <code>/v1/lots?status=</code> apply the same window, so a lot is never open on a date its
  tender refuses. The published row stays in the version's dates.</li>
  <li>Deadlines <em>before</em> the publication date are a stable 0.2&ndash;0.3%
  source background WITHIN ONE NOTICE &mdash; that is where the measurement was
  taken. At ROW level the comparison means something different and the rate is
  <strong>37.6%</strong> (2,981,402 of 7,929,584): a tender row is a union of its
  versions, so <code>published_at</code> advances to the newest notice while a
  deadline the newest notice is silent about is carried forward from an earlier
  one. After an award notice, <code>deadline &lt; published_at</code> is the
  EXPECTED shape, not noise and not data loss (issue 370). The converse is the
  same carry: a result published BEFORE the deadline it carries leaves the row's
  <code>submission_deadline</code> in the future, which is why <code>status</code>
  reads the newest notice as well (issue 533, <a href="#filters">filters</a>): once every
  result the procedure has published is a no-award and together they decide every lot of
  kind <code>Lot</code>, a newest notice that published results closes the Tender outside an
  open-ended scheme (see filters) while its deadline is still served. A D&Ouml;E sdk-0.1 award notice's own <code>EndDate</code> is not a submission
  deadline (an ex-post award of a procedure without a participation contest) and maps to
  none; it stays in <code>/v1/notices/{id}/content</code>.</li>
  <li>A date-only <code>published_at</code> / <code>dispatched_at</code> is stored at its
  <strong>civil day's UTC midnight</strong> (issue 418), so <code>published_after=YYYY-MM-DD</code>
  finds it and a <code>/v1/sql</code> day or year grouping names the day the source stated. The
  parse layer keeps every date-only value at the publisher's local midnight (that is what
  <code>dates</code> on the detail shows), and rows stamped before 2026-09-19 sit there too &mdash;
  the day before, in UTC &mdash; until <code>repair-notice-instants</code> moves them.</li>
  <li>DÖE publishes no journal date: a DÖE <code>published_at</code> is the publication date the
  buyer requested (or one the portal stamped), raised to the day of the notice's IssueDate when
  that is later (issue 539), and a date only when that day wins. A DÖE sdk-1.x notice states
  neither date, so its <code>published_at</code> is its IssueDate itself, time included, the same
  instant as <code>dispatched_at</code>. A requested date is what the buyer
  asked for, and a change notice can repeat its first version's request, while the IssueDate lies
  inside the export that delivered the notice. A DÖE <code>published_at</code> usually runs
  1&ndash;3 days before the day of the portal's daily export that carries the notice, which is how
  the portal cuts its exports (a monthly export spans its month, and the lag shows there only at
  month boundaries). The IssueDate is <code>dispatched_at</code>, so a DÖE dispatch falls on or
  before the day of <code>published_at</code> rather than strictly before it. Until the issue-539
  drain re-dates the notices already stored, a DÖE notice dated before the deploy serves the
  requested (or stamped) date alone; on about 16,344 of them (counted 2026-10-19) its
  <code>dispatched_at</code> falls one or more days after <code>published_at</code>. A Tender
  refolded since the deploy already serves the new date on its versions.</li>
  <li>A Lot's <code>submission_deadline</code> is <strong>the lot's own if it published
  one, otherwise the procedure's</strong> (issue 389). The legacy form generations
  publish ONE procedure-level deadline for the whole notice &mdash; the r209 era's
  <code>DATE_RECEIPT_TENDERS</code> is procedure-scoped by design &mdash; so most
  pre-eForms lots have no date of their own, and that procedure deadline is the one a
  bidder submits by. It is also the one <code>?status=open</code> evaluates, for lots as
  well as tenders, which is why the row shows it: a lot returned as open carries the
  deadline that opened it. A lot that publishes its OWN deadline keeps it even when the
  procedure's is later &mdash; and <code>status</code> reads that same date, so such a lot is
  closed once its own passes, and no lot is ever opened by a SIBLING lot's deadline (issue 424).
  <strong><code>submission_deadline_scope</code> says which of the two it is</strong> &mdash;
  <code>lot</code> or <code>procedure</code> &mdash; on Lots and on Tenders alike (issue 370
  unit 4; on a Tender, <code>lot</code> means the elected deadline is a lot-level date, the head
  election taking the newest over both scopes). What the row still cannot say is whether the
  newest notice republished the date or the row carries it forward from an earlier version: the
  fold's fact tables carry no origin, and <code>dates</code> on
  <a href="#detail">the tender detail</a> (each entry naming its <code>lot</code>) is the
  nearest reading.</li>
</ul>

<h3>Codes and identities</h3>
<ul>
  <li><strong>CPV codes are served as the bare 8-digit code, one code per row</strong>,
  whatever the publisher wrote. Every source but D&Ouml;E's sdk-0.1 pilot era publishes
  exactly that; that era also publishes the check-digit form (<code>45421146-9</code>),
  the division alone (<code>50</code>), and several codes glued into one string. At the
  fold the check digit is dropped (it is derivable from the eight digits), a division is
  padded to CPV's own spelling of it (<code>50000000</code>), and a glued string becomes
  one row per code &mdash; so <code>?cpv=</code> and a <code>GROUP BY code</code> see one
  vocabulary. The archived notice keeps the published text.</li>
  <li>CPV-2003 and CPV-2008 classifications coexist (era-dependent); no cross-era
  mapping is applied. NUTS carries occasional pseudo-codes, and NUTS <em>vintages</em>
  coexist the same way CPV's do: a notice is coded in the revision current when it was
  published, and no code is rewritten afterwards. So Greece appears as <code>GR</code>
  before the 2013 revision and <code>EL</code> after it, and a query for one does not
  find the other.</li>
  <li>Organizations are aggregated by identifier where the source publishes one. A
  row without one is <em>provisional</em>, which means exactly that &mdash; no official
  identifier &mdash; and nothing more. Its identity is then NAME-scoped: mentions
  carrying the same normalised name and country resolve to one provisional row, so
  such a row can hold many thousands of mentions (issues 234, 351). It is not a
  promise of one mention, and a later identifier can still canonicalise or split it.
  The <code>provisional</code> flag on <code>/v1/organizations</code> tells you which
  kind you are looking at. An identifier is what a notice published, and a
  publisher can mistype it: where a check against the official register found the
  number belongs to another company (or to none), the organization carries
  <code>identifier_status: "register_mismatch"</code>. That number then never merges it
  with another organization, and another spelling of the number reaches it only under
  a matching name; a notice publishing exactly that number still resolves to it. A
  parent's, subsidiary's or sister company's number reads
  <code>"related_entity"</code> and is treated the same way until the organization is
  re-keyed onto its own number. Null means no review found a problem, not that the
  number was checked.</li>
</ul>

<p class="muted">Ingestion is strict by design: a notice the parser cannot fully and
faithfully represent is held in quarantine &mdash; whole, diagnosed, and disclosed on
the dashboard &mdash; rather than partially parsed. Current outstanding holds are a
few hundred members out of 2.4M ever held, each with a documented verdict.</p>

<h2 id="data-sources">Data sources &amp; reuse terms</h2>
<p>Every notice comes from one of three official publishers, and each publisher's
terms travel with its data. Every notice and Tender says which one in its
<code>source</code> field; <a href="/v1">/v1</a> lists the same terms as
<code>data_sources</code>. If you republish data from this API, carry the
statement of each source you use.</p>
<table>
  <tr><td class="ep">ted</td><td><a href="https://ted.europa.eu">TED (Tenders Electronic Daily)</a>, Publications Office of the European Union. Free reuse under Commission Decision 2011/833/EU (<a href="https://ted.europa.eu/en/legal-notice">https://ted.europa.eu/en/legal-notice</a>).<br>Source: TED, https://ted.europa.eu, © European Union. tender-db has transformed the data (parsed, normalised and merged into Tenders).</td></tr>
  <tr><td class="ep">doe</td><td><a href="https://oeffentlichevergabe.de">oeffentlichevergabe.de</a> (Datenservice Öffentlicher Einkauf). CC0 1.0 (<a href="https://creativecommons.org/publicdomain/zero/1.0/">https://creativecommons.org/publicdomain/zero/1.0/</a>): no attribution is required, and it is credited anyway.<br>Source: Datenservice Öffentlicher Einkauf, https://oeffentlichevergabe.de (CC0 1.0). Responsibility for a notice's content stays with the authority that published it.</td></tr>
  <tr><td class="ep">fts</td><td><a href="https://www.find-tender.service.gov.uk">Find a Tender Service</a> (UK), Cabinet Office. Open Government Licence v3.0 (<a href="https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/">https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/</a>).<br>Contains public sector information licensed under the Open Government Licence v3.0.</td></tr>
</table>

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
  <a href="/_source">Source (AGPL-3.0-or-later)</a> ·
  <a href="#data-sources">Data sources &amp; reuse terms</a>
</footer>
</div>
</body>
</html>
"####;

#[cfg(test)]
mod tests {
    use super::PAGE;

    /// Issue 490: the lot-value recipe /docs publishes must drive from the partial
    /// value index and seek `tenders` by primary key -- a plain JOIN let turso's cost
    /// model reorder it into a scan of `tenders` once the lot table outgrew ~2.8x
    /// the tenders (it does on prod). The SQL is read off the page itself, so the
    /// documented text and the pinned plan cannot drift apart.
    #[tokio::test]
    async fn the_lot_value_recipe_drives_from_the_partial_index() {
        let at = PAGE.find("<strong>Values.</strong>").expect("/docs has the Values paragraph");
        let block = &PAGE[at..];
        let open = block.find("<pre><code>").expect("a recipe") + "<pre><code>".len();
        let close = block[open..].find("</code></pre>").expect("recipe end");
        let sql = block[open..open + close].replace("&gt;", ">").replace("&lt;", "<");
        assert!(sql.contains("CROSS JOIN tenders t"), "the recipe pins its join order: {sql}");

        let path = format!("/tmp/tender-db-490-recipe-{}.db", std::process::id());
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
        let db = store::Db::open(&path).await.unwrap();
        db.build_tender_indexes().await.unwrap();
        let plan = |sql: String| {
            let db = &db;
            async move {
                db.measure_rows(&format!("EXPLAIN QUERY PLAN {sql}"))
                    .await
                    .unwrap()
                    .iter()
                    .map(|r| format!("{r:?}"))
                    .collect::<Vec<_>>()
            }
        };

        let got = plan(sql.clone()).await;
        let lots = got.iter().position(|l| l.contains("SEARCH vl USING") && l.contains("tender_version_lots_value_eur"));
        let tenders = got.iter().position(|l| l.contains("SEARCH t USING INTEGER PRIMARY KEY"));
        assert!(
            matches!((lots, tenders), (Some(a), Some(b)) if a < b),
            "the value index must drive and tenders be sought by rowid; plan:\n{}",
            got.join("\n")
        );
        assert!(!got.iter().any(|l| l.contains("SCAN t")), "tenders must not be scanned; plan:\n{}", got.join("\n"));

        // Without the literal term the partial index is out of reach -- which is why
        // the page says "literally".
        let bare = sql.replace("vl.value_eur_cents IS NOT NULL AND ", "");
        assert_ne!(bare, sql, "the negative case must actually drop the term");
        let got = plan(bare).await;
        assert!(
            !got.iter().any(|l| l.contains("tender_version_lots_value_eur")),
            "turso 0.7.2 matched the partial index without its WHERE term; plan:\n{}",
            got.join("\n")
        );
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }
    use crate::v1::DATA_SOURCES;
    use crate::v1::openapi::SPEC;

    /// Issue 446: every source the fetch registry archives has its reuse terms
    /// on `/docs`, field for field as `/v1` serves them. A new source added to
    /// `ingest::fetch::SOURCES` without terms fails here, before its data is
    /// served without the statement its licence asks for.
    ///
    /// Each check reads the source's OWN table row, split at `<br>` into the
    /// terms and the statement. A page-wide `contains` could not fail: TED's
    /// and DÖE's urls also occur inside their own attribution text, so deleting
    /// the publisher link left the check green (the pre-deploy review).
    #[test]
    fn every_archived_source_carries_its_reuse_terms_on_the_docs() {
        fn text(html: &str) -> String {
            let mut out = String::new();
            let mut in_tag = false;
            for c in html.chars() {
                match c {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    _ if !in_tag => out.push(c),
                    _ => {}
                }
            }
            out
        }
        for source in ingest::fetch::SOURCES {
            let terms = DATA_SOURCES
                .iter()
                .find(|s| s.source == source)
                .unwrap_or_else(|| panic!("source {source:?} is archived but has no DataSource entry"));
            let start = PAGE
                .find(&format!("<td class=\"ep\">{source}</td>"))
                .unwrap_or_else(|| panic!("/docs has no data-sources row for {source}"));
            let row = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
            let (terms_html, statement) = row.split_once("<br>").expect("terms<br>statement");
            assert_eq!(statement, format!("{}</td>", terms.attribution), "/docs must carry {source}'s statement verbatim");
            assert!(terms_html.contains(&format!("href=\"{}\"", terms.url)), "/docs must link {source}'s publisher");
            assert!(terms_html.contains(&format!("href=\"{}\"", terms.license_url)), "/docs must link {source}'s licence");
            let terms_text = text(terms_html);
            assert!(terms_text.contains(terms.name), "/docs must name {source} as /v1 does: {:?}", terms.name);
            assert!(terms_text.contains(terms.license), "/docs must name {source}'s licence as /v1 does");
        }
        assert_eq!(
            DATA_SOURCES.len(),
            ingest::fetch::SOURCES.len(),
            "a DataSource names a source the registry does not archive"
        );
    }

    /// Issue 370: retired claims must not survive on ANY served surface.
    ///
    /// 370 corrected thirteen published claims at their source and still shipped
    /// two of them live, because the fix walked a hand-written list of files and
    /// this page and the vendored spec were not on it. The `provisional` note in
    /// `sql.rs` got a detector (`the_provisional_note_describes_what_the_resolver
    /// _actually_does`, which also exercises the resolver); `/docs` and
    /// `/v1/openapi.json` got none, so the same sentence sat on prod for another
    /// eight days and was re-reported by the next reader to look.
    ///
    /// This is that detector for the remaining surfaces (`/v1/sql/schema`'s notes
    /// joined them in issue 535). It is a prose guard
    /// and deliberately narrow — it cannot prove a sentence is right, only that a
    /// specific retired reading does not come back. Narrow is the point: every
    /// entry below was served as fact and measurably false.
    #[test]
    fn no_served_surface_repeats_a_retired_claim() {
        // (retired phrasing, what actually holds now)
        const RETIRED: &[(&str, &str)] = &[
            ("single-mention", "issue 234 made identifier-less mentions reuse one row by (name, country)"),
            ("single mention", "issue 234 made identifier-less mentions reuse one row by (name, country)"),
            ("never merged", "issue 351 widened that reuse to country-less names under the wall"),
            ("work was abandoned", "a non-yielding aggregate keeps its slot past the 408 (issue 238)"),
            ("work is abandoned", "a non-yielding aggregate keeps its slot past the 408 (issue 238)"),
            // Issue 463: served from issue 55 (2026-08-09) on, after the snapshot
            // became keyset pages with a pooled read per page.
            ("single consistent transaction", "the SSE snapshot is paged, one read per page, at-least-once (issue 55)"),
            // Issue 531: served from issue 217 on. Legacy TED and DÖE sdk-0.1 publish
            // winners without bids, and FTS's bids are its award suppliers.
            ("a superset of winner", "legacy TED and DÖE sdk-0.1 wins have no bid row (issue 531)"),
            ("a superset of <code>winner</code>", "legacy TED and DÖE sdk-0.1 wins have no bid row (issue 531)"),
            // Issue 535: FTS writes its form code or release tags into the column, and
            // the pre-eForms TED eras and DÖE sdk-0.1 leave it NULL.
            ("eForms notice subtype id", "FTS fills it too, and legacy TED and DÖE sdk-0.1 publish none (issue 535)"),
            ("0 is normal for a single-lot procurement", "a single-lot tender serves 0 or 1 by format (issue 535)"),
            // Issue 531: the /docs lookup example promised the whole bid history
            // from `?bidder=` alone.
            ("everything they ever bid on", "?bidder= misses the wins of sources without published bids (issue 531)"),
            // Issue 536: served from issue 217 on. The lookup was exact, and the
            // exact match missed TED's own (unpadded) spelling of every notice after
            // 2010; the /docs example number answered `[]`.
            (
                "exact match on the number the source printed",
                "TED's widths, its OJ S form and trimmed input all reach the notice (issue 536)",
            ),
            ("publication_id=123456-2026", "the /docs lookup example answered [] (issue 536)"),
            // Issue 536 review: the filter table's row said it too, in other words,
            // and so did the OpenAPI parameter before 536.
            ("a TED OJS number) — exact match", "a TED number answers at every width it is stored at (issue 536)"),
            ("Exact-match an official publication number", "a TED number answers at every width it is stored at (issue 536)"),
            ("at most four seeks", "/v1/tenders reads the seed and the matched versions: four spellings, two statements (issue 536)"),
            // Issue 534: served from issue 393 on. `Tenderer` is every bidder whose
            // tender the notice lists, losers included (527's tender 229981), so the
            // party role names no winner set; `lot_results[].winners` / `?winner=` do.
            ("a winner is `winner` or `Tenderer`", "a Tenderer party is a bidder, won or not (issue 534)"),
            ("winners are 'winner' OR 'Tenderer'", "a Tenderer party is a bidder, won or not (issue 534)"),
            // The `/docs` spelling: this walk squashes whitespace only, so the HTML form
            // needs its own entry (as 531's does); pinned against a wrapped sample below.
            (
                "a winner is <code>winner</code> or <code>Tenderer</code>",
                "a Tenderer party is a bidder, won or not (issue 534)",
            ),
            // Issue 534: the issue-393 re-projection ran (job 1616); no band role list
            // shows a pre-393 element name since.
            ("before the issue-393 re-projection", "the issue-393 re-projection ran, job 1616 (issue 534)"),
            // Issue 534 unit 3: the refold ran (refold-fields 2512, project 2513, 2026-10-15); band
            // 5.2M serves no `AWARD_AND_CONTRACT_VALUE` or `RESPONSIBLE_FOR_MEDIATION_PROCEDURES` since.
            // Squash form, case-matching: the served sentence began "Until the issue-534 refold …".
            ("issue-534 refold has run", "the issue-534 refold ran, jobs 2512 and 2513 (issue 534)"),
            // Issue 532 unit 3: the refold ran (refold-fields 2521, project 2522, 2026-10-15); the
            // Verify and the DÖE witness 1365142 read done. The five caveats shared this token.
            ("issue-532 refold", "the issue-532 refold ran, jobs 2521 and 2522 (issue 532)"),
            // Issue 533: served from issue 370 on. A result notice usually publishes no
            // deadline, so the Tender carries its contract notice's, and a procedure its
            // newest notice cancelled read `open` until that date passed.
            ("By submission deadline.", "status reads the newest notice too (issue 533)"),
            ("(by submission deadline)", "status reads the newest notice too (issue 533)"),
            // Issue 533: its three field refolds ran (refold-fields 2563, project 2564, 2026-10-16), so
            // the `dps` rows and the r2.0.9 `discontinued`/`unsuccessful` reasons are complete. The
            // schema notes' and the OpenAPI classifications' pending caveats, retired.
            ("until a fields refold reaches the rest", "the issue-533 field refolds ran, jobs 2563 and 2564 (issue 533)"),
            ("once its field refolds have run", "the issue-533 field refolds ran, jobs 2563 and 2564 (issue 533)"),
            // Issue 539: served from issue 18 on. A DÖE published_at was the buyer's requested (or a
            // stamped) date, and on about 16,344 notices (counted 2026-10-19) the IssueDate fell on a
            // later day; DÖE's dispatch
            // falls on or before published_at's day since the issue-539 rule.
            ("for publication — earlier than published_at. Same", "a DÖE dispatch can fall on published_at's day (issue 539)"),
        ];
        // Issue 534 review: an entry is only as good as its spelling. The `/docs` form
        // of the winner sentence, wrapped the way the page wraps prose, is caught.
        let html = squash("<p>So a buyer is <code>buyer</code>, and a winner is <code>winner</code>\n    or <code>Tenderer</code>.</p>");
        assert!(
            RETIRED.iter().any(|(retired, _)| html.contains(retired)),
            "no RETIRED entry matches the /docs spelling of the winner sentence: {html}"
        );
        // Issue 535 unit 2: the `/v1/sql/schema` notes are a served surface too; the
        // wrong subtype note lived there, where this walk did not look.
        let schema = crate::v1::sql::served_schema_notes();
        for (surface, text) in [("/docs", PAGE), ("/v1/openapi.json", SPEC), ("/v1/sql/schema", schema.as_str())] {
            // Whitespace squashed: `/docs` wraps its prose mid-phrase, so a retired
            // sentence broken across two source lines must still be caught.
            let text = squash(text);
            for (retired, why) in RETIRED {
                assert!(
                    !text.contains(retired),
                    "{surface} still serves the retired claim {retired:?} — {why}"
                );
            }
        }
    }

    /// The other half of the guard above: the corrections themselves are load
    /// bearing, so a rewrite that simply deletes the sentence must fail too.
    #[test]
    fn the_docs_say_what_provisional_and_a_408_actually_mean() {
        assert!(
            PAGE.contains("NAME-scoped"),
            "/docs must say what identity a provisional row has, not just that it lacks an identifier"
        );
        // Issue 425: the work is now interrupted at the limit, so the old caveat
        // ("the engine offers no interrupt") would be false on both surfaces.
        for (surface, text) in [("/docs", PAGE), ("/v1/openapi.json", SPEC)] {
            assert!(!text.contains("no interrupt"), "{surface} must not claim the engine cannot stop a query");
            assert!(text.contains("issue 425"), "{surface}'s 408 must say the query is stopped at the limit");
        }
    }

    /// Issue 534, the positive half of the retired "a winner is `winner` or
    /// `Tenderer`" and the pre-393 caveat: both role texts say a `Tenderer` is any
    /// bidder, send winner questions to the award decisions, publish what the eForms
    /// review roles mean, and keep the "not yet folded" hedge. Read from the
    /// `parties[]` description and the `tender_version_parties.role` note, not the
    /// whole surface, so a mention elsewhere cannot satisfy it.
    #[test]
    fn the_role_docs_send_winner_questions_to_the_award_decisions() {
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let parties = spec["components"]["schemas"]["TenderDetail"]["allOf"][1]["properties"]["parties"]
            ["description"]
            .as_str()
            .expect("TenderDetail.parties is described")
            .to_owned();
        let note = crate::v1::sql::column_note("tender_version_parties", "role")
            .expect("tender_version_parties.role has a /v1/sql/schema note")
            .to_owned();
        for (surface, text, winners) in [
            ("TenderDetail.parties", parties.as_str(), "ask lot_results[].winners, or ?winner= on /v1/tenders"),
            (
                "/v1/sql/schema tender_version_parties.role",
                note.as_str(),
                "ask tender_version_result_winners (v_lot_results, v_awards), as ?winner= does",
            ),
        ] {
            assert_says(
                surface,
                text,
                &[
                    "is a bidder, won or not (a losing bidder keeps the role)",
                    "so do not read who won from this",
                    winners,
                    "(where to get information on appeals) means",
                    // Kept until a served-role detector exists (issue 534 unit 5 is a
                    // test over the inventory, not a census of the served rows).
                    "a source element name not yet folded",
                ],
            );
        }

        // Issue 534 review: the legacy role list on both surfaces IS the fold's
        // vocabulary, read from the constant the ingest walk holds the fold to. A role
        // added there without a docs line, or a docs line naming a role nothing folds
        // onto, fails here.
        let listed = |surface: &str, text: &str, quote: char| -> Vec<String> {
            let start = text
                .find("fold onto canonical roles:")
                .unwrap_or_else(|| panic!("{surface} must introduce the legacy role list"));
            let rest = &text[start..];
            let end = rest
                .find("eForms is served")
                .unwrap_or_else(|| panic!("{surface}'s legacy role list must end before the eForms half"));
            rest[..end].split(quote).skip(1).step_by(2).map(str::to_owned).collect()
        };
        let expected: std::collections::BTreeSet<&str> =
            ingest::project::LEGACY_PARTY_ROLES.iter().copied().collect();
        for (surface, text, quote) in [
            ("TenderDetail.parties", parties.as_str(), '`'),
            ("/v1/sql/schema tender_version_parties.role", note.as_str(), '\''),
        ] {
            let roles = listed(surface, text, quote);
            let served: std::collections::BTreeSet<&str> = roles.iter().map(String::as_str).collect();
            assert_eq!(roles.len(), served.len(), "{surface} lists a legacy role twice: {roles:?}");
            assert_eq!(
                served, expected,
                "{surface}'s legacy role list must be exactly ingest::project::LEGACY_PARTY_ROLES"
            );
        }
    }

    /// Whitespace runs collapsed to one space, so a phrase matches however the
    /// source wraps it.
    fn squash(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// A served text reduced to its words for phrase pins: `<code>` tags and
    /// backticks dropped (the `/docs` and OpenAPI spellings of one sentence then
    /// agree), whitespace squashed, lowercased (the SQL notes write NULL).
    fn plain(text: &str) -> String {
        squash(&text.replace("<code>", "").replace("</code>", "").replace('`', "")).to_lowercase()
    }

    /// Asserts each phrase is in `text`. Each phrase is one only its own sentence
    /// contains, so deleting any one corrected sentence fails here.
    fn assert_says(surface: &str, text: &str, phrases: &[&str]) {
        let text = plain(text);
        for phrase in phrases {
            assert!(text.contains(phrase), "{surface} must say {phrase:?}:\n{text}");
        }
    }

    /// Issue 530 Build 2: the `identifier` lookups bullet carries the decision's sentence on v4 platform
    /// keys — what a v4 UUID is, aggregating per-notice rows by name, and the twin rule's drop.
    #[test]
    fn the_docs_say_a_v4_uuid_is_a_platform_key_and_a_twin_drops_it() {
        let start = PAGE.find("<li><code>identifier</code> matches").expect("/docs has the identifier bullet");
        let bullet = &PAGE[start..start + PAGE[start..].find("</li>").expect("bullet end")];
        assert_says(
            "/docs identifier bullet",
            bullet,
            &[
                "a v4 uuid in identifier (identifier_kind: \"national\") is a submission platform's record key, not a register number.",
                "some german and swiss platforms mint a fresh one per notice",
                "aggregate such rows by name.",
                "where one notice names a body twice under two such keys, the keys are dropped and its mentions join the body's name-keyed organization (issue 530).",
            ],
        );
    }

    /// Issue 531, the positive half of the retired "a superset of winner": both
    /// surfaces say which sources serve a win under `winner` only, and that a
    /// fuller participation history takes both filters. Read from the `bidder`
    /// row and parameter, not the whole surface, so a mention elsewhere cannot
    /// satisfy it. One phrase per sentence of the row.
    #[test]
    fn the_docs_say_bidder_misses_the_wins_of_sources_without_bids() {
        let start = PAGE.find("<td class=\"ep\">bidder</td>").expect("/docs has a bidder filter row");
        let docs = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["parameters"]["bidder"]["description"]
            .as_str()
            .expect("the bidder parameter is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            assert_says(
                &format!("{surface}'s bidder text"),
                text,
                &[
                    "(ted eforms, eforms-de), won or not",
                    "on fts, an award supplier only: fts publishes no losing bidders",
                    "the pre-eforms ted formats (text, internal-ojs, r2.0.8, r2.0.9) and döe sdk-0.1 \
                     publish winners without bids, so a win there appears under winner only",
                    "to find every tender where an organization is named as a tenderer or a winner, query both; \
                     on the formats without bids, an award to the tender's own buyer is in neither",
                ],
            );
        }
        // The lookup example under "Lookups by real-world key" asks both filters.
        let start = PAGE.find("# VAT → canonical org").expect("/docs has the identifier lookup example");
        let example = &PAGE[start..start + PAGE[start..].find("</code></pre>").expect("example end")];
        for ask in ["/v1/tenders?winner=2", "/v1/tenders?bidder=2"] {
            assert!(example.contains(ask), "the /docs lookup example must ask {ask:?}:\n{example}");
        }
    }

    /// Issue 535 unit 2, the positive half of the retired "eForms notice subtype
    /// id" and "0 is normal for a single-lot procurement": every surface says who
    /// fills `notice_subtype` (FTS too) and who leaves it NULL (sdk-0.1) and where
    /// those eras' own type is, the per-format `lots` rule, and what a NULL
    /// `original_lang` means.
    ///
    /// Read from each field's own text — the OpenAPI field, the `/v1/sql/schema`
    /// note, and on `/docs` the one coverage bullet that names `notice_subtype` —
    /// because "FTS" and "sdk-0.1" already occur elsewhere on every surface. Each
    /// pin is a phrase only its own corrected sentence contains, not a token
    /// another sentence of the same text also carries, so a rewrite that deletes
    /// any one sentence fails here.
    #[test]
    fn the_docs_say_which_sources_leave_lots_subtype_and_language_empty() {
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let described = |value: &serde_json::Value, what: &str| -> String {
            value.as_str().unwrap_or_else(|| panic!("{what} is described")).to_owned()
        };
        let field = |name: &str| {
            described(&spec["components"]["schemas"]["Tender"]["properties"][name]["description"], name)
        };
        let note = |column: &str| {
            crate::v1::sql::column_note("v_tenders", column)
                .unwrap_or_else(|| panic!("v_tenders.{column} has a /v1/sql/schema note"))
                .to_owned()
        };
        let start = PAGE.find("<h3>Coverage varies by era</h3>").expect("/docs has the coverage caveats");
        let section = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let docs = section
            .split("<li>")
            .find(|item| item.contains("<code>notice_subtype</code>"))
            .expect("/docs's coverage caveats carry a notice_subtype bullet")
            .to_owned();

        for (surface, text) in [
            ("/docs", docs.clone()),
            ("Tender.notice_subtype", field("notice_subtype")),
            ("/v1/sql/schema notice_subtype", note("notice_subtype")),
        ] {
            assert_says(
                surface,
                &text,
                &[
                    // Who fills it, FTS included, and FTS's own NULL.
                    "on fts, the uk form code, else the release tags joined with",
                    "null when the release has neither",
                    // Who leaves it NULL.
                    "null for the pre-eforms ted formats (text, internal-ojs, r2.0.8, r2.0.9) and döe \
                     sdk-0.1, which publish no subtype",
                    "kept out of this column (issue 235)",
                    // Where their own type is: these ids occur in this sentence only.
                    "notice_codes",
                    "ted-td_document_type",
                    "txt-td",
                    "ted-nat_notice",
                    "sdk01-noticetypecode",
                ],
            );
        }

        for (surface, text) in [("/docs", docs.clone()), ("Tender.lots", field("lots"))] {
            assert_says(
                &format!("{surface}'s lots rule"),
                &text,
                &[
                    "r2.0.9 (its object description is a lot) and on eforms",
                    "on the text era (no lot structure)",
                    "on r2.0.8 unless a contract notice published a lot annex",
                    "fts makes a lot only from a release's tender.lots, so an fts tender serves 0 until one \
                     of its releases lists a lot",
                    "only an r2.0.9 f14 corrigendum",
                    "prior-information notice with no lot or part",
                    "no row on /v1/lots",
                    "on /v1/tenders",
                ],
            );
        }
        let lots_list = described(&spec["paths"]["/v1/lots"]["get"]["description"], "/v1/lots");
        let detail_lots = described(
            &spec["components"]["schemas"]["TenderDetail"]["allOf"][1]["properties"]["lots"]["description"],
            "TenderDetail.lots",
        );
        for (surface, text) in [("/v1/lots", lots_list), ("TenderDetail.lots", detail_lots)] {
            assert_says(
                &format!("{surface}, on which Tenders have no Lot"),
                &text,
                &[
                    "the text era",
                    "r2.0.8 without a contract-notice lot annex, since an r2.0.8 award notice carries none",
                    "a standalone r2.0.9 f14 corrigendum",
                    "an eforms prior-information notice without lots",
                    "an fts tender whose releases list no tender.lots",
                    "no row",
                ],
            );
        }

        for (surface, text) in [
            ("/docs", docs),
            ("Tender.original_lang", field("original_lang")),
            ("/v1/sql/schema original_lang", note("original_lang")),
        ] {
            let plain_text = plain(&text);
            assert!(
                plain_text.contains("null where the notice states no language")
                    || plain_text.contains("null where that notice states no language"),
                "{surface} must say what a NULL original_lang means:\n{plain_text}"
            );
            assert_says(surface, &text, &["as many döe sdk-0.1 notices do not", "when the release names none"]);
        }
        // The two surfaces that name the code's vocabulary also say a code outside
        // it passes through.
        for (surface, text) in [
            ("Tender.original_lang", field("original_lang")),
            ("/v1/sql/schema original_lang", note("original_lang")),
        ] {
            assert_says(
                surface,
                &text,
                &[
                    "uppercase iso 639-2/t",
                    "a published code the mapping does not recognise passes through uppercased",
                    "fr nl",
                ],
            );
        }
    }

    /// Issue 568: the coverage bullet's `lots` rule says where an r2.0.8 award goes (the lot
    /// its contract notice printed under the award's number) and which awards stay on the
    /// Tender: lettered lots against numbered awards, no number, a number only the award
    /// prints or two lots print, a lot added only in corrigendum prose, renumbered lots (G2:
    /// since issue 570 only a renumbering the remap cannot join, a lot kept at its position
    /// under a dropped or other number), and one number printed on all of two or more awards
    /// of a Tender whose lots print several (G1 — so not a single-award notice, nor a
    /// single-lot Tender). Read from the one coverage bullet that names `notice_subtype` (the
    /// bullet that states the `lots` rule), so a mention elsewhere cannot satisfy it; one
    /// phrase per clause.
    #[test]
    fn the_docs_say_an_r208_award_takes_the_lot_its_contract_notice_numbered() {
        let start = PAGE.find("<h3>Coverage varies by era</h3>").expect("/docs has the coverage caveats");
        let section = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let docs = section
            .split("<li>")
            .find(|item| item.contains("<code>notice_subtype</code>"))
            .expect("/docs's coverage caveats carry a notice_subtype bullet");
        assert_says(
            "/docs's lots rule",
            docs,
            &[
                "an r2.0.8 award takes the lot its contract notice printed under the same number (issue 568)",
                "it stays on the tender (lot_results[].lot null)",
                "where the lots are lettered and the awards numbered",
                "where the award prints no number",
                "where only the award prints the number or two lots print it",
                "where a lot was added only in a corrigendum's prose",
                "where an earlier notice kept a lot at its position but dropped its number or printed another",
                "where an award notice prints one number on all of its two or more awards while the tender's lots print several numbers",
            ],
        );
    }

    /// Issue 568 unit 7, as amended in unit 8 (round 2's R1 and R2 included): the same coverage
    /// bullet says which r2.0.8 awards stay on the Tender by their own text or numbering (a title
    /// naming several lots, another lot than both the printed number and the lot that number finds
    /// by its own title or description, or another lot's title, and several printed numbers), that
    /// a title never keys an award, that a notice whose awards are titled with other lots twice or
    /// more keeps every award its own title does not confirm off the lots (a confirmation names the
    /// printed number as a lot, not as a position within one; one such award keeps only itself
    /// off), that a Tender whose lots print fewer than two numbers is exempt, and the decision's
    /// residual caveat. Read from the one coverage bullet that names `notice_subtype`; one phrase
    /// per clause.
    #[test]
    fn the_docs_say_an_r208_award_naming_several_lots_stays_on_the_tender() {
        let start = PAGE.find("<h3>Coverage varies by era</h3>").expect("/docs has the coverage caveats");
        let section = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let docs = section
            .split("<li>")
            .find(|item| item.contains("<code>notice_subtype</code>"))
            .expect("/docs's coverage caveats carry a notice_subtype bullet");
        assert_says(
            "/docs's lots rule",
            docs,
            &[
                "an r2.0.8 award whose own title names several lots, names another lot than both its printed number and the lot that number finds (by that lot's own title or description), or carries another lot's title, or that prints several lot numbers, stays on the tender (issue 568)",
                "a title only ever keeps an award off a lot, never puts it on one",
                "where two or more awards of one notice are titled with another lot that way, that notice does not number its awards by the contract notice's lots",
                "every award of it not confirmed by its own title (one naming exactly the number the award prints, as a lot and not as a position within one) stays on the tender too",
                "none of this applies on a tender whose lots print fewer than two numbers",
                "an r2.0.8 award's lot is inferred from the number it prints against the contract notice's lots",
                "a small recorded residual (about 2 % of such tenders) keeps an award on another lot",
            ],
        );
        assert!(
            !plain(docs).contains("one award titled with another lot than the number it prints keeps every award"),
            "the decided rule's notice level (one contradicting award) is gone: it takes two since unit 8"
        );
    }

    /// Issue 567: the same coverage bullet says where an r2.0.9 award goes (the lot section of
    /// its own notice printing its LOT_NO, else LOT_NUMBER, exact then normalised) and which
    /// awards stay on the Tender (a list of numbers, a number two sections print, no number
    /// where the sections are not numbered by position). The read-back of the 567 refold saw
    /// all three null shapes served; before this the served reference named none of them.
    #[test]
    fn the_docs_say_an_r209_award_takes_the_lot_its_own_notice_numbered() {
        let start = PAGE.find("<h3>Coverage varies by era</h3>").expect("/docs has the coverage caveats");
        let section = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let docs = section
            .split("<li>")
            .find(|item| item.contains("<code>notice_subtype</code>"))
            .expect("/docs's coverage caveats carry a notice_subtype bullet");
        assert_says(
            "/docs's lots rule",
            docs,
            &[
                "an r2.0.9 award takes the lot section of its own notice that prints its lot_no (else lot_number)",
                "an exact match first, then the one lot whose number matches once normalised (lot 01 is 1) (issue 567)",
                "an r2.0.9 award printing a list of numbers (18,19)",
                "a number two lot sections print",
                "or no number where the sections' numbers are not their positions stays on the tender",
            ],
        );
    }

    /// Issue 570: the same coverage bullet says that a legacy notice's lot section joins the
    /// Tender lot an earlier notice printed under the same number, with its awards, on keys the
    /// first lot-publishing notice set; and which sections keep their positions: one with no
    /// number (rule step 4), every section of a notice where two name one lot (K1) or one
    /// prints a number no earlier lot prints where a lot with another number sits (K2: a range,
    /// a format variant), and every notice after an eForms notice's lots (K3). K4 (no earlier
    /// lot prints a number) needs no sentence: nothing could be joined. Read from the one
    /// coverage bullet that names `notice_subtype`; one phrase per clause.
    #[test]
    fn the_docs_say_legacy_lots_join_by_printed_number_across_notices() {
        let start = PAGE.find("<h3>Coverage varies by era</h3>").expect("/docs has the coverage caveats");
        let section = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let docs = section
            .split("<li>")
            .find(|item| item.contains("<code>notice_subtype</code>"))
            .expect("/docs's coverage caveats carry a notice_subtype bullet");
        assert_says(
            "/docs's lots rule",
            docs,
            &[
                "a pre-eforms ted notice's lot section joins the tender lot an earlier notice printed under the same number, and its awards follow (issue 570)",
                "the first notice that publishes lots keys them, and those keys never move",
                "a section with no number keeps its position",
                "every section of a notice where two of its sections name one lot",
                "where one prints a number no earlier lot prints at a position a lot with another number holds (1 against 1-4, 2. against 2)",
                "of every notice after an eforms notice published lots",
            ],
        );
    }

    /// Issue 456: a party on the detail carries two names, and both surfaces a
    /// reader consults say which is which — `organization_name` is the
    /// organization's head, `mention_name` the name this notice published — and why
    /// they can disagree (a mention binds by its identifier before its name). Read
    /// from the detail section and the `parties` description, not the whole
    /// surface, so a mention elsewhere cannot satisfy it.
    #[test]
    fn the_docs_say_which_name_a_party_serves() {
        let start = PAGE.find("<h2 id=\"detail\">").expect("/docs has a Tender-detail section");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows the detail")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["schemas"]["TenderDetail"]["allOf"][1]["properties"]["parties"]
            ["description"]
            .as_str()
            .expect("the parties array is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            for needle in ["mention_name", "organization_name", "head", "identifier"] {
                assert!(text.contains(needle), "{surface}'s parties text must name {needle:?}:\n{text}");
            }
        }
    }

    /// Issue 564: `name_prefix` seeks the served head's key, so after issue 524's head refresh renamed
    /// an organization its former spelling no longer finds it (designed; the CHANGELOG said so, the
    /// served reference did not). The `name_prefix` row, the lookups section and the OpenAPI parameter
    /// each say it, read from that text only so a mention elsewhere cannot satisfy it.
    #[test]
    fn the_docs_say_name_prefix_no_longer_matches_a_former_spelling() {
        let row_start = PAGE.find("<td class=\"ep\">name_prefix</td>").expect("/docs has a name_prefix row");
        let row = &PAGE[row_start..row_start + PAGE[row_start..].find("</tr>").expect("the row ends")];
        let start = PAGE.find("<h2 id=\"lookups\">").expect("/docs has a lookups section");
        let lookups = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows lookups")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["parameters"]["name_prefix"]["description"]
            .as_str()
            .expect("the name_prefix parameter is described");
        let phrases = [
            "it matches the organization's served name only",
            "after a head refresh renames an organization",
            "its former spelling no longer matches it",
        ];
        for (surface, text) in [("/docs name_prefix row", row), ("/docs lookups", lookups), ("/v1/openapi.json", openapi)] {
            assert_says(surface, text, &phrases);
        }
        assert_says(
            "/docs lookups",
            lookups,
            &["an organization row with op changed on /v1/changes", "keeps the spelling its notice printed"],
        );
    }

    /// Issue 552 (issue 524 rule v12): the served head-name rule carries its line-break clause on all
    /// three surfaces — `/docs`'s Tender-detail section, the OpenAPI `Organization.name` and the
    /// `organizations.name` schema note (the CHANGELOG repeats it, unserved). One phrase per part of the
    /// clause, so dropping or rewording it on one surface fails here.
    #[test]
    fn the_docs_say_how_a_head_with_a_line_break_is_elected() {
        let start = PAGE.find("<h2 id=\"detail\">").expect("/docs has a Tender-detail section");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows the detail")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["schemas"]["Organization"]["properties"]["name"]["description"]
            .as_str()
            .expect("Organization.name is described");
        let schema = crate::v1::sql::served_schema_notes();
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi), ("/v1/sql/schema", schema.as_str())] {
            assert_says(
                &format!("{surface}'s head-name rule"),
                text,
                &[
                    "a head unfit only through a line break or a tab counts as unfit only when",
                    "inside one of its lines or are exactly those of a run of its whole lines",
                    "served as its own words on one line when every break falls mid-phrase",
                    "the line before ends in an unspaced hyphen or a joining word such as und, of or z",
                    "does not end in a legal form, a comma, a semicolon or a colon",
                    "the name on one line does not itself carry such a standalone number",
                ],
            );
        }
    }

    /// Issue 548 U2 (issue 524 rule v13): the served head-name rule says on all three surfaces that the
    /// legal-form check holds only a head its own cluster votes for twice or more, and that the
    /// consortium check lets a group's name be cut to words it already prints (the CHANGELOG repeats
    /// it, unserved).
    #[test]
    fn the_docs_say_when_the_legal_form_and_consortium_checks_hold() {
        let start = PAGE.find("<h2 id=\"detail\">").expect("/docs has a Tender-detail section");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows the detail")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["schemas"]["Organization"]["properties"]["name"]["description"]
            .as_str()
            .expect("Organization.name is described");
        let schema = crate::v1::sql::served_schema_notes();
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi), ("/v1/sql/schema", schema.as_str())] {
            assert_says(
                &format!("{surface}'s head-name rule"),
                text,
                &[
                    "swap one legal-form family for another (ag to sàrl) while the head's own cluster holds at least 2 votes",
                    "drop a consortium mark unless the new name keeps a word besides its legal forms and every such word is a word of the head",
                ],
            );
        }
    }

    /// Issue 463, the positive half of the retired "single consistent
    /// transaction": the SSE snapshot is read in pages (`sse.rs`, issue 55), so
    /// a client must apply `added` as an upsert and `removed` as an idempotent
    /// delete (a `removed` can name an entity the snapshot never sent: one that
    /// left the filter before its page was read, or a retired one, issue 164),
    /// and both surfaces say so. And every `reset` reason `sse.rs` can send is
    /// named on both — `cursor_ahead` was missing from each (review of 463).
    ///
    /// Read from the SSE text itself, not the whole surface: webhook delivery is
    /// "at-least-once" on both already, so a page-wide check passed before the
    /// sentence was fixed.
    #[test]
    fn the_docs_say_the_sse_snapshot_is_at_least_once() {
        let start = PAGE.find("<h2 id=\"sse\">").expect("/docs has an SSE section");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows SSE")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["schemas"]["EventStream"]["description"]
            .as_str()
            .expect("the EventStream schema is described");
        let reasons: Vec<&str> = include_str!("sse.rs")
            .split("reset_event(\"")
            .skip(1)
            .map(|rest| &rest[..rest.find('"').expect("a closed literal")])
            .collect();
        assert!(reasons.contains(&"cursor_ahead"), "the scan of sse.rs found its resets: {reasons:?}");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            assert!(text.contains("at-least-once"), "{surface} must say the SSE snapshot is at-least-once:\n{text}");
            assert!(text.contains("upsert"), "{surface} must say how a client applies a repeated `added`:\n{text}");
            // Issue 547: a value bound reads an earlier version as a superset, so a versioned
            // `changed` can name an entity the client never received.
            assert!(
                text.contains("Apply `changed` as an upsert by id") || text.contains("Apply <code>changed</code> as an upsert by id"),
                "{surface} must say a `changed` is applied as an upsert too:\n{text}"
            );
            assert!(
                text.contains("idempotent delete"),
                "{surface} must say how a client applies a `removed` for an entity it never received:\n{text}"
            );
            for reason in &reasons {
                assert!(text.contains(reason), "{surface} must name the `reset` reason {reason}:\n{text}");
            }
        }
    }

    /// ADR-0017 D4's truth-up (issue 495 unit 1): a null `version` has meant "no
    /// version moved" since the org merges (issue 286) and the sweep wrote seq-less
    /// rows; both published surfaces said "null for notices", and notices are not
    /// even an event entity. A client reading that would drop every in-place row.
    #[test]
    fn the_docs_say_what_a_null_version_means() {
        let start = PAGE.find("<h2 id=\"event\">").expect("/docs has an event-schema section");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows it")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["schemas"]["ChangeEvent"]["properties"]["version"]["description"]
            .as_str()
            .expect("ChangeEvent.version is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            assert!(!text.contains("null for notices"), "{surface} still says the old wording:\n{text}");
            for must in ["organization", "removed", "rewritten in place", "upsert"] {
                assert!(text.contains(must), "{surface} must say `{must}` about a null version:\n{text}");
            }
            // Issue 562: the fold's label correction is one of the causes.
            assert!(
                plain(text).contains("or the fold re-labelling a tender's source or kind"),
                "{surface} must name the label correction as a null-version cause:\n{text}"
            );
        }
    }

    /// Issue 536, the positive half of the retired "exact match on the number the
    /// source printed": both surfaces say which spellings a publication number is
    /// accepted in, that the widths are TED's alone, how FTS and DÖE numbers are
    /// asked, that FTS's numbers collide with TED's 6-digit ones, and where the page
    /// says which spelling it matched. Read from the `publication_id` bullet and the
    /// parameter's own description, not the whole surface. One phrase per sentence,
    /// each one only its own sentence contains, so deleting any fails here.
    #[test]
    fn the_docs_say_which_spellings_a_publication_number_answers() {
        let start = PAGE.find("<li><code>publication_id</code>").expect("/docs has a publication_id lookup bullet");
        let docs = &PAGE[start..start + PAGE[start..].find("</li>").expect("bullet end")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["parameters"]["publication_id"]["description"]
            .as_str()
            .expect("the publication_id parameter is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            assert_says(
                &format!("{surface}'s publication_id text"),
                text,
                &[
                    "takes a ted number as ted prints it, unpadded (79997-2017), at the 6 or 8 digits the archive \
                     stores (079997-2017, 00079997-2017), or in the oj s form (2017/s 044-079997)",
                    "the stored width follows the notice's format, not its year",
                    "a ted number is one notice per year whichever width you send",
                    "those widths answer ted notices only",
                    "issue 000 is no ted issue, so that form looks up the fts spelling only",
                    "uuid or number",
                    "version suffix included",
                    "fts numbers from 2021 on coincide with ted's 6-digit ones",
                    "pair with source=",
                    "when the page matched a stored spelling other than the one sent",
                ],
            );
        }
        // The filter table's row says it too, not "exact match" (issue 536 review).
        let start = PAGE.find("<td class=\"ep\">publication_id</td>").expect("/docs has a publication_id filter row");
        let row = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
        assert_says(
            "/docs's publication_id filter row",
            row,
            &[
                "a ted number is found in any of its spellings",
                "on ted notices only; another source's number (fts, döe) matches as that source stores it",
            ],
        );
        // The lookups table's example is TED's own spelling of a real notice
        // (19163916, stored `079997-2017`), which the exact match missed.
        assert!(
            PAGE.contains("GET /v1/notices?publication_id=79997-2017</td>"),
            "the /docs lookups table must show TED's unpadded spelling"
        );
    }

    /// Issue 533, the positive half of the retired "By submission deadline.": both surfaces
    /// say `status` reads the newest notice beside the deadline, which notices close a Tender
    /// (and which lots they must decide), which schemes are exempt (the declared list and the
    /// admission window) and the explicit legacy no-award that closes one anyway, what reopens
    /// it and how a lot follows its Tender. Read from the `status` row and the openapi `status`
    /// parameter, not the whole surface. One phrase per sentence.
    #[test]
    fn the_docs_say_status_reads_the_newest_notice() {
        let start = PAGE.find("<td class=\"ep\">status</td>").expect("/docs has a status filter row");
        let docs = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["parameters"]["status"]["description"]
            .as_str()
            .expect("the status parameter is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            assert_says(
                &format!("{surface}'s status text"),
                text,
                &[
                    "by what the newest notice says",
                    "not closed by its newest notice",
                    "every result the procedure has published is a no-award",
                    "together they decide every lot of kind lot",
                    "lots groups and parts need no result",
                    "a tender-scoped result decides no lot, so it closes only a tender with no lot-kind lot",
                    "open-ended scheme",
                    "a dynamic purchasing system, procedure oth-single/oth-mult, a qualification-system (eforms 15) \
                     or dynamic-market",
                    "more than a year after the publication of the first notice that published a deadline",
                    "no-award closes even then",
                    "never closes a tender here",
                    "a later notice that publishes no results of its own",
                    "a lot is closed with its tender",
                ],
            );
        }
        assert_says("/docs's status row", docs, &["still read the deadline alone"]);
    }

    /// Issue 532: both surfaces say a place of performance published only as a
    /// country is served as its NUTS level-0 code, and that a place outside the NUTS
    /// countries (or under an overseas region's own code) carries none. Read from the
    /// `country` row and the openapi `country` parameter, not the whole surface. One
    /// phrase per sentence. Unit 3's refold ran (refold-fields 2521, project 2522,
    /// 2026-10-15), so the five pre-refold caveats are gone and RETIRED holds
    /// `"issue-532 refold"`.
    #[test]
    fn the_docs_say_a_country_only_place_is_served_as_its_nuts_level_0_code() {
        let start = PAGE.find("<td class=\"ep\">country</td>").expect("/docs has a country filter row");
        let docs = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let openapi = spec["components"]["parameters"]["country"]["description"]
            .as_str()
            .expect("the country parameter is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", openapi)] {
            assert_says(
                &format!("{surface}'s country text"),
                text,
                &[
                    "only as a country",
                    "nuts level-0 code",
                    "e.g. egypt or the usa, carries none",
                    "an overseas region's own iso code",
                ],
            );
        }
    }

    /// Issue 546 (D1): every served surface says `country` is the place of performance
    /// only, in every era, and where a legacy buyer's or winner's region went
    /// (`notice_classifications`, `TED-CA_CE_NUTS` / `TED-TENDERER_NUTS`). Read from the
    /// `country` row, the openapi `country` parameter, `Tender.country` and the
    /// classifications description, not the whole surface; the sql check reads the whole
    /// served schema notes, as 532's test does. The row and the parameter get one phrase
    /// per clause, read with `<strong>` / `**` stripped so "place of performance" itself is
    /// pinned. Each surface carries the pre-refold caveat, tagged `issue-546 refold`: once
    /// unit 4's refold and its Verify (8432519 serves `['FRK22']`) are done, remove all five
    /// and retire the token through RETIRED.
    #[test]
    fn the_docs_say_country_is_the_place_of_performance_only_in_every_era() {
        let start = PAGE.find("<td class=\"ep\">country</td>").expect("/docs has a country filter row");
        let docs = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let parameter = spec["components"]["parameters"]["country"]["description"]
            .as_str()
            .expect("the country parameter is described");
        for (surface, text) in [("/docs", docs), ("/v1/openapi.json", parameter)] {
            let unbolded = text.replace("<strong>", "").replace("</strong>", "").replace("**", "");
            assert_says(
                &format!("{surface}'s country text"),
                &unbolded,
                &[
                    "the places are the place of performance only, in every era (issue 546)",
                    "and ?country= does not match them",
                    "notice_classifications under field ids ted-ca_ce_nuts (buyer) and ted-tenderer_nuts (winner)",
                    "serves none: no fallback to an address",
                    "keeps the place an earlier one published",
                    "until the issue-546 refold has run",
                ],
            );
        }
        let tender_country = spec["components"]["schemas"]["Tender"]["properties"]["country"]["description"]
            .as_str()
            .expect("Tender.country is described");
        let classifications = spec["components"]["schemas"]["TenderDetail"]["allOf"][1]["properties"]["classifications"]
            ["description"]
            .as_str()
            .expect("TenderDetail.classifications is described");
        let schema = crate::v1::sql::served_schema_notes();
        for (surface, text) in [
            ("Tender.country", tender_country),
            ("TenderDetail.classifications", classifications),
            ("/v1/sql/schema", schema.as_str()),
        ] {
            assert_says(surface, text, &["the place of performance only", "ted-ca_ce_nuts", "issue-546 refold"]);
        }
    }

    /// Issue 538 unit 3: both surfaces say which texts are decoded of a publisher's extra
    /// escaping, how far, what becomes of a line break, and what is left as parsed. Read from
    /// the `/docs` Texts caveats and the OpenAPI `Tender.title` / `TenderDetail.texts`
    /// descriptions, not the whole surface. One phrase per sentence. The pre-refold sentence
    /// carries `issue-538 refold`: once the drain has run, retire it through RETIRED.
    #[test]
    fn the_docs_say_titles_and_descriptions_are_decoded_of_extra_escaping() {
        let start = PAGE.find("<h3>Texts</h3>").expect("/docs has the text caveats");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let title = spec["components"]["schemas"]["Tender"]["properties"]["title"]["description"]
            .as_str()
            .expect("Tender.title is described");
        let texts = spec["components"]["schemas"]["TenderDetail"]["allOf"][1]["properties"]["texts"]["description"]
            .as_str()
            .expect("TenderDetail.texts is described");
        assert_says(
            "/docs's text caveat",
            docs,
            &[
                "titles and descriptions are served with a publisher&rsquo;s extra escaping decoded",
                "up to three levels deep",
                "becomes a line break",
                "nothing else changes",
                "the notice keeps the text as parsed",
                "organization names are not decoded",
                "until the issue-538 refold reaches it",
            ],
        );
        assert_says(
            "Tender.title",
            title,
            &[
                "decoded of a publisher's extra escaping (issue 538)",
                "up to three levels deep",
                "as a line break",
                "nothing else is changed",
                "the notice content keeps the text as parsed",
                "until the issue-538 refold reaches it",
            ],
        );
        assert_says(
            "TenderDetail.texts",
            texts,
            &["title and description values are decoded", "every other field is served as parsed"],
        );
        // The caveats' opening names the decode as the exception to "served as published", and
        // the Texts caveat names the three control characters it does decode.
        let intro = PAGE.find("<h2 id=\"caveats\">").expect("/docs has the caveats");
        let intro = &PAGE[intro..intro + PAGE[intro..].find("<h3>").expect("a section follows it")];
        assert_says("/docs's caveats opening", intro, &["except where a caveat below names a decode"]);
        assert_says("/docs's text caveat", docs, &["control character other than tab, line feed and carriage return"]);
    }

    /// Issue 539: what a DÖE `published_at` is, and why a DÖE `dispatched_at` is not always
    /// earlier than it — on the `/docs` Dates caveats and the OpenAPI `Tender.published_at`,
    /// `Tender.dispatched_at` and `Notice.published_at` descriptions, not the whole surface.
    /// One phrase per sentence. The pre-drain sentences carry `issue-539 drain`: once the drain
    /// (repair-notice-instants, then the ADR-0017 R3 `refold` of the DÖE profiles that re-date
    /// and its trailing project) has run, retire them through RETIRED. `repair-version-instants`
    /// is not part of that drain: it writes no change rows and re-sorts no chain.
    #[test]
    fn the_docs_say_a_doe_publication_date_is_raised_to_its_issue_day() {
        let start = PAGE.find("<h3>Dates</h3>").expect("/docs has the date caveats");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let described = |schema: &str, field: &str| {
            spec["components"]["schemas"][schema]["properties"][field]["description"]
                .as_str()
                .unwrap_or_else(|| panic!("{schema}.{field} is described"))
                .to_owned()
        };
        const DOE: &str = "a döe published_at is the publication date the buyer requested (or one the portal \
                           stamped), raised to the day of the notice's issuedate when that is later (issue 539)";
        // An sdk-1.x notice states no publication date, so the rule does not reach it (26100269
        // serves its timed IssueDate); every surface that states the rule says so.
        const SDK1X: [&str; 2] =
            ["döe sdk-1.x notice states neither", "its published_at is its issuedate itself, time included"];
        assert_says(
            "/docs's date caveat",
            docs,
            &[
                DOE,
                "a date only when that day wins",
                SDK1X[0],
                SDK1X[1],
                "a change notice can repeat its first version's request",
                "usually runs 1&ndash;3 days before the day of the portal's daily export that carries the notice",
                "a döe dispatch falls on or before the day of published_at rather than strictly before it",
                "until the issue-539 drain re-dates the notices already stored, a döe notice dated before the \
                 deploy serves the requested (or stamped) date alone",
                "on about 16,344 of them (counted 2026-10-19)",
                "a tender refolded since the deploy already serves the new date on its versions",
            ],
        );
        // `plain` drops backticks, so the `ted` label is pinned bare.
        assert_says(
            "Tender.dispatched_at",
            &described("Tender", "dispatched_at"),
            &[
                "earlier than published_at, except where a döe notice caused the current version (a tender \
                 labelled ted included",
                "where it falls on or before the day of published_at",
                DOE,
                SDK1X[0],
                SDK1X[1],
                "until the issue-539 drain re-dates the notices already stored, a döe notice's dispatch can \
                 fall days after its published_at",
            ],
        );
        for (surface, text) in
            [("Tender.published_at", described("Tender", "published_at")), ("Notice.published_at", described("Notice", "published_at"))]
        {
            assert_says(
                surface,
                &text,
                &[
                    DOE,
                    "a date only when that day wins",
                    SDK1X[0],
                    SDK1X[1],
                    "usually runs 1–3 days before the day of the portal's daily export that carries the notice",
                    "until the issue-539 drain re-dates the notices already stored",
                ],
            );
        }
        assert_says(
            "Tender.published_at",
            &described("Tender", "published_at"),
            &["a row whose current version a döe notice caused (a ted-labelled tender included) can serve"],
        );
    }

    /// Issue 537 unit 3 (review): the `/docs` value caveat names the lot-sum rule beside the
    /// partner rules, and no longer says every figure without a partner is served. Read from
    /// the Amounts caveats, not the whole surface. One phrase per sentence.
    #[test]
    fn the_docs_say_a_total_ten_to_the_k_times_its_lot_sum_is_refused() {
        let start = PAGE.find("<h3>Amounts</h3>").expect("/docs has the amount caveats");
        let docs = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        assert_says(
            "/docs's amount caveat",
            docs,
            &[
                "1,000,000&times; a sum of the notice's own lots",
                "only when another procedure-level figure of the tender sits at the lots' scale",
                "a figure with neither such a partner nor such a lot sum is served as published",
            ],
        );
    }

    /// Issue 551 (unit 3's review): every surface says a head can be a DERIVED lot sum, and how
    /// to tell — the `/docs` Values paragraph and Amounts caveat (path V, the estimate-only head
    /// that still serves one lot's figure, the bounds), the OpenAPI `value` and
    /// `value_derivation`, and the `/v1/sql` notes on the `tenders` marker columns. Unit 2 required
    /// the two `/docs` lines (a derived head; the estimate-only head). Unit 4a made every one of
    /// them say path W (`lot-sum-awards`) was withdrawn; unit 4b's B4 says it was re-enabled with
    /// its guards (R1–R10), that every awards sum, at any size, needs a total the buyer stated
    /// within 1 % of it, and that `vouched_by` names that total — keeping the read that withdrew
    /// it on `/docs`. B5's stop rule (2026-10-19) switched path W off again; issue 551 B6 serves it
    /// again with two guards on the vouching total (G-single, G-cover): every surface checked here
    /// says `lot-sum-awards` is served. `/docs` (the caveat, the value-bound row, the SQL Values
    /// paragraph), OpenAPI `min_value` / `value` / `value_derivation` / `method` and the
    /// `current_value_derivation` note name the switch-off and B6; `vouched_by` and the detail note
    /// name B6's guarded vouching total; `/docs` and OpenAPI state both guards and what the reads
    /// found. No surface checked here may still say it is switched off, served only while switched
    /// on, or withdrawn (unit 4a's wording). One phrase per sentence, lowercase (`plain`).
    #[test]
    fn the_docs_say_a_head_can_be_a_derived_lot_sum() {
        let start = PAGE.find("<h3>Amounts</h3>").expect("/docs has the amount caveats");
        let caveats = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h3>").expect("a section follows it")];
        assert_says(
            "/docs's amount caveat",
            caveats,
            &[
                "except where value_derivation says it is a lot sum (below)",
                "lot-sum-vouched, when the refused figure is exactly 10<sup>k</sup> times that sum",
                "when that sum is above the figure the election would serve without it",
                "no amount row is invented",
                "a second case, lot-sum-awards, adds up each lot's latest award on a tender that publishes no procedure-level figure at all",
                "only where a total the buyer itself stated vouches for the sum",
                "states a total in the sum's currency <strong>within 1&nbsp;% of it</strong>, at any size",
                "value_derivation.vouched_by names it (field, notice_id, cents)",
                "the sum was wrong on 9 of a uniform 50 of the 15,352 tenders it had changed and on 96 of the 229",
                "it was <strong>re-enabled with its guards</strong> (issue 551 unit 4b, 18 october 2026)",
                // G1' and G2 as the code has them: a small repeated pair is summed (5995030's PLN 8,000.00).
                "no figure repeated on two or more lots at a tenth of the largest lot or more, nor on four or more lots and half of them",
                "ten refusal families (r1&ndash;r10)",
                "and, r10, when no stated total vouches for the sum",
                "29 of a fresh uniform 30 of these sums read right against their notices",
                "it was <strong>switched off again</strong> on 19 october 2026, when its rollout changed about twice as many tenders as projected",
                "59 of 61 of the sums it had served read right, one neutral and one wrong, vouched by a single-lot award notice's own total that the sum did not include",
                "it is <strong>served again with two guards on the vouching total</strong> (issue 551 b6)",
                "a total from a notice whose round has exactly one result does not vouch",
                "an award notice's total, or either end of a val_range_total, vouches only if that notice has a result on at least one of the summed lots (a result that closes the lot or awards nothing counts)",
                "the two refuse 35 of the 3,502 sums served before (of 31 of those read, 4 were wrong)",
                "a fresh sample of 30 of the sums they keep all read right",
                "no procedure figure and no vouching total serves one figure (an award's copy, or null)",
                "still serves one lot's figure, never their sum",
                "the elected figure's derived eur value or, where value_derivation is set, the lot sum's (above)",
                "has <em>no known value</em> unless its lot awards are summed (lot-sum-awards, above, where a stated total vouches for the sum)",
                "otherwise its value is null and neither bound returns it (issue 366)",
            ],
        );
        // Unit 4a's withdrawal text, unit 3's unvouched wording and B5's stop-rule switch-off
        // wording are gone.
        for gone in [
            "is <strong>withdrawn</strong>",
            "refolded back to one figure (an award's copy, or null) until a redesign",
            "no longer summed",
            "lot-sum-awards, when the tender publishes",
            "unless its lots' awards are summed",
            "no figure repeated across the lots",
            "it is <strong>re-enabled",
            "it is <strong>switched off again</strong> since",
            "pending an investigation",
            "nothing has been read wrong yet",
            "is not served while path w is off",
            "not summed instead while lot-sum-awards is switched off",
        ] {
            assert!(!plain(caveats).contains(gone), "/docs's amount caveat still says an earlier path W: {gone:?}");
        }
        // The filter table's value-bound row, and the OpenAPI parameter (unit 3's third review: both
        // still said a Tender with no convertible amount never matches, which path W broke; unit 4a
        // withdrew path W, so it never matched again; unit 4b's B5 switched it back on, vouched, B5's
        // stop rule switched it off again on 2026-10-19, and issue 551 B6 serves it again, guarded).
        let start = PAGE.find("<td class=\"ep\"><code>min_value</code> / <code>max_value</code></td>").expect("/docs has the value-bound row");
        let row = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
        assert_says(
            "/docs's min_value/max_value row",
            row,
            &[
                "where value_derivation is set, the sum of its lots converted the same way",
                "a tender with no convertible amount matches no value bound unless its lot awards are summed (lot-sum-awards, only where a total the buyer stated vouches for the sum, past the two guards issue 551 b6 put on that total",
                "then the bound compares that sum.",
            ],
        );
        // B5's switch-off and unit 4a's withdrawal (the B5 on-state test's guard) are gone from the row.
        for gone in ["switched off", "withdrawn"] {
            assert!(!plain(row).contains(gone), "/docs's min_value/max_value row still says path W is off ({gone:?})");
        }
        assert_says(
            "/docs's served-values bullet",
            caveats,
            &["is a sum of the lots' published figures, added up in their own currency, not a conversion"],
        );
        let start = PAGE.find("<strong>Values.</strong>").expect("/docs has the SQL Values paragraph");
        let values = &PAGE[start..start + PAGE[start..].find("</p>").expect("paragraph end")];
        assert_says(
            "/docs's SQL Values paragraph",
            values,
            &["where it is a derived lot sum (issue 551: 'lot-sum-vouched', or 'lot-sum-awards' with the stated total that vouches for it in the detail's vouched_by, served again since issue 551 b6 after its switch-off on 19 october 2026), tenders.current_value_derivation says so"],
        );
        // B5's switch-off clause sat in this paragraph, outside the Amounts caveat's slice.
        for gone in ["written only while lot-sum-awards is switched on", "off since 19 october 2026"] {
            assert!(!plain(values).contains(gone), "/docs's SQL Values paragraph still says path W is switched off: {gone:?}");
        }
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        let min_value = spec["components"]["parameters"]["min_value"]["description"].as_str().expect("min_value is described");
        assert_says(
            "OpenAPI min_value",
            min_value,
            &[
                "where value_derivation is set, the sum of its lots converted the same way",
                "rows with no convertible amount match only when their lot awards are summed (lot-sum-awards, where a total the buyer stated vouches for the sum: issue 551 unit 4b; switched off on 2026-10-19 and served again since issue 551 b6, with two guards on that total)",
            ],
        );
        let tender = &spec["components"]["schemas"]["Tender"]["properties"];
        let described = |name: &str| tender[name]["description"].as_str().unwrap_or_else(|| panic!("Tender.{name} is described")).to_owned();
        assert_says(
            "OpenAPI Tender.value",
            &described("value"),
            &[
                "it can instead be a derived lot sum, in the lots' own currency, when value_derivation is set",
                "or the lots' latest awards added up on a tender with no procedure figure, served only where a total the buyer stated, within 1% of the sum, vouches for it (lot-sum-awards, issue 551 unit 4b",
                "switched off on 2026-10-19 and served again since issue 551 b6, with two guards on that total)",
                "a multi-lot tender with no procedure figure and no such total serves one figure, an award's copy or null.",
                "still serves one lot's figure, never their sum",
            ],
        );
        assert_says(
            "OpenAPI Tender.value_derivation",
            &described("value_derivation"),
            &[
                "null when value is a published figure",
                "method is lot-sum-vouched",
                "or lot-sum-awards (each lot's latest award added up, on a tender with no procedure figure at all",
                "re-enabled with its guards r1-r10 (issue 551 unit 4b, 2026-10-18)",
                "no award 10x its lot's estimate, 10x or a tenth of its earlier award, or 10x the next largest award",
                "every awards sum, at any size, needs a total the buyer stated within 1% of it, which vouched_by names",
                "it was switched off again on 2026-10-19 (issue 551, b5's stop rule",
                "and is served again since issue 551 b6 with two guards on the total that vouches",
                "a total from a notice whose round in the version has exactly one result does not vouch (g-single",
                "it also removes a one-result notice's total when that one lot is summed)",
                "an award notice's total or either end of a ted val_range_total vouches only if its notice has a result on at least one summed lot, even one that closes the lot or awards nothing (g-cover)",
                "no amount row is invented",
            ],
        );
        let method = tender["value_derivation"]["properties"]["method"]["description"].as_str().expect("value_derivation.method is described");
        assert_says(
            "OpenAPI Tender.value_derivation.method",
            method,
            &["both are served", "lot-sum-awards since issue 551 unit 4b, switched off on 2026-10-19 and served again since issue 551 b6, always with vouched_by"],
        );
        let vouched_by = &tender["value_derivation"]["properties"]["vouched_by"];
        assert_says(
            "OpenAPI Tender.value_derivation.vouched_by",
            vouched_by["description"].as_str().expect("value_derivation.vouched_by is described"),
            &["lot-sum-awards only", "the buyer-stated total that vouches for the sum", "within 1% of it", "since issue 551 b6, the first such total, in chain order, that both of its guards keep"],
        );
        for key in ["field", "notice_id", "cents"] {
            assert!(vouched_by["properties"][key].is_object(), "value_derivation.vouched_by.{key} is declared");
        }
        for (surface, text) in [
            ("OpenAPI min_value", min_value.to_owned()),
            ("OpenAPI Tender.value", described("value")),
            ("OpenAPI Tender.value_derivation", described("value_derivation")),
            ("OpenAPI Tender.value_derivation.method", method.to_owned()),
            ("OpenAPI Tender.value_derivation.vouched_by", vouched_by["description"].as_str().expect("value_derivation.vouched_by is described").to_owned()),
        ] {
            // A bare "withdrawn" would match value_derivation's history of unit 4a, which stays.
            assert!(!plain(&text).contains("withdrawn (issue 551 unit 4a)"), "{surface} still says unit 4a withdrew path W: {text}");
            for gone in [
                "is not served while",
                "not currently served",
                "switched off since",
                "off since 2026-10-19",
                "pending an investigation",
                "under investigation",
                "only while lot-sum-awards is switched on",
            ] {
                assert!(!plain(&text).contains(gone), "{surface} still says path W is switched off ({gone:?}): {text}");
            }
        }
        let note = |column: &str| crate::v1::sql::column_note("tenders", column).unwrap_or_else(|| panic!("tenders.{column} is described"));
        assert_says("/v1/sql tenders.current_value_eur_cents", note("current_value_eur_cents"), &["it can be a derived lot sum's eur value"]);
        assert_says(
            "/v1/sql tenders.current_value_derivation",
            note("current_value_derivation"),
            &[
                "null when the head value is one published figure",
                "'lot-sum-vouched' when",
                "'lot-sum-awards' when it is the lots' latest awards added up on a head with no procedure figure, vouched by a total the buyer stated within 1% of the sum (issue 551 unit 4b",
                "b5's stop rule switched it off again on 2026-10-19, and issue 551 b6 writes it again with two guards on the vouching total",
            ],
        );
        // "no fold writes" covers unit 4a's "no fold writes it" and B5's "no fold writes
        // 'lot-sum-awards' now"; a bare "withdrawn" would match the note's unit 4a history.
        for gone in ["no fold writes", "switched off again since 2026-10-19", "written only while lot-sum-awards is switched on"] {
            assert!(!plain(note("current_value_derivation")).contains(gone), "the /v1/sql note still says path W is switched off: {gone:?}");
        }
        assert_says("/v1/sql tenders.current_value_cents", note("current_value_cents"), &["a derived head's lot sum in minor units"]);
        assert_says("/v1/sql tenders.current_value_currency", note("current_value_currency"), &["the currency of current_value_cents"]);
        assert_says(
            "/v1/sql tenders.current_value_derivation_detail",
            note("current_value_derivation_detail"),
            &["served as value_derivation", "vouched_by {field, notice_id, cents} for 'lot-sum-awards'", "since issue 551 b6 the first such total both of its guards keep"],
        );
        for gone in ["written only while lot-sum-awards is switched on", "off since 2026-10-19"] {
            assert!(!plain(note("current_value_derivation_detail")).contains(gone), "the /v1/sql detail note still says path W is switched off: {gone:?}");
        }
    }

    /// Issue 547 re-review: a date bound implies its sort on a page only, and every text that
    /// says "implies" says so (the four OpenAPI bounds, the two `/docs` table rows, the
    /// both-bounds sentence); the ordering section states the one stream rule; and the Live feed
    /// says, on both surfaces, that a `changed` can name an unreceived entity under
    /// `source`/`kind` too. (Its "a resumed `now` stream is not exact" caveat became issue 563's
    /// refusal, pinned in [`the_docs_say_a_stream_refuses_a_now_bound`].) Issue 560: a Tender's
    /// value move is now a lot's change row, so its exception is gone from both surfaces; in its
    /// place they name the Tender-scope lots filters' (issue 565) and the superset a value bound
    /// reads at a version superseded within the same fold or by a lagging or resumed stream
    /// (issue 561), and say a lot `changed` can arrive when only its Tender's value or closure
    /// moved. Issue 562: a label move is announced by the fold's `version: null` correction, so
    /// its exception is gone too; the Diff says why the new label's subscriber hears `changed`
    /// (or `added`, for an entity the version before did not hold) and the old label's
    /// `removed`.
    #[test]
    fn the_docs_say_a_date_bound_implies_a_sort_on_pages_only() {
        const PAGES_ONLY: &str = "(pages only; a stream snapshots in id order)";
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        for name in ["published_after", "published_before", "deadline_after", "deadline_before"] {
            let text = spec["components"]["parameters"][name]["description"].as_str().expect("the bound is described");
            assert!(text.contains(PAGES_ONLY), "openapi's {name} must qualify its implied sort:\n{text}");
        }
        for row in ["<td class=\"ep\"><code>published_after</code>", "<td class=\"ep\"><code>deadline_after</code>"] {
            let start = PAGE.find(row).expect("/docs has the bound's row");
            let text = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
            assert!(text.contains(PAGES_ONLY), "/docs's {row} row must qualify its implied sort:\n{text}");
        }
        let start = PAGE.find("<h2 id=\"ordering\">").expect("/docs has an ordering section");
        let ordering = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows it")];
        assert_says(
            "/docs's ordering section",
            ordering,
            &[
                "else 400 (pages only; a stream snapshots in id order)",
                "sort unset or id and order unset or asc are accepted and mean that id order, with or without date bounds",
                "any other sort (published_at, deadline) or order=desc is 400",
            ],
        );
        let start = PAGE.find("<h2 id=\"sse\">").expect("/docs has an SSE section");
        let sse = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows SSE")];
        let stream = spec["components"]["schemas"]["EventStream"]["description"]
            .as_str()
            .expect("the EventStream schema is described");
        for (surface, text) in [("/docs", sse), ("/v1/openapi.json", stream)] {
            assert_says(
                &format!("{surface}'s live feed"),
                text,
                &[
                    "source or (on /v1/tenders) kind a changed can name an entity",
                    // Issue 560 retired its exception; the Tender-scope lots filters are 565's.
                    "is checked again only when a change row is written for the lot itself",
                    "(issue 565)",
                    // Issue 560 review: a version superseded within the same fold reads the superset too.
                    "a version since superseded is read as a superset on both sides, whether a later version was written in the same fold or the stream resumes, or lags, past it",
                    "so a changed or an added can name an entity outside the bound, on a live stream as well as a resumed one",
                    "a lot changed can arrive when only its tender's value or closure moved",
                    // Issue 562: a label move is announced, so it is no exception to the exact
                    // final state; the Diff says why a new-label subscriber hears `changed`.
                    "both the version's own change and the version: null correction that follows read the tender's current label on both sides",
                    "so a subscriber to the new label hears changed (or added, for an entity the version before did not hold) and one to the old label removed (issue 562)",
                ],
            );
            assert!(!plain(text).contains("re-evaluates no lot"), "{surface} still names issue 560's gap:\n{text}");
            assert!(!plain(text).contains("(issue 560)"), "{surface} still lists issue 560 as an exception:\n{text}");
            assert!(
                !plain(text).contains("reaches a subscriber to the old label as nothing"),
                "{surface} still lists issue 562 as an exception:\n{text}"
            );
        }
    }

    /// Issue 563 (decision B): an event stream refuses a date bound spelled as the literal `now`,
    /// and both surfaces say so where a client looks. Each date bound's text (the four OpenAPI
    /// parameters, the two `/docs` rows) and the ordering section's `now` sentence say `now` is
    /// pages-only; the Live feed, on `/docs` and in openapi's `EventStream`, says why (a resume
    /// would resolve it again), what to send instead, and keeps `status`'s clock documented. The
    /// retired caveat ("a resumed `now` stream is not exact") is gone from both. `status=open` is
    /// offered in place of `deadline_after=now` alone, and always with its clock caveat. The Live
    /// feed's intro and openapi's `listTenders` name the refusal among a stream's 400s.
    #[test]
    fn the_docs_say_a_stream_refuses_a_now_bound() {
        const PAGES_ONLY: &str = "now is accepted on pages only: an event stream refuses it with 400";
        let spec: serde_json::Value = serde_json::from_str(SPEC).expect("openapi.json is valid JSON");
        for name in ["published_after", "published_before", "deadline_after", "deadline_before"] {
            let text = spec["components"]["parameters"][name]["description"].as_str().expect("the bound is described");
            assert_says(
                &format!("openapi's {name}"),
                text,
                &[PAGES_ONLY, "because a resume would resolve it again and its bound would move"],
            );
            if name == "deadline_after" {
                assert_says(
                    &format!("openapi's {name}"),
                    text,
                    &["or status=open for tenders still open, which also reads the clock when the stream connects"],
                );
            } else {
                assert!(!plain(text).contains("status=open"), "openapi's {name} offers status=open:\n{text}");
            }
        }
        let list = spec["paths"]["/v1/tenders"]["get"]["description"].as_str().expect("listTenders is described");
        assert_says(
            "openapi's listTenders",
            list,
            &["any other sort or order is 400, as is a date bound spelled as the literal now"],
        );
        for row in ["<td class=\"ep\"><code>published_after</code>", "<td class=\"ep\"><code>deadline_after</code>"] {
            let start = PAGE.find(row).expect("/docs has the bound's row");
            let text = &PAGE[start..start + PAGE[start..].find("</tr>").expect("row end")];
            assert_says(&format!("/docs's {row} row"), text, &[PAGES_ONLY]);
        }
        let start = PAGE.find("<h2 id=\"ordering\">").expect("/docs has an ordering section");
        let ordering = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows it")];
        assert_says("/docs's ordering section", ordering, &[PAGES_ONLY, "because a resume would resolve it again"]);
        let start = PAGE.find("<h2 id=\"sse\">").expect("/docs has an SSE section");
        let sse = &PAGE[start..start + 4 + PAGE[start + 4..].find("<h2").expect("a section follows SSE")];
        let stream = spec["components"]["schemas"]["EventStream"]["description"]
            .as_str()
            .expect("the EventStream schema is described");
        for (surface, text) in [("/docs", sse), ("/v1/openapi.json", stream)] {
            assert_says(
                &format!("{surface}'s live feed"),
                text,
                &[
                    "/v1/tenders stream refuses a date bound spelled as the literal now with 400",
                    "a stream resolves now when its connection opens, so a resume would resolve it again",
                    "would not be sent the move",
                    "pass an absolute timestamp instead (unix seconds, rfc 3339 or a bare date); pages accept now \
                     as before (issue 563)",
                    "in place of deadline_after=now, status=open selects tenders still open, but status also reads \
                     the clock when the stream connects, and has no absolute spelling",
                    "drop a tender from a status=open state once its deadline has passed",
                    "after a resume, a change that leaves such a tender closed is not sent; a version that reopens \
                     it arrives as added",
                ],
            );
            let plain = plain(text);
            for retired in [
                "except under a date bound spelled as the literal now",
                "for an exact resume",
                "or status=open for tenders still open",
                "such a tender is sent no further change",
            ] {
                assert!(!plain.contains(retired), "{surface}'s live feed still carries the retired caveat {retired:?}");
            }
        }
        assert_says(
            "/docs's live feed intro",
            sse,
            &[
                "two exceptions answer 400 to a stream request",
                "and, on /v1/tenders, a date bound spelled as the literal now (pass an absolute timestamp",
            ],
        );
        assert!(!plain(sse).contains("the one exception is"), "/docs's live feed intro still names one exception");
    }
}
