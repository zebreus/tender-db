# 548 U1b: the T1 dated 30 judged. Read-only. Inputs: sample.json (arm.py), api/*.json (public GETs, fetch.py).
# Verdict = is the change (old_name -> new_name) right (R: better), neutral (N) or wrong (W: worse, another entity,
# or a same-entity downgrade, as 524 gate (d) labelled 5638950 Brlic). A REVERT = the new name is a spelling the head
# had superseded (new spelling's newest print older than the head's); none in the arm: top_newest > head_newest in
# all 1,318 rows. Bar (D4): at most 1 wrong, 0 reverts.
import json, collections
T = '/home/user/tender-db/.scratch/tender-db/548-triage/u1b/'
V = {
 4930460: ('R', "consortium head 'Zajednica ponuditelja: ABB d.d.; Dalekovod-...' -> 'ABB d.o.o.': 8 of 11 mentions on OIB 73301344150 print ABB d.o.o. alone; the head is a bidder-group line"),
 12915436: ('R', "'Autocars Strebig SAS — groupe Keolis' (misspelt, group tail) -> 'AUTOCARS STRIEBIG': 4 of 6 print Striebig, newest 2025-07-24"),
 18001989: ('N', "'Stichting Trias Innervatie' (2021) -> 'Energiebox': t_18001989 shows the 2022-2024 wins (energy coaches, energiearmoede) printed as Energiebox on KvK 73811483; np_energiebox finds no other Energiebox org, np_trias the stichting only. Trade name vs legal name; rename not confirmed"),
 20247399: ('N', "both carry the role marker: 'Partner: „METALIKA RECYCLING” SPÓŁKA Z O.O.' -> 'METALIKA RECYCLING Sp. z o.o. - partner'; shorter form, marker kept"),
 6162929: ('R', "typo fix: 'Ympäristönhuolto' -> 'Ympäristöhuolto' (6 of 8)"),
 9168556: ('W', "'Usługi Leśne Ryszard Flak' -> 'Partner konsorcjum Usługi Leśne Ryszard Flak': ADDS a consortium-role prefix (4 of 7, 4 notices); np_flak shows the 'Partner konsorcjum ...' shape on 100+ PL orgs. No serve hold tests a role marker ADDED"),
 15402029: ('N', "'Mariusz Siwiak Firma „Drwal\"' -> 'Firma ,,DRWAL\" Mariusz Siwiak': same name reordered, ASCII ,, quotes; top cluster is 'FIRMA DRWAL MARIUSZ SIWIAK' 2 + this 1"),
 12782788: ('R', "'Konsorcjum firm: ... Piotr Graczewski (Wykonawca)' -> 'Zakład Usług Leśnych Piotr Graczewski' (5 of 7): role line dropped"),
 10092211: ('N', "'Instalacije in materiali SA d.o.o.' -> 'IMSA, INSTALACIJE IN MATERIALI SA, prodaja instalacijskega materiala, d.o.o.': the full registered form, but the elected display is the caps print (8) over the mixed-case 5+3+1"),
 11039579: ('R', "head carries an address and the old surname; 'Firma Usługowa Joanna Skwierawska' on the same NIP 24 of 26, newest 2025-02-14 (surname change)"),
 11058007: ('N', "'Carflor Servicios Asistenciales, S.L.' -> 'Carflor, S. L.' (6 of 8 on CIF B24214926; np_carflor: a provisional 'Carflor, S. L.' org too). Legal name unverified"),
 17269219: ('W', "'Mithieux TP' -> 'MITHIEUX': same firm, but the elected print drops 'TP' and is all caps; the same SIRET's lot orgs (np_mithieux 22884413/22884415, 32652078000031LOT1/2) and 'société mithieux tp' print MITHIEUX TP, and np_mithieux shows a different MITHIEUX OI (SIRET 97158247000014). Same-entity downgrade, as 524 labelled Brlic"),
 21495464: ('R', "head is a procedure title ('Výběrové řízení \"pořízení sportovních setů ...\"') -> 'Česká jezdecká federace' (3 of 5)"),
 1578233: ('N', "'DREAL Bourgogne' -> 'MEDDE — DREAL Bourgogne' (16 of 21): parent ministry prefix added; both pre-2016 names"),
 1808521: ('N', "'Instituto Municipal de la Vivienda del Excmo. Ayuntamiento de Málaga' -> 'Instituto Municipal de la Vivienda de Málaga' (58 of 65): shorter form of the same body"),
 22088092: ('N', "'Baudepartement Stadt Zug Abteilung Hochbau' -> 'Baudepartement Stadt Zug' (13 of 16): the sub-unit dropped"),
 8587172: ('R', "'SAS Lerichemont' (2 prints, newest 2017) -> 'Hénéo' (38 of 51, newest 2024-08-29): b_8587172 shows the identifier buying as Hénéo 2024 (SIRH, ménage, radiateurs); np_heneo finds Hénéo orgs on other ids only; Le Richemont was Hénéo's earlier name"),
 8917378: ('N', "'Aspira' -> 'Aspira (AspiraCon Ltd., trading as Aspira)' (15 of 24): adds the legal entity in a long display form"),
 8848515: ('N', "'Dirección Provincial de la Tesorería General de la Seguridad Social de Sevilla' -> 'TGSS-Dirección provincial de Sevilla': abbreviation of the same office"),
 10975171: ('N', "'Enabel — Agence belge de développement' -> 'Enabel' (5 of 9; RW id = Enabel Rwanda): descriptor dropped"),
 12467678: ('N', "'JJ Santo António, Transporte Regular Passageiros, Unipessoal, Limitada' -> 'JJ Santo António - Transporte Rodoviário de Passageiros, Unipessoal, LDA': variant of the same firm's name; register name unverified"),
 11144274: ('N', "'Atelier protégé au service d'entreprise' -> 'APASE' (7 of 9): the body's acronym (the initialism shape; R1, so head_r2_hold is not asked)"),
 12752600: ('R', "'SARL assurances Jouet Lelievre' (2019) -> 'Assurances Jouet Florin' (3 of 5, newest 2021-01-20): np_jouet shows the agency printed Jouet Florin in later notices (22299964, 20874463, 15669052, 15429946) and Jouet Lelièvre in earlier ones: the MMA agency's partner changed on the same SIRET"),
 9431487: ('R', "'Grundfos DK A/S, Center Øst' -> 'Grundfos DK A/S' (4 of 6): site tail dropped"),
 3176582: ('N', "'Békés Megyei Rendőrfőkapitányság' -> 'Békés Megyei Rendőr-főkapitányság' (15 of 23): the official hyphenation"),
 4688514: ('N', "'Uždaroji akcinė bendrovė „Transrevis“' -> 'UAB „Transrevis“' (18 of 23): legal form abbreviated (live mentions 24, one new since the plan)"),
 9225060: ('R', "'Hoivia Oy/ Palvelukoti Keinula' -> 'Hoivia Oy' (6 of 8): unit tail dropped"),
 13974288: ('R', "'Αγροτικός Ελαιουργικός Συν/σμός Αχλαδόκαμπου' -> 'Αγροτικός Ελαιουργικός Συνεταιρισμός Αχλαδοκάμπου': abbreviation expanded, accent corrected"),
 2976481: ('R', "'Dům dětí a mládeže Libertin' -> '..., příspěvková organizace' (4 of 7): the full legal name"),
 3333925: ('R', "'BKK Közút Zrt' (22, newest 2015-11) -> 'Budapest Közút Zártkörűen Működő Részvénytársaság' (92, newest 2022-01): the company's rename"),
}
s = json.load(open(T + 'sample.json'))
assert [r['org_id'] for r in s] and set(V) == {r['org_id'] for r in s}
live = {}
for r in s:
    d = json.load(open(T + 'api/org_%d.json' % r['org_id']))
    live[r['org_id']] = dict(name_is_head=d['name'] == r['old_name'], identifier_same=d['identifier'] == r['identifier'], mentions_live=d['mentions'], mentions_plan=r['n'])
    assert live[r['org_id']]['name_is_head'] and live[r['org_id']]['identifier_same']
tally = collections.Counter(v for v, _ in V.values())
h1 = collections.Counter(V[r['org_id']][0] for r in s if r['head'] <= 1)
h2 = collections.Counter(V[r['org_id']][0] for r in s if r['head'] >= 2)
reverts = [r['org_id'] for r in s if r['top_newest'] < r['head_newest']]
out = dict(meta=dict(arm='top_stale rows with NOT (rival_newest > top_newest): T1 releases these', salt='548-u1b-',
                     bar='at most 1 wrong, 0 reverts', passes_bar=tally['W'] <= 1 and not reverts),
           tally=dict(tally), by_head=dict(H_le_1=dict(h1), H_ge_2=dict(h2)), reverts=reverts,
           verdicts={str(r['org_id']): dict(v=V[r['org_id']][0], why=V[r['org_id']][1], old=r['old_name'], new=r['new_name'],
                                            H=r['head'], top=r['top'], voting=r['voting'], head_newest=r['head_newest'],
                                            top_newest=r['top_newest'], rival_newest=r['rival_newest'], live=live[r['org_id']]) for r in s})
json.dump(out, open(T + 'verdicts.json', 'w'), ensure_ascii=False, indent=1)
print(out['meta'], out['tally'], out['by_head'], 'reverts', reverts)
