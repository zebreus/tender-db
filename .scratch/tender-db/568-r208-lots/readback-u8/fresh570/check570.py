"""570 read-back (e): the checks on the 30 held Tenders of walk570.py (decision.md "Units" 6 (e)).

Per held Tender (its /v1/tenders/{id} and chain contents as read by the walk, after the drain; plus ONE
/v1/notices?tender={id} for the chain's profiles, the column fold's is_legacy_profile reads):
  E1 moved blocks: every award block the remap moves (combined_port.attribute: remap_only_lot != base_lot) is served
     on its expected lot, and that key's HEAD number (the remap-on carry of the chain's lot sections at the last
     version: the latest section landing there, TED-LOT_NO else TED-LOT_NUMBER) equals the block's own number
     (its printed TED-LOT_NO / TED-LOT_NUMBER, else the number of the own-notice section 567 resolved it to),
     compared by normalise_lot_number.
  E2 moved sections: each section the remap moves has a title that agrees with its target lot's title, or is
     untitled. Reference title at a key: the anchor's (first lot-publishing notice) title there, else the latest
     titled section carried there before this notice. 'target' = closer to the target's reference than to its own
     key's (by > 0.02, remap_port.title_check's measure); 'equal' = same squeezed text; 'untitled' = the section
     prints no title; 'target-untitled' = no reference title at the target (nothing to contradict). 'own' fails;
     'tie' / 'target-only' are adjudicated by reading.
  E3 no lot added or removed vs the remap-off replay: the key set after every version is the same with the remap on
     and off, and the served lot_details keys equal the replay's head keys.
  E4 parity: every served lot_result equals combined_port's expected lot (check_live.check: remap + 568 match + the
     amended guards), nothing computed-not-served, nothing served outside the chain, no non-legacy mismatch, and
     the profile and content-prefix legacy tests agree on every chain notice.

Usage: python3 -I -B check570.py WALK.json CACHE_DIR OUT_DETAIL.json OUT_SUMMARY.json [--offline]
"""
import difflib
import json
import os
import sys
import unicodedata

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
U8 = os.path.dirname(HERE)
ROOT = os.path.dirname(U8)
sys.path.insert(0, U8)
import combined_port as C   # noqa: E402
import check_live as CL     # noqa: E402
import draw_fresh30 as DF   # noqa: E402

R = C.R
norm = R.norm


def squeeze(text):
    text = unicodedata.normalize('NFKD', (text or '').lower())
    return ' '.join(''.join(ch for ch in text if not unicodedata.combining(ch)).split())


def sim(a, b):
    return difflib.SequenceMatcher(None, squeeze(a), squeeze(b)).ratio()


def wkey(text):
    """squeeze, then every run of non-word characters (dashes of any kind, dots, quotes) as one space."""
    return ' '.join(''.join(ch if ch.isalnum() else ' ' for ch in squeeze(text)).split())


def verdict(ttl, rt, ro):
    """(verdict, sims) of a moved section's title against the reference titles at its target and own keys.
    equal: same words; segment: one ';'-separated segment of the title is the target's title (and none is the own
    key's), the shape 'procedure title; LS <lot>'; target / own / tie by remap_port's similarity (> 0.02)."""
    if not ttl:
        return 'untitled', {}
    if not rt:
        return 'target-untitled', {}
    if wkey(ttl) == wkey(rt):
        return 'equal', {}
    segs = {wkey(s) for s in ttl.split(';') if s.strip()}
    if len(segs) > 1 and wkey(rt) in segs and not (ro and wkey(ro) in segs):
        return 'segment', {}
    if not ro:
        return 'target-only', {'sim_target': round(sim(ttl, rt), 3)}
    st, so = sim(ttl, rt), sim(ttl, ro)
    return ('target' if st > so + 0.02 else 'own' if so > st + 0.02 else 'tie'), {'sim_target': round(st, 3),
                                                                                   'sim_own': round(so, 3)}


PASS_V = ('untitled', 'target-untitled', 'equal', 'segment', 'target')


def replay_carry(chain, contents, remap):
    """remap_chain's carry with sources: per version the key set, and at the head {key: {kind, num, sources}}."""
    maps = C.remap_chain(chain, contents, remap)
    lots, per_version = {}, []
    for n in chain:
        target = maps[n][0]
        for sid, kind, num, item, title in R.lot_sections(contents[n]):
            tk = target.get(sid, sid)
            lot = lots.setdefault(tk, {'kind': kind, 'num': num, 'sources': []})
            lot['kind'], lot['num'] = kind, num
            lot['sources'].append({'notice': n, 'section': sid, 'num': num, 'title': title})
        per_version.append((n, sorted(lots)))
    return maps, lots, per_version


def ref_title(key, before_notice, chain, contents, anchor, lots, use_anchor=True):
    """The anchor's title at key, else the latest titled section carried at key before `before_notice`
    (use_anchor=False: the carried title only, i.e. the title of the lot the section joins as the chain holds it)."""
    if anchor is not None and use_anchor:
        at = {s[0]: s[4] for s in R.lot_sections(contents[anchor])}
        if at.get(key):
            return at[key], 'anchor'
    pos = chain.index(before_notice)
    earlier = set(chain[:pos])
    titled = [s for s in lots.get(key, {}).get('sources', []) if s['notice'] in earlier and s['title']]
    if titled:
        return titled[-1]['title'], f"carry {titled[-1]['notice']}/{titled[-1]['section']}"
    return None, None


def check_tender(t, contents, prof):
    tid = t['id']
    chain = C.chain_of(t)
    out = {'tender_id': tid, 'source': t.get('source'), 'published_at': t.get('published_at'), 'chain': chain}
    maps_on, lots_on, pv_on = replay_carry(chain, contents, True)
    _, lots_off, pv_off = replay_carry(chain, contents, False)
    lot_pub = [n for n in chain if R.lot_sections(contents[n])]
    anchor = lot_pub[0] if lot_pub else None
    out['anchor'] = anchor
    out['notice_remap'] = {str(n): maps_on[n][1] for n in lot_pub}
    moved_secs = {n: {k: v for k, v in maps_on[n][0].items() if k != v} for n in chain}
    moved_secs = {n: m for n, m in moved_secs.items() if m}
    out['moved_sections'] = {str(n): m for n, m in moved_secs.items()}
    served_title = {l['lot_key']: l.get('title') for l in t.get('lot_details') or []}
    fails, adj = [], []

    # E2 titles
    titles = []
    for n, mv in moved_secs.items():
        own_t = {s[0]: (s[2], s[4]) for s in R.lot_sections(contents[n])}
        for own, tk in sorted(mv.items()):
            num, ttl = own_t[own]
            rt, rt_from = ref_title(tk, n, chain, contents, anchor, lots_on)
            ro, ro_from = ref_title(own, n, chain, contents, anchor, lots_on)
            ct, ct_from = ref_title(tk, n, chain, contents, anchor, lots_on, use_anchor=False)
            co, co_from = ref_title(own, n, chain, contents, anchor, lots_on, use_anchor=False)
            row = {'notice': n, 'section': own, 'target': tk, 'num': num, 'title': ttl, 'ref_target': rt,
                   'ref_target_from': rt_from, 'ref_own': ro, 'ref_own_from': ro_from,
                   'carry_target': ct, 'carry_target_from': ct_from, 'carry_own': co, 'carry_own_from': co_from,
                   'served_title_at_target': served_title.get(tk)}
            v, sims = verdict(ttl, rt, ro)
            cv, csims = verdict(ttl, ct, co)
            row.update(sims, verdict=v, carry_verdict=cv, carry_sims=csims)
            titles.append(row)
            if v in PASS_V:
                continue
            if cv in PASS_V:
                adj.append(f'E2 {n}/{own}->{tk}: anchor-reference verdict {v}, carried-lot verdict {cv} '
                           f'({ttl!r} vs anchor@target {rt!r}, anchor@own {ro!r}; carried@target {ct!r} [{ct_from}])')
            elif v == 'own' and cv == 'own':
                fails.append(f'E2 {n}/{own}->{tk}: title closer to its own key under both references ({ttl!r} vs '
                             f'target {rt!r} / carried {ct!r}, own {ro!r} / carried {co!r})')
            else:
                adj.append(f'E2 {n}/{own}->{tk}: title verdict {v} (carried {cv}) ({ttl!r} vs target {rt!r}, own {ro!r})')
    out['titles'] = titles

    # E3 lot sets
    e3 = []
    for (n1, k1), (n2, k2) in zip(pv_on, pv_off):
        assert n1 == n2
        if k1 != k2:
            e3.append(f'version of notice {n1}: remap-on keys {sorted(set(k1) - set(k2))} added, '
                      f'{sorted(set(k2) - set(k1))} removed vs remap-off')
    served_keys = sorted(served_title)
    head_on = sorted(lots_on)
    if served_keys != head_on:
        e3.append(f'served lot keys differ from the replay head: served-only {sorted(set(served_keys) - set(head_on))},'
                  f' replay-only {sorted(set(head_on) - set(served_keys))}')
    out['lot_keys'] = {'served': len(served_keys), 'replay_on': len(head_on), 'replay_off': len(lots_off),
                       'same_on_off_every_version': not any(x.startswith('version') for x in e3)}
    fails += [f'E3 {x}' for x in e3]
    # E3b served head lot titles vs the latest section title carried at each key (remap on, and off for contrast)
    tl = {'served_eq_remap_on': 0, 'served_ne_remap_on': [], 'on_off_differ': 0, 'on_off_differ_served_is_on': 0}
    for k, s in served_title.items():
        lon = lots_on[k]['sources'][-1]['title'] if k in lots_on else None
        loff = lots_off[k]['sources'][-1]['title'] if k in lots_off else None
        if (s or None) == (lon or None):
            tl['served_eq_remap_on'] += 1
        else:
            tl['served_ne_remap_on'].append(f'{k}: served {s!r}, remap-on carry {lon!r}')
        if (lon or None) != (loff or None):
            tl['on_off_differ'] += 1
            tl['on_off_differ_served_is_on'] += (s or None) == (lon or None)
    out['lot_titles'] = tl
    adj += [f'E3b served lot title differs from the remap-on carry: {x}' for x in tl['served_ne_remap_on']]

    # E1 + E4
    pairs, extra, rows = CL.check(t, contents)
    served = {k: s for s, _, k in pairs}
    moved_blocks = []
    for k, r in sorted(rows.items(), key=lambda kv: (kv[0][0], kv[0][1])):
        if r['remap_only_lot'] == r['base_lot']:
            continue
        own_no, own_from = r['printed'], 'printed'
        if not (own_no or '').strip():
            same = R.same_notice_map(contents[k[0]]).get(k[1])
            sec = {s[0]: s[2] for s in R.lot_sections(contents[k[0]])}
            own_no, own_from = (sec.get(same[0]) if same and same[0] else None), 'own section'
        s = served.get(k, '<absent>')
        head_no = lots_on.get(s, {}).get('num') if isinstance(s, str) else None
        ok_lot = s == r['expected_lot']
        ok_no = s is not None and s != '<absent>' and own_no is not None and head_no is not None \
            and norm(head_no) == norm(own_no)
        mb = {'notice': k[0], 'key': k[1], 'served': s, 'expected': r['expected_lot'], 'base': r['base_lot'],
              'remap_only': r['remap_only_lot'], 'cause': r['cause'], 'own_number': own_no, 'own_from': own_from,
              'head_number_at_served': head_no, 'ok_lot': ok_lot, 'ok_number': ok_no, 'reason': r['reason']}
        moved_blocks.append(mb)
        if not ok_lot:
            fails.append(f"E1 {k[0]}/{k[1]}: served {s} expected {r['expected_lot']}")
        if s is None or s == '<absent>':
            if not ok_lot:
                continue
            adj.append(f"E1 {k[0]}/{k[1]}: moved to no lot (expected {r['expected_lot']}; {r['reason']})")
        elif not ok_no:
            (fails if own_no is not None else adj).append(
                f"E1 {k[0]}/{k[1]}: on {s} whose head number is {head_no!r}, own number {own_no!r} ({own_from})")
    out['moved_blocks'] = moved_blocks
    breaks, nl_bad, outside = [], [], []
    for s, r, k in pairs:
        if r is None:
            outside.append(f'{k[0]}/{k[1]}: served {s}')
        elif r['reason'].startswith('non-legacy'):
            if s != r['expected_lot']:
                nl_bad.append(f"{k[0]}/{k[1]}: served {s} BT-13713 {r['expected_lot']}")
        elif s != r['expected_lot']:
            breaks.append(f"{k[0]}/{k[1]}: served {s} expected {r['expected_lot']} ({r['reason']}; cause {r['cause']})")
    leg = []
    if prof is None:
        adj.append('E4 profiles not read')
    else:
        for n in chain:
            if not prof.get(n):
                adj.append(f'E4 {n}: no profile')
                continue
            if CL.is_legacy_profile(prof[n]) != C.is_legacy(contents[n]):
                leg.append(f'{n}: profile {prof[n]!r} vs prefix {C.is_legacy(contents[n])}')
    out['profiles'] = {str(n): (prof or {}).get(n) for n in chain}
    out['parity'] = {'served_blocks': len(pairs), 'breaks': breaks, 'non_legacy_mismatch': nl_bad,
                     'served_outside_chain': outside, 'computed_not_served': [f'{k[0]}/{k[1]}' for k in extra],
                     'legacy_test_disagrees': leg}
    fails += [f'E4 break {x}' for x in breaks]
    adj += [f'E4 non-legacy {x}' for x in nl_bad] + [f'E4 outside-chain {x}' for x in outside] + \
        [f'E4 computed-not-served {k[0]}/{k[1]}' for k in extra] + [f'E4 legacy-test {x}' for x in leg]
    # remapped notices must be legacy by profile too
    for n in moved_secs:
        if prof and prof.get(n) and not CL.is_legacy_profile(prof[n]):
            fails.append(f'E4 remapped notice {n} has non-legacy profile {prof[n]!r}')
    out['fails'], out['adjudicate'] = fails, adj
    out['verdict'] = 'fail' if fails else ('adjudicate' if adj else 'pass')
    return out


def prior_sets():
    sets = {}
    groups = json.load(open(os.path.join(os.path.dirname(ROOT), '570-lot-keys', 'groups.json')))
    for g in groups['groups']:
        for it in g['items']:
            sets.setdefault(it['tender_id'], []).append('570-sample')
    for i in (8375276, 8387180, 8480304, 7415397, 7025548, 6120045, 7025760):
        sets.setdefault(i, []).append('570-exhibit')
    for i, s in DF.exclusions().items():
        sets.setdefault(i, []).append('568:' + s)
    try:
        for x in json.load(open(os.path.join(U8, 'fresh568', 'draw.json')))['draw']:
            sets.setdefault(x['id'], []).append('fresh568')
    except OSError:
        pass
    return sets


def main():
    a = sys.argv[1:]
    offline = '--offline' in a
    a = [x for x in a if x != '--offline']
    walk_path, cache, out_detail, out_summary = a
    w = json.load(open(walk_path))
    f = CL.Fetcher(cache, 1.6, offline, [])
    sets = prior_sets()
    results = []
    for tid in w['held_ids']:
        t = f.tender(tid)
        contents = {n: f.notice(n) for n in C.chain_of(t)}
        assert all(v is not None for v in contents.values()), tid
        prof = f.profiles(tid)
        r = check_tender(t, contents, prof)
        r['prior_sets'] = sets.get(tid, [])
        results.append(r)
        print(f"{tid}: {r['verdict']} moved_sections={sum(len(m) for m in r['moved_sections'].values())} "
              f"moved_blocks={len(r['moved_blocks'])} fails={r['fails']} adj={r['adjudicate']}", flush=True)
    walked = w['walk']
    summary = {
        'stage': 'fresh570 (e)', 'walked': len(walked), 'held': len(w['held_ids']),
        'skipped_by_reason': {}, 'not_found_404': w['not_found_404'],
        'pass': sum(r['verdict'] == 'pass' for r in results),
        'fail': [r['tender_id'] for r in results if r['verdict'] == 'fail'],
        'adjudicate': [r['tender_id'] for r in results if r['verdict'] == 'adjudicate'],
        'moved_sections': sum(len(m) for r in results for m in r['moved_sections'].values()),
        'moved_blocks': sum(len(r['moved_blocks']) for r in results),
        'title_verdicts': {}, 'gets_check': f.gets, 'fetch_errors': f.errors,
        'tenders': [{'id': r['tender_id'], 'verdict': r['verdict'], 'published_at': r['published_at'],
                     'source': r['source'], 'chain': len(r['chain']), 'notice_remap': r['notice_remap'],
                     'moved_sections': r['moved_sections'],
                     'moved_blocks': [f"{b['notice']}/{b['key']}: {b['base']}->{b['served']} (own {b['own_number']}, head {b['head_number_at_served']})" for b in r['moved_blocks']],
                     'titles': [f"{x['notice']}/{x['section']}->{x['target']}: {x['verdict']}" for x in r['titles']],
                     'lot_keys': r['lot_keys'], 'served_blocks': r['parity']['served_blocks'],
                     'prior_sets': r['prior_sets'], 'fails': r['fails'], 'adjudicate': r['adjudicate']}
                    for r in results]}
    for x in walked:
        if not x.get('held'):
            k = x['why'].split(':')[0]
            summary['skipped_by_reason'][k] = summary['skipped_by_reason'].get(k, 0) + 1
    summary['title_verdicts_carried'] = {}
    for r in results:
        for x in r['titles']:
            summary['title_verdicts'][x['verdict']] = summary['title_verdicts'].get(x['verdict'], 0) + 1
            summary['title_verdicts_carried'][x['carry_verdict']] = \
                summary['title_verdicts_carried'].get(x['carry_verdict'], 0) + 1
    with open(out_detail, 'w') as fh:
        json.dump(results, fh, indent=1, ensure_ascii=False, default=str)
    with open(out_summary, 'w') as fh:
        json.dump(summary, fh, indent=1, ensure_ascii=False, default=str)
    print(json.dumps({k: v for k, v in summary.items() if k != 'tenders'}, ensure_ascii=False))


if __name__ == '__main__':
    main()
