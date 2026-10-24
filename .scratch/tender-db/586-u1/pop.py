"""586 unit 1 (b): the touchpoint-name populations in the saved answers, read locally (no box, no network).

Each population is a list of dicts {pop, key, name, n, parent, cls, org} where `name` is the touchpoint name the
gate would see, `n` its mention / reference weight, `parent` the touchpoint's parent ORG name when known.
"""
import glob
import json
import os

ROOT = '/home/user/tender-db/.scratch/tender-db'
AUD = f'{ROOT}/audit-576-fanout'

# Rust `store::fold_latin` (canonical.rs:45290), char for char.
_LAT = {}
for chars, base in [
    ('àáâãäåāăąæ', 'a'), ('çćčĉ', 'c'), ('ďđ', 'd'), ('èéêëěęēė', 'e'), ('ğģ', 'g'), ('ìíîïıīį', 'i'), ('ķ', 'k'),
    ('ĺľłļ', 'l'), ('ñńňņ', 'n'), ('òóôõöøőœ', 'o'), ('ŕř', 'r'), ('śšşș', 's'), ('ťţț', 't'), ('ùúûüůűūų', 'u'),
    ('ýÿ', 'y'), ('źżž', 'z'), ('ß', 's'),
]:
    for c in chars:
        _LAT[c] = base
# Rust `ingest::project::fold_greek_tonos`.
_GREEK = {'ά': 'α', 'έ': 'ε', 'ή': 'η', 'ί': 'ι', 'ϊ': 'ι', 'ΐ': 'ι', 'ό': 'ο', 'ύ': 'υ', 'ϋ': 'υ', 'ΰ': 'υ', 'ώ': 'ω',
          'ς': 'σ'}


def fold(name):
    """`partyname::fold` / `role_census::fold`: `match_norm` (lowercase, every non-alphanumeric run one space,
    Greek tonos) then `buyer_name_fold` (Latin diacritics)."""
    out, gap = [], False
    for c in name:
        if c.isalnum():
            if gap and out:
                out.append(' ')
            gap = False
            for low in c.lower():
                low = _GREEK.get(low, low)
                out.append(_LAT.get(low, low))
        else:
            gap = True
    return ''.join(out)


def org_name_norm(name):
    c = ' '.join(name.lower().split())
    s = c
    while s and (s[-1] in '.,;' or s[-1].isspace()):
        s = s[:-1]
    return s if s else c


def _rows(path):
    d = json.load(open(path))
    return [dict(zip(d['columns'], r)) for r in d['rows']]


def new_orgs():
    """The 3,751 orgs minted 31974972..31978722 (second-look t2: name, its touchpoint mentions, its parent)."""
    out = []
    for r in json.load(open(f'{AUD}/second-look/raw/t2_classified.json')):
        out.append(dict(pop='NEW', key=r['id'], org=r['id'], name=r['name'], n=r['tpo'], parent=r['pp'], cls=r['cls']))
    return out


def windows():
    """Two fresh notice windows: every touchpoint mention and the org it bound (existing or new), as printed."""
    out = []
    for tag, f in [('WIN-A', '03-t3_window_a-classified.json'), ('WIN-B', '05-t5_window_b-classified.json')]:
        for r in json.load(open(f'{AUD}/second-look/raw/{f}')):
            out.append(dict(pop=tag, key=(r['org'], r['porg']), org=r['org'], name=r['tp_printed'], n=r['n'],
                            parent=r['p_printed'], cls=r['cls'], bound=r['oname']))
    return out


def r3_top60():
    """576 unit 1's R3: the top 60 different-name touchpoint groups by references (the D576-1 (iv) bar)."""
    out = []
    for i, r in enumerate(_rows(f'{ROOT}/576-triage/r3.out')):
        out.append(dict(pop='R3', key=i + 1, org=None, name=r['tpo_name'], n=r['refs'], parent=r['org_name'],
                        cls=r['role']))
    return out


def clusters():
    """Refute's 124 loose clusters of new orgs (class S/N/X/D was assigned in its answer; X = one tribunal's
    spellings, 23 clusters / 50 orgs / 340 mentions)."""
    out = []
    for ci, c in enumerate(json.load(open(f'{AUD}/refute/clusters.json'))):
        for oid, n, name in c['members']:
            out.append(dict(pop='CLU', key=(ci, oid), org=oid, name=name, n=n, parent=None, cls=c['key'], cluster=ci))
    return out


def u4():
    """576 unit 4's read-back: the served touchpoint parties of its six windows and the mention names it read."""
    out = []
    for f in sorted(glob.glob(f'{ROOT}/576-u4/raw/sql/s0[1-7]_*.out')):
        for r in _rows(f):
            out.append(dict(pop='U4', key=(r['tender_id'], r['role'], r['org_id']), org=r['org_id'], name=r['org_name'],
                            n=r['n_rows'], parent=None, cls=r['role']))
    for r in _rows(f'{ROOT}/576-u4/raw/sql/s08_mentions.out'):
        out.append(dict(pop='U4m', key=(r['notice_id'], r['section_id']), org=r['organization_id'], name=r['name'], n=1,
                        parent=None, cls=r['section_id']))
    return out


def existing_bound():
    """Older rows 2759's touchpoints bound (measurer s09, second-look t4) and the 97 same-name twins (refute r5)."""
    out = []
    for r in _rows(f'{AUD}/box/raw/09-s09_pre_names.json'):
        out.append(dict(pop='PRE', key=r['id'], org=r['id'], name=r['name'], n=1, parent=None, cls='s09'))
    for r in _rows(f'{AUD}/second-look/raw/04-t4_served.json'):
        out.append(dict(pop='PRE', key=r['x'], org=r['x'], name=r['name'], n=r['band_2759'], parent=None, cls='t4'))
    for r in _rows(f'{AUD}/refute/raw/05-r5_premin_pk.json'):
        out.append(dict(pop='PRE', key=r['id'], org=r['id'], name=r['name_norm'], n=1, parent=None, cls='r5'))
    return out


def all_pops():
    return new_orgs() + windows() + r3_top60() + clusters() + u4() + existing_bound()


if __name__ == '__main__':
    import collections
    pops = all_pops()
    c = collections.Counter(p['pop'] for p in pops)
    w = collections.Counter()
    for p in pops:
        w[p['pop']] += p['n'] or 0
    for k in c:
        print(k, c[k], w[k])
