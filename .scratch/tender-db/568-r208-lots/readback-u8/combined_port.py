"""568 unit 8 + 570: the expected served lot of every lot_result after the ONE combined refold.

The read-back bar (u7/decision-u7.md "Amendment (unit 8, 2026-10-18)", "The read-back bar", (i) PARITY) compares
every served block with a replay that composes issue 570's remap (option B, 570-lot-keys/decision.md K1-K4) with
issue 568's match (rule steps 1-9 + G1 / G2, G2 read at the remapped key) and the amended refusal guards G3-G6
(FINAL_AMENDED, R1 + R2), in the order crates/ingest/src/project.rs `fold` runs them per notice of a chain:

  1. remap  (fold: `legacy_lot_remap(&state.lots, &lots)`, only for a legacy notice while no non-legacy notice of
            the chain has published a lot section, K3) ........ remap_port.remap, wrapped here with K3 and the carry;
  2. carry  (each lot section lands on its target key: kind, published_lot_no, facts superseded there; G2
            `lot_renumbered` compared at the target) ......... g3_port.replay's own carry (cross_notice_port.carry,
            notice_text_facts), fed the notice with its lot sections RENAMED to their targets (see below);
  3. round  (567's same-notice keys follow their section to the target, `remap_lot_key`) ... remap_port.same_notice_map
            mapped through the notice's target map, exactly remap_port.fold's line for it;
  4. match  (`if state.lots.is_empty() && !lots_renumbered { match_inherited_lots(..) }`: G1, lookup, then
            `unit7_refusals`) .................................. g3_port.replay (cross_notice_port.match_notice) then
            g3_port.apply_amended() with its defaults (FINAL_AMENDED = R1 + R2, the fold's text sides).

Nothing of the three ports' logic is forked. The composition needs two things none of them has, both done here as
wrappers, never as edited copies:
  - The rename. g3_port.replay keys its carry by each notice's own section ids. Under 570 a remapped section is
    carried under its TARGET key, and G2 compares there. Renaming the notice's lot sections (section_id and every
    parent_section_id pointing at them) to their targets before replay() makes replay()'s carry, its lot text facts
    and its G2 exactly the remapped carry. K1 makes the rename injective (asserted); a target is always a carried key
    or the section's own id (step 9). The notice's award blocks keep their RES ids, and a remapped notice has lot
    sections, so 568 never matches its blocks; their keys come from step 3 on the ORIGINAL content.
  - K3 and the legacy test. remap_port.fold (the 570 port) has no K3 and tests legacy by a `TED-` field; fold's
    `NoticeState.legacy` is `is_legacy_profile` (text / internal-ojs / ted-export*). The content endpoint carries no
    profile, so `is_legacy()` below reads `TED-` or `TXT-` (the text profile's fields) and remap_chain() keeps K3's
    flag (set AFTER the notice's own map, as fold does). On every cached chain both agree with the port (all notices
    are TED-only), which `port_570_check()` confirms against 570-lot-keys/port_result.json.
A non-legacy notice's results (eForms / FTS) are outside 567 / 568 / 570: their key is the result's own BT-13713,
reported with reason `non-legacy` (no cached chain has one).

API:
  expected(tender, contents, remap=True, guards=True) -> {(notice_id, result key): row}
      row = {key, notice_id, expected_lot, reason, printed, notice_remap}; tender = /v1/tenders/{id},
      contents = {notice_id: /v1/notices/{id}/content}. remap=False / guards=False give the attribution variants.
  attribute(tender, contents) -> per block: expected (remap+guards), base (neither: the pre-drain rule), remap-only,
      guards-only, and `cause` in {unchanged, 570, 568-u8, both}.

Usage (the run over every cached Tender): python3 -I -B combined_port.py [--scratch DIR]
  DIR (default: this session's scratchpad rb8/) must hold the extracted caches:
    c570/   570-lot-keys/raw-cache.tar.gz          c567/  567-lot-mapping/readback/raw-api-cache.tar.gz (570's exhibits)
    fresh/  568-r208-lots/readback/fresh-cache.tar.gz
  plus, read in place: u7/u7-cache.jsonl.gz and u8-oos/cache_o1, cache_o2.
  Writes expected.json ({tender_id: [{key, notice_id, expected_lot, reason}]}, every source merged, the latest
  cache of a Tender winning) and combined_run.json (per source and block: served pre-drain, base, variants, cause,
  and the named checks) next to this file.
"""
import collections
import copy
import gzip
import json
import os
import re
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)                       # 568-r208-lots
U7 = os.path.join(ROOT, 'u7')
K570 = os.path.join(os.path.dirname(ROOT), '570-lot-keys')
for p in (K570, ROOT, U7):
    if p not in sys.path:
        sys.path.insert(0, p)
import g3_port as G             # noqa: E402  568 replay + FINAL_AMENDED guards
import cross_notice_port as P   # noqa: E402  568 match
import remap_port as R          # noqa: E402  570 option B

SCRATCH_DEFAULT = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8'


# ------------------------------------------------------------------ the composition
def is_legacy(content):
    """fold's `NoticeState.legacy` (`is_legacy_profile`) read from a content shape that carries no profile."""
    return any(v['field_id'].startswith(('TED-', 'TXT-')) for s in content.get('sections', []) for v in s.get('values', []))


def chain_of(tender):
    return [v['caused_by_notice_id'] for v in sorted(tender['versions'], key=lambda v: v['seq'])]


def remap_chain(chain, contents, remap=True):
    """570 over the chain in fold order: {notice: (target {own key: Tender key}, how)}.

    remap_port.remap is rule steps 1-5 (anchor, targets by LegacyLotNumbers::inherited + lookup, K1, K2, K4); this
    wrapper adds K3 (`non_legacy_lots`, set after the notice's own map) and carries the lots each map reads (step 8:
    only notices folded before it), with remap_port.fold's carry: each section's kind and number land on its target.
    """
    lots, non_legacy_lots, out = {}, False, {}
    for nid in chain:
        c = contents.get(nid)
        if c is None:
            out[nid] = ({}, 'missing')
            continue
        legacy = is_legacy(c)
        secs = R.lot_sections(c)
        own = {s[0]: s[0] for s in secs}
        if not secs:
            target, how = own, 'no-lot-sections'
        elif not remap:
            target, how = own, 'remap-off'
        elif not legacy:
            target, how = own, 'non-legacy'
        elif non_legacy_lots:
            target, how = own, 'K3'
        else:
            target, how = R.remap(secs, lots, True)
        non_legacy_lots |= (not legacy) and bool(secs)
        for key, kind, num, _, _ in secs:
            tk = target[key]
            if tk in lots:
                lots[tk]['kind'], lots[tk]['num'] = kind, num
            else:
                lots[tk] = {'kind': kind, 'num': num}
        out[nid] = (target, how)
    return out


def renamed(content, target):
    """The notice with its lot sections renamed to their 570 targets (section_id and parent_section_id)."""
    moves = {k: v for k, v in target.items() if k != v}
    if not moves:
        return content
    c = copy.deepcopy(content)
    new_ids = [moves.get(s['section_id'], s['section_id']) for s in c['sections']]
    assert len(set(new_ids)) == len(new_ids), ('a 570 target collides with a section id', content.get('notice_id'), moves)
    for s in c['sections']:
        s['section_id'] = moves.get(s['section_id'], s['section_id'])
        if s.get('parent_section_id') in moves:
            s['parent_section_id'] = moves[s['parent_section_id']]
    return c


def non_legacy_results(content):
    """eForms / FTS: each LotResult's own BT-13713 (read_results' generic path), {result key: (lot, printed)}."""
    secs = P.sections_of(content)
    out = {sid: [] for sid, s in secs.items() if s['kind'] == 'LotResult'}
    for sid in sorted(secs):
        owner = P.enclosing(secs, sid, P.RESULT_KINDS)
        if owner not in out:
            continue
        for v in sorted(secs[sid].get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0))):
            if v.get('type') == 'id' and v['field_id'].split('-', 2)[:2] == ['BT', '13713']:
                out[owner].append(v['value'])
    return {k: (v[-1] if v else None, v[-1] if v else None) for k, v in out.items()}


def expected(tender, contents, remap=True, guards=True):
    """{(notice_id, result key): row} for every award block of the chain, as the combined fold serves it."""
    tid = tender['id']
    chain = chain_of(tender)
    maps = remap_chain(chain, contents, remap)
    present = [n for n in chain if n in contents]
    rcontents = {n: renamed(contents[n], maps[n][0]) for n in present}
    t2 = dict(tender, versions=[v for v in tender['versions'] if v['caused_by_notice_id'] in contents])
    blocks = G.replay(tid, t2, rcontents)
    by_notice = collections.defaultdict(list)
    for b in blocks:
        by_notice[b['can_notice_id']].append(b)
    if guards:
        for bl in by_notice.values():
            G.apply_amended(bl)  # FINAL_AMENDED (R1 + R2) -> b['lot_u8'], b['guards8']
    out = {}
    for n in chain:
        if n not in contents:
            continue
        c = contents[n]
        target, how = maps[n]
        moved = {k: v for k, v in target.items() if k != v}
        nr = how if not moved else 'remapped ' + ','.join(f'{k}->{v}' for k, v in sorted(moved.items()))
        if not is_legacy(c):
            for key, (lot, printed) in non_legacy_results(c).items():
                out[(n, key)] = {'key': key, 'notice_id': n, 'expected_lot': lot, 'printed': printed,
                                 'reason': 'non-legacy: BT-13713 (outside 567/568/570)', 'notice_remap': nr}
            continue
        if R.lot_sections(c):
            # 567's same-notice key, following its section to the 570 target (fold step 6, remap_lot_key).
            for key, (lot, printed) in sorted(R.same_notice_map(c).items()):
                tk = target.get(lot, lot) if lot else None
                if lot is None:
                    why = '567 same-notice: no lot'
                elif tk != lot:
                    why = f'567 same-notice {lot} + 570 remap -> {tk}'
                else:
                    why = f'567 same-notice {lot}'
                out[(n, key)] = {'key': key, 'notice_id': n, 'expected_lot': tk, 'printed': printed, 'reason': why,
                                 'notice_remap': nr}
            continue
        for b in by_notice.get(n, []):
            lot = b['lot_u8'] if guards else b['lot_568']
            if b['lot_568'] is None:
                why = f"568 none: {b['why_568']}"
            elif guards and b['guards8']:
                why = f"568 refused {'+'.join(b['guards8'])} (tentative {b['lot_568']})"
            else:
                why = f"568 match ({b['why_568']})"
            out[(n, b['result_key'])] = {'key': b['result_key'], 'notice_id': n, 'expected_lot': lot,
                                         'printed': b['printed'], 'reason': why, 'notice_remap': nr}
    return out


def attribute(tender, contents):
    """Per block: the combined expectation and what moved it relative to the pre-drain rule (no remap, no guards)."""
    rg = expected(tender, contents, True, True)
    base = expected(tender, contents, False, False)
    r = expected(tender, contents, True, False)
    g = expected(tender, contents, False, True)
    rows = {}
    for k in rg:
        e, b0 = rg[k]['expected_lot'], base.get(k, {}).get('expected_lot')
        rr, gg = r.get(k, {}).get('expected_lot'), g.get(k, {}).get('expected_lot')
        if e == b0:
            cause = 'unchanged'
        elif rr != b0 and gg == b0 and e == rr:
            cause = '570'
        elif gg != b0 and rr == b0 and e == gg:
            cause = '568-u8'
        else:
            cause = 'both'  # each alone moves it, or only the two together do
        rows[k] = dict(rg[k], base_lot=b0, base_reason=base.get(k, {}).get('reason', ''), remap_only_lot=rr,
                       guards_only_lot=gg, cause=cause)
    return rows


# ------------------------------------------------------------------ caches
def _ld(p):
    with open(p, encoding='utf-8') as fh:
        return json.load(fh)


class Cache:
    """Tenders and notice contents from one or more raw-read directories (first file found wins, in dir order)."""
    T = [re.compile(r'(?:v1_)?tenders?_(\d+)\.json'), re.compile(r't(\d+)\.json')]
    N = [re.compile(r'(?:v1_)?notices?_(\d+)(?:_content)?\.json'), re.compile(r'n(\d+)\.json')]

    def __init__(self, dirs):
        self.tenders, self.notices = {}, {}
        for d in dirs:
            for root, subdirs, files in os.walk(d):
                subdirs.sort()
                for f in sorted(files):
                    p = os.path.join(root, f)
                    m = next((m for m in (rx.fullmatch(f) for rx in self.T) if m), None)
                    if m:
                        self.tenders.setdefault(int(m.group(1)), p)
                        continue
                    m = next((m for m in (rx.fullmatch(f) for rx in self.N) if m), None)
                    if m:
                        self.notices.setdefault(int(m.group(1)), p)

    def tender(self, tid):
        return _ld(self.tenders[tid])

    def contents(self, tender):
        return {n: _ld(self.notices[n]) for n in chain_of(tender) if n in self.notices}


class BundleCache:
    """u7/u7-cache.jsonl.gz: the s150 / f30 / m77 Tenders and every chain notice."""

    def __init__(self, path):
        self.t, self.n, self.sets = {}, {}, collections.defaultdict(list)
        with gzip.open(path, 'rt', encoding='utf-8') as fh:
            for line in fh:
                r = json.loads(line)
                if r['kind'] == 'tender':
                    self.t[r['id']] = r['data']
                    self.sets[r['set']].append(r['id'])
                else:
                    self.n[r['id']] = r['data']
        self.tenders = self.t

    def tender(self, tid):
        return self.t[tid]

    def contents(self, tender):
        return {n: self.n[n] for n in chain_of(tender) if n in self.n}


# ------------------------------------------------------------------ the run
def run_tender(cache, tid, overlay=None):
    """overlay: {(notice, key): lot} read later than the cache (m77: readback/measured77.json, after 568's drain),
    which replaces the cache's served lot as the pre-drain state."""
    t = cache.tender(tid)
    contents = cache.contents(t)
    missing = [n for n in chain_of(t) if n not in contents]
    rows = attribute(t, contents)
    served = {(r['notice_id'], r['key']): r.get('lot') for r in t.get('lot_results', [])}
    src = {k: 'cache' for k in served}
    for k, lot in (overlay or {}).items():
        served[k], src[k] = lot, 'overlay'
    out = []
    for k, row in sorted(rows.items(), key=lambda kv: (kv[0][0], P.res_order(kv[0][1]) if '-' in kv[0][1] else 0, kv[0][1])):
        row = dict(row, served_pre=served.get(k, '<absent>'), served_pre_from=src.get(k))
        out.append(row)
    for k in sorted(set(served) - set(rows)):
        out.append({'key': k[1], 'notice_id': k[0], 'expected_lot': None, 'printed': None,
                    'reason': 'served block the replay does not produce (notice missing or not in the chain)',
                    'notice_remap': None, 'base_lot': None, 'base_reason': '', 'remap_only_lot': None,
                    'guards_only_lot': None, 'cause': 'unmodelled', 'served_pre': served[k], 'served_pre_from': src[k]})
    return {'tender_id': tid, 'chain': chain_of(t), 'missing_notices': missing, 'blocks': out,
            'published_at': t.get('published_at')}


def drift_kind(b):
    """Why a cache's served lot is not the pre-drain rule's (base): the cache predates 568's own drain (2593/2594),
    so an R2.0.8 block it now places was still served null; anything else is unexplained."""
    if b['served_pre'] is None and b['base_lot'] is not None and b['reason'].startswith('568'):
        return 'cache-predates-568-drain'
    return 'unexplained'


def summarise(results):
    """Blocks whose combined expectation differs from the served lot the cache recorded, by cause:
    570 / 568-u8 / both when the cache serves the pre-drain rule (base); `cache-predates-568-drain:<cause>` when the
    cache was read before 568's own drain (its expected change then includes 568's placement, attributed by what
    moves base -> expected; `...:unchanged` = only 568's earlier drain moves it)."""
    c = collections.Counter()
    changed = collections.Counter()
    drift, g2_narrowed = [], []
    for r in results:
        for b in r['blocks']:
            c['blocks'] += 1
            if b['served_pre'] == '<absent>':
                c['computed_not_served'] += 1
                continue
            if b['cause'] == 'unmodelled':
                c['unmodelled'] += 1
                continue
            if b['base_lot'] != b['served_pre']:
                drift.append(f"{r['tender_id']}/{b['notice_id']}/{b['key']}: served {b['served_pre']} base {b['base_lot']} ({drift_kind(b)})")
                c['served_pre_differs_from_base:' + drift_kind(b)] += 1
            if b['expected_lot'] != b['served_pre']:
                c['changed_vs_served_pre'] += 1
                key = b['cause'] if b['base_lot'] == b['served_pre'] else drift_kind(b) + ':' + b['cause']
                changed[key] += 1
            if b['cause'] != 'unchanged':
                c['changed_vs_base:' + b['cause']] += 1
            if b['base_reason'].startswith('568 none: g2') and b['expected_lot'] is not None:
                g2_narrowed.append(f"{r['tender_id']}/{b['notice_id']}/{b['key']} -> {b['expected_lot']}")
    return {'counts': dict(c), 'changed_vs_served_pre_by_cause': dict(changed),
            'newly_matched_because_570_narrows_g2': g2_narrowed, 'served_pre_vs_base_drift': drift,
            'tenders': len(results), 'tenders_with_missing_notices': [r['tender_id'] for r in results if r['missing_notices']]}


def checks_u7(results, measurement_u8):
    """8327340, the s150's 57 refused (decision-u7.md amendment, read-back (c)), the m77 unchanged."""
    by = {(r['tender_id'], b['notice_id'], b['key']): b for r in results for b in r['blocks']}
    out = {}
    f = {f"{n}/{k}": by[(8327340, n, k)]['expected_lot'] for (t, n, k) in by if t == 8327340}
    want_null = [(17821481, f'RES-{i}') for i in (1, 2, 5, 6, 7)] + [(17830774, 'RES-1'), (17830774, 'RES-2')]
    want = {(17821481, 'RES-3'): 'LOT-24', (17821481, 'RES-4'): 'LOT-32', (17840150, 'RES-1'): 'LOT-2'}
    out['8327340'] = {
        'blocks': f,
        'seven_null': all(by[(8327340, n, k)]['expected_lot'] is None for n, k in want_null),
        'named_kept': {f'{n}/{k}': [by[(8327340, n, k)]['expected_lot'], v] for (n, k), v in want.items()},
        'ok': all(by[(8327340, n, k)]['expected_lot'] is None for n, k in want_null)
        and all(by[(8327340, n, k)]['expected_lot'] == v for (n, k), v in want.items()),
    }
    m8 = _ld(measurement_u8)
    s150_refused = [(b['tender_id'], b['can_notice_id'], b['result_key']) for b in m8['blocks']
                    if b['set'] == 's150' and b['guards8']]
    not_null = [f'{t}/{n}/{k}: {by[(t, n, k)]["expected_lot"]}' for t, n, k in s150_refused
                if by.get((t, n, k), {}).get('expected_lot', 'absent') is not None]
    out['s150_refused'] = {'n': len(s150_refused), 'not_null': not_null, 'ok': len(s150_refused) == 57 and not not_null,
                           'per_tender': dict(collections.Counter(t for t, _, _ in s150_refused))}
    # measurement-u8's FINAL_AMENDED lot per matched block vs. this replay (with the remap: any difference is 570's)
    diffs = []
    for b in m8['blocks']:
        k = (b['tender_id'], b['can_notice_id'], b['result_key'])
        e = by.get(k)
        if e is None:
            diffs.append(f'{k}: absent')
        elif e['expected_lot'] != b['lot_u8']:
            diffs.append(f"{b['set']}:{k}: measurement-u8 {b['lot_u8']} vs combined {e['expected_lot']} ({e['cause']}, {e['notice_remap']})")
    out['measurement_u8_FINAL_AMENDED_vs_combined'] = {'blocks': len(m8['blocks']), 'differ': diffs}
    return out


def checks_oos(results, cache_dirs, draw):
    """The OOS 50: g3_port's own FINAL_AMENDED (amended_oos.py's replay) refuses 100 blocks; all must be null here."""
    by = {(r['tender_id'], b['notice_id'], b['key']): b for r in results for b in r['blocks']}
    cache = Cache(cache_dirs)
    refused, kept_diff = [], []
    for tid in draw:
        t = cache.tender(tid)
        bl = G.replay(tid, t, cache.contents(t))
        byn = collections.defaultdict(list)
        for b in bl:
            byn[b['can_notice_id']].append(b)
        for x in byn.values():
            G.apply_amended(x)
        for b in bl:
            k = (tid, b['can_notice_id'], b['result_key'])
            if b['lot_568'] is not None and b['guards8']:
                refused.append(k)
            if b['why_568'] != 'can_has_own_lots' and b['lot_u8'] != by[k]['expected_lot']:
                kept_diff.append(f"{k}: g3_port {b['lot_u8']} vs combined {by[k]['expected_lot']} ({by[k]['cause']})")
    not_null = [f'{k}: {by[k]["expected_lot"]}' for k in refused if by[k]['expected_lot'] is not None]
    return {'refused_by_g3_port': len(refused), 'not_null_in_combined': not_null,
            'ok': len(refused) == 100 and not not_null,
            'per_tender': dict(collections.Counter(k[0] for k in refused)),
            'g3_port_vs_combined_differ': kept_diff}


def port_570_check(results_570, port_result, cache):
    """570-lot-keys/port_result.json: per notice B outcome, moved sections, and every B lot (remap_port.fold 'B',
    the reference, re-run on the same files) against the composition with the guards off; then what the guards
    change on top."""
    pr = _ld(port_result)
    rows = {r['tender_id']: r for r in pr['sample'] + pr['exhibits']}
    outcome_diff, moved_diff, block_diff, guard_moves, moved_ok = [], [], [], [], 0
    by = {r['tender_id']: r for r in results_570}
    for tid, row in rows.items():
        t = cache.tender(tid)
        contents = cache.contents(t)
        maps = remap_chain(chain_of(t), contents, True)
        mine = {str(n): how for n, (tg, how) in maps.items() if n in contents and R.lot_sections(contents[n])}
        for n, how in row['B_notice_outcomes'].items():
            if mine.get(n) != how:
                outcome_diff.append(f'{tid}/{n}: port {how} vs combined {mine.get(n)}')
        mv = {str(n): {k: v for k, v in tg.items() if k != v} for n, (tg, how) in maps.items()}
        mv = {n: m for n, m in mv.items() if m}
        if mv != row['B_moved_sections']:
            moved_diff.append(f"{tid}: port {row['B_moved_sections']} vs combined {mv}")
        _, res_b, _ = R.fold(chain_of(t), contents, 'B')
        nog = expected(t, contents, True, False)
        for k, (lot, _) in res_b.items():
            if nog.get(k, {}).get('expected_lot', '<absent>') != lot:
                block_diff.append(f"{tid}/{k}: remap_port B {lot} vs combined(no guards) {nog.get(k, {}).get('expected_lot', '<absent>')}")
        for m in row['results_moved_by_B']:
            k = (m['notice'], m['block'])
            if nog.get(k, {}).get('expected_lot') == m['B']:
                moved_ok += 1
        for b in by[tid]['blocks']:
            k = (b['notice_id'], b['key'])
            if k in res_b and b['expected_lot'] != res_b[k][0]:
                guard_moves.append(f"{tid}/{b['notice_id']}/{b['key']}: B {res_b[k][0]} -> {b['expected_lot']} ({b['reason']})")
    return {'tenders': len(rows), 'notice_outcomes_differ': outcome_diff, 'moved_sections_differ': moved_diff,
            'results_moved_by_B_reproduced': f"{moved_ok}/{sum(len(r['results_moved_by_B']) for r in rows.values())}",
            'every_block_vs_remap_port_B_differ': block_diff,
            'guards_change_B_lot': guard_moves,
            'ok': not outcome_diff and not moved_diff and not block_diff}


def main():
    args = sys.argv[1:]
    scratch = SCRATCH_DEFAULT
    if '--scratch' in args:
        scratch = args[args.index('--scratch') + 1]
    sources = {}
    # u7: s150 / f30 / m77 (the m77's bundle copy predates 568's drain; readback/measured77.json re-read its 692 CAN
    # blocks after it, and is the pre-drain state where it has the block)
    bundle = BundleCache(os.path.join(U7, 'u7-cache.jsonl.gz'))
    for s in ('s150', 'f30', 'm77'):
        sources[s] = (bundle, bundle.sets[s])
    m77_overlay = collections.defaultdict(dict)
    for tid, rows in _ld(os.path.join(ROOT, 'readback', 'measured77.json'))['served'].items():
        for n, k, lot in rows:
            m77_overlay[int(tid)][(n, k)] = lot
    # OOS 50
    oos_dirs = [os.path.join(ROOT, 'u8-oos', 'cache_o1'), os.path.join(ROOT, 'u8-oos', 'cache_o2')]
    if not all(os.path.isdir(d) for d in oos_dirs):
        oos_dirs = [os.path.join(scratch, 'oos', 'cache_o1'), os.path.join(scratch, 'oos', 'cache_o2')]
    draw = [i for g in _ld(os.path.join(ROOT, 'u8-oos', 'draw.json'))['groups'] for i in g['ids']]
    sources['oos'] = (Cache(oos_dirs), draw)
    # 570's 211: the skeptics' live re-fetches first, then cache_k1..12, then the exhibits (remap_port's run order)
    c570 = os.path.join(scratch, 'c570')
    live = sorted(os.path.join(c570, d, 'live') for d in os.listdir(c570) if d.endswith('_skeptic'))
    ks = [os.path.join(c570, f'cache_k{i}') for i in range(1, 13)]
    cache570 = Cache(live + ks + [os.path.join(scratch, 'c567')])
    groups = _ld(os.path.join(K570, 'groups.json'))
    ids570 = [it['tender_id'] for g in groups['groups'] for it in g['items']] + \
             [8375276, 8387180, 8480304, 7415397, 7025548, 6120045, 7025760]
    sources['570'] = (cache570, ids570)
    # the failed read-back's fresh 30
    f30 = [x['id'] for x in _ld(os.path.join(ROOT, 'readback', 'fresh30.json'))['draw']]
    sources['fresh30'] = (Cache([os.path.join(scratch, 'fresh')]), f30)

    report, expected_all = {}, {}
    results_by = {}
    for name, (cache, ids) in sources.items():
        res = []
        for tid in ids:
            if tid not in cache.tenders:
                res.append({'tender_id': tid, 'error': 'tender not cached', 'blocks': [], 'missing_notices': []})
                continue
            res.append(run_tender(cache, tid, m77_overlay.get(tid) if name == 'm77' else None))
        results_by[name] = res
        report[name] = {'summary': summarise([r for r in res if 'error' not in r]),
                        'errors': [r['tender_id'] for r in res if 'error' in r]}
        for r in res:
            if 'error' in r:
                continue
            expected_all[str(r['tender_id'])] = [
                {'key': b['key'], 'notice_id': b['notice_id'], 'expected_lot': b['expected_lot'], 'reason': b['reason']}
                for b in r['blocks'] if b['cause'] != 'unmodelled']
    report['u7_checks'] = checks_u7(results_by['s150'] + results_by['f30'] + results_by['m77'],
                                    os.path.join(U7, 'measurement-u8.json'))
    report['oos']['checks'] = checks_oos(results_by['oos'], oos_dirs, draw)
    report['570']['checks'] = port_570_check(results_by['570'], os.path.join(K570, 'port_result.json'), cache570)
    # fresh30 cache vs the u7 bundle's f30 copy
    u7b = {(r['tender_id'], b['notice_id'], b['key']): b['expected_lot'] for r in results_by['f30'] for b in r['blocks']}
    report['fresh30']['checks'] = {'vs_u7_bundle_f30_differ': [
        f"{r['tender_id']}/{b['notice_id']}/{b['key']}: fresh {b['expected_lot']} vs u7 {u7b.get((r['tender_id'], b['notice_id'], b['key']), '<absent>')}"
        for r in results_by['fresh30'] for b in r['blocks']
        if u7b.get((r['tender_id'], b['notice_id'], b['key']), '<absent>') != b['expected_lot']]}

    with open(os.path.join(HERE, 'expected.json'), 'w', encoding='utf-8') as fh:
        json.dump(expected_all, fh, ensure_ascii=False, indent=0, sort_keys=True)
    detail = {name: [{'tender_id': r['tender_id'], 'missing_notices': r.get('missing_notices'),
                      'blocks': [b for b in r['blocks'] if b['cause'] != 'unchanged' or b['expected_lot'] != b['served_pre']]}
                     for r in res if 'error' not in r] for name, res in results_by.items()}
    with open(os.path.join(HERE, 'combined_run.json'), 'w', encoding='utf-8') as fh:
        json.dump({'report': report, 'changed_blocks': detail}, fh, ensure_ascii=False, indent=1, default=str)
    print(json.dumps(report, ensure_ascii=False, indent=1, default=str)[:20000])


if __name__ == '__main__':
    main()
