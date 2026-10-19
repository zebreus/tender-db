# 540 — an R2.0.8 corrigendum (F14) serves a corrected address as `buyer`, whichever address it corrects

Status: ready-for-agent — measured 2026-10-19 (.scratch/tender-db/540-triage/, check/). Below id 18,579,247 the shape never occurs. 2011–2015 R2.0.8 corrigenda arrive as OTH_NOT prose with no Organization section (e.g. 17786082 = 079289-2014). There are 0 r208 Change sections in ids 1 to 18,579,247 (the 2,315 Change sections in 1–13M are all eForms). Across 320k ids in 2012, 2014 and 2015, CA_CE_CONCESSIONAIRE_PROFILE sits only at PROCEDURE/0. Not yet read: 557,040 ted-export-r208 notices mixed in with r209 at ids 18,579,251–27,161,439. Next: unit 3 (required, S), sweeping that tail in id windows for Change sections on r208 rows, TED-FORM 14, and CA_CE outside PROCEDURE/0. All zero → wontfix plus the MODIFIED_ADDRESS reason-string update. Any hit → read the WHERE of 3–5 exhibits and decide (unit 2 of the issue). Was: needs-triage — filed 2026-10-14 from issue 534's review round (a schema read, unmeasured). Next: read 3–5 F14
payloads that correct an address and see which role their new block is served under.
Kind: data quality (party roles; buyer grouping)
Relates to:
- 534 (legacy role folds), where this was read;
- 393 (the legacy role fold that maps `CA_CE_CONCESSIONAIRE_PROFILE` → `buyer`);
- 483 (buyer-role census).

## Verify

    curl -s "https://tenders.zebreus.click/v1/notices?publication_id=<an F14 correcting an address>" | jq -c '[.items[].id]'

- **done**: the corrected block of a non-buyer address (a review body's, say) is not served as a `buyer` party.
- **open (2026-10-14)**: unmeasured. Pick the exhibit in unit 1 and replace the placeholder above with its number.

## What was found

This comes from the R2.0.8.S02 eSender schema; no payload has been read yet.
- **Where the new block sits.** A corrigendum (F14) that corrects an address puts it under
  `ADDR_CORR` → `MODIFIED_ADDRESS` → `CA_CE_CONCESSIONAIRE_PROFILE`, next to a `WHERE` saying which address it corrects.
- **How it folds.** `legacy_role` folds `CA_CE_CONCESSIONAIRE_PROFILE` to `buyer`, so the corrected block is served as
  a buyer whichever address `WHERE` names.
- **Why it matters.** Buyers drive how notices group into tenders (481, 483) and the `?buyer=` filter. A corrected
  review-body or contact address would then add a second "buyer" to the tender.

## Proposed units

1. **Measure (S).** Find F14 notices with an `ADDR_CORR` block, by public GETs of `/v1/notices/{id}/content` on a
   bounded sample of R2.0.8 corrigenda. Read what `WHERE` names and which role the block is served under. If every
   `WHERE` names the buyer's own address, close this as harmless.
2. **Decide (S).** Fold by `WHERE`, or drop a non-buyer corrected block, or keep it with a distinct role. Then refold
   the cohort through `refold-fields` (size first).

## Triage (2026-10-19, workflow wf_2a43dfc2-d09: a measurer and an adversarial skeptic)

Files: `.scratch/tender-db/540-triage/`. The Status line above is the skeptic-corrected one.

### Measured

I ran the issue's Next step: find R2.0.8 F14s that correct an address and see which role the new block gets. No such payload exists in the corpus, so there was nothing to read. In the R2.0.8 era a corrigendum is published as an OTH_NOT prose notice, never as the structured eSender form ADDITIONAL_INFORMATION_CORRIGENDUM. So ADDR_CORR / MODIFIED_ADDRESS never arrive, and no corrected block is ever served as `buyer`.

The code reading in the issue is right for the shape it describes:
- In r209/rules.rs, ADDR_CORR and MODIFIED_ADDRESS are Groups and CA_CE_CONCESSIONAIRE_PROFILE is an Org.
- The Org arm in r209/parse.rs writes the pointer into the enclosing section. That is PROCEDURE, or CHG-n if the block sits inside ADD/DELETE/REPLACE.
- So a corrected block would show up as a second TED-CA_CE_CONCESSIONAIRE_PROFILE pointer, at PROCEDURE with ordinal 1 or more, or in another section. role_name → legacy_role would fold it to `buyer`, and buyer_side_mentions would then use it for the 369/481/482 keys.

I measured exactly that signature:
- In three r208 windows totalling 320k notices, the pointer appears only at PROCEDURE ordinal 0: 2012 (16.90M–17.00M), 2014 (17.78M–17.90M) and 2015 (18.30M–18.40M). In the 2015 window that one position holds all 83,197 rows, which also shows the query does find the buyer block.
- No `TED-ADDR_CORR.STATUS` rows in 17.78M–17.80M.
- In a 3,001-id slice of 2014 r208 (2,447 notices carry TED-FORM), TED-FORM never reads 14. 142 of 601 r208 notices in 17,786,000–17,786,600 have no TED-FORM at all.
- One of those, notice 17786082 (079289-2014), is an F14: HEADING 014EA, TD_DOCUMENT_TYPE 2. Its body is CONTENTS/STI_DOC prose ("Anstatt: … muss es heißen: IV.3.4 …") and it has no Organization section.
- Change sections (where ADD/DELETE/REPLACE land): issue 385 found none in 4.4M–13M. The first one in 13M–20.5M is 18,579,247, which is r209 (456425-2015, published 2015-12-26). The r208 band runs from 12.30M (2011) to that point, so it holds no structured corrigendum block at all.
- This agrees with issue 10, whose scope line says "OTH_NOT prose corrigenda" and whose daily counts list no F14.

Not swept: files in the r208 schema ingested later at high ids (issue 10 counts 157 in the 2019-01-02 daily, none of them an F14).


### Numbers

Requests used: 11 of the 12 allowed on /v1/sql, none retried, no 408s, each 3.2–5.1 s round trip including ssh. 10 public GETs, at least 2 s apart. Every query plan was checked locally first (plans.txt): each one is a seek on the primary key or a covering index over a bounded range.
- r01: ADDR_CORR.STATUS in 17,780,000–17,800,000 = 0.
- r02 / r03 / r10: CA_CE pointer outside PROCEDURE/0 = 0 in 17.78M–17.80M, 0 in 17.80M–17.90M, 0 in 16.90M–17.00M.
- r11: 18,300,001–18,400,000 has exactly one position, PROCEDURE/0, with 83,197 rows.
- r04: TED-FORM '14' in 17,786,000–17,796,000 = 0.
- r05: form mix in 17,786,000–17,789,000 is F1 118, F2 1,110, F3 880, F5 117, F6 99, F15 80, others under 10, no 14.
- r07: 142 of 601 notices have no form (all ted-export-r208, all parsed).
- r06: Change sections in 17.7M–17.9M = 0.
- r08: Change sections in 1–20.5M = 356,227, lowest id 11.
- r09: Change sections in 13M–20.5M = 353,912 sections on 149,924 notices, lowest 18,579,247 (r209).
- Era anchors: 12,304,080 is 150000-2011 (R2.0.7.S03, r208 profile); 16,956,407 is 150000-2012; 17,786,793 is 080000-2014; 18,363,603 is 200000-2015; 18,579,247 is r209 (2015-12-26).
- To re-take: the q11 query on any r208 window. Done means one row, PROCEDURE/0. A second row would mean the shape has arrived.


### Decision (as proposed; amended by the skeptic points below where they conflict)

Decision: drop it as `wontfix`. Do not fold by WHERE, do not drop the block, and do not add a distinct role. This is the issue's own close-as-harmless branch, made stronger: there is no WHERE to read because the block never arrives.

Reasoning:
1. The issue is about a shape that appears zero times in the corpus. The direct signature is a second CA_CE pointer, and it was 0 in 320k r208 notices across three years. The way an R2.0.8 F14 actually arrives (OTH_NOT prose) produces no party at all.
2. R2.0.8 is a closed era: TED moved to R2.0.9 on 2015-12-05, so no new notices of this kind will arrive. The only thing that could bring the shape in is a re-fetch of the archives, and the reopen condition in the Status line covers that.
3. Building a WHERE-based fold would mean guessing the format of a WHERE field we have never seen. Issue 385 declined to commit an r208 F14 fixture for the same reason: it "would assert a shape nobody has observed".

What would reverse this: any r208 notice with TED-FORM 14, a Change section, or a CA_CE pointer outside PROCEDURE ordinal 0.


### Units (proposed)

- Unit 1 (measure, S): DONE 2026-10-19. 11 bounded /v1/sql reads and 10 public GETs, everything under .scratch/tender-db/540-triage/ (q*.sql, r*.json, g*.json, plans.txt, notes.txt).
- Unit 2 (record, XS, no refold): Write the Status line above onto line 3 of issue 540. Replace the `## Verify` placeholder with the q11 query: done = one row PROCEDURE/0 (83,197 on 2026-10-19), open = any second row. Append the measurement under `## Comments`. Rewrite the MODIFIED_ADDRESS reason string in NOT_CONTACT_DATA_WRAPPERS (crates/ingest/src/project.rs ~17173), whose 'no F14 payload read' becomes: 'R2.0.8-era corrigenda arrive as OTH_NOT prose; no structured F14 in the corpus, 0 CA_CE outside PROCEDURE/0 in 320k r208 notices (issue 540)'. The test only asserts the string's length is over 40, so the change is comment-only. It rides the next ops/check.sh gate and needs no deploy of its own. Point issue 534's side-observation line (~379) at 540's result.
- Unit 3 (optional, XS): Sweep the late r208-schema files ingested at high ids (issue 10: 157 in the 2019-01-02 daily). Locate their id band with one public GET by publication_id, then run one q11-shaped query on it. Do it only if someone wants the residual closed; it does not block the wontfix.


### Skeptic (amendments)

- The 11 reads say what the triager reports. q01-q11 match r01-r11 exactly, the plans in plans.txt are range seeks, and g04 (17786082 = 079289-2014) is one PROCEDURE section with HEADING 014EA, TD_DOCUMENT_TYPE 2, CONTENTS/STI_DOC prose ('Anstatt: ... muss es heißen') and no Organization section. The code reading is also right: ADDR_CORR is a Rule::Group, ADD/DELETE/REPLACE are Section(Change), and the STATUS attribute is captured (parse.rs:80), so q01 is a valid probe.
- The wontfix rests on a wrong era boundary. Confirmation read c02 (.scratch/tender-db/540-triage/check/c02.json) finds 557,040 ted-export-r208 notices ABOVE 18,579,247, at ids 18,579,251 to 27,161,439, mixed in with r209. So the claims 'the r208 band runs from 12.30M to 18,579,247', 'R2.0.8 is a closed era from 2015-12-05' and 'no structured corrigendum block in the r208 band at all' do not hold. Issue 534 itself reads r208 T02 rows on 2016 tenders.
- r09 shows only that the LOWEST Change-section notice in 13M–20.5M is r209. It does not show that none of the 149,924 Change-carrying notices between 18.58M and 20.5M is r208, and nothing above 20.5M was read. No read covers the 557,040 late r208 notices for Change sections, TED-FORM 14, or a CA_CE pointer outside PROCEDURE/0. That 2016-era tail is the part of the corpus where a structured R2.0.8 F14 is most likely to appear.
- The residual is mis-sized and mis-filed. The proposal calls it 'issue 10's 157 in the 2019-01-02 daily' and makes it an optional Unit 3 that 'does not block the wontfix'. It is 557,040 notices and it is the measurement that decides the issue, so it is the required next unit. It needs splitting into id windows: a Change-section-by-profile read with CROSS JOIN notices (c01 shape) per window, and CA_CE positions restricted to r208 rows. All of it is runnable by the agent, so it is not needs-info.
- The low end checks out. c01 shows all 2,315 Change sections in ids 1–13M (r08 minus r09) are eForms (sdk 0.1–1.14). So below 18,579,247 there really are 0 r208 Change sections, and those figures can stay.
- '320k r208 notices' should read '320k ids'. The four windows add up to 320k ids, and in the 2015 window only 83,197 notices carry a CA_CE pointer, out of 100k ids. Also, every window is below 18.58M.
- Unit 2's new MODIFIED_ADDRESS reason string ('R2.0.8-era corrigenda arrive as OTH_NOT prose; no structured F14 in the corpus') states a conclusion the tail sweep has not reached yet. Hold it until the sweep is done.
- The proposed Verify runs q11 on the 2015 window. That window cannot distinguish the two states for the open part of the question, and it is a /v1/sql data-page read, not a public curl. Point the Verify at a window in the late r208 band instead, filtered to profile = 'ted-export-r208'.
