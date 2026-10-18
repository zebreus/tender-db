"""Verifier's own notice view of one Tender for labelling. Shows the chain notices' notice-level ids, every Lot
section, every LotResult block with its winners, and the Tender's carried lots. It never prints a lot_result's
served `lot`, nor any guard/replay outcome. Usage: python3 -I nview.py CACHE TID"""
import json, os, sys
cache, tid = sys.argv[1], int(sys.argv[2])
ld = lambda p: json.load(open(p, encoding='utf-8'))
t = ld(os.path.join(cache, f'tenders_{tid}.json'))
lst = ld(os.path.join(cache, f'notices_tender_{tid}.json'))
prof = {x['id']: x for x in lst['items']}
def val(v):
    if v.get('type') == 'amount' or 'cents' in v:
        return f"{v.get('cents', 0)/100:,.2f} {v.get('currency')}"
    if 'value' in v: return v['value']
    if 'code' in v: return v['code']
    return {k: x for k, x in v.items() if k not in ('field_id', 'ordinal', 'type', 'lang')}
def fv(sec, fid, lang=None):
    out = []
    for v in sec['values']:
        if v['field_id'] == fid and (lang is None or v.get('lang') in (lang, None, '')):
            out.append(val(v))
    return out
def short(x, n=400):
    s = ' | '.join(str(y) for y in x) if isinstance(x, list) else str(x)
    return s if len(s) <= n else s[:n] + '…'
print(f"TENDER {tid} v{t['version']} key={t['procedure_key']} title={t['title']!r} published={t['published_at']}")
for v in t['versions']:
    print(f"  version seq {v['seq']}: notice {v['caused_by_notice_id']} pub {v['publication_id']} {v['published_at']}")
print('LISTING:', [(i, x['profile'], x['declared_version'], x['publication_id']) for i, x in sorted(prof.items())],
      'more=', lst.get('more'))
print('TENDER LOTS (carried state):')
texts = {}
for x in t['texts']:
    texts.setdefault(x['lot'], []).append(f"{x['field']}[{x['lang']}]: {x['value']}")
for l in t['lot_details']:
    print(f"  {l['lot_key']} title={l.get('title')!r} value={l.get('value')}")
    for s in texts.get(l['lot_key'], []):
        print('      ', short(s, 300))
for a in t['amounts']:
    if a.get('lot'): print('   amount', a)
notices = sorted({v['caused_by_notice_id'] for v in t['versions']} | {r['notice_id'] for r in t['lot_results']})
for n in notices:
    p = os.path.join(cache, f'notices_{n}_content.json')
    if not os.path.exists(p):
        print(f'NOTICE {n}: (content not cached)'); continue
    c = ld(p)
    secs = c['sections']
    proc = [s for s in secs if s['kind'] == 'Notice']
    print(f"\n=== NOTICE {n} profile={prof.get(n, {}).get('profile')} {prof.get(n, {}).get('declared_version')} pub={prof.get(n, {}).get('publication_id')}")
    for s in proc:
        for f in ('TED-TD_DOCUMENT_TYPE', 'TED-FORM', 'TED-NO_DOC_OJS', 'TED-NOTICE_NUMBER_OJ', 'TED-REF_NOTICE.NO_DOC_OJS',
                  'TED-FILE_REFERENCE_NUMBER', 'TED-TITLE_CONTRACT', 'TED-DIV_INTO_LOT_YES', 'TED-DIV_INTO_LOT', 'TED-LOT_DIVISION',
                  'TED-ADDITIONAL_INFORMATION', 'TED-CONCLUSION_FRAMEWORK_AGREEMENT', 'TED-VALUE', 'TED-VALUE_COST'):
            x = fv(s, f)
            if x: print(f'   {f}: {short(x, 500)}')
    for s in secs:
        if s['kind'] == 'Lot':
            print(f"  LOT section {s['section_id']}: number={fv(s,'TED-LOT_NUMBER')} title={short(fv(s,'TED-LOT_TITLE'),200)}")
            d = fv(s, 'TED-LOT_DESCRIPTION')
            if d: print(f"      desc: {short(d, 350)}")
            q = fv(s, 'TED-TOTAL_QUANTITY_OR_SCOPE')
            if q: print(f"      scope: {short(q, 250)}")
            for f in ('TED-VALUE_COST', 'TED-LOW_VALUE', 'TED-HIGH_VALUE', 'TED-ADDITIONAL_INFORMATION_ABOUT_LOTS'):
                x = fv(s, f)
                if x: print(f"      {f}: {short(x, 200)}")
    orgs = {}
    for s in secs:
        if s['kind'] == 'Organization':
            orgs.setdefault(s.get('parent_section_id'), []).append(short(fv(s, 'TED-OFFICIALNAME'), 80))
    for s in secs:
        if s['kind'] == 'LotResult':
            sid = s['section_id']
            print(f"  BLOCK {sid}: LOT_NUMBER={fv(s,'TED-LOT_NUMBER')} CONTRACT_NUMBER={fv(s,'TED-CONTRACT_NUMBER')} date={fv(s,'TED-CONTRACT_AWARD_DATE')} offers={fv(s,'TED-OFFERS_RECEIVED_NUMBER')}")
            print(f"      title: {short(fv(s,'TED-CONTRACT_TITLE'), 400)}")
            est = fv(s, 'TED-INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT.VALUE_COST')
            print(f"      value={fv(s,'TED-VALUE_COST')} estimate={est} winners={orgs.get(sid, [])}")
print('\nSERVED BLOCK KEYS (no lot shown):', [(r['notice_id'], r['key'], r.get('decision')) for r in t['lot_results']])
print('WINNER PARTIES:', [(p['organization_name'], p['lot']) for p in t['parties'] if p['role'] == 'winner'])
