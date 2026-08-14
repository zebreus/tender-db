# 201 — small outstanding quarantine buckets: name and decide the tail

Status: needs-triage
Kind: quarantine composition sweep (the issue-30 discipline applied to the residue)

## What (outstanding-by-reason readout, 2026-08-14, post-200)

- translation-structure-mismatch 94
- duplicate-section-id 23
- ambiguous-field 14
- unknown-field-code 9 (the OC/ON ledger key shows what drained; these 9 are the residue)
- "unreadable zip bundle: … Could not find EOCD" 8 (corrupt-at-source candidates — the
  1996 truncated-bundle family? verify against issue 30's corrupt class)
- unknown-customization 3 (notices declaring an unvendored minor — name WHICH; if a real
  new minor is publishing, it needs vendoring per the ACCEPTED gate)
- (unrepresentable-value 5,169 and unclaimed-content 51 are owned: sub-cent kept class,
  issue 199 + the parked r208 pair)

Each bucket: one grouping query, member extraction where needed, then fix / documented
keep / ledger naming per the 143/144 pattern. The unknown-customization 3 first — that
gate should be empty.

**2026-08-14 ~10:3x CEST box time (orchestrator) — unknown-customization bucket: SDK 1.2
vendored.** The 3 rows are the only eforms-sdk-1.2 notices TED ever carried (one eSender,
the 2022-11-11 daily) — which is exactly why 1.2 was never vendored: it looked unpublished.
Vendored verbatim from the OP-TED 1.2.0 tag (730 fields; upstream ships the tag with a
stale 1.1.0 sdkVersion stamp, normalized with a $comment), completeness harnesses green
over the new inventory, negative test moved to the genuinely-unpublished 1.4. Corrects my
answer to Lennart's coverage question ("1.1/1.2/1.4 never published" — 1.2 was, thrice).
Deploy + 3-row reclaim next firing (daily occupying the queue). Remaining buckets:
translation-structure-mismatch 94, duplicate-section-id 23, ambiguous-field 14,
unknown-field-code 9, unreadable-zip 8.

**2026-08-14 ~11:0x CEST box time (orchestrator) — translation-structure-mismatch read
(code side).** The r209 parser overlays translation copies onto the original-language
structure; a translation OPENING a section the original never had (r209/parse.rs
open_section, `translating` guard) rejects the member whole. The 94 rows are ~15 shapes:
F14 CHG-N blocks (translations enumerating changes differently, ~38), extra review-body/
authority ORG-N sections (~26), LOT-2 on F03, r208 twins. Next: extract 2-3 members (after
the fold frees the disk) and decide per family — tolerate (open the translation-only
section; it is published content) vs documented keep. Deploy stack (ledger entry, sibling
guard, SDK 1.2) + the 3-row 1.2 reclaim also pending queue-idle.

**2026-08-14 ~12:0x CEST box time (orchestrator) — unknown-customization: 0.** Deploy
16e81a8 (ledger 42, sibling guard, SDK 1.2); reclaim 3/3, incremental fold (2s — the
full-sweep trigger is text-era-specific, consistent with 192's data points). Bucket empty;
next: translation-structure-mismatch members.

**2026-08-14 ~12:2x CEST box time (orchestrator) — translation-structure-mismatch
DIAGNOSED (ORG family).** Member 160877_2015 (r208 F02, BE): TWO sections both marked
CATEGORY="ORIGINAL" — DE with 2 ORGANISATION blocks, FR with 3. Belgium publishes
co-originals, and the second original can carry MORE content (the extra FR-side org);
the overlay's structural-replication guard treats every subsequent section as a
translation copy and rejects the member whole. Fix direction: read CATEGORY — only
translation copies keep the strict must-exist guard; a co-ORIGINAL section may open new
sections (its content is first-class published data). The CHG-N/F14 family (~38 rows)
needs its own member look — change notices may genuinely enumerate changes differently
per language. Implementation next firing: r209/parse.rs overlay mode + fixture from this
member + the r208 twin path.

**2026-08-14 ~13:1x CEST box time (orchestrator) — co-original adoption implemented +
deployed (rev eb05139), reclaim running.** r209/parse.rs: a co-ORIGINAL secondary may OPEN
a missing section, which is ADOPTED — inside it the text-only suppression lifts (nothing
there is shared with the primary), so its codes/dates emit too; genuine TRANSLATION copies
keep the strict guard (pinned by an EN-relabel negative on the new fixture
f02-co-original-160877-2015.xml, corpus 6→7). Reprocess reason=translation-structure-
mismatch running over all 94 rows — the CHG/F14 and LOT-2 families may be the same
co-original mechanism (bilingual buyers' F14s enumerating changes per original) or may
re-hold with fresh details; counts decide. Also: disk filled again mid-build (debug tree
regrew to 27G); pruned artifacts >40MB, 22G free — recurring nuisance, consider a
tmpfiles.d-style prune or cargo sweep in the worktree if it keeps biting.

**2026-08-14 ~13:3x CEST box time (orchestrator) — translation-structure-mismatch reclaim:
92/94.** The CHG/F14 and LOT-2 families were the same co-original mechanism — all drained
by adoption. 2 members re-held under the same reason, now from genuine TRANSLATION copies
(an EN translation carrying an extra RES-2 / CHG-1 block the original lacks — the guard
working as intended): 045641_2014 (r208 award) and 275223_2021 (F14). Next: extract both,
decide adopt-vs-keep for translation-borne extra sections — likely a documented keep or a
scoped adoption if the content is a real translated block TED's pipeline added. Remaining
201 buckets after this pair: duplicate-section-id 23, ambiguous-field 14,
unknown-field-code 9, unreadable-zip 8.

**2026-08-14 ~14:1x CEST box time (orchestrator) — translation adoption extended, pending
deploy.** Extracted both holdouts: 045641_2014's ES ORIGINAL carries 1 award where all 23
TRANSLATIONS carry 3 (real award data lost by TED's pipeline in the original); 275223_2021's
LV original has NO CHANGE block where the translations carry one. Every firing of the strict
guard in 30 years of corpus was recoverable content, so walked secondaries (translations
included) now ADOPT missing sections and the translation-structure-mismatch reason RETIRES
(ledger entry added; relabel test flipped to pin adoption; gates 220 green, pushed). Deploy
+ the final 2-row reclaim next firing — the mismatch reclaim's trailing fold is still in
its issue-192 full-sweep (old-era members), don't restart it. After that: duplicate-
section-id (23), ambiguous-field (14), unknown-field-code (9), unreadable-zip (8).

**2026-08-14 ~15:2x CEST box time (orchestrator) — duplicate-section-id fixed (pending
deploy).** Both shapes are one entity republished: DÖE sdk-1.0 emits the Organization
block once per UBO (4× ORG-0010 differing only in the UBO ref, 19 members), a 2024
eSender registers the same org full-then-sparse (4 members). Re-published ids now MERGE
(same id + same kind = same entity, values append); cross-kind collisions still
quarantine. Fixtures ×2, corpus 40, ingest 221 green, pushed. Deploy stack now: adoption
extension + this merge + ledger 43; then reprocess translation-structure-mismatch (2) and
duplicate-section-id (23). Still gated on the long fold.

**2026-08-14 ~16:1x CEST box time (orchestrator) — deploy 3f4457b; translation-structure-
mismatch EXTINCT (2/2 reclaimed, job 679: the defective-original pair recovered via
translation adoption).** Its trailing fold is the old-era full sweep again (~2h), with the
duplicate-section-id reclaim (23 rows) queued behind it — counts next firing. Dashboard
dual-tender-count observation parked until the queue idles (the coverage measure skips
during write jobs by design; if the two numbers still diverge at idle, file it).

**2026-08-14 ~17:1x CEST box time (orchestrator) — ambiguous-field fixed (pending deploy).**
Three discriminator failures, all claimed as published: the misspelled reserved-executionn
list (13, leaf-predicated carve-out à la 'permission'), the attrless SelectionCriteria
ParameterCode (selection-side twin of cause J, same predicate-free-branch guard), and the
German national stift-oer-kommun buyer-legal-type on plain EU (1). Fixtures ×3, corpus 43,
ingest 222 green, pushed. unknown-field-code readout: the 9 rows are 'line 2x: OC' text
stragglers of the already-ledgered OC class — need one member look (next). Deploy + reclaims
(ambiguous-field 14, duplicate-section-id still queued behind the fold chain) when idle.

**2026-08-14 ~18:2x CEST box time (orchestrator) — unknown-field-code + unreadable-zip
attributed.** The 9 OC rows are 1997–98 records never re-attempted since the OC rule
landed (attempts NULL) — reprocess enqueued behind the chain, no code needed. The 8
unreadable-zip rows: 7 are non-EN siblings / a cf companion of COVERED days (documented-
keep candidates), but the 8th exposed a real coverage gap — the corrupt EN UTF8 of
2005-04-09 suppressed its READABLE ISO twin via the name-based supersedence, losing the
whole day (~900+ notices, archive has the data). Spun off as issue 202 (DIAGNOSED, fix
design in file). Queue currently: fold → dup-section reclaim → fold → OC reprocess → fold.

**2026-08-14 ~19:2x CEST box time (orchestrator) — sweep nearly closed.** duplicate-
section-id drained 23/23 (job 681/682); ambiguous-field drained 14/14 (deploy 6c64061,
job 685/686 — bucket empty); unknown-field-code emptied (the 9 OC records claim their OC
line now and re-held under unclaimed-content with fresh orphan-line details — moved to
issue 199's population). Filed 203 (stale dashboard count).

**2026-08-14 ~20:0x CEST box time (orchestrator) — SWEEP CLOSED.** Final bucket done:
unreadable-zip attributed and ledgered ("Corrupt zip bundles in the TED archive (EOCD
missing)", entry 43) — the 2005-04-09 day recovered via issue 202's per-day supersedence
(932 notices, verified), the 7 non-EN siblings documented keep, all 8 rows stay held as
the record of the corrupt bytes. Every bucket this sweep named is now empty, ledgered,
or moved: translation-structure-mismatch retired (0), duplicate-section-id 0,
ambiguous-field 0, unknown-field-code 0 (→ issue 199's orphan-line population),
unknown-customization 0, unreadable-zip 8 documented-keep. Issue 203 resolved as
not-a-bug (fold history rows, not a stale gauge). Outstanding quarantine is now:
unrepresentable-value 5,169 (kept by policy), unclaimed-content ~60 (issue 199 + the
parked r208 pair).
