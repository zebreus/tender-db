# 456 — a mention that publishes another organization's identifier binds to that organization, whatever its name says

Status: ready-for-agent — DESIGN 3 DEPLOYED 2026-10-01 at `b26cf3a` (built `0850733`, review fixes `026819f`): every party serves the mention's own published name as `mention_name` beside the org head. UKRI's tender 8751605 now reads `[5718658,"Sellafield Ltd","Schneider Electric"]`, so a reader sees what the notice said. NEXT: the measurement unit (the census of identifier-bound mentions whose name agrees with none of their org's names), then designs 1 and 2.
Was status: ready-for-agent — filed 2026-10-01 from 448's chunk-7 delta (job 1787). The first unit is a measurement: size the
class across the GB PPON- and COH-bound mentions before designing anything. The design questions are below.
Kind: data quality (organization identity)
Relates to: 327 (an Austrian buyer's GLN in the supplier block, the same source-side shape), 448 (e2-altid, where it
surfaced), 452 (wrong numbers, withheld per org), 206 (the override layer, deferred until a real single-notice need)

## What happens

The resolver binds a mention by its identifier first. A name is consulted only where no identifier exists, by the
altid alias (`names_agree`), and by the org-merge name gates, which judge pairs of orgs and never a single mention. So
when a notice publishes organization A's number on organization B, B's mention joins A, and from then on B's role
reads as A's.

Exhibit, read 2026-10-01 10:1x UTC on prod:

| | before 448's job 1787 | after |
|---|---|---|
| UKRI's tender 8751605 (UKRI-6398 Meter Integration Boiler Plantrooms, published 2026-09-30), Tenderer, mention name `Schneider Electric`, `GB-PPON-PWYP-8439-MZWY` | org 31627746, headed `Schneider Electric` | **org 5718658, `Sellafield Ltd`** |
| Sellafield's tenders 8751634 and 8751782, buyer, PPON-first | org 31627746, **headed `Schneider Electric`** | org 5718658, `Sellafield Ltd` |

The PPON is Sellafield's: 10 FTS notices publish it beside Sellafield's company number 01002607, and Sellafield's
own buyer sections carry it. UKRI's notice put it on Schneider Electric. Both states are wrong in one place. The
first-seen head election made it worse before the merge: the one foreign mention came first, so Sellafield's own
tenders named their buyer "Schneider Electric".

`/v1/tenders/{id}` serves `parties[].organization_name` from the org's head, not from the mention's own name, so the
reader sees the bound org's name and cannot tell that the notice said something else.

## First unit: measure

How many identifier-bound mentions have a name that agrees with none of their org's names? Use the altid key
(`crosswalk::altid_name_key` with `altid_keys_agree`), which already reads GB names well. Run it over the GB national
orgs first. It is a corpus walk, so it is a dry job (a census-style report) and not a `/v1/sql` loop. It reports:
- mentions whose name shares no key with the org's other mentions, split by role (buyer / tenderer / winner) and by
  scheme (PPON, COH, CHC);
- orgs where the foreign mention is the HEAD (the Sellafield shape before the merge: the head election chose the
  outlier);
- 30 samples per bucket, for a reader.

## Design questions (decide after the measurement)

1. **Bind.** When the name disagrees with an org that has ≥ k agreeing mentions, should the mention bind
   name-only (as a provisional org) and record the identifier as foreign? That is precision against recall: a real
   rename or a trading name also disagrees. 448's campaign found many (Synectics → Ocular Integration), and the
   register (448's `ch_fetch.py`) is what told them apart.
2. **Head.** Elect the head by majority across mentions, not first-seen. That also bears on 448's legal-form root
   (3M UK plc → Ltd stays `plc` forever).
3. **Read.** Serve the mention's own name beside the org's head in `parties[]`, so a reader sees "the notice said
   Schneider Electric" even where the bind is wrong. That one is cheap, and it is honest whatever 1 decides.

## Verify

    curl -s https://tenders.zebreus.click/v1/tenders/8751605 | jq -c '.parties[] | select(.role=="Tenderer") | [.organization_id, .organization_name]'

- **open** (2026-10-01): `[5718658,"Sellafield Ltd"]`, the UKRI tenderer bound to Sellafield.
- **done:** anything that no longer presents Sellafield as UKRI's tenderer: a provisional `Schneider Electric` org
  (design 1), or the mention's own name served beside the head (design 3).

## 2026-10-01 20:3x UTC — design 3 deployed

- `parties[].mention_name` is the name the party's own notice published, served beside the org's head
  `organization_name`. A nested legacy party climbs to its mention: the review found `mention_name: null` on tender
  6281334's second winner, and fixed it. A party with no mention row serves `null` end to end. Docs and OpenAPI are
  updated.
- Live: tender 8751605's Tenderer reads `[5718658,"Sellafield Ltd","Schneider Electric"]`. The bind is still wrong
  (design 1), but it is no longer hidden.
