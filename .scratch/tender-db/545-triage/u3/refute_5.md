# 545 unit 3, refuter 5: attack on reviewer 5's rows

Rows: u2b/verdicts2b.json indexes 122, 182, 188, 195, 200, 302, 303, 322 and 347.

## Result

**0 of 8 confirmed rows refuted. 0 of 11 rehome mentions refuted. The Northumbria park (322) stands.**

## What I read

I re-read the saved bodies for every evidence notice, holder, destination and tender. I also used these local files:
- the u2b slices (b1, b4, b5) and `q8_cohslice.out.json`, for every name each holder carries;
- `u2b/r10_sec.out.json`, for the stored section and role of each mention;
- `u2b/r11_dest.out.json`, the name-prefix scan of destination pools;
- the register snapshots `448-campaign/companies-house-2026-09-30.json` and `469-registry-pairs/u1/companies-house.json`;
- the 452/466 verdicts in `519-nhs-digit-codes/measure/m3-raw/admin_identifier.json`.

**New GETs: 6 of 9 allowed.** All returned rc=200, with no retries. Each waited at least 3 s after the last line of the shared log, then slept 2 s. They are listed in `refute_5_gets.log` and were made with `refute5_get.sh`.
- `/v1/notices/46755323/content` (Angel and Star's other notice)
- `/v1/notices/46762739/content` (Enventure's own notice)
- `/v1/notices/46736127/content` (Explorance)
- `/v1/notices/46740834/content` (Explorance)
- `/v1/notices/46756369/content` (Whistlejacket)
- `/v1/notices/46780298/content` (NHS Confederation)

**Check run by script over all 12 evidence (notice, section) pairs.**
- Each section holds exactly three fields: BT-500, BT-501 and BT-514.
- Each is referenced once: one OPT-300-Tenderer, or one OPT-300-Procedure-Buyer for LHP.
- No section is shared by two parties.
- The only same-number sibling is `ORG-GB-COH-11618190#2` on 46874082. It belongs to Angel and Star and is reached by its own TenderingParty, TPA-014202-2025-2-0.
- Every mention in r10 has a single role.

## The attack, row by row

### 122: Prosci UK Limited on NLA's PPON (46768774). Survives.

Could the PPON be Prosci's rather than NLA's?
- No. On the holder, 13 mentions under the GB-PPON scheme carry NLA names (b1): NLA Media Access x4, NLA Media Access Limited x2, "(NLA)" x1, NLA Media Access Ltd x2, Newspaper Licensing Agency (NLA) x3 and THE NEWSPAPER LICENSING AGENCY LIMITED x1. Prosci is the only foreign name.
- The register gives 03003569 as NLA MEDIA ACCESS LIMITED, formerly THE NEWSPAPER LICENSING AGENCY LIMITED.
- There is no trading name, group, agent or reseller link between a newspaper-licensing body and a change-management firm.

Section and destination:
- The section binds only Prosci (Tenderer). The tender's other notice, 46804992, carries only the buyer.
- The destination name scan uses the prefix "prosci", so it would catch any Prosci legal-form variant. It finds 31575934 (the only identified row) and a provisional 31539166. The Proscia rows are a different company.

### 182: DDM Health Ltd on NCCSF's number (46890437). Survives.

The strongest attack: could Norwich City Community Sports Foundation be DDM's delivery partner? The lot text says the winner "demonstrated a good plan for working in collaboration with partners in Norfolk". That still does not move the mention:
- the printed name is DDM, and the section binds only DDM;
- the tender's award notice 46893016 prints DDM again, is already served as 30953794, and is the lot winner;
- unit 466 ruled the same number wrong for DDM ("no link to DDM Health"), wrong/high, applied;
- the register gives 07975193 as DDM HEALTH LIMITED.

The cause is visible:
- NCCSF is one of Norfolk County Council's own suppliers. Its notice 46803407 shows Norfolk CC's framework with 04100611 plus CHC 1088239.
- So the same buyer picked the wrong supplier record.
- A partner's number would not make the mention the partner.

### 188: Diabetes UK on NHS Confederation's number (46786666). Survives.

Is the name wrong rather than the number? No. The cause is visible:
- NEW GET 46780298 is NHS England's own "NHS Employers Do OD 25/26" notice, published 2026-01-20. It prints "NHS Confederation (T/A NHS Employers)" with GB-COH-04358614.
- The Diabetes UK notice is the same buyer, 20 days later, so this is a supplier-record copy-paste.
- The register gives 04358614 as THE NHS CONFEDERATION.
- The subject, user involvement in the NHS Diabetes Programme, is Diabetes UK's established role. The section binds only Diabetes UK.

A residual blind spot that does not refute the row:
- The r11 scan used the prefix "diabetes uk". It cannot see an identified row headed by the charity's legal name (The British Diabetic Association, company 00339181), and no file shows whether one exists.
- So "identified-unique" may not really be unique, and the most-referenced rule could point at a different row of the **same** entity.
- That would split the entity across rows. It would not credit the mention to the wrong entity.

### 195: LHP x4 on Mobysoft's number (Procedure-Buyer). Survives.

Section, register and role:
- All 4 sections bind only LHP as Procedure-Buyer.
- The texts show LHP buying: "pay LHP rent", "LHP are require Azure", "secure LHP's DATA".
- The tenderers (Allpay, Jungle IT, Phoenix) and the review body (FTS-38715, served as provisional LHP 11383345) each have their own sections.
- The register gives 04546648 as MOBYSOFT LIMITED and RS007748 as LINCOLNSHIRE HOUSING PARTNERSHIP LIMITED.
- The Mobysoft row carries 9 Mobysoft-named mentions, including 2 under its own PPON.
- The Buyer role moves to a buyer, so the role check passes.

**Note, not a refutation.** This is not a one-off copy-paste. The same wrong number appears across three procedures, the latest on notice 47297256. That points to LHP's buyer profile. New LHP notices will bind to Mobysoft again after the rehome, which is outside the decision's "new notices are not affected" premise. Record this for unit 4's read-back, or route it to the buyer-profile class.

### 200: LUC on Enventure's number (46762813). Survives.

The cause is visible, from NEW GET 46762739:
- Luton Council's "BT1833 - Resident Surveys", awarded to Enventure Research with 04693096, was published 2025-11-14 at 10:32.
- LUC's "BT1810 - Infrastructure Strategy Study" was published by the same buyer at 11:54, 82 minutes later, with the same number.
- This is a carry-over from the previous form, not a joint bid. The section binds only LUC.

Destination 15715155 (02549296, merged PPON, 39 mentions):
- The register gives 02549296 as LAND USE CONSULTANTS LIMITED, Active.
- The other two rows carry the same number.

### 302: TPS Fruit & Veg Ltd on Explorance's number (46961181). Survives, with a caveat.

The number:
- 08987402 belongs to Explorance. NEW GETs 46736127 and 46740834 are Coventry University notices that print EXPLORANCE GLOBAL LIMITED with 08987402 and Explorance's own PPON PHLV-5135-LDTW in the same section, which reads like platform-supplied data.
- No same-buyer cause is visible: APUC 2023 against Coventry 2025. That does not change the F verdict, because a course-evaluation software firm is not a produce tenderer.
- The section binds only TPS (lot 2).

The destination:
- 21727460 is provisional, with exactly the printed name_norm, and is the only exact match.
- Without the wrong number, this mention would have bound by name to exactly that row. So the rehome reproduces the resolver's own name binding. That is why I do not count "provisional" as unsettled here.
- **Caveat:** 11707140 "TPS Fruit and Veg Ltd" (2 mentions) is the same entity, spelt with "and". A most-referenced tiebreak across both spellings would pick it instead. The rows split either way, but the mention cannot land on a different entity.

### 303: Don Whitley Scientific on Whistlejacket's number (46870983). Survives.

The number:
- NEW GET 46756369 shows Whistlejacket London (09021526) as a Tenderer on WRAP's "FRA084 Creative Services Framework". It is a creative agency, which cannot maintain Whitley A35 anaerobic workstations.
- Its other 8 mentions are all its own name. The section binds only Don Whitley.

Weak points that do not refute:
- Reviewer 5's "picker mis-pick" explanation is a guess. The two notices have different buyers, so no copy-paste is visible.
- 01342672 on destination 13064119 is not in the local register snapshot, so it is checked by name only. The prefix scan "don whitley" finds no other identified row.

### 347: Action Deafness on Angel and Star's number (46874082). Survives.

I tested a sharper attack: could 11618190 belong to an Action Deafness subsidiary, such as "Action Deafness Communications"? If so, parent/subsidiary would park the row.
- NEW GET 46755323 settles it. Warwickshire County Council and NHS Coventry and Warwickshire ICB, a different buyer, independently print "Angel and Star Care Services" with GB-COH-11618190. So the number is Angel and Star's.
- The register gives 06706042 as ACTION DEAFNESS, Active.
- In Derby's award list, contracts 1 and 2 are Action Deafness and then Angel and Star, both printed with 11618190. That looks like a copy error between adjacent entries.
- The evidence section is distinct from `#2` and has its own TenderingParty. Angel and Star keeps its own mention.

### 322: Northumbria. PARK stands (not a confirm, so there was nothing to refute)

I agree with keeping it:
- The register and 452 give 10167191 as NORTHUMBRIA UNIVERSITY SERVICES LIMITED, the university's subsidiary (verdict related).
- The holder serves `related_entity`.
- An out-of-hours IT helpdesk sold to other universities is the kind of commercial service a services subsidiary contracts. So the printed **number** may be the right legal tenderer, with the parent's name printed on it.
- The destination is also ambiguous: four identified university rows carry different PPON or UKPRN keys.

## For the panel

- None of the 8 confirmed rows is refuted.
- Four rows now have a visible cause that is independent of reviewer 5's evidence: Diabetes UK (NHS England), LUC (Luton, 82 minutes apart) and DDM (Norfolk) are same-buyer slips, and for Action Deafness a second buyer prints 11618190 for Angel and Star.
- Caveats to record, none of which blocks a rehome:
  - LHP's error is in its buyer profile, so it will recur on new notices;
  - TPS has a split provisional spelling;
  - Diabetes UK has the name-scan blind spot for a row under the BDA legal name;
  - Don Whitley's 01342672 is not checked against the register.
