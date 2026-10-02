"""Issue 481 calibration: pair features for positives / negatives / unmerged candidates.

Offline only: reads ../positives.json, ../negatives.json, ../unmerged.json.
"""
import json, math, re, unicodedata, difflib, collections, os

BASE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')


def load(name):
    with open(os.path.join(BASE, name)) as f:
        return json.load(f)


# ---------------- normalisers ----------------
def t_ci(s):
    """exact, case-insensitive, whitespace-collapsed."""
    return ' '.join(s.casefold().split())


_NONALNUM = re.compile(r'[^0-9a-zà-ɏ]+')


def t_norm(s):
    """NFKC + casefold + every non-alphanumeric run -> one space."""
    s = unicodedata.normalize('NFKC', s).casefold()
    return ' '.join(_NONALNUM.sub(' ', s).split())


def toks(s):
    return set(t_norm(s).split())


def ref_norm(s):
    """internal reference: NFKC, casefold, drop whitespace."""
    return ''.join(unicodedata.normalize('NFKC', s).casefold().split())


def jacc(a, b):
    if not a or not b:
        return 0.0
    return len(a & b) / len(a | b)


# ---------------- side extraction ----------------
def side(rec, prefer_notice):
    src = rec.get('notice') if (prefer_notice and rec.get('notice')) else rec.get('canonical') or {}
    titles = [v for v in (src.get('title') or {}).values() if v]
    nuts = set()
    for k, v in (src.get('nuts') or {}).items():
        nuts.update(v or [])
    dl = src.get('deadline') or {}
    ev = src.get('estimated_value') or {}
    ids = rec.get('ids') or {}
    return {
        'titles': titles,
        'cpv': set(src.get('cpv_main') or []),
        'nuts': nuts,
        'dl_min': dl.get('min'),
        'dl_max': dl.get('max'),
        'val': ev.get('procedure_cents'),
        'cur': ev.get('procedure_currency'),
        'lots': src.get('lots'),
        'iref': {ref_norm(x) for x in (ids.get('internal_ref') or []) if x and x.strip()},
        'orgs': set(rec.get('buyer_org_ids') or []),
        'names': {b['name_norm'] for b in (rec.get('buyers') or []) if b.get('name_norm')},
        'subtype': rec.get('subtype'),
        'published_at': rec.get('published_at'),
        'src_kind': 'notice' if (prefer_notice and rec.get('notice')) else 'canonical',
    }


def tri(a, b):
    """'na' when either side missing, else 'eq' / 'ne'."""
    if a is None or b is None:
        return 'na'
    if isinstance(a, set):
        if not a or not b:
            return 'na'
        return 'eq' if a & b else 'ne'
    return 'eq' if a == b else 'ne'


def features(a, b, delta_days):
    ta = a['titles']; tb = b['titles']
    t_exact = bool({t_ci(x) for x in ta} & {t_ci(x) for x in tb})
    t_n = bool({t_norm(x) for x in ta} & {t_norm(x) for x in tb})
    tj = max((jacc(toks(x), toks(y)) for x in ta for y in tb), default=0.0)
    tr = max((difflib.SequenceMatcher(None, t_norm(x), t_norm(y)).ratio() for x in ta for y in tb), default=0.0)
    cdiv_a = {c[:2] for c in a['cpv']}; cdiv_b = {c[:2] for c in b['cpv']}
    dl = tri(a['dl_min'], b['dl_min'])
    if a['dl_min'] is not None and b['dl_min'] is not None:
        dl1 = 'eq' if abs(a['dl_min'] - b['dl_min']) <= 86400 else 'ne'
    else:
        dl1 = 'na'
    val = 'na'
    if a['val'] is not None and b['val'] is not None:
        val = 'eq' if a['val'] == b['val'] else 'ne'
    if a['val'] and b['val']:
        val1 = 'eq' if abs(a['val'] - b['val']) <= 0.01 * max(a['val'], b['val']) else 'ne'
    else:
        val1 = 'na'
    lots = tri(a['lots'], b['lots'])
    f = {
        'org': bool(a['orgs'] & b['orgs']),
        'name': bool(a['names'] & b['names']),
        't_exact': t_exact,
        't_norm': t_n,
        't_jacc': tj,
        't_ratio': tr,
        'cpv': tri(a['cpv'], b['cpv']),
        'cpv_div': tri(cdiv_a, cdiv_b),
        'nuts': tri(a['nuts'], b['nuts']),
        'dl': dl,
        'dl1d': dl1,
        'val': val,
        'val1p': val1,
        'iref': tri(a['iref'], b['iref']),
        'lots': lots,
        'sub': (a['subtype'] is not None and a['subtype'] == b['subtype']),
        'dt': abs(delta_days) if delta_days is not None else None,
    }
    # contradiction: a field present on both sides that disagrees
    f['contra'] = [k for k in ('cpv', 'dl', 'val', 'iref', 'lots') if f[k] == 'ne']
    f['contra_soft'] = [k for k in ('cpv', 'dl1d', 'val1p', 'iref', 'lots') if f[k] == 'ne']
    return f


# ---------------- build the three populations ----------------
def build():
    P = load('positives.json')['pairs']
    N = load('negatives.json')['pairs']
    U = load('unmerged.json')['tenders']

    pos = []
    for p in P:
        a = side(p['doe'], True); b = side(p['ted'], True)
        dd = (p['ted']['published_at'] - p['doe']['published_at']) / 86400
        f = features(a, b, dd)
        st = p['doe']['subtype']
        nt = sum(1 for v in p['census_versions'] if v['source'] == 'ted' and v['subtype'] == st)
        nd = sum(1 for v in p['census_versions'] if v['source'] == 'doe' and v['subtype'] == st)
        f.update(kind='pos', tender_id=p['tender_id'], doe_notice=p['doe']['notice_id'],
                 ted_tender=p['tender_id'], profile=p['doe_profile'], subtype=st,
                 clean_pairing=(nt == 1 and nd == 1), pool='pos')
        pos.append(f)

    neg = []
    for n in N:
        a = side(n['doe'], True)            # notice-layer own values when present (twin_known)
        b = side(n['ted'], False)           # TED side: canonical only
        f = features(a, b, n['delta_days'])
        f.update(kind='neg', pool=n['pool'], doe_notice=n['doe']['notice_id'],
                 doe_tender=n['doe_tender_id'], ted_tender=n['ted_tender_id'],
                 ted_has_doe=('doe' in (n['ted'].get('census_sources') or [])),
                 a_src=a['src_kind'], subtype=n['doe']['subtype'], ted_subtype=n['ted']['subtype'])
        neg.append(f)

    unm = []
    for t in U:
        a = side(t['doe'], False)
        recs = []
        for c in t['candidates']:
            if not c.get('has_signals'):
                continue
            b = side(c, False)
            b['subtype'] = c.get('subtype')
            f = features(a, b, c['delta_days'])
            f.update(cand_tender=c['tender_id'], via=c['via'],
                     ted_has_doe=('doe' in (c.get('census_sources') or [])),
                     cand_subtype=c.get('subtype'), cand_island=(c.get('tender') or {}).get('procedure_key') is None)
            recs.append(f)
        unm.append({
            'tender_id': t['tender_id'], 'profile': t['doe_profile'], 'eu': t['eu_legal_basis'],
            'subtype': t['doe']['subtype'], 'n_cand': t['n_candidates'],
            'n_cand_sig': t['n_candidates_with_signals'], 'n_cand_org': t['n_candidates_org_id'],
            'cands': recs, 'stratum': t['sample_stratum'],
        })
    return pos, neg, unm


def wilson(k, n, z=1.959964):
    if n == 0:
        return (float('nan'), float('nan'))
    p = k / n
    den = 1 + z * z / n
    c = p + z * z / (2 * n)
    r = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - r) / den, (c + r) / den)
