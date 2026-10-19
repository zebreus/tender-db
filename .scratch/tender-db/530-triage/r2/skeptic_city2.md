# 530 R2 skeptic: read_city2's W (item 15, target 6622773)

Input: read_city2 scored one W, `r2_city_flagged` item 15, target 6622773 "Geschäftsstelle der Vergabekammer". I tried to show it
R or N. Sources: the saved bodies only. **0 new GETs**, so nothing was added to `api_requests.log`. I also used the plan report
`guid_twin_plan_2659.json` and the planner code that produced its city fields, `crates/store/src/guid_twins.rs`
(`guid_twin_finish`).

## Result

| key | kind | reader | skeptic | why |
|---|---|---|---|---|
| 6622773 | city_flagged target #15 "Geschäftsstelle der Vergabekammer" | W | **W (kept)** | The target is the Thuringian chamber in Weimar. Every planned mention that prints a city prints Münster, where Vergabekammer Westfalen uses this string. These are two statutory review chambers in two Länder, so the fold would build a wrong merge. |

The bar for flagged targets is 0 W, so it still fails. The action is unchanged: put `Geschäftsstelle der Vergabekammer` into
`GUID_TWIN_UNSAFE_NAMES`, or into `extra_unsafe_names` for an R1 re-run.

## Evidence (saved bodies)

**The target side is one body: Vergabekammer Thüringen in Weimar.** The report lists the target's three newest notices, and all
three are the Thuringian chamber:
- **23604678** (TED eForms, BT-05 2023-12-05; buyer Landratsamt Saale-Holzland-Kreis, Eisenberg).
  - ORG-0003: BT-500 "Geschäftsstelle der Vergabekammer", BT-501 `123456`, 99423 Weimar, NUTS DEG05,
    vergabekammer@tlvwa.thueringen.de.
  - Roles: Lot-ReviewOrg on all 8 lots, plus ReviewInfo and Mediator.
  - The BT-501 is a placeholder, not a v4 UUID. So Thuringian sections never enter a 530 twin, and they reach the target through
    the ordinary identifier-NULL path.
- **26223383** is that notice's DÖE copy: the same ContractFolderID `7877b6ea…`, DE1-IssueDate 2023-12-05, and the same name, e-mail,
  99423 Weimar and DEG05.
- **23464988** (TED 2023/S 196, published 2023-10-11; buyer Stadt Jena): ORG-2 has TED-OFFICIALNAME "Geschäftsstelle der
  Vergabekammer", TED-TOWN Weimar.
- The plan's city screen over the target's newest 50 mentions is {weimar}, one city only.

**The planned side is Münster.**
- The plan's entry reads `planned_mentions` 10, `planned_rows` 5, `planned_cities` ["munster"], `city_absent` 5.
- In `guid_twin_finish`, `city_absent` counts planned **mentions** whose BT-513 is set and missing from the target's set. A
  mention with no BT-513 goes to `planned_mentions_without_city` instead.
- So 5 of the 10 planned mentions print Münster, and the other 5 print no BT-513 city. A DÖE copy keeps its city in DE1 fields,
  so these are probably DÖE copies. None of the 10 prints Weimar.
- One correction to read_city2: it says "every planned row is city-absent". The plan counts mentions, not rows, so its data
  supports only "every planned mention that has a city is Münster".

**Münster's chamber uses this exact string.**
- 24336021 (Stadt Gelsenkirchen): ORG-0003 is ReviewInfo and ORG-0004 is ReviewOrg. Both are "Vergabekammer Westfalen bei der
  Bezirksregierung Münster" with BT-502 "Geschäftsstelle der Vergabekammer", Domplatz 1-3, 48143 Münster, poststelle@brms.nrw.de.
  They carry two distinct v4 GUIDs.
- 24375779 (Stadt Sundern): ORG-0003 is ReviewOrg and ORG-0004 is Mediator. Both are "Vergabekammer Westfalen" with BT-502
  "Geschäftsstelle der Vergabekammer", Albrecht-Thaer-Str. 9, 48147 Münster, @brms.nrw.de. They also carry two distinct v4 GUIDs.
- This is exactly the twin shape: one chamber printed once per role, a fresh UUID for each. The 5 planned rows are this pattern
  with the BT-502 string in BT-500. The twin key is `(country, org_name_norm(BT-500))`, so a planned row's BT-500 is that string.

## The R/N arguments I tried, and why each fails

1. **"No planned notice was read, so it is unclear (N)."**
   - The planned city is not an inference. It is BT-513 read from each planned mention's own notice through the `notice_texts` PK.
   - No planned notice id can be reached. `planned_mention_keys` is cut at 2,000 keys (notices up to 24341274) and holds no key
     whose target is 6622773. The three allowed endpoints cannot list an organization's mentions: the 6622773 body is 197 bytes,
     with no mention list. A new GET could not have found a planned notice, so I spent none.
   - The Münster screen, together with two notices in which the Westfalen chamber prints this string, is direct enough.
2. **"Münster and Weimar might be the same body (R)."** They are not. The two chambers differ in:
   - Land: Thüringen (Landesverwaltungsamt) against NRW (Bezirksregierung Münster);
   - address and NUTS: 99423 Weimar, DEG05, against 48143/48147 Münster, DEA33/DEA57;
   - e-mail domain: tlvwa.thueringen.de against brms.nrw.de.

   Each Land sets up its own chambers, and no public body is seated in both places.
3. **"The target may already mix chambers, so the fold adds nothing new."**
   - If the target's 55 older mentions, which the screen does not cover, already hold another chamber, the target is already an
     umbrella. A6 scores that W, because the fold extends the wrong merge.
   - If they do not, the fold creates the mix. It is W either way.
4. **"Without the fold, things are no better."**
   - Without the fold, the 5 GUID rows remain. Each one is a Westfalen section: split per notice, but each row names the right
     body.
   - The fold moves all 10 mentions onto a row whose known mentions are all Thuringian, through 2023-12-05. In the served data,
     the Westfalen chamber's review roles would then appear as Weimar's.
5. **"Maybe the guard already binds it."** It does not.
   - `guid_twin_name_words` gives 2 content words (geschäftsstelle, vergabekammer).
   - The name is not `generic_body_name`, and the folded form "geschäftsstelle der vergabekammer" is not in `GUID_TWIN_UNSAFE_NAMES`.
     That list holds only "Vergabekammer".
   - `office_two_words` is false, because `geschäftsstelle` is not in `GUID_TWIN_OFFICE_NOUNS`.

## Notes for the fix

- Add `"Geschäftsstelle der Vergabekammer"` to `GUID_TWIN_UNSAFE_NAMES` in `crates/ingest/src/project/guid_twin.rs`, or pass it in
  `extra_unsafe_names` for the R1 re-run. The 5 groups then bind, and their GUID rows stay.
- Optional: add `geschäftsstelle` to `GUID_TWIN_OFFICE_NOUNS`, so R2 reads a 2-content-word "Geschäftsstelle <X>" key whole.
- The Mecklenburg-Vorpommern key "Geschäftsstelle der Vergabekammern beim Ministerium für Wirtschaft, Infrastruktur, Tourismus und
  Arbeit Mecklenburg-Vorpommern" (rand2 item 26, target 23021560, R) names its Land. It is a different, safe key, and this
  addition does not touch it, because the unsafe list matches the whole folded name.
