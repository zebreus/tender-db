"""build_cohort.py -- 584 U2: build cohort.json (545/317 per-mention shape) and parked.json from the
saved reads (raw/sql/s1..s6 .out, raw/get/*.json) and the per-body decisions below. Read-only;
asserts every rule it applies. Usage: python3 -I scripts/build_cohort.py <584-u2 dir>"""
import json, os, sys
U2 = sys.argv[1]
U1 = os.path.join(U2, '..', '584-u1')
J = lambda p: json.load(open(os.path.join(U2, p)))
foreign = json.load(open(os.path.join(U1, 'org49_foreign.json')))
s1 = J('raw/sql/s1_mentions.out'); s2 = J('raw/sql/s2_parties.out')
s3 = J('raw/sql/s3_candidates.out'); s4 = J('raw/sql/s4_candidates2.out'); s5 = J('raw/sql/s5_identifiers.out')

# every org row read live today (S3/S4: id at col 1; S5: id at col 2)
orgs = {}
for r in s3['rows'] + s4['rows']:
    orgs[r[1]] = dict(id=r[1], cc=r[2], kind=r[3], ident=r[4], prov=r[5], name=r[6], mentions=r[7])
for r in s5['rows']:
    orgs[r[2]] = dict(id=r[2], cc=r[3], kind=r[4], ident=r[5], prov=r[6], name=r[7], mentions=r[8])

# body -> (printed-name predicate, destination, own identifier used, register source, alternatives)
BODIES = [
 ('UW', lambda n: n == 'Uniwersytet Warszawski', 7788, 'NIP 5250011266',
  'MF white list: UNIWERSYTET WARSZAWSKI, NIP 5250011266, REGON 000001258, Krakowskie Przedmieście 26/28 (the address the buyer section prints)',
  '11785 (REGON 000001258, 351), 1188351 (provisional, 2,255)'),
 ('MPWiK', lambda n: n.startswith('Miejskie Przedsiębiorstwo Wodociągów i Kanalizacji w m. st. Warszawie'), 848, 'NIP 5250005662',
  'KRS 0000146138 (api-krs.ms.gov.pl): MPWiK w m.st. Warszawie S.A., NIP 5250005662, Plac Starynkiewicza 5 (printed in the section name)',
  '16545317 (REGON 015314758, 1,304), 1240504 (KRS, 22), provisionals 1191582 (1,197) / 1191583 (1,035)'),
 ('Terra Mota', lambda n: n == 'Terra Mota Sp. z o.o.', 14553663, 'NIP 7382070551',
  'MF white list: TERRA MOTA Sp. z o.o., NIP 7382070551, KRS 0000294862, Gorlice', 'none (only row under the name prefix)'),
 ('PKO BP', lambda n: n.startswith('Powszechna Kasa Oszczędności Bank Polski'), 10689264, 'NIP 5250007738',
  'MF white list: PKO BP S.A., NIP 5250007738, KRS 0000026438', '10635638 (REGON 016298263, 34), 15814004 (KRS, 2)'),
 ('J&M Engineers', lambda n: n == 'Ireneusz Malewicz J&M Engineers (Lider)', 18347269, None,
  'no register read (sole trader, NIP unknown); no identified PL row under ireneusz malewicz / j&m / i&m / jm engineers',
  '20534059 (provisional, "Ireneusz Malewicz prowadzący działalność gospodarczą pod firmą I&M ENGINEERS…", 1)'),
 ('Architektura Plus', lambda n: n == 'ARCHITEKTURA PLUS DESIGN STUDIO MICHAŁ MARCZAK', 23202450, 'NIP 6871892878',
  'MF white list: MICHAŁ MARCZAK, NIP 6871892878, Wrocław (sole trader)', 'none'),
 ('Remondis Bydgoszcz', lambda n: n.startswith('REMONDIS Bydgoszcz'), 15146954, 'NIP 5540312757',
  'KRS 0001015725: REMONDIS BYDGOSZCZ S.A., NIP 5540312757, REGON 090484404, transformed 2023 from the Sp. z o.o. (KRS 0000037763, same NIP/REGON)',
  '20376473 (REGON 090484404, head "REMONDIS Bydgoszcz S.A.", 20), 20798693 (KRS 0001015725, 3), provisional 22053675 (exact double-space spelling, 6)'),
 ('Digitex', lambda n: n == 'Digitex Sp.z o.o Sp. k.', 20111694, 'NIP 5851476863',
  'MF white list: DIGITEX Sp. z o.o. Sp.k., NIP 5851476863, REGON 366410177, Sopot (NIP 5851469863 on org 16922331 belongs to "CHATA CHŁOPSKA" s.c. — a typo row)',
  '19005947 (REGON 366410177, head "Digitex Sp. z o.o.", 18), provisional 10248431 (6)'),
 ('NOVUM S.A.', lambda n: n == 'NOVUM S.A.', 8998814, 'NIP 9491924705',
  'MF white list: NOVUM S.A., NIP 9491924705, REGON 152185875, KRS 0000368094, Warszawa', '23169433 (REGON 152185875, 2), provisional 6334556 (94)'),
 ('WTU', lambda n: n == 'WTU Spółka z o.o.', 13575097, 'NIP 8992680461',
  "MF white list: 'WTU' Sp. z o.o., NIP 8992680461, REGON 021065730, KRS 0000340110", '9713424 (REGON, 4), provisional 1629934 (16)'),
 ('PZU Zdrowie', lambda n: n.startswith('PZU ZDROWIE S.A.'), 18389587, 'NIP 5272663852',
  'MF white list: PZU ZDROWIE S.A., NIP 5272663852, REGON 143343166, KRS 0000395215 (the printed "Oddział Centra Medyczne w Gdańsku" is a branch of this legal person)',
  '20008293 (REGON, 3), 14416611 (KRS, 1)'),
 ('Evalu', lambda n: n == 'Evalu Sp. z o.o.', 20491831, 'NIP 5262825916',
  'MF white list: EVALU Sp. z o.o., NIP 5262825916, REGON 015891042', '18436807 (REGON, 2), provisional 10008177 (exact spelling, 19)'),
 ('Zarys', lambda n: n == 'Zarys International Sp. z o.o.', 976, 'NIP 6481997718',
  'KRS 0001213002: ZARYS INTERNATIONAL Sp. z o.o., NIP 6481997718, transformed 2025-11-14 from ZARYS INTERNATIONAL GROUP sp. z o.o. sp.k. (KRS 0000540772, same NIP/REGON)',
  '9241 (REGON 273295877, head "Zarys International Sp. z o.o.", 2,129), 23719024 (KRS 0001213002, 2), provisional 6545028 (31)'),
 ('GZGK Wojcieszków', lambda n: n == 'Gminny Zakład Gospodarki Komunalnej Sp. z o.o.', 22583689, 'REGON 061423397',
  'MF white list: GZGK Sp. z o.o., REGON 061423397, NIP 8250003819 (no org holds this NIP), Kościelna 46, 21-411 Wojcieszków = the street, postcode and town notice 29173048 prints in ORG-0003',
  'provisional 2199877 (generic exact name pooling many GZGKs, 34) — rejected'),
 ('Pro 3Ds', lambda n: n == 'Pro3Ds Paweł Krawczyk', 22456216, 'NIP 7123245197',
  'MF white list: PAWEŁ KRAWCZYK, NIP 7123245197, Lublin (buyer: OISW w Lublinie)', 'none'),
]
KEEP = {
 'National Appeals Chamber': 'the English BT-500 of KIO; the same section also prints POL "Krajowa Izba Odwoławcza" (S6) and KIO\'s NIP, and the section is bound in review roles only',  # finalizer 2026-10-24: was 'in a Lot-ReviewOrg (and Lot-Mediator) role', true only of 25398003 (challenge.md §2 note 2; S2)
 'Urząd Zamóweń Publicznych': 'a one-letter misspelling of Urząd Zamówień Publicznych, which is the NIP\'s own holder (MF white list: 5262239325 = UZP, REGON 010828091; KIO sits inside UZP), in Lot-ReviewOrg + Lot-ReviewInfo roles',
}
BUYER_SIDE = {'Procedure-Buyer', 'Lot-TenderEval', 'Lot-TenderReceipt', 'Lot-DocProvider'}
SUPPLIER_SIDE = {'Tenderer', 'bid:tenderer'}
REVIEW = {'Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-Mediator'}

ments = {(r[0], r[1]): r for r in s1['rows']}
parties = {}
for r in s2['rows']:
    if r[0] is None:
        continue
    parties.setdefault((r[0], r[1]), []).append(r)
winners = {r[2]: r for r in s2['rows'] if r[3] == 'winner'}

verdicts, parked, evidence = [], [], []
for mid, notice, section, name in foreign:
    m = ments[(notice, section)]
    assert m[2] == 49 and m[3] == name, (notice, section, m)
    assert m[5] and ''.join(c for c in m[5] if c.isdigit()) == '5262239325', m
    ps = parties.get((notice, section), [])
    assert ps, ('no party rows', notice, section)
    roles = sorted({p[3] for p in ps}); tenders = sorted({p[2] for p in ps})
    assert len(tenders) == 1, ('section bound on several tenders', notice, section, tenders)
    tender = tenders[0]
    on_cur = any(p[10] > 0 for p in ps)
    sides = {('buyer' if r in BUYER_SIDE else 'supplier' if r in SUPPLIER_SIDE else 'review' if r in REVIEW else 'other') for r in roles}
    assert len(sides) == 1, ('section binds roles of more than one side', notice, section, roles)
    side = sides.pop()
    win = tender in winners and side == 'supplier'
    row = dict(mention_rowid=mid, notice_id=notice, section_id=section, printed_name=name, raw_identifier=m[5],
               tender=tender, roles=roles, on_current_version=on_cur, winner_on_49=win)
    if name in KEEP:
        assert side == 'review', (name, roles)
        v = dict(org_id=49, notice_id=notice, section_id=section, action='keep', target_org_id=None, target_name=None,
                 rationale=f"Notice {notice} prints '{name}' in {section} with NIP 5262239325 as {', '.join(roles)} on tender {tender}: {KEEP[name]}, so the mention is the identifier's own body and stays on org 49.",
                 confidence='high')
        row.update(verdict='keep')
    else:
        hits = [b for b in BODIES if b[1](name)]
        assert len(hits) == 1, (name, hits)
        tag, _, dest, own, reg, alts = hits[0]
        o = orgs[dest]
        assert o['cc'] == 'PL', o
        if own:
            assert o['ident'] == ''.join(c for c in own if c.isdigit()), (o, own)
            assert not o['prov']
        else:
            assert o['prov'] and o['name'] == name, o  # provisional exact-name destination (545 precedent)
        role_txt = {'buyer': 'buyer-side (' + ', '.join(roles) + ')', 'supplier': 'Tenderer' + (' and winner' if win else '')}[side]
        if tag == 'MPWiK':
            role_txt += ' as co-buyer beside GDDKiA (ORG-0001, its own NIP 1132097244)'
        why_dest = (f"the org holding the body's own {own} ({o['mentions']} mentions; {reg.split(':')[0].split(' (')[0]} confirms the number)" if own
                    else f"the only PL row carrying exactly the printed name (provisional, {o['mentions']} mention; no identified row exists)")
        v = dict(org_id=49, notice_id=notice, section_id=section, action='rehome', target_org_id=dest, target_name=o['name'],
                 rationale=f"Notice {notice} prints '{name.strip()}' alone in {section} with KIO/UZP's NIP 5262239325, bound {role_txt} on tender {tender}; the review body has no part in that section, so it moves to {dest} '{o['name']}', {why_dest}.",
                 confidence='high')
        row.update(verdict='rehome', body=tag, target_org_id=dest, target_name=o['name'], target_identifier=o['ident'],
                   target_mentions=o['mentions'], target_provisional=bool(o['prov']), own_identifier=own, register=reg, alternatives=alts)
    verdicts.append(v); evidence.append(row)

# output checks (545's list)
keys = [(v['notice_id'], v['section_id'], v['org_id']) for v in verdicts]
assert len(keys) == len(set(keys)) == 38
for v in verdicts:
    assert v['action'] in ('rehome', 'keep') and v['confidence'] in ('high', 'medium', 'low')
    assert (v['action'] == 'rehome') == (v['target_org_id'] is not None)
verdicts.sort(key=lambda v: (v['notice_id'], v['section_id']))
evidence.sort(key=lambda v: (v['notice_id'], v['section_id']))
json.dump({'cohort': '584-2026-10-24', 'verdicts': verdicts}, open(os.path.join(U2, 'cohort.json'), 'w'), indent=1, ensure_ascii=False)
json.dump(evidence, open(os.path.join(U2, 'raw', 'evidence.json'), 'w'), indent=1, ensure_ascii=False)
json.dump({'cohort': '584-2026-10-24', 'parked': parked,
           'note': 'No mention was parked: every one of the 32 names an existing destination (15 bodies, 14 identified by the body\'s own register number, 1 provisional exact-name row), every section is bound on exactly one tender, and no section binds roles of more than one side or names more than one body.'},
          open(os.path.join(U2, 'parked.json'), 'w'), indent=1, ensure_ascii=False)
from collections import Counter
print(Counter(v['action'] for v in verdicts), Counter((v['target_org_id'], v['target_name']) for v in verdicts if v['action'] == 'rehome'))
print('tenders touched by rehomes:', len({e['tender'] for e in evidence if e['verdict'] == 'rehome'}))
