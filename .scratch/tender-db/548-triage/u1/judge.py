# 548 U1: the judged verdicts (R = serving new_name would be right, N = neutral, W = wrong), tallied.
import json, collections
D = '/home/user/tender-db/.scratch/tender-db/548-triage/u1/'
V = {
 'family': {  # all 33 shippable read (10 salted first)
  17990506:'R brand token APS read as a form; AcegasApsAmga S.p.A. is the name', 10486452:'R HDR Communications -> Kardham Digital (2023/2025 notices print Kardham)',
  17145263:'R SARL->SAS conversion, same SIRET', 17310252:'R rename (K-BV ... vormals SIG-...)', 13470498:'R head is the association; the RO CUI is the Bucharest SRL',
  15351295:'R rename to Energie AG Teplo Bohemia (brand token AG)', 14721467:'R? 2026 notice prints CONSTRUCTII MONTAJ AG on this CUI; probable rename',
  3741752:'R consortium-sentence head -> the UAB', 18179679:'R bilingual S.R.L./G.mbH -> S.r.l.', 3728305:'W new carries "v jazyku nemeckom ... anglickom" annotation (H=24)',
  20957059:'R stray JEOL Nordic AB on JEOL (Germany) GmbH VAT', 7081895:'W "A B A U T O B O S S s.r.o." letter-spaced (H=6)', 16388387:'R Mieci -> Renovit Public Solutions (merger)',
  16674633:'R srl -> S.p.A. conversion', 16432719:'R PwC Public Sector -> Intellera Consulting (rename)', 7484446:'R Cofely/ENGIE Services Noord -> EQUANS Noord-Nederland',
  20861533:'R stray SIA SIGMEN on LTG Infra code', 10656161:'R SARL -> SAS', 12448555:'R brand token SPA', 11852438:'R Nomisma full S.p.A. name',
  13995127:'R SARL -> SAS', 19852879:'R stray Progress Rail on ENSER P.IVA', 18362669:'R Sagres -> GORI S.p.A. on GORI P.IVA', 12415594:'R stray Toscana Bus on AVR',
  20415415:'W UAB Barameda (LT) on an Austrian VAT; 7 votes all from ONE notice (q02)', 8309719:'R Maquet Italia -> Getinge Italia (rename)', 17283653:'R Gentile Ambiente S.p.A.',
  17355314:'R Terraculture Provence', 12570897:'R Franchetti S.p.A.', 13263803:'R Swietelsky AG conversion', 10181124:'R rename (full Zrt form)', 10980003:'R Vodos -> Energie AG Kolin a.s.',
  10037554:'W new carries "(angl. Ltd, nem. GmbH, franc. S.R.L.A.)" annotation (H=25)'},
 'gb_family': {10319218:'R CH 00264322 = BETHELL CONSTRUCTION LIMITED', 15909819:'R 05458987 is a company number, not an LLP (OC); LLP head is stray',
  10047073:'R CH 02228297 = EDF ENERGY CUSTOMERS LIMITED', 13448374:'R same entity, PLC short form', 10213161:'R CH = SCOTIA GAS NETWORKS LIMITED',
  11628199:'R CH = CAPITA PLC', 9087207:'R CH 04273754 = MORGAN SINDALL CONSTRUCTION & INFRASTRUCTURE LTD'},
 'consortium_removed': {15165788:'R', 18808472:'R', 10804351:'R', 21596565:'R', 14259882:'R', 8346204:'R',
  18287356:'W "Consortium" is the company name; SARL CPR is an initialism (2 votes one notice, q02)', 14831056:'R Catalan official name of a public consortium',
  11030337:'R', 11072974:'R UTE head -> Cymot S.A. (Cymotsa)'},
 'organ': {4521469:'W organ', 5129828:'W organ', 5044100:'W organ', 12291526:'W organ', 5207211:'W organ', 5326030:'W organ', 7354337:'W organ',
  11229692:'N organ for organ', 13847407:'W organ', 5216866:'W organ'},
 'code_added': {11001761:'W _1274', 21038974:'N lower-case head, code added', 18301242:'N lower-case head, code added', 15323093:'W code added',
  18946095:'N lower-case head, code added', 11476091:'W code added', 10379298:'W code added', 21884902:'N lower-case head, code added', 9502566:'W code added', 14319363:'W code added'},
 'address_added': {9563497:'W', 8270356:'N both carry addresses', 12960092:'W', 8571882:'W', 13256112:'W', 15019832:'W', 18982169:'W', 5372850:'W', 17147496:'W', 9339625:'W'},
 'seat_dropped': {17824715:'N', 3283530:'N', 20132104:'N', 20145666:'W stray quote, caps', 17738449:'W generic (4 orgs served exactly this)',
  9741898:'N', 17902871:'N', 5350673:'W generic (7)', 16701291:'W generic (23)', 17824709:'W generic (4)'},
 'review_body': {11595231:'W', 15556355:'W', 9195726:'W', 6806003:'W', 21185404:'W', 20435435:'W', 9512566:'W', 17974417:'W'},
 'caps_variant': {15933261:'W', 20625534:'N', 16777451:'N', 13804962:'N', 11520587:'N', 16430606:'W', 9367553:'W', 20766023:'R', 21128622:'R', 14864888:'W'},
 'doubled': {2566692:'W', 12031122:'W', 6546919:'W', 12353904:'W', 10064219:'W', 3948628:'W', 15319718:'W', 3909556:'W', 12386123:'W', 14678837:'W'},
 'no_fit': {11760208:'- no spelling to serve', 14972269:'-', 21425568:'-', 10202605:'-', 14548612:'-', 10829487:'-', 7356155:'-', 13901395:'-', 13515946:'-', 14882722:'-'},
 'head_current_dated30': {22699221:'R',21150938:'R',22793049:'R',23187854:'R',22349201:'R w likwidacji',23004257:'N',31571991:'N',11059191:'W drops ehf',22525055:'R renamed ministry',
  22142633:'R',22185918:'N',16324356:'R',5792926:'N',9623394:'N',22493638:'R',22639281:'R',22342044:'N',22560163:'R',23626335:'R GmbHx typo',22140973:'R',30534934:'R',17571396:'R',
  22594426:'R',22695310:'R',22627722:'W adds org number',22667102:'W buyer name on a supplier org',11342945:'R',22671320:'R?',30917051:'R',15824334:'N'},
}
ship = json.load(open(D + 'shippable.json')); samp = json.load(open(D + 'samples.json'))
out = {}
for cls, v in V.items():
    t = collections.Counter(x[0] for x in v.values())
    sampled = [r['org_id'] for r in samp.get(cls, [])]
    ts = collections.Counter(v[o][0] for o in sampled if o in v)
    stray = collections.Counter(v[r['org_id']][0] for r in ship.get(cls, []) if r['org_id'] in v and r['head'] <= 1)
    out[cls] = {'shippable': len(ship.get(cls, [])), 'read': len(v), 'all_read': dict(t), 'salted_sample': dict(ts), 'stray_H<=1_read': dict(stray), 'verdicts': {str(k): x for k, x in v.items()}}
    print(cls, out[cls]['shippable'], 'read', len(v), dict(t), 'salted', dict(ts), 'H<=1', dict(stray))
json.dump(out, open(D + 'judgements.json', 'w'), ensure_ascii=False, indent=1)
