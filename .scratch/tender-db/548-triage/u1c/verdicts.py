# 548 U1c: recorded verdicts for the consortium_removed token-subset release (subset.py), plus the 3 'uncertain' ids.
# Verdict = serving new_name instead of the held head: R right, N neutral, W wrong. Evidence: the held listing's own
# spellings (mentions on this identifier) and public GETs in u1c/api/ (org_<id> = the live served name + identifier,
# np_<x> = name_prefix lookups of the new name, t_/td_ = winner lists and tender details). Output: u1c/verdicts.json.
import json, collections, sys
D = '/home/user/tender-db/.scratch/tender-db/548-triage/'
sys.argv = ['subset.py']
s = json.load(open(D + 'u1c/subset.json'))
M = {r['org_id']: r for r in s['matched_full']}
U = {r['org_id']: r for r in s['shippable_not_matched']}

ANN = 'role annotation dropped, same firm; new spelling carries the identifier on its other notices'
V = {
    # --- the salted 30 (salt 548-u1c-, sha256 asc, skip-ids excluded: none of the 79 is on skip-ids.tsv) ---
    18124034: ('R', 'ARGE of 3 -> S Hoch 2 ZT GmbH: every one of the 5 mentions on AT 9110017975749 names S Hoch 2 ZT GmbH '
                    '(3 alone, 2 in two different ARGEs), so the identifier is S Hoch 2\'s (np_shoch2: no other org serves it)'),
    9134963: ('R', 'Unisys Belgium S.A. (as consortium member) -> Unisys Belgium SA; ' + ANN),
    13580689: ('R', 'ARENA SI (mandataire) -> Arena SI; ' + ANN),
    10266767: ('R', 'buyer group "Groupement de commande VSA/SIMCRA" -> Val de Seine Amenagement; 7 of 9 mentions on SIRET '
                    '44992071900019 print VSA alone; np_valdeseine: 7478730 is served the same name (78 mentions)'),
    16034881: ('R', 'EDRA SA (mandataire) -> EDRA SA; ' + ANN),
    13556809: ('R', 'Greta de Grenoble (mandataire) -> GRETA de Grenoble; ' + ANN),
    18181616: ('R', "groupement l'association l'eclaircie (mandataire) avec ... Fil d'ariane -> association L'Eclaircie; 5 of 6 "
                    "mentions print L'Eclaircie alone on SIRET 38758257000033; np_eclaircie: 11018458 Association l'eclaircie"),
    21480661: ('R', 'societe CHUDEAU (Mandataire du groupement) -> SAS CHUDEAU (14 of 15 mentions CHUDEAU [SAS]); ' + ANN),
    10371452: ('R', 'Atelier Ruelle (Mandataire) -> Atelier ruelle; ' + ANN),
    9074384: ('R', 'Ville et Habitat (mandataire) -> Ville et Habitat; ' + ANN),
    17144366: ('R', 'Groupement AEI - Egis - Ecosphere - Presence - Noctiluca -> AEI; np_aei: 16444757 "AEI" on SIRET '
                    '41498295900036, same SIREN 414982959 as this org (41498295900044)'),
    11219959: ('R', 'Guintoli SAS (mandataire du groupement) -> Guintoli; np_guintoli: 10135477 "Guintoli" on SIREN 447754086 = this SIRET\'s SIREN'),
    10797792: ('R', 'Atelier Nervures - mandataire -> ATELIER NERVURES; ' + ANN),
    17308510: ('R', 'Groupement Kardham (mandataire) / Selas Charrel / GMSI -> Kardham; np_kardham: 10519114 "Kardham" on FR12384407896 '
                    'and 30929212 "KARDHAM SAS (Mandataire)" on 38440789600198, both SIREN 384407896 = this org\'s'),
    16969861: ('R', 'AVR ingenierie en groupement avec ... -> AVR INGENIERIE; np_avr: 18911984 "avr ingenierie" on SIRET 48811993400021, '
                    'same SIREN 488119934'),
    14033882: ('R', 'Flash Azur Voyages (mandataire) -> FLASH AZUR VOYAGES; ' + ANN),
    15501192: ('R', 'Bietergemeinschaft Sodexo Service Solutions Austria GmbH -> the GmbH (12 of 13 mentions); np_sodexoat: 4 orgs served it'),
    14493390: ('R', 'AXLR, SATT du Languedoc Roussillon (mandataire du groupement AXLR/Toulouse Tech Transfer) -> SATT AxLR; '
                    'np_axlr: 10416059 "SATT AxLR" on the same SIRET 75364224800029'),
    8321197: ('R', 'Proxidrop (mandataire) -> PROXIDROP; ' + ANN),
    8346204: ('R', 'Satelec (mandataire) -> Satelec (U1 recorded R too); ' + ANN),
    14259882: ('R', 'Mandataire: Cazal SAS -> CAZAL (U1 recorded R too); ' + ANN),
    8217358: ('R', 'Groupement Guintoli/EHTP/GTS/... -> Guintoli; 42 of 64 mentions print Guintoli [SAS] alone on SIRET 44775408600018; '
                   'np_guintoli: 10135477 "Guintoli" on SIREN 447754086'),
    21104135: ('R', 'francisci transport (Mandataire) -> FRANCISCI TRANSPORT SARL; ' + ANN),
    19357393: ('R', 'SONEPAR FRANCE DISTRIBUTION (mandataire) -> SONEPAR France DISTRIBUTION (mixed-case display twin of the 23-vote '
                    'spelling; odd but not wrong); ' + ANN),
    9998909: ('R', 'ETG SARL (mandataire) -> ETG SARL; ' + ANN),
    9225319: ('R', 'Groupement Segc Topo (mandataire) et SARL ATM et SGE Levier-Castelli -> SEGC Topo; 4 of 5 mentions print SEGC Topo '
                   'alone; np_segc: 1440912 "SEGC Topo" (16 mentions)'),
    15165788: ('R', 'Europolia mandataire Toulouse Metropole -> Europolia (U1 recorded R too); 66 of 73 mentions Europolia'),
    15344744: ('R', 'META Group srl (in consortium with Ecorys ... Trust-IT srl) -> META Group Srl; ' + ANN),
    16970038: ('R', 'Consorcio concorrente Quidgest - Consultores de Gestao S. A. e a LCG -> Quidgest - Consultores de Gestao SA; ' + ANN),
    15497200: ('N', 'Groupement de cooperation sanitaire (GCS) Achats du Centre -> GCS Achats du Centre: the head is the body\'s full '
                    'legal name, not a bidder group; the new is the short form 177 of 178 mentions print. Neither is wrong'),
}
UNCERTAIN = {  # U1's three 'uncertain' ids; none matches the subset test (all three are shippable-not-matched)
    11026616: ('R', 'Families Health & Wellbeing Consortium (2018 TED, 1 mention) -> Spring North LTD: every FTS notice 2024-2026 (t_11026616) '
                    'is won under Spring North on CH 07797133; np_springnorth: 31604091 "Spring North" on GBCHC07797133 (same number). '
                    'A rename; "Consortium" was the holder\'s own name (the CPR shape, but the new name is the renamed entity, not an initialism)'),
    9806693: ('R', 'Entreprise PORALU (mandataire) (2017 TED, 1 mention, td_5718388) -> KAPECI: the 3 other mentions are 2025 eForms '
                   '(t_9806693; td_1158841 prints KAPECI as tenderer). Poralu Menuiserie is SIREN 418245262 (np_poralu 16474152), not this '
                   '418245437, so the identifier is KAPECI\'s and the head is the group mandataire printed beside it'),
    10878002: ('N', 'Kerguen mandroit (mandataire) -> "sas cabinet kerguen - mANDROIT": right firm (Cabinet Kerguen-Mandroit, np_kerguen), '
                    'role annotation dropped, but the elected display spelling has broken casing ("mANDROIT"); the 5 spellings are 1 vote each'),
}
assert set(V) == set(s['salted_30']), set(V) ^ set(s['salted_30'])
assert set(UNCERTAIN) <= set(U)
prior_u1 = {k: v for k, v in json.load(open(D + 'u1/judgements.json'))['consortium_removed']['verdicts'].items() if int(k) in M and M[int(k)]['ships']}
t = collections.Counter(v[0] for v in V.values())
tu = collections.Counter(v[0] for v in UNCERTAIN.values())
union = {**{int(k): v[0] for k, v in prior_u1.items()}, **{k: v[0] for k, v in V.items()}}
out = {'meta': {'read': 'salted 30 of the 79 shippable rows the subset test matches (79 > 60)', 'salt': '548-u1c-',
                'bar': '0 wrong (neutral allowed)', 'passes_bar': t['W'] == 0},
       'salted_30': dict(t), 'uncertain_3': dict(tu),
       'recorded_within_79_incl_u1': {'rows': len(union), 'tally': dict(collections.Counter(union.values()))},
       'verdicts': {str(k): {'v': v[0], 'why': v[1], 'old': M[k]['old_name'], 'new': M[k]['new_name'], 'H': M[k]['head']} for k, v in V.items()},
       'uncertain': {str(k): {'v': v[0], 'why': v[1], 'old': U[k]['old_name'], 'new': U[k]['new_name'], 'H': U[k]['head']} for k, v in UNCERTAIN.items()}}
print('salted 30:', dict(t), '| uncertain 3:', dict(tu), '| recorded within the 79 incl. U1:', out['recorded_within_79_incl_u1'])
print('salted 30 H<=1:', sum(M[k]['head'] <= 1 for k in V), 'H>=2:', sorted((k, M[k]['head']) for k in V if M[k]['head'] >= 2))
json.dump(out, open(D + 'u1c/verdicts.json', 'w'), ensure_ascii=False, indent=1)
