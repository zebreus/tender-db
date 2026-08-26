# 278 — the full non-rebuild projection retires only `ojs:%` keys, so a regrouped island/keyed tender survives as a ghost (double-count, no `removed` event)

Status: FIX DEPLOYED (track 1) — `5357397`, prod green 2026-08-26 ~15:2x UTC. New ghosts
are now prevented on every full projection. Track 2 (clearing the ~45k EXISTING ghosts)
is the remaining work — see below; no urgency (accumulation is paused, the fix holds).
Was: CONFIRMED LIVE AT SCALE (measured on the Aug-24 snapshot).
Kind: correctness (canonical layer integrity + change-feed honesty)
Severity: HIGH — **~45,108 ghost tenders live on prod right now**, double-counting procedures on `/v1/tenders` and every count/dashboard.

## Measured on prod (snapshot `tender-db-1787598039.db`, 2026-08-26)

`plan_notice.notice_id` is a PRIMARY KEY — a notice maps to exactly ONE
group_key, so a `caused_by_notice_id` may legitimately appear under exactly ONE
tender. Reality:

* **45,108 notices** have their `caused_by_notice_id` under **2+ distinct
  tenders**, across **90,216 tender refs** — exactly 2.0 per notice, i.e. ≈45,108
  surplus (ghost) tenders. There is NO legitimate multi-procedure explanation
  (the PK rules it out).
* Sampled pairs are the SAME procedure fragmented across two tenders — verified
  by identical title AND publication_id, differing only in BT-04 uuid:
  - 260507 / 473438: "Neubau … Kindertagesstätte …", pub `00495481-2026`, keys
    `384ffd32…` vs `676b83f0…`.
  - 457837 / 1081988: Czech DPS "Dynamický nákupní systém …", pub `00532135-2026`,
    keys `63f97c14…` vs `ed3c7fd8…`, 200 vs 198 versions.
  Both shapes present: keyed+island (islands 25645486/25770088/25012236 beside
  their keyed twin) AND keyed+keyed (same procedure, two uuids). Both members of
  each pair are live-served by the API (fetched both).
* Mechanism confirmed as THIS issue's full-path gap, not the incremental path:
  the incremental `retire_regrouped_tenders` retires via
  `touched_existing_tender_ids` (`caused_by IN changed` ∪ `new_keyed_keys`), so a
  reparse of notice N (N ∈ changed) finds N's old tender and retires it. But the
  large reparse campaigns (DE-1.x/r208/r209, 218k+ notices) trip the ≥100k
  incremental→full FALLBACK (project.rs:1839), and the full path retires only
  `ojs:%` — so every keyed/island tender a reparse regrouped survived. That is
  where the ~45k came from.

Because notice→group_key is 1:1, the fix (retire every regrouped tender on the
full path) is well-defined and a from-scratch rebuild resets to zero ghosts — so
"full-nonrebuild layer == rebuild layer" is the exact byte-identity invariant.

Was: DIAGNOSED (2026-08-26, exploratory review, verified end-to-end). Originally
HIGH — reachable, mints duplicates in bulk, persists until the next full rebuild.
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

## Remediation plan (owner, 2026-08-26)

Two tracks, in this order — the code fix MUST precede the data cleanup, else a
cleanup is re-polluted the next time the full fallback fires.

1. **Code fix — DONE in the working tree (2026-08-26, awaiting gate+deploy).**
   `Db::retire_regrouped_nonlegacy_tenders` (canonical.rs) retires the two shapes
   `retire_absorbed_legacy_tenders` misses — uuid-keyed (`procedure_key NOT LIKE
   'ojs:%'`) and island — via two `NOT EXISTS` anti-joins against `plan_notice`,
   wired into the full path at project.rs:1149 right after the legacy retirement
   (before `clear_plan`, so `plan_notice` is the live produced set). The proven
   `ojs:%` path is untouched; the three passes partition the key space. Red-first
   test `a_regroup_reparse_retires_keyed_and_island_ghosts_on_the_full_path`
   (project_incremental.rs): a reparse regroups a keyed Tender to a new BT-04 and
   upgrades an island to a key; before the fix the full path's snapshot shows the
   two ghosts (notice under two Tenders) and differs from the incremental path;
   after, full == incremental, the old key/island are gone, `removed` events fired,
   and no notice maps to two Tenders. All five projection byte-identity suites
   (golden/equivalence/fold_source/incremental/resume) stay green — the fix is a
   no-op on rebuild (fresh layer) and byte-identity on full-nonrebuild.
   Original design (as built):
   On the full non-rebuild path, after `build_plan_groups`, retire every tender
   whose group_key `plan_notice` no longer produces — a corpus-wide, set-based
   anti-join (NOT the scoped per-tender loop, which would be 8.1M queries):
   - keyed: `tenders t WHERE t.procedure_key IS NOT NULL AND NOT EXISTS
     (SELECT 1 FROM plan_notice p WHERE p.group_key = t.procedure_key)`
   - island: `t.procedure_key IS NULL AND NOT EXISTS (… p.group_key =
     'island:' || t.island_notice_id)`
   - legacy `ojs:%` stays as-is (already covered) or folds into the same anti-join.
   Route through `retire_tenders_chunked` so it emits `removed` change rows and
   checkpoints. Gate: the projection golden/equivalence suites, plus a new test —
   full-nonrebuild of a corpus with a regrouped keyed+island pair leaves ONE
   tender and emits `removed` for the other; and a whole-corpus assertion that
   after a full non-rebuild pass, `COUNT(DISTINCT tender_id) per caused_by_notice_id`
   is ≤ 1 everywhere (the invariant this bug violates).

2. **Data cleanup of the ~45k existing ghosts — PENDING (track 1 deployed).**
   Now that the fix is live, ANY full projection clears them: a full
   `project(rebuild=false)` re-derives the whole plan and the new retirement pass
   sweeps the ghosts (no layer reset, no index rebuild); a full `rebuild=true`
   does it via a fresh layer AND validates the byte-identity claim end to end.
   Both are heavy (whole-corpus fold, multi-hour, /health downtime for a rebuild),
   so run in a deliberately chosen quiet window, NOT bundled with a routine deploy.
   A lighter targeted sweep (retire just the identified surplus tender per dup
   notice) is possible but needs care picking the right member of each pair; the
   full-projection route is safer and self-validating. Decision deferred to a
   dedicated firing — the ~45k is static (double-counting on `/v1/tenders`, not
   growing), so there is no rush. Re-measure the dup-notice count on the next
   snapshot before AND after to confirm the sweep landed.

No emergency mitigation is needed tonight: the accumulation is paused (the full
fallback only fires on a ≥100k reparse, none scheduled), so the ~45k is static
until the next big reparse. The fix lands as its own gated unit.
