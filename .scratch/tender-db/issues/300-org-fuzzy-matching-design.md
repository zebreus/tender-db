# 300 — organization matcher: the tiered evidence engine (design)

Status: DESIGNED 2026-08-28 — full design below; build not started. Produced
at ultracode effort: three independent design drafts (identifier-maximalist /
evidence-ladder / satellite-first) from a shared research base (repo current-
state with file:line, fresh corpus probes, a 20-country identifier-scheme
cross-walk study), judged by a three-lens panel (false-merge safety,
implementability, evidence fidelity). The evidence-ladder draft won 2 of 3
lenses and is the skeleton; the panel's convergent grafts from the other two
are folded in below and marked ⊕. Stage 0 is the first buildable unit.

Stage-0 progress (2026-08-28, same day): exemplar sheet checked in
(300-exemplars.md) with every probe specimen re-verified live by hand; the
CNFPT determination is MADE — the satellite pair is acronym↔expansion (E4),
so CNFPT is must-FLAG, no corroboration-weakening pressure; the satellite
contamination specimen's mechanism is FOUND — the source notice itself
crosses multilingual BT-500 slots between ORG sections (25038532; publisher
authoring error, our capture faithful), so contamination is a SOURCE-data
property to weight, not a capture bug to fix; first bounded census of the
class: ~10.6% of BT-500 notices carry a same-lang duplicate value across
sections (upper bound; wrong-name subset unmeasured — the sharper census is
specified in the exemplar sheet). org-merge-health BUILT, DEPLOYED
(63ce9c9), and RUN the same day — first baseline (run 1330, ~2 min):
1,164,430 identifier-bearing orgs; ≥2 distinct N2 names 168,905 (14.5%);
≥6: 12,291 (1.06%); ≥20: 1,065; max 794 — all tighter than the [rates]
lower+trim figures, as N2's punctuation folding predicts. The census's
first run already earned its keep: the top of the distribution is a MIX,
so Stage 0's "freeze the top-100 allowlist" is corrected to "CLASSIFY the
top-100" — five of the top twelve are the `NIMAT\d+` placeholder family
(org 211's 794 names are measured STRANGERS: ministries, utilities, an
insurer under one id), reversing the 168 study's "legitimate variance"
call on ELEKTRO PRIMORSKA, and `PL823` (a 5-char VAT stub, 418 names)
joins the lexicon. See the exemplar sheet's reclassified section.

Stage-1 tranche 1 BUILT + DEPLOYED (164a7a6, same day): `ingest::idgate` —
lexicon/sequence/letter-run/short-VAT rules + 16 checksum validators, all
adversarially verified by three independent workflow agents (one blocking
catch: La Poste establishment SIRETs use INSEE digit-sum-mod-5, not Luhn —
a class the ≥97% census could NOT have caught, proving the verify-first
discipline) — riding the census walk, still zero live-path changes. First
enablement census (run 1331): **≥97% and HARD-eligible:** HR:oib 99.8,
FI:ytunnus 98.9, SE:orgnr 98.8, PL:vat-nip 98.8, BE:vat 98.7, IT:piva
98.6, CZ:ico 98.3, GR:afm 97.9, SE:vat 97.6, DE:vat 97.4, NO:orgnr 97.2,
PT:nif 97.1 (+ FI/IT/PT vat twins ≥99, CZ:dic-ico 99.6). **Below the bar,
stay SOFT:** FR:siret 95.8 (3,536 fails — the measured left-zero-padded
forms fail literal Luhn; canonical_key's pad-strip rescues them), PL:nip
95.6, BE:kbo 96.7 (pre-2008 9-digit base suspected), FR:siren 92.0,
PL:regon9 89.6, FR:vat 87.3 (small bucket, sample before trusting).
Placeholder counters: 2,152 lexicon, 680 sequence, 1,917 short-vat,
140,244 letter-run (12% — consistent with the 16% legacy junk baseline
but its composition MUST be sampled before rule-4 ever enforces).
`other` = 763,964 (65.6%) — tranche 2's target: ES letter-NIFs, RO CUI
lengths, DK CVR, NL KvK, AT FN.

Census refinements (a)+(c) BUILT same day; (b) SAMPLED — the letter-run
140k class decomposes into RECOVERABLE STRUCTURE, not junk (live sample):
`REGON470850645` / `NIP…REGON…` label-prefixed and compound fields carrying
real ids (the B8 splitter's target — "REGON" at 5 letters sat past the ≤4
prefix strip, now ≤6); HU `EKRSZ\d+` platform ids (rule-7 scoped scheme);
32-40-char platform hex hashes (FR/BE — now their own census class);
`HRB2320STRALSUND` — the DE court-scope embedded IN the value, raw
material for the court-scoping fix; spelled-out labels
(`FISCALCODENO…`). Consequence recorded: rule 4 must run AFTER the
compound splitter and label-prefix strip, never as a blanket ≥4-letter
reject — the gate-flip tranche implements the splitter first. Checksum
scoring now excludes lexicon/sequence-condemned ids, FR pads score the id
under the padding (`FR:siren-padded`), and the census gains hex_hash +
compound counters — next run re-reads the enablement bar on clean
denominators.

**ENABLEMENT DECISION (run 1332, rev 782dfb9 — measured twice, stable, so
this is the standing input to the gate flip):** HARD (≥97%, checksum may
reject → provisional): DE:vat 97.4, SE:orgnr 98.8, CZ:ico 98.3, IT:piva
98.6, FI:ytunnus 98.9, NO:orgnr 97.2, HR:oib 99.8, PT:nif 97.1, GR:afm
97.9, PL:vat-nip 98.7, CZ:dic-ico 99.6, and the FI/BE/IT/PT/SE VAT forms
(97.6-99.8). SOFT (advisory only — failures recorded as evidence, never
rejecting): FR:siret 95.8, FR:siren 92.0, FR:vat 87.3, PL:nip 95.6,
PL:regon9 89.8, BE:kbo 96.8, HR:vat (pop 52, too small). The
condemned-exclusion refinement moved these rates by ≤0.2 points —
honestly refuting the "junk depresses them" hypothesis: the FR/PL fails
are inherent typo/corruption load, and the bar keeping them SOFT is the
design working. Refined class sizes: letter-run 140,244 → **27,781**
(structure-aware), hex-hash **273,982** (23.5% of id-bearing orgs carry
platform hex ids — merge-inert singletons, not junk to dissolve),
compound 3,332 (recoverable via the splitter).

**THE v2 GATE IS LIVE (58e4d24, deployed 2026-08-28 ~19:0x, health green):**
`normalise_identifier` refuses lexicon/sequence/short-VAT/HARD-checksum
classes to the provisional path. Scope discipline held: NO value reshaping
(rejection cannot create prevention-vs-stock splits; the compound splitter
stays Stage-2 match-time work), and rule 4 (letter-run) stays census-only
pending composition. New condemned mentions mint provisionals instead of
joining stock false-merge orgs — accepted transitional cost until the
dissolve erases the stock side. Two junk-valued test fixtures replaced
with real-shaped ids. Verification note: the planned adversarial workflow
died on a temporary model-usage cap; the two lenses were closed solo with
deterministic checks instead (live call site passes alpha-2 — verified at
project.rs:3108; the green 70-suite gate proves no fixture identifier
shifts; each replacement fixture hand-computed through the gate) — the
checksum arithmetic itself was already triple-verified in the previous
unit.

The dissolve is BUILT and its first prod DRY-RUN is read (job 1333, rev
418699f, ~4 min over the whole corpus): 7,865 condemned by the live gate
(0.68% — matching the census prediction), **6,490 dissolvable** re-resolving
111,369 mentions (79,353 reuse standing provisionals / 32,016 fresh),
51,985 winner rows, 54,091 tenders touched; **1,375 skipped whole** on the
one-mention-per-notice winner guard — org 211/NIMAT dissolves clean (0
multi-mention notices, 7,804 winners), org 15566 (723 multi-mention
notices) waits for tier-2 disambiguation (filed as issue 309). The
allowlist is untouched by construction (soft schemes). The first preview
under-reported party rows (wet-only counting); fixed with org-level
dry-run counts before the wet run.

**THE STAGE-1 CAMPAIGN RAN 2026-08-28 (jobs 1335 tier-1 + 1337 tiers-2/3,
both preview-exact): 7,414 of 7,865 condemned orgs dissolved (94.3%)** —
143,032 mentions re-resolved (dominantly onto reused provisionals: the
wet runs' cross-mention memory collapses same-name strangers onto one
row), 229,013 party + 314,087 bid-party rows moved, 386,768 winner rows
(81,975 duplicate-collapses — placeholder double-counting erased),
~61k tenders on the change feed. Post-campaign census (run 1338): the
condemned classes drained 94% (lexicon 2,152→362, sequence 680→33,
short-VAT 1,917→58 — the ~453 remainder IS the 451 residual), org table
shrank by exactly 7,414. The PL823 org dissolved; the seven flagship
mega-orgs (NIMAT family, 15176, 15566) sit in the residual — their
tenders mention them under DIFFERENT names within one chain, and tier-4
(lot-result origin + parse-layer section descent, designed in issue 309)
is the per-row-precise fix. Satellite backfill re-run enqueued (job 429).

**STAGE 1 ACCEPTANCE (2026-08-28 ~23:5x, closing census run 1343):**
cumulative 7,774 of 7,865 dissolved (98.8%) across three preview-exact wet
runs; 168,440 mentions re-resolved; **557,130 duplicate winner rows
erased** (served award double-counting); the NIMAT family fully dissolved,
allowlist intact, and the corpus's top distinct-name org is now the
TRIBUNAL (700 — legitimate) instead of a placeholder: the distribution's
head is honest for the first time. Winner resolution ended ORIGIN-FIRST
(issue 309's record has the full mechanism story — three prod-measured
course corrections in one night, each caught by preview-vs-exemplar
discipline). **STAGE 1 COMPLETE (2026-08-29 ~02:1x, rev 93eeb29): 7,865
of 7,865 dissolved — 100%.** Tier 5 (dissolve-then-refold) took the final
91: winner rows are derived state, so the honestly ambiguous 1,046 rows
were deleted, their 385 tenders stamped epoch-stale, and the incremental
fold (job 437, 16s) re-derived every winner set from the re-bound
mentions — the specimen CAN's five lots now each name their own real
winner. Closing census (438): **0 lexicon / 0 sequence / 0 short-vat**;
the gate-invariant tripwire returns to 0.
Still due: first-post-flip daily-chain read (~07:4x: provisional mint
rate ticks up by the condemned share; new placeholder mentions no longer
merge).

**STAGE 2 OPENED (2026-08-29 ~03:0x): canonical_key + r2-census built,
adversarially verified.** `ingest::crosswalk::canonical_key` implements
§3.1 with an explicit E1/E2 tier on every key (prefix-strips/truncations
E1; pads E2 per the amendment) and a read-only `r2-census` job (rides the
org-merge-health walk; same-country E1 grouping, per-scheme stats, cap-8
counter, denial counters, 30-group inspection sample to
put_report("r2-census")). Two adversarial verifiers ran before commit;
both catches folded in:
- **ES DIR3 collision (the La-Poste-class catch):** DIR3 authority codes
  (letter+8 digits) share their exact shape with letter-check CIFs, both
  arriving kind=national — an E1 key could auto-merge a company with a
  public administration. ES is DEMOTED WHOLE to E2 (edges only; R3
  corroboration decides); pure-digit 9-char ES bodies refused (the bare
  123456789 placeholder shape).
- **NL demoted to E2** for VAT-derived heads: "legal entities only" has
  no enforceable syntactic test (post-2020 sole-trader heads are not
  RSINs; fiscale-eenheid group ids look like member ids).
- **HU áfakód-5 group ids refused** (csoportazonosító szám names the
  group, not a member). **SK group IČ-DPH has NO syntactic marker** — the
  merge job MUST implement denial rule 1 (mention-evidence VAT-group
  wall) before any SK wet run; recorded in-code.
- Unknown identifier kinds hard-refused (future GLN/DIR3 kinds must not
  ride national arms); prefix-less VATs refused (census scores them
  "other" — ungated class must not key E1); BE establishment range
  (leading 2-8) refused; BG 13→9 truncation kept OUT until ratified;
  FR 0-leading genuine SIRETs restored to E1 (only strip-to-exactly-9 is
  pad-ambiguous → E2).
Deliberate safe narrowings recorded in-code: FI legacy 6-digit no key,
CZ 7-digit vat no pad, FR letter-VAT refused, DK P-nummer no key.
NEXT: deploy + first prod r2-census read → sizes the match-org-identifiers
--r2 merge job (which needs denial rules 1/4/5/7 before any wet run).

**FIRST R2-CENSUS (job 439, 39s, rev f474d40, 2026-08-29 ~03:2x):**
1,156,565 identifier-bearing orgs → 401,761 E1-keyed / 44,916 E2 /
709,888 no key (DE+AT+unlisted). **25,544 same-country E1 groups ≥2
holding 61,286 orgs (~35,700 duplicate rows to collapse), 12,815 mixed
vat+national** — merges E0 could never make. Denials firing: 6,399
ES-UTE, 9 CZ699; 0 country/prefix contradictions, 0 NULL-country keyed
vats (resolver derives country from prefix at mint — consistent).
Per-scheme: FR:siren 12,837 groups (SIRET truncation, max 165);
FI 3,294 / BE 2,158 / PL:nip 1,820 / RO 1,613 / IT 798 — every
prefix-strip scheme max-2 all-mixed (the pinned twin shape, clean);
SK just 7 groups (group-IČ-DPH hazard surface tiny).
Two findings for the merge job, from the sample itself:
- **The group cap must count DISTINCT LITERAL identifiers, not rows:**
  271 of 272 over-cap groups are FR:siren establishment families
  (Colas 165, Onet 68, Bouygues 58, OTIS 54 — one entity each). The
  placeholder signature the cap guards is many strangers on ONE literal
  id; in truncation groups the literals all DIFFER. Cap on
  distinct-literal-count keeps the tripwire and frees the legitimate
  establishment shape.
- **FR groupement class (the UTE analog):** org 10207212 "groupement
  colas / Barthelemy" publishes Colas's establishment SIRET — a
  consortium identified by its lead member's id. Auto-merging it INTO
  Colas is wrong the way UTE merges are wrong. Merge job needs a
  groupement/consortium name veto (groupement/gpt/mandataire tokens →
  edge, not merge); the 100-sample precision review must classify the
  class's size.

**MERGE JOB BUILT (2026-08-29 ~04:1x): `match-org-identifiers` (r2).**
Store fn `match_org_identifiers_r2`: whole-corpus preload → same-country
E1 groups → the denial stack in order — literal-cap (>8 members on ONE
literal id, the census-refined semantic; hard ceiling 200), gate-poison
(rule 5), consortium name veto (the census finding), legal-form veto
(rule 7; crosswalk::legal_form_family with Oy/Ab/Oyj folded to ONE
Nordic family so naming variants and the Linde/AGA renames never veto),
VAT-group mention wall (rule 1: per-member canonical keys from mention
raw_identifier evidence; disjoint same-scheme key sets across members =
conflicting registers = deny — the SK/NL/HU group-id defense) →
survivor non-provisional-then-min-id → WRITE_BATCH merge txns via
repoint_org_references (names satellite rides along) + `org_merge_log`
rows (new §6 table) + the 234-shape change events. Job wiring: dry_run
default TRUE; a WET RUN REQUIRES the stored dry plan
(put_report("r2-merge-plan")) and aborts on >max(2%,50) divergence (T4
parity) — nothing can merge un-previewed; `max_groups` caps the first
prod run; STOPPABLE + heavy_write registered. Store test drives every
denial class once + survivor policy + merge log + feed + rerun
idempotence.

**Adversarial verification (2026-08-29 ~04:4x) — the campaign's third
CONFIRMED catch:** the canonical key that FORMED a group is itself
mention evidence on every member, so the VAT-group wall's disjoint-set
test always intersected on the shared value and same-scheme register
conflicts (the SK DIČ class — the design's named "only defense" case)
could never deny. Fixed: the group's own key is stripped from evidence
sets before the test; E2 keys admitted as CONFLICT evidence (deny-safe);
the mask shape test-pinned. Same review: wet runs re-record the RESIDUAL
plan so capped/stopped continuations pass parity (was: abort against the
stale dry figure); dry runs preview the losers' full blast radius;
failed COMMIT rolls back; committed chunks ring the doorbell on later
errors; evidence JSON drops control chars. Recorded narrowings: name
vetoes read the head name only (satellite variants not consulted — the
precision review covers; an enhancement for Stage 3), per-batch parity
implemented as one whole-plan pre-write check.

**FIRST PROD DRY-RUN (job 440, 17s, rev b9295ac): plan 24,915 of 25,544
groups.** Denials at corpus scale: 566 consortium, 63 legal-form, 0
gate (post-Stage-1, expected), 0 cap (the literal-cap is structurally
near-vestigial — E0 already collapses same-literal rows; the 200
ceiling + gate-poison are the active guards), 0 vat-group-wall
(plausible: twins' evidence reduces to the stripped group key; true
group-VAT shapes are rare same-country). Blast radius: 480,622
mentions, 1,024,472 parties, 2,075,662 bid-parties, 2,532,976 winner
repoints across ~36k losers.

**100-SAMPLE PRECISION REVIEW (2026-08-29 ~05:3x, sample from job 441 —
three independent passes: owner + two adversarial lenses): 99/100.**
The single FAIL, unanimous with the name lens: sample group 91 —
"Consórcio E.I.P. Serviços _ CME" shares lead member E.I.P.'s NIF (a
regular 5xx company-series NIF, per the identifier lens) — the PT
groupement vehicle, missed because the consortium lexicon lacked
"consórcio". RULE-FIXED (5b15041) + test-pinned; the same sample also
pinned the counter-case ("członek konsorcjum" is a MEMBER label — bare
"konsorcjum" stays OUT of the lexicon). Identifier lens validated every
scheme checksum in the sample (all 5 FR VAT keys arithmetically
correct; no placeholders; no wrong truncations). Two UNSUREs
adjudicated PASS: groups 65/79 each carry one SIRET with a corrupt
establishment-NIC, but the merge keys on the checksum-valid SIREN half
(which truncation preserves) and the names corroborate — the exact typo
load that keeps FR a SOFT scheme. G42 (Berkshire Hathaway
European/Specialty/BHSI on ONE SIREN) passed both lenses: one SIREN is
one French legal unit; the register is the ground truth R2 trusts.
NEXT: gate → deploy 5b15041 → re-dry-run (consórcio class must land in
denied_consortium; plan shrinks below 24,915) → CAPPED WET RUN
(max_groups 1000) → residual continuations → Stage-2 close-out census.

**WET MERGES RUNNING (2026-08-29 ~06:0x, rev 99452e1).** Post-fix
dry-run (442): denied_consortium 566→570, plan 24,911 — the consórcio
class denies exactly as reviewed. SLICE 1 (443, capped 1,000, 422s):
merged 1,000 groups / 1,000 losers, 9,856 mentions + 26,168 parties +
4,870 bid-parties + 5,168 winners repointed, 5,519 tenders touched, 0
dups; parity held; residual plan re-recorded 23,911. Tripwires green
(journal clean; Gemeente Westerlo twin spot-checked: survivor holds the
VAT row, loser gone). UNCAPPED CONTINUATION enqueued (job 444, ~23,911
groups, est ~2.5-3h at the measured 0.42s/group). After it: closing
census (expect mixed-kind twins collapsed; distinct-name tail moves),
the residual dashboard read, Stage-2 acceptance line (false-split floor
re-measure below 1.55% is the design's post-metric — needs the 168
study's method re-run, a follow-up unit).

**STAGE 2 STOCK REPAIR COMPLETE (2026-08-29 ~10:2x).** Job 444: ok in
3.6h — all 23,911 residual groups merged, cross-run parity EXACT.
Campaign totals (slices 1+2): **24,911 groups merged, 33,166 duplicate
org rows removed**; 480,610 mentions, 1,024,460 parties, 2,075,648
bid-parties, 2,532,542 winner rows repointed; 422 winner-dup award
double-counts erased; 171,265 tenders touched. Journal clean across the
whole window; the daily chain ran green behind the merge (1,042 notices
→ 1,035 tenders). Closing census (450): identifier-bearing orgs
1,156,565 → **1,123,400 (−33,165 — the losers, exactly)**; gate
tripwire holds 0/0/0. NOTE: distinct-name tail GREW (≥6: 11,734→12,230,
≥20: 926→980) — the union effect of merging twins (survivors carry both
rows' name sets), NOT a bad-merge signal; **tripwire 2's weekly
baseline re-anchors at census run 450.** First-post-flip daily-chain
read (satisfied, late behind the merge): 411 new orgs today — 410
provisional, 1 identifier-keyed canonical; no placeholder-keyed
canonical minted — prevention holds in the live stream.
REMAINING Stage-2 units (filed here, next firings):
1. **Resolver pre-probe canonicalization (the prevention half): BUILT
   2026-08-29 ~10:5x.** MentionResolver gains a canonical-key map
   preloaded beside org_of (same walk), with the merge job's exact
   discipline injected as the SAME `crosswalk::canonical_key_flat` fn
   (prevention and repair cannot drift): E1 only, same-country with
   vat-prefix agreement, consortium-named mentions never canon-bind
   (they mint and POISON the key), and a key with several standing
   owners (a denied family — Colas) is poisoned on preload so new
   mentions fall through to exact-or-mint; the periodic merge job stays
   the arbiter for those. E0 exact equality stays the first probe,
   byte-identical; `None` fns (store tests) keep pre-Stage-2 behavior.
   Store test pins reuse/veto/poison/E2/preload/disabled paths.
2. False-split floor re-measure (<1.55% bar): **DONE 2026-08-29 ~11:3x,
   both readings recorded** (168's exact SQL was not preserved, so both
   plausible measures ran). Literal reading — GROUP BY (country,
   identifier) HAVING ≥2 over identifier-bearing orgs: **ZERO clusters
   corpus-wide** (was 8,416 clusters / 18,018 rows = 1.55%). Canonical
   reading — post-merge r2-census (job 451): groups ≥2 fell 25,544 →
   **634** (3,211 orgs), and those 634 are exactly the deliberate
   denials (570 consortium + 63 legal-form): the remaining split is the
   split the design REQUIRES pending R3-grade corroboration, not a
   failure of the key. The Stage-2 post-metric bar (<1.55%) is passed
   by any reading. **STAGE 2 ACCEPTANCE COMPLETE.**
3. Groupement member-scoped veto: **BUILT 2026-08-29 ~12:0x.** The
   consortium veto now EXCLUDES the flagged member (left standing for
   the edge path) and merges the remainder when ≥2 survive; a remainder
   below two keeps the whole-group deny. The legal-form veto stays
   group-atomic (a family conflict is pairwise evidence with no
   resolution). New `consortium_excluded` counter in report + plan.
   Test pins both paths (3-member family merges around its groupement;
   2-member pair still denies). **RECOVERY EXECUTED (2026-08-29 ~12:4x,
   rev 0c7f0d4, jobs 452/453, preview-exact):** 243 recovered groups
   merged, **1,829 more duplicate orgs removed** (the establishment
   families — 7.5 losers/group), 10,685 winners repointed, 7,116
   tenders touched; 661 groupement members left standing for the edge
   path. **STAGE-2 GRAND TOTALS: 25,154 groups merged, 34,995
   duplicate org rows removed.** The R2 residual is now EXACTLY the
   deliberate holds: 328 remainder-below-two consortium groups + 63
   legal-form groups + the 661 excluded members — all edge material
   for Stages 3/4.

**STAGE 3 OPENED (2026-08-29 ~12:5x) — pool measured.** NULL-country
identifier-bearing rows: **4,816** (the design's pre-campaign 8,420
estimate shrank under Stage 1+2). Length shape: 2,093×8-digit
(MULTI-scheme ambiguous — FI ytunnus / CZ ičo / DK cvr / SI davčna all
fit; checksum anchoring is the discriminator and several schemes can
pass simultaneously → these need the EBSCO flag-first treatment unless
name corroboration decides), 1,388×14-digit (SIRET-shaped,
Luhn-anchorable — the CNFPT NULL class, the cleanest rescue),
782×9-digit (SIREN/NO/GR/PT shapes). **R3-CENSUS BUILT + FIRST RUN (2026-08-29 ~13:3x, rev f5abcc4, job 454,
36s).** `idgate::checksum_anchors` (unique-anchor probe; 8-digit never
unique — DK/SI have no checksum to exclude; Luhn-valid 14-digit anchors
to its truncated SIREN) + the read-only census with cross-language N2
corroboration against the anchored standing org. THE POOL DECOMPOSES:
**903 anchored+corroborated (R3 MERGE CANDIDATES)**, 888 anchored
without name corroboration (edges), 83 anchored/no standing target,
2,135 multi-scheme ambiguous (EBSCO flag-first class), 449 unanchored,
358 register-prefixed (the separate R3 alternative — court-scope
exclusion applies there). Sample of 40 candidates in the r3-census
report for the review. **R3 MERGE ARM BUILT (2026-08-29 ~14:4x).**
`store::match_org_null_country_r3` recomputes the census ladder
verbatim (register-prefix skip → unique real anchor, the DK|SI marker
never unique → exactly ONE standing target, several = the families R2
deliberately declined, skipped as multi_target → exact cross-language
N2 corroboration via org_all_names) then hardens with the R2 stack:
gate-poison BOTH sides, consortium veto both sides (two rows — no
remainder to salvage), VAT-group mention wall with the pair-forming
key stripped (the F1 mask rule) and every tier admitted as conflict
evidence. Candidate = loser, anchored standing row = keep;
org_merge_log rule='r3' with anchor+n2 evidence; same T4 ladder as R2
(dry-run records "r3-merge-plan", wet REQUIRES it, parity max(2%,50),
capped, stoppable, chunked txns, wet re-records the residual).
Supervisor: `match-org-identifiers` gained a `rule` param (r2
default / r3; unknown rejected at enqueue). Store test pins every
census rung + each denial (incl. the wall's mask shape) + parity abort
+ wet merge with blast radius + rerun idempotence. **DEPLOYED (rev f27ab63) + PROD DRY RUN (2026-08-29 ~14:2x, job 455,
ok): plan 891 candidates.** The census's 903 decompose under the
stricter merge exactly: 891 plan + 6 consortium-denied + 6
corroborated-but-multi-target; census's 888 uncorroborated = dry's 880
+ 8 uncorroborated-multi-target (multi_target=14 total skips before
corroboration; several standing targets = an R2-declined family, never
picked between). 0 gate, 0 wall. Blast radius: 5,588 mentions, 9,860
parties, 123 winners across the 891 (all non-provisional NULL-country
rows — measured, pool drift zero since census). "r3-merge-plan"
recorded. **EXEMPLAR GATES READ AGAINST PROD (2026-08-29 ~14:5x,
bounded selects):** every deviation is deny-direction. EBSCO
must-flag ✓ (NULL shapes live in multi-scheme/unanchored). Grand Port
✓ and the strongest evidence for the checksum defense: the Havre
family's NULL rows include 77370019800010 — a one-digit corruption of
the true SIRET 77570019800010 — which FAILS Luhn and lands unanchored,
plus a 12-digit deformed row with no anchor length; only well-formed
SIRETs anchor. Kærunefnd ✓ moot (its NULL twins carry no identifier —
outside this pool, Stage-5 material). Philips moot likewise. CNFPT:
NULL SIRET rows anchor to the post-R2 standing org, merge only on
exact satellite-name corroboration (per Stage-0 determination).
Maintpartner ✗-as-written: its NULL twin is 8-DIGIT (20445111), and
the DK|SI marker makes every 8-digit value structurally unanchorable —
conservative; the design gate predates the census's 8-digit
reclassification ("name corroboration decides"). NEXT-SLICE CANDIDATE
(the largest R3 class, 2,093 rows): admit 8-digit when exactly one of
{CZ ičo, FI ytunnus} checksum passes AND the name corroborates exactly
— the DK/SI ambiguity resolved by the name, Maintpartner's shape.

**VERIFICATION ROUND COMPLETE (2026-08-29 ~16:1x, ultracode workflow:
3 adversarial reviewers × 3-skeptic refuter panels + 4 sample judges,
48 agents).** 13 candidate findings; 3 CONFIRMED, 10 refuted. The
confirmed three: (1) FALSE-MERGE — co-anchored candidates were walled
only against their target, never each other, so two VAT-group members
sharing one anchored number could fold into an evidence-empty keep in
one run (the refuters sharpened it: the wall's verdict even depended
on chunk boundaries — C2 would be denied on a re-run after C1's
mentions repointed; SK:dic is unreachable from anchors, but IT:piva /
BE:kbo / NO:orgnr / SE:orgnr group regimes are live); (2) zero-padded
14-digit forms mis-anchored to a wrong first-9 SIREN while the
crosswalk itself demotes the strip==9 shape to E2; (3) a cancelled run
clobbered the reviewed plan with plan_groups 0 (R2 handler had the
same bug). **HARDENING LANDED (16572b8, gate 74 suites green):**
group-atomic pairwise wall over co-anchored candidates + 8-candidate
co-anchor cap; checksum_anchors mirrors the crosswalk's strip==9
no-anchor judgment; classify-phase cancel no longer records a plan
(both handlers, honest stop messages); plus deny-direction near-misses
from the refuted set — candidate mention evidence keyed under the
target's country (was silently unkeyable ⇒ evidence-less), legal-form
veto head-vs-head (satellite corroboration across a family conflict =
the cross-country twin shape), consortium veto over candidate
satellites, R2's vat prefix/country agreement in the target map,
blast radius counted over the FINAL plan only. **PRECISION REVIEW:
40/40 census-sample candidates adjudicated SAME-entity** (2 initial
suspicions — a generic 'cité administrative' name and one
multi-target row the merge already skips — both dropped by the
skeptic pass). Acceptance evidence for the wet run: census
reconciliation exact + exemplar gates deny-direction + 40/40 sample.

**R3 WET RUNS EXECUTED, PREVIEW-EXACT (2026-08-29 ~16:2x, rev
16572b8, jobs 456/457/458).** Hardened dry run recomputed plan 891 —
IDENTICAL to the pre-hardening plan, every new denial firing zero
times on the live pool (the defenses are real; the pool contains no
instance — the strongest acceptance signal). Capped wet (100) then
the remainder (791) under parity: **891 NULL-country orgs merged into
their checksum-anchored standing rows — 5,588 mentions, 9,860
parties, 123 winners repointed, 2,557 tenders touched — matching the
dry blast radius TO THE ROW.** Pool 4,816 → 3,925. org_merge_log
rule='r3' carries anchor+n2 evidence per merge. Journal clean, health
green. **THE ANCHORED+CORROBORATED SLICE OF STAGE 3 IS COMPLETE.**
Remaining Stage-3 material: the 8-digit name-decides slice (2,093,
next-slice design above), 880 anchored-uncorroborated (edge material,
Stage 4), 358 register-prefixed, 83 no-target, 14 multi-target
deliberate skips.

**8-DIGIT SLICE OPENED (2026-08-29 ~17:0x): real DK/SI checksums
instead of the blanket marker.** The census's "name corroboration
decides" turned out to under-sell what arithmetic can do: DK CVR and
SI davčna BOTH have real mod-11 checks — idgate simply never
implemented them. Validated on live corpus before landing: dk_cvr
(weights 2,7,6,5,4,3,2,1, full sum ≡ 0 mod 11) passes 380/400 of the
DK bucket and the 20 failures are visibly mis-filed foreign numbers
(a UK company number, a P-nummer shape — the checksum is a noise
filter, observed); si_davcna (weights 8,7,6,5,4,3,2, check 11−rem
with BOTH 10 and 11 → 0) passes 60/60 SI-prefixed VAT bodies — the
"10 = not issued" variant in circulation is WRONG for issued numbers
(rem-1 specimens like 11022680 are real; measured). Structural
finding: si_davcna and cz_ico share the weight vector and differ only
at rem 0, so SI/CZ-valid values co-anchor ~91% of the time → honest
multi-anchor skip; the rescue yield is FI-unique/DK-unique/CZ-unique
passes (Maintpartner's 20445111 → UNIQUE FI anchor, computed and
pinned in test). checksum_anchors' 8-digit arm now emits four real
probes, no marker; census + merge consume it by injection, no other
code change. Anchor-path ONLY — deliberately NOT wired into the
census gate/condemns (Stage-1 condemnation policy expansion is a
separate decision; noted for a future issue). First deploy (32c9cd6):
census re-run on the post-merge pool (3,925) decomposed the 8-digit
class into **407 anchored+corroborated / 987 uncorroborated / 293
no-target / 1,439 multi-scheme**; dry run recorded plan 395.

**PANEL ROUND (3 lenses, ~370k tokens): CONFIRMED ARITHMETIC DEFECT,
fixed before any wet.** Two lenses independently grounded the same
bug: si_davcna's `r >= 10 => 0` collapse also accepted prefix-rem-0
values, but rem-0 davčna numbers are NEVER ISSUED (stdnum's si/ddv
deliberately leaves check 11 unmatchable; jsvat guards total != 11; 0
of 107 live specimens are rem-0, p≈3.7e-5 under the defective rule) —
and since SI ≡ CZ at every rem ≥ 1, EVERY unique SI anchor was a
phantom of that class (measured: 0.75% of random 8-digit strings,
100% of unique-SI anchors in a 200k simulation; ~15 phantom rows in
the pool). Fixed: rem 0 → Fail (fi_ytunnus's match shape); the
unique-SI pathway is now structurally EMPTY and SI rescue correctly
waits for a corroboration-decides multi-anchor design. Panel also
verified dk_cvr exact vs stdnum (380/400 reproduced; zero
wrong-country unique anchors among the 20 DK-bucket failures),
proved the false-merge residual of real 8-digit anchors is the
TIGHTEST of all arms (16.1% spurious-unique for a foreign number vs
28.6%/17.9%/18.0% for the 9/10/11-digit arms), flagged the SK-IČO
exposure (SK shares CZ's arithmetic; 92.5% masked by SI/CZ
co-anchoring pre-fix — post-fix SK rem-0-check-1 values DO
unique-anchor CZ; the name-equality wall is the standing defense,
expected false merges well under one — accepted residual, same class
as FR/IT/BE), and caught three doc rots (fixed). Degenerates safe:
00000000 → triple co-anchor, repdigits anchor nowhere. rem-0 phantom
pinned in test (10000070 anchors nowhere).

**8-DIGIT SLICE WET, PREVIEW-EXACT (2026-08-29 ~17:5x, rev 1a34949,
jobs 461-464).** Post-fix census: 408 anchored+corroborated (motion
from the pre-fix 407 exactly as the panel predicted — phantom-SI
gone, a few co-anchors became legitimate uniques). Dry plan 396;
capped 100 + remainder 296 under parity: **396 more NULL-country orgs
merged — 578 mentions, 815 parties, 580 winners repointed, 293
tenders touched, matching the dry blast radius to the row.**
**MAINTPARTNER GATE MET**: NULL row 5276790 merged into FI standing
org 2476219 via its unique FI anchor (verified live). Pool overall:
4,816 → 3,529 (891 + 396 = 1,287 R3 merges total). Journal clean,
health green. Residual Stage-3 material: 987 anchored-uncorroborated
(edges), 296 no-target, 1,435 multi-scheme (incl. all genuine SI
values — rescue needs a corroboration-decides multi-anchor design),
441 unanchored, 358 register-prefixed, 14 multi-target.
Kind: capability (organization layer quality) — design
Relates to: 168 (measured landscape — the empirical input), 234 (provisional
collapse, done), 259 (repair shape + refold-invariance lesson), 307/ADR-0013
D4 (the multilingual satellite), B8 rules 1-7 (data-profile-2026-08.md §3),
CONTEXT.md:57-61 (the bar).

## 0. Posture, and where the auto-merge line sits

The bar: **a false merge corrupts served data; a missed merge is recoverable.**
Every automatic merge rests on identifier-grade evidence. Name evidence —
including cross-language equality from `organization_names` — does exactly two
jobs: candidate generation and corroboration. It never merges anything alone.

The measurements dictate the line:

- The same-country identifier space has **zero** shared-identifier groups
  ([M28] §3 — structurally guaranteed by the resolver key `(country, kind,
  identifier)`, so this is a consistency check, not a discovery). The measured
  false-split pool (8,420 groups / 18,027 rows, the 1.55% floor) is **entirely
  country-mismatch**: 3,769 NULL-country groups + 4,651 cross-country groups.
  Every sampled specimen was a true split of one entity (CNFPT FR/NULL,
  Philips Ibérica ES/FI/NULL, EBSCO GB/SE, Grand Port FR/GP).
- Name evidence cannot bound identity: legitimate single-entity variance
  reaches **947 distinct mention names** on one correctly-keyed org (Tribunal
  Administrativo ES); the 17.4% ≥2-names bound is mostly this benign class.
- The one measured large false-merge class is **placeholder identifiers**
  (`DE123456789` → 144 names on one org; bare `123456789` → 450). The first
  change must REMOVE merge keys, not add them: B8 rule 1 ships first.
- Same-name-same-country duplicate groups are huge (AT 46.9% / CZ 49.2% /
  PT 25.2% of country rows [M28] §4) and every sampled group was ONE entity
  split by a provisional row, a second register scheme, typo'd digits, or a
  name-derived junk identifier — rich candidate material, never merge grounds.

**The line: tiers E0-E2 may auto-merge; E3-E4 only write candidate edges.**

Number provenance (⊕ satellite-first's discipline): figures tagged [rates],
[profile], [168] are checked-in docs; figures tagged [M28] come from the
2026-08-28 probe session, which was security-flagged mid-run (a classifier
block on one command shape) — every [M28] number that becomes a gate constant
is **re-derived once, up front, in Stage 0** under docs/agents/prod-box-reads.md
discipline before any constant freezes (this also settles the cluster-cap
question: the three drafts read max identifier multiplicity as 8/16/32 — the
cap comes from the Stage-0 census, provisionally 8).

Satellite size, reconciled (was flagged as a discrepancy in all three drafts):
job 1321's counts line reported **38,706,271 REPLACE write-ops**; the table PK
`(org_id, lang)` collapses repeat mentions of the same org, and the probe's
disjoint 4M-id windows sum exactly to **4,403,039 rows over 4,306,297 orgs,
70,598 of them with ≥2 languages** [M28] §1. Both numbers are true; capacity
planning uses 4.4M; cross-language corroboration reach is bounded by 70,598
orgs (1.6% of satellite orgs — thinner than the pre-probe assumption, which
strengthens the case that cross-language equality is corroboration, not a
primary engine). Issue 307 carries the correction note.

## 1. Evidence taxonomy — the ladder

| Tier | Evidence | Action |
|---|---|---|
| E0 | Exact `(country, kind, identifier)` equality, identifier passes the v2 gate (§2.1) | Auto-merge (the existing resolver key, now gated) |
| E1 | Deterministic scheme canonicalization: two identifiers arithmetically the same registration under one country's rules (§3.1) | Auto-merge, minus the denial list (§3.2) |
| E2 | Country-impaired identifier equality (NULL-vs-known, garbage code, scheme-family variance like FR/GP) **plus name corroboration** (R3, §4.2) | Auto-merge under R3's full condition stack; else edge |
| E3 | Exact name-key equality alone (N2/N3), incl. cross-language via the satellite; provisional↔canonical name capture | **Candidate edge only. Never merges.** |
| E4 | Weaker name relations: legal-form-stripped equality, acronym↔expansion | Candidate edge only, lower score |

Cross-language equality is (a) a blocking key in the Stage-4 scan — how
zero-overlap translations (PostAuto AG / CarPostal SA; Bundesamt für
Statistik / Office fédéral de la statistique [M28] §6) become findable at
all — and (b) a corroboration source for R3 (the shared key may come from any
language row of either org). Never sole merge grounds: the probe found 1
contaminated org in a sample of 10 (Bertin Exensor AB + Het Ministerie van
Defensie under org 23294544 [M28] §6 — pre-registered as the must-surface-
as-edge-only exemplar). Satellite name INEQUALITY never blocks a merge.

## 2. Normalization pipeline

### 2.1 Identifier plausibility gate v2 — B8 rule 1, first

Lives in `normalise_identifier` (ingest/src/project.rs:4511-4561), the single
choke point. Today `DE123456789` and ascending runs PASS — that is the hole.
Order inside the function:

1. Existing char filter + rejections (<4 chars, digit-less, all-zero,
   single-repeat) — unchanged.
2. NEW placeholder gate (rule 1): reject when the digit run is strictly
   ascending/descending or single-repeated-with-≤1-exception; when the value
   matches the placeholder lexicon seeded from the measured top-30 — which is
   ENTIRELY placeholder family: `NIMAT\d+`, `ORG-?0*\d+`, `BT501`,
   `^0+\d{1,2}$`, `^1234\d{0,6}$`, eForms notice-scoped technical ids [M28]
   §2; and when, after register-prefix stripping, the remainder contains a run
   of **≥4 consecutive letters** (⊕ restored to the documented B8 rule-4
   threshold; the evidence-ladder draft's ≥6 relaxation is dropped —
   `ORG0001MUNICPIODEALVAIZERE`, `AVDAFORAAREAPORTUGUESA1` [M28] §4 are the
   class). Over-blocking is guarded by the ⊕ Stage-1 acceptance band: the
   identifier-rejection rate on a replayed ingest window must stay within
   **[16%, 25%]** against the known 16% junk baseline [168].
3. Existing register-prefix table and VAT sniffer — unchanged.
4. NEW checksum layer, two classes. HARD (reject ⇒ provisional): only schemes
   passing the ⊕ **enablement census** — a checksum may hard-reject only after
   a dry-run census over that scheme's corpus population shows **≥97% pass
   rate** (a low rate means the algorithm is wrong or the scheme is dirty;
   stay soft either way). Candidates: DE VAT, FR SIREN/SIRET Luhn (soft-pass
   SIREN 356000000/La Poste), PL NIP+REGON, CZ IČO, ES NIF, IT P.IVA, BE
   mod-97, SE Luhn, FI Y-tunnus, PT NIF, NO orgnr, HR OIB, GR AFM, SK IČ-DPH.
   SOFT (recorded as evidence attribute, never rejects): everything the
   cross-walk study marked UNSURE (AT, IE, CY, MT, DK, NL post-2020, EE, LT,
   LV, SI, HU, BG, RO).

Refold-invariance (259): this gate is prevention only; the standing stock
needs the Stage-1 repair job (§5).

### 2.2 Country normalization — B8 rule 2

Extend `canonical_country` (project.rs:4495-4506): full alpha-3 fold, TED `1A`
→ NULL, deny-outside-mapped-set → NULL (the AFG-on-an-Icelandic-committee
class [M28] §3). Add a scheme→country-family compatibility table for R3: FR
schemes accept {FR, GP, RE, MQ, GF, YT, NC, PM, …} — Grand Port FR/GP is
coding variance, not two entities.

### 2.3 Name keys — never stored on `organizations`

`organizations.name_norm` (the 234 reuse key, `str::to_lowercase()` only) does
NOT change. The matcher computes its own keys:
- N1 = existing name_norm.
- N2 = N1 + NFKC + punctuation→space + whitespace collapse + quote/dash fold
  (8/10 satellite cross-lang pairs differ only in case/punct/suffix [M28] §6).
- N3 = N2 + legal-form CANONICALIZATION (map to a form token, don't strip;
  family table from the measured 27.3%-coverage suffix census [M28] §5, incl.
  spelled-out Polish/Hungarian forms and Cyrillic ЕООД; punctuation variance
  within families is heavy — s.r.o. / s. r. o. / spol. s r.o.). Stripping is
  used only for E4 edges. Diacritic folding deliberately excluded from v1.

⊕ Storage (from the crosswalk-first draft — the implementability judge found
the compute-on-the-fly alternative mechanically broken: N2-equal rows are NOT
adjacent in an N1-ordered walk, so one streamed pass cannot form N2 groups
without unbounded state): keys live in a **rebuildable scratch satellite**
`org_match_keys(org_id, key_kind, key)` with an index on (key_kind, key),
built by a windowed job from organizations + organization_names, rebuilt
wholesale whenever key semantics change. No entity-schema mutation, no 51M-row
re-backfill on a lexicon tweak, and Stage-4 blocking walks a real index.

## 3. Deterministic cross-walks (E1)

### 3.1 canonical_key(country, kind, value), by corpus payoff

FR (SIRET→SIREN truncate-9; VAT key arithmetic unifies FR-VAT↔SIREN; strip
left-zero padding first — 14-char zero-padded forms are live), PL (VAT↔NIP;
REGON-14→9), IT (VAT↔P.IVA; CF==P.IVA ⇒ same org, CF≠P.IVA ⇒ NO signal
either way), ES (VAT↔NIF), RO (VAT↔CUI, prefix-insensitive digits), CZ
(DIČ↔IČO), BE, SE (strip SE + trailing 01), DK, FI (hyphen), PT, HR, NL
(VAT↔RSIN legal entities only), HU (first-8), BG (VAT↔EIK), LV, SK
(IČ-DPH↔DIČ only — NEVER to IČO), NO, SI, GR (EL≡GR). Explicitly no
cross-walk: DE (court-scoped registers, zero arithmetic yield), AT, IE, LU,
CY, MT, EE (VAT and registrikood are separate series), LT.

**Zero-padding is demoted (exemplar-driven amendment, 2026-08-28):**
prefix-strips (RO/FI VAT, CZ DIČ→IČO at full length) delete redundant
information and stay E1; **padding a short digit string MANUFACTURES
information** and is live-measured unsafe: CZ `0002542` (a corrupted id on
Ministerstvo spravedlnosti, org 2364406) pads onto `00002542`, the REAL
IČO of Puncovní úřad (org 2905864) — a false merge R2 would have executed,
and the checksum cannot catch it because the collided value is genuinely
valid. So pad-derived canonical keys (CZ 7→8, FI legacy 6-digit, FR
left-zero forms, and any future pad rule) are **E2, not E1**: they merge
only under R3's full corroboration stack. The Ministerstvo financí pad
family (7-digit 1864578 + 8-digit 4229 + NULL-country 1924720, identical
names) still merges — corroborated; the Justice/Assay collision flags.

### 3.2 Denial list (checked before ANY E1/E2 auto-merge)

1. **VAT-group wall, ⊕ operationalized via mention evidence** (satellite-first
   draft; all three judges): before any R2/R3 merge, scan both orgs' mentions
   (`organization_mentions_org`, chunked IN 512) for raw_identifier/scheme
   register-number evidence; a shared VAT with CONFLICTING national register
   numbers ⇒ deny + `vat-group-suspect` edge. CZ `CZ699…` group prefixes
   always denied. Without this sourcing the wall cannot fire — org rows carry
   one identifier; the raw evidence lives in mentions.
2. Establishment ≠ entity: SIRET/NIC, REGON-14, DK P-nummer, BE establishment
   range, NO underenheter — truncate to the legal-unit key for matching; the
   fine id survives in immutable `raw_identifier`. GLN/IPA/DIR3/OIN/Leitweg
   are location/office/routing scoped: never merge keys.
3. Ephemeral entities: ES UTE NIFs (letter U), `ID_UTE_TEMP_PLATAFORMA` —
   never merge across procedures [profile §1.3].
4. Group-size cap: E1/E2 groups larger than the cap (Stage-0 census;
   provisionally 8) are refused and routed to edges — a big group is a
   placeholder family the lexicon missed.
5. ⊕ **Gate-failure poisons the cluster** (crosswalk-first): any member of an
   E1/E2 group whose identifier fails the v2 gate disqualifies the ENTIRE
   group from auto-merge; the group routes to the split/flag path.
6. ⊕ **Court-scoped register exclusion** (crosswalk-first; judges 1 and 3):
   court-scoped register prefixes (HRB/HRA/VR/GnR/PR, and FN where
   court-scoped) never satisfy R3's "register-prefixed form" alternative —
   the DE cross-court collision class must not ride R3 across a NULL-country
   boundary. The E0 key-scoping demotion for the standing stock stays a
   Stage-6 measurement item, but this R3 hole closes on day one.
7. ⊕ **Legal-form-contradiction veto (E3-neg,** satellite-first; all three
   judges): same N2 stem but conflicting legal-form families (from N3 tokens,
   computed on the fly) blocks an otherwise-eligible R2/R3 merge → edge. The
   Organschaft signature ("X GmbH" vs "X AG" sharing a VAT) costs only
   recoverable misses.

## 4. Candidate generation and decision rules

### 4.1 Blocks

- B-ID (Stages 2-3): preload all 1.16M identifier-bearing orgs (the exact
  resolver preload, canonical.rs:3729-3744), canonical keys in Rust, HashMap
  groups. No SQL joins, no new entity index.
- B-NAME + B-XLANG (Stage 4): walk `org_match_keys` by (key_kind, key)
  watermark — the scratch satellite makes N2/N3/cross-language grouping a
  real index walk (⊕ replaces the winner's broken on-the-fly plan). Emit E3
  edges for groups mixing provisional+canonical or multiple canonical rows.
- Generic-name stoplist: any name key blocking >20 orgs is skipped for edges
  (counted, not written) and disqualified as R3 corroboration unless the
  identifier hard-checksum-passes — "Gymnázium", "Centre hospitalier" [M28]
  must not corroborate weak ids.

### 4.2 Decision rules

- R1 (=E0): exact key equality post-gate. Unchanged code path.
- R2 (E1): same normalized country, canonical keys unify, both pass the v2
  gate, denial list clear (incl. ⊕ walls 5-7), group ≤ cap ⇒ auto-merge.
- R3 (E2): identical canonical key across a country impairment AND all of:
  v2 gate pass; ≥8 significant chars; hard-checksum pass OR register-prefixed
  form (⊕ court-scoped prefixes excluded); shared exact N2/N3 key between any
  name of A (primary or any satellite lang) and any name of B — stoplisted-
  generic keys require a hard-checksum pass; denial list clear. Anything
  failing ⇒ edge tagged with the failed condition.
- ⊕ EBSCO-class demotion (satellite-first's line; judges 1 and 2 — PL NIP is
  also 10 digits, so a bare Luhn-passing 10-digit id is not single-country
  evidence): cross-country pairs whose identifier is NOT scheme-anchored to a
  country ship **flag-first** (`r3-unanchored` edges); promotion to auto-merge
  only after a sampled-precision review of the edge class holds at 100%.
- Survivor policy: keep = the row whose country the scheme validates; tie →
  non-provisional over provisional → min id (234 precedent).
- ⊕ Corroboration provenance (crosswalk-first T7): every R3 merge records the
  exact corroborating row (org_id, lang, name_norm source) in its
  org_merge_log evidence JSON — so one contaminated satellite row (the org
  23294544 class) is findable and ONLY its merges unwound.

## 5. Stage-1 stock repair: placeholder splits

Prevention (§2.1) touches only new mentions. Standing placeholder orgs
(org 15176 et al.) are 1→N splits — the direction `repoint_org_references`
does not do — so: `repair-placeholder-orgs(batch, after, dry_run=TRUE)`,
PK-watermark walk in the 259 shape (canonical.rs:4677-4754 template);
Rust-side filter to identifier-bearing rows failing the injected v2 gate
(gate fn handed ingest→store, the supervisor.rs:2091 pattern); per condemned
org re-resolve each mention through the post-234 provisional path, rewrite
bindings, repoint party/bid_party/winner rows, delete the org, events per the
merge pattern. Dissolved orgs leave scope ⇒ restart-safe. This deliberate
binding rewrite is a REPAIR under the 259 precedent — R2/R3 merges never
re-route mentions; they repoint wholesale. Same job shape re-scoped serves
Stage 5 (NULL-country buckets) and the recovery path (§8).

## 6. Data structures

`org_candidate_edges(org_a, org_b, rule, tier, score, evidence JSON,
first_seen, last_seen, state, PK(org_a, org_b, rule)) STRICT` — idempotent
REPLACE-style refreshes; periodic stale-sweep; `state='approved'` reserves a
future human-gated merge path (out of scope). `org_merge_log(keep, loser,
rule, evidence, job_id, at, PK(loser, at)) STRICT` — every auto-merge
auditable and targetable for undo. ⊕ `org_match_keys(org_id, key_kind, key)`
scratch satellite + index (rebuildable; §2.3). No columns change on
`organizations`; E0 key and 234 reuse key untouched; satellites emit no
change events (canonical.rs:4908 precedent).

## 7. Jobs and walk shapes

All jobs: dry_run default TRUE, BEGIN IMMEDIATE per batch, ROLLBACK on error
(the poisoned-transaction trap), TRUNCATE checkpoint between write batches,
heavy_write_kind + STOPPABLE_KINDS registration, cancel between batches.
Windows ride PKs or named indexes with equality+range watermarks only; joins
Rust-side (the turso catalogue: 274 composite seek, 306 bare-rowid range and
equi-join-at-volume). Jobs: `repair-placeholder-orgs` (Stage 1),
`match-org-identifiers --r2/--r3` (Stages 2-3: one 1.16M preload, in-RAM
groups, WRITE_BATCH merge txns via repoint_org_references, merged groups
become singletons ⇒ restart-safe), `build-org-match-keys` (Stage 4 pre-step,
rebuildable), `scan-org-name-blocks` / `scan-org-xlang-blocks` (Stage 4,
edges only), `org-merge-health` (standing read-only tripwire walk).
⊕ Dry-run/live parity abort (crosswalk-first T4, with the implementability
judge's tolerance fix): every wet batch compares its counts to the recorded
dry-run plan; divergence beyond a small tolerance (concurrent ingest moves
the ground) ⇒ ROLLBACK + job abort + re-plan. Never force past a divergence.

## 8. Rollout stages

Exemplar sheet checked in as `300-exemplars.md` BEFORE implementation.

- **Stage 0 — baseline (read-only).** `org-merge-health`; ⊕ single up-front
  re-derivation of every [M28] gate constant under prod-box-reads discipline
  (cluster cap, stoplist threshold, pool splits, placeholder census,
  per-scheme checksum pass rates — the §2.1 enablement input); top-100
  multi-name allowlist frozen (Tribunal/947, ELEKTRO/789, Ministères/753,
  Gmina Rzeszów/588); ⊕ CNFPT satellite check: verify whether the
  acronym↔expansion pair actually exists in the satellite — if yes CNFPT is a
  must-MERGE exemplar, if no it is must-FLAG (an acceptance gate must never
  pressure corroboration to weaken).
- **Stage 1 — placeholder gate + split (B8 rule 1). First, by constraint and
  by value.** Gates: every registered placeholder exemplar rejected;
  known-good panel per HARD scheme passes; enablement census ≥97% per HARD
  scheme; ⊕ rejection-rate band [16%,25%] on a replayed window; dry-run
  condemns org 15176 + the 450-name org, zero allowlist members; post-run:
  zero identifier-bearing orgs fail the v2 gate; ≥6-names tail shrinks by the
  placeholder-keyed share.
- **Stage 2 — same-country canonicalization (R2/E1).** Resolver computes
  canonical keys pre-probe; `match-org-identifiers --r2`. Gates: positive
  exemplars (FI Y-tunnus/VAT, RO bare/prefixed, CZ zero-pad, FR SIRET/SIREN)
  merge; negatives (SK DIČ-vs-IČO, CZ699, any DE pair, any UTE) refused;
  ⊕ 100-sample precision review at 100% (satellite-first's bar — one false
  merge fails the stage); first prod run capped (10k) with tripwires green
  before uncapping. Post: false-split floor re-measured below 1.55%.
- **Stage 3 — country-impaired merges (R3/E2).** The measured 8,420-group
  pool. Gates: Maintpartner FI/NULL and Philips 3-way merge; Grand Port FR/GP
  merges; ⊕ EBSCO GB/SE routes to `r3-unanchored` edge (flag-first);
  Kærunefnd/AFG merges only with name corroboration; CNFPT per its Stage-0
  determination; ⊕ 100-sample precision at 100%; plan ≤ re-measured pool.
- **Stage 4 — match-keys build + candidate-edge scans (E3/E4). No entity
  writes** (asserted in tests: zero change events, no entity-table touches).
  Edge volumes within pre-estimated bounds (name-dup pool from AT/CZ/PT rates;
  cross-lang pool bounded by 70,598 multi-lang orgs); stoplist skip-counter
  shows the cap working; ⊕ the org-23294544 contamination exemplar surfaces
  as an edge and nothing else.
- **Stage 5 — NULL-country national-id bucket (B8 rule 5), AFTER Stage 3** so
  rescuable rows merge before residuals split. Census first (class size
  genuinely unmeasured). Prevention: `(None, national, v)` stops being a
  shared resolver bucket. Repair: Stage-1 job re-scoped, conservative (≥3
  distinct N2 names).
- **Stage 6 (deferred, unscheduled):** DE court-scope demotion for the
  standing E0 stock (measurement first); N3 keys feeding richer corroboration;
  any `r3-unanchored` promotion (needs the measured precision record).

## 9. Tripwires (standing from Stage 1) and recovery

1. Gate invariant: identifier-bearing orgs failing the v2 gate == 0 post-
   Stage-1; nonzero ⇒ an ungated write path exists. Red.
2. Distinct-name-count guard (the bad-merge catcher — the rule that would
   have caught DE123456789 on day one): weekly recompute of distinct N2
   mention names per identifier-bearing org. Alerts: any org created after
   Stage 1 entering the ≥6 tail; ⊕ any org gaining ≥20 names week-over-week
   (growth-rate framing — an absolute cap is impossible, legitimate variance
   reaches 947); tail growth >5% over baseline; any new top-100 entrant
   absent from the allowlist.
3. Merge-rate guard: `organization removed` events/day; per-run caps on first
   prod runs.
4. Group-size refusals counter: a spike = a new placeholder family.
5. Provisional-share drift: baseline 90.8%; >1 point unscheduled ⇒ resolver
   regression.
6. Edge-store monotone-growth alarm.
7. ⊕ Refold canary (satellite-first T5; all three judges): after each stage,
   a sampled refold must rewrite ZERO mention bindings outside the split
   job's recorded scope — the standing proof that refold-invariance (259)
   survived the matcher.

Recovery: every auto-merge logged in org_merge_log with rule + evidence
(⊕ incl. corroboration provenance); a bad merge is undone by the Stage-1
dissolve mechanism scoped to the logged keep org, then Stages 2-3 re-merge
the good parts. Bounded, per-org, consistent with "recovery from a bad merge
is re-projection of mentions".

## 10. What this design will NOT do

No name-similarity auto-merges at any threshold, ever (no edit distance,
token ratios, embeddings, phonetics — not even as v1 edge generators beyond
exact N2/N3/stripped keys). No provisional↔canonical capture on name equality
(the 234 line stands; the pair becomes an E3 edge). No registry/network
lookups (VIES, GLEIF, KvK) — offline evidence only. No DE register cross-walk
and no E0 court-scope fix yet (tripwired, Stage 6). No rewriting of
name_norm, the E0 key, or the 234 reuse key. No merge ever re-routes
mentions. No retroactive effect via refold. No review UI or edge-serving
surface (the store accumulates; consuming it is a separate issue). No
establishment/branch modeling. No corpus-wide single UPDATEs, SQL equi-joins
at volume, or unwatermarked scans. No LEI/DUNS enrichment. Person-id schemes
(SE personnummer, BG EGN, CZ/SK birth-number DIČ) noted in evidence
attributes for a future policy.

## 11. Why this is safe, in three sentences

The only merge keys added are arithmetic identities and checksum-validated
equalities over schemes whose error modes were enumerated and denied (VAT
groups via mention evidence, establishments, UTEs, court-scoped registers,
unanchored cross-country ids), applied to a pool whose sampled specimens are
uniformly true splits — while the one measured large false-merge class
(placeholders) is removed FIRST. Name evidence — legitimate variance up to
947 names per entity — is structurally confined to candidate generation and
corroboration by the tier table, not by discipline. Every automatic action is
dry-run-first, exemplar-gated, capped, logged with provenance, parity-checked
against its own dry-run, reversible through an existing repair shape, and
watched by tripwires that include a refold canary and the distinct-name
growth guard.
