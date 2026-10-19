#!/usr/bin/env python3
"""Issue 551 B6 U5: the exclusion set 'every read id', the fresh-read order over P, and the B-band re-read set.

Decision points 7 and 8 (`## B6 — decision`). Offline only: reads recorded files under
.scratch/tender-db/551-lot-sum/ and writes only under b6/u5/. No network.

    python3 -I build_order.py            # writes order.json, excluded_sources.json, band_set.json
"""
import gzip
import hashlib
import io
import json
import os
import re
import sys
import tarfile

BASE = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))  # 551-lot-sum
ISSUE = os.path.normpath(os.path.join(BASE, '..', 'issues',
                                      '551-a-refused-head-falls-to-one-lots-figure-or-null-never-the-lot-sum.md'))
OUT = os.path.join(BASE, 'b6', 'u5')
SALT = '551-b6-fresh-'
EUR100M = 100_000_000_00


def p(rel):
    return os.path.join(BASE, rel)


def sha_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def load(rel):
    with open(p(rel)) as f:
        return json.load(f)


def order_key(tid):
    return hashlib.sha256((SALT + str(tid)).encode()).hexdigest()


inputs = {}


def note_input(rel):
    inputs[rel] = sha_file(p(rel))


# ---------------------------------------------------------------- P (v2+both) and the port's per-id decisions
note_input('b6/u2/P.json')
note_input('b6/u2/decisions.jsonl.gz')
note_input('b6/u4/u4-result.json')
P_all = load('b6/u2/P.json')
P_set = P_all['guard_sets']['v2+both']
P_ships = {s['id']: s for s in P_set['ships']}
P_ids = sorted(P_ships)
assert len(P_ids) == P_set['n'] == 3467, len(P_ids)
P_ids_sha = hashlib.sha256(('\n'.join(map(str, P_ids)) + '\n').encode()).hexdigest()
assert P_ids_sha == P_set['ids_sha256'], (P_ids_sha, P_set['ids_sha256'])
u4 = load('b6/u4/u4-result.json')
assert u4['result']['P_set'] == 'v2+both' and sorted(u4['result']['P_ids']) == P_ids
assert u4['result']['P_ids_sha256_from_P_json'] == P_ids_sha

decisions = {}
with gzip.open(p('b6/u2/decisions.jsonl.gz'), 'rt') as f:
    for line in f:
        r = json.loads(line)
        decisions[r['id']] = r
assert len(decisions) == 3502

# ---------------------------------------------------------------- the read sources (ids with a recorded verdict)
sources = []


def add_source(name, rel, ids, what, verdicts=None, extra=None):
    ids = sorted(set(int(i) for i in ids))
    ent = {'name': name, 'file': rel, 'sha256': (sha_file(p(rel)) if rel and os.path.exists(p(rel)) else None),
           'what': what, 'n': len(ids), 'ids': ids}
    if verdicts is not None:
        tally = {}
        for v in verdicts.values():
            tally[v] = tally.get(v, 0) + 1
        ent['verdict_tally'] = dict(sorted(tally.items()))
    if extra:
        ent.update(extra)
    sources.append(ent)
    return ids


# 1. the post-drain bounded read = B3's "labelled 279" (229 band + 50 uniform), wf_dc85fa7e-c00
pdr = load('postdrain/read_result.json')
pd_v = {int(k): v['verdict'] for k, v in pdr['final'].items()}
assert len(pd_v) == 279 and not pdr['missing']
add_source('postdrain_279', 'postdrain/read_result.json', pd_v,
           "post-drain bounded read (2026-10-17, wf_dc85fa7e-c00): 229 derived awards heads >= EUR 100 m + the "
           "uniform 50 of project 2575's path-W changes; B3's 'labelled 279' and unit 4b's labels", pd_v)

# 2. unit 4b M2 (wf_88da8542-c46): the fresh 30 of M1's walk + the 15 band heads re-read
m2 = load('unit4b/M2-result.json')
m2_v = {}
m2_fresh, m2_band = [], []
for g in m2['groups']:
    for it in g['items']:
        m2_v[it['id']] = it['final']
        (m2_band if g['group'].startswith('band') else m2_fresh).append(it['id'])
assert len(m2_fresh) == 30 and len(m2_band) == 15
add_source('unit4b_M2_fresh30_band15', 'unit4b/M2-result.json', m2_v,
           "unit 4b M2 read (2026-10-17): M1's walk's 30 v2 ships (27 R / 3 W) + the 15 band heads re-read (15 R)",
           m2_v, {'fresh30': sorted(m2_fresh), 'band15': sorted(m2_band)})

# 3. unit 4b M2 fallback "vouched everywhere" (wf_03469a70-c1b): 25 new vouched ships + the 5 reused from M2
fb = load('unit4b/M2-fallback-result.json')
fb_v = {}
for g in fb['groups']:
    for it in g['items']:
        fb_v[it['id']] = it['final']
for i in fb['reused_right']:
    fb_v.setdefault(i, 'right')
assert len(fb_v) == 30
add_source('unit4b_M2_fallback30', 'unit4b/M2-fallback-result.json', fb_v,
           "unit 4b M2 fallback read (2026-10-17): 25 new vouched ships (24 R / 1 W, 5816379) + 5 reused from M2",
           fb_v)

# 4. B3 read18 (wf_24a65e6a-e63): the 18 post-drain new ships
r18 = load('b3/read18/read18.json')
r18_v = {h['id']: h['final_verdict'] for h in r18['heads']}
assert len(r18_v) == 18
add_source('b3_read18', 'b3/read18/read18.json', r18_v,
           "B3 read18 (2026-10-18): the 18 post-drain new ships, 18/18 right (5 band heads)", r18_v)

# 5. B5 investigation read (wf_eecbe1b7-bc7): the 61 = confirm30 + band9 + outside22 (outside30 = outside22 + 8)
inv = load('b5/investigate/read/read.json')
inv_v = {h['id']: h['final_verdict'] for h in inv['heads']}
assert len(inv_v) == 61
summ = inv['summary']
add_source('b5_investigation_61', 'b5/investigate/read/read.json', inv_v,
           "B5 investigation read (2026-10-19): confirm30 (28 R / 1 N / 1 W), band9 (9 R), outside30 (= outside22 + "
           "the 8 confirm30 ids outside 2575's population; 29 R / 1 W)", inv_v,
           {'confirm30': sorted(summ['A_confirm30']['ids']), 'band9': sorted(summ['B_band9']['ids']),
            'outside30': sorted(summ['C_outside30']['ids'])})
assert set(summ['A_confirm30']['ids']) | set(summ['B_band9']['ids']) | set(summ['C_outside30']['ids']) == set(inv_v)

# 5b. the confirm30 draw itself (B5 step 6) and the 61 items file: both must sit inside the 61
cr = load('b5/confirm/confirm_result.json')
ri = load('b5/investigate/read_items.json')
ri_ids = {it['id'] for it in ri['items']}
assert ri_ids == set(inv_v), 'read_items != read heads'
cr_ids = set(cr['draw']['held_ids'])
assert len(cr_ids) == 30 and cr_ids == set(summ['A_confirm30']['ids']), 'confirm draw != the 61\'s confirm30'
cr_band_outside = {r['id'] if isinstance(r, dict) else int(r) for r in cr['band']['outside_ref20_band']}
assert cr_band_outside == set(summ['B_band9']['ids']), (sorted(cr_band_outside), summ['B_band9']['ids'])
cr_ids_in_61 = sorted(cr_ids & set(inv_v))
add_source('b5_confirm30_draw', 'b5/confirm/confirm_result.json', set(summ['A_confirm30']['ids']),
           "B5 step 6 confirmation draw (salt 551-b5-confirm-); its 30 are read in the 61 (verdicts there)",
           None, {'draw_ids_found_in_file': len(cr_ids), 'draw_ids_inside_61': len(cr_ids_in_61),
                  'band_rows_outside_ref20_inside_61': len(cr_band_outside & set(inv_v))})

# 6. B6 U4 guard-cost read (wf_e231858e-cd0): R_single's first 30 + R_excess {7202436} + 7600895 (recorded apart)
rs = load('b6/u4/read_set.json')
u4_ids = list(rs['sets']['R_single']['ids']) + list(rs['sets']['R_excess']['ids']) + \
    list(rs['sets']['cover_overlap_extra']['ids'])
assert len(set(u4_ids)) == 32
u4_v = {i: 'right' for i in u4_ids}
for i in u4['G_single']['non_right_ids'] + u4['G_cover']['R_excess']['non_right_ids']:
    u4_v[i] = 'wrong'
for i, v in u4['cover_overlap_extra_record_only']['verdicts'].items():
    u4_v[int(i)] = v
add_source('b6_u4_32', 'b6/u4/read_set.json', u4_v,
           "B6 U4 guard-cost read (2026-10-19): R_single's first 30 by 551-b6-gcost- (27 R / 3 W), R_excess "
           "7202436 (W), 7600895 (R, recorded separately); verdicts in u4-result.json / read_*.md", u4_v)

# 7. unit 2's 88 labels (panel-result.json) and unit 1's skeptic hand classification of the same 88
pr = load('unit2/panel-result.json')
lab_v = {l['id']: l['best'] for l in pr['labels']}
assert len(lab_v) == 88
add_source('unit2_labels_88', 'unit2/panel-result.json', lab_v,
           "unit 2 (2026-10-16, wf_3d72c491-993): the 88 class members read with dates, label = best served value",
           lab_v)
c2 = load('skeptic/classify2.json')
c2_v = {x['id']: x['cls'] for x in c2}
add_source('unit1_skeptic_classify2_88', 'skeptic/classify2.json', c2_v,
           "unit 1 (2026-10-16): the reading skeptic's hand classification of all 88 class members", c2_v)

# 8. unit 2's out-of-sample reads, recorded in the issue text (Unit 2 section) with their judgements; the 8 API
#    bodies under unit3/data/api are those reads' copies
unit2_text = {
    467176: 'right (holdout fire)', 294968: 'right (holdout fire)', 7256284: 'right (holdout fire)',
    423618: 'right (holdout fire, read: BT-118 = the lot sum)', 5977060: 'closer (holdout fire)',
    8771589: 'closer (holdout fire)', 4930240: 'farther (holdout fire; must-stay pin)',
    7487018: 'G1 blocked correctly (must-stay pin)', 5625591: 'G3 blocked correctly (must-stay pin)',
    928196: 'path V, right on reads (pin, rule 492)', 8822313: 'x1000 confirmed (farther under A; must-stay pin)',
    4925871: 'sums insured confirmed (farther under A; must-stay pin)',
}
api8 = sorted(int(f.split('.')[0]) for f in os.listdir(p('unit3/data/api')) if f.endswith('.json'))
assert set(api8) <= set(unit2_text), api8
add_source('unit2_issue_text_oos_reads', None, unit2_text,
           "issue 551 '## Unit 2 — decided': the holdout fires and out-of-sample reads with recorded judgements "
           "(unit3/data/api holds the 8 API bodies: %s)" % api8, unit2_text)

# 9. unit 4b's out-of-sample six (px2, the B5 cohort's 6 px2 ids): verdicts are unit 2's (all read)
with tarfile.open(p('unit4b/evidence.tar.gz')) as tf:
    px2 = json.load(io.TextIOWrapper(tf.extractfile('px2/features_all.json')))
px2_ids = [x['id'] for x in px2]
add_source('unit4b_px2_oos6', 'unit4b/evidence.tar.gz', px2_ids,
           "unit 4b's out-of-sample 6 (evidence.tar.gz px2/features_all.json; label fields null there, the reads and "
           "judgements are unit 2's: 585917, 5995030, 467176, 294968 right; 7256284 right; 5977060 closer)")

# 10. the pins (b5/pins.py): 16 path-V, 3 path-W, the Lyon-Turin trio, 9 must-stay — each pinned on a recorded read
pins_src = open(p('b5/pins.py')).read()
pin_ids = set()
for name in ('PATH_V', 'PATH_W', 'TRIO'):
    m = re.search(r'^%s\s*=\s*\{(.*?)\}\s*$' % name, pins_src, re.S | re.M)
    assert m, name
    pin_ids |= {int(x) for x in re.findall(r'(\d{5,8})\s*:', m.group(1))}
m = re.search(r'^MUST_STAY\s*=\s*\[(.*?)\]', pins_src, re.S | re.M)
pin_ids |= {int(x) for x in re.findall(r'\d{5,8}', m.group(1))}
assert len(pin_ids) == 31, len(pin_ids)
add_source('b5_pins_31', 'b5/pins.py', pin_ids,
           "B5's 31 pins (16 path-V, 3 path-W, the Lyon-Turin trio, 9 must-stay), each fixed on unit 2's reads")

# 11. B6 decision point 12's pins (6908248 + the partial sums): all read in read18 or the 61
pt12 = [6908248, 6679375, 7153588, 7398935, 7722197, 6779279, 6850401, 7698260]
add_source('b6_decision_point12_pins', 'b6/decision-b6.md', pt12,
           "B6 decision point 12's pins: 6908248 and the partial sums (each read in read18 or the B5 61)")

excluded = set()
for s in sources:
    excluded |= set(s['ids'])

# ---------------------------------------------------------------- considered, NOT excluded (no recorded verdict)
considered = []


def add_considered(name, rel, ids, why):
    ids = sorted(set(int(i) for i in ids))
    inP = sorted(set(ids) & set(P_ids) - excluded)
    considered.append({'name': name, 'file': rel, 'sha256': (sha_file(p(rel)) if rel else None), 'why_not_excluded': why,
                       'n': len(ids), 'in_P_and_not_excluded_n': len(inP), 'in_P_and_not_excluded': inP})


def walk_ids(rel):
    out = []
    for line in open(p(rel)):
        if line.strip():
            r = json.loads(line)
            if 'id' in r:
                out.append(r['id'])
    return out


walk = walk_ids('unit4b/walk.jsonl')
walk_fb = walk + walk_ids('unit4b/walk_fallback.jsonl')
add_considered('unit4b_M1_walk_54', 'unit4b/walk.jsonl', walk,
               "port decisions of the salted shadow walk; only its 30 ships were read (in M2, excluded above)")
add_considered('unit4b_walk_plus_fallback_walk', 'unit4b/walk_fallback.jsonl', walk_fb,
               "port decisions; only the 25 new vouched ships were read (in the fallback read, excluded above)")
fid = load('unit4b/fidelity/derived.json')
add_considered('unit4b_fidelity_10', 'unit4b/fidelity/derived.json', [int(k) for k in fid],
               "hand re-derivation of the port's facts (fidelity of the port), no right/neutral/wrong verdict")
coh = load('b5/cohort_ids.json')
coh_ids = coh['ids'] if isinstance(coh, dict) else coh
add_considered('b5_cohort_563', 'b5/cohort_ids.json', coh_ids,
               "parity population (port vs Rust), not a read; its read members (279, the px2 six) are excluded above")
st = load('b6/selftest_ids.json')
st_ids = st['ids'] if isinstance(st, dict) and 'ids' in st else (
    [i for v in st.values() if isinstance(v, list) for i in v] if isinstance(st, dict) else st)
add_considered('b6_u1_selftest_70', 'b6/selftest_ids.json', [i for i in st_ids if isinstance(i, int)],
               "U1 self-test ids (port mechanics), not a read")
add_considered('b6_u4_twins_consulted', 'b6/u4/api_requests.log', [6896664, 7499938],
               "twin tenders fetched by U4 readers as context for 402503 / 472292; no verdict recorded on either "
               "(read_4.md: 6896664 'no verdict impact')")
inv_aux = set()
for root, dirs, files in os.walk(p('b5/investigate/served')):
    for fn in files + dirs:
        for m_ in re.finditer(r'(?:tenders?|^t|^sk|^skeptic|^sk_|^skp_|^rdr_|_)(\d{5,8})', fn):
            inv_aux.add(int(m_.group(1)))
inv_aux -= set(inv_v)
add_considered('b5_investigation_context_tenders', 'b5/investigate/read/read.json', inv_aux,
               "tenders fetched by B5 readers/skeptics as context (twins, buyer lists); not among the 61, no verdict")

# ---------------------------------------------------------------- safety scan of the written records
# every 5-7 digit number in issue 551's text and in every .md / .txt record under 551-lot-sum/ (u5 aside) that is a
# P id and not excluded, with context, to check none carries a recorded verdict
KNOWN = {
    6896664: "b6/u4/read_4.md: twin of 402503 consulted as context, 'Note (no verdict impact)'; no verdict",
    280000: "568-r208-lots copy (advchk/recheck): an award amount (aw=280000), not a tender id",
    90476: "568-r208-lots copy: a TED publication number (090476-2015), not a tender id",
    7611313: "568-r208-lots copy: issue 568's read-back parity log ('0 parity breaks'), not a 551 verdict",
    7383539: "568-r208-lots copy: issue 568's read-back parity log ('0 parity breaks'), not a 551 verdict",
}
scan_files = [ISSUE]
for root, dirs, files in os.walk(BASE):
    if os.path.normpath(root).startswith(OUT):
        continue
    for fn in files:
        if fn.endswith(('.md', '.txt')):
            scan_files.append(os.path.join(root, fn))
scan_hits = {}
for path in sorted(scan_files):
    txt = open(path, errors='replace').read()
    for m_ in re.finditer(r'(?<![\d.,])(\d{5,7})(?![\d.,])', txt):
        tid = int(m_.group(1))
        if tid in P_ships and tid not in excluded:
            h = scan_hits.setdefault(tid, {'files': set(), 'context': txt[max(0, m_.start() - 160): m_.end() + 160]
                                           .replace('\n', ' ')})
            h['files'].add(os.path.relpath(path, BASE))
unexplained = sorted(set(scan_hits) - set(KNOWN))
assert not unexplained, unexplained

# ---------------------------------------------------------------- the order
eligible = [i for i in P_ids if i not in excluded]
ordered = sorted(eligible, key=lambda i: (order_key(i), i))
excl_in_P = sorted(set(P_ids) & excluded)
entries = []
for rank, tid in enumerate(ordered, 1):
    s = P_ships[tid]
    d = decisions[tid]
    entries.append({'rank': rank, 'id': tid, 'order_key': order_key(tid), 'S': s['S'], 'currency': s['currency'],
                    'n': s['n'], 'S_eur_cents': s['S_eur_cents'], 'vouched_by': s['vouched_by'],
                    'flags': d.get('flags') or []})

order = {
    'what': "issue 551 B6 U5 (decision point 7): the fresh pre-flip read's order over P minus every read id",
    'salt': SALT,
    'order_rule': "sha256('551-b6-fresh-' + decimal id) hex digest ascending (ties by id; none)",
    'P': {'file': 'b6/u2/P.json', 'guard_set': 'v2+both', 'n': len(P_ids), 'ids_sha256': P_ids_sha},
    'inputs_sha256': inputs,
    'excluded_from_P_n': len(excl_in_P),
    'excluded_from_P': excl_in_P,
    'eligible_n': len(ordered),
    'eligible_ids_sha256': hashlib.sha256(('\n'.join(map(str, sorted(eligible))) + '\n').encode()).hexdigest(),
    'ordered_ids_sha256': hashlib.sha256(('\n'.join(map(str, ordered)) + '\n').encode()).hexdigest(),
    'read_plan': "read the first 30 in this order; 0 wrong -> go; exactly 1 wrong -> read ranks 31-60, 0 wrong -> go; "
                 ">= 2 wrong -> keep off, re-decide (decision point 7)",
    'first_60': ordered[:60],
    'entries': entries,
}
assert len(set(ordered)) == len(ordered) and not (set(ordered) & excluded)
assert len(ordered) + len(excl_in_P) == len(P_ids)

# ---------------------------------------------------------------- the B-band re-read set (decision point 8)
band29 = sorted(load('b6/u2/u2-result.json')['extras_per_guard_set']['v2+both']['B_P_band_eur100m'])
within2 = load('b6/u2/u2-result.json')['extras_per_guard_set']['v2+both']['within_2pct_of_eur100m']
note_input('b6/u2/u2-result.json')
assert len(band29) == 29
P_band = sorted(i for i in P_ids if P_ships[i]['S_eur_cents'] >= EUR100M)
P_near = sorted(i for i in P_ids if abs(P_ships[i]['S_eur_cents'] - EUR100M) <= EUR100M * 0.02)
assert P_band == band29, (P_band, band29)

# the last read of each band head and the S / currency / n / vouched_by it read
m1pc = load('unit4b/M1-port-check.json')
note_input('unit4b/M1-port-check.json')
b3port = {}
with gzip.open(p('b3/results/api_port.jsonl.gz'), 'rt') as f:
    for line in f:
        r = json.loads(line)
        if r['id'] in band29:
            b3port[r['id']] = r
note_input('b3/results/api_port.jsonl.gz')
inv_heads = {h['id']: h for h in inv['heads']}
r18_heads = {h['id']: h for h in r18['heads']}


def vb(v):
    return None if not v else {'field': v['field'], 'notice_id': v['notice_id'], 'cents': v['cents']}


band_rows = []
for tid in band29:
    pred = P_ships[tid]
    pv = {'S': pred['S'], 'currency': pred['currency'], 'n': pred['n'], 'vouched_by': vb(pred['vouched_by']),
          'S_eur_cents': pred['S_eur_cents']}
    if tid in inv_heads and tid in summ['B_band9']['ids']:
        h = inv_heads[tid]
        last = {'read': 'B5 investigation read, band9 (2026-10-19, wf_eecbe1b7-bc7)',
                'basis': 'b5/investigate/read/read.json heads[] (served snapshot, read_items.json)',
                'verdict': h['final_verdict'], 'class': h['final_class'],
                'S': h['S_cents'], 'currency': h['currency'], 'n': h['n'], 'vouched_by': vb(h['vouched_by'])}
    elif tid in r18_heads:
        h = r18_heads[tid]
        b = b3port[tid]
        assert (b['S'], b['cur'], b['n']) == (h['S_cents'], h['currency'], h['n']) and \
            b['vouched_by']['field'] == h['vouched_by']
        last = {'read': 'B3 read18 (2026-10-18, wf_24a65e6a-e63)',
                'basis': 'b3/read18/read18.json heads[] + b3/results/api_port.jsonl.gz (the port output it read)',
                'verdict': h['final_verdict'], 'class': h['final_class'],
                'S': b['S'], 'currency': b['cur'], 'n': b['n'], 'vouched_by': vb(b['vouched_by'])}
    else:
        assert tid in m2_band, tid
        h = m1pc[str(tid)]
        last = {'read': 'unit 4b M2 band re-read (2026-10-17, wf_88da8542-c46)',
                'basis': 'unit4b/M1-port-check.json (the v2 port output M2 read)',
                'verdict': m2_v[tid], 'class': None,
                'S': h['S'], 'currency': h['cur'], 'n': h['n'], 'vouched_by': vb(h['vouched_by'])}
        b = b3port.get(tid)
        if b:
            last['b3_port_2026_10_18'] = {'S': b['S'], 'currency': b['cur'], 'n': b['n'],
                                          'vouched_by': vb(b['vouched_by'])}
    changed = []
    for k in ('S', 'currency', 'n', 'vouched_by'):
        if pv[k] != last[k]:
            changed.append(k)
    band_rows.append({'id': tid, 'predicted_v2_both': pv, 'last_read': last, 'changed': changed,
                      'reread': bool(changed),
                      'within_2pct_of_eur100m': tid in within2})

# L_changed and the delta: band rows among them
lch = load('b6/u2/l_changed_provisional.json')
note_input('b6/u2/l_changed_provisional.json')


lch_keys = ('l_changed_provisional', 'in_3502', 'criterion_a_ids', 'criterion_b_ids',
            'ids_of_3502_with_any_non2630_row_up_to_head', 'set_541_2632', 'set_545', 'set_571')
lch_sets = {'b6/u2/l_changed_provisional.json:' + k: set(lch[k]) for k in lch_keys}
assert lch['n_in_3502'] == 0 and not lch['in_3502']
for rel in ('b6/l_changed_545_ids.json', 'b6/l_changed_571_ids.json'):
    note_input(rel)
    lch_sets[rel] = set(load(rel))
lch_all = set().union(*lch_sets.values())
band_from_lchanged = sorted(lch_all & set(band29))
P_from_lchanged = sorted(lch_all & set(P_ids))

reread = sorted(r['id'] for r in band_rows if r['reread'])
band_set = sorted(set(reread) | set(band_from_lchanged))
band = {
    'what': "issue 551 B6 U5 (decision point 8): the B-band re-read set",
    'rule': "re-read any of the 29 band heads (B_P) whose predicted S, n or vouched_by changed vs its last read, and "
            "every band row from L_changed or the delta (0 wrong of any class)",
    'B_P': band29, 'P_band_eur100m_recomputed': P_band, 'within_2pct_of_eur100m': within2,
    'within_2pct_recomputed_from_P': P_near,
    'within_2pct_in_B_P': sorted(set(within2) & set(band29)),
    'within_2pct_outside_B_P': sorted(set(within2) - set(band29)),
    'compare': "strict equality of (S, currency, n) and vouched_by {field, notice_id, cents}: P (v2+both) vs the "
               "figures the head's last recorded read judged",
    'rows': band_rows,
    'changed_ids': reread,
    'l_changed': {'sets': {k: len(v) for k, v in sorted(lch_sets.items())},
                  'ids_found': len(lch_all), 'band_rows_in_l_changed': band_from_lchanged,
                  'P_in_l_changed': P_from_lchanged,
                  'u2_record': "provisional L_changed ∩ 3,502 = 0 (2630-excluded reading, binding for U6); 541/571 "
                               "carriers 0, 545's 48 none"},
    'delta': "none yet: the delta (salt 551-b6-delta-) is the new fires after the drain; it does not exist pre-flip",
    'band_reread_set': band_set,
}

# ---------------------------------------------------------------- write
excl = {
    'what': "issue 551 B6 U5: the exclusion set 'every read id' (every tender id with a recorded verdict in 551's "
            "reads) and its sources",
    'built_by': 'b6/u5/build_order.py',
    'inputs_sha256': inputs,
    'sources': sources,
    'union_n': len(excluded),
    'union_ids': sorted(excluded),
    'union_in_P_n': len(excl_in_P),
    'union_in_P': excl_in_P,
    'per_source_in_P': {s['name']: sorted(set(s['ids']) & set(P_ids)) for s in sources},
    'considered_not_excluded': considered,
    'record_text_scan': {
        'what': "every 5-7 digit number in issue 551's text and in every .md/.txt record under 551-lot-sum/ (u5 "
                "aside) that is a P id and not excluded; each explained, none carries a 551 verdict",
        'files_scanned': len(scan_files),
        'note': "os.walk does not follow the symlinks under b6/u2/advchk/L and b6/u2/recheck/L (links to the "
                "551/568/570 folders); a glob that followed them also hit 280000, 90476, 7611313, 7383539 in issue "
                "568's copies (amounts, a publication number, 568 parity logs), explained in build_order.py KNOWN",
        'hits': {str(k): {'files': sorted(v['files']), 'explained': KNOWN[k], 'context': v['context'],
                          'rank_in_order': ordered.index(k) + 1} for k, v in sorted(scan_hits.items())}},
}
for name, obj in (('order.json', order), ('excluded_sources.json', excl), ('band_set.json', band)):
    with open(os.path.join(OUT, name), 'w') as f:
        json.dump(obj, f, indent=1, ensure_ascii=False)
        f.write('\n')

print('P', len(P_ids), 'excluded union', len(excluded), 'in P', len(excl_in_P), 'eligible', len(ordered))
for s in sources:
    print('  %-32s n=%4d inP=%3d' % (s['name'], s['n'], len(set(s['ids']) & set(P_ids))))
for c in considered:
    print('  considered %-28s n=%4d inP_unexcluded=%d %s' % (c['name'], c['n'], c['in_P_and_not_excluded_n'],
                                                            c['in_P_and_not_excluded'][:12]))
print('record-scan P ids not excluded:', {k: ordered.index(k) + 1 for k in sorted(scan_hits)})
print('first60', ordered[:60])
print('band changed', reread, 'band from L_changed', band_from_lchanged, 'P in L_changed', P_from_lchanged)
for r in band_rows:
    if r['changed']:
        print('  ', r['id'], r['changed'], r['predicted_v2_both'], r['last_read'])
print('P near 100m', P_near, 'within2 file', within2)
