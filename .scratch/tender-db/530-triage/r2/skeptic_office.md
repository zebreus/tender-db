# 530 R2 skeptic: read_office's W (skeptic_office)

Scope: the one W in `read_office.md`, item 5: `r2_office_two_words[5]`, which is the same key as `r2_city_flagged[9]`.
The target is **3353580 "Vergabekammer NRW"**: DE, identifier NULL, provisional, 675 mentions. The plan adds 32 mentions from
16 rows. read_office did not re-read this key. It cited read_city1 item 9, which skeptic_city1 already kept at W.

Method:
- I tried to show the target is one body (R), or that the evidence cannot tell (N).
- I re-derived every fact from the saved bodies in `bodies/` and the plan report. I did not copy them from read_city1 or
  skeptic_city1.
- I checked in `crates/store/src/guid_twins.rs` that the target's `notice_ids` and `cities` come from mentions bound to
  3353580.
- **New GETs: 0.** No allowed endpoint lists an organisation's mentions, so a GET could not change what the target holds.
  Nothing was added to `api_requests.log` (558 lines before and after).

## Verdict: W stands

The documents show two different review chambers, at two seats, among the target's own mentions. Under A6, a target that
already holds two bodies scores W, even when the folded mentions are right, because the fold extends a wrong merge. R2's own
W definition says the same: "the target's name denotes more than one body, or the target holds another body's mentions".

## Evidence (saved bodies)

**Which notices are the target's own.** The plan entry has `notice_ids` [46376226, 46082162, 44543854], `cities`
[koln, munster] and `planned_cities` [munster].
- In `guid_twins.rs`, step 4b reads `target.newest` through `GUID_TWIN_TARGET_NEWEST_SQL`. That is the
  `organization_mentions_org` reverse walk on the target id, over the newest 50 mentions.
- `target_sample` (:1583-1587) takes `notice_ids` from `target.newest`.
- Step 4c (:1216) builds `t.cities` from the BT-513 of those same (notice, section) keys.
- So every listed notice has a mention bound to 3353580. Each of them prints "Vergabekammer NRW" in exactly one ORG section,
  so the bound section is not in doubt.

| notice | section, role | address, phone, NUTS | BT-501 / e-mail | body |
|---|---|---|---|---|
| 46376226 (TBR Remscheid) | ORG-0002, Lot-ReviewOrg on LOT-0001/0002 | Zeughausstraße 2-10, 50667 Köln, +49221-1443045, DEA11 | `Leitweg-ID 05315-03002-81` (05315 = Köln) | Vergabekammer Rheinland, Bezirksregierung Köln |
| 46082162 (the same procedure, another version) | ORG-0002, Lot-ReviewOrg | the same Köln address, phone and NUTS | the same Leitweg | Vergabekammer Rheinland |
| 44543854 (Wald und Holz NRW) | ORG-0002, Lot-ReviewOrg | Albrecht-Thaer-Str. 9, 48147 Münster, +49 2514111691, DEA33 | BT-501 `DEA33` (a NUTS code); vergabekammer@bezreg-muenster.nrw.de, www.bezreg-muenster.de | Vergabekammer Westfalen, Bezirksregierung Münster |

**Planned side.** Both planned notices are Wald und Holz NRW, 2024-10.
- 24313214: ORG-0003 is Lot-ReviewOrg under `0dee9e2b…`, and ORG-0004 is Lot-Mediator under `772257c5…`.
- 24330412: ORG-0003 is Lot-ReviewOrg under `35eed25f…`, and ORG-0004 is Lot-Mediator under `e7982942…`.
- All four sections print the Münster chamber's address, phone, fax and e-mail.
- Planned row 22795963 = `0DEE9E2B…` (24313214 ORG-0003): 2 mentions, `provisional: false`.

**Corroboration only.** 26866950 (Wald und Holz NRW) prints "Vergabekammer NRW" at the Münster seat, with BT-501
`05515-03004-07` (05515 = Münster). It is not in `notice_ids`, so its binding to 3353580 is not shown.

## Every R or N case fails

1. **One authority with two offices?** No. The two sections differ in street, postcode, city, phone, NUTS region (DEA11
   against DEA33) and Leitweg district (05315 against 05515). NRW has two review chambers with separate territories:
   Rheinland at the Bezirksregierung Köln and Westfalen at the Bezirksregierung Münster. That is general knowledge, and the
   documents agree with it.
   - The data model already keeps the two apart, under their own names: 7615 and 11823847 (Köln) against 3233 and 6136373
     (Münster). All four bodies are saved.
   - **This plan itself treats them as two bodies.** 7615 receives +1,346 planned mentions, all from Köln. 11823847 receives
     +296 from Köln. 3233 receives +244 and 6136373 receives +316, all from Münster.
   - Only 3353580 collects both seats.
2. **A typo of one body?** No. Each buyer named the chamber that is correct for its own location: Remscheid lies in the
   Rheinland territory, and Wald und Holz's Hochstift and Egge forest districts lie in the Westfalen territory. "Vergabekammer
   NRW" is a label both buyers used. No body carries that name.
3. **A few stray Köln mentions?** This cannot be measured with the public API: `/v1/organizations/{id}` returns only the count
   (675). Even so, two of the three sampled notices are Köln, from one procedure in two versions, among the newest 50
   mentions. And neither A6 nor R2's W definition sets a share below which a second body is tolerated.
4. **The folded mentions are right, so is the verdict R?** Within each notice, yes.
   - The twin is one chamber printed once per role (ReviewOrg plus Mediator), with an identical address and e-mail, under
     fresh per-notice v4 values.
   - Row 22795963 has 2 mentions and one Tender. It is a per-notice split, not a stable key, so the row-level refusal is
     right.
   - The trouble is where the mentions land. They go onto a row that already mixes Köln and Münster, and A6 decides that
     case as W. The right home would be a Westfalen row, which this name-keyed target cannot reach.
5. **N?** No. Three bound mentions, each printing a full address, are direct evidence. Nothing here is unclear.

## Remedy: agreed, as the exact name

- **Add `vergabekammer nrw` to `extra_unsafe_names`, and to `GUID_TWIN_UNSAFE_NAMES` in Build 2.**
  - The 16 rows and 32 mentions then bind as before, on their GUID rows under decision (c). That is a small coverage cost:
    the Münster chamber stays split per notice for these 16 rows.
- **The exact name covers today's plan.** I walked every object in the report for Vergabekammer names and found 21 keys.
  `vergabekammer nrw` is the only one whose cities span two chambers.
  - The others each have one seat on both sides: Rheinland, Westfalen, des Bundes (and its longer forms), Niedersachsen,
    BW, Brandenburg, Hessen (Darmstadt), Rheinland-Pfalz, Schleswig-Holstein, Nordbayern and Südbayern.
  - Bundeskartellamt 2217 shows `bonn` and `eschborn`. That is one institution, and it is outside this W.
- **Write any "Vergabekammer \<Land\>" shape as an explicit list, not a pattern.** A pattern would also refuse the right
  single-chamber Land folds: Niedersachsen +94, BW +48, Schleswig-Holstein +4, Brandenburg +272.
  - The list is NRW and Nordrhein-Westfalen, plus Bayern, as the umbrella over Nordbayern and Südbayern.
  - Bayern guards future notices only. It is not in today's plan.
- **The sibling umbrella 6622773 "Geschäftsstelle der Vergabekammer"** (Weimar target, Münster planned) is a separate W in
  read_city2 and needs its own entry.
- **Then:** re-run R1 with the additions, and read a fresh random **60** at salt **530003**, excluding rows already read
  (A5 replaced the original 30).

## Tally

| key | kind | reader | skeptic |
|---|---|---|---|
| 3353580 | office_two_words target (duplicate: r2_city_flagged[9]) | W | **W (kept)** |

This agrees with skeptic_city1, which reached the same result on its own.
