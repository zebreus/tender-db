"""Recall-gap lens, issue 481. Offline only: reads ../positives.json, ../negatives.json, ../unmerged.json
(and ../ted_notices_chk.json for TED dispatch times). Sends no requests."""
import json, re, unicodedata, difflib, os, collections, math
B = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
def load(n): return json.load(open(os.path.join(B, n)))

_NA = re.compile(r'[^0-9a-zà-ɏ]+')
def t_norm(s):
    s = unicodedata.normalize('NFKC', s).casefold()
    return ' '.join(_NA.sub(' ', s).split())
def ref_norm(s): return ''.join(unicodedata.normalize('NFKC', s).casefold().split())
def toks(s): return set(t_norm(s).split())
def jacc(a, b): return len(a & b) / len(a | b) if a and b else 0.0

def side(rec, layer):
    """layer: 'notice' (own values) or 'canonical'."""
    src = (rec.get(layer) if layer == 'notice' else rec.get('canonical')) or {}
    titles = [v for v in (src.get('title') or {}).values() if v]
    nuts = set()
    for v in (src.get('nuts') or {}).values(): nuts.update(v or [])
    dl = src.get('deadline') or {}
    ev = src.get('estimated_value') or {}
    ids = rec.get('ids') or {}
    return dict(
        titles=titles, title_langs=sorted((src.get('title') or {}).keys()),
        cpv=set(src.get('cpv_main') or []), cpv_lots=set(src.get('cpv_main_lots') or []), nuts=nuts,
        dl_min=dl.get('min'), dl_max=dl.get('max'), dl_n=dl.get('n'),
        val=ev.get('procedure_cents'), cur=ev.get('procedure_currency'), ev=ev,
        lots=src.get('lots'),
        iref={ref_norm(x) for x in (ids.get('internal_ref') or []) if x and x.strip()},
        iref_raw=[x for x in (ids.get('internal_ref') or []) if x],
        pid=set(ids.get('procedure_id') or []), prev=set(ids.get('previous_notice_ref') or []),
        orgs=set(rec.get('buyer_org_ids') or []),
        names={b['name_norm'] for b in (rec.get('buyers') or []) if b.get('name_norm')},
        idents={(b.get('identifier_kind'), b.get('identifier')) for b in (rec.get('buyers') or []) if b.get('identifier')},
        subtype=rec.get('subtype'), published_at=rec.get('published_at'), dispatched_at=rec.get('dispatched_at'),
        legal=set(rec.get('legal_basis') or []), notice_id=rec.get('notice_id'),
        publication_id=rec.get('publication_id'), layer=layer)

def tri(a, b):
    if a is None or b is None: return 'na'
    if isinstance(a, set):
        if not a or not b: return 'na'
        return 'eq' if a & b else 'ne'
    return 'eq' if a == b else 'ne'

def feats(a, b, dt_days):
    na = {t_norm(x) for x in a['titles']}; nb = {t_norm(x) for x in b['titles']}
    f = dict(
        org=bool(a['orgs'] & b['orgs']), name=bool(a['names'] & b['names']),
        ident=bool(a['idents'] & b['idents']),
        t_norm=bool(na & nb),
        t_jacc=max((jacc(set(x.split()), set(y.split())) for x in na for y in nb), default=0.0),
        t_ratio=max((difflib.SequenceMatcher(None, x, y).ratio() for x in na for y in nb), default=0.0),
        t_contain=any((x in y or y in x) and min(len(x), len(y)) >= 20 for x in na for y in nb),
        cpv=tri(a['cpv'], b['cpv']), cpv_div=tri({c[:2] for c in a['cpv']}, {c[:2] for c in b['cpv']}),
        cpv_any=tri(a['cpv'] | a['cpv_lots'], b['cpv'] | b['cpv_lots']),
        nuts=tri(a['nuts'], b['nuts']),
        dl=tri(a['dl_min'], b['dl_min']),
        dl_any=('na' if a['dl_min'] is None or b['dl_min'] is None else
                'eq' if {a['dl_min'], a['dl_max']} & {b['dl_min'], b['dl_max']} else 'ne'),
        val=('na' if a['val'] is None or b['val'] is None else 'eq' if a['val'] == b['val'] else 'ne'),
        iref=tri(a['iref'], b['iref']), lots=tri(a['lots'], b['lots']),
        pid=tri(a['pid'], b['pid']),
        sub=(a['subtype'] is not None and a['subtype'] == b['subtype']),
        dt=abs(dt_days) if dt_days is not None else None,
        disp=('na' if a['dispatched_at'] is None or b['dispatched_at'] is None else
              'eq' if a['dispatched_at'] == b['dispatched_at'] else 'ne'),
        legal=tri(a['legal'], b['legal']))
    f['contra'] = [k for k in ('cpv', 'dl', 'val', 'iref', 'lots') if f[k] == 'ne']
    return f

def wilson(k, n, z=1.959964):
    if n == 0: return (float('nan'), float('nan'))
    p = k / n; den = 1 + z * z / n; c = p + z * z / (2 * n)
    r = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - r) / den, (c + r) / den)

def build():
    P = load('positives.json')['pairs']; N = load('negatives.json')['pairs']; U = load('unmerged.json')['tenders']
    chk = {r['id']: r for r in load('ted_notices_chk.json')}
    pos = []
    for p in P:
        a = side(p['doe'], 'notice'); b = side(p['ted'], 'notice')
        f = feats(a, b, (p['ted']['published_at'] - p['doe']['published_at']) / 86400)
        f.update(kind='pos', tender_id=p['tender_id'], rec=p, A=a, Bs=b)
        pos.append(f)
    neg = []
    for n in N:
        a = side(n['doe'], 'notice' if n['doe'].get('notice') else 'canonical')
        tedrec = dict(n['ted']); c = chk.get(tedrec['notice_id'])
        if c: tedrec['dispatched_at'] = c['dispatched_at']
        b = side(tedrec, 'canonical')
        f = feats(a, b, n['delta_days'])
        f.update(kind='neg', pool=n['pool'], rec=n, A=a, Bs=b,
                 ted_has_doe=('doe' in (n['ted'].get('census_sources') or [])))
        neg.append(f)
    unm = []
    for t in U:
        a = side(t['doe'], 'canonical')
        cs = []
        for c in t['candidates']:
            cr = dict(c); ck = chk.get(c['notice_id'])
            if ck: cr['dispatched_at'] = ck['dispatched_at']
            if c.get('has_signals'):
                b = side(cr, 'canonical'); f = feats(a, b, c['delta_days'])
            else:
                b = None
                f = dict(org=c['via'] == 'org_id', name=True, sub=(c.get('subtype') == t['doe']['subtype']),
                         dt=abs(c['delta_days']), t_norm=None,
                         disp=('na' if not ck or t['doe'].get('dispatched_at') is None else
                               'eq' if ck['dispatched_at'] == t['doe']['dispatched_at'] else 'ne'))
            f.update(c=c, Bs=b, has_signals=bool(c.get('has_signals')),
                     ted_has_doe=('doe' in (c.get('census_sources') or [])))
            cs.append(f)
        unm.append(dict(t=t, A=a, cands=cs))
    return pos, neg, unm
