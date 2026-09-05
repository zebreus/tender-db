# 357 — The identifier-under-several-codes residue, case by case through the verdict path

Status: SLICES 1–5 APPLIED, SLICE 6 (the last full one) STARTING 2026-09-05 19:2x UTC — cohort
`cluster-country-2026-09-05`. Slice 1: 488 moved / R2 316 groups. Slice 2: 239 / 171. Slice 3:
113 / 74. Slice 4: 117 / 69. Slice 5: **144 moved (job 729), R2 89 groups / 89 rows, 179
mentions, 325 parties, 331 bid-parties, 384 winners repointed (job 731)**. Running total:
**1,101 rows corrected, 719 duplicate groups folded**; 2,258 verdicts recorded (keeps included).
Slice-6 packet (job 733): 2,939 clusters, 1,861 reviewable, 1,219 already reviewed left out,
600 of the 642 remaining carried.
Kind: data quality (organization layer) — campaign
Relates to: 326 (the census and the predicate that decided what it could), 355 (the
verdict path and the campaign shape), 314 (the per-member evidence), 311 (the principle)

## The residue

Issue 326's census groups every identifier standing under two or more country codes: 4,303
clusters on 2026-09-01. Its predicate moved 312 rows where the arithmetic named exactly one
survivor and the mistyped code was one letter off, and left the rest on purpose:
`no-one-letter-pair` (the codes differ by more than a letter — a Slovak IČO under `AD`, a
Norwegian orgnr under `LT`), `nobody-asked` (no scheme of the value's shape for any code in
the cluster — DE and ES registers carry no checksum), `anchor-names-several`,
`asked-and-refused`, and the stranger codes still standing in `anchor-names-one` clusters
(`GA`, `VA`, `VU` beside a Bulgarian EIK). Two classes are excluded for good: `too-short`
(a two-digit value is not an identity) and `footprint-excluded` (an embassy is one entity
filed from everywhere it operates).

This is exactly Lennart's steer in issue 311: the rule is the deny-direction floor; what it
cannot settle gets individual review with the full evidence. The 355 campaign showed the
shape works on the same-name cross-border cut (487 cases, 153 moves, 88/99 agreement with an
independent earlier review); this is the wider cut, keyed by identifier rather than by name.

## Built: `country-cluster-packet`

`Db::country_cluster_packet` runs the census uncapped, drops the two excluded classes,
orders the rest by total mentions (the reviewer's first read and the campaign's budget), takes
`cases_cap`, and for each cluster lists the member rows by one identity-index seek per
(code, kind) — the org ids a country verdict needs — each with the per-member evidence the
314 packet established (`anchors`, `country_probed`/`country_agrees`, mentions, name
variants, a few publication ids). The census's own reading (`asked`, `named`,
`named_schemes`, `one_letter_pair`, `heavy_one_letter`, mentions per code) travels with the
case. Stored as report `country-cluster-packet`; stoppable. Two store tests: exclusions,
member ids and evidence, heaviest-first order, the cap truncating the listing and not the
tally.

## The campaign (next)

Rubric v3 with one addition for clusters: a spray shape (one heavy code, several one-mention
strangers sharing its number) is the 326 census's own signature and the strangers move to the
heavy code — unless the name says the row is a foreign branch or mission. Stratify by census
verdict and by shape (spray vs balanced); the 355 floor applies (no agreeing row moves, no
weight-only moves, shared-checksum and shared-register pairs park). Two concurrent agents on
this box: size the first slice to what finishes in a firing.

## Slice 1 reviewed (2026-09-05, 49 agents, ~4.8M tokens, 2h38m)

600/600 cases. Verdicts: wrong-country 485 (no-one-letter-pair/spray 396 of 493), distinct-entities
73, same-entity-two-registrations 27, needs-more-evidence 15. 590 moves (580 high): weight 402,
arithmetic 158, national-format 27, name-language 3. Challenger: agreed on 569 cases, disputed 30
moves (strangers with real standing — Lithuanian firms under LV with 6–22 mentions; an Italian
trade agency's German office; a Belgian development agency's country offices), flagged 1 missed
move. Blind readers (60 cases): 57 same verdict, 58 same HIGH move set. Floor parked 66 highs
(35 mover-not-a-stray, 26 shared-register, 5 name-says-foreign-filing). **Apply set: 488 HIGH**
(weight 333, arithmetic 134, national-format 21): VU→BG 59, VA→BG 45, VN→BG 26, VE→BG 25,
AD→CZ 23, GA→BG 18, SE→FI 17, NO→DK 13, EE→ES 10 … — the Bulgarian spray class (ministry
directorates and municipalities with thousands of mentions beside single-mention copies tagged
VU/VA/VE/VN/GA/GH/IO/DE/FR) is most of it, read case by case and holding. Record:
`357-cluster-country-verdicts-slice1.json`.

## Slice 2 reviewed (2026-09-05, 34 agents, ~3.1M tokens, 1h41m)

378/378 new clusters (the 222 slice-1 clusters still standing were excluded, not re-litigated).
Verdicts: wrong-country 298, distinct-entities 59, same-entity-two-registrations 18,
needs-more-evidence 3. 314 moves (296 high): arithmetic 141, weight 120, national-format 38,
name-language 15. Challenger: agreed on 360 cases, disputed 18 moves, flagged 4 missed moves.
Blind readers (38 cases): 36 same verdict, 35 same HIGH move set. Floor parked 37 highs
(30 shared-register — the Svalbard/Åland/Réunion class, 5 mover-not-a-stray, 2 foreign-filing
names). **Apply set: 243 HIGH** (arithmetic 110, weight 100, national-format 33). Record:
`357-cluster-country-verdicts-slice2.json`.

## Slice 3 reviewed (2026-09-05, 26 agents, ~2.1M tokens, 1h10m)

219 new clusters (the packet's 600 carried 381 already reviewed — the skip landed after, `c4fd5f8`).
Verdicts: wrong-country 159, distinct-entities 30, needs-more-evidence 23, same-entity-two-
registrations 7. 164 moves (159 high). Challenger: agreed on 206, disputed 11, flagged 2 missed.
Blind readers (22): 19 same verdict, 18 same HIGH set. Floor parked 32 (30 shared-register — the
overseas-department class, now issue 358; 2 standing: IBM World Trade Corporation under US,
B2Mobility GmbH under DE). **Apply set: 118 HIGH** (arithmetic 61, weight 36, national-format
21). Record: `357-cluster-country-verdicts-slice3.json`.

## Slice 4 reviewed (2026-09-05, 30 agents, ~2.5M tokens; resumed after a container restart)

332 new clusters — the lighter end of the cohort (heaviest member 3–11 mentions by quartile),
so the verdict mix shifts: wrong-country 193, distinct-entities 90, needs-more-evidence 34,
same-entity-two-registrations 15. 204 moves (181 high). Challenger: agreed on 311, disputed 21,
flagged 2 missed. Blind readers (34): 27 same verdict, 32 same HIGH set — the verdict
disagreements are the reviewer choosing needs-more-evidence where the reader moved (the deny
direction), and one the other way: STEMCELL Technologies FR/ES, where the reader was right that a
Spanish CIF starting with N is a NON-RESIDENT foreign entity's own tax number (now a floor:
`non-resident-cif`, and the case parked by hand). Floor parked 47 (39 shared-register — issue
358; 8 own-legal-form). **Apply set: 117 HIGH** (arithmetic 67, national-format 34, weight 16);
139 `keep` rows recorded. Record: `357-cluster-country-verdicts-slice4.json`.

## Slice 5 reviewed (2026-09-05, 43 agents, ~4.2M tokens, 3h19m)

584 new clusters — the light tail (heaviest member 2–5 mentions; 556 balanced pairs). Verdicts:
wrong-country 338, distinct-entities 169, needs-more-evidence 58, same-entity-two-registrations
19. 346 moves (275 high). Challenger: agreed on 550, disputed 27, flagged 5 missed. Blind readers
(59): 48 same verdict, 48 same HIGH set — lower than the heavier slices, as thin evidence should
give. Floor parked 107 (86 shared-register — issue 358; 15 own-legal-form, now including the
Estonian OÜ, which caught Estonian firms moving to Finland on their Finnish branch registration;
6 non-resident CIF). **Apply set: 144 HIGH** (arithmetic 88, national-format 55 — mostly German
Handelsregister numbers under AT and Northern Irish company numbers under IE — weight 4);
246 `keep` rows recorded. Record: `357-cluster-country-verdicts-slice5.json`.

## A shape the rubric missed, found on slice 2's heaviest movers (2026-09-05)

The six heaviest planned moves of slice 2 were parents carrying a branch's or subsidiary's
number: ROLAND Rechtsschutz-Versicherungs-AG under DE (26 mentions) with its Italian branch's
P.IVA, Agfa Graphics NV under BE with its Polish subsidiary's NIP (its own register refused the
number), Beryl Med LTD under GB with Beryl Med Poland's NIP, Ferrovial Construcción S.A. under
ES with its Portuguese registration, Arch Insurance (EU) DAC under IE with its Italian branch's
number. The rubric's literal conditions held (byte-identical number, the other side validates,
under the 30-mention standing bar) and the challengers agreed — but the entity IS registered
where the row says, and a country move would fuse the parent's mentions into the branch row.
That is the rubric's own same-entity-two-registrations (or a wrong IDENTIFIER, issue 311's
class), never a country move.

**Floor added** (the deny direction, in code): a mover with ≥ 10 mentions parks unless its
own name says "branch in <to>" (`filial i Finland`, `Filiale Italiana`, `клон България`) or
carries the DESTINATION's unambiguous legal form or script (UAB → LT, Oy → FI, A/S → DK,
Sp. z o.o. → PL, Cyrillic → BG, GmbH → DE …) without its own country's. Slice 2: 12 heavy
movers parked by the floor, read by hand: 5 restored (a Lithuanian UAB under LV, a Bulgarian
АД under SK, Total E&P's `клон България` under FR, Applied Medical's `Filiale Italiana` under
NL, a Spanish-named Acciona company under RO), 7 stay parked as mediums. The twelve heavy
movers already applied in the 355 campaign and slice 1 were re-read against the same rule:
all carry the destination's form or language (Protector Forsikring ASA, Kemira Oyj, Syntrade
Oy, Stibo Complete A/S, FCC Construccion S.A., UAB Defensa, Turboenergy Power's Moldovan IDNO,
Indo UK Healthcare's Indian CIN …) — they hold.

## Not in scope

Merging the standing duplicate identities a move creates: R2's arm, run after the wet apply
as in 355. The `too-short` and `footprint-excluded` classes.
