# 584 unit 2: challenge of the KIO-NIP rehoming cohort and the guard spec (2026-10-24)

## Verdict

**The cohort is GO, unchanged.** That is `cohort.json`, cohort `584-2026-10-24`: 32 rehome, 6 keep, 0 parked.
- Every rehome lands on the same legal person the section prints, in the role it was published in.
- Every section binds one party.
- I found no row that needs a different verdict or target.

**The guard spec is NO-GO as written.** It has three blocking defects (D1–D3 below), and the issue's sizing is incomplete (S1).

Neither of these blocks posting and applying this cohort. Both block closing 584 or deciding the guard.

## Budget and what I read

All reads are logged in `requests.log` (entries tagged "challenger"). The raw files are under `challenge-raw/`.

| | preparer | challenger | workflow total |
|---|---|---|---|
| `/v1/sql` | 6 | 2 (`c1_verify`, `c2_org36`) | **8 of 10** |
| public GETs | 7 | 16 | **23 of 40** |

- Both SQL statements were plan-probed locally. Every step is an index seek or a PK/rowid seek, and no base table is scanned.
- `c1` is 74 seeks and took 0.40 s. `c2` uses the `_org` indexes plus one 6,000-row LIMITed index walk and took 0.75 s.
- There were no 408s, no retries, and no 5xx responses.
- The 16 GETs were sequential, with at least 3 s between them, and all returned 200.
- External lookups: 1 MF white-list fetch (10 NIPs) and 1 web search. Nothing was POSTed, no cargo or git command ran, and nothing was written outside `584-u2/`.

## 1. Re-derivation of the rehome rows from primary reads

I read 15 notices with `/v1/notices/{id}/content`. 14 are cohort notices, one per body, covering all 15 bodies. The 15th is a destination's own notice.

For each body I compared three things:
- the address printed in the cohort section;
- the MF white-list address of the destination's NIP (my fetch);
- the destination row in `c1` (it exists, its key matches, and its own mentions print that NIP under the body's name).

| body (mentions) | notice read | printed in the cohort section | destination: key; register (MF, my fetch unless noted); its own mentions (`c1`) | result |
|---|---|---|---|---|
| Uniwersytet Warszawski (13) | 25592320 | Krakowskie Przedmieście 26/28, dzpmed@uw.edu.pl. ORG-0001 is the only org referenced as Buyer, DocProvider, TenderEval and TenderReceipt. Review roles go to TPO-0001, which is bound to org 1617 | 7788, NIP 5250011266 (MF via the preparer; `c1` row present) | confirmed |
| MPWiK (5) | 23540012 | ORG-0001 (GDDKiA) **and** ORG-0002 (MPWiK) are both `OPT-300-Procedure-Buyer@ContractingParty`. Plac Starynkiewicza 5. The section copies GDDKiA's phone and e-mail (template copy) | 848, NIP 5250005662 (KRS via the preparer) | confirmed co-buyer |
| PKO BP (1) | 24300743 | the only Tenderer, 02-515 Warszawa (PKO's head office) | 10689264, NIP 5250007738 | confirmed |
| J&M (1) | 24425302 | ul. Litewska 6A, 22-100 Chełm. Consortium lead (`OPT-170`=1), with BICO (ORG-0003, its own NIP) as partner in the same TPA-0001 | 18347269 (provisional). Its only mention, notice 22130241 ORG-2, has the identical name and no id. Org 20534059 is "Ireneusz Malewicz … pod firmą I&M ENGINEERS z siedzibą w Chełmie": the same sole trader, also provisional. A web search found no NIP | confirmed (exact-name precedent G3(c)) |
| Terra Mota (2) | 24766144 | ul. Bardiowska 3, 38-300 Gorlice | 14553663, NIP 7382070551. MF: Bardiowska 3, Gorlice. Its 2 mentions carry 7382070551 | confirmed |
| Architektura Plus (1) | 24865719 | ul. Buforowa 70A/35, 52-129 Wrocław | 23202450, NIP 6871892878. MF: MICHAŁ MARCZAK, Buforowa 70A/35, 52-127 Wrocław. Its 2 mentions carry the NIP under this exact name | confirmed |
| Remondis Bydgoszcz (1) | 25374444 | ul. Inwalidów 45, 85-749 Bydgoszcz | 15146954, NIP 5540312757. MF: REMONDIS BYDGOSZCZ S.A., REGON 090484404, Inwalidów 45 | confirmed (same legal person as the S.A.) |
| Digitex (1) | 25522748 | ul. Platanowa 2, 81-855 Sopot | 20111694, NIP 5851476863. MF: DIGITEX Sp. z o.o. Sp.k., KRS 0000660257, Platanowa 2 Sopot. Its 6 mentions carry the NIP | confirmed |
| NOVUM S.A. (1) | 25564521 | ul. Tadeusza Czackiego 7/9/11, 00-043 Warszawa | 8998814, NIP 9491924705. MF: NOVUM S.A., Czackiego 7/9/11. Its 18 mentions carry the NIP | confirmed (not the Racławicka "Novum" provisionals) |
| WTU (1) | 25636957 | ul. Klementyny Hoffmanowej 6B/4, 30-419 Kraków. One body behind 5 bids (TPA-0001…5 all point at ORG-0003) | 13575097, NIP 8992680461. MF: 'WTU' Sp. z o.o., Hoffmanowej 6B/4 Kraków. Its 13 mentions carry the NIP | confirmed. A Wrocław-series NIP/REGON, but MF puts the company at the printed Kraków address |
| PZU Zdrowie (1) | 25649705 | name "…Odział Centra Medyczne w Gdańsku", address Rondo Daszyńskiego 4, 00-843 Warszawa (the head office) | 18389587, NIP 5272663852. MF: PZU ZDROWIE S.A., Rondo Daszyńskiego 4 (status "Niezarejestrowany" for VAT; the NIP is valid). Its 22 mentions carry the NIP | confirmed: the branch is only a label on the legal person |
| Evalu (1) | 25791954 | 04-690 Warszawa | 20491831, NIP 5262825916. MF: Sumaków 14, 04-690 Warszawa. Its 15 mentions carry the NIP | confirmed |
| Zarys International (1) | 25867833 | ul. Ziemska 44, 41-803 Zabrze (e-mail copied from KIO: odwolania@uzp.gov.pl) | 976, NIP 6481997718. MF: ZARYS INTERNATIONAL Sp. z o.o., REGON 273295877, KRS 0001213002, Ziemska 44 Zabrze | confirmed. Org 9241 (keyed by that REGON) is the same body |
| GZGK Wojcieszków (1) | 29173048 (preparer's GET) | Kościelna 46, 21-411 Wojcieszków | 22583689, REGON 061423397. Its only mention (notice 23906011) carries that REGON. **No** org holds `8250003819`, `PL8250003819` or KRS `0000423839` (`c1`) | confirmed; not parked |
| Pro3Ds (1) | 29920705 | al. Spółdzielczości Pracy 32, 20-147 Lublin | 22456216, NIP 7123245197. MF *working* address is Budowlana 6, 20-469 Lublin, which differs. But the destination's own notice 25235025 ORG-0004 prints "Pro3Ds Paweł Krawczyk", **ul. Spółdzielczości Pracy 32 lok. 428**, www.pro3ds.pl, NIP 7123245197, and all 3 of its mentions carry that NIP | confirmed |

Checks across all rows:
- **All 38 mentions still sit on org 49** (`c1`: 38/38), so none would be a no-op at apply time.
- All 15 targets exist with the stated key, and every `target_name` equals the live head name.
- `cohort.json` matches `584-u1/org49_foreign.json` key for key: 38 verdicts, `org_id` 49, all high, and `target_org_id` set on every rehome.

**Does each section bind one party?**
- The preparer's S2 shows every section on exactly one tender.
- The notice reads add the role side. Every cohort section is referenced only by roles of one body:
  - UW: buyer-side roles;
  - MPWiK: Procedure-Buyer;
  - supplier sections: `OPT-300-Tenderer` only.
- None of them is also referenced by a review or buyer role. 545's "more than one party" park rule does not fire.

**Right party in the right role?** Yes, on every row:
- MPWiK is a published co-buyer.
- J&M is the published consortium lead. The bid's partner, BICO, keeps its own section and org, so after the refold the consortium serves J&M plus BICO, not KIO plus BICO.

## 2. Corrections

**Verdict and target corrections: none.**

Optional notes, which do not block:

1. **Zarys 976 vs 9241, Remondis 15146954 vs 20376473, Digitex 20111694 vs 19005947.**
   - Each pair is one body, tied together by MF (NIP ↔ REGON).
   - The NIP-first rule picks the NIP-keyed row, so keep 976, 15146954 and 20111694.
   - 976's head name "ZARYS INTERNATIONAL GROUP SP .Z O.O." reads like the former group's name. The served name is cosmetic.
   - The three pairs are unmerged twins and belong in a merge or rename queue, not in this cohort.
2. **Cosmetic rationale error.** Four "National Appeals Chamber" keep rationales say "in a Lot-ReviewOrg (and Lot-Mediator) role". S2 shows Mediator only on 25398003; the other four are ReviewOrg only.

## 3. Parked rows

- `parked.json` is empty, and no row needs parking. GZGK is not a park: it has an existing REGON-keyed destination, and no org holds its NIP (`c1`).
- **The two tenders the cohort cannot fix stay out of it correctly.** On 816754 and 1058540 the tenderer section prints KIO's own name.

## 4. Apply-step corrections to `prep.md`

1. **"Expect a dry plan of exactly 32 moves" holds only if nothing else is pending.**
   - `apply_rehoming` reads every row of `org_mention_rehoming` with `applied_at IS NULL`, across all cohorts (`canonical.rs:33847`).
   - So the plan holds 32 moves only if no other cohort's high rehome with a target is pending.
   - The report's `pending` count will also include every keep ever recorded. Keeps are never stamped: this cohort's 6, 545's 12, and others.
   - Review the dry plan **tuple-wise** against the 32 `(notice, section, 49, target)` tuples in `cohort.json`, not by count. The wet ladder compares tuples anyway (`:33982`).
2. **The blast radius is correct as stated.**
   - 186 party and 19 bid-party rows belong to the mentions themselves, plus org-49 KIO rows on 17 notices.
   - I re-counted the 17 from S1: the 5 MPWiK notices plus 12 supplier notices. On 24425302 KIO sits on org 36, and on 24300743 it sits on BGK.

## 5. Scope finding S1 (major): org 36 holds the same NIP, is the live binding row, and carries a larger pocket of the same defect

**The facts.**
- `GET /v1/organizations/36`: "Krajowa Izba Odwoławcza", PL national **5262239325**, **205,496 mentions**, with merged vat PL5262239325.
- `c1` confirms that org 36 and org 49 hold the identical triple.
- The resolver keeps the **lowest id** per triple (`canonical.rs:21255–21262`, issue 439, `ORDER BY id`, first row wins). So every *new* mention printing KIO's NIP binds **org 36, not org 49**.
- S1 already showed org 36 holding mentions inside cohort notices: 23620337 ORG-0004, 24368074 ORG-0004 and 24425302 ORG-0004/0005.
- Unit 1 measured org 49 only. That is 30,567 mentions, 13% of the NIP's mentions.

**Measured (`c2_org36`, `challenge-raw/c2_org36_view.txt`).**
- **315 bid-party rows** on org 36, over 106 notices and 99 tenders, and **309 winner rows** over 96 tenders.
- Of the bid-party rows:
  - **133 tenderer mentions on 91 tenders print another body's name** (125 distinct names). Examples: VARIMED, SWISSPOL, "KONSORCJUM: LIDER: AROMA…", STANIMEX, METALZBYT-HURT, Bank Gospodarstwa Krajowego (24775003 ORG-0003), Politechnika Gdańska, Zarys International (25813620 ORG-0007), and "„REMONDIS KROeko” Sp. z o.o." (a *sister* company of Remondis Bydgoszcz, not the same body).
  - Only 9 mentions on 8 tenders print KIO's own name (the 816754 shape).
- **The buyer side is not sized.** The most recent 6,000 mentions of org 36 (notices 47.36–47.43 M, i.e. this week) already include "Gmina Skaryszew", "Ministerstwo Finansów" ×2, "Platforma eZamówienia", "Ever Cleaning", STANIMEX, METALZBYT-HURT, "KRAJOWA IZBA GOSPODARCZA" ×2 and "Krajowa Izba Kontroli".

**Consequences.**
- (a) The issue's "only 38 carry another name … small W pocket" holds for org 49 only. On the supplier side alone the class is at least 4× larger and still growing.
- (b) The 32 are still right to apply now. Org 36 needs its own cohort. Its sizing needs a census job: about 41 keyset pages, so it has no `/v1/sql` path within a 10-statement budget. Consortium names and sister companies (Remondis KROeko) must be parked or handled individually.
- (c) Prep's line "new UW notices printing this NIP will keep binding to KIO" is true, but they bind to **org 36**.
- (d) Any "org 49's foreign mentions stay at 0" monitor (`guard-spec.md` §6, last bullet) watches the wrong row.
- (e) The 36/49 duplicate is itself a same-identifier pair that was never merged.

## 6. Guard spec defects (`guard-spec.md`)

I checked every code reference in the spec against the tree. These are correct:
- `canonical.rs`: `:23198` E0, `:23251` guarded, `:23321` names read, `:23387` `guarded_refused`, `:23741` `(None, None)`, `:23816` 234 probe, `:15758` `publishes`, `:39011` `org_head_drift`;
- `project.rs`: `:5490` `gated_identifier`, `:6269` the call, `:6280` `guid_twin_gate`;
- `guid_twin.rs:460`; `crosswalk.rs:1327` / `:1404`;
- the indexes `organizations_kind_id` and `tender_version_parties_mention_key`.

The defects:

**D1 (blocking): the guard is modelled on the wrong holder.**
- "v's organization" is treated as org 49, but E0's `org_of.get(&key)` returns org 36 (lowest id, §5).
- `guard_sim.py` and the "established vocabulary" are therefore computed over the minority holder's 30,567 names, not the 205,496 the live bind compares against.
- The sim's headline, "refuses exactly the 32 and nothing else", was never tested on org 36's names.
- **Fix:** define the vocabulary per identifier triple over all holders, or over the org E0 actually returns, and re-run the simulation on org 36.

**D2 (blocking): one shared token is too weak; generic Polish nouns pass.**
- I ran the spec's own test D (`scripts/guard_sim.py`, org-49 vocabulary) on names; output in `challenge-raw/guard_probe.out`.
- These would **pass and bind KIO**:
  - "Bank Gospodarstwa Krajowego" (`krajow-`). This is **observed**: a BGK tenderer mention sits on org 36 today.
  - "KRAJOWA IZBA GOSPODARCZA" (observed ×2 in org 36's recent sample).
  - "Krajowa Izba Kontroli" (observed).
  - "Urząd Miasta Szubin" and "Urząd Gminy Wojcieszków" (`urząd`, via the UZP family).
  - "Publiczna Szkoła Podstawowa nr 1" (`publi-`).
  - "Krajowa Spółka Cukrowa S.A." and "Krajowy Ośrodek Wsparcia Rolnictwa".
- These were correctly refused: Gmina Skaryszew, Ministerstwo Finansów, Platforma eZamówienia, STANIMEX, METALZBYT, Ever Cleaning.
- Under option A (every PL holder), tokens like zakład, gmina, powiat, szpital, przedsiębiorstwo, miejskie, komunalnych and uniwersytet make unrelated bodies agree. For example, GZGK "Gminny **Zakład** Gospodarki **Komunalnej**" and Terra Mota's buyer "**Zakład** Usług **Komunalnych**" agree even on exact tokens.
- **Fix:** require agreement on a *distinctive* token, through a corpus-frequency (IDF) floor or a generic-noun stoplist that includes krajowy/krajowa, urząd, publiczny, izba, zakład and gmina. Or require the mention to cover the holder's distinctive tokens. The "refuses exactly the 32" table must be re-derived after that change.

**D3 (blocking): the scope "only the E0 exact-triple bind" leaves the canonical arm open.**
- PL VAT and NIP fold to one canonical key. `crates/ingest/src/crosswalk.rs:590` asserts `key(PL, vat, "PL5262239325") == key(PL, national, "5262239325")`, using this very NIP.
- Org 36 already carries `PL5262239325` as a merged vat identifier.
- So a foreign section printing "PL5262239325" binds through the Stage-2 canonical arm (`canonical.rs:23230`, `canon_hit`), or through the rekey arm, and never meets the E0 guard.
- **Fix:** put the name test at the point where any identifier arm commits to a holder, not only at E0. At minimum cover E0, rekey and canonical.

**D4 (should fix): the vocabulary learns from the mentions it is meant to judge.**
- "Families with ≥ 1 % of the holder's mentions" accepts any misprint that reaches 1 % of a smaller magnet. With the N = 50 floor, one buyer's repeated template does that.
- The floor is ambiguous: "with a floor of k = 50 mentions" could apply to the family or to the holder. `guard_sim.py` applies only `c >= 0.01*total`.
- **Fix:** take the vocabulary from a reviewed list (option B), or from the register name of the number (MF white list: 5262239325 = URZĄD ZAMÓWIEŃ PUBLICZNYCH), not from self-learned families.

**D5 (minor): two claims overstate.**
- §3 says "a rehomed mention whose notice is re-parsed … today would fall back to 49".
  - It re-resolves only if one of name, country, `raw_identifier` or `scheme` changes (`canonical.rs:15758–15763`).
  - If it does re-resolve, E0 returns org **36**.
- §5 says the guard catches the mirror shape on org 11786. It reaches only *new* binds, and only because E0 returns the lower-id UW row 11785 for REGON 000001258. The 173 mentions already on 11786 need a cohort, and `publishes` keeps them there.

**D6 (minor): option B and the role-family condition.**
- "Roles are known in `NoticeState::read`" covers the mention's own role in the notice being parsed.
- The holder's ≥ 99 % review-role share is a database property, so under B it can only be a compiled constant per reviewed identifier. Say so.
- Note also that KIO frequently appears as a touchpoint (`TPO-0001` in the UW notices, bound to provisional org 1617), so role shares must count touchpoint-aliased rows.

## 7. Recommended next steps

1. **POST `cohort.json` unchanged.**
2. Run apply-rehoming dry and compare the plan tuple-wise to the 32 tuples.
3. Run the wet apply, then `project`.
4. Read back 306091 (Verify): the buyer should be 7788. Also read back one supplier tender, e.g. 914542 (WTU, 5 lots, expected 13575097).
5. **Reopen 584's sizing for org 36:**
   - a census of org 36's mention names by role;
   - a second cohort following 545's rules, with consortium and sister-company parks;
   - a decision on merging the 36/49 duplicate.
6. **Revise `guard-spec.md` for D1–D3 (and D4)** before the guard decision. Re-run the simulation on org 36's names with the distinctive-token rule.
