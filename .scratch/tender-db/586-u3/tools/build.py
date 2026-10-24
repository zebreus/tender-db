"""build.py -> minted.json, aliased.json (one object per pick, mechanical checks only, no verdicts)."""
import json, os, sys, collections, importlib.util
HERE = '/home/user/tender-db/.scratch/tender-db/586-u3'
spec = importlib.util.spec_from_file_location('classify586', os.path.join(HERE, 'tools', 'classify.py'))
cl = importlib.util.module_from_spec(spec); spec.loader.exec_module(cl)
from pop import org_name_norm  # 586-u1/pop.py (on sys.path via classify.py)

def load(p):
    try:
        return json.load(open(os.path.join(HERE, p)))
    except (FileNotFoundError, ValueError):
        return None

def val(v):
    return v.get('value', v.get('code', v.get('cents')))

def compact(sec):
    if not sec: return None
    return {'section_id': sec['section_id'], 'kind': sec['kind'], 'parent_section_id': sec['parent_section_id'],
            'values': [[v['field_id'], v.get('lang'), val(v)] for v in sec['values']]}

def names(sec, field):
    return [val(v) for v in (sec or {}).get('values', []) if v['field_id'] == field]

def mentions():
    out = collections.defaultdict(dict)
    for n in ('s_mentions_b', 's_mentions_c'):
        d = load(f'raw/sql/{n}.out')
        for r in d['rows']:
            r = dict(zip(d['columns'], r))
            out[r['notice_id']][r['section_id']] = r
    return out

MEN = mentions()
ISO3 = {'FRA': 'FR', 'ESP': 'ES', 'POL': 'PL', 'MKD': 'MK', 'NLD': 'NL', 'DEU': 'DE', 'ITA': 'IT', 'PRT': 'PT', 'GRC': 'GR', 'BEL': 'BE', 'AUT': 'AT'}

def window_groups(part):
    names = ['m1', 'm2', 'm3', 'm4', 'n6', 'o8'] if part == 'minted' else ['a0', 'a2', 'a3', 'a5', 'a6', 'x540072']
    g = collections.defaultdict(list)
    for n in names:
        d = load(f'raw/sql/{n}.out')
        for r in d['rows']:
            r = dict(zip(d['columns'], r))
            g[(r['tender_id'], r['seq'], r['role'], r['org_id'])].append(
                [r['notice_id'], r['section_id'], r.get('tpo', r['section_id']), r.get('lot_min', r.get('lot_id_min')), r['n_rows'], n])
    return g
WG = {'minted': window_groups('minted'), 'aliased': window_groups('aliased')}
ORDER = {'norm': 0, 'A': 1, 'C0': 2, 'C1': 2, 'C2': 2, 'C3': 2, 'C3r': 2, 'C4': 2, 'drop': 3, 'mint': 4}

def build(p, part):
    t = load(f"raw/tender/{p['tender_id']}.json") or {}
    o = load(f"raw/org/{p['org_id']}.json") or {}
    c = load(f"raw/content/{p['notice_id']}.json") or {}
    secs = {s['section_id']: s for s in c.get('sections', [])}
    tpo_id = p['tpo_section']
    tpo = secs.get(tpo_id)
    parent_id = tpo['parent_section_id'] if tpo else None
    parent = secs.get(parent_id)
    buyers = [val(v) for s in c.get('sections', []) for v in s['values'] if v['field_id'] == 'OPT-300-Procedure-Buyer']
    refs = [[s['section_id'], v['field_id'], v['ordinal']] for s in c.get('sections', []) for v in s['values']
            if v.get('is_ref') and val(v) == tpo_id]
    role_field = 'OPT-301-' + p['role']
    role_refs = {}
    for s in c.get('sections', []):
        for v in s['values']:
            if v['field_id'] == role_field:
                role_refs.setdefault(s['section_id'], []).append(val(v))
    lots_to_tpo = sorted(l for l, vs in role_refs.items() if tpo_id in vs)
    served = [x for x in t.get('parties', []) if x['role'] == p['role'] and x['organization_id'] == p['org_id']]
    served_lots = sorted({x['lot'] for x in served if x['lot']})
    tp_names = names(tpo, 'BT-500-Organization-TouchPoint')
    par_names = names(parent, 'BT-500-Organization-Company')
    pairs = [[a, b, cl.decision(a, b)] for a in tp_names for b in par_names]
    best = min((x[2] for x in pairs), key=lambda k: ORDER[k]) if pairs else None
    men = MEN.get(p['notice_id'], {})
    item = {
        'item': p['item'], 'stratum': p['stratum'], 'tender_id': p['tender_id'], 'seq': p['seq'], 'role': p['role'],
        'organization_id': p['org_id'], 'mention_notice_id': p['notice_id'], 'mention_section_id': p['section_id'],
        'touchpoint_section': tpo_id, 'n_party_rows': p['n_rows'],
        'pick': {'key': p['key'], 'rank': p['rank'], 'window': p['window']},
        'tender': {'version': t.get('version'), 'source': t.get('source'), 'publication_id': t.get('publication_id'),
                   'published_at': t.get('published_at'), 'notice_subtype': t.get('notice_subtype'), 'country': t.get('country'),
                   'title': (t.get('title') or '')[:200],
                   'versions': [[v.get('seq'), v.get('caused_by_notice_id'), v.get('publication_id')] for v in t.get('versions', [])],
                   'served_party': [[x['role'], x['lot'], x['organization_id'], x['mention_name'], x['organization_name']] for x in served],
                   'buyers_served': [[x['organization_id'], x['mention_name']] for x in t.get('parties', []) if x['role'] in ('Procedure-Buyer', 'buyer')],
                   'parties': [[x['role'], x['lot'], x['organization_id'], x['mention_name']] for x in t.get('parties', [])]},
        'organization': o,
        'notice': {'touchpoint': compact(tpo), 'parent_org': compact(parent),
                   'buyer_sections': [compact(secs.get(b)) for b in buyers],
                   'referencing_fields': refs, 'role_field': role_field, 'role_field_by_lot': role_refs,
                   'other_touchpoints': [[s['section_id'], s['parent_section_id'], names(s, 'BT-500-Organization-TouchPoint')]
                                         for s in c.get('sections', []) if s['kind'] == 'Touchpoint' and s['section_id'] != tpo_id]},
        'mentions': {'touchpoint_key': men.get(tpo_id), 'parent_key': men.get(parent_id),
                     'buyer_keys': {b: men.get(b) for b in buyers}},
        'oracle': {'pairs': pairs, 'best': best},
    }
    mn = {x['mention_name'] for x in served}
    chk = {
        'version_matches_seq': t.get('version') == p['seq'],
        'served_lots': served_lots, 'lots_whose_role_field_names_the_touchpoint': lots_to_tpo,
        'served_at_unreferenced_lot': sorted(set(served_lots) - set(lots_to_tpo)),
        'served_mention_names': sorted(mn),
        'parent_is_buyer': parent_id in buyers,
        'touchpoint_parent': parent_id,
    }
    if part == 'minted':
        chk['served_name_is_touchpoint_name'] = bool(mn) and mn <= set(tp_names)
        chk['touchpoint_mention_org'] = (men.get(tpo_id) or {}).get('organization_id')
        chk['touchpoint_mention_is_party_org'] = chk['touchpoint_mention_org'] == p['org_id']
        chk['touchpoint_bt514'] = names(tpo, 'BT-514-Organization-TouchPoint')
        chk['parent_bt514'] = names(parent, 'BT-514-Organization-Company')
        chk['oracle_says_mint'] = best == 'mint'
    else:
        chk['touchpoint_has_no_mention'] = tpo_id not in men
        chk['parent_mention_org'] = (men.get(parent_id) or {}).get('organization_id')
        chk['parent_mention_is_party_org'] = chk['parent_mention_org'] == p['org_id']
        chk['served_name_is_parent_name'] = bool(mn) and mn <= set(par_names)
        chk['pre586_norm_equal_any_pair'] = any(org_name_norm(a) == org_name_norm(b) for a in tp_names for b in par_names)
        chk['oracle_tier'] = best
        chk['buyer_party_org'] = [x['organization_id'] for x in t.get('parties', []) if x['role'] == 'Procedure-Buyer']
        chk['party_org_is_served_buyer'] = p['org_id'] in chk['buyer_party_org']
    grp = WG[part].get((p['tender_id'], p['seq'], p['role'], p['org_id']), [])
    chk['sql_rows_same_tender_role_org'] = grp
    chk['sql_party_rows_total'] = sum(x[4] for x in grp)
    chk['sql_rows_cover_served_lots'] = chk['sql_party_rows_total'] == len(served_lots)
    if part == 'minted' and not chk['touchpoint_bt514']:
        chk['org_country_equals_parent_bt514'] = o.get('country') in [ISO3.get(x, x) for x in chk['parent_bt514']]
    item['checks'] = chk
    return item

picks = json.load(open(os.path.join(HERE, 'picks.json')))
for part in ('minted', 'aliased'):
    items = [build(p, part) for p in picks[part]['picks']]
    meta = {k: v for k, v in picks[part].items() if k != 'picks'}
    json.dump({'part': part, 'meta': meta, 'items': items}, open(os.path.join(HERE, f'{part}.json'), 'w'), indent=1, ensure_ascii=False)
    print(part, len(items))
