"""Issue 551 B6 U1: write b6/rule_v2_g.py, the B6 copy of b3/rule_v2_b3.py (the port of record, sha256 5400bc55...).
b3/rule_v2_b3.py is evidence and is never edited. Every edit is an exact, asserted, single-occurrence replacement,
listed in EDITS below; `diff ../b3/rule_v2_b3.py rule_v2_g.py` shows them all.

    python3 -I -B make_rule_g.py [--check | --write]     (--check, the default, writes nothing)
"""
import hashlib
import os

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, '..', 'b3', 'rule_v2_b3.py')
SRC_SHA = '5400bc555f15ba09e852afe132009a900a2e5a3634e7546ff703fc9a8a89ade5'
OUT = os.path.join(HERE, 'rule_v2_g.py')


def sha(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def patch(src, edits):
    for i, (old, new) in enumerate(edits):
        n = src.count(old)
        assert n == 1, (i, n, old[:160])
        src = src.replace(old, new)
    return src


EDITS = [
    # (1) provenance header
    ("568 unit 8's refusals need no edit: the port reads the served lot_results, where a refused block is `lot: null`.\n",
     "568 unit 8's refusals need no edit: the port reads the served lot_results, where a refused block is `lot: null`.\n"
     "\n"
     "B6 COPY (issue 551 B6 U1, 2026-10-19), written by ../b6/make_rule_g.py from b3/rule_v2_b3.py (sha256 5400bc55...).\n"
     "Changes, each an asserted replacement (make_rule_g.py EDITS):\n"
     "  (a) the fresh tender layer: /v1/tenders/{id} is read ONLY from scratchpad b6/tenders/{id}.json.gz (filled by\n"
     "      b6/fetch_b6.py); notice contents from b6/notices/{id}.json.gz first, then the existing layers (B3 raw, the M1\n"
     "      cache, B2's bundle, B3's verifier, B5's fetches, the 568 read-back band / verify_band / u7 sets). Notice\n"
     "      contents are immutable; tender JSON is not, so no older tender layer is read. A miss raises FetchError. The\n"
     "      rule makes no GET (the fetch code is removed, not just unreachable).\n"
     "  (b) the two candidate guards of issue 551 \"B6 -- decision\" point 2, worded exactly as decided:\n"
     "      G-single: a stated total from a notice whose round in this version has exactly ONE LotResult (keyed or not;\n"
     "                the served lot_results rows with that notice_id) does not vouch.\n"
     "      G-cover:  a total of kind Award (r3_kind == 'award': BT-161, BT-118, BT-1118, TED-VAL_TOTAL on a CAN-type form)\n"
     "                or Range (field in ('TED-LOW', 'TED-HIGH')) vouches only if its notice names at least one summed\n"
     "                key, by R3's own test (results_by_lot[key] holds a row of that notice), keyless results excluded.\n"
     "                Estimate totals are unchanged.\n"
     "      Both only filter R10's candidate list (facts()['vouch_candidates']); the refusal name stays R10. Each emits a\n"
     "      diagnostic (g_single_removed / g_cover_removed, and their union g_removed). R1..R9 and R3 are unchanged.\n"
     "      Each candidate's kind, round_results, names_summed_keys, g_single and g_cover come from the pure function\n"
     "      cand_flags() (kind from guard_kind(): Range by field, decision A2), which the self-tests call directly.\n"
     "  (c) the switch: decide(tender_id, guards=frozenset() | {'single'} | {'cover'} | {'single', 'cover'}) and\n"
     "      decide_facts(f, guards) / decide_all(f), so v2, v2+single, v2+cover and v2+both come from the same facts.\n"
     "      guards=frozenset() is v2 exactly (v2_decision, unchanged).\n"
     "  (d) the rates files are unchanged (m551/rates by default) but the directory is a parameter (set_rates_dir(),\n"
     "      or the RULE_G_RATES_DIR environment variable at import), and decide() records their sha256.\n"),
    # (2) paths: the B6 layers and the rates-dir parameter
    ("RATES_DIR = SCRATCH + '/rates'\n",
     "RATES_DIR = os.environ.get('RULE_G_RATES_DIR') or SCRATCH + '/rates'      # B6 (d): a parameter, see set_rates_dir()\n"),
    ("MIN_GAP_S = 1.5            # one GET per 1.5 s, across processes (stamp file)\n",
     "# B6 (a): the fresh tender layer (tenders ONLY from here) and the notice layers, first hit wins. Each layer is\n"
     "# (directory, filename pattern, gzip). The rb8 layers are 568 read-back notice contents, the same\n"
     "# /v1/notices/{id}/content JSON ({notice_id, sections}) under another file name.\n"
     "SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'\n"
     "B6 = SP + '/b6'\n"
     "B6_TENDERS = B6 + '/tenders'\n"
     "B6_NOTICES = B6 + '/notices'\n"
     "TENDER_LAYERS = [(B6_TENDERS, '{}.json.gz', True)]\n"
     "NOTICE_LAYERS = [(B6_NOTICES, '{}.json.gz', True),\n"
     "                 (B3_RAW + '/notices', '{}.json', False),\n"
     "                 (CACHE + '/notices', '{}.json', False),\n"
     "                 (B3_BUNDLE + '/notices', '{}.json', False),\n"
     "                 (SP + '/b3/verify/raw/notices', '{}.json', False),\n"
     "                 (SP + '/b5/raw/notices', '{}.json', False),\n"
     "                 (SP + '/rb8/band/notices', 'notices_{}_content.json', False),\n"
     "                 (SP + '/rb8/verify_band/notices', 'n{}.json', False),\n"
     "                 (SP + '/rb8/sets/u7notices', 'notices_{}_content.json', False)]\n"),
    # (3) the reader: offline, layered, no GET anywhere in the rule
    ("_SSL = None\n\n\ndef _wait_turn():\n",
     "def _layers(kind):\n"
     "    return TENDER_LAYERS if kind == 'tenders' else NOTICE_LAYERS\n"
     "\n"
     "\n"
     "def find_cached(kind, key):\n"
     "    \"\"\"B6: the path of the first layer holding `key` (kind 'tenders' or 'notices'), or None.\"\"\"\n"
     "    for base, pat, _gz in _layers(kind):\n"
     "        p = os.path.join(base, pat.format(int(key)))\n"
     "        if os.path.exists(p):\n"
     "            return p\n"
     "    return None\n"
     "\n"
     "\n"
     "def _load(p):\n"
     "    if p.endswith('.gz'):\n"
     "        with gzip.open(p, 'rt', encoding='utf-8') as fh:\n"
     "            return json.load(fh)\n"
     "    with open(p, encoding='utf-8') as fh:\n"
     "        return json.load(fh)\n"
     "\n"
     "\n"
     "def _cached(kind, key, path, refresh=False):\n"
     "    # B6 (a): offline. Tenders: the fresh b6 layer only. Notices: b6, then the older layers (immutable contents).\n"
     "    p = find_cached(kind, key)\n"
     "    if p is None:\n"
     "        raise FetchError(path, 'not cached (the B6 rule never fetches; run b6/fetch_b6.py)')\n"
     "    x = _load(p)\n"
     "    got = x.get('id') if kind == 'tenders' else x.get('notice_id')\n"
     "    if got is not None and int(got) != int(key):\n"
     "        raise FetchError(path, f'{p} holds {kind} {got}, not {key}')\n"
     "    return x\n"
     "\n"
     "\n"
     "def _REMOVED_wait_turn():\n"),
]


def edits_strip_fetch(src):
    """(3b) Remove the dead GET code: the old `_wait_turn` / `_get` / `_cached` bodies, between the marker the edit
    above left and `def tender_detail`."""
    a = src.index('def _REMOVED_wait_turn():\n')
    b = src.index('def tender_detail(tid, refresh=False):\n')
    gone = src[a:b]
    assert gone.count('def ') == 3 and '_get(path)' in gone and "raise FetchError(path, 'not cached (B3" in gone, gone[:200]
    return src[:a] + src[b:]


EDITS_2 = [
    # (4) imports: gzip for the layers
    ("import collections\nimport hashlib\n", "import collections\nimport gzip\nimport hashlib\n"),
    # (5) rates dir as a parameter (B6 (d))
    ("def rates_file_hashes():\n",
     "def set_rates_dir(path):\n"
     "    \"\"\"B6 (d): read the rates files from `path` (ecu-ert_h_eur_d.csv, ecu-ert_bil_eur_d.csv, eurofxref-hist.zip).\"\"\"\n"
     "    global RATES_DIR, _RATES, _RATES_SHA\n"
     "    RATES_DIR, _RATES, _RATES_SHA = path, None, None\n"
     "\n"
     "\n"
     "_RATES_SHA = None\n"
     "\n"
     "\n"
     "def rates_sha256():\n"
     "    \"\"\"B6 (d): {'dir': RATES_DIR, 'files': {name: sha256}} of the rates files in use (computed once per dir).\"\"\"\n"
     "    global _RATES_SHA\n"
     "    if _RATES_SHA is None:\n"
     "        _RATES_SHA = {'dir': RATES_DIR, 'files': rates_file_hashes()}\n"
     "    return _RATES_SHA\n"
     "\n"
     "\n"
     "def rates_file_hashes():\n"),
    # (6) R10: the candidates carry the facts both guards read; R10 itself (v2) is unchanged
    ("    f['vouches'] = [(t['field'], t['notice_id'], t['cents']) for t in vouch]\n",
     "    f['vouches'] = [(t['field'], t['notice_id'], t['cents']) for t in vouch]\n"
     "    # B6 (b): the facts G-single and G-cover read, per R10 candidate (in the v2 candidate order), from the pure\n"
     "    # function cand_flags() (defined above facts(); the self-tests call it directly on real-shaped inputs).\n"
     "    summed_keys = summed_keys_of(summed)\n"
     "    round_n = round_counts(x.get('lot_results', []))\n"
     "    f['vouch_candidates'] = [cand_flags(t, summed_keys, results_by_lot, round_n, len(NF[t['notice_id']]['results']))\n"
     "                             for t in vouch]\n"),
    # (6b) the pure per-candidate derivation, before facts()
    ("def facts(tender_id, refresh=False):\n",
     "# ---------------------------------------------------------------- B6 (b): the per-candidate guard facts (pure)\n"
     "def summed_keys_of(summed):\n"
     "    \"\"\"The summed keys G-cover may count: keyless results excluded (a None or '' key is never a summed key).\"\"\"\n"
     "    return [s['key'] for s in summed if s['key'] not in (None, '')]\n"
     "\n"
     "\n"
     "def round_counts(lot_results):\n"
     "    \"\"\"{notice_id: number of this version's served lot_results rows with that notice_id, keyed or not}.\"\"\"\n"
     "    return collections.Counter(lr.get('notice_id') for lr in lot_results)\n"
     "\n"
     "\n"
     "def guard_kind(field, r3_kind):\n"
     "    \"\"\"G-cover's kind (decision A2): Range by FIELD (TED-LOW / TED-HIGH carry r3_kind None in the port), Award\n"
     "    by r3_kind == 'award' (BT-161, BT-118, BT-1118, TED-VAL_TOTAL on a CAN-type form), Estimate by r3_kind.\"\"\"\n"
     "    if field in ('TED-LOW', 'TED-HIGH'):\n"
     "        return 'range'\n"
     "    return r3_kind if r3_kind in ('award', 'estimate') else None\n"
     "\n"
     "\n"
     "def cand_flags(t, summed_keys, results_by_lot, round_n, content_lot_results=None):\n"
     "    \"\"\"One R10 candidate `t` (a usable total: field, notice_id, cents, r3_kind, form) -> its guard facts.\n"
     "      round_results:     round_n[notice_id], the served lot_results rows of that notice, keyed or not\n"
     "      names_summed_keys: the summed keys for which R3's test holds -- results_by_lot[key] (a list of (i, row))\n"
     "                         holds a row of that notice; only keys in `summed_keys` are tested, so a keyless row\n"
     "                         never names a key\n"
     "      g_single:          round_results == 1                (G-single removes it)\n"
     "      g_cover:           kind in (award, range) and no key named   (G-cover removes it)\"\"\"\n"
     "    nid = t['notice_id']\n"
     "    kind = guard_kind(t['field'], t.get('r3_kind'))\n"
     "    names = [k for k in summed_keys if k not in (None, '')\n"
     "             and any(lr.get('notice_id') == nid for _i, lr in results_by_lot.get(k, []))]\n"
     "    c = {'field': t['field'], 'notice_id': nid, 'cents': t['cents'], 'kind': kind, 'form': t.get('form'),\n"
     "         'round_results': round_n.get(nid, 0), 'content_lot_results': content_lot_results,\n"
     "         'names_summed_keys': names}\n"
     "    c['g_single'] = c['round_results'] == 1\n"
     "    c['g_cover'] = kind in ('award', 'range') and not names\n"
     "    return c\n"
     "\n"
     "\n"
     "def facts(tender_id, refresh=False):\n"),
    # (7) the guards and the switch, after v2_decision
    ("# ---------------------------------------------------------------- the M1 port check\n",
     "# ---------------------------------------------------------------- B6 (b) + (c): the guards and the switch\n"
     "GUARDS = ('single', 'cover')\n"
     "GUARD_SETS = (frozenset(), frozenset({'single'}), frozenset({'cover'}), frozenset({'single', 'cover'}))\n"
     "\n"
     "\n"
     "def guard_name(guards):\n"
     "    g = frozenset(guards)\n"
     "    return 'v2' if not g else 'v2+' + ('both' if g == frozenset(GUARDS) else next(iter(g)))\n"
     "\n"
     "\n"
     "def _cand_key(c):\n"
     "    return [c['field'], c['notice_id'], c['cents']]\n"
     "\n"
     "\n"
     "def guarded_vouches(f, guards):\n"
     "    \"\"\"-> (kept candidates, g_single_removed, g_cover_removed). Each guard only removes R10 candidates; a\n"
     "    candidate both remove is listed under both. With guards=frozenset() nothing is removed (v2).\"\"\"\n"
     "    g = frozenset(guards)\n"
     "    assert g <= set(GUARDS), g\n"
     "    kept, rs, rc = [], [], []\n"
     "    for c in f.get('vouch_candidates') or []:\n"
     "        drop = False\n"
     "        if 'single' in g and c['g_single']:\n"
     "            rs.append(c)\n"
     "            drop = True\n"
     "        if 'cover' in g and c['g_cover']:\n"
     "            rc.append(c)\n"
     "            drop = True\n"
     "        if not drop:\n"
     "            kept.append(c)\n"
     "    return kept, rs, rc\n"
     "\n"
     "\n"
     "def decide_facts(f, guards=frozenset()):\n"
     "    \"\"\"v2 (guards empty) or v2 + the named guards, on facts `f`. -> dict with ship, first (the first refusing\n"
     "    check; R10 when a guard empties R10's candidates), S, currency, n, vouched_by (the first kept candidate),\n"
     "    vouches (every kept candidate, [field, notice_id, cents]), refusals and the guard diagnostics.\"\"\"\n"
     "    g = frozenset(guards)\n"
     "    ship, first, S, cur, n = v2_decision(f)\n"
     "    kept, rs, rc = guarded_vouches(f, g)\n"
     "    refusals = v2_refusals(f)\n"
     "    if g and v1_fires(f) and f.get('pred'):\n"
     "        P = dict(f['pred'])\n"
     "        s_eur = f.get('S_eur')\n"
     "        P['R10'] = s_eur is not None and s_eur >= R10_MIN_EUR_CENTS and not kept\n"
     "        refusals = [k for k in V2_ORDER if P.get(k)]\n"
     "        ship, first = (not refusals), (refusals[0] if refusals else None)\n"
     "    vb = ({'field': kept[0]['field'], 'notice_id': kept[0]['notice_id'], 'cents': kept[0]['cents']} if kept else None)\n"
     "    if not g:\n"
     "        assert vb == f.get('vouched_by') and [tuple(_cand_key(c)) for c in kept] == list(f.get('vouches') or []), f['id']\n"
     "    removed = {tuple(_cand_key(c)): c for c in rs + rc}\n"
     "    return {'guards': sorted(g), 'name': guard_name(g), 'ship': bool(ship), 'first': first, 'S': S, 'currency': cur,\n"
     "            'n': n, 'vouched_by': vb, 'vouches': [_cand_key(c) for c in kept], 'refusals': refusals,\n"
     "            'g_single_removed': [_cand_key(c) for c in rs] if 'single' in g else None,\n"
     "            'g_cover_removed': [_cand_key(c) for c in rc] if 'cover' in g else None,\n"
     "            'g_removed': [list(k) for k in removed]}\n"
     "\n"
     "\n"
     "def decide_all(f):\n"
     "    \"\"\"{name: decide_facts(f, guards)} for v2, v2+single, v2+cover, v2+both, on the same facts.\"\"\"\n"
     "    return {guard_name(g): decide_facts(f, g) for g in GUARD_SETS}\n"
     "\n"
     "\n"
     "def decide(tender_id, guards=frozenset()):\n"
     "    \"\"\"B6 (c): decide one Tender from the cached facts under `guards`. Records the rates files' sha256 and where\n"
     "    the tender read came from (its b6 sidecar's fetch time when present).\"\"\"\n"
     "    f = facts(tender_id)\n"
     "    out = decide_facts(f, guards)\n"
     "    out.update(id=int(tender_id), S_eur=f.get('S_eur'), v1_why=f['v1']['why'], flags=f['flags'],\n"
     "               vouch_candidates=f.get('vouch_candidates'), tender_read=tender_read_meta(tender_id),\n"
     "               rates=rates_sha256())\n"
     "    return out\n"
     "\n"
     "\n"
     "def tender_read_meta(tender_id):\n"
     "    p = find_cached('tenders', tender_id)\n"
     "    if p is None:\n"
     "        return None\n"
     "    meta = os.path.join(os.path.dirname(p), f'{int(tender_id)}.meta.json')\n"
     "    m = _load(meta) if os.path.exists(meta) else {}\n"
     "    return {'path': p, 'fetched_at': m.get('fetched_at')}\n"
     "\n"
     "\n"
     "# ---------------------------------------------------------------- the M1 port check\n"),
    # (8) CLI: decide ids under the four variants
    ("    else:\n        for t in a:\n            f = facts(int(t))\n            print(t, v2_decision(f), f['flags'])\n",
     "    else:\n"
     "        # B6: python3 -I -B rule_v2_g.py [--rates-dir DIR] ID [ID ...] -> one JSON line per id, all four variants\n"
     "        if a and a[0] == '--rates-dir':\n"
     "            set_rates_dir(a[1])\n"
     "            a = a[2:]\n"
     "        for t in a:\n"
     "            f = facts(int(t))\n"
     "            print(json.dumps({'id': int(t), 'decisions': decide_all(f), 'flags': f['flags'],\n"
     "                              'vouch_candidates': f.get('vouch_candidates'), 'rates': rates_sha256()}, default=str))\n"),
]


def build():
    got = sha(SRC)
    assert got == SRC_SHA, f'{SRC} sha256 {got} is not the port of record {SRC_SHA}'
    with open(SRC, encoding='utf-8') as fh:
        src = fh.read()
    src = patch(src, EDITS)
    src = edits_strip_fetch(src)
    src = patch(src, EDITS_2)
    for banned in ('_get(', '_wait_turn', 'urlopen('):
        assert banned not in src, banned
    return src


def main(argv=None):
    """--check (the default): regenerate in memory and compare with rule_v2_g.py, writing nothing (exit 1 on a
    difference). --write: replace rule_v2_g.py. Any other argument is refused (argparse, exit 2)."""
    import argparse
    import sys
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group()
    g.add_argument('--check', action='store_true', help='compare only (the default); writes nothing')
    g.add_argument('--write', action='store_true', help='replace rule_v2_g.py with the regenerated port')
    a = ap.parse_args(argv)
    src = build()
    new_sha = hashlib.sha256(src.encode('utf-8')).hexdigest()
    if not a.write:
        cur = sha(OUT) if os.path.exists(OUT) else None
        same = cur == new_sha
        print(f'{"MATCH" if same else "DIFFER"}: regenerated {new_sha} vs {OUT} {cur}')
        return 0 if same else 1
    tmp = OUT + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as fh:
        fh.write(src)
    os.replace(tmp, OUT)
    print(f'{OUT} sha256 {sha(OUT)} (from {SRC_SHA[:12]}...)')
    return 0


if __name__ == '__main__':
    import sys
    sys.exit(main())
