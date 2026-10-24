# 584: a name-agreement guard on national PL identifier binds (design spec, code half)

Status: a spec only, not built. **Revised 2026-10-24 by the unit-2 finalizer after the challenge** (`challenge.md` §5–§6).
- The challenger found three blocking defects (D1–D3) and three lesser ones (D4–D6). The finalizer checked each against the tree and the saved reads.
- D1, D2, D4, D5 and D6 are confirmed and fixed below.
- D3 is confirmed in its conclusion but **corrected in its mechanism**: today the canonical arm does not bind KIO's VAT spelling, because the 36/49 duplicate poisons that key. The hole opens if 36 and 49 are merged (§1, Scope).
- The pre-challenge text is kept at `final/guard-spec.v1-prechallenge.md`.
- Sizing still comes first (§6). The spec reopens 456's "design 1" narrowly. Issue 545 rejected design 1 for GB identifiers in favour of reviewed rehoming, so the guard needs its own decision, made on the census in §6.

## 0. What changed in this revision

| defect | verdict | where it is fixed |
|---|---|---|
| **D1** wrong holder (org 49 modelled; E0 returns org 36) | confirmed: `org_of` keeps the lowest id per triple (`canonical.rs:21255–21262`, issue 439), and both 36 and 49 carry `(PL, national, 5262239325)` (`c1`) | §1 "Holder" |
| **D2** one shared generic token passes | confirmed: the challenger's probe reproduces byte for byte (`challenge-raw/guard_probe.out`; BGK, KIG, "Urząd Miasta Szubin" and others bind KIO) | §1 "Distinctive token"; §3 re-run |
| **D3** scope "E0 only" leaves other arms open | conclusion confirmed, mechanism corrected (see Scope) | §1 "Scope" |
| **D4** vocabulary learned from the mentions it judges; floor ambiguous | confirmed: `guard_sim.py:64` applies only `c >= 0.01*total`, with no k = 50 floor | §1 "Holder names" (reviewed list only) |
| **D5** two overstatements | confirmed: `publishes` compares name, country, raw_identifier and scheme only (`canonical.rs:15758–15763`) | §3 "Re-parses"; §5 |
| **D6** option B role share; touchpoints | confirmed | §2 B; §4 |
| **New (finalizer)** a name-only guard refuses the review body's own misnamed sections | notice 25028774 ORG-0002 prints "Krajowa Izba Gospodarcza" with KIO's NIP, UZP's address (Postępu 17a), `odwolania@uzp.gov.pl` and a gov.pl/uzp site, in Lot-ReviewOrg/ReviewInfo/Mediator roles (GET #24, `final/get/`). It is KIO, misnamed by the publisher | §1 "Role condition" is now mandatory, not a mitigation |

## 1. The rule

A mention bound by identifier to a holder of **(PL, national, v)** keeps that bind unless all three of these hold:
- (a) the mention sits in a **buyer or tenderer role**;
- (b) the holder is a **review-role magnet** (see the role condition below);
- (c) the mention shares **no distinctive token** with the holder's reviewed names.

When all three hold, the mention binds **by name**, as if it carried no identifier. The raw value stays on the mention row. This is the same treatment rule 530-T gives a refused platform key (`identifier = None`, raw kept).

**Holder (D1).** The holder is the triple, not one org row. It means every org whose head identifier is `(PL, national, v)`; for KIO's NIP that is {36, 49}, both headed "Krajowa Izba Odwoławcza".
- E0 returns the lowest id, which is org 36 (205,496 mentions). Org 49 has 30,567.
- Every simulation, vocabulary and monitor is computed over the union, never over one row.

**Holder names (D4).** The names come from a reviewed list only, never from families learned from the holder's own mentions:
- the heads of every holder;
- their `organization_names` satellites, in all languages (org 49 carries ENG "National Appeals Chamber");
- the register name of the number. The MF white list gives 5262239325 = URZĄD ZAMÓWIEŃ PUBLICZNYCH, REGON 010828091. KIO sits inside UZP and has no NIP of its own.

The register name replaces the 1 % "established vocabulary", which accepted whatever any repeating template printed.

**Mention side.** The head BT-500 and every labelled variant (`Mention.variants`). A bilingual section passes on either language.

**Significant token.**
- Fold: NFC, then lower case.
- Drop Polish legal forms and function words: sp, spółka, z, o, oo, s.a., sa, akcyjna, ograniczoną, odpowiedzialnością, komandytowa, k, w, i, na, przy, ul, oddział, …
- Drop tokens shorter than 3 letters, and digits.

**Distinctive token (D2).** A significant token whose 5-letter stem is not on a generic-noun stoplist.
- The stoplist starts with: krajow-, izb-, urząd/urzęd-, publi-, państ-, polsk-, narod-, gmin-, powia-, miejs-, miast-, komun-, zakła-, przed-, szpit-, uniwe-, szkoł-, centr-, ośrod-, bank-, spółd-, wojew-, regio-, depar-, biuro, sekre-, preze-, gospo-, rolni-, and English national/chamber/office/public/procurement.
- The census (§6) replaces this hand list with a document-frequency floor over PL org head names, plus the hand list as a floor.
- For KIO's number, the distinctive holder tokens are **odwoławcza**, **zamówień** and **appeals**, and the initialisms are KIO, UZP and NAC.

**Agreement between two tokens.** Two tokens agree in any of these cases:
- they are equal;
- they share a prefix of at least min(5, len−1) letters. Polish declension needs this: "Prezes **Krajowej Izby Odwoławczej**" shares no exact token with "Krajowa Izba Odwoławcza";
- **after folding diacritics (ł→l), they are within edit distance 2 and both are at least 8 letters long.** KIO's own misspellings need this rule: on org 49 alone, "Odowławcza" ×24 (all casings), "Odwolawcza" ×1 and "Owoławcza" ×1. Those are the 26 mentions E1 refuses and E2 passes (§3).

**Initialism.** An upper-case token of 2–6 letters that spells a holder name's initials passes, for example "KIO" or "KIO UZP".

**Role condition (mandatory; was an optional mitigation).** The guard fires only when both of these hold:
- the holder is a review-role magnet: at least 99 % of its party rows are Lot-ReviewOrg, Lot-ReviewInfo or Lot-Mediator, counting touchpoint-aliased rows;
- the mention sits in a buyer role (Procedure-Buyer, Lot-TenderEval, Lot-TenderReceipt, Lot-DocProvider, Lot-AddInfo) or a tenderer role.

Without this condition the guard refuses the review body's own misnamed sections. That includes 25028774's "Krajowa Izba Gospodarcza" and, probably, org 36's "KRAJOWA IZBA GOSPODARCZA" ×2 and "Krajowa Izba Kontroli". Each would be re-bound by name to another body, which turns an R into a W.

**Scope (D3, corrected).** The name test sits at **every point where an identifier arm commits a mention to an existing holder**, not only at E0:
- the E0 exact-triple bind (`canonical.rs:23198`), for PL `national` **and** `vat` triples;
- the 453 re-key alias (`rekey_target`, the next branch);
- the Stage-2 canonical arm (`canon_hit`, `canonical.rs:23231`);
- the 452 guarded arm, whose own exact-name rule is stricter and already holds.

Country PL only. GB, the 572 agent/principal class and every other country are untouched.

*Correction to the challenger's mechanism.* Today a section printing "PL5262239325" does **not** canon-bind to KIO, for two reasons:
- The resolver's maps are built from `organizations` head identifiers only (`canonical.rs:21280–21284`). So org 36's merged `vat PL5262239325` (in `organization_merged_identifiers`) is no E0 key.
- Both 36 and 49 produce the same canonical key (`crosswalk.rs:590` asserts that the VAT and NIP forms fold together). The second owner therefore **poisons** that key (`canonical.rs:21312`), and `canon_hit` skips poisoned keys.

The challenger's conclusion still stands, for three reasons:
- **Merging 36 and 49**, which `challenge.md` §7 step 5 proposes, un-poisons the key. Every VAT spelling would then canon-bind to the survivor with no name test.
- A future head-`vat` row would E0-bind outside a `national`-only scope.
- Org 36's merged VAT identifier shows that the VAT spelling has reached KIO before, through a merge.

**So the guard must ship before, or together with, any 36/49 merge.**

## 2. Where it lives

There are two options. A is the general guard; B is the minimal one.

### A. The resolver, recommended for the general guard

**Location.** The identifier arms of `resolve_one_mention` (`crates/store/src/canonical.rs:23126`): E0 at `:23198`, rekey, then `canon_hit` at `:23231`.

**Mechanics.**
- On a hit for a PL `national` or `vat` key whose holder set qualifies under the role condition, read the holders' names. Issue 452's guarded arm already reads names on the writer connection at `canonical.rs:23321` (`SELECT name FROM organizations WHERE id = ? UNION ALL SELECT name FROM organization_names WHERE org_id = ?`).
- Add the reviewed register name from a small table.
- Memoize per holder for the run.
- On disagreement, take the identifier-less path, the `(None, None)` arm at `canonical.rs:23741`. Its 234 probe is `name_norm = ? AND country = ? AND identifier IS NULL` (`canonical.rs:23816`); it binds that row or mints one.
- Count refusals in a resolver tally, the way `guarded_refused` is counted (`canonical.rs:23387`). Never cache a refusal into `org_of`.
- **Option A needs the mention's role at resolve time.** The resolver resolves mentions, not party rows: `Mention` (`canonical.rs:7417`) carries no role. So either the notice's role map rides into `resolve_one_mention`, or the role condition has to be approximated. This is the main cost of A and must be settled before building.

**Do not reuse the 452 `guarded` path itself** (`canonical.rs:23251`), for two reasons:
- It needs an exact name match, which would refuse the inflected and UZP spellings.
- A mention refused there mints a new owner of the same key. That would mint a "Uniwersytet Warszawski" row keyed by KIO's NIP.

### B. Project, the minimal version

`gated_identifier` (`crates/ingest/src/project.rs:5490`, called at `project.rs:6269`) is pure: it sees (raw, scheme, country) and no organization.
- So a guard there can only be a **compiled, reviewed table** of template identifiers with their holder names.
- Today the table would hold one row: `PL:national:5262239325` → {Krajowa Izba Odwoławcza, National Appeals Chamber, Urząd Zamówień Publicznych; initialisms KIO, UZP}. The VAT form `PL5262239325` must match that row too.
- It would be applied next to `guid_twin::guid_twin_gate` (`project.rs:6280`; `project/guid_twin.rs:460`). That is the precedent: "Pure: compiled KEEP and names, no DB read."
- **D6.** The mention's own role is known while the notice is read (`NoticeState::read`). The holder's ≥ 99 % review-role share is a database fact, so under B it can only be a **compiled constant per reviewed identifier**: the row is in the table because a reviewer measured that share. The share must count touchpoint-aliased rows. In the UW notices KIO is `TPO-0001` (Lot-ReviewOrg/ReviewInfo), bound to provisional org 1617 rather than to 36 or 49 (notice 25592320).

This is option (a) of issue 584's unit 1, the reviewed exclusion list.

**Choose after the census.** If the class is a handful of template identifiers (review bodies, UZP, perhaps the Prokuratoria Generalna), ship B. If it is spread across many holders, ship A.

## 3. What it would have done here

**The pre-challenge table is withdrawn.** It was "test D refuses exactly the 32", from `scripts/guard_sim.py` over org 49 only, with a self-learned vocabulary. It failed D1, D2 and D4.

**Indicative re-run (finalizer, offline, no box reads):** `final/guard_sim2.py`, outputs `final/guard_sim2_E1.out` and `final/guard_sim2_E2.out`.
- E1 is the distinctive-token rule with stem(5). E2 is E1 plus the diacritic fold and edit distance ≤ 2.
- Both use reviewed holder names only (§1) and read mention head names. Roles are not modelled.

| population | E1 refuses | E2 refuses | what E2 refuses beyond the cohort |
|---|---|---|---|
| org 49, all 30,567 mentions | 60 / 22 names | **34 / 17 names** | the 32; plus "Prezes Izby" ×1 and "Krajowa Izba Gospodarcza" ×1, both KIO in review roles, which the role condition keeps |
| org 36, its 142 bid-party mentions (`c2`) | 133 | **133** | exactly the 133 foreign-named tenderer mentions; the 9 that print KIO's own name pass |
| org 36, its 6,000 most recent mentions (`c2`, 63 names) | 12 / 10 names | **10 / 8 names** | Ministerstwo Finansów ×2, KRAJOWA IZBA GOSPODARCZA ×2, Krajowa Izba Kontroli, Gmina Skaryszew, Platforma eZamówienia, STANIMEX, METALZBYT-HURT, Ever Cleaning |
| the challenger's probe names | — | every generic-word case refused (BGK, KIG, Krajowa Izba Kontroli, Urząd Miasta Szubin, Urząd Gminy Wojcieszków, Publiczna SP nr 1, Krajowa Spółka Cukrowa, KOWR) | KIO, KIO UZP, "Prezes Krajowej Izby Odwoławczej", "Urząd Zamóweń Publicznych" and "National Appeals Chamber" pass |

- **E2 is the candidate test.** E1 refuses 26 of KIO's own misspellings.
- **Not a census.** Org 36's other ~199,000 mentions are not available locally. The §6 census must re-derive this table over 36 ∪ 49 with roles before any decision.

**The 6 KIO spellings in cohort 584.**
- The 5 "National Appeals Chamber" sections pass in two ways. Each is bilingual (POL "Krajowa Izba Odwoławcza", S6), and "appeals" is a distinctive holder token through the satellite and the register-name list.
- "Urząd Zamóweń Publicznych" passes on "zamów-".

**The 32 under E2 plus the role condition.** All 32 sit in buyer or tenderer roles on a review-role magnet, so all 32 are refused and bind by name, per the S3/S4 listings:

| outcome | bodies | where they land |
|---|---|---|
| existing provisional row of that exact name (PL, identifier NULL) | UW, MPWiK (the address-bearing spelling), J&M, Remondis, NOVUM, Evalu, Zarys | UW → 1188351 (2,255 mentions); MPWiK → 22014758; J&M → 18347269; Remondis → 22053675; NOVUM → 6334556; Evalu → 10008177; Zarys → 6545028 |
| GZGK's generic provisional row | GZGK | 2199877: a poor landing, because it pools many gmina companies |
| no existing row, so a mint | Terra Mota, PKO BP, Architektura Plus, Digitex, WTU, PZU Zdrowie, Pro3Ds | new provisional rows |

So the guard turns W (the review board served as buyer or winner) into N (a provisional split of the right body).
- The reviewed cohort is still what lands the 32 on the identified rows (7788, 848, …).
- The guard prevents **recurrence**: future notices that print 5262239325 under another body's name. Those bind to **org 36** today, not 49.

**Re-parses (D5, corrected).** An already-recorded mention is re-resolved only when one of its four published facts changes: name, country, `raw_identifier` or `scheme` (`RecordedMention::publishes`, `canonical.rs:15758–15763`). A refold that re-reads the same facts leaves it where it is, rehomed rows included.
- If a rehomed mention *is* re-resolved today, E0 sends it to **org 36**, not 49.
- `apply_rehoming` will not re-move it: its verdict row keeps its real-move stamp, and `record_rehoming` never clears one (`canonical.rs:33735–33739`). See `runbook.md` §F.
- With the guard in place, such a mention would bind by name instead.

## 4. The risk of splitting legitimate renames

The role condition confines the guard to review-role magnets. That removes most of the PL renames risk the pre-challenge spec listed. Under option A, though, the census must show which other holders qualify.
- **Legal-form transformations keep a token, so they do not split.** This cohort has three: Remondis Bydgoszcz Sp. z o.o. → S.A., Zarys International Group sp.k. → Zarys International Sp. z o.o., and Digitex. "remondis", "zarys" and "digitex" agree.
- **These would split if the role condition were dropped:**
  - **(i) A full rename with no shared token** under one NIP, such as a successor company or a rebranded municipal company.
  - **(ii) Units printing their parent's NIP.** Since the 2017 VAT centralisation, a school, library or MOPS (jednostka budżetowa) prints its gmina's or powiat's NIP. Whether that split is a defect is 456/572's question; 572-L ruled "leave as published" for the GB analogue.
  - **(iii) A sole trader's business name against the person's name.** MF registers 6871892878 as "MICHAŁ MARCZAK".
  - **(iv) Branches named only by town.**
  - **(v) Generic-noun-only names** (D2). "Gminny Zakład Gospodarki Komunalnej" against "Zakład Usług Komunalnych" agree only on stoplisted nouns, so they no longer agree. That is the intended effect, but it widens (i)–(iv) for holders whose names are mostly generic.
- **With the role condition, a gmina, company or university holder is never a review-role magnet, so (i)–(v) cannot fire.** Option A must verify this in the census per qualifying holder.
- **Option A also needs a magnet floor**, so that thin holders never qualify: at least N = 50 party rows across the holder set.

## 5. What it does not fix

**The mirror shape: KIO's name printed under a buyer's identifier.**
- Org 11786, "Krajowa Izba Odwoławcza" with 173 mentions, holds UW's REGON 000001258. Notice 24300743 binds a KIO-named section to BGK.
- **D5, corrected.** The guard reaches only *new* binds of that shape, and only if a buyer holder qualifies, which the role condition rules out: UW and BGK are not review-role magnets.
- The 173 mentions already on 11786 stay there, because `publishes` holds them. They need their own reviewed cohort or a merge verdict.
- E0 today returns UW's lower-id row 11785 for REGON 000001258, so new KIO-named mentions with that REGON land on UW, not 11786.
- So the mirror shape needs a cohort plus a source-side rule, not this guard. It should be counted in the census, not assumed fixed.

**A section printing KIO's own name as the tenderer.**
- Tenders 816754 and 1058540 on org 49, plus 9 bid-party mentions on 8 tenders on org 36.
- Name and identifier agree, so no guard can help. Only a source-side rule could.

## 6. Size it first: a census, read-only, before any code decision

**Shape.** A dry supervisor job. It is a corpus walk, so it has no `/v1/sql` path: org 36 alone needs about 41 keyset pages.
- Walk `organizations` with `country = 'PL' AND identifier_kind IN ('national','vat')`, windowed by id on `organizations_kind_id`.
- **Group holders by triple (D1).** For each triple whose holder set has at least N = 50 party rows, read every holder's names. Read their mention names (`organization_mentions_org`) and those mentions' roles (`tender_version_parties_mention_key`), counting touchpoint-aliased rows.
- Score every mention with E2 plus the role condition.
- Extend 524's `org-head-drift` census rather than starting a new one (`Db::org_head_drift`, `canonical.rs:39011`). It already counts `stray_mentions` per org.
- Its agreement key is GB-tuned and exact (`crosswalk::altid_name_key` / `altid_keys_agree`, `crates/ingest/src/crosswalk.rs:1327` / `:1404`). PL needs the distinctive-token, fold, edit-distance and initialism test above, the triple grouping, and the role join that 524 left `not_measured`.

**Report:**
- refused mentions and holder triples, with the top 20 per triple;
- refused mentions by mention role (buyer / tenderer / winner / review) and by the triple's role family;
- the four buckets of §4: template magnet, unit under a parent NIP, rename or branch, typo;
- the mirror direction (§5);
- 30 samples per bucket for a reader, with R/W/N bars as in 545/574.

**Decision bar (proposed):**
- If template magnets hold at least 90 % of refusals and at most ~10 identifiers, ship **B** (the frozen table), before or with any 36/49 merge (§1 Scope). Feed the existing refusals to 317 cohorts.
- Otherwise ship **A** with the role condition. Do so only if its 30-sample read shows 0 W and N ≤ 2 among refusals in the unit and rename buckets.
- Either way, the census is re-run after the drain. It must show that **the triple's** foreign buyer/tenderer mentions stay at 0, on 36 and 49 together.
  - A monitor on org 49 alone watches the minority holder: new binds go to 36.
  - After this cohort, org 49 holds none, while org 36 holds at least 133 foreign tenderer mentions and an unsized buyer side.
