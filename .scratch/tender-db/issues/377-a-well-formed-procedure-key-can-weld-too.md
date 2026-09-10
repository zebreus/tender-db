# 377 — a well-formed BT-04 procedure key welds 378 buyers across four countries, and issue 369's gate cannot see it

Status: ready-for-agent — **UNIT 2 DECIDED 2026-09-10: NO GATE. Three signals have now been measured
and all three are blind to a case they exist for; the class is 4 tenders of which 3 are probably
legitimate. Unit 3 (read the source for a platform-level cause) is the only open unit that could still
produce a rule.** UNIT 1 DONE 2026-09-10, and it CORRECTS this issue's own claim: the class
is 4 tenders, only ONE of which is convicted, and buyers-per-version does NOT separate a weld from a
Dynamic Purchasing System — both accumulate buyers across notices. Country spread is what convicted
430681. See the last section before building anything.** Was: needs-triage (filed 2026-09-10 by the owner, from the weld gauge's first run carrying the
per-version column — issue 364's section 12). The gauge was built to find this and it did, on the
first output that could distinguish a weld from a joint procurement.
Kind: defect (identity / grouping) — the same fusion issue 369 fixed, through a key its gate is
blind to by construction
Relates to: 369 (whose placeholder-SHAPE gate this evades), 364 (the legacy-OJS twin, and the gauge
that found this), ADR-0003 (BT-04 as a published fact)

## Observed

Tender **430681**, title `Tervakosken koulu- ja monitoimikeskus, Vaihe 1` — a Finnish school and
multipurpose centre, phase 1. It carries:

| | |
| --- | --- |
| versions | **789** (2023-09-12 → 2025-01-13, subtypes 16/29/17/30) |
| distinct buyer organizations | **378** |
| buyers per version | **0.5** |
| procedure key | `5e001394-80da-44e1-8955-e9fe17674c90` |

**The buyers are not Finnish.** Grouped by the organization layer's country:

| country | orgs |
| --- | --- |
| **CH** | **341** |
| DK | 24 |
| FI | 12 |
| FR | 1 |

By name they are Swiss federal, cantonal and municipal bodies — `Bundesamt für Bauten und Logistik
BBL`, `Stadt Zürich Amt für Hochbauten`, `Flughafen Zürich AG`, `Swissgrid AG`, `Bau- und
Verkehrsdepartement des Kantons Basel-Stadt`, `Bundesamt für Strassen ASTRA`. One Tender fuses
several hundred unrelated construction procurements across four countries.

## Why issue 369's gate cannot catch it

369 refuses a procedure key only when it is **placeholder-SHAPED** and its notices disagree on the
buyer. `is_placeholder_key` (`crates/ingest/src/project.rs:5134`) is
`free.len() <= 6 || uuid_longest_run(s) >= 8`. This key has **14 distinct free nibbles and a longest
run of 2**, so the shape half never fires and the buyer half is never consulted. The key is a
perfectly ordinary random UUID.

**369's choice of shape as the pre-filter was right on its own evidence and is what makes this
invisible.** Its census refuted a shape-only gate (it would refuse 10 correctly-grouped tenders to
fix 3), and buyer-disagreement alone "would refuse joint procurements corpus-wide". Shape bounded the
blast radius. But shape is a property of the STRING, and nothing stops a publisher emitting one
well-formed UUID for everything it publishes.

## What actually separates the two, and it is now measured

The gauge's `per-ver` column (buyers ÷ versions) splits the >= 50 band cleanly, because a joint
procurement names all its buyers in ONE notice while a weld accumulates them across notices:

| tender | buyers | versions | per-ver | what it is |
| --- | --- | --- | --- | --- |
| 331647 | 505 | 1 | **505.0** | `Skupno javno naročilo` — genuine Slovenian joint procurement |
| 7940336 | 541 | 4 | 135.2 | same shape |
| **430681** | **378** | **789** | **0.5** | **this issue** |
| 4228069 | 354 | 928 | **0.4** | legacy weld, key `ojs:2010-001662` |
| 4459994 / 4459995 | 274 / 272 | 277 / 276 | **1.0** | legacy welds, keys `ojs:2011-010241` / `-010242` |

Three orders of magnitude between the two populations. **This is the discriminator 369 needed and did
not have** — and unlike shape, it is a property of the GROUPING rather than of the string, so a
publisher cannot evade it by generating prettier keys.

Note the adjacent pair 4459994/4459995: consecutive tender ids, consecutive OJS keys, near-identical
counts, one Lithuanian lab equipment and one Slovak office furniture. The legacy mechanism fires on
neighbouring notices.

## Units

1. **Size the class.** How many tenders carry a NON-`ojs:` key with many buyers and low per-version?
   **Not done, and the attempt is recorded rather than hidden:** a windowed `/v1/sql` census joining
   `tenders` for `procedure_key` exceeded the 10 s cap at 250,000-id windows, and the un-joined
   variant exceeded it too on its second window. Not retried wider (`docs/agents/prod-box-reads.md`:
   the cap bounds the wait, not the work). This wants an in-process job or a narrower stride, not a
   bigger `/v1/sql` read.
2. **Decide whether per-version concentration joins the gate.** The candidate rule: refuse a
   procedure key whose notices disagree on the buyer AND whose buyers-per-version is low — replacing
   shape as the pre-filter, or sitting beside it. Needs unit 1's numbers first; a rule calibrated on
   one tender is what this project keeps having to correct.
3. **Read the source.** Is one publishing platform emitting a constant BT-04? 341 of 378 buyers are
   Swiss, which points at one national platform rather than at scattered publisher error. Bounded
   metadata read of the notices' sources and platform ids.
4. **Repair, once the rule exists.** Same shape as 369's: refuse, re-group, retire the welded Tender.

## Done when

- the class is sized rather than exemplified;
- there is a decision, written with its reasoning, on whether per-version concentration gates the key;
- 430681 is either split or explained.

*Not claimed here:* that this is common. It is one tender, found because the gauge's first useful
listing surfaced it. The whole point of unit 1 is that a single example is not a population — the
mistake this issue's two siblings each had to correct.

## Unit 1 DONE (2026-09-10) — and it corrects this issue's own discriminator claim

**The class is 4 tenders, not a population.** Every tender with >= 50 distinct buyers (1,326 of them,
a third instrument agreeing with the census and the report), classified by key type and by whether
its buyers are concentrated in single notices or spread across them:

| key | concentrated (>= 10 buyers/version) | mixed (1–10) | **spread (< 1/version)** |
| --- | --- | --- | --- |
| `ojs:` (legacy closure, 364) | 872 | 54 | **83** |
| other (eForms BT-04) | 283 | 30 | **4** |

Two things fall out immediately. **1,155 of 1,326 are concentrated** — many buyers named in ONE
notice, which is joint procurement, not fusion. The ">= 50 is where the reading is safe" claim this
gauge shipped with was not merely unproven, it was wrong about seven entries in eight. And the
genuinely weld-SHAPED set is **87 tenders, 6.6 % of the band** — 83 legacy, 4 eForms.

### The correction: per-version does NOT separate a weld from a DPS

This issue claimed buyers-per-version is "the discriminator 369 needed". **That is too strong**, and
reading the other three of the four says why:

| tender | buyers | per-ver | title | buyer countries |
| --- | --- | --- | --- | --- |
| **430681** | 378 | 0.48 | Finnish school, phase 1 | **CH 341, DK 24, FI 12, FR 1** |
| 333104 | 91 | 0.57 | **`Dinamiskās iepirkumu sistēmas`** izveide … elektroenerģijas | LV 91 |
| 1012301 | 54 | 0.35 | **`DIS`** avseende projektledare … Naturhistoriska riksmuseet | SE 46, +6 |
| 769785 | 71 | 0.36 | Italian irrigation-system works | IT 68, +3 |

**Two of the three name themselves a Dynamic Purchasing System** — `dinamiskās iepirkumu sistēmas`
(Latvian), `DIS` (Swedish). A DPS runs many rounds over years and admits buyers over time, so its
buyers accumulate ACROSS notices — **the same shape as a weld**. Issue 364 unit 2 flagged exactly this
("some of these are legitimate DPS rounds… one system but not one procurement") and it is the shape,
not an edge case.

So per-version separates *joint procurement named in one notice* from *buyers accumulated across
notices*. It does not separate the two things that accumulate. It is a hint, which is what the render
says, and now there is a concrete reason rather than a hedge.

**What actually convicted 430681 was the COUNTRY spread**: 341 Swiss buyers under a Finnish title.
A DPS is one system under one authority's rules; it does not span four countries. The other three are
single-country (91/91 LV; 46 of 52 SE; 68 of 71 IT), consistent with legitimate systems.

### What this changes

- **Unit 2's rule cannot key on per-version alone.** Any rule that refuses a low-per-version key would
  refuse Latvia's and Sweden's DPS notices. Country spread is the candidate signal that survived this
  reading — but it is one example, and that is exactly how this issue's siblings got their thresholds
  wrong.
- **The class is 4, of which 1 is convicted.** Sizing it was worth doing precisely because it turned
  "a defect class" into "one tender and three probable false positives". A rule built for four rows,
  three of which are correct as they stand, is not obviously worth building at all.
- **Issue 364's 83 spread legacy tenders inherit the same question** and are the larger prize. Some
  are certainly DPS too; nobody has read them.

## Unit 2 DECIDED (2026-09-10): no rule. The signal it would key on does not work.

Issue 364 read all 83 of its own spread candidates the same way this issue read its 4, and the
country signal that convicted 430681 **fails on 2816628** — the weld 364 was filed about, whose
buyers are 81 % Polish and therefore indistinguishable from a Polish national framework.

So the count is now three signals tried and three failed: buyer count (87 % of the >= 50 band is
joint procurement named in one notice), buyers-per-version (a DPS accumulates identically), and
country spread (a single-country weld looks like that country's framework). **No aggregate over the
buyer set separates a weld from a legitimate multi-buyer arrangement.**

**Therefore this issue does not get a gate.** Building one would mean picking a threshold on a signal
already measured to be blind to the case it exists for, over a class of 4 tenders of which 3 are
probably legitimate Dynamic Purchasing Systems. That is the shape of every calibration this issue's
siblings have had to withdraw.

**What stands instead:**

- **430681 is a confirmed weld** and can be split by hand if anyone wants it split — 341 Swiss buyers
  under a Finnish school title is not a judgement call. Unit 4's repair applies to it alone.
- **Unit 3 (read the source) is still worth doing** and is now the only open unit with a chance of
  producing a rule: if one publishing platform emits a constant BT-04, the fix is at the parser, keyed
  on the platform, not on any statistic about buyers.
- **The gauge keeps its value** without a gate. It measures, three instruments agree on its bands, and
  the spread/concentrated split is real. Adjudicating individual tenders was never its job.

*Reversed if:* unit 3 finds a platform-level cause, or the class grows past four in a later run —
section 12 makes that visible without anyone remembering to look.

