# Hand classification of the 72 GB-PPON census-rule strays in slice GBPPONPC..GBPPONPD (q3).
# V = the holder itself (spelling, typo, case, abbreviation, legal form, longer/shorter form,
#     trading name, rename of the same body; includes "the head IS the stray")
# U = a sub-unit, parent, subsidiary, hosted service or predecessor of the holder (not a separate
#     wrong attribution worth moving)
# T = not an organization name (lot/project title, product text, "DO NOT USE")
# C = a consortium naming the holder and a partner
# F = a different organization (the 545 class); role and destination from q4/q5/q6
import json
J = {
 'Bordesley Green Girls School':'V','Change Grow Live (CGL)':'V','University of Northumbria':'V',
 'northumbria university newcastle':'V','G. H. JAMES CYFYNGEDIG':'V','Plant Equipment Maintenance & Service':'T',
 'Home-start Cymru for Families':'V','Sheffield Health and Social Care NHS Foundation Trust':'U',
 'Derek Lewis Ltd':'F','Pensions Policy Institute':'F','Newcastle-under-Lyme Borough Council':'V',
 'South Yorkshire Fire and Rescue Service':'V','Parego Wales Ltd':'V','Camddwr Contractors':'V','Clear Links Ltd':'V',
 'Business Services Organisation Procurement and Logistics Service':'U','Co-op Academies Trust':'U',
 'Equality Commission for Northern Ireland (ECNI)':'V','Newport City Homes':'V','Small Airbags - Market Engagement':'T',
 'Uniphar Medical T/A Tekno':'V','Uniphar Medtech UK Ltd t/a M.E.D Surgical':'V',
 'Department of Education - Council for the Curriculum, Examinations and Assessment (CCEA)':'V',
 'Working Word Public Relations Limited T/A Working Word Limited':'V','Healthcare 21 NI':'F',
 'Tees Valley Combined Authority':'F','Awair GB Lt':'V','Aurobindo (Milpharm Ltd)':'V','PROFILE Systems & Sofware':'V',
 'Northumbria Police':'U','South Yorkshire Mayoral Combined Authority Executive':'U','NHS BLOOD AND TRANSPLANT (NHSBT)':'V',
 'Orbit Homes':'U','Cambrian Construction Equipment Ltd':'U','Soho Housing':'V','SAV Systems UK Limited':'V',
 'Lot B - Southern Sites Fish Passage: Detailed Design and Permitting':'T',
 'Lot C: River Colne Constructed Wetland Outline Design and Feasibility':'T','GLL':'V','O.C. Consulting Limited,':'V',
 'Welsh Local Government Association':'V','Cynnal Cymru Sustain Wales and Manchester  Metropolitan University':'C',
 'ALCHERA DATA TECHNOLOGIES LTD':'V','DIGITAL ASSET MANAGEMENT':'T','Equal Education Partners ':'F',
 'Shared Resource Service (SRS)':'U','Shared Resource Services (SRS)':'U','Hartlepool Development Corporation':'F',
 'South Tees Development Corporation':'F','NI Ambulance Service Trust':'V','NIAS':'V','The Valuation Office Agency':'V',
 'Valuation Office Agency (DVS)':'V','Woodknowledge Wales':'V','NHS England DO NOT USE':'T',
 'Portsmouth Hospitals University NHS Trust':'F','Jeol (U.K.) Limited':'F','SCIENCE & TECHNOLOGY FACILITIES COUNCIL':'U',
 'UK Reseach and Innovation':'V','UK Research and Innovation - Cabinet Office - Public Procurement':'V',
 'United Kingdom Research and Innovation':'V','City of Stoke-on-Trent Council':'V','Grŵp Cynefin':'V',
 'QCI (UK) LIMITED':'F','Equinix Inc':'V','WATKINS COACHES LTD':'F','Pobl Group':'U','ARCADIS TETRA TECH 2 LIMITED':'V',
 'Office of the Police Ombudsman Northern Ireland':'V','Health Innovation Network':'U',
 'AF145 Service Contract for Biosafety Cabinets and Fume Hoods':'T','AF174 Plate Imaging & Analysis System':'T',
}
# F detail: (role from q6, destination from q4)
FD = {
 'Jeol (U.K.) Limited':('Tenderer+winner','identified: 31620129 GBCOH939456 JEOL (UK) LTD (also 12991212 "00939456", 9534435 "939456")'),
 'Derek Lewis Ltd':('Tenderer','identified: 30928138 GBCOH01102211 DEREK LEWIS LIMITED (probable; holder "Dewi\'s Ltd")'),
 'Pensions Policy Institute':('Tenderer','provisional only: 31587812'),
 'Healthcare 21 NI':('Tenderer','identified: 15757939 VAT GB574532920 "Healthcare 21 Group N.I." / 30915822 GBCOH05020682 (relation to holder "Primary Healthcare 21" unproven)'),
 'Equal Education Partners ':('Tenderer','identified: 31582974 GBCOH07611972 and 30928130 GBPPONPVWC8236ZZJZ (two rows)'),
 'Portsmouth Hospitals University NHS Trust':('Tenderer','identified: 30971975 GBPPONPBDT9319TTHZ'),
 'QCI (UK) LIMITED':('Tenderer','absent (no name_norm prefix "qci (uk)")'),
 'WATKINS COACHES LTD':('Tenderer','provisional only: 2479435 "Watkins Coaches Ltd"'),
 'Tees Valley Combined Authority':('Procedure-Buyer (on STDC\'s PPON)','identified: 30913801 GBPPONPDNG8971ZVCR'),
 'South Tees Development Corporation':('Procedure-Buyer (on TVCA\'s PPON)','identified: 31555134 GBPPONPCYV2926WZMP'),
 'Hartlepool Development Corporation':('Procedure-Buyer (on TVCA\'s PPON)','provisional only: 31610040'),
}
rows = json.load(open('q3_strays.json'))
from collections import Counter
c = Counter(); out = []
for r in rows:
    if r['scheme'] != 'GB-PPON': continue
    k = J[r['stray']]; c[k] += 1
    out.append({**r, 'class': k, **({'role': FD[r['stray']][0], 'destination': FD[r['stray']][1]} if k == 'F' else {})})
assert sum(c.values()) == 72, sum(c.values())
json.dump(out, open('q3_strays_judged.json', 'w'), indent=1, ensure_ascii=False)
print(dict(c))
f = [o for o in out if o['class'] == 'F']
sup = [o for o in f if o['role'].startswith('Tenderer')]
print('F supplier-side', len(sup), 'identified dest', sum(1 for o in sup if o['destination'].startswith('identified')))
print('F buyer-side', len(f) - len(sup), 'identified dest', sum(1 for o in f if not o['role'].startswith('Tenderer') and o['destination'].startswith('identified')))
