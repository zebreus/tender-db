"""build.py -> 576-u4/sample.json from picks.json + raw GET bodies."""
import json, os, re, unicodedata
U4 = '/home/user/tender-db/.scratch/tender-db/576-u4'
R = os.path.join(U4, 'raw')
picks = json.load(open(os.path.join(U4, 'picks.json')))
WIN_SQL = {}
for name in ('s01_g0', 's02_g1', 's03_g2', 's04_g3', 's05_g4', 's06_g5', 's07_n6'):
    WIN_SQL[name] = open(os.path.join(R, 'sql', name + '.sql')).read().strip()

def vals(sec, prefix):
    return [v for v in sec['values'] if v['field_id'].startswith(prefix)]

def names(sec, field):
    return [v.get('value') for v in sec['values'] if v['field_id'] == field]

def code(sec, field):
    c = [v.get('code') for v in sec['values'] if v['field_id'] == field]
    return c[0] if c else None

def norm(s):
    s = unicodedata.normalize('NFKC', s or '').casefold()
    s = ''.join(ch for ch in unicodedata.normalize('NFKD', s) if not unicodedata.combining(ch))
    return re.sub(r'[\W_]+', ' ', s).strip()

M = json.load(open(os.path.join(R, 'sql', 's08_mentions.out')))
MENTIONS = {}
for r in M['rows']:
    row = dict(zip(M['columns'], r))
    MENTIONS.setdefault(row['notice_id'], []).append(row)
EXTRA = {}
def jload(f):
    return json.load(open(os.path.join(R, 'extra', f)))
EXTRA[15] = {'note': 'tender 306270 v10 serves this Lot-ReviewOrg on LOT-0001..LOT-0009; each lot is anchored at its own per-lot notice. The 8 sibling notices were fetched (raw/extra/content_*.json): each has one Lot section and its OPT-301-Lot-ReviewOrg points at its own TPO-0001 with the same BT-500.',
             'sibling_notices': []}
import glob
for f in sorted(glob.glob(os.path.join(R, 'extra', 'content_*.json'))):
    d = json.load(open(f))
    EXTRA[15]['sibling_notices'].append({
        'notice_id': d['notice_id'],
        'lots': [x['section_id'] for x in d['sections'] if x['kind'] == 'Lot'],
        'refs_to_TPO-0001': [[x['section_id'], v['field_id']] for x in d['sections'] for v in x['values'] if v.get('is_ref') and v.get('value') == 'TPO-0001'],
        'TPO-0001_BT-500': [v.get('value') for x in d['sections'] if x['section_id'] == 'TPO-0001' for v in x['values'] if v['field_id'] == 'BT-500-Organization-TouchPoint']})
EXTRA[21] = {'note': "The notice's buyer section ORG-0007 'Stadt Erlangen - EBE' resolves to org 22208093 (identifier 09562EBE87); the touchpoint TPO-0001 under ORG-0001 (Zentrale Vergabestelle) prints the same name and was minted as identifier-less org 31976959. The served Procedure-Buyer is org 1062 (ORG-0001's name).",
             'org_22208093': jload('org_22208093.json')}
EXTRA[24] = {'note': "Not a touchpoint matter: the served Procedure-Buyer (mention 'Uniwersytet Warszawski', NIP 5262239325) is organization 49, whose name is 'Krajowa Izba Odwoławcza' and whose identifier is that NIP.",
             'org_49': jload('org_49.json')}
out = []
for p in picks['picks']:
    t = json.load(open(os.path.join(R, 'tender', f"{p['tender_id']}.json")))
    o = json.load(open(os.path.join(R, 'org', f"{p['org_id']}.json")))
    c = json.load(open(os.path.join(R, 'content', f"{p['notice_id']}.json")))
    secs = {s['section_id']: s for s in c['sections']}
    tp = secs.get(p['section_id'])
    parent = secs.get(tp['parent_section_id']) if tp else None
    # buyers: ORG sections referenced by OPT-300-Procedure-Buyer
    buyer_ids = [v['value'] for s in c['sections'] for v in s['values'] if v['field_id'] == 'OPT-300-Procedure-Buyer']
    buyer_secs = [secs[b] for b in dict.fromkeys(buyer_ids) if b in secs]
    # every reference to this touchpoint anywhere in the notice
    refs = [{'section_id': s['section_id'], 'section_kind': s['kind'], 'field_id': v['field_id'], 'ordinal': v.get('ordinal'), 'scheme': v.get('scheme')}
            for s in c['sections'] for v in s['values'] if v.get('is_ref') and v.get('value') == p['section_id']]
    # lots of this notice whose same-role field points at this TPO
    role_field = 'OPT-301-' + p['role']
    lots_ref_same_role = sorted({r['section_id'] for r in refs if r['field_id'] == role_field})
    # other touchpoints of the notice (compact)
    others = []
    for s in c['sections']:
        if s['kind'] == 'Touchpoint' and s['section_id'] != p['section_id']:
            others.append({'section_id': s['section_id'], 'parent_section_id': s['parent_section_id'],
                           'BT-500': names(s, 'BT-500-Organization-TouchPoint'), 'BT-514': code(s, 'BT-514-Organization-TouchPoint'),
                           'referenced_by': sorted({f"{v['field_id']}@{s2['section_id']}" for s2 in c['sections'] for v in s2['values']
                                                   if v.get('is_ref') and v.get('value') == s['section_id']})})
    lot_by_id = {l['id']: l['lot_key'] for l in t.get('lot_details', [])}
    lot_key = lot_by_id.get(p['lot_id_min'])
    served = [x for x in t.get('parties', []) if x['role'] == p['role'] and x['organization_id'] == p['org_id']]
    served_lots = sorted({x['lot'] for x in served if x['lot']})
    buyers_served = [x for x in t.get('parties', []) if x['role'] == 'Procedure-Buyer']
    tp_names = names(tp, 'BT-500-Organization-TouchPoint') if tp else []
    parent_names = names(parent, 'BT-500-Organization-Company') if parent else []
    checks = {
        'current_version_ok': t.get('version') == p['seq'],
        'served_at_window_lot': lot_key in served_lots,
        'window_lot_key': lot_key,
        'served_lots_for_role_org': served_lots,
        'notice_lots_referencing_tpo_in_this_role': lots_ref_same_role,
        'served_lots_not_referenced_by_this_notice': sorted(set(served_lots) - set(lots_ref_same_role)),
        'notice_lots_not_served': sorted(set(lots_ref_same_role) - set(served_lots)),
        'n_party_rows_in_group (this notice)': p['n_rows'],
        'tp_bt514': code(tp, 'BT-514-Organization-TouchPoint') if tp else None,
        'parent_bt514': code(parent, 'BT-514-Organization-Company') if parent else None,
        'org_country': o.get('country'),
        'country_fallback_used': (tp is not None and code(tp, 'BT-514-Organization-TouchPoint') is None),
        'tp_name_equals_parent_name (casefold, accents and punctuation folded)': bool(set(map(norm, tp_names)) & set(map(norm, parent_names))),
        'parent_is_buyer': bool(parent and parent['section_id'] in buyer_ids),
        'buyer_bt514': [code(b, 'BT-514-Organization-Company') for b in buyer_secs],
        'served_mention_names': sorted({x['mention_name'] for x in served}),
        'served_mention_name_equals_tp_name': sorted({x['mention_name'] for x in served}) == sorted({' '.join((n or '').split()) for n in tp_names}) or bool({x['mention_name'] for x in served} & set(tp_names)),
        'tp_name_equals_buyer_name (casefold, accents and punctuation folded)': bool(set(map(norm, tp_names)) & {norm(n) for b in buyer_secs for n in names(b, 'BT-500-Organization-Company')}),
        'tp_mention_org_id (s08)': [m['organization_id'] for m in MENTIONS.get(p['notice_id'], []) if m['section_id'] == p['section_id']],
        'buyer_mention_org_ids (s08)': [m['organization_id'] for m in MENTIONS.get(p['notice_id'], []) if m['section_id'] in buyer_ids],
    }
    out.append({
        'item': p['item'], 'stratum': p['stratum'],
        'tender_id': p['tender_id'], 'seq': p['seq'], 'lot': lot_key, 'lot_id': p['lot_id_min'], 'role': p['role'],
        'organization_id': p['org_id'], 'mention_notice_id': p['notice_id'], 'mention_section_id': p['section_id'],
        'served_party': served,
        'served_buyers': buyers_served,
        'tender_meta': {k: t.get(k) for k in ('source', 'publication_id', 'published_at', 'notice_subtype', 'version', 'lots', 'title', 'country')},
        'organization': o,
        'touchpoint_section': tp,
        'parent_org_section': parent,
        'buyer_section': buyer_secs[0] if buyer_secs else None,
        'other_buyer_sections': buyer_secs[1:],
        'referencing_fields': refs,
        'other_touchpoints_in_notice': others,
        'checks': checks,
        'notice_mentions (s08: organization_mentions of mention_notice_id)': MENTIONS.get(p['notice_id'], []),
        'extra_context': EXTRA.get(p['item']),
        'how_chosen': {
            'window': p['window'], 'statement_sql': WIN_SQL[p['window']['statement']],
            'row': {k: p[k] for k in ('tender_id', 'seq', 'role', 'org_id', 'notice_id', 'section_id', 'lot_id_min', 'n_rows', 'org_country', 'org_name', 'tp_bt514')},
            'pick_rank_sha256': p['rank'], 'pick_key': p['key'], 'stratum': p['stratum'],
            'rule': 'pick.py: rank = sha256("576-u4-pick-" + key) ascending; strata filled in order NL AddInfo/TenderReceipt (6), no own BT-514 (3), Lot-ReviewOrg (6), rest round-robin by role preferring least-used country; caps <=2 per tender, <=1 per organization, <=1 per touchpoint section',
        },
    })
json.dump(out, open(os.path.join(U4, 'sample.json'), 'w'), indent=1, ensure_ascii=False)
for x in out:
    ch = x['checks']
    print(x['item'], x['stratum'][:8], x['tender_id'], x['role'], x['lot'], 'cur', ch['current_version_ok'], 'servedAtLot', ch['served_at_window_lot'],
          'served', ch['served_lots_for_role_org'][:6], 'ref', ch['notice_lots_referencing_tpo_in_this_role'][:6],
          'tp514', ch['tp_bt514'], 'par514', ch['parent_bt514'], 'org', ch['org_country'], 'same', ch['tp_name_equals_parent_name (casefold, accents and punctuation folded)'],
          'parentBuyer', ch['parent_is_buyer'], 'buyer514', ch['buyer_bt514'])
