#!/usr/bin/env python3
"""519 M6: the review (reader pass + a rule-based challenger pass) -> review.json.

usage: review.py      (paths fixed under 519-m6/)

Reader: the per-org decisions below, made against packet.json, the live owner reads
(raw/sql/r4_*, r6_owners, r7_coh_owners, r8_ppon_owners), the register pages (register/*.json),
the ODS answers (ods/*.json) and the public-API name searches (api/*.json), under the spec's
rubric (build-specs/519.md §3.3) and decisions D2-D5, D8.

Challenger: an independent, mechanical re-derivation from the same evidence (NOT a second agent;
see the deviation note in reading.md): unique live owner of the correction, register name test,
mention-name test against the target, site/place/collaborative/other-body screens. It agrees only
when verdict, correction, expected shape and target all match the reader's row.
"""
import collections, html, json, os, re

D = '/home/user/tender-db/.scratch/tender-db/519-m6'
P = json.load(open(os.path.join(D, 'packet.json')))
CASES = {c['org']: c for c in P['cases']}
SURV = P['code_survivors']


def rows(name):
    return json.load(open(os.path.join(D, 'raw', 'sql', name + '.out')))['rows']


OWN = collections.defaultdict(list)          # canonical key -> [(org, literal, name)]
for r in rows('r4_0') + rows('r8_ppon_owners') + rows('r9_ppon_owners2'):
    s = r[1][6:] if r[1].startswith('GBPPON') else r[1][4:]
    if (r[0], r[1], r[2]) not in OWN[('ppon', s)]:
        OWN[('ppon', s)].append((r[0], r[1], r[2]))
for r in rows('r7_coh_owners'):
    n = r[1]
    for p in ('GBCOH', 'GB'):
        if n.startswith(p) and len(n) - len(p) == 8:
            n = n[len(p):]
    OWN[('coh', n)].append((r[0], r[1], r[2]))
REG = {}
for f in os.listdir(os.path.join(D, 'register')):
    d = json.load(open(os.path.join(D, 'register', f)))
    REG[d['number'].replace('GBCHC', 'CHC')] = d
CH = 'https://find-and-update.company-information.service.gov.uk/company/'
ODS = 'https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/'
API = 'https://tenders.zebreus.click/v1/organizations'

# ----------------------------------------------------------------------------------------------
# Reader decisions. (verdict, correction, shape, target, confidence, route, rationale, evidence)
# shape: merge | move | unkeyed | none | hold   (hold = posted at medium, never re-keyed)
# ----------------------------------------------------------------------------------------------
R = {}


def d(org, corr, shape, target, conf, route, why, ev, verdict='wrong'):
    R[org] = dict(verdict=verdict, correct_identifier=corr, shape=shape, target=target,
                  confidence=conf, route=route, rationale=why, evidence=ev)


def body_ppon(org, serial, target, why, ev, conf='high', route='body-ppon'):
    d(org, 'GBPPON' + serial, 'merge' if conf == 'high' else 'hold', target, conf, route, why, ev)


# --- QHL15E: fixed by decision §4 and the U3 panel (spec §4.3 + Check fix 2)
body_ppon(31601033, 'PWQY9673JBYN', 30916123,
          'QHL15E is two codes glued (QHL + 15E) and answers ODS 404; all 10 live mentions name NHS Birmingham and Solihull ICB. '
          'Decision §4 and the U3 panel route it to QHL survivor 30916123, the standing PPON org GBPPONPWQY9673JBYN.',
          'unit2-decision.md §4; u3/u3-panel.json; R3/r8 owner 30916123')

# --- body-ppon (D2): an NHS body's own row, every mention name the body's, a standing PPON org that is the body
for org, serial, tgt, why in [
    (31597984, 'PBMJ3394DBQQ', 31564277, '1051858 is a bare number, no ODS code; the one mention names Warrington and Halton Teaching Hospitals NHS FT, whose same-name PPON row (RWW survivor) is 31564277.'),
    (31617199, 'PYHT5943VXGV', 30913986, '110001 is a local supplier number, ODS 404; both mentions name Gloucestershire Hospitals NHS FT, whose PPON row is the RTE survivor 30913986.'),
    (31592495, 'PHNB6299PVMY', 30972038, '110095 is a local number, ODS 404; all 5 mentions name Salisbury NHS FT, whose PPON row is the RNZ survivor 30972038.'),
    (30913869, 'PNWR4212QWHQ', 30913999, '110100 is a local number, ODS 404; the 7 mentions name Somerset NHS FT (one with the location suffix Yeovil), whose same-name PPON row is 30913999.'),
    (31592496, 'PXRL3879BBGR', 30915634, '110149 is a local number, ODS 404; all 5 mentions name Royal United Hospitals Bath NHS FT, whose same-name PPON row is 30915634.'),
    (31621091, 'PGZV5874HRZD', 31552751, '1031647539 is a bare number; the mention names Cheshire and Wirral Partnership NHS FT, whose PPON row is the RXA survivor 31552751.'),
    (31594293, 'PGZV5874HRZD', 31552751, '1111 is no ODS code; the mention names Cheshire and Wirral Partnership NHS FT, whose PPON row is the RXA survivor 31552751.'),
    (31644655, 'PGJT9638TWQR', 30919565, '10775747 is the register number of Keynes Informatics Ltd (dissolved), an unrelated company; the mention names Liverpool University Hospitals NHS FT, whose PPON row is the REM survivor 30919565.'),
    (31545892, 'PTTC1488WBVZ', 30916237, '400022 is a local number, ODS 404; the mention names Hertfordshire Community NHS Trust, whose same-name PPON row is 30916237.'),
    (31601064, 'PXRJ1463RVVN', 31567287, '130110 is a local number, ODS 404; the mention names South West Yorkshire Partnership NHS FT, whose same-name PPON row is 31567287.'),
    (31607612, 'PLMH6718XNVJ', 31552078, 'D4UY is sub-ICB code D4U1Y with a digit dropped; the mention names NHS Frimley ICB, whose PPON row is the QNQ survivor 31552078.'),
    (31595790, 'PMBT7532HJYG', 31596679, 'G6V25 is G6V2S (North London NHS FT) mistyped; the mention names North London NHS FT, whose same-name PPON row is 31596679.'),
    (31624070, 'PBCG3899RLCL', 31573365, 'The value is the typed label NHS HUMBER AND NORTH YORKSHIRE ICB - 02Y, 02Y being its own sub-ICB location; both mentions name the ICB, whose PPON row is the QOQ survivor 31573365.'),
    (31623762, 'PYCP3777NMJZ', 30917465, 'The value is the typed label of RP5DR, Doncaster Royal Infirmary, a site of the same trust; the mention names Doncaster and Bassetlaw Teaching Hospital NHS FT, whose same-name PPON row is 30917465.'),
    (31564001, 'PRVB1661VNRY', 31535615, 'ocds-h6vhtk-05029b is an OCDS release id, no ODS code; both mentions name Calderdale and Huddersfield NHS FT, whose PPON row is the RWY survivor 31535615.'),
    (31542386, 'PGCG2135TXWL', 31581032, 'ocds-h6vhtk-05091a is an OCDS release id; the mention names Countess of Chester Hospital NHS FT, whose PPON row is the RJR survivor 31581032.'),
    (31544072, 'PGCG2135TXWL', 31581032, 'ocds-h6vhtk-058ffd is an OCDS release id; the mention names Countess of Chester Hospital NHS FT, whose PPON row is the RJR survivor 31581032.'),
    (31551020, 'PDXN7482RVVM', 30969250, 'ocds-h6vhtk-05a9be is an OCDS release id; the mention names North Cumbria Integrated Care NHS FT, whose PPON row is the RNN survivor 30969250.'),
    (31564748, 'PCLY6219YPTV', 31542352, 'ocds-h6vhtk-05d28a is an OCDS release id; the mention names NHS Leicester, Leicestershire and Rutland ICB, whose same-name PPON row is 31542352.'),
    (31549692, 'PCLY6219YPTV', 31542352, 'QK1000 is LLR ICB code QK1 with 000 glued, ODS 404; all 16 mentions name NHS Leicester, Leicestershire and Rutland ICB, whose same-name PPON row is 31542352.'),
    (31613106, 'PJTG6389PMXW', 31574072, 'RA09 is the trust code RA9 with a 0 inserted; the mention names Torbay and South Devon NHS FT, whose same-name PPON row is 31574072.'),
    (31611764, 'PRBY9272TQGX', 31571667, 'RF1 is the trust code R1F transposed, ODS 404; all 4 mentions name Isle of Wight NHS Trust, whose same-name PPON row is 31571667 (D2 names this route).'),
    (31566459, 'PLYP5457LQCD', 31544250, 'RV09 is the trust code RV9 with a 0 inserted; the mention names Humber Teaching NHS FT, whose same-name PPON row is 31544250.'),
    (31612078, 'PDVQ1486BNXY', 31554134, 'SC351 is no ODS code (404); the mention names Derbyshire Community Health Services NHS FT, whose same-name PPON row is 31554134.'),
    (31597313, 'PDVQ1486BNXY', 31554134, 'Z2576474 is no ODS code; the mention names Derbyshire Community Health Services NHS FT, whose same-name PPON row is 31554134.'),
    (31575689, 'PQNZ4473TCJQ', 31580764, 'T6YOW is T6Y0W with O for 0, ODS 404; the mention names NHS Norfolk and Suffolk ICB, whose same-name PPON row is 31580764 (D2 names this route).'),
    (31606280, 'PCXT8262NYRV', 31576333, 'UK1 - London is a NUTS code, no ODS code; the mention names NHS North East London ICB, whose same-name PPON row is 31576333.'),
    (30974734, 'PPBC2775WXPX', 31537637, 'ukf22 is a NUTS code (ODS 404); the mention names Leicestershire Partnership NHS Trust, whose same-name PPON row is 31537637.'),
    (31608597, 'PRXC6549RWBT', 30917613, 'UKG23 is a NUTS code (ODS 404); both mentions name North Staffordshire Combined Healthcare NHS Trust, whose PPON row is the RLY survivor 30917613.'),
    (30915329, 'PTNY7183GXBY', 31559449, 'ukn06 is a NUTS code (ODS 404); all 11 mentions name Northern Ireland Blood Transfusion Service, whose same-name PPON row is 31559449.'),
    (30971947, 'PZDC7131GJTH', 31556601, 'W248289 is no ODS code; the mention names South Tees Hospitals NHS FT, whose PPON row is the RTR survivor 31556601.'),
]:
    body_ppon(org, serial, tgt, why + ' Every mention name is the body\'s and the PPON org stands under that literal (R3/r8).',
              f'packet mention names; R3 + r8 owner {tgt}')

# body-ppon rows held at medium (split reviews: a place, another body named by the value, two PPON orgs)
body_ppon(31624630, 'PMYZ6944BYLH', 31556600,
          'Of 2 mentions, one names NENC ICB and one "NENC ICB - Tees Valley Place", a place (D2 exclusion); 16H answers ODS 404. '
          'Held at medium: a place never takes a PPON merge, though the ICB PPON row 31556600 (QHM survivor) is the body.',
          'packet mention names; r8 owner 31556600', conf='medium')
body_ppon(31623760, 'PBLN7375RYXJ', 31535006,
          'The one mention names Gateshead Health NHS FT, but the value is the typed label of NHS England region Y63, another body. '
          'Held at medium: the value names another body, so the same-name PPON row 31535006 is not plain enough for a merge.',
          'packet raw identifier; r8 owner 31535006', conf='medium')
body_ppon(31605690, 'PTWP8794GHJN', 30926718,
          'OC415501 is Collaborative Procurement Partnership LLP (dissolved), a procurement collaborative the trust bought through; the mention names Leeds and York Partnership NHS FT (PPON row 30926718, RGD survivor). '
          'Held at medium: the value names a related body that may itself have been the party.',
          CH + 'OC415501', conf='medium')
body_ppon(31534565, 'PHHT9556BWJJ', 31535216,
          '11911171 is Care City Innovation C.I.C., a NELFT-linked CIC; the mention names North East London NHS FT (PPON row 31535216, RAT survivor). '
          'Held at medium: the value names a related body that may itself have been the party.',
          CH + '11911171', conf='medium')
body_ppon(31570955, 'PDWV5543BPQJ', 30916559,
          'GB-CHC-1053767 is a charity number (a separate legal body; the Charity Commission page refused the fetch, HTTP 403); the mention names Hertfordshire Partnership University NHS FT (PPON row 30916559). '
          'Held at medium: the value names a charity, likely the trust\'s own charity, not the trust.',
          'https://register-of-charities.charitycommission.gov.uk/charity-search/-/charity-details/1053767 (403)', conf='medium')
body_ppon(31589609, 'PDCY5582YRHQ', 30917018,
          'The value is PPON PQBL-2761-HHXG with a stray B; that PPON is carried by 31541470 "The Newcastle Hospitals NHS FT" (7 Newcastle mentions), while the RTD survivor is the PPON row 30917018. '
          'Held at medium: two standing PPON orgs of one body (a split for a later unit), so the reader picks the survivor (D2) and a challenger could pick 31541470.',
          'R4 owner 31541470; r6 names; code_survivors RTD -> 30917018', conf='medium')

# --- ppon-value
for org, serial, tgt, why in [
    (31567746, 'PGCG2135TXWL', 31581032, 'All 5 mentions name Countess of Chester Hospital NHS FT; the PPON is carried by 31581032, the RJR survivor of that name.'),
    (31577998, 'PGZM4323VGQT', 30929970, 'The mention names North West Anglia NHS FT; the PPON is carried by 30929970, the RGN survivor of that name.'),
    (31600190, 'PHGM8136GWTX', 31535091, 'The mention names NHS Kent and Medway ICB; the PPON is carried by the same-name org 31535091.'),
    (31550843, 'PHHT9556BWJJ', 31535216, 'All 3 mentions name North East London NHS FT; the PPON is carried by 31535216, the RAT survivor of that name.'),
    (30916411, 'PHNT9958NGDZ', 30915216, 'The mention names Hull University Teaching Hospitals NHS Trust; the PPON is carried by 30915216, the RWA survivor of that name.'),
    (31567498, 'PNWR4212QWHQ', 30913999, 'The mention names Somerset NHS FT; the PPON is carried by the same-name org 30913999.'),
    (31597028, 'PRVB1661VNRY', 31535615, 'The mention names Calderdale and Huddersfield NHS FT; the PPON is carried by 31535615, the RWY survivor of that name.'),
    (31544155, 'PTPB4248VJQT', 30917291, 'The mention names Mid Yorkshire Teaching NHS Trust; the PPON is carried by 30917291, the RXF survivor, under the trust\'s former name Mid Yorkshire Hospitals NHS Trust.'),
    (31554288, 'PXDB2967MWLG', 31545296, 'Both mentions name Alder Hey Children\'s NHS FT; the PPON is carried by 31545296, the RBS survivor of that name.'),
    (31574468, 'PZCJ1964YNTQ', 31555080, 'The mention names Avon and Wiltshire Mental Health Partnership NHS Trust; the PPON is carried by the same-name org 31555080.'),
]:
    d(org, 'GBPPON' + serial, 'merge', tgt, 'high', 'ppon-value',
      'The GB-NHS value is a PPON serial, not an ODS code. ' + why, f'r8 owner {tgt}; packet mention names')
d(31596144, 'GBPPONPBND1678TCRM', 'hold', 30915589, 'medium', 'ppon-value',
  'The value is PPON PBND-1678-TCRM; its owner 30915589 carries 96 Royal Surrey County Hospital NHS FT mentions of 100 (r6), so it is this trust, but its head reads NHS Buckinghamshire Oxfordshire & Berkshire West ICB. '
  'Held at medium: the target heads as another body (524 head refresh), so the merge is not plain; the prior expected no owner.',
  'R4 owner 30915589; r6 mention names (96/100 Royal Surrey)')
d(31550549, 'GBPPONPHPY3425XCGD', 'hold', 30928165, 'medium', 'ppon-value',
  'The value is PPON PHPY-3425-XCGD; its owner 30928165 carries 36 North Bristol NHS Trust mentions of 41 (r6), but heads as "Bristol NHS Foundation Trust.". '
  'Held at medium: the target head names a different-sounding body, so the merge is not plain; the prior expected no owner.',
  'R4 owner 30928165; r6 mention names (36/41 North Bristol)')
d(31544177, 'GBPPONPHQR1485BDJQ', 'hold', 31557115, 'medium', 'ppon-value',
  'The PPON is carried by 31557115 Cambridgeshire Community Services NHS Trust (RYV survivor), but 1 of 3 mentions names Queen Elizabeth Hospital King\'s Lynn NHS Trust. '
  'Held at medium: mixed mention names are never high for a merge.', 'r8 owner 31557115; packet mention names')
d(31911529, 'GBPPONPNNX5248NRVD', 'hold', None, 'medium', 'ppon-value',
  'The value is a PPON serial that no standing org carries (R4: neither GBPPON nor PPON spelling); the mention names West Midlands Ambulance Service, whose RYA survivor carries a different PPON (30913621). '
  'D3: a PPON correction with no owner would move to the bare serial, so it is posted at medium (held).', 'R4 (0 rows for PNNX5248NRVD)')
d(30953773, 'GBPPONPWNJ1991NGDW', 'hold', 30914040, 'medium', 'ppon-value',
  'The PPON is carried by 30914040 "Business Services Organisation, Procurement and Logistics Service"; the 44 mentions name BSO ITS (23), BSO PaLS (20) and BSO (1), two directorates of one NI body. '
  'Held at medium: the owner heads as one directorate and half the mentions name another, so the merge is not plain; the prior expected no owner.',
  'R4 owner 30914040; packet mention names')
d(31536836, None, 'none', None, 'high', 'ppon-value',
  'The value is the PPON of Sheffield Teaching Hospitals NHS FT (owner 30916153), the host trust, while the mention names South Yorkshire Bassetlaw Pathology, a pathology network. '
  'Rubric: a PPON of a different entity (a host trust) gets no correction, and a network never takes a PPON (D2).', 'r8 owner 30916153')

# --- body-no-ppon
body_ppon(31633969, 'PWCM9262VJDY', 31542357,
          '78H NHS spells sub-ICB code 78H, whose parent QPM has the PPON survivor 31542357; the mention names NHS Northamptonshire ICB. Every mention name is the body\'s (spec §3.2 example).',
          'code_survivors QPM; r8 owner 31542357', route='body-no-ppon')
body_ppon(31616732, 'PBQZ2117NPPD', 31552077,
          'D9YOV is sub-ICB code D9Y0V with O for 0, under QRL whose PPON survivor is 31552077; the mention names Hampshire & Isle of Wight ICB. Every mention name is the body\'s (spec §3.2 example).',
          'code_survivors QRL; r8 owner 31552077', route='body-no-ppon')
body_ppon(31549058, 'PBWM8336DJRG', 31569664,
          'QE1000 is the ICB code QE1 with 000 glued, ODS 404; all 4 mentions name NHS Lancashire & South Cumbria ICB, whose PPON org is 31569664 (M5; code_survivors QE1). '
          'Every mention name is the body\'s (Check item 14: the route stands).', 'code_survivors QE1; r8 owner 31569664', route='body-no-ppon')
body_ppon(31547404, 'PVQC9261WGRM', 30913873,
          'PVQC-921-WGRM is PPON PVQC-9261-WGRM with one digit dropped, carried by 30913873 Nottingham University Hospital NHS Trust (97 mentions); both mentions name Nottingham University Hospitals NHS Trust. '
          'Every mention name is the body\'s.', 'R4 range read; r8 owner 30913873; API name search', route='body-no-ppon')
body_ppon(31608238, 'PCLY6219YPTV', 31542352,
          'QN1 is no ODS code (404); the mention names NHS Leicester, Leicestershire and Rutland ICB (with a stray bullet), whose same-name PPON row is 31542352. Every mention name is the body\'s.',
          'r8 owner 31542352', route='body-no-ppon')
body_ppon(31592494, 'PRZJ5573LWLP', 31535854,
          '110044 is a local number, ODS 404; all 5 mentions name Great Western Hospitals NHS FT (2 with the location Swindon), whose same-name PPON row is 31535854 (83 mentions). '
          'Every mention name is the body\'s; the 10-12 twin read missed the row because the head carried Swindon.',
          API + '?name_prefix=great%20western%20hospitals', route='body-no-ppon')
body_ppon(31601787, 'PNXM2771CPCW', 30914076,
          '222656143 is a 9-digit number, no ODS code; the mention names Birmingham Womens and Childrens NHS FT, whose PPON row 30914076 carries the same name with apostrophes (281 mentions). '
          'Every mention name is the body\'s; the twin read missed it on the apostrophes.',
          API + '?name_prefix=birmingham%20women', route='body-no-ppon')
body_ppon(31592975, 'PCXW3895DYVX', 30917610,
          '1059467 is the charity number GB-CHC-1059467 that 31601629 carries for The Royal Wolverhampton NHS Trust (its charity); the mention names Royal Wolverhampton Trust, whose PPON row is 30917610. '
          'Held at medium: the value names the trust\'s charity, a separate body.', API + '?name_prefix=the%20royal%20wolverhampton',
          conf='medium', route='body-no-ppon')
body_ppon(31592497, 'PHPY3425XCGD', 30928165,
          '400038 is a local number, ODS 404; all 5 mentions name North Bristol NHS Trust, and PPON row 30928165 is North Bristol by 36 of 41 mention names (r6), but it heads as "Bristol NHS Foundation Trust.". '
          'Held at medium: the target head names a different-sounding body.', 'r6 mention names; ' + API + '?name_prefix=north%20bristol',
          conf='medium', route='body-no-ppon')
body_ppon(31624249, 'PMYZ6944BYLH', 31556600,
          'A83057005 is GP-practice code A83057 with 005 glued; the mention names NHS North East North Cumbria ICB, whose PPON row is the QHM survivor 31556600. '
          'Held at medium: the value names another body (a practice under the ICB).', 'code_survivors QHM; r8 owner 31556600',
          conf='medium', route='body-no-ppon')
body_ppon(31544639, 'PDQL2869XVHY', 31554399,
          'QM8 is the ICB code QM7 mistyped (ODS 404); both mentions name "NHS Herts Valley and West Essex ICB", and a PPON row 31554399 "NHS Hertfordshire and West Essex ICB" stands with 1 mention. '
          'Held at medium: the names differ and the PPON row is thin, so the merge is not plain.',
          API + '?name_prefix=nhs%20hertfordshire%20and%20west%20essex', conf='medium', route='body-no-ppon')
body_ppon(30974733, 'PWXN8614MPHH', 30930117,
          'UKF2 is a NUTS code; the mention names "University Hospitals of Leicester NHS Trust  Leicester Royal Infirmary", the trust plus a site, and the RWE survivor 30930117 is its PPON row. '
          'Held at medium: a site-qualified name (D2 exclusion) is not plain enough for a merge.', 'code_survivors RWE; r8 owner 30930117',
          conf='medium', route='body-no-ppon')
for org, code, why in [
    (31534478, 'RY3', 'GB-NHS-GB-NHS-RY3 is code RY3 with the scheme glued twice; the mention names East of England Community Health and Care NHS Trust, the name the standing GBNHSRY3 row 31572008 carries, and no PPON row stands for it (API search).'),
    (31593802, 'RY2', 'The value is the typed label of RY2; the mention names Bridgewater Community Healthcare NHS FT, which stands as GBNHSRY2 31543388 and has no PPON row (API search).'),
    (31606744, 'RY2', 'R2Y is RY2 transposed (ODS 404); the mention names Bridgewater Community NHS FT, which stands as GBNHSRY2 31543388 and has no PPON row (API search).'),
    (31593803, 'RW4', 'The value is the typed label of RW4; the mention names Mersey Care NHS FT, which stands as GBNHSRW4 30915592 and has no PPON row (API search).'),
    (31593804, 'RY7', 'The value is the typed label of RY7; the mention names Wirral Community Health and Care NHS FT, which stands as GBNHSRY7 31539451 and has no PPON row (API search).'),
    (31616537, '92G', '92G - ODS is sub-ICB code 92G glued to ODS; the mention names BSW ICB, whose QOX survivor is provisional, so no PPON org stands for the body.'),
]:
    d(org, code, 'unkeyed', None, 'high', 'body-no-ppon',
      why + ' Ruling 8: a mistyped or glued code whose body has no standing PPON org gets the right ODS code, which keys nothing (withheld and flagged only).',
      'R1 own-code row; API name search')
for org, why in [
    (31562109, '133078 is a local number, ODS 404; the mention names Northumbria Healthcare NHS FT, whose RTF survivor is provisional and which has no PPON row of its own (only its subsidiary NHFM, API search).'),
    (31567523, '174343 is a local number, ODS 404; the mention names Bridgewater Community Healthcare NHS FT, which has no PPON row (API search) and the value spells no code.'),
    (31547563, '8KE0 is no ODS code; the mention names Access Community Trust, a charity and not an NHS body (the prior\'s body test matched only the word Trust).'),
    (30914099, 'Y62 NORTH WEST COMMISSIONING REGION is NHS England North West\'s typed label on a row whose 17 mentions name L&SC ICB, GM ICB, C&M ICB and NHS England North West, several bodies.'),
]:
    d(org, None, 'none', None, 'high', 'junk' if org == 31547563 else 'body-no-ppon', why + ' Wrong with no correction: withheld and flagged; the org stays.', 'packet; API name search')

# --- coh-value
for org, num, shape, tgt in [
    (31627902, '01323762', 'move', None), (31560007, '01721624', 'merge', 16087363), (31647470, '02647131', 'merge', 31647712),
    (31627903, '03777037', 'merge', 30972137), (31627904, '04081379', 'merge', 31556251), (31627905, '04251815', 'merge', 30974841),
    (31627906, '04620338', 'merge', 31553530), (31627907, '05145427', 'move', None), (31627908, '05329299', 'move', None),
    (31627909, '05779309', 'merge', 31553538), (31588045, '06927650', 'merge', 31538765), (31627910, '07737949', 'merge', 30974799),
    (31627911, '08073027', 'move', None), (31627912, '08809516', 'move', None), (31627913, '09353634', 'move', None),
    (31617326, '09425102', 'move', None), (31627914, '09449312', 'merge', 31571718), (31627915, '09463267', 'move', None),
    (31627916, '10213678', 'move', None), (31598262, '10334174', 'merge', 31565478), (31627917, '10833910', 'move', None),
    (31596487, '10991987', 'merge', 31572323), (31605552, '11417169', 'merge', 31570190), (31627918, '11696513', 'merge', 14647720),
    (31627919, '11822551', 'move', None), (31627920, '14198384', 'merge', 30974964), (31627921, '14368239', 'merge', 31571727),
    (31552295, '15229502', 'move', None), (31627922, 'CE003108', 'move', None),
]:
    reg = REG[num]
    name = CASES[org]['head']
    own = OWN.get(('coh', num), [])
    where = (f'a standing org carries it ({tgt} {own[0][1]} "{own[0][2]}"), so the arm merges into it' if shape == 'merge'
             else 'no standing org carries it (r7), so the arm moves the row onto the bare number')
    prev = f' (previous name {reg["previous_names"][0].rsplit(" ", 1)[0]})' if org == 31627918 else ''
    d(org, num, shape, tgt, 'high', 'coh-value',
      f'The value is company number {num}; the register names it {reg["name"]}{prev}, which is this org ("{name}"). '
      f'{where[0].upper() + where[1:]}.', CH + num)
d(31579940, '11963500', 'merge', 31552066, 'high', 'coh-value',
  'EE133835 answers 404 on the register; the same-name standing org 31552066 carries GBCOH11963500, which the register names LINCOLN BISHOP UNIVERSITY (formerly Bishop Grosseteste University). '
  'Both mentions name Lincoln Bishop University, so its own number is the correction and the arm merges into 31552066.', CH + 'EE133835 (404); ' + CH + '11963500')
d(31603370, '13685497', 'hold', None, 'medium', 'coh-value',
  'The register names 13685497 WHITE CLIFFS MEDICAL SERVICES LTD; the mention names "White Cliff Medical", a near but not exact name (token score 0.4). '
  'Held at medium: the register match is not plain; with no owner it would move.', CH + '13685497')
d(31570685, '06927650', 'hold', 31538765, 'medium', 'coh-value',
  'OC313437 is DORKING HEALTHCARE LLP, dissolved, a different legal entity; the mention names "Dorking Healthcare Ltd (NNE)", whose own number 06927650 (DORKING HEALTHCARE LIMITED) 31538765 carries. '
  'Held at medium: the value names the dissolved LLP and the (NNE) suffix is unexplained.', CH + 'OC313437; ' + CH + '06927650')
d(31569876, 'GBPPONPYRL5371BNMH', 'hold', 30916969, 'medium', 'coh-value',
  'OC389166 is HEALTH SERVICES LABORATORIES LLP, a pathology partnership UCLH belongs to; the mention names University College London Hospitals NHS FT (UCLH), whose PPON row is the RRV survivor 30916969. '
  'Held at medium: the value names a related body that may itself have been the party.', CH + 'OC389166; code_survivors RRV')

# --- company-coh (D5 exception: a same-name standing org carries the register-confirmed number)
for org, num, tgt in [
    (31547562, '02611510', 31570814), (31594381, '05597217', 30916290), (31542548, '13119584', 31535346),
    (31572061, '01768840', 12335686), (31572062, '09225457', 14842867), (31599269, '09225457', 14842867),
    (31571838, '01202866', 30916889), (31571862, '07974901', 31596839), (31571926, '08714511', 31597495),
    (31571914, '08714511', 31597495), (31599276, '08714511', 31597495), (31599273, '08714511', 31597495),
    (31571876, '00928555', 14842878), (31608372, '08904332', 30926962), (31608376, '09116741', 30926964),
    (31593954, '01786914', 31537833), (31565687, '00507597', 30917690), (31616687, '00928555', 14842878),
    (31566382, '07314767', 31545671), (31623642, '03682679', 30968917),
]:
    reg = REG[num]
    own = OWN[('coh', num)][0]
    d(org, num, 'merge', tgt, 'high', 'company-coh',
      f'The value {CASES[org]["identifier"][5:]} is no ODS code or register number of this org; the same-name standing org {tgt} carries {own[1]}, which the register names {reg["name"]}. '
      f'Every mention names "{CASES[org]["head"]}", so that number is the correction and the arm merges into {tgt}.', CH + num)
for org in (31598399, 31572050):
    d(org, None, 'none', None, 'high', 'company-coh',
      'The same-name row 14842856 "Asda Stores Ltd" carries 03918551, but the register names 03918551 ASDA HOLDINGS UK LIMITED, not Asda Stores. '
      'The glued pharmacy codes are no identifier of this org, so wrong with no correction (withheld and flagged).', CH + '03918551')
# junk rows reclassified by the D5 exception (the review found the company's register-confirmed number on a standing row)
d(31571908, '00928555', 'merge', 14842878, 'high', 'company-coh',
  'The value is glued Boots pharmacy codes (FK182/FNL49/FF360); both mentions name Boots UK Limited, and 14842878 "Boots UK Ltd" carries 00928555, which the register names BOOTS UK LIMITED. '
  'D5 exception: reclassified from junk; the arm merges into 14842878.', CH + '00928555')
d(31599268, '00928555', 'merge', 14842878, 'high', 'company-coh',
  'The value is glued Boots pharmacy codes (FF360/FNL49/FK182); the mention names Boots The Chemist Ltd, the register\'s previous name of 00928555 (BOOTS THE CHEMISTS LIMITED until 2007), which 14842878 carries. '
  'D5 exception: reclassified from junk; the arm merges into 14842878 (its name gate may deny it, a residue).', CH + '00928555')
d(31571907, '09225457', 'merge', 14842867, 'high', 'company-coh',
  'The value is glued pharmacy codes; both mentions name Bestway National Chemists Ltd T/A Well Pharmacy, and 14842867 "Bestway National Chemists Ltd" carries 09225457, which the register names BESTWAY NATIONAL CHEMISTS LIMITED. '
  'D5 exception: reclassified from junk; the arm merges into 14842867.', CH + '09225457')
d(31572070, '00928555', 'hold', 14842878, 'medium', 'company-coh',
  'The value is glued pharmacy codes; the mention names "Boot UK Ltd", a likely misspelling of Boots UK Ltd (14842878, 00928555 BOOTS UK LIMITED). '
  'Held at medium: a misspelt name is not plain.', CH + '00928555')

# --- nhse-523
for org, code, conf, why in [
    (31613184, '14J', 'high', 'The value is the typed label GREATER MANCHESTER COMMISSIONING HUB (14J); both mentions name NHS England Greater Manchester.'),
    (31619925, 'Q76', 'high', 'The value is the typed label NHS ENGLAND MIDLANDS (NORTH MIDLANDS) (Q76); both mentions name NHS England Midlands.'),
    (31623644, 'Q74', 'high', 'The value is the typed label NHS ENGLAND NORTH EAST AND YORKSHIRE (CUMBRIA AND NORTH EAST) (Q74); both mentions name that NHS England office.'),
    (31620223, 'Q74', 'high', 'The value is Q74 followed by the typed label of the same NHS England office; all 3 mentions name NHS England North East and Yorkshire Health and Justice.'),
    (31598682, 'Q72', 'high', 'The value is the typed label NHS ENGLAND NORTH EAST AND YORKSHIRE (YORKSHIRE AND HUMBER) (Q72); all 3 mentions name NHS England North East and Yorkshire.'),
    (31623761, 'Q720A', 'high', 'The value is the typed label of NHS England local office site Q720A (South Yorkshire & Bassetlaw); the mention names NHS England North East and Yorkshire (South Yorkshire).'),
    (31619926, 'Y54', 'high', 'The value is the typed label NORTH OF ENGLAND COMMISSIONING REGION (Y54); both mentions name NHS England North East and Yorkshire offices.'),
    (31613185, 'Y62', 'high', 'The value is the typed label NORTH WEST COMMISSIONING REGION (Y62); all 3 mentions name NHS England North West.'),
    (31603424, 'Y63', 'medium', 'The value is the typed label Y63 NORTH EAST AND YORKSHIRE COMMISSIONING REGION; 4 of 9 mentions name NHS England NEY and the rest name ICBs and trusts (mixed).'),
]:
    d(org, code, 'unkeyed', None, conf, 'nhse-523',
      why + ' NHS England (issue 523): wrong with the NHS England code the value spells, which keys nothing; never a PPON.', 'packet raw identifiers')
for org, why in [
    (30913729, 'A3HOT is no NHS England code (ODS 404); all 3 mentions name NHS England North East and Yorkshire.'),
    (31538079, 'GB-NHS-GB-NHS-32T spells 32T, no NHS England code; the mention names NHS England North West - Health and Justice.'),
    (31537283, 'The value lists three codes (Q75, Q83, Q84); the 5 mentions name NHS England - North West or NHS England, so no single code is the right one.'),
    (31596501, 'The value lists two codes (Q75 & Q84); the mention names NHS England - North West, so no single code is the right one.'),
]:
    d(org, None, 'none', None, 'high', 'nhse-523', why + ' NHS England (issue 523): wrong, no correction, never a PPON.', 'packet raw identifiers')

# --- site-or-hosted
d(31536288, None, 'none', None, 'high', 'site-or-hosted',
  '86101 is no ODS code (404); the mention names Newcastle Hospitals NHS FT (Newcastle Advanced Therapies), a hosted unit. 513 HOSTED_OR_SITE precedent (D2): wrong, no correction.', 'packet')
d(31615538, None, 'none', None, 'high', 'site-or-hosted',
  'UKC14 is a NUTS code (404); 3 of 4 mentions name NENC ICB (County Durham Place), a place. 513 HOSTED_OR_SITE precedent (D2): wrong, no correction.', 'packet')

# --- new since 10-12
d(31967674, None, 'none', None, 'high', 'new-body-no-ppon',
  'SLA20 answers ODS 404 (new class E since 10-12); both mentions name NHS Lanarkshire (Scotland), a Scottish health board with no standing PPON org (API search shows only provisional rows). '
  'Wrong with no correction: withheld and flagged.', ODS + 'SLA20 (404); ' + API + '?name_prefix=nhs%20lanarkshire')

# --- D8: over the 64-char POST cap (recorded, never posted)
d(31623641, None, 'none', None, 'high', 'body-no-ppon',
  'The typed label names Calderdale and Huddersfield NHS FT as application provider YGM58 on a West Yorkshire Vaccination & Immunisation Collaborative row, a collaborative (D2 exclusion). '
  'Wrong with no correction; D8: the 72-character literal exceeds the POST cap, so it is not posted.', 'packet')
d(31610156, 'GBPPONPMTJ1118QVJH', 'hold', None, 'medium', 'body-no-ppon',
  'The typed label is NHS England region Y63 with "(NHS South Yorkshire ICB)"; the mention names NHS South Yorkshire ICB (QF7), whose PPON row was not read (D8 row). '
  'D8: the 68-character literal exceeds the POST cap, so it is not posted.', 'packet')
R[31610156]['correct_identifier'] = None
d(31610157, 'GBPPONPMTJ1118QVJH', 'hold', 30917022, 'medium', 'body-no-ppon',
  'The typed label is NHS England region Y63 with "(NHS West Yorkshire ICB)"; the mention names NHS West Yorkshire ICB, whose PPON row is the QWO survivor 30917022. '
  'D8: the 67-character literal exceeds the POST cap, so it is not posted.', 'packet; r8 owner 30917022')

# --- every remaining case is junk (D5): wrong, no correction
for org, c in CASES.items():
    if org not in R:
        assert c['route'] == 'junk', (org, c['route'])
        d(org, None, 'none', None, 'high', 'junk',
          f'The value {c["identifier"][5:]} is no ODS code ({c["shape"]}, ODS {c["ods"]}); the mention names "{c["head"]}", which carries no NHS code and no register-confirmed number on a same-name standing row. '
          'D5: wrong with no correction, withheld and flagged; the org stays.', 'packet')

assert set(R) == set(CASES), set(CASES) ^ set(R)

# ----------------------------------------------------------------------------------------------
# Challenger (rule-based re-derivation)
# ----------------------------------------------------------------------------------------------
FORMS = {'ltd', 'limited', 'plc', 'llp', 'the', 'and', 'co', 'company', 'uk', 'cic', 'group', 'nhs', 'trust',
         'foundation', 'ft', 'icb', 'integrated', 'care', 'board', 'of'}
SCREEN = re.compile(r'\bplace\b|pathology|collaborative|advanced therapies|on behalf|royal infirmary|lead icb', re.I)


def words(s):
    s = html.unescape(s or '').lower().replace('&', ' and ')
    return {w for w in re.split(r'[^a-z0-9]+', s) if w and w not in FORMS}


def score(a, b):
    wa, wb = words(a), words(b)
    return len(wa & wb) / len(wa | wb) if wa and wb else 0.0


def challenge(org, r):
    c = CASES[org]
    names = list(c['mention_names'])
    corr = r['correct_identifier']
    notes = []
    if corr is None:
        return ('none', None, None), 'no correction claimed; nothing to re-key'
    if corr.startswith('GBPPON'):
        own = OWN.get(('ppon', corr[6:]), [])
        if len(own) != 1:
            return ('hold', corr, None), f'{len(own)} live owners of {corr}: no merge'
        tgt, lit, tname = own[0]
        worst = min(score(n, tname) for n in names) if names else 0
        screened = [n for n in names if SCREEN.search(n)]
        if screened:
            return ('hold', corr, tgt), f'a mention names a site/place/network ({screened[0]!r}): never high'
        if worst < 0.5:
            return ('hold', corr, tgt), f'a mention name scores {worst:.2f} against the owner head {tname!r}'
        return ('merge', corr, tgt), f'unique live owner {tgt} {tname!r}; every mention name scores >= {worst:.2f}'
    if re.fullmatch(r'\d{8}|[A-Z]{2}\d{6}', corr):
        reg = REG.get(corr)
        if not reg or not reg.get('name'):
            return ('none', None, None), f'register gives no name for {corr}'
        regnames = [reg['name']] + [p.rsplit(' ', 1)[0] for p in reg.get('previous_names', [])]
        best = min(max(score(n, rn) for rn in regnames) for n in names)
        if best < 0.5:
            return ('hold', corr, None), f'a mention name scores {best:.2f} against the register names'
        own = OWN.get(('coh', corr), [])
        if len(own) > 1:
            return ('hold', corr, None), f'{len(own)} owners: multi-target'
        if own:
            return ('merge', corr, own[0][0]), f'register {reg["name"]!r} matches (>= {best:.2f}); unique owner {own[0][0]}'
        return ('move', corr, None), f'register {reg["name"]!r} matches (>= {best:.2f}); no owner, so a move'
    return ('unkeyed', corr, None), f'{corr} is an ODS/NHS England code: keys nothing'


out = []
for org in sorted(R):
    r, c = R[org], CASES[org]
    (cs, cc, ct), why = challenge(org, r)
    rs = r['shape']
    # Agreement = verdict, correction, shape and target all match (spec §4.3). A held row agrees
    # when the challenger also holds the same correction (its target, if any, is informational).
    if rs == 'hold':
        agrees = cs == 'hold' and cc == r['correct_identifier']
    else:
        agrees = (cs, cc, ct) == (rs, r['correct_identifier'], r['target'])
    if not agrees and r['confidence'] != 'high':
        why += ' | the reader held it at medium; on disagreement the row posts at medium anyway'
    out.append({
        'org': org, 'identifier': c['identifier'], 'route': r['route'], 'prior_route': c['route'],
        'verdict': r['verdict'], 'correct_identifier': r['correct_identifier'],
        'expected': {'shape': rs, 'target': r['target']}, 'confidence': r['confidence'],
        'rationale': r['rationale'], 'evidence': r['evidence'],
        'challenge': f'rule-based challenger: {cs} {cc or ""} {ct or ""} - {why}'.strip(),
        'challenger_agrees': bool(agrees),
        'head': c['head'], 'mentions': c['mentions'], 'mention_names': c['mention_names'],
    })
json.dump(out, open(os.path.join(D, 'review.json'), 'w'), indent=1, ensure_ascii=False)
cnt = collections.Counter((o['expected']['shape'], o['confidence'], o['challenger_agrees']) for o in out)
for k, v in sorted(cnt.items(), key=str):
    print(k, v)
print('rows', len(out), 'disagree on high:', [o['org'] for o in out if o['confidence'] == 'high' and not o['challenger_agrees']])
