# Assemble positives.json, negatives.json, unmerged.json from the cached reads. No requests.
import sys, json, collections, re, datetime as dt; sys.path.insert(0,'.')
from census_lib import *
DAY = 86400
def iso(e): return dt.datetime.utcfromtimestamp(e).strftime('%Y-%m-%dT%H:%M:%SZ') if e is not None else None
def cpv_norm(code):
    out = []
    for tok in re.split(r'[\s,;]+', code or ''):
        if not tok: continue
        stem = tok
        if '-' in tok:
            s, c = tok.split('-', 1)
            if len(s) == 8 and s.isdigit() and len(c) == 1 and c.isdigit(): stem = s
        if 2 <= len(stem) <= 7 and stem.isdigit(): stem = stem.ljust(8, '0')
        if stem not in out: out.append(stem)
    return out
S = json.load(open('sample.json')); info = S['info']
ten = {t['id']: t for t in json.load(open('tenders_sample.json'))}
head = {t['id']: t for t in json.load(open('cand_head.json'))}
buy = json.load(open('buyers_sample.json'))
O = {int(k): v for k, v in json.load(open('orgs.json')).items()}
for s in json.load(open('orgs_siblings.json')): O.setdefault(s['id'], s)
try:
    for k, v in json.load(open('orgs2.json')).items(): O.setdefault(int(k), v)
except FileNotFoundError: pass
N = {int(k): v for k, v in json.load(open('notices_doe.json')).items()}
TN = {}
try:
    for r in json.load(open('ted_notices_chk.json')): TN[r['id']] = r
except FileNotFoundError: pass
LEG = collections.defaultdict(list)
for nid, f, c in json.load(open('legal_doe.json')): LEG[nid].append(c)
R = json.load(open('cands_sel.json'))
T = load()
SAT = json.load(open('sats.json'))
import os
NL = json.load(open('nl.json')) if os.path.exists('nl.json') else {}
E = {'rows': []}
CB = collections.defaultdict(list)
try:
    cbj = json.load(open('cand_buyers.json'))
    for row in cbj['rows']: CB[(row[0], row[1])].append(row)
except FileNotFoundError: cbj = None
# ---------- canonical satellites per (tender, seq)
sat = collections.defaultdict(dict)
for t, s, lang, val in SAT['title']['rows']: sat[(t, s)].setdefault('title', {})[lang] = val
for t, s, code in SAT['cpv']['rows']: sat[(t, s)].setdefault('cpv_main', []).append(code)
for t, s, lot, code in SAT.get('nuts',{'rows':[]})['rows']:
    d = sat[(t, s)].setdefault('nuts', {'procedure': [], 'lots': []})
    (d['procedure'] if lot is None else d['lots']).append(code)
for t, s, n, mn, mx, npr in SAT['deadline']['rows']:
    if n: sat[(t, s)]['deadline'] = dict(n=n, min=mn, max=mx, n_procedure_level=npr, min_iso=iso(mn), max_iso=iso(mx))
for t, s, n in SAT['lots']['rows']: sat[(t, s)]['lots'] = n
am = collections.defaultdict(list)
for t, s, lot, cents, cur, eur, q in SAT['amount']['rows']: am[(t, s)].append((lot, cents, cur, eur, q))
for k, rows in am.items():
    pr = [r for r in rows if r[0] is None]; lr = [r for r in rows if r[0] is not None]
    sat[k]['estimated_value'] = dict(
        procedure_cents=pr[0][1] if pr else None, procedure_currency=pr[0][2] if pr else None, procedure_eur_cents=pr[0][3] if pr else None,
        lots_cents_sum=sum(r[1] for r in lr if r[1] is not None) if lr else None, lots_eur_cents_sum=sum(r[3] for r in lr if r[3] is not None) if lr else None,
        n_lot_rows=len(lr), currencies=sorted({r[2] for r in rows if r[2]}), withheld_rows=sum(1 for r in rows if r[4]))
for k in sat:
    if 'nuts' in sat[k]:
        sat[k]['nuts'] = {kk: sorted(set(v)) for kk, v in sat[k]['nuts'].items()}
    if 'cpv_main' in sat[k]: sat[k]['cpv_main'] = sorted(set(sat[k]['cpv_main']))
# ---------- notice-layer own values
nlv = collections.defaultdict(dict)
TITLE_F = {'BT-21-Procedure', 'DE1-ProcurementProject-Name', 'SDK01-ProcurementProject-Name'}
for n, f, lang, val in NL.get('title',E)['rows']:
    if f in TITLE_F: nlv[n].setdefault('title', {})[lang or '?'] = val
for n, f, code in NL.get('class',E)['rows']:
    d = nlv[n]
    if f in ('BT-262-Procedure',) or (('MainCommodity' in f) and 'ProcurementProjectLot' not in f):
        d.setdefault('cpv_main', []).extend(cpv_norm(code))
    elif f.startswith('BT-262') or ('MainCommodity' in f):
        d.setdefault('cpv_main_lots', []).extend(cpv_norm(code))
    elif f.startswith('BT-5071') or f.endswith('RealizedLocation-Address-CountrySubentityCode'):
        lvl = 'lots' if (f.endswith('-Lot') or 'ProcurementProjectLot' in f or f.endswith('-Part')) else 'procedure'
        d.setdefault('nuts', {'procedure': [], 'lots': []})[lvl].append(code)
dl = collections.defaultdict(list)
for n, f, u, ht in NL.get('dates',E)['rows']: dl[n].append(u)
for n, v in dl.items(): nlv[n]['deadline'] = dict(n=len(v), min=min(v), max=max(v), min_iso=iso(min(v)), max_iso=iso(max(v)))
amn = collections.defaultdict(list)
for n, f, cents, cur in NL.get('amounts',E)['rows']: amn[n].append((f, cents, cur))
for n, rows in amn.items():
    pr = [r for r in rows if r[0] in ('BT-27-Procedure',) or (('Estimated' in r[0] or 'TotalAmount' in r[0]) and 'ProcurementProjectLot' not in r[0])]
    lr = [r for r in rows if r not in pr]
    nlv[n]['estimated_value'] = dict(procedure_cents=pr[0][1] if pr else None, procedure_currency=pr[0][2] if pr else None,
                                     lots_cents_sum=sum(r[1] for r in lr) if lr else None, n_lot_rows=len(lr), currencies=sorted({r[2] for r in rows}))
for n, c in NL.get('lots',E)['rows']: nlv[n]['lots'] = c
for n, f, code in NL.get('legal', {'rows': []})['rows']: nlv[n].setdefault('legal_basis', []).append(code)
for n, f, val in NL.get('de1legal', {'rows': []})['rows']: nlv[n].setdefault('legal_basis', []).append(val)
for n in nlv:
    d = nlv[n]
    for k in ('cpv_main', 'cpv_main_lots', 'legal_basis'):
        if k in d: d[k] = sorted(set(d[k]))
    if 'nuts' in d: d['nuts'] = {kk: sorted(set(v)) for kk, v in d['nuts'].items()}
IDS = collections.defaultdict(lambda: collections.defaultdict(list))
for n, f, val in NL.get('ids',E)['rows']:
    key = {'BT-22-Procedure': 'internal_ref', 'DE1-ProcurementProject-ID': 'internal_ref', 'SDK01-ProcurementProject-ID': 'internal_ref',
           'BT-04-notice': 'procedure_id', 'DE1-ContractFolderID': 'procedure_id', 'SDK01-ContractFolderID': 'procedure_id',
           'OPP-090-Procedure': 'previous_notice_ref'}[f]
    if val not in IDS[n][key]: IDS[n][key].append(val)
# ---------- DE1 (eForms-DE 1.x): lot leaves share the procedure field ids; scope them by section_id
try:
    DE1 = json.load(open('de1_sections.json'))['rows']
except FileNotFoundError:
    DE1 = []
de1n = collections.defaultdict(lambda: collections.defaultdict(list))
for t_, n, sec, f, v in DE1: de1n[n][t_].append((sec, f, v))
for n, kinds in de1n.items():
    if 'class' in kinds:
        d = nlv[n]
        d['cpv_main'] = sorted({c for sec, f, v in kinds['class'] if 'MainCommodity' in f and sec == 'PROCEDURE' for c in cpv_norm(v)})
        d['cpv_main_lots'] = sorted({c for sec, f, v in kinds['class'] if 'MainCommodity' in f and sec != 'PROCEDURE' for c in cpv_norm(v)})
        d['nuts'] = dict(procedure=[], lots=[], unscoped=sorted({v for sec, f, v in kinds['class'] if 'RealizedLocation' in f}))
        d['scope_note'] = 'DE1: procedure/lot separated by section_id; NUTS sit in ND-RealizedLocation sections (scope not separable)'
    if 'title' in kinds:
        pr = [v for sec, f, v in kinds['title'] if sec == 'PROCEDURE']
        if pr:
            nlv[n]['title'] = {('PROCEDURE#%d' % i): v for i, v in enumerate(pr)}
    if 'ids' in kinds:
        IDS[n]['internal_ref'] = sorted({v for sec, f, v in kinds['ids'] if f == 'DE1-ProcurementProject-ID' and sec == 'PROCEDURE'})
        IDS[n]['procedure_id'] = sorted({v for sec, f, v in kinds['ids'] if f == 'DE1-ContractFolderID'})
        if not IDS[n]['internal_ref']: del IDS[n]['internal_ref']
        if not IDS[n]['procedure_id']: del IDS[n]['procedure_id']
# ---------- helpers
def org_view(o):
    r = O.get(o)
    if not r: return dict(organization_id=o)
    return dict(organization_id=o, name=r.get('name'), name_norm=r.get('name_norm'), country=r.get('country'),
                identifier_kind=r.get('identifier_kind'), identifier=r.get('identifier'), provisional=r.get('provisional'))
def buyers_of(t, nid, seq=None):
    rows = [b for b in buy if b['tender_id'] == t and b['mention_notice_id'] == nid]
    if not rows and seq is not None and (t, seq) in CB:
        rows = [dict(zip(['tender_id','seq','lot_id','role','organization_id','mention_notice_id','mention_section_id'], r)) for r in CB[(t, seq)]]
        own = [b for b in rows if b['mention_notice_id'] == nid]
        rows = own or rows
    orgs = sorted({b['organization_id'] for b in rows})
    roles = sorted({b['role'] for b in rows})
    return dict(buyer_org_ids=orgs, buyer_roles=roles, buyers=[org_view(o) for o in orgs],
                mentions=sorted({(b['mention_notice_id'], b['mention_section_id']) for b in rows}))
BUY_BY_TS = collections.defaultdict(list)
for b in buy: BUY_BY_TS[(b['tender_id'], b['seq'])].append(b)
def first_seq(t): return min(T[t]) if T.get(t) else None
def version(t, s):
    v = T[t].get(s, {})
    return dict(seq=s, notice_id=v.get('notice_id'), publication_id=v.get('publication_id'), published_at=v.get('published_at'),
                published_iso=iso(v.get('published_at')), subtype=v.get('subtype'))
def src_of(v):
    return 'ted' if v['shape'] == 'ted8' else 'doe'
def side(t, s, nid, src, prof=None, want_nl=False):
    v = version(t, s)
    d = dict(v)
    d['source'] = src
    if src == 'doe':
        n = N.get(nid, {}); d['profile'] = (prof or n.get('profile')); d['dispatched_at'] = n.get('dispatched_at')
        d['legal_basis'] = sorted(set(LEG.get(nid, []) + nlv.get(nid, {}).get('legal_basis', [])))
    else:
        n = TN.get(nid)
        if n: d['profile'] = n['profile']; d['notices_source_checked'] = n['source']; d['dispatched_at'] = n['dispatched_at']
        if nid in nlv and 'legal_basis' in nlv[nid]: d['legal_basis'] = nlv[nid]['legal_basis']
    d['ids'] = {k: v for k, v in IDS.get(nid, {}).items()}
    d.update(buyers_of(t, nid, s))
    c = dict(sat.get((t, s), {}))
    c['carried_state_possible'] = s > 1
    d['canonical'] = c
    if want_nl:
        d['notice'] = nlv.get(nid, {})
    return d
def div(cpvs): return sorted({c[:2] for c in (cpvs or []) if c})
def tnorm(s): return re.sub(r'\s+', ' ', (s or '')).strip().casefold()
def titles(sig): return {tnorm(v) for v in (sig.get('title') or {}).values() if v}
def eq_titles(a, b): return bool(titles(a) & titles(b))
# ---------------- positives
positives = []
for r in R:
    if r['kind'] != 'pos': continue
    t = r['tender_id']; tr = ten[t]
    ds, ts = r['doe_seq'], r['ted_seq']
    dn = T[t][ds]['notice_id']; tn = T[t][ts]['notice_id']
    vers = [dict(version(t, s), source=src_of(T[t][s])) for s in sorted(T[t])]
    positives.append(dict(
        tender_id=t, procedure_key=tr['procedure_key'], island_notice_id=tr['island_notice_id'], tenders_source=tr['source'], kind=tr['kind'],
        current_seq=tr['current_seq'], doe_profile=info[str(t)]['profile'],
        census_versions=vers,
        pairing=dict(rule='DOE = first core-month (2025-04) DOE version; TED = TED version of the same subtype nearest in publication time (any subtype if none)',
                     doe_seq=ds, ted_seq=ts, delta_days=round((T[t][ts]['published_at'] - T[t][ds]['published_at']) / DAY, 2),
                     same_subtype=T[t][ts]['subtype'] == T[t][ds]['subtype'], twin_found_via_buyer_org_within_30d=any(x['is_twin'] for x in r['cands']),
                     competitors_same_buyer_within_30d=sum(1 for x in r['cands'] if not x['is_twin'] and x['via'] == 'org_id')),
        doe=side(t, ds, dn, 'doe', info[str(t)]['profile'], want_nl=True),
        ted=side(t, ts, tn, 'ted', want_nl=True)))
# ---------------- unmerged
unmerged = []; dropped_unm = sorted(t for t in S['unm'] if ten[t]['source'] != 'doe')
cand_by_rec = {}
for r in R:
    if r['kind'] != 'unm': continue
    t = r['tender_id']; tr = ten[t]
    if tr['source'] != 'doe': continue
    prof = info[str(t)]['profile']
    ds = r['doe_seq']; dn = r['doe_notice']
    dside = side(t, ds, dn, 'doe', prof)
    lb = dside['legal_basis']
    sel = set(r['sel'])
    cands = []
    for x in sorted(r['cands'], key=lambda x: (x['via'] != 'org_id', abs(x['published_at'] - r['doe_pub']))):
        ct = x['tender_id']; h = head.get(ct)
        c = dict(tender_id=ct, seq=x['seq'], via=x['via'], shared_org_ids=x['orgs'], notice_id=x['notice_id'], publication_id=x['publication_id'],
                 published_iso=iso(x['published_at']), subtype=x['subtype'], delta_days=round((x['published_at'] - r['doe_pub']) / DAY, 2),
                 n_ted_versions_in_window=x['n_ted_versions_in_window'], has_signals=ct in sel,
                 census_sources=sorted({src_of(v) for v in T.get(ct, {}).values()}))
        if ct in sel:
            if h: c['tender'] = {k: h[k] for k in ('source', 'procedure_key', 'island_notice_id', 'kind', 'current_seq', 'current_published_at')}
            c['canonical'] = sat.get((ct, x['seq']), {})
            c['ids'] = {k: v for k, v in IDS.get(x['notice_id'], {}).items()}
            bb = CB.get((ct, x['seq']), [])
            own = [b for b in bb if b[5] == x['notice_id']] or bb
            c['buyer_org_ids'] = sorted({b[4] for b in own}); c['buyers'] = [org_view(o) for o in c['buyer_org_ids']]
            c['title_equal_ci'] = eq_titles(dside['canonical'], c['canonical'])
        cands.append(c)
    unmerged.append(dict(
        tender_id=t, procedure_key=tr['procedure_key'], island_notice_id=tr['island_notice_id'], tenders_source=tr['source'], kind=tr['kind'],
        doe_profile=prof, legal_basis=lb, eu_legal_basis=any(x.startswith('3') for x in lb),
        sample_stratum=('all DOE-only above-threshold / eForms-family in 2025-04' if (any(x.startswith('3') for x in lb) or not prof.startswith('eforms-sdk-0.1')) else 'random national sdk-0.1 sample'),
        doe=dside, name_norm_sibling_orgs=r['sibling_orgs'],
        n_candidates=len(cands), n_candidates_org_id=sum(1 for c in cands if c['via'] == 'org_id'), n_candidates_name_norm=sum(1 for c in cands if c['via'] == 'name_norm'),
        n_candidates_with_signals=sum(1 for c in cands if c['has_signals']), candidates=cands))
    cand_by_rec[t] = cands
# ---------------- negatives
negatives = []
pos_by_t = {p['tender_id']: p for p in positives}
for r in R:
    if r['kind'] != 'pos': continue
    p = pos_by_t[r['tender_id']]
    for x in r['cands']:
        if x['is_twin'] or x['tender_id'] not in set(r['sel']) or x['via'] != 'org_id': continue
        ct = x['tender_id']
        tside = dict(tender_id=ct, seq=x['seq'], notice_id=x['notice_id'], publication_id=x['publication_id'], published_iso=iso(x['published_at']),
                     published_at=x['published_at'], subtype=x['subtype'], source='ted',
                     tender={k: head[ct][k] for k in ('source', 'procedure_key', 'island_notice_id', 'kind', 'current_seq')} if ct in head else None,
                     canonical=sat.get((ct, x['seq']), {}), ids={k: v for k, v in IDS.get(x['notice_id'], {}).items()})
        bb = CB.get((ct, x['seq']), []); own = [b for b in bb if b[5] == x['notice_id']] or bb
        tside['buyer_org_ids'] = sorted({b[4] for b in own}); tside['buyers'] = [org_view(o) for o in tside['buyer_org_ids']]
        tside['census_sources'] = sorted({src_of(v) for v in T.get(ct, {}).values()})
        dsd = p['doe']
        dcan = dsd['canonical']
        negatives.append(dict(pool='twin_known', label='negative',
            label_basis='DOE notice belongs to a UUID-merged Tender whose own TED twin is known; the TED side is a different Tender of the same buyer org',
            doe_tender_id=p['tender_id'], doe_procedure_key=p['procedure_key'], ted_tender_id=ct,
            shared_org_ids=x['orgs'], delta_days=round((x['published_at'] - dsd['published_at']) / DAY, 2),
            same_subtype=x['subtype'] == dsd['subtype'],
            same_cpv_division=bool(set(div(dcan.get('cpv_main'))) & set(div(tside['canonical'].get('cpv_main')))),
            internal_ref_equal=bool(set(dsd['ids'].get('internal_ref', [])) & set(tside['ids'].get('internal_ref', []))),
            title_equal_ci=eq_titles(dcan, tside['canonical']),
            doe={k: v for k, v in dsd.items()}, ted=tside))
for u in unmerged:
    if not u['procedure_key']: continue          # islands: the label is not certain -> unmerged.json only
    for c in u['candidates']:
        if c['via'] != 'org_id' or not c.get('has_signals'): continue
        pk = (c.get('tender') or {}).get('procedure_key')
        if not pk or pk == u['procedure_key']: continue
        d = u['doe']
        negatives.append(dict(pool='doe_only_key_disjoint', label='negative (near-certain)',
            label_basis='DOE-only Tender keyed by a UUID procedure id; the TED Tender carries a different UUID key (a forwarded twin shares the UUID, ADR-0003)',
            doe_tender_id=u['tender_id'], doe_procedure_key=u['procedure_key'], ted_tender_id=c['tender_id'],
            shared_org_ids=c['shared_org_ids'], delta_days=c['delta_days'], same_subtype=c['subtype'] == d['subtype'],
            same_cpv_division=bool(set(div(d['canonical'].get('cpv_main'))) & set(div(c['canonical'].get('cpv_main')))),
            internal_ref_equal=bool(set(d['ids'].get('internal_ref', [])) & set(c['ids'].get('internal_ref', []))),
            title_equal_ci=eq_titles(d['canonical'], c['canonical']),
            doe=d, ted=dict(c, source='ted')))
negatives.sort(key=lambda n: (not n['same_cpv_division'], not n['same_subtype'], abs(n['delta_days'])))
# ---------------- metadata, request log, quick stats
log = [json.loads(l) for l in open('reqlog2.jsonl')]
shapes = collections.OrderedDict()
for e in log:
    lab = re.sub(r'-\d{4}-\d{2}-\d{2}$|-\d+$', '', e['label'])
    s = shapes.setdefault(lab, dict(requests=0, rows=0, example_sql=None, max_wall_s=0))
    s['requests'] += 1; s['rows'] += e['row_count'] or 0; s['max_wall_s'] = max(s['max_wall_s'], e['wall_s'])
    if s['example_sql'] is None:
        q = e['sql']; parts = q.split(' UNION ALL ')
        s['example_sql'] = parts[0] + (f'  [... UNION ALL x{len(parts)} arms of the same shape]' if len(parts) > 1 else '')
REQ = dict(total=len(log), ok=sum(1 for e in log if e['ok']), errors=sum(1 for e in log if not e['ok']),
           truncated=sum(1 for e in log if e.get('truncated')), http_408=0, max_wall_s=max(e['wall_s'] for e in log),
           first=log[0]['at'], last=log[-1]['at'], log_file='reqlog2.jsonl (every exact SQL, wall time, row count)', raw_bodies='r2/NNN-<label>.json', shapes=shapes)
PERIOD = dict(doe_core_month='2025-04-01..2025-04-30 (tender_versions.published_at, UTC)', ted_window='2025-03-02..2025-05-30 (core month +-30 days)',
              census='tender_versions by published_at day windows: every non-FTS version in the core month, every TED-shaped (NNNNNNNN-YYYY) version in the outer window',
              census_tenders=len(T))
COMMON = dict(gaps=None, period=PERIOD, requests=REQ, generated_at=dt.datetime.utcnow().strftime('%Y-%m-%dT%H:%M:%SZ'),
              signal_notes=dict(
                  canonical='tender_version_* at the named seq: CUMULATIVE fold state. On a merged Tender the later version carries the earlier version\'s facts where the later notice is silent (carried_state_possible=true when seq>1). Use `notice` (own values) for positives.',
                  notice='notice-layer own values of that notice only (positives both sides): title BT-21-Procedure/DE1-ProcurementProject-Name/SDK01-ProcurementProject-Name; cpv_main procedure-level BT-262-Procedure/DE1|SDK01 ProcurementProject-MainCommodity (normalised like project.rs normalize_cpv); nuts BT-5071-*/…RealizedLocation-Address-CountrySubentityCode; deadline BT-131(d)-*/…TenderSubmissionDeadlinePeriod-EndDate; estimated value BT-27-*/…EstimatedOverallContractAmount; lots = notice_sections kind Lot/ProcurementProjectLot',
                  ids='notice_ids: internal_ref = BT-22-Procedure | DE1-ProcurementProject-ID | SDK01-ProcurementProject-ID; procedure_id = BT-04-notice | DE1-/SDK01-ContractFolderID; previous_notice_ref = OPP-090-Procedure',
                  buyers='tender_version_parties rows with role LIKE %uyer% whose mention_notice_id is this notice (provenance-exact), organizations row for names/identifiers',
                  times='epoch seconds UTC; *_iso helpers added'))
def jdump(name, obj): json.dump(obj, open(name, 'w'), ensure_ascii=False, indent=1, default=list)
# quick stats
def eq_titles(a, b): return bool(titles(a) & titles(b))
ps = collections.Counter()
def cmp4(x, y):
    if x in (None, [], {}, set()) and y in (None, [], {}, set()): return 'both_missing'
    if x in (None, [], {}, set()) or y in (None, [], {}, set()): return 'one_missing'
    if isinstance(x, (set, list)): return 'agree' if set(x) & set(y) else 'differ'
    return 'agree' if x == y else 'differ'
for p in positives:
    a, b = p['doe']['notice'], p['ted']['notice']
    ps['n'] += 1
    sig = dict(
        title_exact_ci=(titles(a), titles(b)),
        cpv_main=(a.get('cpv_main'), b.get('cpv_main')),
        cpv_division=(div(a.get('cpv_main')), div(b.get('cpv_main'))),
        nuts_any=(sorted(set(sum(((a.get('nuts') or {}).get(k, []) for k in ('procedure', 'lots', 'unscoped')), []))), sorted(set(sum(((b.get('nuts') or {}).get(k, []) for k in ('procedure', 'lots', 'unscoped')), [])))),
        deadline_min=((a.get('deadline') or {}).get('min'), (b.get('deadline') or {}).get('min')),
        value_procedure_cents=((a.get('estimated_value') or {}).get('procedure_cents'), (b.get('estimated_value') or {}).get('procedure_cents')),
        lots=(a.get('lots') or None, b.get('lots') or None),
        buyer_org_id=(p['doe']['buyer_org_ids'], p['ted']['buyer_org_ids']),
        internal_ref=(p['doe']['ids'].get('internal_ref'), p['ted']['ids'].get('internal_ref')),
        procedure_id=(p['doe']['ids'].get('procedure_id'), p['ted']['ids'].get('procedure_id')),
        legal_basis=(p['doe'].get('legal_basis'), p['ted'].get('legal_basis')))
    for k, (x, y) in sig.items(): ps[(k, cmp4(x, y))] += 1
    ps[('pub_delta_days', 'le7' if abs(p['pairing']['delta_days']) <= 7 else 'gt7')] += 1
    ps[('canonical_title_doe_equals_notice_title_doe', eq_titles(p['doe']['canonical'], a))] += 1
us = collections.Counter()
for u in unmerged:
    k = (u['doe_profile'], 'EU' if u['eu_legal_basis'] else 'nat')
    us[(k, 'n')] += 1
    us[(k, 'has_cand')] += u['n_candidates'] > 0
    us[(k, 'has_org_cand')] += u['n_candidates_org_id'] > 0
    us[(k, 'title_exact_cand')] += any(c.get('canonical') and eq_titles(u['doe']['canonical'], c['canonical']) for c in u['candidates'])
ns = collections.Counter()
for n in negatives:
    ns[(n['pool'], 'n')] += 1; ns[(n['pool'], 'same_cpv_div')] += n['same_cpv_division']; ns[(n['pool'], 'same_subtype')] += n['same_subtype']
    ns[(n['pool'], 'title_exact')] += eq_titles(n['doe']['canonical'], n['ted']['canonical'])
    ns[(n['pool'], 'internal_ref_equal')] += n['internal_ref_equal']
STATS = dict(positives={str(k): v for k, v in ps.items()}, unmerged={str(k): v for k, v in us.items()}, negatives={str(k): v for k, v in ns.items()})
GAPS = [
 "No sdk-0.1 positives exist: 0 of the 12,513 Tenders with a 2025-04 DOE version and a TED version carry an sdk-0.1 DOE version (numeric or uuid channel). Positives are all eForms-DE 1.1/1.2/2.0/2.1, while the DOE-only population is 98.6% sdk-0.1 (13,465 of 13,656), so the sdk-0.1 band needs hand labels.",
 "sdk-0.1 buyer orgs never share an organization_id with TED. sdk-0.1 mentions carry no country and no identifier, so they become provisional orgs with country NULL. Blocking by org id finds 0 candidates for all 530 numeric islands; blocking by name_norm siblings is added (via=name_norm).",
 "DOE-only Tenders that share an organization_id with a TED Tender are essentially the eForms-family ones, and their islands (PINs) are mostly real twins. Negatives therefore come from pool twin_known (the DOE notice belongs to a UUID-merged Tender, so the DOE side is not DOE-only) and pool doe_only_key_disjoint (a UUID-keyed DOE-only Tender against a TED Tender with a different UUID).",
 "The time window is +-30 days, the reach of the census. That sits inside the 60 days requested for negatives.",
 "Canonical tender_version_* values are cumulative fold state (carried_state_possible). Positives also carry each notice's own notice-layer values. Unmerged records, candidates and negatives carry canonical values plus notice-layer ids only.",
 "Candidate signals are capped: the 3 nearest competitors per positive, and the 15 nearest candidates per non-sdk-0.1 unmerged record. All other candidates are listed with ids and dates only (has_signals=false). sdk-0.1 records have signals on every candidate.",
 "DE1 (eForms-DE 1.x) stores lot leaves under the procedure field ids. CPV, title and internal_ref were re-read with section_id and scoped to PROCEDURE. DE1 NUTS sit in ND-RealizedLocation sections whose scope cannot be separated (nuts.unscoped).",
 "FTS (6-digit ids) is excluded from the census. TED-shaped (8-digit) census versions were checked against notices.source: 758 of 758 sampled are ted. The 2025-04 DOE id span holds no DOE notice with a TED-shaped publication_id.",
 "4 sampled 'DOE-only' eForms-DE Tenders turned out to be merged, with a TED version after 2025-05-30, and were dropped (unmerged meta lists them).",
 "5 positives are named island:<TED notice>. They are ADR-0011 OPP-090 chains with many versions. Their pairing is still the same-subtype version nearest in time.",
 "Deadlines and estimated values are often absent (CANs, PINs). On the notice layer, 247 positives have a deadline on neither side, and 382 have no procedure-level estimated value on either side.",
 "BT-22 / Vergabenummer is not projected and was read from notice_ids: BT-22-Procedure, DE1-ProcurementProject-ID (PROCEDURE section), SDK01-ProcurementProject-ID.",
 "Org fragmentation: name_norm families reach 1,806 orgs (immobilien bremen) and 363 (GIZ). The reverse lookups covered every one of the 5,199 sibling orgs (16 requests)."]
COMMON['gaps'] = GAPS
jdump('positives.json', dict(meta=dict(COMMON, description='UUID-merged TED<->DOE Tenders whose first DOE version is in 2025-04; per pair the DOE version and its same-subtype nearest TED version, canonical (cumulative) and notice-layer (own) signals side by side',
                                       sample='stratified random (seed 4810) of 12,513 merged Tenders with a core-month DOE version: de-1.1 120/1934, de-1.2 120/3773, de-2.0 170/6758, de-2.1 48/48', quick_stats=STATS['positives']), pairs=positives))
jdump('unmerged.json', dict(meta=dict(COMMON, description='DOE-only procedure Tenders (tenders.source=doe) with a 2025-04 DOE version, each with its signals and the TED Tenders of the same buyer org id (via=org_id) or of a same-name_norm sibling org (via=name_norm) with a TED version within +-30 days',
                                      sample='every DOE-only Tender in 2025-04 that is above-threshold (EU legal basis) or eForms-family, plus seeded random national sdk-0.1 (200 numeric islands, 100 uuid channel); %d dropped because tenders.source=ted (TED version outside the census window): %s' % (len(dropped_unm), dropped_unm),
                                      candidate_signals='signals are read for every candidate of sdk-0.1 records and for the 15 nearest (same subtype first) of other records; the rest are listed with ids/dates only (has_signals=false)',
                                      quick_stats=STATS['unmerged']), tenders=unmerged))
jdump('negatives.json', dict(meta=dict(COMMON, description='Pairs of DIFFERENT Tenders sharing a buyer organization_id with publication within +-30 days (inside the requested 60), ordered same-CPV-division first',
                                       pools=dict(twin_known='DOE notice of a UUID-merged Tender (its TED twin known) x up to 3 nearest other TED Tenders of the same buyer org (same subtype first). DOE side is not DOE-only: DOE-only Tenders in 2025-04 are 98.6% sdk-0.1, whose buyer orgs never share an organization_id with TED (no country/identifier on sdk-0.1 mentions)',
                                                  doe_only_key_disjoint='DOE-only UUID-keyed Tender x TED Tender of the same buyer org with a different UUID key'),
                                       quick_stats=STATS['negatives']), pairs=negatives))
print(json.dumps(STATS, indent=1))
print('positives', len(positives), 'unmerged', len(unmerged), 'negatives', len(negatives), collections.Counter(n['pool'] for n in negatives))
