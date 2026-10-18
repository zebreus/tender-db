"""Issue 551 B3: write rule_v2_b3.py and gen_b3.py, the B3 copies of unit4b/rule_v2.py and unit4b/gen_fixtures.py
(the committed M1/B2 files are evidence and are never edited). Every edit is an exact, asserted, single-occurrence
replacement, listed in EDITS_* below; `diff` the outputs against the originals to see them all.

    python3 -I make_port_copy.py
"""
import hashlib, os
HERE = os.path.dirname(os.path.abspath(__file__))
U4B = os.path.join(HERE, '..', 'unit4b')
RULE_SHA = '39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce'
GEN_SHA = None   # recorded below at first run

def sha(p):
    return hashlib.sha256(open(p, 'rb').read()).hexdigest()

def patch(src, edits, name):
    for old, new in edits:
        n = src.count(old)
        assert n == 1, (name, n, old[:120])
        src = src.replace(old, new)
    return src

SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'

EDITS_RULE = [
 # (1) provenance header
 ('"""Issue 551 unit 4b, M1: path W v2 ("lot-sum-awards"), ported to PUBLIC-API facts.\n',
  '"""Issue 551 unit 4b, M1: path W v2 ("lot-sum-awards"), ported to PUBLIC-API facts.\n\n'
  'B3 COPY (issue 551 B3, 2026-10-18), written by ../b3/make_port_copy.py from unit4b/rule_v2.py (sha256 39be6d1e...05ce).\n'
  'Changes, each an asserted replacement (see make_port_copy.py EDITS_RULE): (a) issue 570: a summed or round award\'s\n'
  'published lot number is read from the notice\'s lot section whose 570 remap TARGET is the served key\n'
  '(remap_port.remap with K3, through 568-u8 combined_port.remap_chain over the version chain in fold order), not from\n'
  'the section whose own id equals the served key; (b) R10 at every size ("vouched everywhere", M2 fallback PASSED);\n'
  '(c) reads are offline: /v1/tenders/{id} from the B3 post-drain re-reads only, notice contents from the B3 fetches,\n'
  'then the M1 cache, then B2\'s committed bundle (a refold does not change a notice\'s parsed content); no GET;\n'
  '(d) facts() is split into facts_from(x, contents, v1) so the 4,745 recorded records can be fed in (b3/records.py).\n'
  '568 unit 8\'s refusals need no edit: the port reads the served lot_results, where a refused block is `lot: null`.\n'),
 # (2) offline layered cache, B3 scratch
 ("SCRATCH = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551'\nCACHE = SCRATCH + '/cache'\nRATES_DIR = SCRATCH + '/rates'\n",
  "SCRATCH = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/m551'\nCACHE = SCRATCH + '/cache'\nRATES_DIR = SCRATCH + '/rates'\n"
  "B3_RAW = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b3/raw'\n"
  "B3_BUNDLE = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/b3/bundle/cache'\n"),
 ("def _cached(kind, key, path, refresh=False):\n    d = os.path.join(CACHE, kind)\n",
  "def _cached(kind, key, path, refresh=False):\n"
  "    # B3: offline. Tenders: the post-drain re-reads only. Notices: B3 fetches, then the M1 cache, then B2's bundle.\n"
  "    layers = [B3_RAW] if kind == 'tenders' else [B3_RAW, CACHE, B3_BUNDLE]\n"
  "    for base in layers:\n"
  "        p = os.path.join(base, kind, f'{int(key)}.json')\n"
  "        if os.path.exists(p):\n"
  "            return json.load(open(p))\n"
  "    raise FetchError(path, 'not cached (B3 never fetches here; run b3/fetch.py)')\n"
  "    d = os.path.join(CACHE, kind)\n"),
 # (3) R10 at every size
 ("R10_MIN_EUR_CENTS = 1_000_000_000          # EUR 10,000,000.00 (amendment (a))\n",
  "R10_MIN_EUR_CENTS = 0                      # B3: R10 at every size (M2 fallback \"vouched everywhere\" PASSED)\n"),
 # (4) the 570 remap, imported pinned
 ("_spec = importlib.util.spec_from_file_location('merge_rule_551',",
  "_CP = os.path.join(HERE, '..', '..', '568-r208-lots', 'readback-u8', 'combined_port.py')\n"
  "_RP = os.path.join(HERE, '..', '..', '570-lot-keys', 'remap_port.py')\n"
  "for _p, _h in ((_CP, '5363593c663c651bcbf125464ccfd24b96d45b56e1834086352f7d8903d7673c'),\n"
  "               (_RP, 'abe2b78ae5a0506dd327a412f9753c77a8d2a448fba080d4d73c0aae083bd503')):\n"
  "    _got = hashlib.sha256(open(_p, 'rb').read()).hexdigest()\n"
  "    if _got != _h:\n"
  "        raise SystemExit(f'rule_v2_b3: {_p} changed (sha256 {_got}, pinned {_h})')\n"
  "_cspec = importlib.util.spec_from_file_location('combined_port_570', _CP)\n"
  "CP = importlib.util.module_from_spec(_cspec)\n"
  "_cspec.loader.exec_module(CP)\n"
  "# B3: the fold's `legacy` is `is_legacy_profile(profile)`; the content endpoint carries no profile, and the FTS parser\n"
  "# writes `TED-VAL_TOTAL_TAX_BASIS` on OCDS notices (crates/ingest/src/fts/parse.rs), which combined_port's TED-/TXT-\n"
  "# test reads as legacy. An FTS notice is never legacy (`fts:*`), so that one field does not count here.\n"
  "CP.is_legacy = lambda content: any(v['field_id'].startswith(('TED-', 'TXT-')) and v['field_id'] != 'TED-VAL_TOTAL_TAX_BASIS'\n"
  "                                   for s in content.get('sections', []) for v in s.get('values', []))\n\n\n"
  "def remap_inverse(x, contents):\n"
  "    \"\"\"Issue 570 (B3): {notice: {served Tender key: the notice's own lot section id}} over the version chain in fold\n"
  "    order (seq), via combined_port.remap_chain (remap_port.remap + K3 + the carry). A notice with no lot section maps\n"
  "    nothing; an unremapped one maps each own id to itself.\"\"\"\n"
  "    chain = [v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0)\n"
  "             if v.get('caused_by_notice_id')]\n"
  "    maps = CP.remap_chain(list(dict.fromkeys(chain)), contents)\n"
  "    inv = {}\n"
  "    for nid, c in contents.items():\n"
  "        # identity over every lot section the port reads (LOT_KINDS), then the 570 moves (K1: injective)\n"
  "        target = {s['section_id']: s['section_id'] for s in c.get('sections', []) if s.get('kind') in LOT_KINDS}\n"
  "        target.update((maps.get(nid) or ({}, None))[0])\n"
  "        inv[nid] = {t: own for own, t in target.items()}\n"
  "        assert len(inv[nid]) == len(target), (nid, target)\n"
  "    return inv, {nid: how for nid, (target, how) in maps.items()}\n\n\n"
  "_spec = importlib.util.spec_from_file_location('merge_rule_551',"),
 # (5) facts() split: facts_from(x, contents, v1)
 ("def facts(tender_id, refresh=False):\n    x = tender_detail(tender_id, refresh=refresh)\n    tid = int(x['id'])\n    nids = version_notice_ids(x)\n    NF = {}\n    for nid in nids:\n        NF[nid] = notice_facts(notice_content(nid))\n    v1 = v1_eval(x)\n",
  "def facts(tender_id, refresh=False):\n    x = tender_detail(tender_id, refresh=refresh)\n"
  "    contents = {nid: notice_content(nid) for nid in version_notice_ids(x)}\n"
  "    return facts_from(x, contents, v1_eval(x))\n\n\n"
  "def facts_from(x, contents, v1):\n"
  "    \"\"\"B3: facts() on a given detail `x`, its notices' contents and a v1 evaluation (v1_eval(x), or b3/records.py's\n"
  "    entry-based one for the 4,745 records).\"\"\"\n"
  "    tid = int(x['id'])\n    nids = version_notice_ids(x)\n    NF = {}\n    for nid in nids:\n        NF[nid] = notice_facts(contents[nid])\n"
  "    INV, HOW = remap_inverse(x, contents)\n"),
 ("         'notices': nids, 'v1': v1, 'flags': list(v1['flags'])}\n",
  "         'notices': nids, 'v1': v1, 'flags': list(v1['flags']),\n"
  "         'remap': {str(n): h for n, h in HOW.items() if h == 'remapped' or h.startswith('refused') or h in ('K3', 'missing')}}\n"),
 # (6) pub_no reads the section whose 570 target is the served key
 ("        L = nf['lots'].get(key)\n        if L and L.get('lot_no') is not None:\n            return L['lot_no'], f'lot section {key} of notice {nid}'\n",
  "        own = INV.get(nid, {}).get(key)          # B3 / issue 570: the section whose remap target is `key`\n"
  "        L = nf['lots'].get(own) if own is not None else None\n"
  "        if L and L.get('lot_no') is not None:\n"
  "            return L['lot_no'], f'lot section {own} of notice {nid}' + ('' if own == key else f' (570: remapped onto {key})')\n"),
]

EDITS_GEN = [
 ('"""Generate crates/store/src/canonical_551_v2_fixtures.rs:',
  '"""B3 COPY (issue 551 B3), written by ../b3/make_port_copy.py from unit4b/gen_fixtures.py: imports rule_v2_b3.py\n'
  '(the 570-remap-aware port copy) instead of rule_v2.py, and B1\'s lot-number carry lands each notice\'s lot section on\n'
  'its 570 remap TARGET key (fold step 6), not on its own section id. Only build() is used (by b3/export.py).\n\n'
  'Generate crates/store/src/canonical_551_v2_fixtures.rs:'),
 ("RULE_V2_SHA256 = '39be6d1ebc0ed2828ce3dd2a10b4db33f98bdf83ec5c5a7bb2d5fe4401a305ce'\n",
  "RULE_V2_SHA256 = None  # B3: rule_v2_b3.py is checked by make_port_copy.py's output, not pinned here\n"),
 ("_got = sha256(os.path.join(HERE, 'rule_v2.py'))\nif _got != RULE_V2_SHA256:\n    raise SystemExit(f'gen_fixtures: rule_v2.py changed (sha256 {_got}, pinned {RULE_V2_SHA256})')\nspec = importlib.util.spec_from_file_location('rule_v2_b2', os.path.join(HERE, 'rule_v2.py'))\n",
  "spec = importlib.util.spec_from_file_location('rule_v2_b3', os.path.join(HERE, 'rule_v2_b3.py'))\n"),
 ("PORT_CHECK = json.load(open(os.path.join(HERE, 'M1-port-check.json')))\n",
  "PORT_CHECK = json.load(open(os.path.join(HERE, '..', 'unit4b', 'M1-port-check.json')))\n"),
 ("    # B1's lot-number carry over the version notices.\n    carried = {}\n    snapshot = {}\n    for nid in vseq:\n        for sid, L in NF[nid]['lots'].items():\n            no = sid if fts else L.get('lot_no')\n            carried[sid] = (L['kind'], None if no is None else str(no))\n",
  "    # B1's lot-number carry over the version notices; B3 / issue 570: each section lands on its remap target.\n"
  "    INV, _how = R.remap_inverse(x, {nid: R.notice_content(nid) for nid in f['notices']})\n"
  "    TARGET = {nid: {own: t for t, own in inv.items()} for nid, inv in INV.items()}\n"
  "    carried = {}\n    snapshot = {}\n    for nid in vseq:\n        for sid, L in NF[nid]['lots'].items():\n            no = sid if fts else L.get('lot_no')\n"
  "            carried[TARGET.get(nid, {}).get(sid, sid)] = (L['kind'], None if no is None else str(no))\n"),
 ("            port_no, _ = _port_pub_no(x, NF, lr['notice_id'], s['key'], lr.get('key'))\n",
  "            port_no, _ = _port_pub_no(x, NF, lr['notice_id'], INV.get(lr['notice_id'], {}).get(s['key']), lr.get('key'))\n"),
 ("def _port_pub_no(x, NF, nid, key, result_key):\n    nf = NF.get(nid)\n    if nf is None:\n        return None, 'unread'\n    L = nf['lots'].get(key)\n",
  "def _port_pub_no(x, NF, nid, key, result_key):\n    # B3: `key` is the notice's own section id whose 570 target is the summed key (None: no such section).\n"
  "    nf = NF.get(nid)\n    if nf is None:\n        return None, 'unread'\n    L = nf['lots'].get(key) if key is not None else None\n"),
]

def main():
    rs = os.path.join(U4B, 'rule_v2.py')
    gs = os.path.join(U4B, 'gen_fixtures.py')
    assert sha(rs) == RULE_SHA, sha(rs)
    print('gen_fixtures.py sha256', sha(gs))
    open(os.path.join(HERE, 'rule_v2_b3.py'), 'w').write(patch(open(rs).read(), EDITS_RULE, 'rule'))
    open(os.path.join(HERE, 'gen_b3.py'), 'w').write(patch(open(gs).read(), EDITS_GEN, 'gen'))
    print('wrote rule_v2_b3.py', sha(os.path.join(HERE, 'rule_v2_b3.py')))
    print('wrote gen_b3.py', sha(os.path.join(HERE, 'gen_b3.py')))

if __name__ == '__main__':
    main()
