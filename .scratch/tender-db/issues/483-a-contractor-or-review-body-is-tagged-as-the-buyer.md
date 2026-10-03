# 483 — a contractor, review body or platform vendor sits in the buyer slot, and every buyer-based guard trusts it

Status: ready-for-agent — filed 2026-10-03 from issue 482's two-cluster precision read. The first unit is a census:
how often a notice's `Procedure-Buyer` mention is also its winner/tenderer, a review body or an eSender/platform, by
Source and subtype.
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

- **open** (2026-10-03): it lists `Ratio Web Sp. z o.o.` among the buyers.
- **done:** Instytut Adama Mickiewicza only.
