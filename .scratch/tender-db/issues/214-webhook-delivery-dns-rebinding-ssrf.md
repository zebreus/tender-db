# 214 — webhook target is vetted only at registration; delivery re-resolves the host with no IP pin (DNS-rebinding SSRF to the Hetzner metadata endpoint)

Status: needs-triage — SECURITY / MEDIUM, CONFIRMED (code) 2026-08-15. Filed from the API review (subagent).
Note: the gap is **acknowledged in-code** as a v1 limitation (`webhooks.rs:98-106`); this issue argues the
accepted-risk rationale underweights the reachable cloud-metadata endpoint and should be revisited.
Kind: security (SSRF via the webhook delivery sweeper)
Blocked by: —
Relates to: 08 (webhooks), 178 (webhook-slot-strands), 43 (the credential surface metadata could pivot to)

## Symptom

The public-IP check (`vet_url`) runs once, at `register`. Every later delivery POSTs to the stored URL,
and `reqwest` performs its own DNS resolution with no pin to the vetted address — so an attacker who
controls DNS for their registered hostname can repoint it to an internal/link-local address after
registration (classic DNS-rebinding TOCTOU).

## Root cause

`crates/app/src/webhooks.rs`:

- `:107-131` — `vet_url` (scheme = https, host resolves, every address `is_public_ip`) is called **only**
  from `register` at `:182`.
- `:454-469` — `sign_and_send` (the sweeper) does `.post(&endpoint.url)` (`:462`) on a client built with
  `redirect(Policy::none())` (`:288`) but **no fixed resolver / IP pin**, resolving the stored host fresh
  each sweep.
- `:98-106` / `:102` — the doc comment acknowledges "this resolves at check time; between the check and
  reqwest's own [resolution] the mapping can change," on the rationale that "the box holds nothing an
  internal request could usefully reach."

`redirect(Policy::none())` closes redirect-based SSRF but not rebinding.

## Why the accepted-risk rationale should be revisited

The deployment is Hetzner (`crates/app/src/v1/health.rs:36`). `169.254.169.254` — which `is_public_ip`
correctly rejects at *vet* time — is the cloud metadata service, reachable from the box, and the
un-pinned *delivery* path can still reach it after a rebind. Metadata can expose instance identity and,
depending on configuration, credentials — a more useful internal target than the rationale assumes.

## Failure scenario

Register `https://rebind.attacker.tld/hook` while it resolves to a public IP (passes `vet_url`). Repoint
DNS to `169.254.169.254`. On the next sweep the server POSTs the signed batch there and returns the
response body / status into the delivery log the attacker owns.

## Fix

Resolve the host once at delivery, verify **all** returned addresses are public, and **pin the connection
to a vetted IP** for the actual POST — e.g. `reqwest`'s `resolve()` / a custom DNS resolver, or connect
to a pre-checked `SocketAddr` — re-validating on each delivery rather than trusting the registration-time
check. Keep `Policy::none()`. (Blocking the link-local/metadata ranges at the connector is a cheap
belt-and-braces addition.)

## Verification

- Unit/integration: a resolver that returns a public IP at register and a private IP at delivery causes
  the delivery to be refused (not POSTed).
- `vet_url`'s existing tests (`:590-596`) stay green; add a delivery-time rebinding test.
