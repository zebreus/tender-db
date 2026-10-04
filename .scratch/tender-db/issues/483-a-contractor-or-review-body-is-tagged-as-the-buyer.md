# 483 — a contractor, review body or platform vendor sits in the buyer slot, and every buyer-based guard trusts it

Status: ready-for-agent — NEXT: unit 2, demote at projection (job 1943 read 2026-10-04, below): drop a decisively
flagged buyer mention from the served buyer role when a clean buyer is left (95 of 139 at stride 10), and promote
`real-buyer-elsewhere`'s organization where none is (Staatliches Bauamt, OPW, Domstolsverket; ~30). A platform name
alone (Mercell, 1) keeps its role. The swap waits on the org-level role index (deferred, below).
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
  (a platform name alone, 2 of 9), leave the role and only drop it from the guards.
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
