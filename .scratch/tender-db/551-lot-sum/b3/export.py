"""Issue 551 B3: export the decision-parity inputs and the port's outcomes for the 562 API ids (279 labelled + 6 out of
sample + 277 walked), from the post-drain re-reads, through the 570-aware port copy (rule_v2_b3.py / gen_b3.py).

    python3 -I export.py OUT_DIR

Writes OUT_DIR/api_in.tsv (the Rust harness input: one block per Tender, the fixture chain gen_b3.build makes),
OUT_DIR/api_port.jsonl (the port's decision per id + facts the projection check compares) and OUT_DIR/proj_in.jsonl
(the ingest harness input: each Tender's chain notices in fold order with an inferred profile and the content path).
"""
import importlib.util, json, os, sys
from urllib.parse import quote

HERE = os.path.dirname(os.path.abspath(__file__))
U4B = os.path.join(HERE, '..', 'unit4b')
spec = importlib.util.spec_from_file_location('gen_b3', os.path.join(HERE, 'gen_b3.py'))
G = importlib.util.module_from_spec(spec)
spec.loader.exec_module(G)
R = G.R


def s(x):
    """Percent-encoded (every byte outside [A-Za-z0-9_.~-]); '-' is None."""
    if x is None:
        return '-'
    x = quote(str(x), safe='')
    return '%2D' if x == '-' else x


def ids():
    pc = json.load(open(os.path.join(U4B, 'M1-port-check.json')))
    out = {int(k): ('labelled-' + v['group'] if v['group'] != 'oos' else 'oos') for k, v in pc.items()}
    for f, tag in (('walk.jsonl', 'walk'), ('walk_fallback.jsonl', 'walk-fallback')):
        for line in open(os.path.join(U4B, f)):
            r = json.loads(line)
            if 'id' in r:
                out.setdefault(int(r['id']), tag)
    return out


def old_outcomes():
    """M1 / M2-fallback-era outcomes (pre-drain data, R10 from EUR 10 m in M1; the walk files record theirs)."""
    old = {}
    pc = json.load(open(os.path.join(U4B, 'M1-port-check.json')))
    for k, v in pc.items():
        old[int(k)] = {'v1_fires': v['v1_fires'], 'ship_m1': v['ship'], 'first_m1': v['first'], 'vouched': v['vouched_by'] is not None,
                       'S': v['S'], 'cur': v['cur'], 'n': v['n']}
    for f in ('walk.jsonl', 'walk_fallback.jsonl'):
        for line in open(os.path.join(U4B, f)):
            r = json.loads(line)
            if 'id' in r:
                old.setdefault(int(r['id']), {'v1_fires': r['v1_fires'], 'ship_m1': r['outcome'] == 'ship', 'first_m1': r['first'],
                                              'vouched': r.get('vouched_by') is not None, 'S': r['S'], 'cur': r['currency'], 'n': r['n']})
    return old


def profile_of(content, source):
    """The notice's profile, inferred from its content (the content endpoint carries none): only the predicates the
    projection branches on matter (is_legacy_profile, text, is_sdk01_profile, is_de1_profile, fts)."""
    # the FTS parser writes TED-VAL_TOTAL_TAX_BASIS on OCDS notices (crates/ingest/src/fts/parse.rs): not a TED field
    fields = [v['field_id'] for sct in content.get('sections', []) for v in sct.get('values', []) if v['field_id'] != 'TED-VAL_TOTAL_TAX_BASIS']
    if any(f.startswith('TXT-') for f in fields):
        return 'text'
    if any(f.startswith('TED-') for f in fields):
        return 'ted-export-r209'
    if any(f.startswith('DE1-') for f in fields):
        return 'eforms:eforms-de-1.2'
    if any(f.startswith('SDK01-') for f in fields):
        return 'eforms:eforms-sdk-0.1'
    if source == 'fts':
        return 'fts:ocds'
    return 'eforms:eforms-sdk-1.10'


def content_path(nid):
    for base in (R.B3_RAW, R.CACHE, R.B3_BUNDLE):
        p = os.path.join(base, 'notices', f'{int(nid)}.json')
        if os.path.exists(p):
            return p
    return None


def emit(out, tid, versions, rates):
    out.write(f'T\t{tid}\n')
    for c, r in sorted(rates.items()):
        out.write(f'R\t{s(c)}\t{r!r}\n')
    for v in versions:
        out.write(f'V\t{v["notice"]}\n')
        for a in v['facts']:
            out.write(f'F\t{s(a[0])}\t{int(a[1])}\t{s(a[2])}\t{s(a[3])}\n')
        for l in v['lots']:
            out.write(f'L\t{s(l["key"])}\t{s(l["kind"])}\t{s(l["no"])}\t{s(l["title"])}\n')
            for a in l['amounts']:
                out.write(f'LF\t{s(a[0])}\t{int(a[1])}\t{s(a[2])}\t{s(a[3])}\n')
        for r in v['rounds']:
            out.write(f'N\t{r["notice"]}\n')
            for x in r['results']:
                out.write('A\t' + '\t'.join([s(x['key']), s(x['lot']), s(x['cents']), s(x['currency']), s(x['decision']),
                                             ','.join(str(w) for w in x['winners']) or '-', s(x['no']), '1' if x['group'] else '0']) + '\n')
            for b in r['bids']:
                parties = ','.join(f'{s(role)}:{org}' for role, org in b['parties']) or '-'
                out.write('B\t' + '\t'.join([s(b['key']), s(b['lot']), s(b['cents']), s(b['currency']), s(b['quality']), parties]) + '\n')
        for t in v['totals']:
            out.write('S\t' + '\t'.join([str(t['notice']), s(t['logical']), s(t['field']), t['kind'].split('::')[1], str(t['cents']),
                                         s(t['currency']), ','.join(s(k) for k in t['lots']) or '-']) + '\n')
    out.write('E\n')


def main():
    od = sys.argv[1]
    os.makedirs(od, exist_ok=True)
    pop = ids()
    old = old_outcomes()
    tsv = open(os.path.join(od, 'api_in.tsv'), 'w')
    pj = open(os.path.join(od, 'api_port.jsonl'), 'w')
    proj = open(os.path.join(od, 'proj_in.jsonl'), 'w')
    skipped = []
    for tid in sorted(pop):
        x = R.tender_detail(tid)
        try:
            versions, rates, o, notes = G.build(tid)
        except SystemExit as e:
            skipped.append((tid, str(e)))
            f = R.facts(tid)
            ship, first, S, cur, n = R.v2_decision(f)
            o = {'ship': ship, 'first': first, 'S': S, 'cur': cur, 'n': n, 'vouched_by': f.get('vouched_by'), 'today': f['v1']['today']}
            versions, notes = None, [f'NOT EXPORTED to Rust: {e}']
        f = R.facts(tid)
        row = {'id': tid, 'population': pop[tid], 'ship': o['ship'], 'first': o['first'], 'S': o['S'], 'cur': o['cur'], 'n': o['n'],
               'vouched_by': o['vouched_by'], 'v1_fires': R.v1_fires(f), 'v1_why': f['v1']['why'], 'refusals': R.v2_refusals(f),
               'flags': f['flags'], 'remap': f.get('remap'), 'notes': notes, 'served': f['v1']['served'],
               'served_matches_election': f['v1']['served_matches_election'], 'old': old.get(tid),
               'source': x.get('source'), 'exported': versions is not None, 'vouches': f.get('vouches'),
               'S_eur': f.get('S_eur')}
        # the port's B1 lot-number carry at EVERY version notice (gen_b3.build's carry, repeated here because build()
        # returns only the round notices' and the head's): each section on its 570 target, None included
        fts = x.get('source') == 'fts'
        vseq = list(dict.fromkeys(v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0)
                                  if v.get('caused_by_notice_id')))
        contents = {nid: R.notice_content(nid) for nid in f['notices']}
        INV, HOW = R.remap_inverse(x, contents)
        carried, snap = {}, {}
        for nid in vseq:
            target = {own: t for t, own in INV.get(nid, {}).items()}
            for sid, L in R.notice_facts(contents[nid])['lots'].items():
                no = sid if fts else L.get('lot_no')
                carried[target.get(sid, sid)] = (L['kind'], None if no is None else str(no))
            snap[str(nid)] = {k: [kind, no] for k, (kind, no) in sorted(carried.items())}
        row['port_lots_all_versions'] = snap
        row['remap_how'] = {str(k): v for k, v in HOW.items()}
        # the port's facts the projection-parity check compares (B1's four facts, from gen_b3.build's chain)
        if versions is not None:
            row['port_facts'] = {
                'lots_by_version': {str(v['notice']): {l['key']: l['no'] for l in v['lots']} for v in versions},
                'results': [[r['notice'], x_['key'], x_['lot'], x_['no'], x_['group']] for r in versions[-1]['rounds'] for x_ in r['results']],
                'totals': [[t['notice'], t['logical'], t['field'], t['kind'].split('::')[1], t['cents'], t['currency'], t['lots']]
                           for t in versions[-1]['totals']],
                'head_lot_keys': sorted(l['key'] for l in versions[-1]['lots']),
            }
            emit(tsv, tid, versions, rates)
        pj.write(json.dumps(row, default=str) + '\n')
        chain = [v['caused_by_notice_id'] for v in sorted(x.get('versions', []), key=lambda v: v.get('seq') or 0) if v.get('caused_by_notice_id')]
        chain = list(dict.fromkeys(chain))
        extras = sorted(set(R.version_notice_ids(x)) - set(chain))
        notices = []
        for nid in chain:
            p = content_path(nid)
            c = json.load(open(p))
            prof = profile_of(c, x.get('source'))
            notices.append({'id': nid, 'profile': prof, 'path': p,
                            'source': 'ted' if prof in ('text', 'ted-export-r209') else (x.get('source') or 'ted'),
                            'publication_id': next((v.get('publication_id') for v in x['versions'] if v['caused_by_notice_id'] == nid), '') or ''})
        proj.write(json.dumps({'id': tid, 'source': x.get('source'), 'notices': notices, 'extras': extras}) + '\n')
    print('exported', len(pop) - len(skipped), 'skipped', skipped)


if __name__ == '__main__':
    main()
