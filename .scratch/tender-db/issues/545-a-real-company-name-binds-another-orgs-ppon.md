# 545 — a real company name on another organization's PPON binds that organization (Jeol on UKRI)

Status: ready-for-agent — unit 1 MEASURED 2026-10-19 (.scratch/tender-db/545-triage/, 10 bounded /v1/sql reads): about 1 in 10–14 GB strays names a different organization. PPON slice (1,577 of 14,338 orgs): 8 of 72 GB-PPON strays, all Tenderer-side (the Tees Valley trio are group bodies, so U), 3 with a firm identified home (Jeol → 31620129 GBCOH939456, EEP, Portsmouth) and 2 probable. COH slice (2,091 of 31,486): 6 of 87, 4 identified; Bedford/Comensura is a buyer+tenderer shared-section collision, parked. Corpus roughly 200–300 mentions on GBPPON/GBCOH-keyed orgs, including foreign names held twice that the stray rule misses (CEBR ×2 on IFS); orgs keyed otherwise and COH twice-held keys not measured. Verify still open (8664993 serves UKRI as Tenderer and winner). DECIDED: reviewed rehoming through issue 317's machinery (POST /admin/rehoming cohort 545 → apply-rehoming dry, which writes the rehoming-plan report → wet → project), HIGH verdicts with an existing destination only; any mention whose (notice, section) binds more than one party is parked; no resolver rule (this decides issue 456 design 1, recorded on 456; 8751605 Schneider/Sellafield joins the read-backs); not accepted as is. NEXT: unit 2a/2b, the candidate listing over the remaining GBPPON letters and GBCOH ranges, with a scripted twice-held check (no code, at most 12 reads per session). Was: needs-triage — filed 2026-10-15 from issue 529's design panel. Next: measure the class before choosing
anything.
Kind: data quality (organization identity)
Relates to:
- 529 (unit 3: the FASTER `Individual` on UKRI's PPON, accepted);
- 452 and 466 (identifier verdicts; the wrong-identifier class);
- 522 (the exact-triple bind comes before verdicts).

## Verify

    ssh root@zebreus.click "curl -s 'http://127.0.0.1:8080/v1/tenders/8664993'" | head -c 600

- **done**: the supplier `Jeol (U.K.) Limited` on notice 46834978 is served under its own organization, not UK
  Research & Innovation (30913706).
- **open (2026-10-15)**: tender 8664993 serves UKRI as a bidder. The mention `Jeol (U.K.) Limited` carries
  `GB-PPON-PDQJ-7126-JDHG`, UKRI's PPON, on 46834978, whose buyer is UKHSA.
  Confirmed from the public API on 2026-10-15 04:5x UTC: `/v1/tenders/8664993` ("Service and Maintenance x JEOL",
  060996-2026) serves the Tenderer as org 30913706 `UK Research & Innovation` with `mention_name` `Jeol (U.K.)
  Limited`. Its one bid names UKRI, and the buyer is 30914819 UK Health Security Agency.

## What was found

The resolver binds the exact identifier triple first (canonical.rs ~20063). A supplier that publishes another
organization's PPON lands on that organization, whatever its name says. Here a real company name, an instrument
maker, sits on UKRI's PPON, probably a copy-paste in the buyer's form. Unlike 529's stand-in `Individual`, the name
is a real, different organization. So this is a name-versus-identifier disagreement, not a placeholder.

A 452 `wrong` verdict cannot move the bind, for the reason 522 unit 1 gives. `org-head-drift` (issue 524) counts
`stray_mentions` by scheme: 1,162 GB-PPON stray mentions on 955 orgs (job 2492). Some share of those is this class.

## Units

1. **Measure.** From 524's census samples, or a bounded per-scheme read: how many GB-PPON (and COH) mentions carry a
   name that agrees with neither the org's head nor any other mention of it, and how many of those name a company
   that exists elsewhere in the corpus under its own identifier.
2. **Decide** from the measurement. Options to weigh:
   - a reviewed rehoming of the measured mentions (issue 317), which moves recorded mentions only;
   - a resolver rule that refuses the triple when the name clearly names a different, identified org;
   - accept as publisher error.

## Triage (2026-10-19, workflow wf_2a43dfc2-d09: a measurer and an adversarial skeptic)

Files: `.scratch/tender-db/545-triage/`. The Status line above is the skeptic-corrected one.

### Measured

I ran issue 545's unit 1 ("Measure"). Everything was read-only. I used 10 of the 12 allowed /v1/sql requests: one ssh transport failure (rc=255, before the query ran, not a 408) was retried once. I made 1 public GET. Before each read went to the box I read its plan locally on a copy of plan.db (python sqlite3). That copy lacks two prod indexes, so I created organizations_identifier_id and organizations_name_norm_id on it first. Every join read is a CROSS JOIN driven from a range on the identifier index, with GROUP BY led by o.identifier. All files are in /home/user/tender-db/.scratch/tender-db/545-triage/: SQL, plans, raw outputs, requests.log, notes.md, strays.py, judge.py, prefilter.py.

Reads:
- **Verify:** public GET /v1/tenders/8664993.
- **q1, exhibit:** UKRI's (org 30913706) distinct mention names.
- **q2 / q7, populations:** count of orgs with GBPPON* and GBCOH* identifiers, taken from the identifier index alone.
- **q3 / q8, slices:** every mention name of every org in one identifier range, grouped by (org, name, scheme). The PPON slice is GBPPONPC to GBPPONPE; the COH slice is GBCOH04 to GBCOH05. I replayed the census's stray rule locally from the org-head-drift census code (issue 524 unit 1, canonical.rs about lines 38840–39045): a key held once, under a top key held 2 or more times, that agrees with no other key. The alt key comes from the r524judge/keys.py port. "Keys agree" is approximated as equal once legal-form tokens are dropped.
- **q4 / q9, destinations:** for each foreign-looking stray, a range read on name_norm across all organizations, to see whether the named company exists under its own identifier.
- **q5 / q6 / q9, roles:** each foreign mention's section_id and its party role, from tender_version_parties.

Then I hand-classified every stray into five classes:
- **V:** the holder's own variant, typo or rename.
- **U:** a sub-unit, parent, subsidiary or predecessor of the holder.
- **T:** not an organization name (a lot title or similar text).
- **C:** a consortium naming the holder and a partner.
- **F:** a different organization, which is the 545 class.

Code read to explain the numbers:
- RecordedMention::publishes (canonical.rs about line 15575) keeps a recorded mention while its name, country, raw identifier and scheme are unchanged. So a rehoming survives refolds.
- The rehoming machinery from issue 317 already exists: POST /admin/rehoming, then the rehoming-packet, rehoming-plan and apply-rehoming jobs, then a refold.
- FTS section ids are ORG-<scheme>-<identifier>. So two parties that publish the same identifier share one mention row.


### Numbers

**Verify: still OPEN.** /v1/tenders/8664993 serves the Tenderer and the lot winner as 30913706 UK Research & Innovation, with mention_name "Jeol (U.K.) Limited". The buyer is 30914819.

**Exhibit (q1).** UKRI holds 12 distinct names over 3,594 mentions, all on PDQJ. Five are census-rule strays, and only one is the 545 class:
- Jeol: F;
- STFC: U (a UKRI council);
- "UK Reseach and Innovation": V;
- "...- Cabinet Office - Public Procurement": V;
- "United Kingdom Research and Innovation": V.

**Populations.** 14,338 orgs carry a GBPPON identifier (about 790 per second letter); 31,486 carry a GBCOH one.

**PPON slice (GBPPONPC to GBPPONPE).** 1,577 orgs (11.0% of PPON orgs); 991 have 2 or more mentions. It holds 72 GB-PPON strays on 58 orgs, and it contains the Jeol exhibit. Classified: V 40, U 12, T 8, C 1, F 11.
- **8 F are supplier-side (all role Tenderer):**
  - Jeol on UKRI;
  - Derek Lewis on Dewi's;
  - Pensions Policy Institute on Institute for Fiscal Studies;
  - Healthcare 21 NI on Primary Healthcare 21;
  - Equal Education Partners on Torfaen CBC;
  - Portsmouth Hospitals University NHS Trust on NHS England;
  - QCI (UK) on Placing Talent;
  - Watkins Coaches on Rhondda Cynon Taf CBC.
- **5 of those 8 exist elsewhere under their own identifier:**
  - Jeol: 31620129 GBCOH939456. Also fragmented as 12991212 "00939456" and 9534435 "939456".
  - Derek Lewis: 30928138 GBCOH01102211.
  - Healthcare 21: 15757939 (VAT) and 30915822 (COH).
  - Equal Education Partners: 31582974 (COH) and 30928130 (PPON).
  - Portsmouth: 30971975 GBPPONPBDT9319TTHZ.
- **2 exist only as provisional rows:** Pensions Policy Institute (31587812) and Watkins Coaches (2479435). **1 is absent:** QCI (UK).
- **3 F are buyer-side, all Tees Valley group bodies:**
  - Tees Valley Combined Authority (TVCA) on South Tees Development Corporation's (STDC's) PPON;
  - STDC on TVCA's PPON;
  - Hartlepool Development Corporation on TVCA's PPON.
  - TVCA and STDC are both identified; Hartlepool is provisional only.

**COH slice (GBCOH04 to GBCOH05).** 2,091 orgs (6.6%); 1,163 have 2 or more mentions. It holds 87 GB-COH strays, plus 23 GB-PPON strays, all variants.
- **6 F:**
  - Bedford Borough Council on Comensura's number, as buyer. Bedford is identified: PPON 30914266.
  - DDM Health: GBCOH07975193, on two rows.
  - Diabetes UK: PPON 31562654.
  - Land Use Consultants: GBCOH2549296, plus two more rows.
  - Autocross Euroshell: absent.
  - London Cactus: absent.
- About 5 more are ambiguous sister-company pairs. The rest are V or U.

**Extrapolation.** About 655 GB-PPON strays sit on PPON-keyed orgs (the census counts 1,162 in all). About 1,310 GB-COH strays sit on GBCOH orgs (census 1,690). The rest are on orgs keyed some other way.

**The stray rule misses foreign names held twice.** The PPON slice has 4 such supplier-side keys:
- Centre for Economics and Business Research (CEBR) ×2 on the Institute for Fiscal Studies. CEBR is identified under PPON 30928327 and COH 31542054.
- Pagabo ×2 on Cumbria, Northumberland, Tyne and Wear NHS FT.
- Wavemaker t/a WPP 650 ×2 on the Ministry of Justice.
- Charlton Park Academy ×2 on HM Land Registry.

So strays capture only about 2 in 3 of the class's keys.

**Size.** About 1 in 10 GB strays is a different organization: 11/72 PPON, 6/87 COH. Corpus-wide that is roughly 150 to 250 mentions, an order of magnitude from 17 sampled events. About 60% have an identified destination, which is often split across several rows.

**Prefilter.** Drop a stray if its distinctive tokens appear in the org's other names. That cuts 183 strays to 45 to review, with supplier-side F recall of 14 out of 14. It misses only the Tees Valley buyer trio, which share the token "tees".


### Decision (as proposed; amended by the skeptic points below where they conflict)

Decision (the owner's call, made on this measurement): fix the class with reviewed rehoming through issue 317's machinery: POST /admin/rehoming with cohort 545, then rehoming-plan, apply-rehoming and a refold. Apply only high-confidence F verdicts. A verdict needs a destination that already exists: a GBCOH- or GBPPON-keyed row, the most-referenced one when the destination is split across rows, or a provisional row whose name_norm matches exactly. Everything else stays parked, as issue 317 parks it. I reject both other options.

Why I reject the resolver rule ("refuse the triple when the name clearly names a different, identified org"):
1. **Wrong targets measured in a sample of only 159 strays:**
   - Newport City Homes on Tai Hedyn is a rename, yet an identified Newport City Homes row exists (21674343, identifier 30192R). The rule would move a correct mention to the old row.
   - British Geological Survey, a part of UKRI, has its own identified PPON rows (30915801, 31575787).
   - Group companies such as Tribal Education, Chubb Ltd and G4S Care & Justice carry their own company numbers.
2. **Destinations are split across rows.** Jeol UK has 3 identified rows, Land Use Consultants 3, DDM Health 2, Equal Education Partners 2. A fold-time rule would have to pick one by name matching, the over-merge risk that 351's genericness wall exists to stop. It would also run on every future notice.
3. **There is no reliable signal to key on.** Supplier-side F sits on buyer PPONs (UKRI, NHS England, Rhondda Cynon Taf, Torfaen) and on supplier ones (Dewi's, the Institute for Fiscal Studies, Placing Talent) alike. Issue 529 already rejected "the identifier's org only buys".

Why I reject "accept as publisher error": the harm is real and visible. The bid, and often the award, is credited to the identifier's holder, usually a large public body: UKRI is served as winner of a JEOL maintenance contract, Rhondda Cynon Taf CBC as a coach-company bidder, NHS England as a Portsmouth bid. About 60% of these mentions have a home that already exists. The fix is cheap in code:
- the rehoming path exists, takes HIGH verdicts, and survives refolds (RecordedMention::publishes);
- the errors are one-off copy-pastes on single notices, so moving the recorded mentions is the whole fix; new notices are not affected.

Scope:
- **In:** F of both roles, including the Tees Valley buyer trio where the destination is identified, and foreign names held twice.
- **Out:** V, U and C.
- **Routed elsewhere:** T names (lot titles, "AF145 ...", "NHS England DO NOT USE", "Small Airbags - Market Engagement") go as a note to 510/511's net. The split Jeol UK identifiers ("00939456", "939456", "GBCOH939456") go as a note to 470/473's leading-zero class.


### Units (proposed)

- Unit 2 — candidate listing, read-only, no code (S–M). Re-run q3_slice.sql's shape over the remaining GBPPON second-letter ranges (two letters per read, about 1,600 orgs and 2k rows each, 8–9 reads) and over GBCOH ranges of about 2,000 orgs each (about 15 reads), plan read locally first, at most 12 /v1/sql requests per session. Locally, replay strays.py plus the twice-held foreign-key check and prefilter.py. Expect roughly 500 candidates to review. Write the packet to 545-triage/. Out of reach: GB-PPON and GB-COH strays on orgs keyed another way (bare company numbers, VAT), about 40% of the census's 2,852. Record them as not measured.
- Unit 3 — review (M, agent panel). Classify each candidate V/U/T/C/F with its role (tender_version_parties) and its destination (a bounded name_norm range read, UNION ALL of seeks, as in q4/q9). A verdict is HIGH only when the name is unambiguously a different organization and the destination is an existing row: canonical GBCOH/GBPPON first, otherwise a provisional row with exactly the same name_norm. Park sister-company and ambiguous pairs (Rosslee / R Developments, MSS Water / Acorn Chemical). Produce a verdict JSON in the 317 rehoming shape.
- Unit 4 — apply (S). POST /admin/rehoming with cohort 545, run rehoming-plan dry, reconcile its count against the HIGH verdicts, then apply-rehoming wet, refold-notices over the touched notices, and project. Read back the Verify: 8664993's Tenderer and winner are served as 31620129 (or the chosen JEOL UK row). Spot-check 10 rehomed mentions and confirm the holders (UKRI, NHS England, Rhondda Cynon Taf, Torfaen) no longer serve those bids. Then close 545 with DONE <date> on the Status line.
- Side notes, not 545 work (XS). Append to 510/511's tail the T-class names on buyer orgs ('Lot B - Southern Sites Fish Passage…', 'AF145 Service Contract…', 'NHS England DO NOT USE', 'Small Airbags - Market Engagement'). Append to 470/473 that JEOL UK's company number is split across three identified rows (12991212 '00939456', 9534435 '939456', 31620129 'GBCOH939456'), and likewise Land Use Consultants (7799672 '2549296', 15715155 '02549296', 31593929 'GBCOH2549296').


### Skeptic (amendments)

- NOT RECONCILED WITH ISSUE 456. Issue 456 is the same class: 'a mention that publishes another organization's identifier binds to that organization, whatever its name says'. Its exhibit is Schneider Electric on Sellafield's PPON (UKRI tender 8751605). It is still open (ready-for-agent), and its 'design 1' is exactly the resolver rule this decision rejects. Unit 1 of 456 rides the same 524 census. The proposal never mentions 456. The tracker rule is 'pick an owner, do not defer'. Fix: record on 456 that 545 decides design 1 (rejected, in favour of reviewed rehoming), or fold 545 into 456. Add 8751605 to the cohort's read-backs either way. 456's Verify needs a done-condition that reflects the decision.
- THE BEDFORD/COMENSURA F CANNOT BE FIXED BY REHOMING. q9 shows two roles on tender 8816363, Procedure-Buyer and Tenderer, and both sit on the single section ORG-GB-COH-04085767 (notes.md records the same collision). Issue 317's design is 'ONLY THE MENTION MOVES', keyed (notice_id, section_id). Rehoming that mention to Bedford 30914266 would make Bedford the Tenderer on its own tender. Yet the decision puts it in scope ('F of both roles') and counts it as an identified destination. Unit 3 needs a guard: park any candidate whose (notice, section) binds more than one party or role in tender_version_parties. The section-collision finding never reached the decision or the units.
- THE TEES VALLEY TRIO IS CLASSED F, BUT ITS OWN DEFINITIONS MAKE IT U. U is defined as 'sub-unit, parent, subsidiary'. South Tees DC and Hartlepool DC are TVCA's mayoral development corporations. The slice itself holds 'South Tees Development Corporation (part of TVCA Group)' and '(part of the TVCA Group)' on TVCA's PPON. Teesside International Airport ×8 and Middlesbrough DC ×2 sit on the same org and are not counted. The decision then puts the trio 'In', which contradicts 'Out: U' and unit 3's rule to park sister and ambiguous pairs. Corrected: PPON F is 8/72, all Tenderer-side; there is no buyer-side F in the PPON slice.
- THE '5 OF 8 HAVE AN IDENTIFIED HOME' FIGURE IS OVERSTATED. judge.py itself flags two of the five. Derek Lewis is 'probable', on holder Dewi's Ltd. For Healthcare 21 NI, the 'relation to holder Primary Healthcare 21 is unproven'; the holder also carries 'Primacy Healthcare 21', and Healthcare 21 Group NI looks like a sister company, so unit 3 should park it. Firm identified homes: Jeol, Equal Education Partners and Portsmouth (3), plus 2 probable.
- THE PREFILTER RECALL IS MISLABELLED. Re-running prefilter.py: PPON 73→23 kept, F recall 8/11; COH 110→22 kept, 6/6. That is 183→45, all-F recall 14/17. The 14 includes Bedford, which is buyer-side. Supplier-side F is 13 (8 PPON + 5 COH), all kept. So '14 of 14 supplier-side' is wrong. Also, the prefilter only sees strays, so it was never tested on the twice-held keys.
- THE SIZE RANGE IS INCONSISTENT WITH ITS OWN COMPONENTS. notes.md gives about 100 PPON F, about 90 COH F, and about 73 twice-held PPON mentions (4 keys ×2 / 0.11). That sums to about 260, above the stated 150–250 ceiling. It also leaves out two parts nobody measured: twice-held keys in COH, and strays on orgs keyed some other way (about 40% of the census). The GBCOH04 slice holds company numbers from roughly 2000–2004, so it is not a random sample. Fix: state the range for GBPPON/GBCOH-keyed orgs only, after reclassification (about 200–300 with the trio removed and the twice-held keys added), and list the parts that were not measured.
- THE TWICE-HELD CHECK WAS DONE BY EYE. No file in 545-triage implements it: strays.py and prefilter.py only handle strays. It was not run on the COH slice. The '2 in 3' capture figure rests on 4 keys from one slice. I confirmed all 4 in q3_slice: CEBR ×2 on IFS, Pagabo ×2 on CNTW, Wavemaker ×2 on MoJ, Charlton Park ×2 on HMLR. Unit 2 says 'replay … the twice-held foreign-key check', but there is nothing to replay. Unit 2 must script it, for example the prefilter applied to every non-top key held at most k times. The CEBR case also looks consortium-like: IFS is Tenderer on the same lot of 8617082 in versions 1, 2 and 4.
- THE DESTINATION TIEBREAK IS MISSING. The rule 'a provisional row whose name_norm matches exactly' can match many rows. Watkins Coaches has two provisional rows (8878703, 2479435), and q4 shows about 500 provisional 'JEOL' rows. Unit 3 needs a tiebreak (most-referenced, same country) or must park.
- JEOL UK HAS A FOURTH IDENTIFIED GB ROW. q4 shows 17870866 GB '226825653' 'JEOL UK Ltd', which looks like a VAT number. The notes and the 470/473 side note list three rows. So the claim that a destination split across rows is common holds even more strongly.
- MACHINERY NAMING IN THE DECISION AND UNIT 4. 'rehoming-plan' is not a job: it is the report that apply-rehoming dry-run writes (supervisor.rs about lines 13258 and 13306). The flow is POST /admin/rehoming → apply-rehoming dry → wet (held against the stored plan tuple by tuple) → project/refold. apply-rehoming walks unapplied verdicts from every cohort, 317's 93 parked ones included. So unit 4 must reconcile against the plan's eligible/moves, not against pending.
- UNIT 2 IS MIS-SIZED. About 8 PPON reads plus about 15 COH reads is about 23 /v1/sql requests, but the cap is 12 per session. Split it: 2a for PPON (16 remaining letters), 2b for COH, or size it M with two sessions stated.
- VERIFIED CORRECT. Populations: 14,338 GBPPON and 31,486 GBCOH. Slice orgs: 1,577 with 991 at 2+ mentions; 2,091 with 1,163. Strays: 72 GB-PPON (+1 GB-NHS) on 58 orgs, and 87 GB-COH + 23 GB-PPON in the COH slice. I re-ran strays.py and the output is byte-identical. UKRI: 12 rows and 3,594 mentions. Classification counts match judge.py (V40/U12/T8/C1/F11). Requests: 10 /v1/sql and 1 GET, within the rules. Verify is still open. The RecordedMention::publishes claim matches canonical.rs:15578. The census figures 1,162 and 1,690 match issue 524. Every join plan is index-driven. No confirmation reads were needed; I used 0 of 2.
