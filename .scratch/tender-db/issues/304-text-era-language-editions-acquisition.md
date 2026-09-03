# 304 — acquire the text era's missing language editions (the un-downloaded TED zips)

Status: STAGE-1 CODE BUILT 2026-08-28 ~03:2x (inert — TranslationPolicy::EnOnly
stays the dispatch default; the campaign act is the flip + a measured one-month
re-parse). r208 needs no twin: both ted-export profiles run the same r209
module, so form_section is the single change site. The policy test pins both
directions (All keeps labelled FR texts + adopts the FR copy's extra
organisation via the 201 path; EnOnly parses with zero FR texts). Next:
stage-1 measurement month (pick 2018-08), then Lennart's breadth decision.
(Filed 2026-08-27 on Lennart's question "we were missing some TED download
for historical multi language — what do we do?"; supersedes the WRONG line in
291 gap #4 that called text-era non-EN "recoverable by re-dispatch")
Kind: acquisition campaign (staged, capacity-gated)
Relates to: 291 (language capability), ADR-0013 (supersession stays wholesale —
safe under more languages), 232 (the CO archive study is a separate question),
docs/research/ted-access-channels.md (the per-era channel map).

## The facts (ted-access-channels.md, re-read 2026-08-27)

- The text era's bulk archive ships ONE ZIP PER LANGUAGE per daily package
  (1993–1999 flat zips, initially EN-only with languages added through the
  90s; 2000–2007 `{LG}_…_ISO_ORG.ZIP`, ~38 zips/daily; 2008–2010 utf8+meta
  pairs per language, ~46 zips/daily).
- v1 deliberately fetched **EN only** (20× size saving). The other editions
  were NEVER DOWNLOADED — this is a real archive gap, unlike r208/r209 where
  translations sit inside XML we already store.
- The EN edition is often a TRANSLATION: the `OL:` field names the original
  language. So 1993–2010 currently serves translated text as its only text —
  the authenticity gap is the product cost, not just coverage breadth.
- Sizes (from the research): all-language text era ≈ +136 GB (2004–2010) /
  ~150 GB total archive; whole all-language history ≈ 188 GB compressed.
- **The "does not fit" premise is STALE**: /data measured 2026-08-27 at
  1.7 TB with 721 GB free. The archive fits with ~570 GB headroom; the open
  question is the PARSED-layer growth (multilingual text measured ~86% of
  parsed size), which must be measured, not assumed.

## Plan (staged, each stage gated)

1. **r208/r209 first, no download needed**: parser-policy change to keep all
   language variants from the stored XML → re-parse → refold (the proven
   DE-1.x/251 machinery). Measure parsed+canonical growth on ONE month before
   the corpus-wide run.
2. **Text-era acquisition**: fetch the missing language zips through the
   ordinary fetch registry, staged per year (newest text years first —
   2008–2010 have the most structure), with a disk checkpoint after each year.
   Decide breadth at stage start: all languages (+~150 GB, the flexible
   choice) vs original-language-focused subset (zips are per-language per-day,
   so a subset means choosing LANGUAGES, not notices).
3. **Flip the dispatch policy in the same stage** — today's dispatcher
   policy-skips text-era non-EN members (`text-era-non-english`, 1,571 rows),
   so newly fetched editions would land straight in the skip ledger unless the
   policy flips with the campaign.
4. **Refold rides the era-scoped machinery** after each stage's re-parse; the
   ADR-0013 fallback chain then serves the new variants automatically
   (`?lang=` already shipped, 7f89576).

Not in scope: the CO archive (232's winner-half study) — a different corpus
question. Sequencing: after the running epoch refold lands and its deploy
batch (D5 flip + backfill-values + ?lang=) is out.

## Stage-1 design (owner, 2026-08-28 — code-read of r209/parse.rs)

The parser already contains ALL the machinery; stage 1 is a policy widening
at exactly one choice point. `form_section` (r209/parse.rs ~559) sorts the
FORM_SECTION copies: ≥1 ORIGINAL (several for bilingual buyers — Belgium
FR+NL, Bolzano DE+IT) + one TRANSLATION per language. Today the primary
ORIGINAL walks fully; the other ORIGINALs and ONLY the English translation
contribute texts via the positional `translating` walk; every other
TRANSLATION copy is a translation-copy skip. The `kept_languages` gate
(~201) drops multilingual `ML_TI_DOC`/`AA_NAME` copies the same way.

Build: a `TranslationPolicy` (EnOnly | All | Langs(set)) threaded to the two
gates — `form_section`'s `english` pick becomes a filtered set of TRANSLATION
copies, `kept_languages` widens identically; default stays EnOnly so the
build deploys inert. The campaign is then: flip policy → one-month re-parse
(the 251/DE-1.x machinery) → measure parsed-layer growth (predict texts ×
~kept-language count for that month) → Lennart decides breadth for the
corpus run. The positional text-matching path is already exercised by the
bilingual-buyer fixtures; new tests: a TRANSLATION-kept fixture asserting
labelled texts land per language, and an inert-default test pinning that
EnOnly parses byte-identically to today.

r208 shares the FORM_SECTION shape (verify its parse.rs twin when building).

## Stage-1 growth measurement (2026-08-28 10:5x, 2018-08 sample, n=400)

Sampled 400 of 48,210 notices from the archived 2018-08 monthly (regex copy
census over FORM_SECTION): EnOnly keeps 410 form copies; All keeps 611 —
**1.5× copies, 1.57× form-copy bytes**. Correction to the premise: the
r208/r209 bulk packages do NOT carry all-language translations — a notice
ships its ORIGINAL(s) + an EN translation (for non-EN originals) +
occasionally one more; the full per-language renderings live in TED's
interface, not the bulk XML. So the stage-1 flip is CHEAP (+~57% legacy
form bytes ≈ a few GB parsed growth era-wide) and recovers every translation
we actually hold, but legacy language BREADTH beyond that is bounded by the
packages themselves — the wider question folds into the text-era acquisition
stages (which fetch per-language editions that DO exist as separate zips).
Numbers ready for the breadth decision; the flip itself needs no decision
gate at this cost and can ride any future re-parse campaign (e.g. 251-style)
rather than warranting its own.

## Stage 1 — THE FLIP (2026-09-02, owner, on Lennart's "how about full multilanguage")

Decision-free per the sizing above, so it starts now. Design settled by one
measurement: today's TED daily (fetch 531) is **100% eForms** (sdk-1.12/1.13/1.14,
3,299 notices), so no ted-export notice arrives on the daily chain any more and
the dispatch default can flip globally — it changes re-parses and nothing else.
A per-job policy parameter would have been machinery for a case that does not
exist. `EnOnly` stays available to callers; the test that pinned it as the
default now pins BOTH directions (default keeps FR, opt-out still drops it).

### Pre-flip baseline, fetch 94 (2018-08 monthly), measured on prod

| | |
| --- | --- |
| notices | **48,210** — 44,209 `ted-export-r209` + 4,001 `ted-export-r208`, all `parsed` |
| `notice_texts` rows | **3,218,322** |
| text bytes (`SUM(LENGTH(value))`) | **214,776,240** |
| by language | PL 634,676 · FR 508,670 · DE 507,268 · EN 403,617 · ES 132,440 · RO 123,778 · CS 121,287 · BG 104,350 · HU 96,713 · NULL 83,638 · IT 79,165 · NL 72,432 |
| DB file | 526,546,280,448 B |

Two things the baseline says before the run: the ORIGINAL languages are already
the bulk of what is stored (PL/FR/DE ahead of EN), so what the flip adds is the
translation copies beside them; and the +57% form-byte prediction from the
regex census should show up as roughly +1.5-1.8M `notice_texts` rows for this
month — that is the number the post-run read is judged against.

Measurement notes for whoever re-runs this: `WHERE fetch_id = ? GROUP BY profile`
is a 7.5M-entry walk of `notices_profile` on turso (10 s cap, hit once); `GROUP
BY +profile` seeks `notices_fetch_id` in 0.1 s. The joins are driven from
`notices` by fetch and probe `notice_texts` through its PK. Recorded in
docs/agents/prod-box-reads.md with the other two planner traps.

### The run

Enqueue shape: `reparse {profiles:["ted-export-r209","ted-export-r208"],
after: 93, packages: 1}` — `reparse_packages` orders by fetch id strictly above
`after`, so this selects exactly fetch 94, with the paired incremental project
folding the re-parsed notices. Then the post-run read of the same four numbers.

### Correction to the sizing premise, from the gate

The flip's first casualty was `oth_not_yields_chain_edge_and_prose_body`: the
2014 OTH_NOT corrigendum fixture pinned `["EN", "PT"]`, and under `All` it
yields **all 24 languages** — BG, CS, DA, DE, EL, EN, ES, ET, FI, FR, GA, HR,
HU, IT, LT, LV, MT, NL, PL, PT, RO, SK, SL, SV. So "original + EN + occasionally
one more" is the *average* the regex census saw, not the shape: corrigenda (and
presumably other OP-translated types) carry the full set. The +57% form-byte
figure stands as a month-level average, but growth is type-dependent and the
prod run on fetch 94 is the number that counts. The test now pins both
directions: the default keeps the whole set labelled, and `EnOnly` still yields
exactly `["EN", "PT"]`.

### The run — interim read (2026-09-02 ~13:5x UTC, rev `d416104`)

Job 607: **48,210 notices re-parsed in 333 s** (~145/s), 0 unmatched, 0
failing. Paired project 608 folding (160,334 notices planned — the month plus
its touched-tender expansion).

| fetch 94 | before | after | |
| --- | --- | --- | --- |
| `notice_texts` rows | 3,218,322 | **7,278,523** | **2.26×** (+4,060,201) |
| DB file | 526,546,280,448 | 527,031,521,280 | +485 MB (fold in flight) |

That is well past the +1.5-1.8M rows the byte census implied, and consistent
with the correction above: the census averaged over form copies, and the
notice types that carry all 24 copies (corrigenda at least) dominate the row
growth. Bytes and the per-language split need the id-sliced read (the single
join now exceeds the 10 s cap at this row count) — below.

One thing to understand before the corpus run: `run_reparse` stamps
**every** tender of the named profiles epoch-stale (`stamp_stale_for_profiles`,
3,529,040 tenders here), not only the package it re-parsed. What the paired
fold does with a stamp it does not visit decides whether that is harmless
bookkeeping or a 3.5M-tender rewrite waiting for the next fold.

### The month, measured (id-sliced read, 8 × 6,027 ids, ~3 s each)

| fetch 94 | before | after | |
| --- | --- | --- | --- |
| `notice_texts` rows | 3,218,322 | 7,278,523 | 2.26× |
| text bytes | 214,776,240 | **327,515,262** | **+52.5%** |

**The sizing was right where it counts.** The regex census predicted +57% legacy
form bytes; the parsed layer grew +52.5%. The row multiplier is larger because
what the copies add is mostly short strings — 24 title renderings, corrigendum
prose in every language — so the average text shrinks from 66.7 to 45.0 bytes.
Per month: ~+4.1M rows, ~+113 MB of text, +485 MB of DB file (the parsed layer's
share; the canonical layer's share lands with the fold). Over the ~93 packages
r208+r209 hold, expect on the order of **+45 GB parsed + a comparable canonical
share** — against 702 GB free. Fits with room, and `disk-census`/`diskwatch`
(80%) are watching.

### What the epoch-stale stamp means for the corpus run — READ BEFORE STAGING

`run_reparse` stamps by PROFILE, deliberately (a superset: "a stale stamp only
forces a rewrite that recomputes identical content, while a missed one silently
loses the re-parse"). So the 1-package run stamped **all 3,529,040 r208/r209
tenders**, and its paired fold — 160k planned notices, over the ≥100k
incremental→full fallback — is now a full-path pass rewriting every one of them
(job 608, pre-pass over 4.3M notices at the time of writing; hours).

That is correct and safe, and it means a STAGED corpus run would be a disaster:
92 capped enqueues = 92 profile-wide stamps = 92 multi-hour full folds. The
corpus run is therefore **one uncapped enqueue**, queued behind 608 so FIFO
orders it — `reparse {profiles, after: 94}` continuing exactly where 607's
counts line said to — with its own paired fold at the end. One stamp, one fold.

**Job 609 (reparse, 92 packages) + 610 (project) ENQUEUED 2026-09-02 ~14:0x UTC.**
Expected: 92 × ~333 s ≈ 8.5 h of re-parse after 608 lands, then one full fold
(the epoch-refold precedent: ~10.5 h). Tomorrow's 07:35 daily chain queues
behind it and lands late — the same cost the ADR-0014 refold paid.

**NO DEPLOY until 610 lands.** A deploy restarts the service and recovery re-runs
the running job from the top (deploy.sh's own busy guard says so). The deployed
rev will drift behind origin/main for docs-only commits meanwhile; that is
expected, not a finding. `reparse` and `project` are both in STOPPABLE_KINDS if
something has to give.

Spot-check of the parsed layer (notice 19946383, the month's first): title copies
now present per language — see the firing log; the end-to-end `?lang=` check on a
2018-08 tender is the next firing's job once 608 has folded them.

### Correction (2026-09-02 ~14:1x UTC): what job 608 actually is

Read too early above: 608 is an **incremental** fold of the **43,538 touched
Tenders** (the month's re-parsed notices and their chains), not a full-path
rewrite of the 3,529,040 profile-stamped ones — the journal says `incremental
fold: 2779/43538 Tenders folded`. The pre-pass over 14.3M notices and the
26 GB WAL peak were its plan build (mention resolution, one transaction); the
WAL **truncated to 0 at commit**, exactly the ADR-0014 refold's shape. So:

* the 3.5M epoch-stale stamps stay pending and are rewritten when a fold next
  visits them — job 610, after the corpus re-parse, which touches those
  Tenders anyway. One rewrite, where it belongs.
* the "staged corpus run would be a disaster" conclusion stands for a
  different reason than stated: not because each capped run's fold rewrites
  3.5M, but because each capped run re-stamps 3.5M and the FINAL fold pays
  for all of them once either way — 92 stamps buy nothing and cost the
  stamping writes. One uncapped enqueue remains right.
* issue 339 filed: the full-path/plan-build stage shows no phase record while
  its transaction runs.

### End-to-end probe: POSITIVE — and it found the corpus run's floor gap (2026-09-02 ~15:1x UTC)

After 608 folded the month, tender 6287622 (notice 19946383, the month's first):

| version | caused by | languages in `tender_version_texts` |
| --- | --- | --- |
| seq 1 | 19946383 (2018-08, re-parsed) | **24 languages × 9 texts** — BUL … SWE, normalised |
| seq 2 (head) | 20221184 (2019-01 award, fetch 89) | ENG × 11 |

The fold carries every copy through (no fold-side language filter exists —
checked the two `NoticeValue::Text → Fact::Text` sites). The head is English-only
because seq 2's notice is still on its EnOnly parse and ADR-0013 supersedes text
fields wholesale — the documented rule, not a defect. Which exposed the real
finding: **fetch 89 is below the corpus run's floor.**

`after` is a fetch-ID floor and the monthlies were fetched NEWEST-FIRST: ids
2–93 are 2026-06 back to 2018-09, so `{after: 94}` (job 609) walks 2018-08 and
OLDER only. The newer legacy half was going to be skipped. Measured exactly —
one `fetch_id = ?` seek per id, printed then counted, non-answers abort:
**70 packages with parsed ted-export notices at ids 24–93** (2024-06 → 2018-09;
r209 stragglers reach into the eForms-era monthlies).

Queue surgery, done: dropped the queued fold 610; enqueued
`reparse {profiles, after: 0, packages: 70}` — ascending by id it walks 24→93 and
the cap stops it exactly before 94 — with its paired fold. The queue is now
**609 (ids 95+, running) → 611 (ids 24–93) → 612 (the one fold)**. The 3.5M
profile-wide stamps are still paid once, by 612.

Revised ETA: 609 ~21:00 UTC today (153 notices/s), 611 ~+6 h, 612's fold the
epoch-refold order (~10 h) → landing tomorrow afternoon UTC. Tomorrow's 07:35
daily chain queues behind it. **Deploy stays frozen until 612 lands.**

Side finding filed as issue 343: the head version's three tender-level ENG
titles are tie-broken differently by the fold's `current_title` and the
read-time pick.

### Landing runbook (written 2026-09-02 17:5x UTC; executes whichever firing catches 612)

In order. Nothing here before 612 shows `ok` in `/admin/jobs`.

1. **Read 612's counts line** against expectation: tenders written should be
   on the order of the 3.5M profile-stamped plus the re-parsed cohort's chains;
   "0 verified unchanged" for the stamped ones (epoch-forced). Health green,
   journal clean, WAL truncated, disk free noted.
2. **Queue idle → deploy main tip** (`./deploy.sh`, HEAD; the guard refuses a
   stale ref). The bundle: 339 (fold barrier tick), 340 (`original_lang`
   column — boot runs the O(1) `ALTER`), 343 (title tie rule), plus the day's
   docs. Health green on the new rev; all refs equal again.
3. **Enqueue `backfill-original-lang`**; read its counts line (tenders walked).
   Then the close-out read for 340: `original_lang` NULL share per era on a
   bounded window (`GROUP BY +profile` shapes) — the NULL share IS the 1990s
   text-era share and the note should say so.
4. **Probes**, all bounded:
   * a legacy r209 tender whose HEAD is now a re-parsed notice: default title
     and `?lang=de` / `?lang=fr` flip as the copies exist; `original_lang`
     visible in the detail JSON;
   * the 6287622 head: `v_tenders.title == /v1/tenders/6287622 title` (343);
   * `/v1/lots?lang=…` on the same tender.
5. **Corpus-level language read** for the stage-1 close-out: title languages
   in one r209-dominant window (the 6.5M–6.505M shape from the 291 answer)
   before/after — the "after" should show the translation copies beside the
   originals; and `tender_version_texts` growth against the +52.5% month.
6. **Board**: close 340 with the backfill numbers; close 304 stage 1 with the
   corpus numbers and the per-package disk cost measured during the run
   (below); 343 CLOSED on the probe. 339 is NOT testable on 612 — it runs the
   binary deployed before the fix (`d416104`), so its job row is expected to
   read "pre-pass <count>" through its whole first bucket: the CONTROL case.
   The fix shows on the first bucketed fold after the deploy; close 339 then.
7. **Tomorrow's daily chain** will have queued behind the campaign; confirm
   it ran (`probe`/`process`/`project` ok) and `fetch-rates` too.

Stop conditions during the wait: free disk < 150 GB, WAL > 300 GB, or a
`reclaim stamped NO ledger rows` line — cancel the running kind (both are
STOPPABLE) and read before acting.

### Disk cost, measured mid-run (2026-09-02 17:5x UTC, 609 at 51/92)

| | |
| --- | --- |
| DB file, pre-campaign | 526,546,280,448 B |
| DB file now | 573,451,116,544 B (**+46.9 GB**) |
| covered | fetch 94 (re-parse + its fold) + 51 packages of 609 = 2,092,646 notices |
| per package / per notice | **≈0.9 GB / ≈22 KB** (parsed layer; the fold's share for the month is inside the total) |
| free now | 667 GB (earlier lower readings held the 26 GB WAL peak and pre-pass spill, since released) |

Projection: 609's remaining 41 packages ≈ +37 GB, 611's 70 ≈ +63 GB → ~567 GB
free before the fold; 612's canonical share (versions × up to 24 languages of
text) bounded by the parsed share again → **~420–470 GB free at landing.** The
150 GB stop bound is not in play. `disk-census` will record the step on Sunday;
`diskwatch` (80%) stays far below threshold.

### Run telemetry (2026-09-02 19:5x UTC): the policy applies uniformly; cost per package holds

| | |
| --- | --- |
| 609 | 83 / 92 packages, 3,234,350 notices re-parsed, ~150/s, WAL bounded (3 MB) |
| DB file | 604,544,212,992 B — **+78.0 GB** over the month + 83 packages ≈ **0.93 GB/package** |
| disk | 66% used, 581 GiB free; diskwatch `ok` hourly at its 80% drop-in threshold |
| audit | fetch 150 (2013-12, r208): 37,869 notices, 11,124,569 `notice_texts` rows — **294 rows/notice** against the 2018-08 month's post-flip 151 (pre-flip 67) |

So the older r208 forms carry even more copies per notice than the 2018 sample —
the flip is applying everywhere the re-parse reaches — while bytes per package
stay on the estimate (older titles are shorter). Projection unchanged: ~+70 GB
for the remaining 9 + 70 packages, then the fold's canonical share; landing
well above the 150 GB stop bound.

### 609 landed (2026-09-02 20:4x UTC) — the older legacy half, clean

```
re-parsed 3511248 notices across 92 packages (3512214 members walked,
0 unmatched, 0 now failing and left untouched); stamped 3529040 tender(s)
epoch-stale — 21,900 s (6.1 h, ~160 notices/s)
```

`0 now failing` is the line that matters: the current parser parses everything
the stored layer held. Audit on a late package (fetch 178): every notice
`parsed`. Quarantine outstanding 310 (+2 since the ADR-0014 landing — the
dailies' own, not this run). DB +86 GB for the month plus 92 packages
(≈0.93 GB/package held to the end); 567 GiB free at 66%.

611 (ids 24–93, the newer legacy half) started at once: 10/70 after 31 min —
faster per package, as those eForms-era monthlies carry fewer r208/r209
notices. Then 612, the one fold.

### 611 landed (2026-09-03 02:18 UTC) — the newer legacy half, clean; 612 folding

```
re-parsed 3630304 notices across 70 packages (4057639 members walked,
0 unmatched, 0 now failing and left untouched); stamped 3529040 tender(s)
epoch-stale; 93 package(s) held back by the cap — 21,520 s (6.0 h, ~169 notices/s)
```

Again `0 now failing`. The 93 held-back packages are the cap from the queue
surgery doing its job (ids 94+ were 609's). The stamp count equals 609's because
`stamp_stale_for_profiles` stamps every tender of the named profiles, not the
run's own — by design; 612 is what settles it.

Both re-parses together: **7,141,552 notices / 162 packages / 12.1 h, 0 failing.**

612 (`project`, the one fold) started 02:18 UTC, straight off the queue.
Telemetry at 02:53 UTC, 35 min in:

| | |
| --- | --- |
| phase | planning, 6,030,000 / 14,331,573 notices planned (~2,900/s → plan build done ≈03:40 UTC, then identity → pre-pass → buckets) |
| WAL | 379 MB, growing ~4 MB/min; every chunk logs `plan checkpoint … busy=true wal_frames=0 checkpointed=0 — WAL not fully reclaimed` — the plan build's one open transaction, expected, not a reclaim fault |
| `writer_longest_wait_seconds` | 752 s high-water mark (a writer queued behind a campaign transaction at some point since boot; queue depth 0 now) |
| DB file | 638,310,158,336 B (**+111.8 GB** since the campaign began); 508 GiB free at 70% |
| rss | 14.6 GB (the 512 MiB page cache plus the plan) |
| health | 200 on `d416104`; 0 `reclaim stamped NO ledger rows`; no failed units; load 1.0 |

Stop bounds (free < 150 GB, WAL > 300 GB) nowhere near. Next firings: watch the
WAL through the plan build's commit and the bucketed fold's first bucket (the
339 CONTROL case — expect "pre-pass <count>" to sit through it), then the
landing runbook above.

### 612 plan built (2026-09-03 04:41 UTC); pre-pass running

| | |
| --- | --- |
| plan build | 14,331,573 notices planned in 2 h 23 min (02:18 → 04:41 UTC); ~2,900/s over the legacy ids, ~1,000/s over the eForms ids |
| WAL through the build | peaked at **0.8 GB** at the last read before commit, then truncated to 4 KB — the "tens of GB" carried in this issue's stop-bound reasoning did not materialise on this shape |
| grouping | 7,922,692 tenders in 681,106 islands, 236.6 s; peak RSS 25 GB |
| pre-pass | 31 count-balanced stripes of 462,308 parsed notices over ids (0, 29,957,753]; first 8 done in 158–461 s each; shard 8 (a sparse 8.26M-id stretch) at 731/s; 6.89M swept at 04:51 UTC |
| spill | `tender-db.db.proj_buckets` 18 GB at that point |
| disk | 71% used, 489 GiB free (DB 640.1 GB, +113.6 GB since the campaign began) |

So the fold is near-total: the stale stamp plus touched-group expansion reach
7.9M of the tender layer, which is the hours-long part. The 339 CONTROL read
holds: the job row shows `pre-pass 6887156` with no total, as the pre-fix binary
does. Load 26 during the pre-pass is the 8 workers, expected.

### Runbook addendum (2026-09-03 05:5x UTC) — storage step between "queue idle" and "deploy"

Between runbook steps 1 and 2: with the queue idle, run `tender-db-snapshot.service`
once by hand. It takes the post-campaign reflink (the new prod-read target) and
retires the 08-28 pre-campaign snapshot; Sunday's scheduled run retires 08-30 and
returns the ~400 GB the two of them hold (issue 169, 2026-09-03). Then deploy.
Also measured this firing: the fold runs at ~575 tenders/s (2 versions per
tender in this stretch), 5.31M/7.92M at 05:50 UTC → lands ≈ 07:05 UTC, and the
07:35 UTC daily chain queues behind it. Disk 73%, 493 GB free.
