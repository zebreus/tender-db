# Issue 545 unit 2a — proposed verdict per candidate, with the guards applied as code.
#
# usage: python3 -I verdicts.py            (writes verdicts.json + verdicts.txt; lists refcount needs)
#
# Inputs (all in this directory unless noted):
#   cand_judged.json   the 208 prefilter-kept candidates with their hand class (judge2a.py)
#   r7_sec.out.json    per (org, notice): mention rows (section, name, country), party and bid-party rows
#   r8_dest.out.json   destination lookups (name_norm range seeks)
#   r9_*.out.json      mention counts of tied destination rows (most-referenced tiebreak), plus the
#                      unit-1 twice-held F keys' sections/roles and lookups
#   ../q4_lookup.out.json, ../q6_froles.out.json   unit 1's destination and role reads (carried candidates)
#
# Guards, in order (the triage's skeptic points, binding):
#   G0 class: V/T/C -> NOT-545 (dropped); U/A/F? -> PARK (sub-unit/group/sister, agent-principal, unknown relation)
#   G1 shared-section collision: if the (notice, section) a candidate mention sits on is bound by more than one
#      party role (tender_version_parties roles, plus bid-party roles, 'tenderer' == 'Tenderer'), PARK — issue
#      317 moves the mention, so every party bound to that section would move with it (Bedford/Comensura).
#   G2 buyer-side on a public buyer's identifier (council on council, trust on trust, body on department):
#      PARK as possible shared procurement (one body running another's tender under its own account is a
#      practice, not a one-off copy-paste, and recurs on new notices).
#   G3 destination: (a) exactly one identified (non-provisional) row whose vote key equals the mention's and
#      whose country equals the mention's -> that row; (b) several such rows -> the most-referenced one
#      (mention count) if it is a unique maximum; (c) no identified row -> provisional rows with exactly the
#      mention's name_norm and the same country: one -> it, several -> most-referenced unique maximum;
#      otherwise PARK (no destination / tie).
#   Result: REHOME-HIGH (destination id) or PARK-<reason> or NOT-545.
import json, glob, os, sys
from collections import defaultdict
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from keys import vote_key

D = os.path.dirname(os.path.abspath(__file__)) + '/'

def name_norm(s):  # canonical.rs org_name_norm
    c = ' '.join(s.lower().split())
    t = c.rstrip('.,; \t\n')
    return t if t else c

def load_rows(path):
    j = json.load(open(path))
    assert 'rows' in j and 'columns' in j and not j.get('truncated'), path
    return j['columns'], j['rows']

# ---- r7: sections and roles ------------------------------------------------------------------------
mentions = defaultdict(list)   # (org, notice) -> [(section, name, country)]
roles = defaultdict(set)       # (notice, section) -> {role}
party_rows = defaultdict(list) # (notice, section) -> [(tender, seq, lot, role)]
_, rows = load_rows(D + 'r7_sec.out.json')
for k, a, b, c, d, e, f, g in rows:
    kind, org, notice = k.split('|'); org = int(org); notice = int(notice)
    if kind == 'm':
        mentions[(org, notice)].append((b, d, e))
    elif kind == 'p':
        roles[(e, f)].add(d); party_rows[(e, f)].append((a, b, c, d))
    elif kind == 'b':
        roles[(e, f)].add('Tenderer' if d == 'tenderer' else 'bid:' + d)

# ---- r8 + q4: destination rows ---------------------------------------------------------------------
dest_rows = {}  # id -> (country, identifier, provisional, name)
_, rows = load_rows(D + 'r8_dest.out.json')
for q, i, country, ident, prov, name, nn in rows:
    dest_rows[i] = (country, ident, prov, name)
_, rows = load_rows(D + '../q4_lookup.out.json')
for q, i, country, ident, prov, name in rows:
    dest_rows.setdefault(i, (country, ident, prov, name))

refcount = {}
extra_mentions = []  # r9 join rows for the unit-1 twice-held keys
for p in sorted(glob.glob(D + 'r9_*.out.json')):
    cols, rows = load_rows(p)
    for r in rows:
        if r[0].startswith('rc|'):
            refcount[int(r[0][3:])] = r[1]
        elif r[0].startswith('n:'):
            dest_rows.setdefault(r[1], (r[2], r[3], r[4], r[5]))
        elif r[0].startswith('j|'):
            extra_mentions.append(r)
for r in extra_mentions:  # j|org: notice, section, name, country, tender, seq, lot, role
    _, org = r[0].split('|'); org = int(org)
    notice, section, name, country, tender, seq, lot, role = r[1:9]
    if (section, name, country) not in mentions[(org, notice)]:
        mentions[(org, notice)].append((section, name, country))
    if role is not None:
        roles[(notice, section)].add(role); party_rows[(notice, section)].append((tender, seq, lot, role))

def id_kind(ident):
    """(kind, normalized) of an identifier, to tell split rows of ONE entity from distinct entities."""
    i = (ident or '').upper().replace(' ', '')
    if i.startswith('GBCOH'): return ('coh', i[5:].lstrip('0'))
    if i.startswith('GBPPON'): return ('ppon', i[6:])
    if i.startswith('GBNHS'): return ('nhs', i[5:])
    if i.startswith('GB') and i[2:].isdigit() and len(i[2:]) in (9, 12): return ('vat', i[2:])
    if i.isdigit() and len(i) <= 8: return ('coh', i.lstrip('0'))
    if i.isdigit() and len(i) in (9, 12): return ('vat', i)
    return ('other', i)

def distinct_entities(pool):
    seen = defaultdict(set)
    for i in pool:
        k, v = id_kind(dest_rows[i][1]); seen[k].add(v)
    return any(len(vs) > 1 for vs in seen.values())

def toks(s):
    return set(vote_key(s).split())

def near_identified(name, country):
    """identified same-country rows in the lookups whose vote-key tokens contain, or are contained in,
    the mention's (a near match the exact rule does not take): a hint for the reviewer, and a reason
    not to send a mention to a provisional row."""
    mt = toks(name); out = []
    import difflib
    for i, (c, idf, prov, n) in dest_rows.items():
        if prov or not idf or c != country: continue
        rt = toks(n)
        if not mt or not rt or rt == mt: continue
        if mt <= rt or rt <= mt or difflib.SequenceMatcher(None, vote_key(n), vote_key(name)).ratio() >= 0.9:
            out.append(i)
    return sorted(out)

def destination(name, country):
    vk = vote_key(name); nn = name_norm(name)
    ident = sorted(i for i, (c, idf, prov, n) in dest_rows.items()
                   if not prov and idf and c == country and vote_key(n) == vk)
    pool, kind = (ident, 'identified') if ident else (
        sorted(i for i, (c, idf, prov, n) in dest_rows.items() if prov and c == country and name_norm(n) == nn),
        'provisional-exact')
    if not pool:
        return None, 'PARK-no-destination', [], []
    if kind == 'provisional-exact' and near_identified(name, country):
        return None, 'PARK-destination-ambiguous (identified near-match exists)', pool, []
    if len(pool) == 1:
        return pool[0], kind + '-unique', pool, []
    if kind == 'identified' and distinct_entities(pool):
        return None, 'PARK-distinct-entities (tied rows carry different numbers)', pool, []
    need = [i for i in pool if i not in refcount]
    if need:
        return None, 'NEEDS-REFCOUNT', pool, need
    best = max(refcount[i] for i in pool)
    top = [i for i in pool if refcount[i] == best]
    if len(top) == 1:
        return top[0], kind + '-tiebreak-most-referenced', pool, []
    return None, 'PARK-destination-tie', pool, []

# Buyer-side F on a public buyer's own identifier (G2), by (org, name)
SHARED_PROCUREMENT = {
 (30917785, 'Castle Point Borough Council'), (30917785, 'Castlepoint Borough Council'),  # on Braintree DC
 (30916395, 'Uttlesford District Council'),       # on Chelmsford CC
 (30914669, 'Hartlepool Borough Council'),        # on North Tyneside BC
 (31549806, 'Alder Hey'),                         # on The Walton Centre NHS FT
 (30914094, 'Betsi Cadwaladr University Health Board'),  # on Bangor University
 (31556399, 'Waterways Ireland'),                 # on Department of Justice (NI)
}

def judge(c):
    v = {k: c[k] for k in ('org', 'ident', 'head', 'name', 'scheme', 'n', 'detector', 'class')}
    v['source'] = c.get('source', 'u2a')
    cls = c['class']
    if cls in ('V', 'T', 'C'):
        v['verdict'] = 'NOT-545'; return v
    if cls in ('U', 'A', 'F?'):
        v['verdict'] = {'U': 'PARK-sub-unit/group/sister', 'A': 'PARK-agent-principal',
                        'F?': 'PARK-relation-unknown'}[cls]
        if c.get('note'): v['note'] = c['note']
        return v
    assert cls == 'F'
    notices = sorted({c['first_notice'], c['last_notice']} - {None})
    if not notices:
        v['verdict'] = 'PARK-evidence-missing'; return v
    ev = []; collision = False; country = None; unbound = False
    for n in notices:
        ms = [m for m in mentions.get((c['org'], n), []) if m[1] == c['name']]
        if not ms:
            ev.append({'notice': n, 'error': 'mention row not found'}); continue
        for section, _, mc in ms:
            country = country or mc
            rs = sorted(roles.get((n, section), set()))
            pr = sorted({(t, l, r) for t, s, l, r in party_rows.get((n, section), [])})
            ev.append({'notice': n, 'section': section, 'roles': rs,
                       'tenders': sorted({t for t, _, _ in pr})})
            if len(rs) > 1: collision = True
            if not rs: unbound = True
    v['evidence'] = ev
    v['side'] = 'buyer' if any('Procedure-Buyer' in e.get('roles', []) for e in ev) else (
        'supplier' if any('Tenderer' in e.get('roles', []) for e in ev) else 'unbound')
    if any('error' in e for e in ev):
        v['verdict'] = 'PARK-evidence-missing'; return v
    if collision:
        v['verdict'] = 'PARK-shared-section'; return v
    if (c['org'], c['name']) in SHARED_PROCUREMENT:
        v['verdict'] = 'PARK-shared-procurement'
    d, why, pool, need = destination(c['name'], country or 'GB')
    v['destination_rule'] = why
    v['destination_pool'] = [(i,) + tuple(dest_rows[i]) + ((refcount[i],) if i in refcount else ()) for i in pool]
    if need: v['needs_refcount'] = need
    hint = near_identified(c['name'], country or 'GB')
    if hint and d is None: v['near_identified_hint'] = [(i,) + tuple(dest_rows[i]) for i in hint]
    if 'verdict' in v:
        if d: v['would_be_destination'] = d
        return v
    if d is None:
        v['verdict'] = why; return v
    v['verdict'] = 'REHOME-HIGH'; v['destination'] = d
    v['destination_row'] = dest_rows[d]
    if unbound: v['note'] = 'no party row binds this mention: the move changes the org\'s names only'
    return v

# ---- unit 1's PPON-slice candidates, carried with the skeptic's corrections --------------------------
CARRIED = [
 dict(org=30913706, ident='GBPPONPDQJ7126JDHG', head='UK Research & Innovation', name='Jeol (U.K.) Limited', notices=[46834978], cls='F'),
 dict(org=31560149, ident='GBPPONPCHZ9218WJNZ', head="Dewi's Ltd", name='Derek Lewis Ltd', notices=[46831537], cls='F?', note='skeptic: probable relation to holder'),
 dict(org=31549890, ident='GBPPONPCLN8377ZPPT', head='Institute for Fiscal Studies', name='Pensions Policy Institute', notices=[46807946], cls='F'),
 dict(org=31561172, ident='GBPPONPCXQ4528BMJJ', head='Primary Healthcare 21', name='Healthcare 21 NI', notices=[46826106], cls='F?', note='skeptic: likely sister company'),
 dict(org=30914626, ident='GBPPONPDNG5431HGYV', head='Torfaen County Borough Council', name='Equal Education Partners ', notices=[47293389], cls='F'),
 dict(org=30913629, ident='GBPPONPDPT3135BWWY', head='NHS England', name='Portsmouth Hospitals University NHS Trust', notices=[46832653], cls='F'),
 dict(org=31564015, ident='GBPPONPDVV9229ZLDD', head='Placing Talent', name='QCI (UK) LIMITED', notices=[46870095], cls='F'),
 dict(org=30916371, ident='GBPPONPDXZ9391VHZQ', head='Rhondda Cynon Taf CBC', name='WATKINS COACHES LTD', notices=[46755521], cls='F'),
 dict(org=31549890, ident='GBPPONPCLN8377ZPPT', head='Institute for Fiscal Studies', name='Centre for Economics and Business Research Ltd', notices=[46760965], cls='C', n=2, note='skeptic: consortium-like (IFS Tenderer on the same lot of 8617082)'),
 dict(org=31546270, ident='GBPPONPDBW5798GPLT', head='Cumbria, Northumberland, Tyne and Wear NHS Foundation Trust', name='Pagabo', notices=[46844727], cls='A', n=2),
 dict(org=30913753, ident='GBPPONPDNN2773HVYN', head='Ministry of Justice', name='WAVEMAKER LIMITED t/a WPP 650', notices=None, cls='F', n=2),
 dict(org=30953754, ident='GBPPONPDZV7449YCMT', head='HM Land Registry', name='Charlton Park Academy', notices=None, cls='F', n=2),
 dict(org=31555134, ident='GBPPONPCYV2926WZMP', head='South Tees Development Corporation', name='Tees Valley Combined Authority', notices=[46819605], cls='U'),
 dict(org=30913801, ident='GBPPONPDNG8971ZVCR', head='Tees Valley Combined Authority', name='South Tees Development Corporation', notices=[46774084], cls='U'),
 dict(org=30913801, ident='GBPPONPDNG8971ZVCR', head='Tees Valley Combined Authority', name='Hartlepool Development Corporation', notices=[46434375], cls='U'),
]
# unit 1's q6 party rows for the carried notices
_, rows = load_rows(D + '../q6_froles.out.json')
for k, tender, seq, lot, role, mn, ms in rows:
    roles[(mn, ms)].add(role); party_rows[(mn, ms)].append((tender, seq, lot, role))
_, rows = load_rows(D + '../q5_fmentions.out.json')
for k, notice, section, org, name, raw, scheme in rows:
    if (section, name, 'GB') not in mentions[(org, notice)]:
        mentions[(org, notice)].append((section, name, 'GB'))  # q5 read no country; all GB-PPON

def carried():
    out = []
    for c in CARRIED:
        notices = c['notices']
        if notices is None:  # unit-1 twice-held F: notices come from r9's join arms
            notices = sorted({r[1] for r in extra_mentions if int(r[0].split('|')[1]) == c['org'] and r[3] == c['name']})
        out.append({'org': c['org'], 'ident': c['ident'], 'head': c['head'], 'name': c['name'], 'scheme': 'GB-PPON',
                    'n': c.get('n', 1), 'detector': 'HELD-2' if c.get('n', 1) == 2 else 'STRAY', 'class': c['cls'],
                    'first_notice': notices[0] if notices else None, 'last_notice': notices[-1] if notices else None,
                    'source': 'unit1-carried', 'note': c.get('note')})
    return out

# ---- issue 456's exhibit ---------------------------------------------------------------------------
EXHIBIT_456 = {'org': 5718658, 'ident': '01002607', 'head': 'Sellafield Ltd', 'name': 'Schneider Electric',
               'scheme': 'GB-PPON', 'n': 1, 'detector': 'exhibit-456', 'class': 'F', 'first_notice': 46966800,
               'last_notice': 46966800, 'source': 'issue-456'}

if __name__ == '__main__':
    cands = json.load(open(D + 'cand_judged.json'))
    allc = cands + carried() + [EXHIBIT_456]
    out = [judge(c) for c in allc]
    json.dump(out, open(D + 'verdicts.json', 'w'), indent=1, ensure_ascii=False, default=list)
    from collections import Counter
    print(Counter(v['verdict'] for v in out))
    print('by source', Counter((v['source'], v['verdict']) for v in out if v['verdict'] != 'NOT-545'))
    need = sorted({i for v in out for i in v.get('needs_refcount', [])})
    print('NEEDS REFCOUNT', len(need), need)
    with open(D + 'verdicts.txt', 'w') as f:
        for v in out:
            if v['verdict'] == 'NOT-545': continue
            f.write(f"{v['verdict']:28s} [{v['class']:2s}] {v['source']:13s} {v['org']} {v['head'][:38]!r} <- {v['name']!r} x{v['n']}"
                    f" side={v.get('side', '-')} dest={v.get('destination', v.get('would_be_destination', '-'))}"
                    f" rule={v.get('destination_rule', '-')}\n")
            for e in v.get('evidence', []):
                f.write(f"      evidence {e}\n")
            for p in v.get('destination_pool', []):
                f.write(f"      pool {p}\n")
            for p in v.get('near_identified_hint', []):
                f.write(f"      hint {p}\n")
            if v.get('note'): f.write(f"      note {v['note']}\n")
