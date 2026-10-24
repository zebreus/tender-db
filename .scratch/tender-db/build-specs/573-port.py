# 573 U3 build spec helper: a Python port of the proposed `store::head_dropped_accent` and the R0d
# trigger (R1's bars + a dropped-accent form of a spelling of the elected cluster), run over a stored
# `org-name-refresh-plan` report. Offline, read-only: no network, no box.
#
#   python3 -I 573-port.py <plan.json[.gz]>
#
# On a v13 plan (e.g. 548-triage/org-name-refresh-plan-2026-10-19-v13.json.gz, plan 2653) it predicts
# the R0d set from the R1-elected entries (planned + held); on a v14 plan it also lists the entries the
# planner elected R0d and diffs them against the prediction from the same plan's listing.
# Limits: an entry lists only its top 5 spellings, and "S is in the elected cluster" is approximated by
# a diacritic-folded word key equal to the entry's new_name's (the real test reads every spelling of
# the elected cluster, so the planner may find a few more; the spec says each extra is read first).
import gzip, json, re, sys, unicodedata

path = sys.argv[1]
raw = gzip.open(path).read() if path.endswith('.gz') else open(path, 'rb').read()
doc = json.loads(raw)
b = json.loads(doc['body']) if 'body' in doc and isinstance(doc['body'], str) else doc


def collapse(s):
    return ' '.join(s.split())


def dropped(head, spelling):
    """head_dropped_accent: spelling minus every non-ASCII char == head (whitespace collapsed, ASCII
    case-insensitive; a head with a non-ASCII char never matches); some word lost a letter and kept an
    ASCII one; no word of 2+ non-ASCII letters and no ASCII letter."""
    stripped = ''.join(c for c in spelling if ord(c) < 128)
    h = collapse(head)
    if not h.isascii() or collapse(stripped).lower() != h.lower():
        return False
    partial = False
    for w in spelling.split():
        lost = sum(1 for c in w if ord(c) > 127 and c.isalpha())
        kept = any(ord(c) < 128 and c.isalpha() for c in w)
        if lost >= 2 and not kept:
            return False
        if lost >= 1 and kept:
            partial = True
    return partial


def folded(s):
    s = unicodedata.normalize('NFKD', s)
    s = ''.join(c for c in s if unicodedata.category(c) != 'Mn').casefold()
    return ' '.join(re.findall(r'\w+', s))


predicted, actual = [], []
for sec in ('planned', 'held'):
    for r in b.get(sec, []):
        head = r.get('old_name') or ''
        if r.get('rule') == 'R0d':
            actual.append((r['org_id'], sec, r['verdict'], r['reason'], head, r.get('new_name')))
        if r.get('rule') not in ('R1', 'R0d') or not r.get('new_name'):
            continue
        if any(s != head and dropped(head, s) and folded(s) == folded(r['new_name']) for s, _ in r['spellings']):
            predicted.append((r['org_id'], sec, '%s:%s' % (r['verdict'], r['reason']), head, r['new_name'],
                              r['voting'], r['top'], r['head'], r.get('n')))

print('epoch', b.get('epoch'), 'as_of', b.get('as_of_date'), 'rev', b.get('rev'))
print('predicted R0d (R1-elected entries whose head drops the accents of a top spelling):', len(predicted))
for p in predicted:
    print('  ', p)
print('  mentions (sum of n):', sum(p[8] or 0 for p in predicted))
# Issue 573 unit 2's decided set (size.md, plan 2653): the v14 re-plan must elect exactly these R0d.
DECIDED = {22397161, 22399787, 22399788, 22433242, 22435268, 22441088, 22441089, 22460863, 22466820, 22466821,
           22488363, 22499321, 22499322, 22527505, 22527506}
if actual or str(b.get('epoch', '')).endswith('v14'):
    print('planner R0d entries:', len(actual), ' held with rule R0d:', sum(1 for a in actual if a[1] == 'held'))
    for a in actual:
        print('  ', a)
    ai = {a[0] for a in actual if a[1] == 'planned'}
    print('vs the decided 15 -- extra (read before the wet run):', sorted(ai - DECIDED),
          ' missing (explain from the entry or GET /v1/organizations/<id>):', sorted(DECIDED - ai))
    print('wet org_ids (decided and planned):', sorted(ai & DECIDED))
