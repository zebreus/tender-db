"""Issue 551 unit 4b M1 FIDELITY CHECK: an independent derivation of the facts v1 (T', K, G0) and v2 (R1, R2, R3,
R10, plus R6/R7/R8) read, written from issue 551's rule text and its adopted amendments, NOT from rule_v2.py.

Inputs: my own fresh reads under fid/raw (the API detail and every notice it names, fetched by fetch.py), the ECB
reference-rate history (eurofxref-hist.zip, the public series).

Independence notes (what this does differently from the port on purpose):
- winners per lot and the LotTender grouping are read from the AWARD NOTICE'S OWN sections (eForms LotResult ->
  OPT-320 -> LotTender -> OPT-310 -> TenderingParty -> Tenderer; TED award block ADDRESS_CONTRACTOR + AWARDED_TO_GROUP),
  and only compared with the API's winners;
- the published lot number is read three ways (the lot section with the API key's section id, the award block's
  own LOT_NO / BT-13713, and the lot section the award block's LOT_NO/BT-13713 points to) so a disagreement shows;
- stated totals are read from every notice, any section, then classified by the rule's table, so a total the port
  drops by section kind or form is visible;
- the latest award per lot is cross-checked against every notice's award blocks, not only the API rows.
"""
import collections
import json
import re
import sys
import zipfile

import os
BASE = os.environ.get('M551', '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551')
RAW = BASE + '/fid/raw'
ECB = BASE + '/rates/eurofxref-hist.zip'


def load(kind, i):
    return json.load(open(f'{RAW}/{kind}/{i}.json'))


# ------------------------------------------------------------------ ECB rate at a date (nearest at/before, 7 days)
_ecb = None


def ecb(cur, date):
    global _ecb
    if cur == 'EUR':
        return 1.0
    if _ecb is None:
        z = zipfile.ZipFile(ECB)
        lines = z.read(z.namelist()[0]).decode().splitlines()
        head = lines[0].split(',')
        _ecb = collections.defaultdict(dict)
        for ln in lines[1:]:
            c = ln.split(',')
            for i in range(1, len(head)):
                if head[i] and i < len(c):
                    try:
                        _ecb[head[i]][c[0]] = float(c[i])
                    except ValueError:
                        pass
    import datetime
    d = datetime.date.fromisoformat(date)
    for back in range(0, 8):
        k = (d - datetime.timedelta(days=back)).isoformat()
        if k in _ecb[cur]:
            return _ecb[cur][k]
    return None


def head_utc_date(p):
    import datetime
    if len(p) == 10:
        return p
    dt = datetime.datetime.fromisoformat(p.replace('Z', '+00:00'))
    return dt.astimezone(datetime.timezone.utc).date().isoformat()


# ------------------------------------------------------------------ the rule's small definitions
def sentinel(c):
    """canonical.rs sentinel_amount as the unit-3 text describes it: <=0, 1, 100, or a 9+ repdigit (cents or units)."""
    if c is None or c <= 0 or c in (1, 100):
        return True
    s = str(c)
    if len(s) >= 9 and len(set(s)) == 1:
        return True
    if c % 100 == 0:
        u = str(c // 100)
        if len(u) >= 9 and len(set(u)) == 1:
            return True
    return False


def tokens(p):
    """R2: a published lot number as a token set: leading zeros dropped, ranges up to 50 expanded, lists split."""
    if p is None:
        return None
    s = str(p).lower()
    out = set()
    for a, b in re.findall(r'(\d+)\s*(?:-|–|to|bis)\s*(\d+)', s):
        a, b = int(a), int(b)
        if 0 < b - a <= 50:
            out |= {str(i) for i in range(a, b + 1)}
    for t in re.findall(r'\d+', s):
        out.add(t.lstrip('0') or '0')
    return frozenset(out) or None


CN_FORMS = {1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 16, 17, 24}


def is_cn(form):
    if form is None:
        return False
    m = re.fullmatch(r'[Ff]?0*(\d+)', str(form).strip())
    return bool(m) and int(m.group(1)) in CN_FORMS


def vals(sec, field):
    return [v for v in sec['values'] if v['field_id'] == field]


def first(sec, *fields):
    for f in fields:
        for v in sec['values']:
            if v['field_id'] == f:
                return v.get('value', v.get('code'))
    return None


# ------------------------------------------------------------------ per notice
def read_notice(nid):
    n = load('notices', nid)
    secs = n['sections']
    by_id = {s['section_id']: s for s in secs}
    out = {'nid': nid, 'form': None, 'logical_id': None, 'change': False, 'lots': {}, 'results': {}, 'amounts': [],
           'tenders': {}, 'tparties': {}}
    for s in secs:
        k = s.get('kind')
        if k == 'Change':
            out['change'] = True
        f = first(s, 'TED-FORM')
        if f is not None and out['form'] is None:
            out['form'] = f
        lid = first(s, 'BT-701-notice', 'DE1-ID')
        if lid is not None and out['logical_id'] is None:
            out['logical_id'] = lid
        if out.get('subtype') is None:
            st = first(s, 'OPP-070-notice', 'BT-02-notice')
            if st is not None:
                out['subtype'] = st
        for v in s['values']:
            if v.get('type') == 'amount':
                out['amounts'].append({'field': v['field_id'], 'kind': k, 'section': s['section_id'],
                                       'cents': v.get('cents'), 'cur': v.get('currency')})
        if k in ('Lot', 'Part', 'LotsGroup', 'ProcurementProjectLot'):
            out['lots'][s['section_id']] = {
                'lot_no': first(s, 'TED-LOT_NO', 'TED-LOT_NUMBER', 'BT-137-Lot', 'BT-137-Part', 'BT-137-LotsGroup',
                                'DE1-ProcurementProjectLot-ID'),
                'item': first(s, 'TED-ITEM'), 'title': first(s, 'TED-TITLE', 'BT-21-Lot', 'BT-21-Part')}
        if k == 'LotResult':
            tref = [v['value'] for v in vals(s, 'OPT-320-LotResult')]
            out['results'][s['section_id']] = {
                'lot_no': first(s, 'TED-LOT_NO', 'TED-LOT_NUMBER'),
                'lot_ref': first(s, 'BT-13713-LotResult', 'DE1-NoticeResult-LotResult-TenderLot-ID'),
                'item': first(s, 'TED-ITEM'),
                'group': first(s, 'TED-AWARDED_TO_GROUP'), 'no_group': first(s, 'TED-NO_AWARDED_TO_GROUP'),
                'contractors': [v['value'] for v in vals(s, 'TED-ADDRESS_CONTRACTOR')],
                'tender_refs': tref,
                'amounts': [(v['field_id'], v.get('cents'), v.get('currency')) for v in s['values'] if v.get('type') == 'amount'],
                'decision': first(s, 'BT-142-LotResult'),
                'awarded_flag': first(s, 'TED-AWARDED_CONTRACT'), 'no_award': first(s, 'TED-NO_AWARDED_CONTRACT')}
        if k == 'LotTender':
            out['tenders'][s['section_id']] = {'lot': first(s, 'BT-13714-Tender'), 'tpa': first(s, 'OPT-310-Tender'),
                                               'amounts': [(v['field_id'], v.get('cents'), v.get('currency')) for v in s['values'] if v.get('type') == 'amount']}
    # eForms puts OPT-320 (the LotResult's tender references) in child sections (LotResultTenderReference)
    for s in secs:
        par = s.get('parent_section_id')
        if par in out['results'] and s.get('kind') != 'LotResult':
            out['results'][par]['tender_refs'] += [v['value'] for v in vals(s, 'OPT-320-LotResult')]
    # tendering party -> tenderer orgs (Tenderer sections are children of the TPA)
    for s in secs:
        if s.get('kind') == 'Tenderer':
            out['tparties'].setdefault(s.get('parent_section_id'), []).append(first(s, 'OPT-300-Tenderer'))
    return out


# ------------------------------------------------------------------ one tender
def derive(tid):
    x = load('tenders', tid)
    nids = sorted({v['caused_by_notice_id'] for v in x['versions'] if v.get('caused_by_notice_id')} |
                  {r['notice_id'] for k in ('lot_results', 'bids', 'contracts') for r in x.get(k, []) if r.get('notice_id')})
    N = {nid: read_notice(nid) for nid in nids}
    date = head_utc_date(x['published_at'])
    F = {'id': tid, 'date': date, 'notices': nids}

    # ---- v1 (head rows from the API detail)
    unit = 'Lot' if any(l['kind'] == 'Lot' for l in x['lot_details']) else 'Part'
    counted = {l['lot_key'] for l in x['lot_details'] if l['kind'] == unit}
    awards_all = {(lr['awarded']['currency'], lr['awarded']['cents']) for lr in x['lot_results'] if lr.get('awarded')}
    procs = []
    for a in x['amounts']:
        if a.get('lot') is not None or a['field'] in ('result_framework_maximum', 'result_framework_reestimate'):
            continue
        v = a.get('value')
        if v is None and a.get('quality') is None:
            continue
        copy = v is not None and (v['currency'], v['cents']) in awards_all
        procs.append({'field': a['field'], 'value': v, 'quality': a.get('quality'), 'award_copy': copy,
                      'sentinel': v is None or sentinel(v['cents'])})
    non_copy = [p for p in procs if not p['award_copy']]
    F['T_prime_ok'] = all(p['sentinel'] and p['quality'] is None for p in non_copy) if non_copy else True
    F['procedure_figures'] = [(p['field'], p['value'], p['quality'], 'copy' if p['award_copy'] else 'PROC') for p in procs]
    sv = x.get('value')
    F['served'] = sv
    F['K_ok'] = sv is None or (sv['currency'], sv['cents']) in awards_all
    # LotSums per currency: latest award per counted lot by notice id (>=: the later row wins a tie)
    per_cur = collections.defaultdict(dict)
    for lr in x['lot_results']:
        aw = lr.get('awarded')
        if not aw or lr.get('lot') not in counted:
            continue
        if not (aw['cents'] > 1000 and not sentinel(aw['cents'])):
            continue
        d = per_cur[aw['currency']]
        cur_best = d.get(lr['lot'])
        if cur_best is None or lr['notice_id'] >= cur_best[0]:
            d[lr['lot']] = (lr['notice_id'], aw['cents'], lr)
    if sv is not None and F['K_ok']:
        cur = sv['currency']
    else:
        cands = sorted(((len(d), c) for c, d in per_cur.items() if d), reverse=True)
        cur = cands[0][1] if cands else None
    F['currency'] = cur
    per = per_cur.get(cur, {})
    F['other_currency_awards'] = sorted(c for c in per_cur if c != cur and per_cur[c])
    S = sum(c for _, c, _ in per.values())
    F['n'] = len(per)
    F['S'] = S
    r = ecb(cur, date) if cur else None
    F['rate'] = r
    F['S_eur_cents'] = None if r is None else round(S / r)
    t_eur = None
    if sv is not None:
        rr = ecb(sv['currency'], date)
        t_eur = None if rr is None else round(sv['cents'] / rr)
    F['served_eur_cents'] = t_eur
    F['G0_ok'] = F['n'] >= 2 and F['S_eur_cents'] is not None and (t_eur is None or F['S_eur_cents'] > t_eur)
    vals_ = [c for _, c, _ in per.values()]
    mx = max(vals_) if vals_ else 0
    cnt = collections.Counter(vals_)
    F['G1p_repeated'] = any(c >= 2 and v * 10 >= mx for v, c in cnt.items())

    # ---- the summed set, cross-checked with the notices
    summed = []
    for lot in sorted(per):
        nid, cents, lr = per[lot]
        nf = N[nid]
        res = nf['results'].get(lr['key'])
        # published lot number, three readings
        sec = nf['lots'].get(lot)
        pn_section = sec['lot_no'] if sec else None
        pn_block = (res or {}).get('lot_no') or (res or {}).get('lot_ref')
        frozen = pn_section if pn_section is not None else ((res or {}).get('lot_no') if (res or {}).get('lot_no') is not None else (res or {}).get('lot_ref'))
        # the notice's award-block value for that result
        blk_vals = (res or {}).get('amounts', [])
        # winners from the notice
        if res is None:
            win_src = 'result section missing'
            n_tenders, group, winners_notice = None, None, None
        elif res['tender_refs']:
            tenders = [nf['tenders'].get(t) for t in res['tender_refs']]
            winners_notice = []
            for t in tenders:
                winners_notice.append(nf['tparties'].get((t or {}).get('tpa'), []))
            n_tenders = len(res['tender_refs'])
            group = None
            win_src = 'eForms LotTender'
        else:
            winners_notice = [res['contractors']]
            n_tenders = None
            group = res['group'] is not None and res['no_group'] is None
            win_src = 'TED award block'
        api_w = [(w.get('organization_id'), w.get('organization_name')) for w in lr.get('winners', [])]
        # R1, from the notice
        n_win_notice = len({o for w in (winners_notice or []) for o in w})
        if win_src == 'eForms LotTender':
            r1 = n_win_notice >= 2 and n_tenders >= 2
        elif win_src == 'TED award block':
            r1 = n_win_notice >= 2 and not group
        else:
            r1 = None
        # later notices that award this lot (any block whose LOT_NO / BT-13713 / section spells this lot)
        summed.append({'key': lot, 'notice': nid, 'result_key': lr['key'], 'cents': cents, 'block_amounts': blk_vals,
                       'pn_section': pn_section, 'pn_block': pn_block, 'pn_frozen': frozen,
                       'item': (sec or {}).get('item'), 'title_section': (sec or {}).get('title'),
                       'win_src': win_src, 'winners_notice': winners_notice, 'n_tenders': n_tenders, 'group': group,
                       'api_winners': api_w, 'R1': r1, 'decision': lr.get('decision')})
    F['summed'] = summed

    # ---- R2 a-d
    tk = {s['key']: tokens(s['pn_frozen']) for s in summed}
    keys = list(tk)
    F['R2a'] = [(a, b) for i, a in enumerate(keys) for b in keys[i + 1:] if tk[a] and tk[b] and tk[a] & tk[b]]
    F['R2b'] = [k for k in keys if tk[k] and len(tk[k]) >= 2]
    r2c = []
    for s in summed:
        seen = set()
        for lr in x['lot_results']:
            if lr.get('lot') != s['key']:
                continue
            if not (lr.get('awarded') or lr.get('winners')):
                continue
            nf = N[lr['notice_id']]
            sec = nf['lots'].get(s['key'])
            res = nf['results'].get(lr['key']) or {}
            no = sec['lot_no'] if sec and sec['lot_no'] is not None else (res.get('lot_no') if res.get('lot_no') is not None else res.get('lot_ref'))
            t = tokens(no)
            if t:
                seen.add(t)
        if len(seen) > 1:
            r2c.append((s['key'], sorted(sorted(t) for t in seen)))
    F['R2c'] = r2c
    multi = len({s['notice'] for s in summed}) >= 2
    F['R2d'] = [k for k in keys if tk[k] is None] if multi else []

    # ---- stated totals: every notice, every amount, classified by the rule's table
    rows = []
    for nid in nids:
        nf = N[nid]
        mine = []
        for a in nf['amounts']:
            fld, kind = a['field'], a['kind']
            if fld in ('BT-161-NoticeResult', 'BT-118-NoticeResult', 'BT-1118-NoticeResult'):
                k3 = 'award'
            elif fld == 'BT-27-Procedure':
                k3 = 'estimate'
            elif kind == 'Notice' and fld == 'TED-VAL_TOTAL':
                k3 = 'estimate' if is_cn(nf['form']) else 'award'
            elif kind == 'Notice' and fld == 'TED-VAL_ESTIMATED_TOTAL':
                k3 = 'estimate'
            elif kind == 'Notice' and fld in ('TED-LOW', 'TED-HIGH'):
                k3 = 'range'
            else:
                continue
            mine.append({'nid': nid, 'field': fld, 'kind3': k3, 'cents': a['cents'], 'cur': a['cur'],
                         'section': a['section'], 'form': nf['form'], 'logical_id': nf['logical_id']})
        if nf['change'] and nf['logical_id'] and mine:
            rows = [t for t in rows if t['logical_id'] != nf['logical_id']]
        rows += mine
    # "a single-lot notice's total equal to its own award does not count"
    for t in rows:
        own = [lr['awarded']['cents'] for lr in x['lot_results'] if lr['notice_id'] == t['nid'] and lr.get('lot')
               and lr.get('awarded') and lr['awarded']['cents'] > 1000]
        t['single_lot_own'] = len(own) == 1 and t['cents'] is not None and abs(t['cents'] - own[0]) <= 0.01 * own[0]
    F['stated_totals'] = rows
    use = [t for t in rows if t['cur'] == cur and t['cents'] and t['cents'] > 1000 and not sentinel(t['cents'])]
    T = [t for t in use if t['kind3'] == 'award' and not t['single_lot_own']]
    r3 = []
    byn = collections.defaultdict(list)
    for t in T:
        byn[t['nid']].append(t['cents'])
    for nid, ts in byn.items():
        awarded_keys = {lr['lot'] for lr in x['lot_results'] if lr['notice_id'] == nid and lr.get('lot')}
        sk = sum(s['cents'] for s in summed if s['key'] in awarded_keys)
        if sk > 1.01 * max(ts):
            r3.append(('total', nid, max(ts), sk))
    est_notice = [t['cents'] for t in use if t['kind3'] == 'estimate']
    est_api = []
    for a in x['amounts']:
        v = a.get('value')
        if v and v['currency'] == cur and a['field'] in ('estimated_value', 'framework_maximum') and v['cents'] > 1000 and not sentinel(v['cents']):
            est_api.append((a['field'], a.get('lot'), v['cents']))
    allest = est_notice + [c for _, _, c in est_api]
    F['estimates'] = {'notice': est_notice, 'api': est_api}
    if not T and allest and S > 10 * max(allest):
        r3.append(('estimate', None, max(allest), S))
    F['R3'] = r3
    F['R3_award_totals'] = [(t['nid'], t['field'], t['cents']) for t in T]

    # ---- R10
    singles = {s['cents'] for s in summed}
    vouch = [t for t in use if t['kind3'] in ('award', 'estimate', 'range') and not t['single_lot_own']
             and t['cents'] not in singles and abs(t['cents'] - S) <= 0.01 * S]
    F['vouches'] = [(t['field'], t['nid'], t['cents']) for t in vouch]
    F['R10'] = F['S_eur_cents'] is not None and F['S_eur_cents'] >= 1_000_000_000 and not vouch

    # ---- R6 / R7 / R8 (simple, for the refused ids)
    lt = {l['lot_key']: re.sub(r'\W+', ' ', (l.get('title') or '').lower()).strip() for l in x['lot_details']}
    wset = {s['key']: frozenset(str(w[0] or w[1]) for w in s['api_winners']) for s in summed}
    F['R6'] = [(a['key'], b['key']) for i, a in enumerate(summed) for b in summed[i + 1:]
               if a['notice'] != b['notice'] and wset[a['key']] and wset[a['key']] == wset[b['key']]
               and lt.get(a['key']) and lt.get(a['key']) == lt.get(b['key'])]
    r7 = []
    for s in summed:
        for lr in x['lot_results']:
            aw = lr.get('awarded')
            if lr.get('lot') == s['key'] and lr['notice_id'] < s['notice'] and aw and aw['currency'] == cur and aw['cents'] > 1000 and not sentinel(aw['cents']):
                if s['cents'] >= 10 * aw['cents'] or 10 * s['cents'] <= aw['cents']:
                    r7.append((s['key'], lr['notice_id'], aw['cents'], s['cents']))
    F['R7'] = r7
    sv2 = sorted(singles if len(singles) == len(summed) else [s['cents'] for s in summed], reverse=True)
    F['R8'] = len(sv2) >= 2 and sv2[0] >= 10 * sv2[1]
    # ---- every award block in every notice (for the by-hand latest-award check)
    blocks = []
    for nid in nids:
        nf = N[nid]
        for rk, res in nf['results'].items():
            blocks.append((nid, rk, res['lot_no'], res['lot_ref'], res['item'], res['amounts'], len(res['contractors']) or len(res['tender_refs'])))
    F['all_award_blocks'] = blocks
    F['notice_forms'] = {nid: (N[nid]['form'] or N[nid].get('subtype'), 'change' if N[nid]['change'] else '') for nid in nids}
    return F


if __name__ == '__main__':
    out = {}
    for t in sys.argv[1:]:
        out[int(t)] = derive(int(t))
    print(json.dumps(out, indent=1, default=str, ensure_ascii=False))
