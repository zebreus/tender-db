# 584: a name-agreement guard on national PL identifier binds (design spec, code half)

Status: a spec only, not built. Sizing comes first (§6). It reopens 456's "design 1" narrowly. Issue 545 rejected design 1 for GB identifiers in favour of reviewed rehoming, so the guard needs its own decision, made on the census in §6.

## 1. The rule

A mention whose identifier is **(PL, national, v)** binds by identifier only if it **shares a significant token** with the names of v's organization. Otherwise the mention binds **by name**, as if it carried no identifier. The raw value stays on the mention row. This is the same treatment rule 530-T gives a refused platform key (`identifier = None`, raw kept).

**Mention side.** The head BT-500 and every labelled variant (`Mention.variants`). A bilingual section passes on either language.

**Organization side.** Three sources:
- the head name;
- every `organization_names` satellite, in all languages;
- the org's **established vocabulary**: the name families carried by at least 1 % of the org's mentions, with a floor of k = 50 mentions.

The vocabulary is needed because v's legal holder may print a name that is not the head. 5262239325 is UZP's NIP (MF white list), and "Urząd Zamówień Publicznych" is 752 of org 49's 30,567 mentions. Head plus satellites alone would refuse all of them (§3).

**Significant token.**
- Fold: NFC, lower case.
- Drop Polish legal forms and function words: sp, spółka, z, o, oo, s.a., sa, akcyjna, ograniczoną, odpowiedzialnością, komandytowa, k, w, i, na, przy, ul, oddział, …
- Drop tokens shorter than 3 letters, and digits.

**Agreement between two tokens.** They agree if they are equal, or if they share a prefix of at least min(5, len−1) letters. Polish declension needs the prefix rule: "Prezes **Krajowej Izby Odwoławczej**" (1,332 mentions) shares no exact token with "Krajowa Izba Odwoławcza".

**Initialism.** An upper-case token of 2–6 letters that spells an org name's initials passes: "KIO", "KIO UZP".

**Scope**, so the guard cannot become 456 design 1 by the back door:
- country PL, `identifier_kind = 'national'` (NIP, REGON or KRS), and only the E0 exact-triple bind;
- the holder has at least N = 50 mentions (a "magnet");
- GB, the 572 agent/principal class, and every other country are untouched.

## 2. Where it lives

There are two options. A is the general guard; B is the minimal one.

### A. The resolver, recommended for the general guard

**Location.** The E0 arm of `resolve_one_mention`: `crates/store/src/canonical.rs:23198` (`if let Some(&org_id) = org_of.get(&key)`).

**Mechanics.**
- On a hit for a (PL, national) key whose holder qualifies, read the holder's names. Issue 452's guarded arm already does this, on the writer connection, at `canonical.rs:23321` (`SELECT name FROM organizations WHERE id = ? UNION ALL SELECT name FROM organization_names WHERE org_id = ?`).
- Read the vocabulary from a small table built by the census job in §6.
- Memoize per org for the run.
- On disagreement, take the identifier-less path, the `(None, None)` arm at `canonical.rs:23741`. Its 234 probe is `name_norm = ? AND country = ? AND identifier IS NULL` (`canonical.rs:23816`); it binds that row or mints one.
- Count the refusals in a resolver tally, the way `guarded_refused` is counted (`canonical.rs:23387`). Never cache a refusal into `org_of`.

**Do not reuse the 452 `guarded` path itself** (`canonical.rs:23251`). It needs an exact name match, which would refuse the inflected and UZP spellings. And a refused mention there mints a new owner of the same key, which would mint a "Uniwersytet Warszawski" row keyed by KIO's NIP.

### B. Project, the minimal version

`gated_identifier` (`crates/ingest/src/project.rs:5490`, called at `project.rs:6269`) is pure: it sees (raw, scheme, country) and no organization. So a guard there can only be a **compiled, reviewed table** of template identifiers with their holder's vocabulary. Today that table would be `PL:national:5262239325 → {KIO, UZP spellings, National Appeals Chamber}`. It would be applied next to `guid_twin::guid_twin_gate` (`project.rs:6280`; `project/guid_twin.rs:460`), which is the precedent: "Pure: compiled KEEP and names, no DB read."

This is option (a) of issue 584's unit 1, the reviewed exclusion list.

**Choose after the census.** If the class is a handful of template identifiers (review bodies, UZP, perhaps the Prokuratoria Generalna), ship B. If it is spread across many holders, ship A.

## 3. What it would have done here

Measured offline by `scripts/guard_sim.py` over org 49's 30,567 mention head names (from `584-u1`, no box reads); output in `raw/guard_sim.out`.

| variant of the test | refused | of which the 32 | of which KIO/UZP-own |
|---|---|---|---|
| A: exact token, head + satellites | 2,289 | 32 | 2,257 ("Prezes Krajowej Izby…" 1,332+, UZP 733+, "KIO" 20, …) |
| B: stem(5), head + satellites | 816 | 32 | 784 (UZP forms, "KIO", "KIO UZP") |
| C: B + initialism | 795 | 32 | 763 (UZP forms) |
| **D: C + vocabulary (families ≥ 1 %)** | **32** | **32** | **0** |

Only D is acceptable. It refuses exactly the 32 and nothing else.

**The 6 KIO spellings.**
- **"National Appeals Chamber" × 5 still bind.** Each section is bilingual: ENG "National Appeals Chamber" plus POL "Krajowa Izba Odwoławcza" (S6). Org 49 also carries ENG "National Appeals Chamber" as a satellite. So the mention passes on its variant, and on the satellite even if it had no variant.
- **"Urząd Zamóweń Publicznych"** (the typo) binds under D only: the stems urząd/publi agree with the UZP family. Under A–C it would be refused, along with every UZP-only spelling.

**The 32 under D.** None of them is served as KIO any more. They bind by name, per the S3/S4 listings:

| outcome | bodies | where they land |
|---|---|---|
| existing provisional row of that exact name (PL, identifier NULL) | UW, MPWiK (the address-bearing spelling), J&M, Remondis, NOVUM, Evalu, Zarys | UW → 1188351 (2,255 mentions); MPWiK → 22014758; J&M → 18347269; Remondis → 22053675; NOVUM → 6334556; Evalu → 10008177; Zarys → 6545028 |
| GZGK's generic provisional row | GZGK | 2199877: wrong-ish, because it pools many gmina companies |
| no existing row, so a mint | Terra Mota, PKO BP, Architektura Plus, Digitex, WTU, PZU Zdrowie, Pro3Ds | new provisional rows |

So the guard turns W (the review board served as buyer or winner) into N (a provisional split of the right body). The reviewed cohort is still what lands them on the identified rows (7788, 848, …). The guard prevents **recurrence**: future UW notices that print 5262239325.

**Re-parses.** Already-recorded mentions are untouched while their notice publishes the same fields (`RecordedMention::publishes`, `canonical.rs:15758`). So the guard reaches only new mentions and re-parsed stale ones. That includes a rehomed mention whose notice is re-parsed, which today would fall back to 49 by identifier.

## 4. The risk of splitting legitimate renames

- **Legal-form transformations keep a token, so they do not split.** This cohort has three: Remondis Bydgoszcz Sp. z o.o. → S.A., Zarys International Group sp.k. → Zarys International Sp. z o.o., Digitex. "remondis", "zarys" and "digitex" agree.
- **These do split:**
  - **(i) A full rename with no shared token** under one NIP, such as a successor company or a rebranded municipal company.
  - **(ii) Units printing their parent's NIP.** This is the big PL-specific class. Since the 2017 VAT centralisation, a school, library or MOPS (jednostka budżetowa) prints its gmina's or powiat's NIP. "Szkoła Podstawowa nr 3" shares no token with "Gmina X", so it would be refused and split from the gmina. Whether that is a defect is 456/572's question. 572-L ruled "leave as published" for the GB analogue.
  - **(iii) A sole trader's business name against the person's name.** MF registers 6871892878 as "MICHAŁ MARCZAK". A mention reading only "Architektura Plus Design Studio" would be refused against a head "Michał Marczak".
  - **(iv) Branches.** "PZU Zdrowie … Oddział Centra Medyczne w Gdańsku" passes on "pzu" and "zdrowie". A branch named only by its town would not.
- **Mitigations:**
  - the magnet floor (N ≥ 50) and the vocabulary keep most holders' own variants passing;
  - a **role-family condition** would make the guard very narrow. Refuse only when the holder's mentions are at least 99 % review-role (Lot-ReviewOrg, Lot-ReviewInfo, Lot-Mediator) and the refused mention sits in a buyer or tenderer role. That is exactly KIO's shape, and it leaves gminas and companies alone. Roles are known in `NoticeState::read` (project.rs), so this fits option B naturally. Option A would need the census table to carry the holder's role family.

## 5. What it does not fix

- **The mirror shape: KIO's name printed under a buyer's identifier.** Org 11786 "Krajowa Izba Odwoławcza" holds UW's REGON with 173 mentions. Notice 24300743 binds a KIO-named section to BGK. The same guard catches that shape in the other direction, because "Krajowa Izba Odwoławcza" shares no token with the holder "Uniwersytet Warszawski" or "Bank Gospodarstwa Krajowego". It should be counted in the census, not assumed.
- **A section printing KIO's own name as the tenderer** (tenders 816754 and 1058540). Name and identifier agree, so no guard can help. Only a source-side rule could.

## 6. Size it first: a census, read-only, before any code decision

**Shape.** A dry supervisor job. It is a corpus walk, so it has no `/v1/sql` path.
- Walk `organizations` with `country = 'PL' AND identifier_kind = 'national'`, windowed by id on `organizations_kind_id`.
- For each org with at least N mentions, read its names and its mention names (`organization_mentions_org`) and the roles of those mentions (`tender_version_parties_mention_key`).
- Score every mention with test D.
- Extend 524's `org-head-drift` census rather than starting a new one (`Db::org_head_drift`, `canonical.rs:39011`). It already counts `stray_mentions` per org. But its agreement key is GB-tuned and exact: `crosswalk::altid_name_key` / `altid_keys_agree`, `crates/ingest/src/crosswalk.rs:1327` / `:1404`. PL needs the stem, initialism and vocabulary test above, plus the role join that 524 left `not_measured`.

**Report:**
- refused mentions and holders, with the per-holder top 20;
- refused mentions by mention role (buyer / tenderer / winner / review) and by the holder's role family;
- the four buckets of §4: template magnet, unit under a parent NIP, rename or branch, typo;
- the mirror direction (§5);
- 30 samples per bucket for a reader, with R/W/N bars as in 545/574.

**Decision bar** (proposed):
- If template magnets hold at least 90 % of refusals and at most ~10 identifiers, ship **B** (the frozen table). Feed the existing refusals to a 317 cohort like this one.
- Otherwise ship **A** with the role-family condition, and only if its 30-sample read shows 0 W and N ≤ 2 among refusals in the unit and rename buckets.
- Either way, the census is re-run after the drain to show that 49's foreign mentions stay at 0.
