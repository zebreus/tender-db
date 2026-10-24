"""Issue 586 unit 1: the salted-40 tables of fold.md (final rules), with the hand judgement recorded in JUDGED.
usage: python3 -I tables.py <audit-576-fanout dir>"""
import hashlib, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fold

D = sys.argv[1]
t2 = json.load(open(os.path.join(D, 'second-look/raw/t2_classified.json')))
WHY = {'A': 'same name, spelling only', 'C0': 'same organ, JGL/Pleno spelling', 'C1': 'the entity under its organ',
       'C2': 'the town hall, another word for it', 'C3': 'the bare town under its town hall',
       'C3r': 'the town hall of the bare-named parent'}
# Every row of both salted 40s was read by hand on 2026-10-24: all R (same body). No W.
JUDGED = 'R'
rows = [(r, fold.tier(r['name'], r['pp'])) for r in t2]
for k, salt in [('A', '586-u1-tierA'), ('C', '586-u1-tierC')]:
    pop = [(r, t) for r, t in rows if t and t[0] == k]
    s = sorted(pop, key=lambda rt: hashlib.sha256(f"{salt}:{rt[0]['id']}".encode()).hexdigest())[:40]
    print(f'\n#### Tier {k}: salt `{salt}`, 40 of {len(pop)}\n')
    print('| # | org | tier | m | parent (printed, same notice) | touchpoint | judged |')
    print('|---|---|---|---|---|---|---|')
    for i, (r, t) in enumerate(s, 1):
        pp = r['pp'].replace('|', '/').replace('\n', ' ')
        nm = r['name'].replace('|', '/').replace('\n', ' ')
        print(f"| {i} | {r['id']} | {t} | {r['tpo']} | {pp} | {nm} | {JUDGED}: {WHY[t]} |")
