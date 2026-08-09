# Research summary and consolidated open-item register

Compiled 2026-07-19 by the completeness review. Index of all research docs,
then every open item across all docs plus the lead's running gap register,
deduplicated and classified. `turso-scale.md` still carries TODO placeholders
(its agent is finishing); those are listed as known-pending, not re-litigated.

---

## 1. Document index

**api-layer.md** — Implementation patterns for the public API on the Dioxus
0.7.9 / axum 0.8.9 lock. Establishes: public API as plain axum merged into the
dioxus router (server functions only for dashboard RPC); SSE via
DB-as-log + `watch` doorbell with a gapless snapshot-then-diff protocol; one
global u64 cursor (append-only `changes` table, written transactionally with
canonical writes, opaque string externally); webhooks as Postgres-slot-style
consumers of the same log with Standard-Webhooks signatures; argon2id
passwords, `tdb_`-prefixed SHA-256-stored tokens, DB-backed sessions;
governor/tower_governor rate limiting; layered SQL-endpoint hardening.
Verified crate-version compatibility table. Recommends sqlparser 0.62 for the
SQL gate (superseded — see §3).

**eforms-data-model.md** — The eForms SDK dissected for the canonical schema.
Headline numbers (SDK 1.15.0): 1256 fields / 357 distinct BT ids / 323 nodes /
96 business entities; 773 net content-bearing fields (480 attribute fields + 3
OPA are plumbing); 51 notice subtypes; 275 codelists; 44 id-ref fields.
Establishes the entity graph (Procedure-Lot/LotsGroup/Part-LotResult-
Bid-TenderingParty-Contract-Organization/Touchpoint/UBO-Review), identifier
semantics (BT-04 procedure UUID as Tender key; LOT-XXXX procedure-scoped;
ORG-/TEN-/CON- notice-local, never canonical keys), SDK version drift
1.0->1.15 (25 field ids removed, xpaths move -> per-(version, profile)
completeness checklists), the withheld-field (BT-195..198) satellite pattern,
multilingual satellite tables, and the documented pointless-BT whitelist.
Debunks a hallucinated third-party overview (~500 BTs, invented xpaths).

**ted-access-channels.md** — How to get TED data. Establishes: bulk tar.gz
packages (daily+monthly, 1993->today, anonymous, no ETags/checksums) as the
only ingestion path; Search API v3 floor is July 2016 (cross-check +
gap-filler only); three format eras (tagged text 1993-2010, TED_EXPORT
R2.0.7/8/9 2011-2024, eForms late-2022->) with per-file dispatch in mixed
2023-24 packages; daily cadence Mon-Fri final by 09:30 CET is the latency
floor. Sizes: full history ~188 GB compressed (does not fit 75 GB), XML era
~36 GB (fits), EN-only text era +8 GB; ~12.9 M notices total, ~9.0 M in the
XML era, ~870 k/yr now, eForms raw growing ~5 GB/yr. TED reuse terms: free
with attribution + transformation notice.

**german-portals.md** (+ **german-portals-state-commercial-survey.md**) —
Source #2 selection. Establishes: oeffentlichevergabe.de (DOE/BKMS) is the
only qualifying German portal — anonymous bulk API (eForms-DE XML, OCDS,
CSV), complete history back to 2022-12, CC0, strictly T+1. ~20-25 k notice
versions/month; ~40-45 % is below-threshold sdk-0.1 volume that never
reaches TED (genuine net-new coverage). ADR-0003's cross-reference verified
exact: identical notice UUIDs and BT-04 procedure UUIDs on both sides (15/15
systematic check). eForms-DE profile mechanics: KoSIT SDK-DE fork, DEX
extension fields, national subtypes E1-E4, 14 national codelists. The
sub-survey confirms every state/commercial portal fails (no history, no bulk
access, or paywalled) and they feed the DOE anyway.

**eforms-de-profile.md** — What DOE support concretely requires. Establishes:
ten concurrent CustomizationIDs in the archive (eforms-de 1.0-2.1,
eforms-sdk 0.1-1.13); ProfileID unreliable -> importer needs its own DE->EU
base-version fallback table; SDK-DE 1.14.4 vs EU 1.14.2 delta is exactly +4
fields / +4 nodes / 0 removed, xpaths identical, plus constraint tightening
and 14 codelists; DEX fields spec-only until VergStatVO H2 2026. The
`eforms-sdk-0.1` dialect is a permanent ~40 %-of-volume profile (no spec
exists; 362-path empirical inventory is the checklist), with two producer
channels — numeric (~91 %, no procedure id at all -> unmergeable by design)
and uuid. DOE<->TED pair diff: DOE is the richer original (national codes
downconverted toward TED), TED owns OJS publication identity -> per-field-class
merge precedence.

**ted-empirical-checks.md** — Identifier assumptions tested on 551 real chain
notices + 10 743 daily-scan notices. Verdicts: BT-04 as Tender key HOLDS
(551/551 XML==API, 100 % presence on competition/result/veat; planning/BRIN
lack it -> link by reference); LOT-id stability HOLDS except FA/DPS round-local
relabeling (all violations HU) -> defensive import rule, no round entity
needed; (BT-701, BT-757) NOT reliable identity — key Notices by publication
number; version 01 may never exist, versions have gaps, tranche CANs are
additive not superseding; BT-13716 present in only 58 % of change notices
(ids trustworthy when present) -> change scoping must be diff-based (ADR-0001
amended); withheld fields real but rare (~2 %), BT-198 reveal never observed
-> satellite table, no reveal machinery; SDK 2.0 threatens fields.json-shaped
tooling more than notice structure.

**ted-legacy-mapping.md** — R2.0.7/8/9 into the canonical model. Establishes:
legacy notices chain via OJ notice numbers (REF_NOTICE) — transitive
union-find edges; corrigenda/modifications chain at 97-100 %, awards at
71-74 % (~17 % genuinely dropped links -> dashboard metric); chains break
forward at the eForms boundary (two unlinked Tenders for straddling
procedures). Canonical core has high legacy fill rates (deadline/CPV 100 %,
award value 91 %); roughly half the 357 BTs have a legacy source (unverified
estimate); ~23 legacy elements have NO eForms equivalent -> legacy-only
canonical satellites needed. F14 gives typed machine-applicable diffs; OTH_NOT
and text-era corrigenda are prose version events. Organization IDs: ~50 %
NATIONALID fill, 16 % junk, no scheme attribute -> normalization + plausibility
gate; 2011-2013 has zero. Defines the four era profiles (text / r208 / r209 /
eforms) with per-profile mapped-or-ignored checklists, era-scoped quarantine,
and the re-worded completeness promise (its §8.3).

**turso-capabilities.md** — What turso 0.7.0 can do, hands-on. Establishes:
STRICT/FK/CHECK/views/triggers/CTE/JSON/UPSERT/RETURNING/partial+expression
indexes all work; NO recursive CTEs, NO rank/lag/lead/frames, NO FTS5 (native
tantivy FTS exists but poisons sqlite3-CLI readability of the file), no custom
collations (-> shadow columns), no STORED generated columns. `query_only` is
real but PRAGMA-escapable -> app-level single-SELECT allow-list is the primary
wall; timeout-by-drop works on file-backed DBs. foreign_keys defaults OFF,
per connection. Concurrent readers genuinely parallelise. File format
bidirectionally compatible with sqlite3 (modulo native FTS).

**turso-scale.md** (in progress) — 10 GB on the production VPS, release
builds, both 0.7.0-pre.10 and 0.7.0 final (performance-identical). Bulk load
~11.3 k text-heavy rows/s (~38 MB/s) at ~140 MB RSS with FKs+5 indexes; open
47 ms cold; point queries sub-ms cold; full 10 GB scan 20.5 s at disk speed
with ~32 MB RSS; CREATE INDEX ~13 s/M rows; ANALYZE 11 s. **VACUUM INTO
OOM-kills the 8 GB box at 10 GB DB size** (both versions) and writes the
destination as a WAL pair; integrity_check passed on a known-bad backup ->
verify by row counts. The "pre.10 pin" was never a pin — the lock already
resolves 0.7.0 final; bump to `=0.7.0`. Pending TODOs: checkpoint+copy backup
numbers, load-then-index comparison, crash-loop results, implications/open
questions.

---

## 2. Consolidated open-item register

Sources: the nine research docs' own open questions, CONTEXT.md flagged
ambiguities, ADR consequences, the lead's running gap register, and new gaps
found by this review (marked **[new]**). Deduplicated; each item cites its
source doc(s).

### A. Needs research before planning

**Blocking** (answering these later risks a pivot or rework):

- **A1 [resolved — not applicable].** A GDPR/personal-data concern was raised
  here and researched; Lennart's lawyer assessed GDPR as not relevant for
  this dataset (public business data). The research doc was removed and no
  privacy-driven schema hooks, exposure tiers, or redaction mechanisms are
  needed. The AGPL §13 source-offer obligation (unrelated to privacy) is
  kept: the running server must link its source — API root + dashboard
  footer.
- **A2 [new] Real-data DB-size estimate and the 75 GB disk arithmetic.** No
  document adds up the whole box: raw archive (36-44 GB compressed) + Notice
  parsed layer + versioned canonical layer + indexes + quarantine + WAL +
  local backup staging (checkpoint+copy needs ~another full DB size on the
  same disk). ted-access-channels itself warns the DB layer is "plausibly the
  same order of magnitude as the uncompressed source" — which is 250-350 GB
  and would sink the box; turso-scale's 40 GB working assumption is synthetic
  (2.4 KB/notice), not derived from real parsed notices (multilingual
  TRANSLATION forms, per-field satellites and version history could multiply
  it). Pilot-parse one real month into a draft schema, or produce a defensible
  bound, before schema/retention decisions are frozen. [ted-access-channels
  §6, turso-scale §5, register: off-box backups]
- **A3 (known-pending) turso-scale completion**: checkpoint+copy backup timing
  and maintenance window, load-then-index guidance, kill -9 crash-loop
  results, implications. The backup path and crash safety are load-bearing for
  ops planning. [turso-scale TODOs]

**Nice-to-have before planning** (cheap, reduce uncertainty; none should
change the architecture):

- A4. DOE rate limits — one email to
  support@datenservice-oeffentlicher-einkauf.de before the full backfill;
  same email can ask for the sdk-0.1 dialect spec (A6) and numeric-channel
  provenance. [german-portals, eforms-de-profile]
- A5. eForms-DE 1.0/1.1/1.2 spec/SDK artifacts (projekte.kosit.org Maven) to
  fix the checklist source for those versions. [eforms-de-profile]
- A6. Formal spec for the `eforms-sdk-0.1` dialect (likely only empirical;
  the 362-path inventory suffices as fallback). [eforms-de-profile]
- A7. More DOE<->TED conversion pairs, especially a CAN with award values;
  confirm DEX stripping once VergStatVO goes live. [eforms-de-profile]
- A8. Whether 2022-12-backlog sdk-0.1 above-threshold notices (13.6 k) have
  TED counterparts / are mergeable (likely not — numeric ids only).
  [eforms-de-profile]
- A9. Mechanical per-BT legacy tally (annex tables + converter XSLT) to
  replace the "roughly half" estimate, and the definitive legacy-only
  canonical field list from `ted-elements-not-convertible.md` + XSD sweep
  (~23 elements known; schema can reserve a satellite pattern meanwhile).
  Also the exact Regulation-Annex BT count for doc accuracy.
  [ted-legacy-mapping §5, eforms-data-model]
- A10 **[new]** XML parser crate evaluation + parse-throughput measurement.
  turso-scale asserts import will be "parser-bound" with zero parsing numbers;
  full rebuild-from-archive time (the stated recovery path for bad merges)
  depends entirely on it. Also needed: namespace-aware matching (the ns2..ns9
  prefix bug class already bit one scan), Latin-1 decoding for the text era.
  A day of probing quick-xml (or similar) on real packages.
  [eforms-de-profile §6, turso-scale §5, ted-legacy-mapping]
- A11. R2.0.7 XSD hunt (web.archive.org / SIMAP) or empirical 2011-2013 delta
  sweep. [ted-legacy-mapping]
- A12. Full-year legacy linkage sweep (per-year/per-country REF_NOTICE
  curves). [ted-legacy-mapping]
- A13. R2.0.9 S01->S05 XSD revision diffs. [ted-legacy-mapping]
- A14. Utilities/concession/social/defence form dissection (F04-F13, F21-F25)
  — can also happen when writing those mappings. [ted-legacy-mapping]
- A15. FA/DPS round-local lot ids: HU-specific or broader? (Defensive rule
  already designed either way.) [ted-empirical-checks]
- A16. The 18 % CN-less CAN procedures: direct-award vs pre-2016/pre-eForms
  competition. [ted-empirical-checks]
- A17. BT-1252 formats at bulk scale (n=8 so far). [ted-empirical-checks]
- A18. Text-era format semantics (field codes, meta-vs-text fidelity, `OL:`
  presence) — conditional on backfill decision C1/C4. [ted-access-channels,
  ted-legacy-mapping]
- A19 **[new]** Tiny turso probe: `AUTOINCREMENT` (the cursor design leans on
  it; it is absent from the verified feature matrix) plus the 0.7.0 additions
  (sequences, REINDEX, WITHIN GROUP) the capabilities doc predates.
  [api-layer §3, turso-capabilities §1, turso-scale §4]
- A20. data.europa.eu CSV as independent cross-check of per-year counts; exact
  Search-API index floor date. [ted-access-channels]

### B. Resolvable during planning/design

- B1 **[new]** **Backfill <-> change-cursor <-> versioned-canonical
  interplay.** The recommended backfill order is newest->oldest, notices
  arrive out-of-order and years late, yet the canonical layer is versioned
  with validity ranges and the change cursor is the monotonic spine of SSE,
  poll and webhooks. Undesigned: whether backfill emits change events
  (9 M-event floods to subscribers), how out-of-order ingestion inserts
  versions mid-history, whether re-projection (the ADR-0003 merge-undo
  mechanism) renumbers cursors, and what "deterministically rebuildable"
  means for cursor stability. Must be settled before the changes-table schema
  is frozen. [api-layer §3, ted-access-channels §5, ADR-0001, ADR-0003]
- B2 **[new]** Filtered-SSE diff semantics. "SSE on every endpoint" means
  per-subscription filters; emitting correct added/changed/removed requires
  evaluating the filter against old AND new state (an entity leaving a
  filtered set is `removed` for that subscriber while merely `changed`
  globally). api-layer's open question covers only the snapshot side. Needs
  the query-parameter design (B5) first; per-subscriber evaluation cost on
  one box bounds how rich filters can be. [api-layer §2/§7 open q.3, CONTEXT]
- B3. Physical shape of the parsed Notice layer (per-profile relational
  mirror vs generic field store) — drives A2's size estimate. [ADR-0001,
  eforms-data-model]
- B4. Temporal model for canonical versions: which timestamp orders validity
  (publication date vs ingestion time), given late-arriving and gap-ridden
  version sequences. [ADR-0001, ted-empirical-checks §2]
- B5. REST query-parameter design (filtering, pagination, ETags). [api-layer]
- B6. utoipa/OpenAPI in v1 or deferred. [api-layer]
- B7. SQL statement gate: turso_parser (lead's intent; already a transitive
  dep) vs sqlparser — settle in an ADR and run the corpus test of intended
  demo queries against the chosen parser. [api-layer §7, turso-capabilities
  §2, register]
- B8. Organization-ID normalization + plausibility gate (uppercase, strip
  spaces, digit requirement, vat/national tagging, (country, id) scoping).
  [ted-legacy-mapping §6, CONTEXT]
- B9. Era coverage declarations (static per-profile field-availability table
  feeding dashboard docs + completeness tests). [ted-legacy-mapping §8.3]
- B10. TLS/HTTP2 termination and proxy choice (nginx vs direct rustls) —
  affects SSE connection limits, X-Accel-Buffering, SmartIpKeyExtractor
  safety; ties into C23/C24. [api-layer §2/§6, register]
- B11. Store-crate concurrency shape: one writer task, N reader connections,
  per-connection pragmas (foreign_keys=ON!). [turso-capabilities §3/§7]
- B12. Withheld-field satellite table design (recommendation settled).
  [eforms-data-model §3.6, ted-empirical-checks §5]
- B13. National-extension satellite pattern (DEX today, any-country
  tomorrow). [german-portals §4, eforms-de-profile]
- B14. Change-log row + event schema finalization (field names are
  proposals). [api-layer §3]
- B15. Scraper observability metrics design. [register]
- B16. VPS dev/prod layout conventions. [register]
- B17. Quarantine reprocessing workflow (reprocess-after-fix, dashboard
  surfacing, per-profile counts). [ADR-0004, ted-legacy-mapping §8.2]

### C. User decisions

**Scope & completeness**

- C1. **Backfill depth**: (a) eForms-only 2023-> (~11 GB raw), (b) XML era
  2011-> (~36 GB, 9.0 M notices) — *recommended*, (c) full 1993-> (+8 GB
  EN-only). [ted-access-channels]
- C2. **Adopt the re-worded completeness promise**: "everything the source
  era publishes, nothing silently dropped" (per-profile checklists,
  era-scoped quarantine). Recommended: yes — the current wording is
  era-impossible. [ted-legacy-mapping §8.3, ted-access-channels, CONTEXT]
- C3. Text era: English-only vs all language editions (EN is often a
  translation — fidelity vs 20x size). [ted-access-channels]
- C4. Text-era ingestion depth: header-only profile now (makes XML-era chains
  terminate cleanly) vs defer entirely. [ted-legacy-mapping]
- C5. BRIN X01/X02 notices: in canonical v1 or explicitly out (not
  procurement; no BT-04). [eforms-data-model]
- C6. Reviews (REV) and E5 contract-completion: first-class canonical
  entities or notice-layer-only in v1. [eforms-data-model]
- C7. Parts: separate table vs Lots-with-kind (affects public API shape).
  [eforms-data-model]
- C8. sdk-0.1 numeric-channel notices (no procedure id, dirty CPVs, ~91 % of
  below-threshold volume): canonical Tender layer at launch or
  Notice-layer-only until identity is worked out. [eforms-de-profile]
- C22. Second German source (service.bund.de as deliberately-ugly stress
  Source): now or later. Recommended: later; no schema impact.
  [german-portals]

**Data representation**

- C9. Money representation in STRICT tables: TEXT decimal vs INTEGER minor
  units vs REAL (SQL-endpoint users will aggregate). [eforms-data-model,
  register]
- C10. Currency normalization for analytics (derived EUR-at-date column?
  which rate source?). [register]
- C11. Timezone policy: original-offset timestamps, UTC, or both.
  Research-recommended: store original offset + derived UTC.
  [eforms-data-model §3.2]
- C12. Language policy: codelist labels EN-only vs all 24; storage depth for
  legacy TRANSLATION form copies and 24-language ML_TITLES.
  [eforms-data-model §5, ted-legacy-mapping §2.6, register]
- C13. Cross-source merge precedence per field class. Research-recommended:
  DOE wins national code values + DEX satellites, TED wins OJS publication
  identity, anything else conflicting -> flag for review. [eforms-de-profile
  §5, CONTEXT, ADR-0003]

**API / product**

- C14. FTS strategy: native turso FTS in main file (loses sqlite3 escape
  hatch) vs separate search DB file vs LIKE-only v1. Research-recommended:
  separate file, or LIKE for v1. [turso-capabilities]
- C15. SQL-endpoint dialect promise: document as "turso SQL" (no recursive
  CTEs, partial window functions) or restrict further. [turso-capabilities]
- C16. Anonymous SSE: allowed (matches "basic endpoints unauthenticated")?
  Per-IP connection cap? [api-layer]
- C17. Change-log retention: promise "forever" or reserve pruning
  (410/reset path exists either way). [api-layer]
- C18. Webhook secret storage: plaintext in SQLite vs process-key encrypted.
  [api-layer]
- C19. SSRF policy for webhook URLs: block private ranges + require https,
  vs dev-time localhost convenience. [api-layer]
- C20. Rate-limit numbers (per-IP rps, SQL per-user quota, concurrency cap).
  [api-layer]
- C21 **[new]** Account recovery: username+password with no email means a
  lost password is unrecoverable. Accept (document it) or add an admin-reset
  path? [CONTEXT, api-layer §5]

**Operations & legal**

- C23. Deployment method: NixOS module vs nixos-anywhere onto the Ubuntu VPS
  (OS mismatch). [register]
- C24. Off-box backup target and public hostname. [register]
- C25. Dashboard data-quality panel: add "unchained awards" (~17 % of legacy
  awards) and "junk organization ids" (16 % measured) next to quarantine.
  Recommended: yes. [ted-legacy-mapping]
- C26 **[resolved]** GDPR stance: not applicable per Lennart's lawyer —
  public business data. No exposure restrictions or redaction mechanism.

### D. Implementation-time / operational tasks

- D1. Change `Cargo.toml` to `turso = "=0.7.0"` (the current requirement is
  not a real pin; the lock already resolves 0.7.0); rerun probes + crash loop
  on every future bump (SDK pinning policy generally — also eForms SDK,
  SDK-DE tags). [turso-scale §0/§4, register]
- D2. Importer discipline: never drop a mid-flight write future then COMMIT
  (0.7.0 poisons the tx); ROLLBACK/reset after any abandoned write.
  [turso-scale §4]
- D3. Store crate: stop serialising reads behind one mutex; per-connection
  `foreign_keys=ON`, `busy_timeout`, `synchronous=NORMAL`.
  [turso-capabilities §3/Implications]
- D4. Package-immutability probes: monthly re-hash of old TED packages; DOE
  monthly-export immutability after month end. [ted-access-channels,
  german-portals]
- D5. Periodic BT-198 reveal recheck (`51_republication.py`) once ingestion
  is live. [ted-empirical-checks]
- D6. Nightly integrity check: Search-API day count == daily-package file
  count; per-year ground-truth table feeds the coverage dashboard.
  [ted-access-channels]
- D7. Keep the mirrored XSD/spec artifacts safe (currently only on the VPS;
  archive pages have finite lifetimes) — consider committing or off-box
  copying `/opt/tender-db/ted-xsd/`, SDK clones, sample inventories.
  [ted-legacy-mapping, eforms-de-profile]
- D8. Watch upstream: turso (interrupt(), MVCC, window fns, FTS
  stabilisation, CDC pragma), eForms SDK 2.0 metadata shape (threatens the
  fields.json checklist tooling), next eForms-DE major, TED URL-pattern
  changes (keep URL templates in config, not code). [turso-capabilities,
  ted-empirical-checks §6, ted-access-channels §8]
- D9. Backup verification = sqlite3 `integrity_check` AND row-count
  comparison vs source (integrity_check alone passed on a known-bad backup);
  keep one non-turso backup path in the runbook. [turso-scale §1/§5]
- D10. Fetcher: release-calendar-driven scheduling, fetch after 09:30 CET,
  retry-404 with backoff, hash-keyed idempotency, <=3 concurrent downloads,
  long jobs in tmux on the VPS. [ted-access-channels §8, CONTEXT]
- D11 **[new]** License plumbing: AGPL §13 source-offer link on
  dashboard/API; TED attribution + "data transformed" notice (footer + API
  `source` field); mirror DOE's liability note in our terms.
  [ted-access-channels §7, german-portals §7, CONTEXT]
- D12. Disk monitoring from day one (headroom is years, not decades).
  [ted-access-channels §6]
- D13. Auth implementation details: argon2 in `spawn_blocking`, token
  SHA-256 storage, cookie flags, Standard-Webhooks signing (~20 lines, no
  extra dep). [api-layer]
- D14. sdk-0.1 path-inventory maintenance: unknown path -> quarantine +
  extend the committed fixture. [eforms-de-profile]
- D15. Era-profile build order: eforms -> r209 (F02/F03/F14/F20 first) ->
  r208 -> text. [ted-legacy-mapping §8.4]

---

## 3. Cross-document corrections to be aware of

- **turso-capabilities.md is partially superseded by turso-scale.md**: VACUUM
  INTO needs no experimental flag but OOMs at 10 GB (do NOT use it for
  backups, contrary to capabilities' Implications §6); CREATE INDEX is
  ~13 s/M rows in release builds (31 s/M was a debug artifact); the subject
  version "0.7.0-pre.10 pinned" was never actually pinned.
- **api-layer.md's sqlparser recommendation** is superseded by the
  turso_parser recipe in turso-capabilities.md (lead's intent); its open
  question 1 (statement interruption) is answered by turso-capabilities §2
  (timeout-by-drop works on file-backed DBs).
- **eforms-data-model.md §6.2's** clean "same BT-701, higher BT-757"
  correction model is superseded by ted-empirical-checks §2 (publication
  number is the only reliable per-publication key); CONTEXT.md is already
  amended.
- **ted-access-channels' "Era 3: eForms 2023-10-25->" heading** understates
  the start: eForms notices appear from Nov 2022 (voluntary), as its own
  measurements and eforms-data-model §7.1 show. Dispatch is per-file on the
  root element, so this is cosmetic.
- **german-portals.md Implications §5** says "no per-notice endpoint" while
  its own §2 verified an undocumented per-notice endpoint
  (`/api/notices/{uuid}`). Bulk ZIPs remain the ingestion path; the endpoint
  is for targeted re-fetch.

---

## 4. Status update (post-wave-3, 2026-07-19 evening)

Closes out the register above; written by the lead after the final research
round. The cross-document contradictions of §3 are all FIXED in the docs
themselves (commit b632540).

**Blocking items — all closed:**
- A-blocking GDPR/personal data → resolved as not applicable (Lennart's
  lawyer: public business data). The research doc was removed; only the
  AGPL §13 source-offer note survives (see A1).
- A-blocking disk budget → `pilot-sizing.md`: measured 22.3 KB/notice
  (multilingual texts = 86%); NO backfill scenario fits 75 GB; Hetzner
  volume required (~300 GB eForms-only, 500–750 GB XML era). Raw archive
  belongs on the filesystem, not in-DB.
- A2/A10/A19 (parser probe, sizing, AUTOINCREMENT) → `pilot-sizing.md`:
  roxmltree recommended (namespace-URI matching, all eras UTF-8); backfill
  wall-clock ~40 min (eForms) / ~2.2 h (XML era), writer-bound;
  AUTOINCREMENT monotonicity verified.
- turso-scale.md TODOs → filled: backup = checkpoint+copy (~20 s window at
  10 GB); crash torture 240/240 clean; pin bumped to `=0.7.0` (commit
  f233a6b); new 0.7.0 write-poisoning guardrail documented.

**New user decisions added to §2.C by wave 3:** Hetzner volume size,
multilingual-text language policy (the big cost lever),
account-recovery policy (no email ⇒ lost password = lost account?).

**Verdict update:** with the GDPR question resolved and sizing closed, the research phase meets
the bar set in §"verdict" — planning can start. The two design-phase
must-haves stand: backfill↔cursor↔versioning interplay and filtered-SSE
diff semantics are first-class planning agenda items.

---

## 5. Decision session results (2026-07-19, grilling session with Lennart)

All §2.C user decisions are now resolved:

- Backfill: full history 1993→; text era header-only, English-only fetched
  (model stays multilingual); parsed DB stores EN + original language.
- Storage: 500 GB Hetzner volume; raw archive on the filesystem; **no
  off-box backups for now** (accepted risk — everything rebuildable).
- Completeness promise: era-scoped "everything the source era publishes,
  nothing silently dropped" (ADR-0004 amendment).
- sdk-0.1 numeric-channel notices: single-notice Tenders.
- Merge precedence: per field class (ADR-0003 amendment).
- Representation: INTEGER cents + currency; UTC + original offset;
  codelist labels EN-only; Reviews notice-layer-only; Parts =
  Lots-with-kind; BRIN = minimal Tenders of distinct kind.
- API policy: anonymous SSE with ~5 streams/IP; change log kept forever
  with reserved pruning (reset path); webhook secrets plaintext,
  https-only + public-address-only delivery (dev escape); generous
  rate-limit posture; lost password = lost account.
- Deployment: Ubuntu + nix-built bundle under hardened systemd unit
  (ADR-0006); NixOS module + VM test kept as CI/distributable; hostname
  tenders.zebreus.click.
- CONTEXT.md stays unified (glossary + decisions in one file), per Lennart.

Open action items for Lennart: order/attach the 500 GB volume (or provide a
Hetzner API token). Everything else is planning-phase material (§2.B).

---

## 6. Post-launch: periodic drift audits (D8)

The D8 watch duty is discharged by dated audit docs, each re-checking the
claims above against upstream and correcting the research docs in place:

- **upstream-drift-2026-08.md** (2026-08-09) — first audit. Headlines: TED
  Search-API floor is a rolling `today − 10y` (July's "fixed July 2016" was
  the rolling edge, corrected in ted-access-channels.md); German DEX
  statistics fields went LIVE in the DÖE feed ~2026-08-06 and are already
  persisted by the importer (eforms-de-profile.md superseded note); EU SDK
  1.15.1 is a no-op for us; eForms-DE 2.1 acceptance ends 2026-12-02 →
  issue 165; turso 0.7.2 hygiene bump → issue 166 (corruption-fix exposure
  audited: none). Next audit ~2026-09.
