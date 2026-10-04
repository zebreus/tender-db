# 483 — a contractor, review body or platform vendor sits in the buyer slot, and every buyer-based guard trusts it

Status: ready-for-agent — NEXT: deploy unit 2 (landed 2026-10-04, not deployed, below) through the gate, then
`refold-buyer-roles` dry → read the cohort → wet, and the daily re-derives the ~1,400 notices; then run Verify
(438807 serves POLREGIO). The swap waits on the org-level role index (deferred, below).
Kind: data correctness (parties)
Relates to: 482 (16 of 45 false splits in the 2-cluster read were role mis-tags), 481 unit 2b/2c (the buyer guard
reads `Procedure-Buyer`), 456 (mention binding), the served `parties[]`

## What is wrong

Award notices sometimes put the contractor, a review body or a platform vendor in the buyer role. Examples, all from
the 482 read (`.scratch/tender-db/issues/482-*.md`, the 2026-10-03 two-cluster section):

- **The winner tagged as `Procedure-Buyer`:**
  - 16698: the roles are SWAPPED. The CAN tags Ratio Web Sp. z o.o. as `Procedure-Buyer` and
    `Contract-Signatory`, and the real buyer Instytut Adama Mickiewicza as `Tenderer` (read live 2026-10-03).
  - 299165: the same swap (ISS HS Sp. z o.o. as buyer, Uniwersyteckie Centrum Kliniczne WUM as tenderer).
  - 439076: DOL-TRANS-TOUR.
- **A review body tagged as buyer:**
  - 533381: the Tribunal Catalán de Contratos.
  - 159306: ÚOHS.
  - Also KIO, a Vergabekammer and Förvaltningsrätten.
- **The procurement office or platform tagged as buyer:**
  - 438807: Urząd Zamówień Publicznych Departament Odwołań (also its `Lot-Mediator` and `Lot-ReviewInfo`); the
    real buyer POLREGIO S.A. holds only `Lot-AddInfo`, `Lot-TenderReceipt`, `LotResult-Financing`/`-Paying`.
  - 198229: European Dynamics.

The consequences:
- The served Tender names the wrong buyer. Supersession lets the newest notice win.
- The 481 buyer guard and the 482 hub gate read these as buyer-disjoint.
- Organization statistics count a contractor's "buying".

## First unit: census (dry)

For each notice with a `Procedure-Buyer` mention, flag the mention when any of these holds:
- (a) its organization is also a winner or tenderer on the same notice;
- (b) its organization or name matches a review-body pattern (BT-… review body role elsewhere, or a list of known
  review bodies);
- (c) it is the notice's eSender or docs provider.

Count by Source and subtype, with 30 samples per class. That decides the fix: demote the role at projection, or ignore
it in guards only.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/16698 | jq -c '[.parties[]? | select(.role|test("uyer")) | .organization_name] | unique'

- **open** (2026-10-03): `["Ratio Web Spółka z ograniczoną odpowiedzialnością"]`, the contractor as the only buyer.
- **done:** Instytut Adama Mickiewicza only.

## Unit 1: `buyer-role-census` — LANDED 2026-10-03 (not yet deployed)

A queued, read-only admin job (`crates/ingest/src/project/role_census.rs`; Spec `BuyerRoleCensus { stride }`;
report `buyer-role-census`; docs in `docs/operations.md`, section "`buyer-role-census`"). Modelled on
`procedure-key-census`: chunked keyset walk over parsed notices (20,000-id windows, 1,000 notices per read through
the new `Db::parsed_window`), stoppable between windows and chunks, no report stored on a stop.
- **Stride.** A full walk is ~3 ms/notice (482's measure), the better part of a day; the job reads one window in
  every `stride` (default 10, `"stride":1` for all). Counts are of the sample: multiply by `stride`.
- **Roles from the parse.** Every id-ref's role (eForms OPT-300/301 suffix, legacy element, sdk-0.1
  `ContractingParty`/`WinningParty`), nested Organizations folded onto the outer one, resolved organizations from
  `organization_mentions`.
- **Classes per buyer mention** (decisive ones make it not clean): `contractor-org` (same resolved org or same section
  as a winner/tenderer/contractor) and `contractor-name` (same folded name, no org match), `review-body-name` (curated
  `NAME_PATTERNS`, data rows: KIO, Vergabekammer, ÚOHS, Förvaltningsrätten, Tribunal Català, TACRC, TAR, …),
  `esender` (`Procedure-SProvider`), `platform-name` (European Dynamics, EU-Supply, Mercell, …) — decisive;
  `review-body-role` (the notice's own review-body role names it) and `docs-provider` — counted and sampled, NOT
  decisive (a buyer naming itself as review body or handing out its own documents is a mis-tag of that role or normal).
  Legacy `ADDRESS_REVIEW_INFO` is not a review body (the canonical fold merges it with `ADDRESS_REVIEW_BODY`; the
  census reads the element).
- **Decided: a buyer that tenders in another lot is still flagged** — a buyer is never its own supplier. Whether the
  notice also names a real buyer is `clean_buyer_left` / `no_clean_buyer`, not a dropped flag (pinned by
  `a_buyer_tendering_in_another_lot_is_flagged_but_a_real_buyer_is_left`).
- **Report:** `no_clean_buyer` (notices whose only buyers are all decisively flagged), per class
  `mentions`/`notices`/`no_clean_buyer`/`every_buyer` + 30 bottom-k samples (publication, subtype, flagged name, basis,
  other buyers with their flags), `read` per `source/subtype` and `cells` per `source/subtype/class`.
- Tests (as first landed; superseded by the review fixes below): the 482 shapes as unit tests (16698 same-section, 299165 two sections one org, DOL-TRANS-TOUR by name, six
  review bodies by name, European Dynamics as eSender, Gmina Olkusz as its own review body stays clean), a folded-corpus
  walk in `project_incremental.rs` (window 1 = window 1,000, stride 2, stop), and the supervisor wiring (default
  stride, stop stores nothing, issue-467 stack/poll budgets).

**How to decide after the run.** If `no_clean_buyer` is small against `decisively_flagged_notices` (a real buyer is
usually beside the flagged one), demoting the flagged mention at projection is safe and fixes the served `parties[]`.
If most flagged notices have no clean buyer left, demoting leaves them buyerless: make the guards ignore the flagged
mention (unknown, never decisive) and leave the served role, or recover the buyer from another notice of the procedure.

## Unit 1 review fixes — 2026-10-03 (not yet deployed)

An adversarial review of `9c2d2c7` found the census blind to the main shape in the 482 evidence. Verified live
(`/v1/tenders/16698`, `/438807`, `/299165`, `/159306`, `/533381`; 5 public reads) and fixed:
- **Blocker — the swap.** 16698 is not "the contractor is also a tenderer": Ratio Web is buyer + signatory and the
  Instytut Adama Mickiewicza is the tenderer; 299165 likewise. No contractor class could fire, so the census would
  have measured the class near zero on the notices that motivated it. New decisive class `buyer-tenderer-swap`
  (commercial legal form on the buyer, `COMMERCIAL_FORMS`; a public-shaped tenderer with no commercial form,
  `PUBLIC_STEMS`; a public-shaped buyer such as a municipal company is excluded) and its weak half
  `swap-legal-form` (non-decisive). Fixtures rebuilt from the real role layout; the old same-section shape is kept
  as the synthetic other-lot case.
- **Major — 438807.** New non-decisive `real-buyer-elsewhere` (the buyer holds no buyer-shaped role —
  TenderReceipt/TenderEval/AddInfo/Paying/Financing/Signatory/DocProvider — while a non-buyer, non-contractor does;
  the basis names the recoverable buyer) and `review-info-role` (ReviewInfo/Mediator, legacy
  `ADDRESS_REVIEW_INFO` — now read, not dropped). `departament odwolan` added. Role rows fixed: `ReviewOrg` (no
  such role) dropped; `Part-ReviewOrg`, `ReviewBody`, `Part-DocProvider`, the Part-/Lot- ReviewInfo/Mediator and
  the buyer-shaped roles added; a test holds every eForms row to `sdk/fields-*.json`.
- **Major — review-body names.** `review-body-name` is decisive only when corroborated (a review(-adjacent) role on
  the same organization, another non-review buyer, or `real-buyer-elsewhere`); otherwise
  `review-body-name-alone` (non-decisive). The report's `patterns` counts notices per class × list label.
- **Major — resolver fusions.** `contractor-org` split into `contractor-same-section` and
  `contractor-org-same-name` (decisive) and `contractor-org-other-name` (non-decisive: fusions, Eigenbetrieb
  in-house awards). Samples carry BT-105 `procedure_type`; `procedures` counts procedure type × class.
- **Minor — the poll budget never reached the chunk path.** A seeded poll now does: measured 420–430 KiB (the
  store/turso chain, like `run_project`'s), budget 456 KiB. The cancelled tripwire poll stays at 144 KiB.
- **Minor — docs.** operations.md's table, the 16698 description, the UZP note and "how to decide" corrected.

**Deferred** (ready-for-agent, not blocking the stride-10 run):
- Winner vs losing tenderer: `Tenderer` covers every tenderer; telling the winner needs the LotResult →
  LotTender → TenderingParty graph. Worth it only if `contractor-*` turns out large.
- The strongest swap signal — the tenderer's organization is a `Procedure-Buyer` on other notices — needs an
  org-level role index the store does not keep; the legal-form heuristic stands in for it. Read the swap samples.
- CZ ÚOHS is also the review body of its own procurements, so its own tenders corroborate themselves; read
  `patterns["review-body-name/CZ ÚOHS"]` samples before acting on that entry.
- No checkpoint/resume (`resume_after` ignored) and `parsed_window` reads every value table where only id-refs,
  names and the subtype/procedure type are needed. Add a resume cursor or narrow the read before any stride-1 run;
  stride 10 (~1 h at 2–3 ms/notice over ~235 windows) needs neither.

**How to decide (amended).** `no_clean_buyer` only means something with the swap and `real-buyer-elsewhere` classes
in view; read their samples first. The demote can recover a buyer from `real-buyer-elsewhere`'s basis or the swap's
tenderer; where neither exists, guard-only.


## Census read — 2026-10-03 (job 1942, stride 10, rev 9c7a950)

Report saved: `.scratch/tender-db/483-roles/census-1942.json`. 1,470,018 notices read (1,426,320 with a buyer,
1,563,471 buyer mentions); 225,557 flagged by any class, 3,151 decisively, **2,993 with no clean buyer**. Per class
(notices / no clean buyer), with what the 30 samples hold:

| class | notices | ncb | what the samples are | verdict |
|---|---|---|---|---|
| `esender` | 2,391 | 2,351 | buyers sending their own notices: Sprinkenhof GmbH, Senatsverwaltung für Wirtschaft, Bezirksamt Friedrichshain-Kreuzberg, Gmina Kraszewice, Gmina Cieszyn, Département de l'Aube | **not decisive** — 0/30 wrong buyers |
| `buyer-tenderer-swap` | 318 | 305 | 28/30 legacy. Company buyers awarding to a public institute: PKP PLK → Instytut Kolejnictwa, Hrvatske ceste → Institut IGH, ELES → Elektroinštitut Milan Vidmar, Dresdner Verkehrsbetriebe → TU Dresden, Slovenski državni gozdovi ×3. Real swaps: Bernard Gruppe ZT GmbH / Stadt Köln, Fliesen Görner GmbH / Landratsamt Ansbach (maybe Aon Sweden) | **not decisive** — ~2-3/30 |
| `contractor-org-same-name` | 135 | 134 | the authority in the CONTRACTOR slot: SPMS, Município de Pombal, Comune di Sarezzo, Kent County Council, Stadt Hilden ×3 (an F20 modification), Trafikverket, Ministry of Justice | **not decisive** — the buyer is right, the contractor mention is the mis-tag |
| `contractor-name` | 125 | 122 | the same: Gobierno Vasco (2002406: the legacy text's "Supplier(s): 1: Gobierno Vasco, A la atención de…" read live), Ayuntamiento de Granollers, Comune di Messina, Cardiff County Council | **not decisive** |
| `contractor-same-section` | 3 | 3 | Kirklees, Clackmannanshire, Cabinet Office (UK award updates) | **not decisive** |
| `review-body-name` | 197 | 96 | with `real-buyer-elsewhere`: Vergabekammer ×3 beside Staatliches Bauamt Erlangen-Nürnberg, Vergabekammer Nordbayern / Stadt Waldershof, High Court of Ireland / OPW, Förvaltningsrätten i Göteborg / Domstolsverket, DKOM / Ministrstvo za javno upravo — real mis-tags. With only "its review role": KIO ×5, ÚVO ×3, ÚOHS, KZK, DKOM, Markkinaoikeus, tribunaux administratifs ×4, PCRB Malta — review bodies buying for themselves. "another buyer": Tar Község (a Hungarian village matched by the bare `tar`) | **decisive only with another party's corroboration** (~8/30) |
| `platform-name` | 9 | 2 | European Dynamics S.A. ×8 (usually with a clean buyer beside it), Mercell | **decisive** |
| weak classes | | | `docs-provider` 49,842, `review-body-role` 45,204, `review-info-role` 75,342, `real-buyer-elsewhere` 61,588, `swap-legal-form` 18,529: the buyer naming itself in those blocks, as designed | stay counted |

**The 482 premise does not hold at corpus scale.** The roles the 482 read blamed (the swap, the court) are rare and,
except the court beside a recoverable buyer, not separable from legitimate buyers by anything the notice carries.
`no_clean_buyer` was 78% `esender` and 20% swap/contractor — false flags.

**Decision.**
- **Unit 1c (this commit): re-cut the decisive set.** `esender`, the three contractor classes and the swap are
  counted, not decisive. `review-body-name` is decisive only when ANOTHER party corroborates it
  (`real-buyer-elsewhere`, or another non-review buyer); its own review role makes it `review-body-name-alone`.
  The bare `tar` pattern is replaced by `tar <region>` for the 19 regions. Expected decisive at stride 10: ~60
  review-body notices + 9 platform ≈ 0.005% of notices.
- **Unit 2 (after the re-run): demote at projection**, not guard-only. At ~700 corpus notices the guards gain
  nothing measurable, while the served `parties[]` is wrong on exactly those notices; `real-buyer-elsewhere`'s
  basis names the buyer to promote (Staatliches Bauamt, OPW, Domstolsverket). Where no buyer is recoverable
  (a platform name alone, 2 of 9), leave the role ~~and only drop it from the guards~~. **Superseded by
  unit 2 (2026-10-04): one served/guard verdict — the platform-alone mention stays served AND in the
  guards (369/481/482); see "Unit 2 — landed", Decisions.**
- **The swap stays as data** until the org-level role index exists (the tenderer's organization is a
  `Procedure-Buyer` on other notices) — that signal, not legal forms, can tell Stadt Köln as a tenderer from TU
  Dresden as one. Deferred, ready-for-agent once something else needs the index.
- **The contractor-slot mis-tag** (the buyer named as its own winner, ~260 notices at stride 10 → ~2,600) is a
  real defect of the served awards, the other way round: filed as issue 484, not this unit — the
  winner mention on those notices is the one to doubt (a legacy text parse of "Supplier(s): 1: <buyer contact>").

## Re-run under unit 1c — 2026-10-04 (job 1943, stride 10, rev `dfbf93d`)

Report: `.scratch/tender-db/483-roles/census-1943.json`. Same 1,470,018 notices; **139 decisively flagged (was
3,151), 38 with no clean buyer (was 2,993)**: `review-body-name` 131 (36 no clean buyer), `platform-name` 9 (2).
The 30 review-body samples:
- **"real buyer elsewhere", no clean buyer (11)**: Vergabekammer ×4 / Staatliches Bauamt Erlangen-Nürnberg,
  Vergabekammer Nordbayern / Stadt Waldershof, High Court of Ireland / OPW, Förvaltningsrätten i Göteborg /
  Domstolsverket, Förvaltningsrätten i Falun / Gästrike återvinnare, Conseil d'État / its own direction, Raad van
  State / "Digitaal via TenderNed" (a platform label, not a buyer: the recovery needs a name check). Real mis-tags,
  recoverable except the last.
- **"another buyer", a clean buyer left (19)**: KIO ×3, tribunaux administratifs ×3, DKOM ×2, Raad van State ×3,
  Vergabekammer ×2, KZK, IUB, Markkinaoikeus, the Cyprus authority, PCRB, Verwaltungsgericht Wien — the review body
  referenced in the buyer slot beside the real buyer (it also holds the review role). Dropping it leaves the real
  buyer: safe.
- Patterns: Vergabekammer 36, tribunal administratif 28, KIO 23, Raad van State 10, DKOM 8, Verwaltungsgericht 5,
  High Court 4, TAR 3 (the bare-`tar` false hit is gone).
- `platform-name`: European Dynamics ×8 beside a clean buyer (7) or with Armagh council elsewhere; Mercell alone.

So the decisive set is precise (~30/30 by hand) and small (~1,400 notices corpus-wide). Unit 2 is the demote.

## Unit 2 — landed (not deployed), 2026-10-04

**The demote at projection.** `role_census::buyer_fix` turns the census's own verdicts (`judge`, the
decisive classes `review-body-name` and `platform-name`) into a per-notice fix — the single source of
truth, no parallel heuristic:
- a clean buyer mention left → the flagged mentions lose the buyer role (KIO / a tribunal
  administratif / European Dynamics beside the real buyer);
- none left → the party `real-buyer-elsewhere` names is promoted to the dialect's buyer role
  (`Procedure-Buyer`; legacy / sdk-0.1 `buyer`) and the flagged mentions dropped (438807 POLREGIO,
  the Staatliches Bauamt beside a Vergabekammer, Armagh council beside European Dynamics) — except a
  portal / platform label (new `NameList::Portal`: TenderNed, "digitaal via", Negometrix, achatpublic,
  the Spanish Plataforma de Contratación; plus the platform list), then nothing changes;
- nothing recoverable (Mercell alone, the Raad van State with only "Digitaal via TenderNed") → served
  as published.

The demoted party keeps its other roles. `Verdict` now carries the `real-buyer-elsewhere` party's
index and `Party` its section, so the promote names exactly the census basis's organization.

**Both readers, one verdict.** `NoticeState::read` applies it to the raw role references
(`apply_buyer_fix`, through the nested-org aliases) → the served `parties[]`, `v_tender_buyers`,
organization statistics. `buyer_side_mentions` applies it to the guards' buyer side → 369's buyer
key, 481's guard tokens and Phase-1 org sections (a promoted signatory moves to the buyers), 482's
hub key and the procedure-key census. Both fold paths go through these two functions, so full and
daily agree by construction (pinned by `absorb_and_compare`, one delta and — review fix — the CN and the CAN on different days in either order). The census itself still reads the raw
slot (`notice_parties`), so it keeps measuring the parse.

**Decisions.**
- **Parsed-side, no resolved organizations** (the census binds `organization_mentions`): the plan row
  is read before Phase 1, and the served role must agree with the guards. "Same organization" is then
  by folded name only; it matters only for `real-buyer-elsewhere` / "another buyer" against a
  differently named party.
- **Cheap gate:** only a notice whose BUYER party's name (any `ORG_NAME_FIELD_IDS` value of a party
  a buyer reference names, through nested halves) holds a review-body or platform pattern reads its
  mentions again for the verdict (review fix: was every party's name); `buyer_side_mentions`
  passes the mentions it already read.
- **The no-clean case promotes only when a non-portal party is recoverable;** otherwise nothing is
  dropped (the guards keep reading the flagged mention: unit 1c's "drop it from the guards only" for
  a platform alone was not taken — one served/guard verdict, and job 1943 has 1 such notice in the
  sample).
- `decides_the_fold`'s allowlist test now lists the census's role fields and their DE-1.x aliases
  (the demote makes them grouping inputs), with the fold-impact note.

**Re-projection: `refold-buyer-roles`** (`Spec::RefoldBuyerRoles { dry_run }`, dry by default,
stoppable, report `buyer-role-refold`; docs in `docs/operations.md`, "The buyer-role demote and
`refold-buyer-roles`"). Walks `organization_mentions` in 250,000-id strides
(`Db::mentions_named`, the name test in Rust) → notices whose own version SERVES such an organization
as buyer (`Db::notices_serving_buyer`) → notices whose parse gives a non-empty `buyer_fix`
(`role_census::buyer_role_refold_window`). Wet: `unmark_projected_by_ids` + `stamp_stale_for_notices`
(the issue-179 pair), so the next daily re-plans exactly those notices with the Tenders they sit in
(moving them between Tenders both ways — review fix test below). Cheaper than a projection
epoch (whole corpus) or a stride-1 census (a day): one pass of the mentions table plus ~2 seeks per
pattern-named mention and a parse of the few thousand candidates.

**Tests.**
- `role_census::tests::a_flagged_buyer_is_dropped_beside_a_clean_one_and_yields_to_a_recoverable_buyer`
  — 438807 (UZP → POLREGIO), KIO beside Gmina Żórawina, Vergabekammer → Staatliches Bauamt, Raad van
  State + "Digitaal via TenderNed" kept, European Dynamics beside QQI dropped, European Dynamics →
  Armagh promoted, Mercell kept, PCRB + European Dynamics beside Mater Dei both dropped; the
  non-decisive shapes untouched; the gate and the portal check.
- `role_census::tests::the_served_roles_and_the_guard_inputs_read_the_same_demote` — `NoticeState`
  roles, `buyer_mentions`, `GuardSide` sections and `buyer_key` agree (promote, drop, keep).
- `tests/project_incremental.rs::a_review_body_or_platform_in_the_buyer_slot_is_demoted_on_full_and_daily_folds`
  — full and daily folds byte-identical; served buyers per notice; 481's guard joins the
  Vergabekammer CAN to the Bauamt's CN by OPP-090 (`buyer_disjoint` 0; it is 1 with the fix disabled,
  checked); the cohort finds nothing on a unit-2 layer. (The simulated pre-unit-2 row is gone: see
  the review fixes.)
- `supervisor::tests::refold_buyer_roles_enqueues_dry_stores_its_report_and_a_stop_stores_none`, and
  the job in the cancellable list, the future-size gauges and the 144 KiB poll budget.

### Unit 2 review fixes — 2026-10-04 (not deployed)

- **Which party is promoted** (`role_census::promotable`): the first other party in a STRONG
  buyer-shaped role, skipping the eSender, a review body by role or by name (KIO's long name giving
  information beside a `KIO` buyer — the folded-name `same` misses it without resolved orgs), a
  portal/platform label, a nameless party, and a party whose only buyer-shaped role is the documents
  provider or the new `RoleKind::Financing` (`LotResult-Financing`, still buyer-shaped for the
  census's `real-buyer-elsewhere`, so the census classes are unchanged). A portal label FIRST no
  longer blocks the real buyer after it. Not done: ranking TenderReceipt above Paying (one more role
  bit; first-in-order among strong roles instead).
- **Platform name alone stays in the guards** — the Decision bullet above is struck through and
  marked superseded; `buyer_fix`'s doc and operations.md say so.
- **"Another buyer" corroboration without a review role: accepted risk, not tightened.** A court
  buying jointly beside a CPB/ministry loses its buyer role; none of job 1943's 19 such samples was
  one (all held the review role too). Requiring the review role would diverge from the validated
  census class and miss the shape where the review slot itself is empty (KIO typed into the buyer
  slot instead of the review slot).
- **Census vs projection target (orgs = None):** handled by `promotable`'s review-body-name skip;
  the dry `buyer-role-refold` report's `promoted` column, not census-1943, is the validation set
  (operations.md says to read it before going wet).
- **Plan marker:** the plan DDL creates `plan_buyer_demote`; `plan_is_complete` refuses a plan
  without it (481 2c's pattern), so no resume reuses pre-demote buyer keys / guard tokens. Pinned in
  `project_resume.rs::a_plan_from_before_the_buyer_guard_is_rebuilt_not_resumed` ("pre-483u2").
- **Cheaper gate:** `may_need_fix` tests only the names of the parties a buyer reference names
  (KIO as review body alone no longer passes); the name patterns are padded once
  (`PADDED_PATTERNS`, a `LazyLock`) instead of a `format!` per pattern per name. The verdict is
  still computed in both readers (one gate pass each for the ~99.99% that fail it).
- **A demoted mention that also signs** is dropped from the guards' signatories too; the dead
  `signatories.retain(promote)` line is gone (the if/else fold files a promoted section under the
  buyers only, and the test message says so).
- **Dry report counts:** the dry summary says `would re-queue <fixed>` (was always 0).
- **The re-queue moves notices between Tenders (the major finding):**
  `project_incremental.rs::refold_buyer_roles_moves_a_notice_between_tenders_on_the_daily` builds the
  pre-demote layer with the REAL fold (the stored parse gains the role reference that makes the fix
  non-empty only afterwards), on the full non-rebuild and the daily path side by side
  (`absorb_and_compare` at each step), and re-queues only the cohort: **join** — the Vergabekammer CAN
  refused by OPP-090 (`buyer_disjoint` 1, its own Tender) joins the CN, its old Tender retired;
  **split** — a CAN joined through a shared KIO buyer splits out when KIO is dropped, re-queuing the
  CAN only, the CN's Tender keeping the CN alone. Both equal a fresh rebuild by publication. So
  per-notice re-queue suffices; no whole-Tender re-queue.
- **More tests:** no promotion beside a clean buyer with a buyer-shaped third party; the portal-first
  order; the nameless / eSender / funding body / documents provider / review-body-named candidates;
  legacy (promoted as `buyer`), sdk-0.1 (`ContractingParty` dropped) and a nested inner-half buyer
  reference (`the_demote_reads_the_legacy_sdk01_and_nested_shapes`); the CN and CAN on different
  days in either order (`the_demote_holds_when_the_cn_and_its_can_arrive_on_different_days`); the
  supervisor test seeds a stride (dry finds 1 with a promote, wet re-queues 1 and stamps) and the
  seeded stride's poll budget (measured 360–390 KiB, set 416 KiB).
- **Dry run holds the heavy-write belt:** documented in operations.md (follows `requeue-uuid-hubs`).

## Deployed `ebb9f20` 2026-10-04; dry 1954 read → promote tightened (unit 2b)

`refold-buyer-roles` dry, job 1954 (report `.scratch/tender-db/483-roles/refold-dry-1954.json`): 8,372,154
pattern-named mentions → 12,278 notices serving one as buyer → **1,176 fixed** (981 drops beside a clean buyer, 195
promotes). The drops are the census's classes (KIO 153, DKOM 91, Raad van State 48, European Dynamics 43, High Court
38, Vergabekammer des Bundes 46, Klagenævnet 27, Verwaltungsgericht Wien 22, Markkinaoikeus 20, tribunaux
administratifs …). **The promotes were ~80 % right** (Domstolsverket, Staatliches Bauamt ×n, OPW, Klinikum Stuttgart,
Landeshauptstadt München Baureferat, Stadt Waldershof, ESID Metz, ČEZ, Adif …) but ~35 were wrong:
- **swapped notices**: KIO / Bundeskartellamt as buyer, the SUPPLIER as `Contract-Signatory` and the real buyer as
  `Tenderer` (ted:00703641-2024: CAMFIL POLSKA signing, Narodowe Centrum Badań Jądrowych "tenderer"; ted:00062161-2025:
  Wackler + 3B Dienstleistung signing, BImA "tenderer") — ~20 Polish medical suppliers (Roche, Sysmex, Radiometer,
  Sarstedt, Neuca, Arthrex), Braun GmbH, Günter Jacobi, ADPN, AL ALBA ESE;
- **tender agents** receiving tenders: PSI BV for the Raad van State, ATEUS Rechtsanwälte GmbH, CWPA Planning;
- one legacy free-text sentence as a name.

**Unit 2b (this commit):** `Contract-Signatory` is its own `RoleKind::Signatory` and `LotResult-Paying` its own
`RoleKind::Paying` (both still buyer-shaped for the census). `promotable` refuses the signatory alone, a company that
is not public-shaped unless it pays or finances (POLREGIO pays, 438807 still promotes it), and names of more than 16
words. Correct or unchanged: a swapped notice keeps its review-body buyer as published (the swap's real buyer is the
TENDERER, recoverable only with the org-level role index — deferred). NEXT: gate → deploy → dry again → read the
promotes → wet → the daily re-derives → Verify 438807.

## 2b deployed (`5b4a2a8`); dry 1955 → WET 1956 — 2026-10-04 ~03:30 UTC

Dry 1955 (`483-roles/refold-dry-1955.json`): **1,091 fixed** (was 1,176), **110 promotes** (was 195). By hand: ~105
right (Landeshauptstadt München Baureferat ×12, Staatliches Bauamt ×12, Gästrike återvinnare ×6, Stiftung Preußischer
Kulturbesitz ×4, Domstolsverket, OPW, Adif, ACOSS, Ville de Nice, GDDKiA, DPP, Osakidetza, the Irish schools behind
European Dynamics …). **Residue, ~5:** the legacy sentence "Inhoudelijke en procedurele aspecten…" (≤ 16 words after
all), PSI ×2 (tender agent of the Raad van State, no legal form in the name), Rembud Trzebinia, CWPA Planning and
Architecture. Each replaces an already-wrong buyer (the review body) with another wrong one — no correct buyer is
lost — so wet now; the residue is the deferred org-level role index's to fix (an agent is never a buyer elsewhere).
Wet: job 1956 → the next daily (07:35 UTC) re-derives. NEXT: read the daily's `issue-481` line and
`/v1/tenders/<438807's tender>` parties (Verify), then close unit 2.

## 2026-10-04 07:35 UTC daily (project 1965) re-derived the cohort — Verify

Project 1965: 1,660 notices (the 1,091 re-queued + the day's), `issue-481 … refused 1 (buyer-disjoint 1); largest
component: 8 key(s) at notice 25668433`. Served buyers now (`/v1/tenders/<id>` parties):
- 265758 (ted:00340810-2024): **Office of Public Works (OPW)** — was The High Court of Ireland;
- 187092 (ted:00316183-2026): **Armagh City, Banbridge and Craigavon Borough Council** — was European Dynamics;
- 790032 (ted:00067601-2024): **Gästrike Återvinnare** — was Förvaltningsrätten i Falun;
- 119479: its newest notice (00167972-2024) names "Vergabekammer Südbayern" alone with its review role — the
  non-decisive `review-body-name-alone` shape, served as published (by design).

**438807 cannot be met by name**: its notices' own mention is "Urząd Zamówień Publicznych" (the `Departament Odwołań`
suffix is organization 791's name from OTHER notices), and the bare UZP is deliberately not listed (it buys for
itself). The real signal there — the buyer is also the mediator / review-info body while POLREGIO receives tenders and
PAYS — is `real-buyer-elsewhere` + `review-info-role`, both non-decisive on the census (a CPB leaves paying to its
client). The Verify exhibit becomes 265758 (OPW); 438807 waits on a later unit if a precise rule for that shape is
measured. NEXT: close unit 2; the swap (16698) and 438807's shape stay open on the org-level role index.

## Verify (amended 2026-10-04)

    curl -s https://tenders.zebreus.click/v1/tenders/265758 | jq -c '[.parties[] | select(.role=="Procedure-Buyer") | .organization_name]'

- **done** (2026-10-04 07:4x UTC): `["Office of Public Works (OPW)"]`.
