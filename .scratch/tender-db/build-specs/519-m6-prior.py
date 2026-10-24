"""519 M6 offline prior (2026-10-24): classify the 10-12 E/F GB-NHS orgs and propose a
candidate correct_identifier from the 10-12 name twins and the 513/519 closing plan 2348.
A HINT for the review packet, never a verdict."""
import json, re, sys, collections
B = sys.argv[1]  # .scratch/tender-db/519-nhs-digit-codes
cl = json.load(open(f'{B}/gbnhs-orgs-classified.json'))
tw = json.load(open(f'{B}/name-twins-2026-10-12.json'))
plan = json.load(open(f'{B}/u3/plan-2348-dry-closing.json'))
ppon_owner = {}   # serial -> (org, identifier, name, via)
for c in plan['codes']:
    for h in c['holders']:
        idf = h.get('identifier') or ''
        if idf.startswith('GBPPON'):
            ppon_owner.setdefault(idf[6:], (h['org_id'], idf, h['name'], '513-' + c['code']))
for k, rows in tw.items():
    for (oid, cc, kind, idf, prov) in rows:
        if (idf or '').startswith('GBPPON') and cc == 'GB' and kind == 'national':
            ppon_owner.setdefault(idf[6:], (oid, idf, k, 'twin'))
NHSE = re.compile(r'nhs ?england|commissioning region|commissioning hub|health and justice', re.I)
NHSBODY = re.compile(r'\bnhs\b|trust\b|integrated care board|\bicb\b|health board|ambulance|blood transfusion|business services organisation', re.I)
PLACE = re.compile(r'\bplace\)|\(.*(advanced therapies|pathology|lead icb).*\)|pathology', re.I)
def shape(code):
    if re.fullmatch(r'P[A-Z]{3}\d{4}[A-Z]{4}', code): return 'ppon'
    if re.fullmatch(r'P[A-Z]{3}\d{3,4}[A-Z]{4,5}', code): return 'ppon-malformed'
    if re.fullmatch(r'\d{8}', code): return 'coh8'
    if re.fullmatch(r'(SC|NI|OC|SO|NC|FC|SE|GE|IP|SP|CE|EE)\d{6}', code): return 'coh2-or-register'
    if re.fullmatch(r'(F[A-Z0-9]{4}){2,}', code): return 'glued-pharmacy'
    if code.startswith('OCDS'): return 'ocds'
    if code.startswith('CQC'): return 'cqc'
    if re.fullmatch(r'UK[A-Z0-9]{1,3}|UK1LONDON', code): return 'nuts'
    if re.fullmatch(r'G\d{7}', code): return 'g-number'
    if re.search(r'(NHS|ENGLAND|TRUST|ICB|REGION|HUB|CONTRACTOR|APPLICATION)', code) and len(code) > 12: return 'typed-name'
    if re.fullmatch(r'ODS[0-9A-Z]{4,6}|[0-9A-Z]{2,4}(ODS|NHS)|GBNHS[0-9A-Z]{2,4}', code): return 'ods-glued'
    if re.fullmatch(r'\d{4,10}', code): return 'numeric'
    return 'other'
rows = []
for x in cl:
    f = x['ods'].get('found')
    if f not in (None, False):
        continue
    code, name = x['code'], x['name']
    s = shape(code)
    twins = [o for o in tw.get(x['name_norm'], []) if o[0] != x['id']]
    pp = [o for o in twins if (o[3] or '').startswith('GBPPON')]
    coh = [o for o in twins if re.fullmatch(r'(GBCOH)?\d{8}|(GBCOH)?[A-Z]{2}\d{6}', o[3] or '')]
    prop = {'route': None, 'correct': None, 'target': None, 'note': ''}
    if NHSE.search(name):
        prop.update(route='nhse-523', note='NHS England: wrong, no merge here (issue 523)')
    elif s == 'ppon':
        o = ppon_owner.get(code)
        prop.update(route='ppon-value', correct='GBPPON' + code,
                    target=list(o) if o else None,
                    note='owner known' if o else 'no known owner: a move writes the bare serial -> post medium')
    elif PLACE.search(name):
        prop.update(route='site-or-hosted', note='513 HOSTED_OR_SITE precedent: wrong, no correction')
    elif NHSBODY.search(name) and len(pp) == 1:
        o = pp[0]
        prop.update(route='body-ppon', correct=o[3], target=[o[0], o[3], x['name_norm'], 'twin'])
    elif s in ('coh8', 'coh2-or-register'):
        prop.update(route='coh-value', correct=code, target=[[o[0], o[3]] for o in coh] or None,
                    note='register check: the number must be this org\'s')
    elif len(coh) >= 1 and not NHSBODY.search(name):
        prop.update(route='company-coh', correct=None, target=[[o[0], o[3]] for o in coh],
                    note='a same-name org carries a company number: register-check it, then correct = it')
    elif NHSBODY.search(name):
        prop.update(route='body-no-ppon', note='typo: correct = the right ODS code (unkeyed); junk: no correction')
    else:
        prop.update(route='junk', note='wrong, no correction (withheld)')
    rows.append({'org': x['id'], 'identifier': x['identifier'], 'code': code, 'name': name,
                 'mentions': x['mentions'], 'ods': 'not-ods-shaped' if f is None else '404',
                 'shape': s, 'prior': prop})
c = collections.Counter(r['prior']['route'] for r in rows)
m = collections.Counter()
for r in rows: m[r['prior']['route']] += r['mentions']
print(len(rows), sum(r['mentions'] for r in rows))
for k, v in c.most_common(): print(f'{k:16} {v:4} orgs {m[k]:5} mentions')
kn = sum(1 for r in rows if r['prior']['route']=='ppon-value' and r['prior']['target'])
print('ppon-value with known owner', kn)
code_survivors = {}
for c in plan['codes']:
    s = [h for h in c['holders'] if ('GBNHS' + c['code']) in (h.get('merged_identifiers') or []) or h.get('identifier') == 'GBNHS' + c['code']]
    if s:
        h = s[0]
        code_survivors[c['code']] = {'org': h['org_id'], 'identifier': h.get('identifier'), 'name': h['name'], 'provisional': h['provisional']}
# M5 (measure/m5.md): QE1 (a digit parent, U6) has a standing PPON org beside its GBNHSQE1 row.
code_survivors['QE1'] = {'org': 31569664, 'identifier': 'GBPPONPBWM8336DJRG',
                         'name': 'nhs lancashire and south cumbria integrated care board', 'provisional': False,
                         'note': 'not a 513 survivor: M5, the PPON org beside 30915463 GBNHSQE1'}
json.dump({'generated': '2026-10-24, offline from the 2026-10-12 files (gbnhs-orgs-classified.json, '
                        'name-twins-2026-10-12.json, u3/plan-2348-dry-closing.json); a hint for the M6 packet, never a verdict',
           'counts': {k: {'orgs': v, 'mentions': m[k]} for k, v in c.items()},
           'code_survivors': code_survivors, 'rows': rows}, open(sys.argv[2], 'w'), indent=1)
