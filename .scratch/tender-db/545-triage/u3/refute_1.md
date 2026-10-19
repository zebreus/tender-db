# 545 unit 3: adversarial check of reviewer 1 (u2a indexes 7, 11, 32, 84, 90, 96, 105, 112, 113)

Read-only. I re-read the saved bodies first. Then I made **2 new public GETs**, both rc=200, no retries:
`/v1/notices/46834778/content` and `/v1/notices/47288227/content`. Each is logged in `api_requests.log` and in
`refute1_gets.log`, and was made through `refute1_get.sh` (flock on `.get.lock`, sleep 2 s, one retry at most for a
408 or 5xx). I made no /v1/sql calls, no POSTs, no admin calls and no git calls, and edited nothing outside `u3/`.
Local evidence came from `u2a/r7_sec.out.json`, `u2a/r8_dest.out.json`, `u2a/cand.json`, `u2b/tail_k6.json` and `u2a/s1.out.json`.

**Result: I refuted 0 of 9.** All 7 CONFIRMs survive, with each mention rehomed at high confidence. Both CUH PARKs stand.
Four rows carry flags that do not change their verdict (32, 96, 90, 105).

## What I attacked on every row

- **(a) Shared section.** `r7_sec` lists every party row and bid-party row of each evidence (org, notice).
  - Every evidence section carries exactly one role, Tenderer, or Procedure-Buyer for CUH.
  - On notice 46763175 the store holds two separate mentions for SEHSCT 30929662. They sit in different sections:
    - `ORG-GB-PPON-PLGY-6381-WWWR`, Procedure-Buyer.
    - `ORG-GB-PPON-PLGY6381WWWR`, the Tenderer, which is the one that moves.
  - The 317 key (notice, section) therefore moves only the PPL/PRS mention.
- **(b) Holder owns its identifier (no inverted premise).** Every holder except HFMA has many mentions:
  - MSE: 437 mentions, with GBNHSRAJ merged in.
  - CCS: 708.
  - VOA: 61.
  - SEHSCT: 194, and 191 of them are SEHSCT spellings.
  - DfT: 388.
  - DEFRA: 800.

  HFMA is discussed under row 32.
- **(c) Relation.** None of the printed names is a trading name, former name, group member, agent or consortium
  partner of its holder.
- **(d) Same entity.** I checked every destination against r8's name-range rows. For two of them a name printed elsewhere carries the company number:
  - PPL PRS: provisional 31536020 is printed "PPL PRS Limited (10376001)".
  - Abbott and NLA: the destination row also carries the firm's own PPON, merged in (Abbott PBMG-2473-WGVP, NLA PLMM-3896-RGZM). So the holder's PPON is not the firm's.
- **(e) Role.** Every rehomed mention is Tenderer, and every destination is a supplier.

## Per row

- **7 Abbott → 11546762.** Not refuted. Abbott already has its own PPON, merged into the destination, so MSE's PPON on Abbott is a copy error. Among GB rows, the only identified "abbott medical" row is 11546762; the other six are provisional spellings.
- **11 Thomas Silvey → 31560264.** Not refuted. CCS is the framework authority on the buyer's side. Of the "thomas silvey" rows, only 31560264 (GBCOH00360922) is identified.
- **32 TIAA → 11853802.** Not refuted, but flagged.
  - The reviewer calls 46783223 HFMA's "own notice". It is NHS England's.
  - My 2 GETs show that **every** known use of PHGQ-9419-WJLV is published by NHS England, and the PPON is a magnet across three suppliers:
    - HFMA ×4: 46783223 and 47288227, two separate One NHS Finance procedures.
    - TIAA ×1: 46818192.
    - The Academy for Healthcare Science ×1: 46834778.
  - So HFMA's ownership rests on one publisher's data entry.
  - The rehome holds anyway. TIAA's own PPON row 31546403 (PBRR-3532-NWYQ) carries 10 TIAA mentions ("TIAA LTD" ×7, "TIAA Limited" ×3), so PHGQ is not TIAA's. All four pool rows are TIAA Limited.
  - Destination flag:
    - 11853802 is keyed by the bare number "04546319". The decision text says "a GBCOH- or GBPPON-keyed row".
    - Read literally, that picks 31546403 (GBPPON, 10 mentions) over 11853802 (11).
    - The panel follows the Jeol precedent (12991212 "00939456"), and the entity is the same either way.
- **84 NLA → 30915240.** Not refuted. NLA's own PPON is merged into the destination. VOA has no role in National Highways' licence renewal.
- **90 PPL/PRS → 31546963.** Not refuted.
  - The number 10376001 is independently tied to the name (provisional "PPL PRS Limited (10376001)").
  - The destination's 5 mentions are "PPL PRS Ltd" ×4 and "PPL PRS ltd" ×1.
  - Flag: tender 8620561 stays split across three PPL PRS rows: 31546963 after the rehome, provisional 31552269 (46763209, junk id "GB-PPON-PPL/PRS"), and 31554686 (46768689, PVTY-2243-HBDQ). That is merge debt, not a wrong entity.
- **96 Fire Service College → 11314818.** Not refuted, but flagged.
  - 11314818 is keyed by the bare number "08102633" and has 3 mentions.
  - 31573940 is GBPPON PTQB-6315-ZCDD with 2 mentions ("Fire Service College Ltd", "Fire Services College Limited").
  - Read literally, the decision text would pick 31573940. Both rows are the same entity.
  - FSC is ex-DCLG and unrelated to DfT. UK SBS bought for UKRI.
- **105 Red Kite → 31578178.** Not refuted.
  - The only identified row is GBCOH08716366. Provisional 31545639 holds the procedure's other notice, 46748484 (FTS-internal id), so tender 8595247 stays split across 2 rows.
  - APHA, the buyer, is DEFRA's agency. Red Kite is not.
- **112 / 113 CUH: PARK upheld.**
  - On 46815909 CUH prints PPCP-8863-HDGN inside its own buyer section, next to GB-NHS-RGT, and 46796632 repeats it.
  - The Teleflex ×3 (YAS 46756087) and Hologic ×2 (BHRUT 46877124 and 46882065) mentions are Tenderers that other buyers listed under it.
  - So the identifier is CUH's, and these two mentions are out of the 545 premise. The fix is an identifier verdict giving GBPPONPPCP8863HDGN to CUH.

## Notes for the panel

- Add PHGQ-9419-WJLV, NHS England's magnet, to the magnet-PPON note next to 30926847 and PPCP-8863-HDGN. New NHS England notices may keep binding suppliers to it, so it is not a one-off copy-paste.
- Two destinations are keyed by bare numbers and are not GBCOH- or GBPPON-keyed: TIAA 11853802 and FSC 11314818. If unit 4 applies the decision text literally, the same-entity alternatives are 31546403 and 31573940.
