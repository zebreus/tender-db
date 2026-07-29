# 79 — reprocess: skip decompressing inner archives with no held members (sparse-in-huge only)

Status: filed, DO NOT BUILD yet — only if the DE 1.x reprocess drags (team-lead will signal)
Kind: performance
Relates to: 77 (parse-only-held), 76 (reprocess mechanism)

## Observation (2026-07-29, 77 in prod)

Issue 77 (parse only held members) removed the *parse* cost from the reprocess, but the
per-package walk cost remained higher than expected — ~20s for a many-member package even
when only 2-3 members are held. Root cause: a package member is a stream-compressed
`.tar.gz`; reaching a later entry means gzip-decompressing everything before it (no random
seek within one gzip stream). So the decompression floor is `O(package bytes)` regardless of
how few members are held.

This only bites **sparse-in-huge** buckets: few held members spread across a large package.
The concrete case is the DÖE `eforms-sdk-1.0` bucket (2-3 held among ~24K members in a DÖE
monthly). Dense buckets are unaffected: the TED big buckets (1.7/1.10) hold most of what they
walk, so they parse most members anyway (77's win is real); DE 1.x (218K DÖE) is likely dense
in DÖE monthlies too (most of a DÖE monthly IS eForms-DE).

## Fix (if needed) — skip inner archives with no held members

A prod package is nested: an **uncompressed** outer `.tar` whose members are `.tar.gz` (a TED
monthly's dailies) or the DÖE monthly is a zip of per-notice XML. The outer container's entry
names/paths are cheap to list without decompressing the inner gzip. So, in a targeted reprocess:
before decompressing an inner `.tar.gz` (or a zip member), check whether ANY of its member
paths are in the held set; if none, skip it entirely — pay only the inner archives that contain
held members. This cuts a DÖE-monthly walk from "decompress the whole month" to "decompress only
the days/entries with held members".

- Needs the walker (`crates/ingest/src/package.rs`) to expose/accept a "held prefix" predicate at
  the inner-archive boundary, consulted before the inner decompress.
- Keeps byte-identity (only non-held inner archives are skipped — their members were skipped for
  parsing anyway under issue 77).
- Bounded/resumable/isolated unchanged.

## Why NOT build now

- SDK 1.0 is only ~3.4K and nearly reclaimed; the walk cost there is a one-time ~tens of minutes.
- The dense TED big buckets (1.7/1.10, ~600K) don't need it — they parse most of what they walk.
- DE 1.x (218K) is likely dense in DÖE packages too. Build ONLY if its reprocess actually drags.

Filed so it's characterized and ready; team-lead signals if the DE 1.x pace warrants it.
