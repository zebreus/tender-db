# 530 R2 skeptic: read_city1's W (skeptic_city1)

Scope: the one W in `read_city1.md`, item 9: `r2_city_flagged[9]`, which is also `r2_office_two_words[5]`. The target is
**3353580 "Vergabekammer NRW"**: DE, identifier NULL, 675 mentions, plus 32 planned mentions from 16 rows.

Method:
- I tried to show the target is one body (R) or that the evidence cannot tell (N).
- I checked every factual claim behind the W against the saved bodies in `bodies/`.
- **New GETs: 0.** The allowed endpoints cannot list an organisation's mentions, so a GET could not change the count of
  bodies on the row. Nothing was added to `api_requests.log`.

## Verdict: W stands

The documents show two distinct review chambers under this one target. A6 scores that W: "a target that already holds two
bodies scores W, even if the folded mention itself is right, because the fold extends a wrong merge."

## What I checked

The plan entry's `notice_ids` are `[46376226, 46082162, 44543854]`. They come from the target's own newest-50 mentions
(an `organization_mentions_org` reverse walk on 3353580), so each of these notices has a mention bound to 3353580. In each
of them only one section prints the name "Vergabekammer NRW", so I can tell which section binds.

| notice | section | printed address | other evidence | body |
|---|---|---|---|---|
| 46376226 (TBR Remscheid, Sperrgut-Pressfahrzeug) | ORG-0002, Lot-ReviewOrg | Zeughausstraße 2-10, 50667 Köln, +49221-1443045, DEA11 | BT-501 `Leitweg-ID 05315-03002-81` (05315 is Köln's district key) | Vergabekammer Rheinland, Bezirksregierung Köln |
| 46082162 (the same procedure, another version) | ORG-0002, Lot-ReviewOrg | the same Köln address and phone | the same BT-501 | Vergabekammer Rheinland |
| 44543854 (Wald und Holz NRW, Reviergruppe Egge-Nord) | ORG-0002, Lot-ReviewOrg | Albrecht-Thaer-Str. 9, 48147 Münster, vergabekammer@bezreg-muenster.nrw.de, www.bezreg-muenster.de | BT-501 `DEA33` (a NUTS code, not a key) | Vergabekammer Westfalen, Bezirksregierung Münster |

- **Every claim in read_city1 holds.** The addresses, e-mail, website, phone numbers and BT-501 values are as reported.
- **The plan's city screen agrees.** The entry has `cities: ["koln", "munster"]` and `multi_city: true`.
- **Corroboration only:** 26866950 (Wald und Holz NRW, Naturwaldzellen) prints the Münster chamber as ORG-0002 with BT-501
  `05515-03004-07` (05515 is Münster's district key). Its binding to 3353580 is not shown, because it is not in
  `notice_ids`. As read_city1 said, it does not count toward the verdict.

## The R and N cases fail

- **Not one body under two addresses.** Rheinland (Köln, covering Regierungsbezirke Köln and Düsseldorf) and Westfalen
  (Münster, covering Münster, Detmold and Arnsberg) are two separate chambers with separate territories. They are not one
  authority with a branch office.
  - The data model already keeps them apart under their own names: 7615 "Vergabekammer Rheinland" (29,617 mentions),
    11823847 "… c/o Bezirksregierung Köln" (6,223), 3233 "Vergabekammer Westfalen" (37,696) and 6136373 "… bei der
    Bezirksregierung Münster" (15,723). All four org bodies are saved.
  - Each buyer named the chamber that is correct for its own location: Remscheid is in the Rheinland territory, and
    Münster is in the Westfalen territory. "Vergabekammer NRW" is a label that both use, not the name of a body.
- **Not a typo-merge of one body.** The two sections differ in street, postcode, city, phone and Leitweg district.
- **The planned mentions being right does not save it.** The plan has `planned_cities: ["munster"]`, and the two notices I
  could reach (24313214 and 24330412, both Wald und Holz NRW) print twin ORG-0003 Lot-ReviewOrg and ORG-0004 Lot-Mediator
  sections at the Münster chamber's address and e-mail.
  - So within each notice, the twin is one chamber printed once per role, and folding the mentions together is right.
  - The trouble is where they land: a row that already holds Köln mentions. A6 decides that case explicitly.
  - The right home would be a Westfalen row, but the name-keyed target cannot reach one.
- **Not an N.** Three bound mentions, each with a full address, are direct evidence. Nothing here is unclear.

## Remedy: agreed, with one narrowing

- **Add the exact name `Vergabekammer NRW` to `extra_unsafe_names` / `GUID_TWIN_UNSAFE_NAMES`.** That removes the 16 rows
  and 32 mentions from the plan.
  - I searched every Vergabekammer name in the plan report's lists (targets, controls, r2_*). No other
    "Vergabekammer Nordrhein-Westfalen"-style umbrella appears, so the exact name covers today's plan.
- **Write a "Vergabekammer \<Land\>" shape only as an explicit list of Länder that have more than one chamber, not as a
  pattern.** In the same report, the single-chamber Land names are one body each:
  - `vergabekammer niedersachsen` 1222120 (luneburg, +94 planned);
  - `vergabekammer bw` 2175136 (karlsruhe, +48);
  - `vergabekammer schleswig-holstein` 9190 (kiel; "kiel dusternbrook" is a district of Kiel, +4).

  A pattern would refuse those right folds and gain nothing.
  - "Vergabekammer Bayern" (Nordbayern in Ansbach and Südbayern in München) is the only other multi-chamber Land name I
    would list. It is not in today's plan, so listing it only guards future notices.
  - That chamber layout is general knowledge, not something these documents show.
- **Next steps:** re-run R1, then read a fresh random 60 at salt 530003 (A5: any W triggers the re-read).
- **Same family, already scored:** the sibling umbrella 6622773 "Geschäftsstelle der Vergabekammer" (Weimar target, Münster
  planned) is `r2_city_flagged[15]`, scored W in `read_city2.md`. The exact-name entry above does not cover it, so it needs
  its own entry.

## Tally

| key | kind | reader | skeptic |
|---|---|---|---|
| 3353580 | city_flagged_target (also office_two_words[5]) | W | **W (kept)** |
