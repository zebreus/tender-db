# 300 — organization matcher: the tiered evidence engine (design)

Status: DESIGNED 2026-08-28 — full design below; build not started. Produced
at ultracode effort: three independent design drafts (identifier-maximalist /
evidence-ladder / satellite-first) from a shared research base (repo current-
state with file:line, fresh corpus probes, a 20-country identifier-scheme
cross-walk study), judged by a three-lens panel (false-merge safety,
implementability, evidence fidelity). The evidence-ladder draft won 2 of 3
lenses and is the skeleton; the panel's convergent grafts from the other two
are folded in below and marked ⊕. Stage 0 is the first buildable unit.
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

FR (SIRET→SIREN truncate-9; VAT key arithmetic unifies FR-VAT↔SIREN), PL
(VAT↔NIP; REGON-14→9), IT (VAT↔P.IVA; CF==P.IVA ⇒ same org, CF≠P.IVA ⇒ NO
signal either way), ES (VAT↔NIF), RO (VAT↔CUI, prefix-insensitive digits), CZ
(DIČ↔IČO + zero-pad-8), BE, SE (strip SE + trailing 01), DK, FI (hyphen;
legacy 6-digit pad), PT, HR, NL (VAT↔RSIN legal entities only), HU (first-8),
BG (VAT↔EIK), LV, SK (IČ-DPH↔DIČ only — NEVER to IČO), NO, SI, GR (EL≡GR).
Explicitly no cross-walk: DE (court-scoped registers, zero arithmetic yield),
AT, IE, LU, CY, MT, EE (VAT and registrikood are separate series), LT.

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
