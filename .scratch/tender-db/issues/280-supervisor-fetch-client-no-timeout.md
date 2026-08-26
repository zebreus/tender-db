# 280 — the supervisor's fetch client has no timeout, so a stalled download wedges the whole job queue with no watchdog

Status: FIXED in working tree (2026-08-26, owner), awaiting gate+deploy
Kind: operational / availability (silent hang of the ingestion queue)
Severity: HIGH
Relates to: 241 (bounded the /v1 REQUEST path's hang; this is the ingestion queue, never bounded), 252/247 (cancel plumbing — which cannot rescue this)
Found by: the 2026-08-26 supervisor review; verified end-to-end.

## The bug

`init()` built the supervisor's shared HTTP client as `reqwest::Client::new()`
(supervisor.rs:92) — NO connect, read, or total timeout, unlike the webhook
client (`Client::builder().timeout(TIMEOUT)`, app/src/webhooks.rs:150). Every
fetch/probe/rehash job downloads through `fetch::download_once`, which awaits
`req.send().await?` (fetch.rs:376) then an unbounded
`while let Some(chunk) = resp.chunk().await?` loop (fetch.rs:385).

The worker runs exactly one job at a time and blocks on `handle.await`. A source
connection that establishes and then stalls (half-open TCP, slow-loris body, an
upstream that accepts and never sends) makes `send()`/`chunk()` hang forever, so
the job never returns and the worker never pops the next one. The entire durable
queue behind it — the day's process/project fold, the weekly data-quality,
everything — freezes. `cancel()` can't help: `probe`/`fetch`/`rehash-probe` are
not in `STOPPABLE_KINDS`, and even a stoppable kind only reads the flag between
units, never mid-network-read. Nothing red fires (the job shows in-progress);
recovery is a manual restart.

## Fix (shipped in this change)

A `Supervisor::fetch_client()` built with
`connect_timeout(30s)` + `read_timeout(120s)` (reqwest 0.12.28). Deliberately NOT
a total `.timeout()` like the webhook client's 10s: a package download
legitimately streams for many minutes, and a total cap would abort a healthy
large monthly. `read_timeout` resets on each successful read, so it bounds only an
idle gap between bytes — the exact stall signature — never total transfer.
`connect_timeout` bounds the half-open-connection case. Test clients keep
`Client::new()` (no real network). No unit test asserts wall-clock timeout
behaviour (would require a stalling server); the change is a config on the client
builder, compile-checked, matching the established webhook pattern.

## Residue / possible follow-up

A coarse per-job watchdog (abort the worker JoinHandle after a kind-specific
deadline) would defend against a hang anywhere else in a job, not just the network
read. Not built here — the timeout closes the known reachable path; the watchdog
is a broader belt worth considering if any other unbounded await surfaces.
