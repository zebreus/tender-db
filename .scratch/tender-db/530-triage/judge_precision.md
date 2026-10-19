# 530 unit 2 — precision judge

Judge: precision first. The two worst outcomes are (1) a refusal that splits a real stable entity, and (2) a refusal
whose 234 fallback row, keyed (name_norm, country), merges two different bodies that share a name. Inputs:
`guid_census_2655.json` (job 2655, rev 0787596), `crates/store/src/guid_census.rs` (definitions),
`crates/ingest/src/idgate.rs` (`condemns`, `uuid_v4`), `project.rs:11944-11957` (the gate call site),
`project.rs:10574-10741` (`content_words`, `guard_name_words`, `generic_body_name`), issues 234, 312 and 300 design:84-85
and §10. I made 13 public GETs, listed at the end.

## Decision

**A narrow (a) with two exemptions, plus a (b) cut down to one FR platform shape that only ships if its read passes,
plus (c) for everything else. (d) is rejected.**

- **530-T, the within-notice twin (a).** It is pair-scoped. A GUID value already bound to a row keeps binding, and
  a name that is not safe as a fallback key keeps binding. In the stock, only single-use rows are dissolved.
- **530-F, the 8-4-4-16 raw shape (b).** It covers TED's "other" cell: 3,511 rows, all single-use, 0 spanning
  several tenders, 0 stable. It ships only if R5 below passes.
- **(c) for the remainder**, about 59.3k single-use rows, with their cost written down and a sentence at
  `docs.rs:880`.

## Why not each option as worded

**(a) as worded in the issue fails on precision.** The rule is "one name under two GUIDs in one notice ⇒ refuse". It
reaches **70 stable rows** and 179 rows that span several tenders (11,437 rows minus 11,258 single-use). The
notice-scoped reading reaches 114 stable rows, and 254 and 255 (the Verify controls) are among them: both are
`twin_notice`. The samples show what that means. Several platforms publish a **stable pair** of GUIDs per body, one
per ORG profile:
- SBH | Schulbau Hamburg 3866/3868 (4,472/4,548 mentions, about 1,150 tenders each, twin in notices 1532, 1777,
  5517, 5710 and 9078);
- GMH 22040636/37, Sprinkenhof 10966, Stadt Nürnberg Hochbauamt 1085;
- VK Nordbayern 2767, KUBUS 11107, EKK plus 21986305, Vergabekammer Westfalen 21987063 (422 mentions).

So a twin inside one notice does not prove that a value is not an entity key. It proves only that the entity has
more than one key. This is not a category argument of the `notice_number` kind (425 of 425). It needs an
exemption.

**(d) is rejected.** Five reasons:
1. **It is stateful and depends on order.** It fires on DB state at mint time ("an earlier GUID row with ≤ 2
   mentions"), so a value's fate depends on whether some unrelated single-use row of the name exists at that
   moment. The trigger row is itself one of the rows the dissolve removes. After the dissolve, the next fresh
   value binds and becomes the new trigger. A trigger row that grows to 3 mentions stops firing.
2. **It hits stable rows.** It would have refused **11 stable rows** at mint (5351 at 271 mentions, 8756 at 126,
   21987063 at 422, all Vergabekammer Westfalen), plus 1,570 rows that span several tenders (30530641 among
   them). Every flip of its state between a refusal and a bind leaves such a row split.
3. **Its fallback merges different bodies, and this time in BUYER roles.** 234 bounded the generic-name harm by
   finding it only in review and eSender roles. That bound does not hold for this class. The GETs show:
   - `Tiefbauamt` CH: 9 GUID rows, 1 mention each, buyers. (d) folds 8 of them into identifier-NULL row 6199146
     (15 mentions). That is any canton's or town's civil-engineering office.
   - `Hochbauamt` CH: 4 GUID rows. They would fold into 6956143 (89 mentions). The same name already sits on three
     distinct national keys (`493466` at 42 mentions, `583264` at 8, `839832` at 4), so the name already denotes
     at least 3 bodies.
   - `Vergabekammer` DE: **24 GUID rows** at 2–6 mentions. They would fold into 1431661, already at **4,280
     mentions** of every chamber.
   - `Vergabestelle` DE: a GUID row of 8 mentions, beside identifier-NULL row 1804271 (178 mentions).
4. **Its reach is large and nobody has read its precision.** It touches 4,481 names and 57,353 rows, and no read
   of its fallback rows exists.
5. **It reaches the forward shape only by name equality.** The census's own reach figure (75.4%) does not tell a
   right merge from a wrong one.

**(b) beyond one cell has no clean boundary.**
- doe+ted dashed-lower holds 338 of the 345 stable rows.
- ted dashed-lower holds 1,821 rows that span several tenders (CH winners: FR_Swisscom has 53 tenders).
- Rows first seen in 2026 still include 708 that span several tenders.
- CH holds 1,788 rows that span several tenders.
- "doe dashed-lower" (1,834 rows, all single-use) is selected by "seen once in a DÖE-only notice", not by a
  platform, and a young stable key would land there too.

The one clean cell is **TED "other" = the 8-4-4-16 shape**: 3,511 rows, 3,511 single-use, 0 spanning several
tenders, 0 stable, 1 mention each. I verified the shape on one specimen, notice 277:
`FD29FC2F-A2BF-49A1-9B234B870B531C27`, from the FR Avenue-Web platform. A neighbouring section reads
`FD29FC01-04FA-…` with the same prefix, which looks like sequential minting. Zero of 3,511 rows reuse a value
across tenders, so a refusal there cannot split a stable key.

**Pure (c) is safe, but it gives up a fix whose risk can be bounded.** With the two exemptions below, 530-T reaches
**0 stable rows and 0 rows spanning several tenders by construction**: stable `one_tender_twin` = 0, and the 179
twin rows that span several tenders are all kept. The only remaining exposure is the fallback name. The guard and
read R2 bound it, and it can be undone: `raw_identifier` stays on every refused mention, which is the restore path
312 used.

## Rule wording (as the gate applies it)

**530-T: within-notice twin.** It runs in the projection, beside `idgate::condemns`, because it needs the notice and
`condemns` sees one value.

The grouping, for each notice copy separately (TED and DÖE are each their own notice):
- Take the ORG sections whose normalised BT-501 value satisfies `idgate::uuid_v4`.
- Group them by `(country, org_name_norm(name))`. A section with no name or no country joins no group.

The refusal: in a group holding **two or more distinct** uuid_v4 values, refuse the identifier of every section in
the group. Each mention then takes the identifier-NULL (234) path, and its `raw_identifier` is kept. There are two
exceptions:
- **(i) Bound key.** A value that already keys a standing organization binds as before (`org_of` holds
  `(country, "national", value)`). In a pair where one value is bound and the other is new, only the new one is
  refused.
- **(ii) Name-safe guard.** If the group's name is not a safe fallback key, every value in the group binds as
  before. A name is not safe when either of these holds:
  - `content_words(guard_name_words(name)) < 2`;
  - its folded words are a `generic_body_name`. That list is extended with Vergabekammer, Vergabestelle, Zentrale
    Vergabestelle, Beschaffungsstelle, Tiefbauamt, Hochbauamt, Bauamt, Bauverwaltung, Gemeindeverwaltung,
    Stadtverwaltung, Finanzbehörde, Tribunal administratif, plus whatever R2 finds.

The rule leaves every section outside the group alone. It is pair-scoped; the notice-scoped reading is rejected
because it reaches 114 stable rows, including 254 and 255.

Stock dissolve selection (unit 3, through 440's `repair-placeholder-orgs`). A row is dissolved if and only if all
of these hold:
- its identifier satisfies uuid_v4;
- its mentions span **exactly one distinct Tender** (the `tender_versions.caused_by_notice_id` basis);
- at least one of its mentions sits in a 530-T group that passes the guard;
- it is not in the stable population (254, 255, 8378, or 100+ mentions). This is a hard tripwire, even though
  single-use already implies it.

Exemption (i) cannot be the stock criterion, because every stock value is bound. After the dissolve, (i) sees the
value as unbound, so a refold refuses it the same way and the dissolve stays put. That is why the gate half has to
ship even though its forward reach is small.

**530-F: the 8-4-4-16 platform shape.** It ships only if R5 passes. Refuse a uuid_v4 value whose published BT-501
text, trimmed, is 35 hex characters with dashes at offsets 8, 13 and 18 and nowhere else. Exemptions (i) and (ii)
apply. In the stock, dissolve the rows of that shape whose mentions span one Tender (3,511 measured).

**Fallback.** A refusal always lands on the identifier-NULL row for `(name_norm, country)`, never on an
identified row (300 §10). For Göttingen that is **2294259**: DE, identifier NULL, 216 mentions, confirmed by GET.
It is not 16393 (country NULL, 804 mentions) and not the VAT row 22183653.

## Expected effect (census 2655)

- **530-T** dissolves at most **11,258** single-use rows (15.2% of 74,023): DE 11,237, FR 18, AT 1, other 2. By
  role: review_or_info 8,262, buyer 2,774, other 183, tenderer 39. The guard's exclusions are not yet measured
  (Vergabekammer DE is one). That is about 2 mentions per row (TED+DÖE), or about 22k mentions re-pointed to
  identifier-NULL rows.
  - Stable rows reached: **0**. Rows spanning several tenders reached: **0**. All 179 twin rows that span several
    tenders are kept, the 70 stable ones among them, and 254, 255 and 8378 bind through (i) whatever they are.
  - The forward value is small. Single-use twin rows by first year: 2024 7,459, 2025 3,753, **2026 36**. The
    Göttingen shape has nearly stopped. The gate half mostly keeps a re-projection from minting the dissolved
    rows again.
- **530-F** adds 3,511 FR rows (1 mention each). Its overlap with the twin rows is 18. Combined:
  **14,751 single-use rows (19.9%)**, still 0 stable and 0 spanning several tenders.
- **Kept deliberately (c):** about **59,272** single-use rows (80.1%), plus whatever the guard excludes:
  - all of CH (27,837 single-use, no twins);
  - the DE no-twin shape, about 30.8k rows (Südbayern 30971684/734/757, Bremen 30532145/147, the Vergabekammer
    Westfalen and Südbayern runs);
  - nearly all of 2026's per-notice minting (17,533 single-use, 13,085 cross-only). That is about 1.9k rows a
    month, and the docs sentence has to own it.
- **Verify line 1** (Göttingen) reads the non-twin Göttingen GUID rows as deliberately kept, with the reason "no
  within-notice twin; cross-notice name equality declined for precision". That count comes from the dry plan;
  22759261 and 22759262 are twins and fold into 2294259. Lines 2-3 hold through (i): 254 stays at 24,219 mentions or
  more, 255 at 73,039 or more.
- **docs.rs:880 sentence:** "Some German and Swiss eForms platforms publish a fresh BT-501 platform key (a v4 UUID)
  per notice, so one body can appear as many organizations with `identifier_kind: "national"` and one or two
  mentions each. Such a key is a platform record id, not a register number. Group these by name to aggregate them
  (issue 530)."

## Risks

1. **The generic-name guard will be incomplete.** A two-word generic ("Zentrale Vergabestelle") or a town name
   shared by two places ("Stadt Neustadt") can fold different bodies together. Read R2 bounds this, and the
   refusal can be undone through `raw_identifier`.
2. **Young stable pairs.** A body on a stable-pair platform that has no row yet is refused in its twin notices. If
   it also appears in a notice without the twin, that solo value mints a row and keeps binding afterwards, so
   there is a one-time split. The bound on this is the 36 single-use twin rows first seen in 2026.
3. **TED and DÖE copies can disagree.** If one copy carries the twin and the other does not, one notice splits
   between a GUID row and the identifier-NULL row. The dry plan counts this.
4. **Exemption (i) depends on state.** It only ever leans toward binding a value that already has a row, so its
   worst case is the status quo. The stock selection is corpus-level and does not depend on order.
5. **Some fallback rows already hold more than one body** from 234 (Hochbauamt CH 6956143, Vergabekammer DE
   1431661). Adding mentions there extends a wrong merge. The guard has to keep such names out, and R2 reads the
   targets.
6. **530-F rests on one verified specimen.** If R5 fails, 530-T ships alone.
7. **This decision leaves most of the class split** (80%). The precision-first price is recall. The way to reopen
   it is below.

## Reads before the build ships

- **R1, dry plan (in-app, writes nothing):** planned rows and mentions; per target, the identifier-NULL row's id,
  name, country and mentions; guard exclusions by reason; disagreeing copy pairs; canaries. The bar is exact, and
  any miss means stop:
  - 0 planned rows that span 2 or more Tenders;
  - 0 planned rows in the stable population;
  - 254, 255, 8378, 30530641, 3866, 3868 and 21987063 not planned;
  - 22759261 and 22759262 planned, with target 2294259;
  - every target has identifier NULL;
  - planned count ≤ 11,258 (530-T) and ≤ 3,511 (530-F). Anything above means the definition drifted.
- **R2, fallback precision (sampled):** read all of the **30 targets receiving the most planned mentions**, which is
  where a generic name would concentrate. Add **30 random planned rows**, stratified as review 15, buyer 10,
  other or tenderer 5. Score each row:
  - R: the target row is the same body;
  - N: cannot tell;
  - W: the target's name denotes more than one body, or the target holds another body's mentions.

  **Bar: 0 W in 60, N ≤ 3.** Zero in 60 puts the 95% upper bound on the error rate at about 5%. Any W extends the
  guard, then re-run R1 and read a fresh 30, which must also come back 0 W. Also read every planned name key with
  exactly 2 content words that starts with an office noun.
- **R3:** the Verify block's dry figures. The Göttingen residue is counted and its reason recorded.
- **R4 (530-F only):** included in R5 below.
- **R5 (530-F only):** every mention of every TED "other" row carries the 35-character 8-4-4-16 shape. No row of
  that shape spans 2 or more Tenders anywhere, doe+ted included. No stable row carries it. Read 15 random FR
  targets, with 0 W.
- **After the wet run:** the Verify block. Then re-run guid-census: single-use twin rows fall to about the guard's
  exclusions, and the stable population is unchanged at 345 rows and at least 311,552 mentions.

## 300 design:84-85, answered

The line reads "merge-inert singletons, not junk to dissolve". The answer is: **upheld for the class, amended for a
sub-class that is proven per notice.**

- **"Merge-inert" stands.** 312 found 75,548 distinct values over 75,555 rows, and `canonical_key` returns `None`
  for a v4 UUID. Nothing here contradicts either.
- **"Singletons" is now measured, and it is the defect.** 91.0% of rows span one distinct Tender. A key minted per
  ORG section per notice does not merge, so a split never heals.
- **"Not junk to dissolve" stands for the class.** It holds 7,300 rows spanning several tenders and a 345-row stable
  population (311,552 mentions, 96.2% spanning several tenders), and 70 stable rows sit in twin pairs. So there is
  no class-wide condemn and no class-wide dissolve.
- **The amendment:** a row is dissolved when its own notice shows two values for one name-safe name, the value never
  recurs beyond one Tender, and it is not already bound. Rows of the one platform shape with zero reuse in 3,511 rows
  are dissolved too. The grounds are evidence inside the notice or the platform, not that the value looks like hex.
  Every other GUID row stays a merge-inert singleton by decision (c), with its cost recorded here and in the docs.

## Reopen conditions

- Revisit (d) only in a form that does not depend on order and is name-safe. One example: a probation binding,
  where a fresh value binds only once it recurs in a second Tender, promoted from the kept `raw_identifier`. It also
  needs a fallback-row read that comes back 0 W.
- Revisit (b) for CH if a country × role breakdown shows a cell with 0 rows spanning several tenders (CH buyers
  look per notice, CH winners are stable).

## GETs (13 of 15, at least 2 s apart)

1. Vergabestelle
2. Tiefbauamt
3. Hochbauamt
4. Bauverwaltung
5. tenders by buyer=22728353 (empty: an old notice's row is not the current buyer)
6. tenders by buyer=22857048 (empty, same reason)
7. Tribunal administratif
8. Vergabekammer
9. SBH | Schulbau Hamburg: 3866/3868 plus identifier-NULL row 11219650 at 2,919 mentions
10. Gemeindeverwaltung
11. notice 277 content (the 8-4-4-16 shape)
12. org 2294259
13. org 16393
