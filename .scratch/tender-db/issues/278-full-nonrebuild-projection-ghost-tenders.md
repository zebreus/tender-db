# 278 — the full non-rebuild projection retires only `ojs:%` keys, so a regrouped island/keyed tender survives as a ghost (double-count, no `removed` event)

Status: DIAGNOSED (2026-08-26, owner — exploratory review, adversarially verified end-to-end)
Kind: correctness (canonical layer integrity + change-feed honesty)
Severity: HIGH — reachable, mints duplicates in bulk, persists until the next full rebuild
Relates to: 58 (its tracking NOTE assumes the full path is correct under a now-obsolete premise), 236 (the OPP-090 island edge that supplies the inputs), 46 (feed_generation), 164 (removal-event visibility), 103 (the sibling sweep gap — see issue 279-partial-rewrite below)
Found by: the 2026-08-26 fresh-eyes projection review; verified against project.rs / canonical.rs line-by-line.

## The bug

There are two retirement paths and they are NOT symmetric:

* **Scoped incremental** (`retire_regrouped_tenders`, canonical.rs ~4977) checks
  EVERY touched tender's group_key — island (`island:<id>`), keyed (BT-04 uuid),
  and legacy (`ojs:`) — against `plan_notice`, and retires any whose key the plan
  no longer produces.
* **Full non-rebuild** (`project_with_progress_phase2_stoppable(db, false, …)`)
  does its ONLY retirement at project.rs:1149 via
  `retire_absorbed_legacy_tenders(&legacy_keys, now)`, whose query
  (canonical.rs:4754) is literally
  `SELECT id, procedure_key FROM tenders WHERE procedure_key LIKE 'ojs:%'`.
  Island tenders (`procedure_key IS NULL`) and keyed BT-04 tenders
  (`procedure_key = <uuid>`) are never scanned.

But `build_plan` reads the WHOLE parsed corpus on this path too
(`parsed_chunk_on(&conn, 0, i64::MAX, …)`, project.rs:1249), including the
ADR-0011 previous-notice regroup pass (`plan_group_merge`) and island→keyed
upgrades — and `rebuild=false` does NOT reset the layer (`reset_tender_layer`
runs only under `if rebuild`). So when a notice regroups from `island:N` (or one
keyed uuid) into another tender's key, `tender_identity(rebuild=false)` folds it
under the surviving key and **leaves the old island/keyed tender in `tenders`
with its versions intact — no delete, no `removed` change row.**

## Why it is live now (the 58 premise expired)

Issue 58's NOTE frames the full path as correct ("a full and an incremental run
keep both, and incremental matches full") — under the explicit assumption *"no
in-place re-parse path today."* That assumption no longer holds: the DE-1.x /
r208 / r209 / quarantine reprocess campaigns all reparse in place, and a
legacy-era reparse trips the **incremental→full fallback** (project.rs:1839,
which calls the full function and `return`s — so `retire_regrouped_tenders` at
project.rs:1946 is never reached). That fallback regroups the whole corpus and
relabels islands, exactly the path with no non-legacy retirement.

## Failure scenario (verified reachable)

An eForms award notice N names a previous contract notice via OPP-090 whose BT-04
is a different key K2. On daily incrementals N stays an island (prev not in the
scoped plan — issue 236, which measures 27–39% of EU award tenders as such
islands). A later legacy reparse trips the incremental→full fallback; the full
regroup relabels `island:N`→K2 and folds N under the K2 tender.
`retire_absorbed_legacy_tenders` filters `ojs:%`, so the `island:N` tender (key
NULL) survives. Now `caused_by = N` appears under BOTH tenders: `/v1/tenders`
list and counts double-count the procedure, and no `removed` event is emitted, so
change-feed/SSE subscribers never learn the island died. The ghost persists until
the next full **rebuild** (which resets the layer). Because islands accumulate,
one full fallback can mint many ghosts at once.

## Fix direction

On the full non-rebuild path, retire ALL absorbed tenders whose group_key the
plan no longer produces — i.e. run the `retire_regrouped_tenders` logic over the
full tender set, not only `ojs:%` — or route every `rebuild=false` full
re-projection through a whole-corpus regroup-retirement pass. Must emit `removed`
change rows for the retired tenders (the 46/164 protocol) and stay byte-identity
with a from-scratch rebuild's final layer (a rebuild resets, so the invariant to
pin is: full-nonrebuild layer == rebuild layer == incremental-to-fixpoint layer).

## Acceptance

* A test that builds an island tender, regroups its notice into a keyed tender on
  the full non-rebuild path, and asserts the island tender is gone + a `removed`
  change row was emitted (red against today's code).
* The existing projection golden/equivalence gates stay green.
* Prod: after deploy, a targeted probe for duplicated `caused_by_notice_id`
  across two live tenders (bounded `/v1/sql`) to size any already-minted ghosts,
  then a scoped re-projection or the next rebuild clears them.
