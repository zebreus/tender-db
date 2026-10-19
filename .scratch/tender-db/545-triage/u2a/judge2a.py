# Issue 545 unit 2a — hand classification of the 208 prefilter-kept candidates in cand.txt
# (GB-PPON-keyed orgs, letters GBPPON0..PB and PG..PZ/RD; PC..PD was unit 1's slice).
#
# Classes (unit 1's, plus A and F?):
#   V  the holder itself (spelling, abbreviation, legal form, trading name, Welsh name, rename)
#   U  a sub-unit, parent, subsidiary, hosted body, group/sister body or predecessor of the holder  -> PARK
#   A  buying agent / consortium / framework operator <-> member or principal (NWSSP and a health
#      board, Star and Trafford, Welland and Rutland, Northern Housing Consortium and its member
#      councils, a broker and the schools it buys for). Two different bodies, but WHICH one is the
#      party is the agent/principal question, not a copy-paste of an unrelated name               -> PARK
#   T  not an organization name (lot/project title, address, a person's first name, junk)
#   C  consortium naming the holder and a partner
#   F  a different organization with no known corporate relation to the holder (the 545 class);
#      goes on to the guards (section collision, destination) in verdicts.py
#   F? a different name whose relation to the holder is unknown (small firms: a possible trading
#      name, rename or sister company)                                                            -> PARK
import json, sys

J = {
 (30926792, 'NHS Wales Shared Services Partnership-Procurement Services'): 'A',
 (30928204, '2, 10, 18, 27, 34, 42 Queensland Road'): 'T',
 (30928204, 'HN 1 and 2 (15 & 17) Hornsey Street, HN 3 (13) Hornsey Street and  HN 4 (9) Hornsey Street'): 'T',
 (30913906, 'Abri Ltd'): 'F',
 (30914819, 'UKHSA'): 'V',
 (31548303, 'Inutec Limited'): 'U',            # Tradebe Inutec: same group
 (30917832, 'Procurement & Logistics Service'): 'A',  # BSO PaLS buys for the HSC trusts
 (30914265, 'Abbott Medical UK Limited'): 'F',
 (30916628, 'NHS NORTH CENTRAL LONDON INTEGRATED CARE BOARD'): 'A',
 (30928312, 'NHS Wales Shared Services Partnership-Procurement Services (hosted by Velindre University NHS Trust)'): 'A',
 (30930374, 'Plattform'): 'V',
 (30914406, 'Thomas Silvey'): 'F',
 (31542351, 'Support to manage transition to systems of new delegated responsibilities'): 'T',
 (30917785, 'Castle Point Borough Council'): 'F',
 (30917785, 'Castlepoint Borough Council'): 'F',
 (31577607, 'Lighting Solutions '): 'V',
 (30930582, 'the Co-Operative Development Society Limited '): 'V',
 (30930308, 'CORE Education Trust'): 'A',      # Invictus Partnership buys for schools/trusts
 (30930308, 'William Farr C of E Comprehensive School'): 'A',
 (31545519, 'Capital Works - Windows & Doors'): 'T',
 (31545519, 'Fire and Security Systems Maintenance & Repair Services'): 'T',
 (31545519, 'Provision of Waking Watch Services'): 'T',
 (31546102, 'Tom Reeves trading as Making Trax'): 'V',
 (30913854, "Prince Henry's High School"): 'U',   # another school, possibly the same trust: sister, parked
 (30969284, 'Q3 Services Group Limited'): 'V',
 (30917275, 'Star Procurement'): 'A',
 (30917275, 'Star procurement'): 'A',
 (31563568, 'ITGL Limited'): 'U',              # acquired by Conscia
 (30914149, 'London Legacy Development Corporation'): 'U',  # LS185's parent
 (30914149, 'London Stadium185'): 'V',
 (30929723, 'NHS Wales Shared Services Partnership-Procurement Services (hosted by Velindre University NHS Trust)'): 'A',
 (31548943, 'Everglow'): 'F?',
 (31560765, 'TIAA LTD'): 'F',
 (31560765, 'The Academy for Healthcare Science Limited'): 'F',
 (31535216, 'North East London Foundation Trust'): 'V',
 (30913560, 'Greater London Authority (GLA)'): 'U',
 (30913560, 'London Underground Limited'): 'U',
 (30913560, 'TFL'): 'V',
 (31574005, 'Oxygencare'): 'V',
 (30913556, 'Cognition Education UK Ltd'): 'F',
 (30917503, 'Worthing Borough Council'): 'U',  # Adur & Worthing joint councils
 (30972047, 'Cyber & Specialist Operations Command (CSOC)'): 'U',
 (30972047, 'MOD'): 'V',
 (31544143, 'University of Cambridge - UBS'): 'V',
 (31565242, 'Kajima Partnerships a 1 8 b 1 8 c 1 7 8 f 1 2 3 4 5 6 7 8 g 1 2 3 4 5 6 7 8 h 8'): 'T',
 (30913748, 'London School of Economics and Political Science'): 'F',
 (31567914, 'ND Care and Support'): 'V',
 (30915064, 'Liverpool City Region Combined Authority'): 'U',  # Merseytravel is part of LCRCA
 (30913887, 'The Royal Borough  of Kensington & Chelsea'): 'U',  # bi-borough shared services
 (30916588, 'NLA MEDIA ACCESS LIMITED'): 'F',
 (31563610, 'ADS'): 'V',
 (30913691, 'Market Engagement - Engine Oil Collection, Treatment, Recovery, Recycling or Disposal Services'): 'T',
 (30913691, 'Market Engagement - Textiles and Shoes Processing Services'): 'T',
 (30913691, 'Wood Waste Disposal Services'): 'T',
 (31558765, 'ELV Solutions'): 'F?',
 (30929662, 'PPL/PRS'): 'F',
 (30914865, 'RCD Lockindge Ltd'): 'F',
 (30914865, 'UKAEA'): 'V',
 (30968739, 'The Howard Partnership Trust'): 'A',  # Ginger Energy, a broker, buying for trusts
 (30968739, 'The Keys Academy Trust'): 'A',
 (30919333, 'INTUITIVE LIMITED'): 'F',
 (30914046, 'FIRE SERVICE COLLEGE LIMITED'): 'F',
 (30915372, 'Together for Children'): 'U',     # Sunderland's children's services company
 (31575297, 'Lifestone Home Limited'): 'F?',
 (31575297, 'Lifestone Home Limited '): 'F?',
 (31539566, 'East Herts District Council'): 'V',
 (31585035, 'Crisis 24 Limited'): 'V',
 (30919318, 'CDEL/SC/ERL 25-26-012-MM - HEALTHCARE RE-ROOF ITT'): 'T',
 (31539237, 'Sally'): 'T',
 (31556600, 'NHS England - North East & Yorkshire'): 'U',  # NHSE regional team <-> ICB commissioning
 (30913583, 'Red Kite Veterinary Consultants Ltd'): 'F',
 (31534764, 'Local Councils Road Innovation Group'): 'V',
 (31568749, 'Cymru Quarry  Group'): 'U',
 (31550973, 'M&D Care'): 'V',
 (30969130, 'GEMINI SURGICAL UK LTD'): 'F?',
 (31536232, 'IDT'): 'V',
 (31559355, 'Oil 4 Wales Ltd'): 'V',
 (31549426, 'CAMBRIDGE UNIVERSITY HOSPITALS NHS FOUNDATION TRUST'): 'F',
 (31549426, 'Cambridge University Hospitals NHS Foundation Trust'): 'F',
 (31549426, 'Hologic Ltd'): 'F',
 (30969319, 'SECURI-PLEX LIMITED'): 'V',
 (30969320, 'Diagnose UK Limited'): 'F?',
 (30915586, 'Liverpool & Sefton - Targeted Support'): 'T',
 (30976598, 'Sten10 Ltd'): 'V',
 (30918257, 'Mechplant NW Limited'): 'F?',
 (31585594, 'XenZone'): 'V',                   # Kooth's former name
 (30915197, 'Direct Award of Contract for Employee Engagement Surveys (Our Voice)'): 'T',
 (30915197, 'The Centre for Systemic Social Work'): 'F',
 (30929605, 'STAL UK Limited'): 'U',
 (31538012, 'Renishaw Neuro Solutions Limited'): 'F',
 (30929925, 'NHS Resolution'): 'V',            # NHS Litigation Authority's operating name
 (31554622, 'AtkinsRéalis'): 'V',
 (30913740, 'Xiel'): 'F',
 (30915637, 'Landscape and Environmental Design'): 'T',
 (30915637, 'Replacement of Glade Bridge, Riverside Drive, Rickmansworth'): 'T',
 (30915637, 'Watersmeet Electrical Intake Room'): 'T',
 (31547103, 'Nortemecánica'): 'V',
 (31541859, 'SportNI'): 'V',
 (30914497, 'Provision of MEES Exemptions and/or Design Works'): 'T',
 (30916247, 'Tech Against Terrorism'): 'F',
 (30967462, 'Owen Environmental'): 'F?',
 (31571667, 'ERMC Ltd'): 'F',
 (30916394, 'Stratford Waterfront Retail Company Ltd'): 'U',  # LLDC subsidiary
 (30913935, 'National Radiotherapy trials (RTTQA) Group'): 'U',  # hosted by Mount Vernon (ENHT)
 (30914652, 'Conservation Architect'): 'T',
 (30919606, 'THE AD CONSTRUCTION GROUP LIMITED'): 'U',  # parent of Architectural Decorators
 (31556399, 'Waterways Ireland'): 'F',
 (30926713, 'CollectionHQ'): 'F',
 (30926713, 'collectionHQ'): 'F',
 (30914820, 'Remarkable Autism'): 'A',         # RPJ3 buys on behalf of trusts/schools
 (30913750, 'NHS Wales Shared Services Partnership-Procurement Services (hosted by Velindre University NHS Trust)'): 'A',
 (30913971, 'Denbigshire County Council '): 'V',
 (30914094, 'Betsi Cadwaladr University Health Board'): 'F',
 (31541254, 'Anti-Bullying Alliance'): 'U',    # hosted by NCB
 (31541254, 'NCB'): 'V',
 (31541254, 'NCB RiP'): 'V',
 (30915408, 'Carmarthen County Council'): 'V',
 (30913969, 'DE'): 'V',
 (31567753, 'Martins Removal Services Ltd'): 'V',
 (31568013, 'Learning Mate Solutions Ltd'): 'V',
 (30913763, 'Ardal'): 'A',                     # Ardal: the shared procurement service
 (30919377, 'NHS Wales Shared Services Partnership-Procurement Services (hosted by Velindre University NHS Trust)'): 'A',
 (30913798, 'Rutland County Council'): 'A',    # Welland Procurement serves Rutland
 (30913704, '00419-AST Windows and Doors Replacement Programme'): 'T',
 (30913704, '00538-CS - Recruitment Tracking System'): 'T',
 (30913779, 'DE&S'): 'V',
 (31586384, 'Economic Modeling LLC t/a Lightcast'): 'V',
 (30974616, 'Scape Group Limited (trading as SCAPE)'): 'A',  # framework operator
 (30928095, 'E&O Laboraties Ltd'): 'V',
 (31548720, 'Spencers ECA'): 'U',
 (30976610, 'InfluencyX'): 'V',
 (30969160, 'MIAA (Mersey Internal Audit Agency)'): 'V',
 (31537958, 'Sure Campus Ltd'): 'V',
 (31567271, 'ivolveDC Ltd'): 'V',
 (30914077, 'University Hosptials Birmigham NHS Trust'): 'V',
 (30915344, 'UK-Chatham: Men in Sheds'): 'T',
 (31538014, 'Silver Rail'): 'V',
 (30916757, 'CABI Europe – UK'): 'V',
 (30930344, 'FM Services'): 'V',
 (30926693, 'Vohkus'): 'F',
 (31540150, 'Cyngor Sir Ynys Mon'): 'V',
 (30913561, 'Bolton NHS Foundation Trust'): 'F',
 (30913738, 'NHS '): 'T',
 (30914669, 'Hartlepool Borough Council'): 'F',
 (30973303, 'University Of The West Of England'): 'V',
 (30973303, 'University of West of England'): 'V',
 (31559744, 'Tinint'): 'V',
 (31559744, 'Tinnint'): 'V',
 (30917424, 'Edseco Limited trading as The Education People'): 'U',  # KCC-owned
 (30917424, 'Laser Energy'): 'U',              # LASER, KCC's energy buying service
 (31555564, 'Johnston Gilpin & Co'): 'F?',
 (30915378, 'Ramsgate Royal Harbour, Kent Mechanical Dredging'): 'T',
 (30915378, 'Replacement Backup Solution Platform'): 'T',
 (30969195, 'NCC Commercial Services t/a Cyson Cymru'): 'V',
 (31549806, 'Alder Hey'): 'F',
 (31551518, 'Gama Healthcare Ltd'): 'F',
 (30913608, 'CORPORATION OF LONDON T/A BARBICAN CENTRE'): 'V',
 (30913608, 'City of London Police'): 'U',
 (30916395, 'Uttlesford District Council'): 'F',
 (31563071, 'Gerald Poole'): 'V',
 (30913648, 'Virgin Media Limited'): 'F',
 (30926827, 'ROG First Light Exhibitions Scenic Engineering Model Contractor'): 'T',
 (30926827, 'ROG First Light Wayfinding'): 'T',
 (30926827, 'Royal Museums Greenwich'): 'V',
 (31559992, 'ReactXR Limited'): 'V',
 (30926847, 'Compendium Regeneration Limited'): 'F',  # org 30926847: a PPON published by 7+ firms
 (30926847, 'M Group Services'): 'F',
 (30926847, 'RSS International Ltd'): 'F',
 (30926847, 'Taylor Wimpey'): 'F',
 (30926847, 'The Law Society'): 'F',
 (31547119, 'SEC PROCUREMENT LIMITED T/A SOUTH EAST CONSORTIUM'): 'V',
 (31547119, 'SEC Procurement Ltd T/A South East Consortium'): 'V',
 (30915227, 'SWANSEA BAY UNIVERSITY HEALTH BOARD'): 'F',
}
# Northern Housing Consortium (30953692): every kept candidate there is a member body named on NHC's PPON.
NHC = 30953692

def classify(r):
    if r['org'] == NHC:
        return 'A'
    return J[(r['org'], r['name'])]

if __name__ == '__main__':
    rows = [r for r in json.load(open('cand.json')) if r['keep']]
    from collections import Counter
    c = Counter(); missing = []
    for r in rows:
        try:
            r['class'] = classify(r); c[r['class']] += 1
        except KeyError:
            missing.append((r['org'], r['name']))
    assert not missing, missing
    unused = set(J) - {(r['org'], r['name']) for r in rows}
    assert not unused, unused
    json.dump(rows, open('cand_judged.json', 'w'), indent=1, ensure_ascii=False)
    print('kept', len(rows), dict(sorted(c.items())))
    print('F mentions', sum(r['n'] for r in rows if r['class'] == 'F'), 'F keys', c['F'])
