# 483 — a contractor, review body or platform vendor sits in the buyer slot, and every buyer-based guard trusts it

Status: ready-for-agent — NEXT: deploy unit 1 (`buyer-role-census`, landed 2026-10-03, below) → run the census
(stride 10 first) → read 30 samples per class → choose the fix: demote the role at projection, or guard-only (the
481/482 guards ignore a flagged buyer mention).
Kind: data correctness (parties)
Relates to: 482 (16 of 45 false splits in the 2-cluster read were role mis-tags), 481 unit 2b/2c (the buyer guard
reads `Procedure-Buyer`), 456 (mention binding), the served `parties[]`

## What is wrong

Award notices sometimes put the contractor, a review body or a platform vendor in the buyer role. Examples, all from
the 482 read (`.scratch/tender-db/issues/482-*.md`, the 2026-10-03 two-cluster section):

- **The winner tagged as `Procedure-Buyer`:**
  - 16698: the CAN tags Ratio Web Sp. z o.o.; the real buyer is Instytut Adama Mickiewicza.
  - 299165: Naprzód Catering.
  - 439076: DOL-TRANS-TOUR.
- **A review body tagged as buyer:**
  - 533381: the Tribunal Catalán de Contratos.
  - 159306: ÚOHS.
  - Also KIO, a Vergabekammer and Förvaltningsrätten.
- **The procurement office or platform tagged as buyer:**
  - 438807: Urząd Zamówień Publicznych.
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
- Tests: the 482 shapes as unit tests (16698 same-section, 299165 two sections one org, DOL-TRANS-TOUR by name, six
  review bodies by name, European Dynamics as eSender, Gmina Olkusz as its own review body stays clean), a folded-corpus
  walk in `project_incremental.rs` (window 1 = window 1,000, stride 2, stop), and the supervisor wiring (default
  stride, stop stores nothing, issue-467 stack/poll budgets).

**How to decide after the run.** If `no_clean_buyer` is small against `decisively_flagged_notices` (a real buyer is
usually beside the flagged one), demoting the flagged mention at projection is safe and fixes the served `parties[]`.
If most flagged notices have no clean buyer left, demoting leaves them buyerless: make the guards ignore the flagged
mention (unknown, never decisive) and leave the served role, or recover the buyer from another notice of the procedure.

