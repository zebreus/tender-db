# 548 U1c: the consortium_removed token-subset release test, written down and applied to the held listing.
# Read-only. Input: ../org-name-refresh-plan-2026-10-16.json.gz (epoch 524u2-v12, rev 3475272), ../u1/shippable.json
# (U1's skeptic-corrected ship filter; reproduced below as `ships`), ../../524-v11/skip-ids.tsv.
# Output: u1c/subset.json (counts, every matching row, the salted 30), stdout table.
#
# THE TEST (a row held as `consortium_removed` would be released):
#   tokens(new) - LEGAL_FORMS  is non-empty  AND  is a subset of  tokens(old)
# where `old` is the held head (old_name) and `new` the spelling the change would serve (new_name).
#
# tokens(s):
#   1. fold: lower-case, then the Latin diacritic fold (NFKD, combining marks dropped; plus the
#      non-decomposing letters l-stroke, o-slash, d-stroke, sharp s, ae, oe, dotless i, thorn, eth),
#      which is what crosswalk::fold_diacritics does for these letters;
#   2. split on every character that is not a letter or digit (Python \W, underscore also a separator);
#   3. join each RUN of consecutive single-letter tokens into one token (`S.A.` -> `sa`, `S. L. U.` ->
#      `slu`, `A/S` -> `as`, `Ges. m. b. H.` -> `gesmbh` is NOT made: `ges` is 3 letters; `m b h` -> `mbh`),
#      as crosswalk::folded_family reads a name's single-letter runs. A single DIGIT is not joined.
#   tokens(new) is a SET; legal-form tokens are dropped from the NEW side only (the head may carry any).
#
# LEGAL_FORMS: the two lists the head election already uses, verbatim (crates/ingest/src/crosswalk.rs):
#   family_token's keys (the bind guard family's table) and R2_LEGAL_FORM_TOKENS (head_r2_hold's list).
# EXTRA_FORMS (sensitivity only, NOT part of the test): `sau` (S.A.U.), `co` (& Co. KG), `lda`, `sprl`,
#   `bvba`, `cvba`; reported as a second count so U3 can see what a wider list would add.
import json, gzip, hashlib, datetime, unicodedata, re, collections, sys

D = '/home/user/tender-db/.scratch/tender-db/548-triage/'
FAMILY_TOKEN_KEYS = ['gmbh', 'mbh', 'ag', 'sarl', 'eurl', 'sas', 'sasu', 'oy', 'oyj', 'ab', 'abp', 'uab', 'aps', 'bv',
                     'nv', 'kft', 'zrt', 'nyrt', 'spa', 'srl', 'sia', 'sro', 'spzoo']
R2_LEGAL_FORM_TOKENS = ['sa', 'srl', 'spa', 'sas', 'sarl', 'ag', 'bv', 'nv', 'ab', 'as', 'oy', 'kg', 'gmbh', 'ltd', 'plc',
                        'llp', 'inc', 'sl', 'slu', 'sca', 'scs', 'scg', 'spl', 'snc', 'scop', 'sci', 'selarl', 'selas',
                        'eurl', 'sasu', 'pph', 'fuh', 'phu', 'zoz', 'spzoz', 'spz', 'ett', 'grsk']
LEGAL_FORMS = frozenset(FAMILY_TOKEN_KEYS) | frozenset(R2_LEGAL_FORM_TOKENS)
EXTRA_FORMS = frozenset(['sau', 'co', 'lda', 'sprl', 'bvba', 'cvba'])
NON_DECOMPOSING = {'ł': 'l', 'ø': 'o', 'đ': 'd', 'ß': 'ss', 'æ': 'ae', 'œ': 'oe', 'ı': 'i', 'þ': 'th', 'ð': 'd'}


def fold(s):
    s = ''.join(NON_DECOMPOSING.get(c, c) for c in s.lower())
    return ''.join(c for c in unicodedata.normalize('NFKD', s) if not unicodedata.combining(c))


def tokens(s):
    out, run = [], ''
    for t in (t for t in re.split(r'[\W_]+', fold(s)) if t):
        if len(t) == 1 and t.isalpha():
            run += t
            continue
        if run:
            out.append(run)
            run = ''
        out.append(t)
    if run:
        out.append(run)
    return out


def subset(old, new, forms=LEGAL_FORMS):
    n = {t for t in tokens(new) if t not in forms}
    return bool(n) and n <= set(tokens(old))


# --- U1's ship filter (u1/filter.py `cut`), reproduced so this file stands alone; checked against shippable.json.
o = json.load(gzip.open(D + 'org-name-refresh-plan-2026-10-16.json.gz'))
b = o['body']
b = json.loads(b) if isinstance(b, str) else b
as_of = b['as_of']
day = lambda s: None if s is None else datetime.date.fromisoformat(s).toordinal() - datetime.date(1970, 1, 1).toordinal()
cur_cut, stale_cut = as_of // 86400 - 1095, as_of // 86400 - 365


def cut(r):
    if r['rule'] not in ('R0e', 'R0n', 'R1'): return 'rule_' + r['rule']
    if r['recent_released'] == 'unfit': return 'released_unfit'
    if r['rule'] != 'R1': return None
    if r['recent_released'] is None:
        hn = day(r['head_newest'])
        if hn is not None and hn >= cur_cut: return 'head_current'
        tn = day(r['top_newest'])
        if r['clusters'] > 1 + (1 if r['head'] > 0 else 0) and (tn is None or tn < stale_cut): return 'top_stale'
    return None


C = [r for r in b['held'] if r['reason'] == 'consortium_removed']
ship_u1 = {r['org_id'] for r in json.load(open(D + 'u1/shippable.json'))['consortium_removed']}
ships = {r['org_id'] for r in C if cut(r) is None}
assert ships == ship_u1, 'ship filter drifted from u1/shippable.json'

skip = set()
for line in open('/home/user/tender-db/.scratch/tender-db/524-v11/skip-ids.tsv'):
    f = line.split('\t')
    if f[0].isdigit(): skip.add(int(f[0]))

SALT = '548-u1c-'
key = lambda r: hashlib.sha256((SALT + str(r['org_id'])).encode()).hexdigest()


def tally(forms):
    m = [r for r in C if subset(r['old_name'], r['new_name'], forms)]
    ms = [r for r in m if r['org_id'] in ships]
    return m, ms, {
        'full_listing_rows': len(m), 'full_R1': sum(r['rule'] == 'R1' for r in m), 'full_R2': sum(r['rule'] == 'R2' for r in m),
        'full_mentions': sum(r['n'] for r in m),
        'full_by_cut': dict(collections.Counter(cut(r) or 'ships' for r in m)),
        'shippable_rows': len(ms), 'shippable_mentions': sum(r['n'] for r in ms),
        'shippable_H<=1': sum(r['head'] <= 1 for r in ms)}


m, ms, t = tally(LEGAL_FORMS)
_, _, t_extra = tally(LEGAL_FORMS | EXTRA_FORMS)
_, _, t_noforms = tally(frozenset())
pool = sorted([r for r in ms if r['org_id'] not in skip], key=key)
salted = pool[:30] if len(ms) > 60 else ms
unmatched_ship = [r for r in C if r['org_id'] in ships and r not in ms]

print('consortium_removed held', len(C), 'R1', sum(r['rule'] == 'R1' for r in C), 'R2', sum(r['rule'] == 'R2' for r in C), 'shippable', len(ships))
print('TEST (code legal-form lists):', t)
print('  + EXTRA_FORMS            :', t_extra)
print('  no legal-form drop       :', t_noforms)
print('matched shippable on skip-ids:', sorted(r['org_id'] for r in ms if r['org_id'] in skip))
print('read set:', 'salted 30' if len(ms) > 60 else 'all', [r['org_id'] for r in salted])
print('CPR 18287356 matched:', any(r['org_id'] == 18287356 for r in m))
# U3's by_hold check: a release takes EVERY row matching its predicate out of that reason (shippable or not: the rest move on to
# head_current, top_stale, a serve hold or an R2 change). Predicted drops over the FULL listing, not the shippable counts.
H = b['held']
fam = [r for r in H if r['reason'] == 'family' and r['head'] <= 1]
gbf = [r for r in H if r['reason'] == 'gb_family']
pred = {'gb_family_whole': {'orgs': -len(gbf), 'mentions': -sum(r['n'] for r in gbf), 'of': b['by_hold']['gb_family']},
        'family_H<=1': {'orgs': -len(fam), 'R1': sum(r['rule'] == 'R1' for r in fam), 'R2': sum(r['rule'] == 'R2' for r in fam),
                        'mentions': -sum(r['n'] for r in fam), 'of': b['by_hold']['family']},
        'consortium_subset': {'orgs': -t['full_listing_rows'], 'R1': t['full_R1'], 'R2': t['full_R2'], 'mentions': -t['full_mentions'],
                              'with_extra_forms_orgs': -t_extra['full_listing_rows'], 'of': b['by_hold']['consortium_removed']},
        'shippable_changes_not_drops': {'gb_family': 7, 'family_H<=1': sum(1 for r in fam if cut(r) is None), 'consortium_subset': t['shippable_rows']},
        'cross_guard': 'a family row released at H<=1 meets the LATER bind guards next; 15783942 (Consortium consisting of Kontron ... -> '
                       'Kontron Transportation GmbH, H=1) would move family -> consortium_removed unless the consortium subset release '
                       'ships too (it matches the test). gb_family 15909819 (EY UK LLP -> Ernst & Young Limited) meets no later guard.'}
print('U3 by_hold predictions:', json.dumps(pred))
row = lambda r: {k: r[k] for k in ('org_id', 'rule', 'head', 'n', 'identifier', 'country', 'old_name', 'new_name', 'spellings', 'head_newest', 'top_newest')}
json.dump({'meta': {'test': 'tokens(new) - LEGAL_FORMS non-empty and subset of tokens(old); see header', 'salt': SALT,
                    'legal_forms': sorted(LEGAL_FORMS), 'extra_forms_sensitivity': sorted(EXTRA_FORMS)},
           'counts': t, 'by_hold_predictions': pred, 'counts_with_extra_forms': t_extra, 'counts_without_form_drop': t_noforms,
           'salted_30': [r['org_id'] for r in salted],
           'matched_full': [row(r) | {'ships': r['org_id'] in ships} for r in m],
           'shippable_not_matched': [row(r) for r in unmatched_ship]},
          open(D + 'u1c/subset.json', 'w'), ensure_ascii=False, indent=1)
