"""Issue 551 B6 U3: export the Rust-vs-port parity inputs for path W v2 + G (R10's candidate guards G-single and
G-cover), per population, and the port of record's decisions under the four guard sets.

    python3 -I -B export_u3.py check   [--pop P ...]       which notice contents no layer holds (no GET)
    python3 -I -B export_u3.py export  OUT_DIR [--pop P ...] [--limit N]
    python3 -I -B export_u3.py proj    OUT_DIR [--pop b3api s61 c563]   projection inputs for those (see cmd_proj)

The port of record is b6/rule_v2_g.py (sha256 pinned below), loaded UNMODIFIED; only its layer lists (module globals
TENDER_LAYERS / NOTICE_LAYERS, read at call time by find_cached) are pointed at each population's reads, and its
`facts` is wrapped (same function, memoised per id, with the served-aware v1 reading below). The fixture builder is
B3's b3/gen_b3.py (sha256 pinned), loaded through b3/export.py (its `emit`, `s` and `profile_of` are reused); its
module global `R` is rebound to rule_v2_g, so the fixture is built from exactly the facts the port decides on.

POPULATIONS (U3 = issue 551 "B6 -- decision" point 4):
  b3api   B3's 562 API ids (279 labelled + 6 out of sample + 277 walked) on B3's reads (scratchpad b3/raw/tenders;
          notices in B3's order: b3/raw, the M1 cache, B2's bundle). Rust input: B3's own api_in.tsv, copied verbatim
          (and re-generated here: the copy must be byte-identical to the re-generated file).
  b3rec   B3's 4,745 unit-1 records (unit3/data/ctx.json.gz), the port side as b3/records.py builds it (its v1_of /
          detail_of reused) through rule_v2_g.facts_from. Rust input: B3's own rec_in.tsv, verbatim. U3 FIX (an
          input-encoding gap of the port side, found by U3's first comparison): merge_rule.serve is given each
          withheld amount as the port of record's reader encodes it (rule_v2_g._entry: the API serves a withheld
          amount without a value, read as cents -1), not with the store's -100 placeholder, which matches a withheld
          lot award's -100 placeholder and so files the figure as an award copy (the shape 5d3fc52 removed from the
          Rust). It touches the 7 records with a withheld amount. B3's encoding is kept as b3rec_port_b3enc.jsonl.
  c563    B5's cohort (b5/cohort_ids.json: B3's 562 + 7511890) on B5's re-reads (scratchpad b5/raw/tenders, read after
          the cohort drain with path W ON), notices in B5's order then the B6 layer.
  s61     the 61 B5 investigation heads (b5/investigate/read_items.json) on the served snapshots the readers read
          (b5/investigate/served/{id}.json, snapshotted before the switch-off, path W ON); notices in rule_v2_g's order.
  f3502   the 3,502 served lot-sum-awards heads (U2's decisions.jsonl.gz ids) on the fresh B6 layer (scratchpad
          b6/tenders, read 05:47-07:41Z 2026-10-19 with path W OFF); notices in rule_v2_g's order. The port's
          decisions must equal U2's per id and guard set (checked here); the projection input is written for these.

SERVED-AWARE v1 (B5's cohort_parity.port_decision, operations.md step 3): the port's API reader checks G0 against
TODAY'S SERVED head ('W/G0-served'). Where the served head is itself a lot-sum-awards value (c563 and s61 were read
with path W on) that check refuses every real ship; the Rust twin reads the chain's facts, never a served head. So on
a head served lot-sum-awards a 'W/G0-served' v1 reason is read as 'W' (v1 fires) before facts_from runs; every such
id is listed (`v1_served_aware`). On every other head the port's own reason stands.

Files per population P in OUT_DIR: P_in.tsv (Rust input, B3's line format), P_port.jsonl (one row per id: the
port's decisions under v2 / v2+single / v2+cover / v2+both and the facts compare_u3.py reads), and for f3502
f3502_proj_in.jsonl (the projection harness input: each Tender's chain notices in fold order, inferred profile,
content path) whose port facts are in f3502_port.jsonl (`port_facts`, `port_lots_all_versions`).
"""
import argparse
import collections
import copy
import gzip
import hashlib
import importlib.util
import json
import os
import shutil
import socket
import sys
import time
import urllib.request

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
B6 = os.path.dirname(HERE)
ROOT = os.path.dirname(B6)                      # .scratch/tender-db/551-lot-sum
B3 = os.path.join(ROOT, 'b3')
B5 = os.path.join(ROOT, 'b5')
SP = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad'

RULE = os.path.join(B6, 'rule_v2_g.py')
RULE_SHA = '23cbd8508a3afa35b195f8a2ee91a75bb3b8f08aa2e2f3f9abbd0ea80454452f'
PINNED = {
    os.path.join(B3, 'gen_b3.py'): '2e9635bae0a49b7924780d7829d549ec9edf5180a69c37d9b6062aa83cc6d9de',
    os.path.join(B3, 'export.py'): '2203cab002f581f8733a364f7442c535efd38d1234dada64907460adbd577ea5',
    os.path.join(B3, 'records.py'): '124382c2e056cb9aed1d8fb90916e21278bbc6ab2b1fc8151ec04f358f16fc14',
}
B3_API_IN = os.path.join(SP, 'b3', 'run', 'api_in.tsv')
B3_API_IN_SHA = '37a738e290af7a8e6b7af0128bf9bbc9c1ab0fb0c8abce8b4e0b695e90797c51'
B3_REC_IN = os.path.join(SP, 'b3', 'run', 'rec_in.tsv')
B3_REC_IN_SHA = '4c0dc216305b11e0c946791f6cad7f6a828b7f275475028558648e4c930f4d3b'
B3_API_PORT = os.path.join(SP, 'b3', 'run', 'api_port.jsonl')
B3_REC_PORT = os.path.join(SP, 'b3', 'run', 'rec_port.jsonl')
U2_DECISIONS = os.path.join(B6, 'u2', 'decisions.jsonl.gz')
U2_DECISIONS_SHA = '0add92e36c70ca76b81049e9faf678e91305e790d1fd914ff4952a0f1d0749cb'
COHORT = os.path.join(B5, 'cohort_ids.json')
ITEMS61 = os.path.join(B5, 'investigate', 'read_items.json')
SERVED61 = os.path.join(B5, 'investigate', 'served')
API_NOTICES = os.path.join(HERE, 'api', 'notices')    # public-API fetches made by U3 (if any), last notice layer

POP_ORDER = ('b3api', 'b3rec', 'c563', 's61', 'f3502')


def sha(p):
    with open(p, 'rb') as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def no_network():
    def refuse(*a, **k):
        raise RuntimeError('export_u3: network disabled (U3 exports from the recorded layers only)')
    socket.socket.connect = refuse
    socket.create_connection = refuse
    urllib.request.urlopen = refuse


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def load_all():
    got = sha(RULE)
    if got != RULE_SHA:
        raise SystemExit(f'export_u3: {RULE} sha256 {got} is not the port of record {RULE_SHA}')
    for p, h in PINNED.items():
        if sha(p) != h:
            raise SystemExit(f'export_u3: {p} changed (sha256 {sha(p)}, pinned {h})')
    RG = load('rule_v2_g_u3', RULE)
    EX = load('b3_export_u3', os.path.join(B3, 'export.py'))      # loads gen_b3 (EX.G) and rule_v2_b3 (EX.R)
    G = EX.G
    G.R = RG                     # gen_b3.build reads R.* at call time: the fixture comes from rule_v2_g's facts
    EX.R = RG
    REC = load('b3_records_u3', os.path.join(B3, 'records.py'))
    remember_layers(RG)
    return RG, EX, G, REC


# ------------------------------------------------------------------------------------------------ layers
ORIG_LAYERS = {}


def remember_layers(RG):
    """rule_v2_g's own layer lists, captured once at load (set_layers edits the module lists in place)."""
    ORIG_LAYERS['tenders'] = [tuple(l) for l in RG.TENDER_LAYERS]
    ORIG_LAYERS['notices'] = [tuple(l) for l in RG.NOTICE_LAYERS]


def default_notice_layers(RG):
    return list(ORIG_LAYERS['notices'])


def pop_layers(RG, pop):
    """-> (tender layers, notice layers) for population `pop`, as rule_v2_g's (dir, pattern, gzip) triples."""
    base_notices = default_notice_layers(RG)
    api = [(API_NOTICES, '{}.json', False)]
    if pop == 'b3api':
        return ([(os.path.join(SP, 'b3', 'raw', 'tenders'), '{}.json', False)],
                [(os.path.join(SP, 'b3', 'raw', 'notices'), '{}.json', False),
                 (os.path.join(SP, 'm551', 'cache', 'notices'), '{}.json', False),
                 (os.path.join(SP, 'b3', 'bundle', 'cache', 'notices'), '{}.json', False)] + api)
    if pop == 'c563':
        b5_order = [(os.path.join(SP, 'b3', 'raw', 'notices'), '{}.json', False),
                    (os.path.join(SP, 'm551', 'cache', 'notices'), '{}.json', False),
                    (os.path.join(SP, 'b3', 'bundle', 'cache', 'notices'), '{}.json', False),
                    (os.path.join(SP, 'b3', 'verify', 'raw', 'notices'), '{}.json', False),
                    (os.path.join(SP, 'b5', 'raw', 'notices'), '{}.json', False)]
        rest = [l for l in base_notices if l not in b5_order]
        return [(os.path.join(SP, 'b5', 'raw', 'tenders'), '{}.json', False)], b5_order + rest + api
    if pop == 's61':
        return [(SERVED61, '{}.json', False)], base_notices + api
    if pop == 'f3502':
        return list(ORIG_LAYERS['tenders']), base_notices + api
    raise KeyError(pop)


def set_layers(RG, pop):
    t, n = pop_layers(RG, pop)
    RG.TENDER_LAYERS[:] = t          # in place: find_cached reads the module globals at call time
    RG.NOTICE_LAYERS[:] = n


def pop_ids(pop):
    if pop == 'b3api':
        return sorted(json.loads(l)['id'] for l in open(B3_API_PORT))
    if pop == 'c563':
        return sorted(int(i) for i in json.load(open(COHORT))['ids'])
    if pop == 's61':
        return sorted(int(i['id']) for i in json.load(open(ITEMS61))['items'])
    if pop == 'f3502':
        if sha(U2_DECISIONS) != U2_DECISIONS_SHA:
            raise SystemExit(f'export_u3: {U2_DECISIONS} changed')
        with gzip.open(U2_DECISIONS, 'rt', encoding='utf-8') as fh:
            return sorted(json.loads(l)['id'] for l in fh)
    raise KeyError(pop)


# ------------------------------------------------------------------------------------------------ facts
class Facts:
    """rule_v2_g.facts with the served-aware v1 reading, memoised for the current id only (the fixture builder and
    the decisions read the same dict). Notice contents are memoised per id too (gen_b3.build re-reads them)."""

    def __init__(self, RG):
        self.RG = RG
        self.orig_facts = RG.facts
        self.orig_notice = RG.notice_content
        self.key = None
        self.f = None
        self.notices = {}
        self.served_aware = None
        RG.facts = self.facts
        RG.notice_content = self.notice_content

    def reset(self):
        self.key, self.f, self.notices, self.served_aware = None, None, {}, None

    def notice_content(self, nid, refresh=False):
        nid = int(nid)
        if nid not in self.notices:
            self.notices[nid] = self.orig_notice(nid)
        return self.notices[nid]

    def facts(self, tid, refresh=False):
        tid = int(tid)
        if self.key == tid:
            return self.f
        self.reset()
        RG = self.RG
        x = RG.tender_detail(tid)
        contents = {nid: self.notice_content(nid) for nid in RG.version_notice_ids(x)}
        v1 = RG.v1_eval(x)
        served_lsa = (x.get('value_derivation') or {}).get('method') == 'lot-sum-awards'
        if served_lsa and v1['why'].startswith('W/G0-served'):
            self.served_aware = v1['why']
            v1 = dict(v1, why='W', fires=True)
        self.f = RG.facts_from(x, contents, v1)
        self.key = tid
        return self.f


def slim(d):
    return {k: d[k] for k in ('ship', 'first', 'S', 'currency', 'n', 'vouched_by', 'vouches', 'refusals',
                              'g_single_removed', 'g_cover_removed', 'g_removed')}


# ------------------------------------------------------------------------------------------------ check
def cmd_check(args):
    no_network()
    RG, EX, G, REC = load_all()
    out = {}
    for pop in args.pop:
        if pop == 'b3rec':
            continue
        set_layers(RG, pop)
        missing_t, missing_n, use = [], set(), collections.Counter()
        for t in pop_ids(pop):
            if RG.find_cached('tenders', t) is None:
                missing_t.append(t)
                continue
            x = RG.tender_detail(t)
            for nid in RG.version_notice_ids(x):
                p = RG.find_cached('notices', nid)
                if p is None:
                    missing_n.add(nid)
                else:
                    use[os.path.dirname(p)] += 1
        out[pop] = {'missing_tenders': missing_t, 'missing_notices': sorted(missing_n), 'layer_use': dict(use)}
        print(pop, 'missing tenders', len(missing_t), 'missing notices', len(missing_n), dict(use))
    json.dump(out, open(os.path.join(HERE, 'outputs', 'check_layers.json'), 'w'), indent=1)


# ------------------------------------------------------------------------------------------------ export
def export_api_pop(RG, EX, G, FX, pop, od, limit=None):
    set_layers(RG, pop)
    ids = pop_ids(pop)[:limit] if limit else pop_ids(pop)
    tsv = open(os.path.join(od, f'{pop}_in.tsv' if pop != 'b3api' else 'b3api_regen.tsv'), 'w')
    pj = open(os.path.join(od, f'{pop}_port.jsonl'), 'w')
    proj = open(os.path.join(od, f'{pop}_proj_in.jsonl'), 'w') if pop == 'f3502' else None
    u2 = {}
    if pop == 'f3502':
        with gzip.open(U2_DECISIONS, 'rt', encoding='utf-8') as fh:
            for l in fh:
                r = json.loads(l)
                u2[r['id']] = r
    stats = collections.Counter()
    t0 = time.time()
    tdig, ndig = [], {}
    for i, tid in enumerate(ids):
        FX.reset()
        tp = RG.find_cached('tenders', tid)
        tdig.append(f'{tid}\t{sha(tp)}\n')
        x = RG.tender_detail(tid)
        f = RG.facts(tid)
        for nid in f['notices']:
            p = RG.find_cached('notices', nid)
            ndig[nid] = os.path.dirname(p)
        dec = RG.decide_all(f)
        row = {'id': tid, 'population': pop, 'decisions': {k: slim(v) for k, v in dec.items()},
               'v1_why': f['v1']['why'], 'v1_served_aware': FX.served_aware, 'v1_fires': RG.v1_fires(f),
               'served_method': (x.get('value_derivation') or {}).get('method'), 'S_eur': f.get('S_eur'),
               'flags': f['flags'], 'vouch_candidates': f.get('vouch_candidates'),
               'summed': [[s['key'], s['notice_id'], s['cents']] for s in f.get('summed') or []],
               'tender_path': tp}
        if FX.served_aware:
            stats['served_aware'] += 1
        try:
            versions, rates, o, notes = G.build(tid)
        except SystemExit as e:
            versions, notes = None, [f'NOT EXPORTED to Rust: {e}']
            stats['not_exported'] += 1
        assert RG.facts(tid) is f, tid          # gen_b3.build read the same facts dict
        row['exported'] = versions is not None
        row['notes'] = notes
        if versions is not None:
            # gen_b3's own outcome is v2_decision on the same facts: it must equal the port's v2
            v2 = dec['v2']
            assert (o['ship'], o['first'], o['S'], o['cur'], o['n']) == (v2['ship'], v2['first'], v2['S'], v2['currency'], v2['n']), tid
            EX.emit(tsv, tid, versions, rates)
        if pop == 'f3502':
            r = u2.get(tid)
            same = r is not None and all(slim(r['decisions'][k]) == slim(dec[k]) for k in dec) \
                and r['vouch_candidates'] == f.get('vouch_candidates')
            row['u2_same'] = same
            stats['u2_same' if same else 'u2_differs'] += 1
            if versions is not None:
                write_projection(RG, EX, x, f, versions, row, proj)
        pj.write(json.dumps(row, default=str, sort_keys=True) + '\n')
        stats['exported' if versions is not None else 'skipped'] += 1
        if (i + 1) % 250 == 0:
            print(f'  {pop}: {i + 1}/{len(ids)} in {time.time() - t0:.0f}s', flush=True)
    tsv.close()
    pj.close()
    if proj:
        proj.close()
    ndig_lines = ''.join(f'{n}\t{d}\n' for n, d in sorted(ndig.items()))
    layer_use = collections.Counter(ndig.values())
    meta = {'population': pop, 'ids': len(ids), 'stats': dict(stats), 'seconds': round(time.time() - t0, 1),
            'tender_layers': RG.TENDER_LAYERS, 'notice_layers': RG.NOTICE_LAYERS,
            'tender_reads_sha256': hashlib.sha256(''.join(tdig).encode()).hexdigest(),
            'notice_layer_map_sha256': hashlib.sha256(ndig_lines.encode()).hexdigest(),
            'notices': len(ndig), 'notice_layer_use': dict(layer_use)}
    print(pop, json.dumps(meta['stats']), f"{meta['seconds']}s", flush=True)
    return meta


def write_projection(RG, EX, x, f, versions, row, proj):
    """b3/export.py's projection rows (port_lots_all_versions, port_facts, proj_in), on rule_v2_g's facts."""
    fts = x.get('source') == 'fts'
    vseq = list(dict.fromkeys(v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0)
                              if v.get('caused_by_notice_id')))
    contents = {nid: RG.notice_content(nid) for nid in f['notices']}
    INV, HOW = RG.remap_inverse(x, contents)
    carried, snap = {}, {}
    for nid in vseq:
        target = {own: t for t, own in INV.get(nid, {}).items()}
        for sid, L in RG.notice_facts(contents[nid])['lots'].items():
            no = sid if fts else L.get('lot_no')
            carried[target.get(sid, sid)] = (L['kind'], None if no is None else str(no))
        snap[str(nid)] = {k: [kind, no] for k, (kind, no) in sorted(carried.items())}
    row['port_lots_all_versions'] = snap
    # the served chain in version (seq) order, as a list: a JSON object's key order does not survive sort_keys
    row['served_chain'] = vseq
    row['remap_how'] = {str(k): v for k, v in HOW.items()}
    row['port_facts'] = {
        'lots_by_version': {str(v['notice']): {l['key']: l['no'] for l in v['lots']} for v in versions},
        'results': [[r['notice'], x_['key'], x_['lot'], x_['no'], x_['group']] for r in versions[-1]['rounds'] for x_ in r['results']],
        'totals': [[t['notice'], t['logical'], t['field'], t['kind'].split('::')[1], t['cents'], t['currency'], t['lots']]
                   for t in versions[-1]['totals']],
        'head_lot_keys': sorted(l['key'] for l in versions[-1]['lots']),
    }
    chain = vseq
    extras = sorted(set(RG.version_notice_ids(x)) - set(chain))
    notices = []
    for nid in chain:
        p = RG.find_cached('notices', nid)
        c = contents[nid]
        prof = EX.profile_of(c, x.get('source'))
        notices.append({'id': nid, 'profile': prof, 'path': p,
                        'source': 'ted' if prof in ('text', 'ted-export-r209') else (x.get('source') or 'ted'),
                        'publication_id': next((v.get('publication_id') for v in x['versions'] if v['caused_by_notice_id'] == nid), '') or ''})
    proj.write(json.dumps({'id': x['id'], 'source': x.get('source'), 'notices': notices, 'extras': extras}) + '\n')


def api_withheld(e):
    """The entry as the port of record's own reader encodes it: rule_v2_g._entry turns an amount the API serves
    with a quality and no value (withheld) into cents -1 (eur_cents None); unit 1's stored rows carry the store's
    -100 placeholder there. -> (entry copy, whether any amount changed)."""
    e = copy.deepcopy(e)
    changed = False
    for v in [e['head']] + list(e.get('chain') or []):
        for a in v.get('amounts') or []:
            if a.get('quality') is not None and a.get('cents') != -1:
                a['cents'], a['eur_cents'] = -1, None
                changed = True
    return e, changed


def export_records(RG, REC, od, limit=None, encoding='api'):
    """b3/records.py's port side on rule_v2_g (its v1_of / detail_of reused); the Rust input is B3's rec_in.tsv.
    encoding 'api' (primary, b3rec_port.jsonl): merge_rule.serve gets the entry with every withheld amount at the
    port reader's -1 (api_withheld); 'b3' (b3rec_port_b3enc.jsonl, for the record): the stored rows as B3 fed them."""
    MR = REC.MR
    ctx = MR.load_ctx()
    recs = {}
    for line in open(os.path.join(ROOT, 'results-slim.jsonl')):
        r = json.loads(line)
        recs[r['id']] = r
    pj = open(os.path.join(od, 'b3rec_port.jsonl' if encoding == 'api' else 'b3rec_port_b3enc.jsonl'), 'w')
    stats = collections.Counter()
    t0 = time.time()
    ids = sorted(ctx)[:limit] if limit else sorted(ctx)
    for tid in ids:
        e = ctx[tid]
        if encoding == 'api':
            e, changed = api_withheld(e)
            if changed:
                stats['withheld_reencoded'] += 1
                stats.setdefault('withheld_reencoded_ids', []).append(tid)
        rates = MR.rates_of(e)
        v1, versions, head = REC.v1_of(e, rates)
        x = REC.detail_of(tid, head)
        contents = {nid: {'notice_id': nid, 'sections': []} for nid in RG.version_notice_ids(x)}
        f = RG.facts_from(x, contents, v1)
        dec = RG.decide_all(f)
        pj.write(json.dumps({'id': tid, 'population': 'b3rec', 'pop': recs[tid]['pop'], 'encoding': encoding,
                             'decisions': {k: slim(v) for k, v in dec.items()}, 'v1_why': v1['why'],
                             'v1_served_aware': None, 'v1_fires': RG.v1_fires(f), 'S_eur': f.get('S_eur'),
                             'vouch_candidates': f.get('vouch_candidates'), 'exported': True, 'notes': []},
                            default=str, sort_keys=True) + '\n')
        stats['exported'] += 1
    pj.close()
    print('b3rec', encoding, dict(stats), f'{time.time() - t0:.0f}s', flush=True)
    return {'population': 'b3rec', 'encoding': encoding, 'ids': len(ids), 'stats': dict(stats), 'seconds': round(time.time() - t0, 1)}


def cmd_export(args):
    no_network()
    od = args.out_dir
    os.makedirs(od, exist_ok=True)
    RG, EX, G, REC = load_all()
    FX = Facts(RG)
    metas = {}
    for pop in [p for p in POP_ORDER if p in args.pop]:
        if pop == 'b3rec':
            if sha(B3_REC_IN) != B3_REC_IN_SHA:
                raise SystemExit('export_u3: B3 rec_in.tsv changed')
            shutil.copyfile(B3_REC_IN, os.path.join(od, 'b3rec_in.tsv'))
            metas[pop] = export_records(RG, REC, od, args.limit, 'api')
            metas['b3rec_b3enc'] = export_records(RG, REC, od, args.limit, 'b3')
            continue
        metas[pop] = export_api_pop(RG, EX, G, FX, pop, od, args.limit)
        if pop == 'b3api':
            if sha(B3_API_IN) != B3_API_IN_SHA:
                raise SystemExit('export_u3: B3 api_in.tsv changed')
            shutil.copyfile(B3_API_IN, os.path.join(od, 'b3api_in.tsv'))
            regen = sha(os.path.join(od, 'b3api_regen.tsv'))
            metas[pop]['b3_api_in_sha256'] = B3_API_IN_SHA
            metas[pop]['regenerated_sha256'] = regen
            metas[pop]['regenerated_equals_b3'] = regen == B3_API_IN_SHA
            print('b3api: regenerated fixture file equals B3 api_in.tsv:', regen == B3_API_IN_SHA, flush=True)
    for p in sorted(os.listdir(od)):
        if p.endswith(('.tsv', '.jsonl')):
            metas.setdefault('_files', {})[p] = sha(os.path.join(od, p))
    metas['_pins'] = {'rule_v2_g.py': RULE_SHA, **{os.path.basename(k): v for k, v in PINNED.items()},
                      'rates': RG.rates_file_hashes() if hasattr(RG, 'rates_file_hashes') else None}
    json.dump(metas, open(os.path.join(od, 'export_meta.json'), 'w'), indent=1, default=str)


def cmd_proj(args):
    """Projection inputs only, for API populations other than f3502 (f3502's come with `export`): writes
    OUT_DIR/{pop}_proj_in.jsonl and OUT_DIR/{pop}_projport.jsonl (id, port_facts, port_lots_all_versions, remap_how),
    from the same facts and fixture build as `export`. The Rust inputs and port decisions are not rewritten."""
    no_network()
    od = args.out_dir
    RG, EX, G, REC = load_all()
    FX = Facts(RG)
    for pop in [p for p in POP_ORDER if p in args.pop]:
        assert pop not in ('b3rec', 'f3502'), pop
        set_layers(RG, pop)
        n = 0
        with open(os.path.join(od, f'{pop}_proj_in.jsonl'), 'w') as proj, open(os.path.join(od, f'{pop}_projport.jsonl'), 'w') as pp:
            for tid in pop_ids(pop):
                FX.reset()
                x = RG.tender_detail(tid)
                f = RG.facts(tid)
                versions, rates, o, notes = G.build(tid)
                row = {'id': tid, 'population': pop}
                write_projection(RG, EX, x, f, versions, row, proj)
                pp.write(json.dumps(row, default=str, sort_keys=True) + '\n')
                n += 1
        print(pop, 'projection inputs', n, flush=True)


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest='cmd', required=True)
    c = sub.add_parser('check')
    c.add_argument('--pop', nargs='+', default=list(POP_ORDER))
    e = sub.add_parser('export')
    e.add_argument('out_dir')
    e.add_argument('--pop', nargs='+', default=list(POP_ORDER))
    e.add_argument('--limit', type=int)
    q = sub.add_parser('proj')
    q.add_argument('out_dir')
    q.add_argument('--pop', nargs='+', default=['b3api', 's61'])
    args = ap.parse_args()
    {'check': cmd_check, 'export': cmd_export, 'proj': cmd_proj}[args.cmd](args)


if __name__ == '__main__':
    main()
