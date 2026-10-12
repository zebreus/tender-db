#!/usr/bin/env python3
"""Issue 519 M1 (and the M5 holders): every distinct mention name of every child holder, scored with
`child_check` (child_check_sim.py) under each ODS-dated parent, then each holder classified.

    python3 -I -B m1_score.py        # writes m1.json (and prints the tallies)

Inputs (all already on disk; this script reads nothing from prod):
- holders: ../sub-icb.json (52 RO98 orgs), ../gbnhs-orgs-classified.json (25 RO198/RC2 site orgs), and
  the zero-only child clusters found for M5 (m1-raw/m5_*), the QHM survivor's own zero-coded mentions
  listed for information only;
- mentions + notices.published_at: m4-reads.json (M4's bounded reads of the same 77 orgs, reused so the
  box is not read twice) and m1-raw/m5_zero_holders_ments.json, m1-raw/m5_notices_*.json;
- parents: the ODS ORD record of each child code (../ods-table-codes/, else ods-ro98/ = M2's refetch,
  else ../ods/), relationship RE5 -> RO261 for RO98 codes and RE6 -> RO197 for RO198 sites, with their
  Operational dates (Legal when no Operational). Other RE5 targets (RO210 regions, RO173) are dropped and
  listed;
- 513 bodies: ../../513-nhs/u5/plan-s4-after-daily.json (the newest census): each parent's verdict
  status and live survivor (PPON or anchor), and the nhs-body ledger's armed name keys.
- overrides: m1_overrides.json, the reviewed calls for refused names the rules below cannot place.
"""
import collections
import datetime as dt
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import child_check_sim as cc  # noqa: E402

U519 = os.path.dirname(HERE)
SCRATCH = os.path.dirname(U519)
RAW = os.path.join(HERE, 'm1-raw')


def load(p):
    return json.load(open(p, encoding='utf-8'))


def org_name_norm(name):
    """canonical.rs `org_name_norm`: lower-case, whitespace collapsed, trailing `.`/`,`/`;` cut."""
    collapsed = ' '.join((name or '').lower().split())
    stripped = collapsed.rstrip('.,; \t\n\r')
    return stripped if stripped else collapsed


# ---- ODS parents ---------------------------------------------------------------------------------
def ods_record(code):
    for p in (os.path.join(U519, 'ods-table-codes', code + '.json'), os.path.join(HERE, 'ods-ro98', code + '.json'),
              os.path.join(U519, 'ods', code + '.json')):
        if os.path.exists(p):
            d = load(p)
            if str(d.get('http')) == '200' and d.get('body'):
                return d['body']['Organisation'], os.path.relpath(p, HERE)
    return None, None


def dates(ds):
    op = [d for d in ds if d.get('Type') == 'Operational']
    lg = [d for d in ds if d.get('Type') == 'Legal']
    d = (op or lg or [{}])[0]
    return d.get('Start'), d.get('End')


def parents_of(code, rel_id, target_role):
    o, src = ods_record(code)
    if o is None:
        return None, [], src, None
    keep, dropped = [], []
    for r in (o.get('Rels') or {}).get('Rel', []):
        if r.get('id') != rel_id:
            continue
        tgt = r['Target']['OrgId']['extension']
        role = r['Target']['PrimaryRoleId']['id']
        s, e = dates(r.get('Date', []))
        row = {'parent': tgt, 'parent_role': role, 'start': s, 'end': e, 'status': r.get('Status')}
        (keep if role == target_role else dropped).append(row)
    # One row per (child, parent, interval): adjacent intervals with one parent merged (06K's S1Y5D).
    keep.sort(key=lambda r: (r['parent'], r['start'] or ''))
    merged = []
    for r in keep:
        if merged and merged[-1]['parent'] == r['parent'] and merged[-1]['end'] and r['start'] and \
                (dt.date.fromisoformat(merged[-1]['end']) + dt.timedelta(days=1)).isoformat() == r['start']:
            merged[-1]['end'] = r['end']
            merged[-1]['status'] = r['status']
        else:
            merged.append(dict(r))
    merged.sort(key=lambda r: r['start'] or '')
    return merged, dropped, src, o.get('Name')


def unix(d, end=False):
    if not d:
        return None
    t = dt.datetime.fromisoformat(d).replace(tzinfo=dt.timezone.utc)
    if end:
        t += dt.timedelta(days=1)
    return int(t.timestamp())


# ---- 513 bodies ----------------------------------------------------------------------------------
plan = json.loads(load(os.path.join(SCRATCH, '513-nhs', 'u5', 'plan-s4-after-daily.json'))['body'])
by_code = {c['code']: c for c in plan['codes']}
armed = collections.defaultdict(set)  # name_norm -> {code}
for r in plan['nhs_body_merged']:
    e = r['evidence']
    for k in e.get('loser_name_norms') or []:
        if k:
            armed[k].add(e['code'])


def body_of(code):
    """The 513 body standing on an allowlisted code: verdict status and live survivor."""
    c = by_code.get(code)
    if c is None:
        return None
    v = c.get('verdict') or {}
    members = set(v.get('members') or [])
    live = [h for h in c['holders'] if h['org_id'] in members and h['stands']]
    surv = None
    if v.get('status') == 'applied' and live:
        h = live[0] if len(live) == 1 else max(live, key=lambda h: h['mentions'])
        kind = 'ppon' if (h.get('identifier') or '').startswith('GBPPON') or h.get('ppons') else \
            ('anchor' if h.get('provisional') else 'identity')
        surv = {'org_id': h['org_id'], 'name': h['name'], 'kind': kind, 'identifier': h.get('identifier')}
    return {'status': v.get('status'), 'cohort': v.get('cohort'), 'survivor': surv}


# ---- holders -------------------------------------------------------------------------------------
m4 = load(os.path.join(HERE, 'm4-reads.json'))
published = {n[0]: n[1] for n in m4['notices']}
for f in sorted(os.listdir(RAW)):
    if f.startswith('m5_notices_'):
        for n in load(os.path.join(RAW, f))['rows']:
            published[n[0]] = n[1]
head = {o[0]: {'identifier': o[1], 'name': o[2], 'provisional': o[3]} for o in m4['orgs']}
mentions = collections.defaultdict(list)  # org -> [(notice, name, raw)]
for m in m4['mentions']:
    mentions[m[0]].append((m[1], m[3], m[5]))
for tag in ('m5_zero_holders', 'm5_zero_holders2'):
    for o in load(os.path.join(RAW, tag + '_orgs.json'))['rows']:
        head[o[0]] = {'identifier': o[3], 'name': o[4], 'provisional': o[6]}
    for m in load(os.path.join(RAW, tag + '_ments.json'))['rows']:
        mentions[m[0]].append((m[1], m[3], m[5]))

holders = []
for r in load(os.path.join(U519, 'sub-icb.json')):
    holders.append({'org_id': r['org'], 'child': r['code'], 'kind': 'sub-icb', 'rel': 'RE5', 'target_role': 'RO261'})
for r in load(os.path.join(U519, 'gbnhs-orgs-classified.json')):
    o = r.get('ods') or {}
    if o.get('primary') == 'RO198' and o.get('rc') == 'RC2':
        holders.append({'org_id': r['id'], 'child': r['code'], 'kind': 'site', 'rel': 'RE6', 'target_role': 'RO197'})
# The zero-only tails' holders (M5), less the QHM survivor 31556600 (20 x 00P, 5 x 00N), which is the body.
ZERO = {31603816: '00P', 31605903: '00P', 31615636: '00P', 31610859: '00P', 31615979: '00P', 31632651: '00L',
        30915461: '00X', 31624168: '00X'}
for org, code in ZERO.items():
    holders.append({'org_id': org, 'child': code, 'kind': 'zero-only', 'rel': 'RE5', 'target_role': 'RO261'})
assert len(holders) == 52 + 25 + 8, len(holders)

overrides = load(os.path.join(HERE, 'm1_overrides.json')) if os.path.exists(os.path.join(HERE, 'm1_overrides.json')) else {}


def best_body(name):
    """The table row a refused name fits best (child rival set: Active RO98 rows dropped), and whether
    `child_check` admits the name under it — the body a stray belongs to, when the table has it."""
    m = cc.split(name)
    best, bc = None, None
    for c in cc.codes:
        if cc.table[c]['role'] == 'RO98' and cc.table[c]['status'] == 'A':
            continue
        k = cc.key(m, [(cc.table[c]['name'], cc.table[c]['role'])])
        if best is None or k > best:
            best, bc = k, c
    if bc is None or best[1] == 0:
        return None, None
    return bc, cc.label(cc.child_check(bc, name))[0]


def auto_call(name, chosen, labels, other_parents_admit, child_lineage):
    """A refused name's call when the rules can place it: (call, body, why) or None (needs review).
    call: 'stray' (another body's name) | 'noise' (the body's own name the gate refuses)."""
    lab, rival = labels[chosen]
    if other_parents_admit:
        q = other_parents_admit[0]
        return 'stray', q, f'admitted under the other dated parent {q}'
    if lab == 'nhse':
        return 'stray', 'NHS England (523)', 'nhse'
    if lab == 'better' and rival in child_lineage:
        return 'noise', rival, f'better{{{rival}}}: the child\'s own predecessor CCG'
    bb, bb_lab = best_body(name)
    if lab != 'tie' and bb and bb != chosen and bb_lab == 'admitted' and bb not in child_lineage:
        return 'stray', bb, f'{lab}; child_check admits it under {bb} ({cc.table[bb]["name"]})'
    return None


def phase_of(kind, ch):
    """The decision's partition (sec. 1.5) for the chosen parent: A = allowlisted, applied 513 body, PPON
    survivor (phase A); A2 = applied body with an anchor survivor (held, ruling 5); A3 = parent off the
    allowlist (digit or five-character ICB: U6 or nothing); none = no dated parent in the table."""
    if ch is None:
        return 'none'
    if not ch['allow']:
        return 'A3-off-allowlist'
    b = ch['body'] or {}
    surv = b.get('survivor') or {}
    if b.get('status') != 'applied' or not surv:
        return 'no-applied-body'
    return 'A-ppon-survivor' if surv.get('kind') == 'ppon' else 'A2-' + surv.get('kind', '?') + '-survivor'


out_holders = []
names_out = []
for h in holders:
    org = h['org_id']
    hd = head.get(org)
    parents, dropped, src, ods_name = parents_of(h['child'], h['rel'], h['target_role'])
    lineage = set()
    o, _ = ods_record(h['child'])
    if o:
        lineage.add(h['child'])
        for s in (o.get('Succs') or {}).get('Succ', []):
            if s.get('Type') == 'Predecessor':
                lineage.add(s['Target']['OrgId']['extension'])
    in_table = [p for p in (parents or []) if p['parent'] in cc.table]
    for p in in_table:
        p['allow'] = cc.table[p['parent']]['allow']
        p['body'] = body_of(p['parent']) if p['allow'] else None
    ms = mentions.get(org, [])
    by_name = collections.OrderedDict()
    for notice, name, raw in ms:
        e = by_name.setdefault(name or '', {'n': 0, 'instants': [], 'raws': collections.Counter()})
        e['n'] += 1
        e['raws'][raw] += 1
        if published.get(notice) is not None:
            e['instants'].append(published[notice])
    if hd and hd['name'] not in by_name:
        by_name[hd['name']] = {'n': 0, 'instants': [], 'raws': collections.Counter(), 'head_only': True}
    # Score every name under every dated parent in the table.
    scored = []
    for name, e in by_name.items():
        labels = {p['parent']: cc.label(cc.child_check(p['parent'], name)) for p in in_table}
        periods = {}
        for p in in_table:
            s, en = unix(p['start']), unix(p['end'], end=True)
            b = sum(1 for t in e['instants'] if s is not None and t < s)
            a = sum(1 for t in e['instants'] if en is not None and t >= en)
            periods[p['parent']] = {'before_start': b, 'in_period': len(e['instants']) - b - a, 'after_end': a}
        scored.append({'name': name, 'name_norm': org_name_norm(name), 'n': e['n'], 'head': bool(hd and name == hd['name']),
                       'first': min(e['instants']) if e['instants'] else None,
                       'last': max(e['instants']) if e['instants'] else None,
                       'unpublished': e['n'] - len(e['instants']),
                       'raws': dict(e['raws']), 'check': {k: {'verdict': v[0], 'rival': v[1]} for k, v in labels.items()},
                       'periods': periods, 'armed_513_key': sorted(armed.get(org_name_norm(name), []))})
    # The parent the holder would join: the allowlisted dated parent admitting the most mentions
    # (then the head name, then the earlier period); else the best non-allowlisted one.
    def admits(p):
        return sum(s['n'] for s in scored if s['check'][p['parent']]['verdict'] == 'admitted')

    def head_ok(p):
        return any(s['head'] and s['check'][p['parent']]['verdict'] == 'admitted' for s in scored)

    cands = sorted(in_table, key=lambda p: (-(1 if p['allow'] else 0), -admits(p), -int(head_ok(p)), p['start'] or ''))
    chosen = cands[0]['parent'] if cands else None
    if cands and admits(cands[0]) == 0:
        alt = max(in_table, key=lambda p: (admits(p), int(head_ok(p))))
        if admits(alt) > 0:
            chosen = alt['parent']
    # Classify each name under the chosen parent.
    n_adm = n_stray = n_noise = n_review = n_site = 0
    strays = collections.Counter()
    for s in scored:
        if chosen is None:
            s['call'] = 'no-parent'
            continue
        lab = s['check'][chosen]['verdict']
        if lab == 'admitted':
            s['call'] = 'admitted'
            n_adm += s['n']
            continue
        other = [p['parent'] for p in in_table if p['parent'] != chosen and s['check'][p['parent']]['verdict'] == 'admitted']
        key = f'{h["child"]}|{s["name"]}'
        ov = overrides.get(key)
        call = (ov['call'], ov.get('body'), 'reviewed: ' + ov['why']) if ov else \
            auto_call(s['name'], chosen, {k: (v['verdict'], v['rival']) for k, v in s['check'].items()}, other, lineage)
        if call is None:
            bb, bb_lab = best_body(s['name'])
            s['call'] = 'review'
            s['best_body'] = {'code': bb, 'check': bb_lab, 'name': cc.table[bb]['name'] if bb else None}
            n_review += s['n']
            continue
        s['call'], s['body'], s['why'] = call
        if s['call'] == 'stray':
            n_stray += s['n']
            strays[s['body']] += s['n']
        elif s['call'] == 'site':
            n_site += s['n']
        else:
            n_noise += s['n']
    total = sum(s['n'] for s in scored)
    head_call = next((s['call'] for s in scored if s['head']), None)
    kinds = {k for k, n in (('stray', n_stray), ('noise', n_noise), ('site', n_site), ('review', n_review)) if n}
    if chosen is None:
        cls = 'no-parent'
    elif n_review:
        cls = 'needs-review'
    elif n_adm == 0:
        cls = 'none-admitted:' + ('other-body' if kinds == {'stray'} else 'site-named' if kinds == {'site'} else
                                  'own-name-noise' if kinds == {'noise'} else 'mixed')
    elif n_adm == total and head_call == 'admitted':
        cls = 'all-admitted'
    elif kinds == {'stray'}:
        cls = 'other-body-strays'
    elif kinds == {'noise'}:
        cls = 'own-name-noise'
    elif kinds == {'stray', 'noise'}:
        cls = 'strays-and-noise'
    else:
        cls = 'mixed:' + '+'.join(sorted(kinds))
    ch = next((p for p in in_table if p['parent'] == chosen), None)
    out_holders.append({
        'org_id': org, 'child': h['child'], 'kind': h['kind'], 'head_name': hd['name'] if hd else None,
        'identifier': hd['identifier'] if hd else None, 'provisional': hd['provisional'] if hd else None,
        'ods_name': ods_name, 'ods_source': src, 'parents': parents, 'dropped_rels': dropped,
        'parents_off_table': [p['parent'] for p in (parents or []) if p['parent'] not in cc.table],
        'chosen_parent': chosen, 'chosen_allow': ch['allow'] if ch else None,
        'chosen_body': ch['body'] if ch else None,
        'mentions': total, 'names': len(scored), 'admitted_mentions': n_adm, 'stray_mentions': n_stray,
        'noise_mentions': n_noise, 'site_mentions': n_site, 'review_mentions': n_review,
        'strays_by_body': dict(strays), 'head_call': head_call,
        'class': cls, 'phase': phase_of(h['kind'], ch), 'name_detail': scored})

json.dump({'computed': dt.datetime.now(dt.timezone.utc).isoformat(timespec='seconds'),
           'tsv_version': cc.VERSION, 'holders': out_holders}, open(os.path.join(HERE, 'm1.json'), 'w'), indent=1)

# ---- print --------------------------------------------------------------------------------------
for kind in ('sub-icb', 'site', 'zero-only'):
    hs = [h for h in out_holders if h['kind'] == kind]
    c = collections.Counter(h['class'] for h in hs)
    mm = collections.Counter()
    for h in hs:
        mm[h['class']] += h['mentions']
    print(kind, len(hs), sum(h['mentions'] for h in hs), {k: (c[k], mm[k]) for k in sorted(c)})
rev = [(h['child'], s['name'], s['n'], s['check'][h['chosen_parent']], s.get('best_body')) for h in out_holders
       for s in h['name_detail'] if s.get('call') == 'review']
print('names to review', len(rev), sum(r[2] for r in rev))
if '--review' in sys.argv:
    for r in rev:
        print('  ', r)
