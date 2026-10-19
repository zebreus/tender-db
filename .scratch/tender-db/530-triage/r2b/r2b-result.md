# 530 R2b: result (tally of the fresh R2 re-read of dry plan job 2660)

Inputs:
- Issue 530: "Unit 2 — decided", "Unit 3 Build 1", "R2 — read … FAILED on 3 W; fixes decided" and "Build 1b — built,
  gated, deployed; R1 re-run PASSED".
- `530-triage/decision.md`: R2, plus the critic amendments A5 (R2's sample and bar), A6 (city screen) and A7 (role coherence).
- `530-triage/r2/r2-result.md`: R2's method and its three W findings, fixed in Build 1b (deployed 8e9222c).
- The plan report `530-triage/guid_twin_plan_2660.json`. `body.r2_random` holds the fresh 60 at salt 530003, stratified
  buyer 25 / review_or_info 25 / other or tenderer 10. The 60 rows R2 read are excluded (`r2_read_excluded` 60, of which
  59 are still planned).
- The reads `r2b/read_rand{1,2,3,4}.md`, covering items 0-14, 15-29, 30-44 and 45-59.
- Matching: I matched every read row to `r2_random[i]` by org id. All 60 matched, in order, with the plan's stratum and
  target. No row was missing, duplicated or extra.

## Verdict: R2 PASSES. Build 2 may ship.

| list | picks | R | N | W | bar | result |
|---|---|---|---|---|---|---|
| fresh random 60 (salt 530003; buyer 25, review_or_info 25, other or tenderer 10) | 60 | 60 | 0 | 0 | 0 W, N ≤ 3 | **pass** |
| buyer | 25 | 25 | 0 | 0 | | |
| review_or_info | 25 | 25 | 0 | 0 | | |
| other or tenderer (7 other, 3 tenderer_winner) | 10 | 10 | 0 | 0 | | |
| top 30 targets, city-flagged 36, office keys 7 | 0 unread | | | | 0 W | **nothing new to read** |
| role-incoherent | 0 (empty) | | | | 0 W | **nothing to read** |

- **One R is conditional on the mint:** item 32. See "Missing target in the sample" below.
- **No W, so no fix.** No new unsafe name goes into `GUID_TWIN_UNSAFE_NAMES` or `extra_unsafe_names`, no guard changes, and
  no further fresh 60 is needed.
- **Neither of R2's W shapes occurs.**
  - The umbrella shape does not occur: no target is an umbrella chamber name collecting different chambers. The chamber
    targets in the sample each name one chamber: 7615 VK Rheinland, 12774348 VK Rheinland c/o BezReg Köln, 1166553
    Niedersachsen, 6136373 Westfalen bei der BezReg Münster, 1175826 Hessen, 1166261 Rheinland-Pfalz and 23021560 MV
    Geschäftsstelle. The MV Geschäftsstelle is the long key that F1 kept off the list.
  - The review-slot shape does not occur: no review slot prints another body's contact details. No planned group holds a
    Lot-ReviewOrg beside a buyer-side role (`roles.buyer_with_review_org` = 0). Every ReviewInfo section inside a buyer
    group prints the buyer's own address, e-mail and phone: items 6, 12, 14, 16, 17, 20, 44, 47, 48, 51, 54 and 58.

## The bound (A5's sentence, restated for this sample)

**As drawn: 0 W in 60.** The one-sided 95% upper bound on the plan's error rate is 4.9%, about 535 of the 10,995 planned
rows.

| stratum | planned rows | picks | bound | rows |
|---|---|---|---|---|
| buyer | 2,731 | 25 | 11.3% | 308 |
| review_or_info | 8,063 | 25 | 11.3% | 910 |
| other or tenderer | 201 | 10 | 25.9% | 52 |

**Correction for independence: 43 picks are independent, not 60.**
- **Why the two draws overlap.** The draw orders rows by `((org_id XOR salt) × 2654435761) mod 2³²`. Since 530002 XOR 530003 = 1,
  `h(o, 530003) = h(o XOR 1, 530002)` exactly. So the salt-530003 order is the salt-530002 order applied to each row's XOR-1
  neighbour. Twin sections are minted at consecutive ids, so the "fresh" draw pulls in the twin partners of R2's picks.
  - The read files flagged this as a side note: read_rand2's notes, and read_rand3's independence note.
- **The overlap.** 23 of the 60 picks are the XOR-1 neighbour of an R2 pick.
  - 17 are the twin partner in the same notice group, with the same target: items 0, 1, 2, 3, 18, 25, 27, 28, 29, 33, 38, 39,
    40, 41, 42, 43 and 49. Their fold decision (this notice's group → this target) is one R2 had already read and scored R.
    Each agrees with R2's verdict.
  - 5 sit in the same notice as an R2 pick but in another group, with another target: items 26, 31, 44, 45 and 46. These
    are new folds.
  - 1 is in another notice: item 7.
- **Bound on the independent picks: 0 W in 43.** The upper bound is 6.7%, about 740 rows.

| stratum | independent picks | bound | rows |
|---|---|---|---|
| buyer | 20 | 13.9% | 380 |
| review_or_info | 13 | 20.6% | 1,659 |
| other or tenderer | 10 | 25.9% | 52 |

- **Pooled, indicative only and not the bar.** Pooling R2's 59 still-planned picks with these 43 gives 102 distinct folds and
  0 W, a bound of about 2.9% (about 318 rows). The pool is the union of two hash-ordered draws, not one simple random sample.
- **Why the pass stands.** The bar is 0 W and N ≤ 3 on the 60, and the 60 has 0 W and 0 N. So does the independent 43.
- **For any later fresh draw** (for example, the recurrence arm or an R3 sample), avoid the overlap in one of two ways:
  - choose a salt whose XOR with every salt already read is at least 256, so that XOR neighbours fall in other notices;
  - or skip any pick that shares a notice with a row already read, not only the row itself.

## Flagged lists of plan 2660: nothing R2 did not already read

I checked every item by target id, or by (name_norm, country) for a missing key. Each matches an item that R2 read in the
same list of plan 2659.
- **`r2_top_targets` (30).** These are the same 30 targets, index for index, as 2659. The only delta is that 7615 has
  669 rows / 1,338 mentions planned, down from 673 / 1,346.
- **`r2_city_flagged` (36).** All 36 were in R2's 41. Five dropped out:
  - 3353580 "Vergabekammer NRW" (W1) and 6622773 "Geschäftsstelle der Vergabekammer" (W2), now refused under the unsafe
    names;
  - 5180467 Leinemann Partner and 1206438 IKK classic, whose groups the role guard now binds (both are flag b, so any
    remaining planned row would put them back on the complete list);
  - 20316475 GSI, whose R2 random row 23761350 is now refused by the role guard. It is the one R2 pick not planned in 2660.
- **`r2_office_two_words` (7).** All 7 were in R2's 8. W1 (3353580) dropped out.
- **`r2_incoherent`.** The list is empty: `roles.incoherent_count` 0, `tripwire_role_mix` 0, `role_ref_disagree` 0.
  - Build 1b runs the role guard at the wide scope: any buyer-side role beside Lot-ReviewOrg binds, giving 130 groups and
    292 mentions.
  - So R2's conditional extra read of the 70 AddInfo/SProvider+ReviewOrg groups does not apply. No refused role set in
    `roles.by_role_set` mixes ReviewOrg with a buyer-side role.
- **R2's W items are gone.**
  - 3353580 and 6622773 occur nowhere in the 2660 report.
  - The 21 unsafe-name rows (16 "vergabekammer nrw" and 5 "geschäftsstelle der vergabekammer", 42 mentions) are all absent
    from `planned_row_ids`.
  - 22752175-77 (AOK, W3) are not planned. They are in the not-planned controls and in `guard_samples.role_mix`.
- **Plan delta.** 11,103 → 10,995 rows and 22,324 → 22,056 mentions. 2660's planned set is a strict subset of 2659's.
  - 21 rows left under the unsafe names.
  - The other 87 left with the role guard, the only other rule change in Build 1b.

## Missing target in the sample

- **Item 32, row 22828976:** "Stadt Boizenburg/Elbe - Der Bürgermeister - c/o KSM Kommunalservice Mecklenburg AöR - Zentrale
  Vergabestelle" (Lot-TenderReceipt), with no identifier-NULL target.
  - The key covers 16 rows and 32 mentions. Its planned city is {schwerin} only, so flag c is not set.
  - The name denotes one town acting through KSM. The buyer's own name and KSM's own name are other keys.
  - So the row Build 2 mints by the issue-234 path would be one body. **R, conditional on the mint**, as decided for the
    missing keys.
- **Plan-wide, unchanged from 2659:** 52 missing keys, 312 rows, 636 mentions. Build 2 mints one identifier-NULL row per key,
  and R3 checks that one-per-key.

## R verdicts with caveats (none affects the bar)

- **Target's own mentions not read**, so the verdict rests on the name, the clean city screen and the notice:
  - item 37, LVwA Sachsen-Anhalt 1168880, Mediator beside ReviewOrg at the LVwA seat in Halle;
  - item 46, Unna 2399618;
  - item 57, Götz-Gebäudemanagement RMG 10392760;
  - item 58, IT.NRW 1592402.
- **Target read through a sibling planned notice instead of its newest mentions:** 19560123, 14436538, 14467750, 2463330,
  17573857, 2439295, 23021560, 1176684 and 12774348 (rand2), plus 5 rand1 targets. The allowed endpoints have no route from
  an organization to its newest notices. None of these names is generic or an umbrella.
- **City-flagged targets in the sample:**
  - 7615: flag b is only the postcode 50667 in the city field (items 30, 34, 42 and 43);
  - 30899495: one unlocated Geilenkirchen mention that is not this row; both copies print Aachen (item 16);
  - 8808564 Deloitte GmbH WPG: the Düsseldorf office of one legal entity (items 56 and 59; R2 city3 item 31 was R).
- **Kolpingstadt Kerpen (item 0)** is the buyer-side group of a different notice from R2's incoherent Kerpen group, 24453957.
  Build 1b now binds that group: it is in `guard_samples.role_mix`.

## Per-item verdicts

"R2 overlap" marks the picks whose XOR-1 neighbour is an R2 pick.

| # | row | name | stratum | target | verdict | R2 overlap |
|---|---|---|---|---|---|---|
| 0 | 22959818 | Kolpingstadt Kerpen | buyer | 2383569 | R | twin of R2 pick 22959819 (same group, same target) |
| 1 | 22879584 | Verwaltungs-Berufsgenossenschaft (VBG) - Hauptverwaltun… | buyer | 10749091 | R | twin of R2 pick 22879585 (same group, same target) |
| 2 | 22886294 | Bundesstadt Bonn - Referat Vergabedienste | buyer | 15234277 | R | twin of R2 pick 22886295 (same group, same target) |
| 3 | 22770770 | Brandenburgischer Landesbetrieb für Liegenschaften und… | buyer | 14256311 | R | twin of R2 pick 22770771 (same group, same target) |
| 4 | 22956859 | Landkreis Göttingen | buyer | 2294259 | R |  |
| 5 | 22929057 | Rheinland-Pfälzische Technische Universität Kaiserslaut… | buyer | 20509663 | R |  |
| 6 | 22917694 | Stadt Münster | buyer | 2251109 | R |  |
| 7 | 22830289 | Handwerkskammer Ostmecklenburg-Vorpommern | buyer | 17956113 | R | XOR-1 of R2 pick 22830288 (other notice) |
| 8 | 22819953 | Universität Stuttgart | buyer | 1262231 | R |  |
| 9 | 22952705 | Bau- und Liegenschaftsbetrieb NRW Aachen | buyer | 2443792 | R |  |
| 10 | 22769455 | Landkreis Havelland - Der Landrat - | buyer | 7114335 | R |  |
| 11 | 22818997 | SBO Servicebetriebe Oberhausen - Eigenbetrieb der Stadt… | buyer | 17012302 | R |  |
| 12 | 22899365 | Stadtverwaltung Cottbus | buyer | 1514797 | R |  |
| 13 | 22910304 | Bau- und Liegenschaftsbetrieb NRW Zentrale | buyer | 7195752 | R |  |
| 14 | 22957742 | Stadt Remscheid, Fachdienst Recht und Vergabe, Abteilun… | buyer | 16921787 | R |  |
| 15 | 22772664 | Universität Stuttgart | buyer | 1262231 | R |  |
| 16 | 23001478 | Stadt Aachen - Zentrale Vergabestelle FB60/300 | buyer | 30899495 | R |  |
| 17 | 22834338 | Ministerium für Umwelt, Naturschutz und Verkehr des Lan… | buyer | 19560123 | R |  |
| 18 | 22859790 | IHP GmbH - Leibniz-Institut für innovative Mikroelektronik | buyer | 1648483 | R | twin of R2 pick 22859791 (same group, same target) |
| 19 | 22819228 | HIL Heeresinstandsetzungslogistik GmbH | buyer | 1405749 | R |  |
| 20 | 22852201 | Oberfinanzdirektion Karlsruhe - LZfD | buyer | 14436538 | R |  |
| 21 | 22846983 | Stadt Marl - Amt für Klimaschutz und Nachhaltigkeit - Z… | buyer | 14467750 | R |  |
| 22 | 22860702 | Polizeipräsidium Düsseldorf | buyer | 2463330 | R |  |
| 23 | 22771735 | Landratsamt Ostalbkreis - Zentrale Vergabestelle | buyer | 17573857 | R |  |
| 24 | 22850671 | Stadt Gütersloh | buyer | 2439295 | R |  |
| 25 | 22834379 | Geschäftsstelle der Vergabekammern beim Ministerium für… | review_or_info | 23021560 | R | twin of R2 pick 22834378 (same group, same target) |
| 26 | 22957397 | Aufsichts- und Dienstleistungsdirektion | review_or_info | 1176684 | R | XOR-1 of R2 pick 22957396 (same notice, other group) |
| 27 | 22975387 | Vergabekammer Rheinland · c/o Bezirksregierung Köln | review_or_info | 12774348 | R | twin of R2 pick 22975386 (same group, same target) |
| 28 | 22965731 | Vergabekammer Niedersachsen beim Nds. Ministerium für W… | review_or_info | 1166553 | R | twin of R2 pick 22965730 (same group, same target) |
| 29 | 22868221 | Vergabekammer Westfalen bei der Bezirksregierung Münster | review_or_info | 6136373 | R | twin of R2 pick 22868220 (same group, same target) |
| 30 | 23096532 | Vergabekammer Rheinland | review_or_info | 7615 | R |  |
| 31 | 22858437 | Hamburg Messe und Congress GmbH | review_or_info | 1401506 | R | XOR-1 of R2 pick 22858436 (same notice, other group) |
| 32 | 22828976 | Stadt Boizenburg/Elbe - Der Bürgermeister - c/o KSM Kom… | review_or_info | missing (minted) | R if minted |  |
| 33 | 22824043 | Bau- und Liegenschaftsbetrieb NRW Bielefeld | review_or_info | 1203308 | R | twin of R2 pick 22824042 (same group, same target) |
| 34 | 23037085 | Vergabekammer Rheinland | review_or_info | 7615 | R |  |
| 35 | 22833833 | Universitätsklinikum Köln AöR | review_or_info | 1281557 | R |  |
| 36 | 22852079 | Bau- und Liegenschaftsbetrieb NRW Düsseldorf | review_or_info | 1173255 | R |  |
| 37 | 22926566 | Landesverwaltungsamt Sachsen-Anhalt | review_or_info | 1168880 | R |  |
| 38 | 23001168 | Vergabekammer des Landes Hessen bei dem Regierungspräsi… | review_or_info | 1175826 | R | twin of R2 pick 23001169 (same group, same target) |
| 39 | 22842359 | Norddeutscher Rundfunk | review_or_info | 1202351 | R | twin of R2 pick 22842358 (same group, same target) |
| 40 | 22816859 | Stadt Kyritz | review_or_info | 7175314 | R | twin of R2 pick 22816858 (same group, same target) |
| 41 | 22840799 | Vergabekammer Rheinland-Pfalz beim Ministerium für Wirt… | review_or_info | 1166261 | R | twin of R2 pick 22840798 (same group, same target) |
| 42 | 22897539 | Vergabekammer Rheinland | review_or_info | 7615 | R | twin of R2 pick 22897538 (same group, same target) |
| 43 | 22966356 | Vergabekammer Rheinland | review_or_info | 7615 | R | twin of R2 pick 22966357 (same group, same target) |
| 44 | 22914740 | Zentraldienst der Polizei des Landes Brandenburg | review_or_info | 1168328 | R | XOR-1 of R2 pick 22914741 (same notice, other group) |
| 45 | 22846853 | Rheinland-Pfälzische Technische Universität Kaiserslaut… | review_or_info | 20509663 | R | XOR-1 of R2 pick 22846852 (same notice, other group) |
| 46 | 22898598 | Kreisstadt Unna - Zentrale Vergabestelle | review_or_info | 2399618 | R | XOR-1 of R2 pick 22898599 (same notice, other group) |
| 47 | 22825834 | Medizinische Hochschule Hannover | review_or_info | 1167697 | R |  |
| 48 | 22858540 | Stadtverwaltung Kaiserslautern - Stabstelle IV.1 Zentra… | review_or_info | 16342601 | R |  |
| 49 | 22838237 | Wasserverband Eifel-Rur - Zentrale Vergabestelle - | review_or_info | 20038121 | R | twin of R2 pick 22838236 (same group, same target) |
| 50 | 23096323 | Stiftung Preußische Schlösser und Gärten Berlin-Branden… | other | 8100 | R |  |
| 51 | 22856874 | Zentraldienst der Polizei des Landes Brandenburg | other | 1168328 | R |  |
| 52 | 22825576 | Stadt Bielefeld - Amt für Organisation, IT und Zentrale… | other | 13304619 | R |  |
| 53 | 22934649 | Stadtwerke Schwerin GmbH (SWS) | other | 4791242 | R |  |
| 54 | 22825013 | Zentraldienst der Polizei des Landes Brandenburg | other | 1168328 | R |  |
| 55 | 23113381 | Stiftung Preußische Schlösser und Gärten Berlin-Branden… | other | 8100 | R |  |
| 56 | 22942475 | Deloitte GmbH Wirtschaftsprüfungsgesellschaft | tenderer_winner | 8808564 | R |  |
| 57 | 22807887 | Götz-Gebäudemanagement RMG GmbH | tenderer_winner | 10392760 | R |  |
| 58 | 23070360 | Landesbetrieb Information und Technik NRW | other | 1592402 | R |  |
| 59 | 22942480 | Deloitte GmbH  Wirtschaftsprüfungsgesellschaft | tenderer_winner | 8808564 | R |  |

Every row in the table has the same plan facts: DE, 2 mentions (the TED and DÖE copies of one notice), 1 Tender,
`planned: whole`, not in KEEP, not stable, `twin: true`, `twin_after: false`. In both copies each row's BT-501 sits on the
same ORG section, and its twin partners carry other v4 values that change from notice to notice. So no row is a stable key.
The reasons for each verdict are in the read files and in `r2b-result.json` (`items[].why`).

## Next

- **R2 passes.** Build 2 may ship as decided:
  - the KEEP file;
  - the gate wired in;
  - the wet path;
  - the recurrence arm (A1);
  - the mint of the 52 missing keys.
- **R3** follows the wet run.

## Evidence base

- **GETs.** 240 public GETs are logged in `r2b/api_requests.log`, from 20:44:00.830Z to 21:01:49.439Z.
  - By endpoint: 129 `/v1/notices/{id}/content` and 111 `/v1/organizations/{id}`.
  - By reader: rand1 66, rand2b 66, rand3 59 and rand4 49.
  - All returned 200, with no 408 and no retry (every line is `attempt=1`), and no path was fetched twice.
  - The readers serialized through `r2b/.api.lock`. The smallest gap between any two logged requests is 2.049 s.
  - There are 240 saved bodies in `r2b/bodies/`. The readers also reused R2's cached bodies in `r2/bodies/` with 0 GETs.
- **This tally made 0 GETs.** No `/v1/sql`, ssh, POST, git or cargo was used.
- **Machine-readable copy:** `r2b/r2b-result.json`.
