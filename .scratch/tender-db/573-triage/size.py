# 573 U1: size the dropped-accent / mojibake head class over plan 2653 (v13), offline, python only.
import gzip, json, hashlib, datetime, unicodedata, re, sys, collections

REPO = '/home/user/tender-db/'
PLAN = REPO + '.scratch/tender-db/548-triage/org-name-refresh-plan-2026-10-19-v13.json.gz'
OUT = sys.argv[1] if len(sys.argv) > 1 else REPO + '.scratch/tender-db/573-triage/size.json'
SALT = '573-u1-'

b = json.loads(json.load(gzip.open(PLAN))['body'])
assert b['epoch'] == '524u2-v13' and not b['held_truncated'] and not b['truncated']
as_day = b['as_of'] // 86400
def day(s):
    return None if s is None else datetime.date.fromisoformat(s).toordinal() - datetime.date(1970, 1, 1).toordinal()

BIND = {'family', 'gb_family', 'consortium_removed', 'nhs_check', 'placeholder_id'}
SERVE = {'review_body', 'organ', 'code_added', 'address_added', 'caps_variant', 'doubled', 'seat_dropped'}
JUNK_ASKED = {'recent', 'residue', 'head_current'}   # holds where the junk exemption is asked today

# ---- the shape tests ------------------------------------------------------------------------------
def drop_non_ascii_letters(w):
    # delete the word's non-ASCII letters (and combining marks, for decomposed input)
    return ''.join(c for c in w if not (ord(c) > 127 and unicodedata.category(c)[0] in 'LM'))

def dropped_accent(head, spelling, loose=False):
    """Each head word == the corresponding spelling word with its non-ASCII letters deleted (case-folded),
    at least one word differing. A spelling word made only of non-ASCII letters (`à`) deletes to nothing
    and is dropped from the sequence (flagged). loose=True also lets a head word equal its spelling word
    as-is (a partial drop)."""
    hw = head.split()
    sw = spelling.split()
    dele = [drop_non_ascii_letters(w) for w in sw]
    emptied = [d for d in dele if d == '']
    sw2 = [(w, d) for w, d in zip(sw, dele) if d != '']
    if len(hw) != len(sw2) or not hw:
        return None
    differ = 0
    for h, (w, d) in zip(hw, sw2):
        if h.casefold() == d.casefold():
            if w != d:
                differ += 1
        elif loose and h.casefold() == w.casefold():
            pass
        else:
            return None
    if differ == 0:
        return None
    return {'words_differing': differ, 'emptied_words': len(emptied)}

def dropped_chars(head, spelling):
    """The same shape read char by char, as a byte-dropping decoder makes it: the spelling with EVERY
    non-ASCII char deleted (letters, marks, NBSP, typographic punctuation), whitespace collapsed, equals
    the head (case-folded); at least one word that keeps an ASCII letter lost a letter (a partial drop,
    not an omitted word); and no deleted run of 2+ non-ASCII letters stood as a whole word (an omitted
    Greek/Cyrillic word: `JOHNSON & JOHNSON HELLAS ΑΕΒΕ` vs `Johnson & Johnson Hellas`)."""
    stripped = ''.join(c for c in spelling if ord(c) < 128)
    if ' '.join(stripped.split()).casefold() != ' '.join(head.split()).casefold():
        return None
    partial = False
    for w in spelling.split():
        lost = [c for c in w if ord(c) > 127 and unicodedata.category(c)[0] in 'LM']
        kept = [c for c in w if ord(c) < 128 and c.isalpha()]
        if lost and kept:
            partial = True
        if len(lost) >= 2 and not kept:
            return None
    return {'partial': True} if partial else None

# a port of crosswalk::mojibake (rule v12 / 2a.3) and the control arm of head_spelling_unfit
CONT = set(range(0x80, 0xC0)) | {0x152, 0x153, 0x160, 0x161, 0x178, 0x17D, 0x17E, 0x192, 0x2C6, 0x2DC, 0x20AC, 0x2122} \
    | set(range(0x2013, 0x203B))
def mojibake_rs(name):
    ch = list(name)
    pair = any((a in 'ÃÂÄÅ' and ord(b_) in CONT) or (a == 'È' and b_ in '™›') for a, b_ in zip(ch, ch[1:]))
    wf = any(c in 'ÃÂ' and i > 0 and ch[i - 1].islower() and (i + 1 == len(ch) or ch[i + 1] == ' ')
             for i, c in enumerate(ch))
    return pair or wf
def control_rs(name):
    return any(unicodedata.category(c) == 'Cc' or c == '�' for c in name)

def markers(name):
    """The task's marker list: Ã/Â/Ä followed by a continuation-like char, €, ™, U+FFFD."""
    m = []
    ch = list(name)
    for a, b_ in zip(ch, ch[1:]):
        if a in 'ÃÂÄ' and ord(b_) in CONT:
            m.append(a + b_)
    if '€' in name: m.append('€')
    if '™' in name: m.append('™')
    if '�' in name: m.append('U+FFFD')
    return sorted(set(m))

def crude_key(s):
    s = unicodedata.normalize('NFKD', s)
    s = ''.join(c for c in s if unicodedata.category(c) != 'Mn').casefold()
    return ' '.join(re.findall(r'\w+', s))

# ---- the chain of holds ---------------------------------------------------------------------------
def head_current_next(r):
    hn = day(r['head_newest'])
    return r['rule'] == 'R1' and hn is not None and hn >= as_day - 1095
def top_stale_next(r):
    tn = day(r['top_newest'])
    return r['rule'] == 'R1' and r['clusters'] > 1 + (r['head'] > 0) and (tn is None or tn < as_day - 365)

def read(r, sec):
    vr = '%s:%s' % (r['verdict'], r['reason'])
    reason, rule, new = r['reason'], r['rule'], r['new_name']
    out = {}
    # what blocks it today
    if sec == 'planned':
        if rule == 'R2':
            out['blocked_by'] = 'exclude_rules R2 (wet filter; the plan changes it)'
        elif r.get('recent_released') == 'unfit':
            out['blocked_by'] = 'exclude_released unfit (wet filter)'
        else:
            out['blocked_by'] = 'nothing: a planned R1 change, ships on the next wet run'
    else:
        out['blocked_by'] = reason
    # if this hold were released, what re-holds next (today's order)
    nxt = []
    if sec == 'held' and rule == 'R1' and reason not in ('head_current', 'top_stale'):
        if reason in ('recent', 'residue'):
            nxt.append('bind guards + serve holds (not asked yet)')
        if head_current_next(r):
            nxt.append('head_current')
        elif top_stale_next(r):
            nxt.append('top_stale')
    if sec == 'held' and rule == 'R2':
        nxt.append('exclude_rules R2 (wet filter)')
    out['then'] = nxt
    # (a) R0d: an R0-style defective-head rule — the top cluster's fit spelling (the plan's new_name, R1/R2
    # need a unique top), asked only of the bind guards; no recency, residue, head_current, top_stale or
    # serve hold; R2's initialism hold not asked; not in exclude_rules.
    if new is None:
        a = 'no: no fit spelling (no_fit holds R0 too)'
    elif sec == 'held' and reason in BIND:
        a = 'no: the bind guard %s is asked of R0 too' % reason
    elif sec == 'held' and reason in ('recent', 'residue'):
        fam = r['family_old'] and r['family_new'] and r['family_old'] != r['family_new'] and r['head'] >= 2
        a = 'yes, unless a bind guard holds (not yet asked on a %s hold%s)' % (
            reason, '; family_old %s != family_new %s at H=%d: family likely holds' % (r['family_old'], r['family_new'], r['head']) if fam else '')
    elif sec == 'planned' and rule == 'R1' and r.get('recent_released') != 'unfit':
        a = 'ships today already (planned R1)'
    else:
        a = 'yes (bind guards already passed)'
    out['a_R0d'] = a
    # (b) the unfit test taught the shape: the junk exemption is asked only at recent/residue/head_current;
    # it releases `unfit`, and every wet run passes exclude_released unfit.
    if sec == 'planned':
        b_ = 'ships today already (planned R1)' if (rule == 'R1' and r.get('recent_released') != 'unfit') else \
            'no: %s' % out['blocked_by']
    elif reason in JUNK_ASKED and rule == 'R1':
        b_ = 'no: released `unfit`, then dropped by exclude_released unfit on every wet run'
        if reason in ('recent', 'residue'):
            b_ += ' (and still asked of the bind guards + serve holds first)'
    elif rule == 'R2':
        b_ = 'no: R2 (exclude_rules R2), and %s is not asked of the junk exemption' % reason
    else:
        b_ = 'no: %s never asks the junk exemption, the hold stands' % reason
    out['b_unfit'] = b_
    # (b') a narrow release the filters admit (a new release reason no exclude_released names)
    if sec == 'held' and reason in JUNK_ASKED and rule == 'R1' and new is not None:
        bp = 'yes' if reason == 'head_current' else 'yes, unless a bind guard or serve hold holds (not yet asked)'
    elif sec == 'planned' and rule == 'R1':
        bp = 'ships today already' if r.get('recent_released') != 'unfit' else 'yes (its release would be the new reason)'
    else:
        bp = 'no: ' + out['b_unfit'].split('no: ', 1)[-1]
    out['b2_admitted_release'] = bp
    return vr, out

# My read of each hit (the full class is <= 60, so this listing is the read; no salted draw).
READ = {
    'dropped': ('R', 'the head is the top spelling with its accented letters dropped; the change restores them'),
    22399787: ('R', 'restores `à`; the cluster prints `Pointe-à\\xa0-Pitre`, so the served display reads `Pointe-à -Pitre` (a space where the NBSP was): cosmetic, not wrong'),
    22399788: ('R', 'as 22399787: the same two spellings and counts on a neighbouring org'),
    22504462: ('R', 'head = Atline\'s platform name `Marchés sécurisés` (garbled) + the company; the company\'s own spellings top (`Atline Services`); the cp1252 repair of the head is no listed spelling'),
    22678565: ('N', 'R2 at T=4 of V=10 over 5 clusters; the new name is the same unit (3° Reparto Genio A.M. Bari Palese) with `ROMA` added — not wrong, not clearly better than a repaired head'),
}
rows = []
loose_only = []
for sec in ('held', 'planned'):
    for r in b[sec]:
        head = r['old_name'] or ''
        da = [(s, n, m, 'literal') for s, n in r['spellings'] if s != head for m in [dropped_accent(head, s)] if m]
        if not da:
            da = [(s, n, {'words_differing': None, 'emptied_words': None}, 'char_level') for s, n in r['spellings']
                  if s != head and dropped_chars(head, s)]
        mk = markers(head)
        if not da and not mk:
            lo = [(s, n) for s, n in r['spellings'] if s != head and dropped_accent(head, s, loose=True)]
            if lo:
                loose_only.append({'org_id': r['org_id'], 'verdict_reason': '%s:%s' % (r['verdict'], r['reason']),
                                   'old_name': head, 'spelling': lo[0][0]})
            continue
        vr, chain = read(r, sec)
        if not da:
            chain['a_R0d'] = 'only if R0d also fires on a marker head: ' + chain['a_R0d']
        best = max(da, key=lambda x: x[1]) if da else None
        match = best[0] if best else None
        grade = READ.get(r['org_id']) or (READ['dropped'] if da else ('?', 'unread'))
        rows.append({
            'org_id': r['org_id'], 'section': sec, 'verdict_reason': vr, 'rule': r['rule'], 'country': r['country'],
            'identifier': r['identifier'], 'old_name': head,
            'shape': ('dropped_accent' if da else '') + ('+' if da and mk else '') + ('mojibake_marker' if mk else ''),
            'test': best[3] if best else 'marker',
            'matched_spelling': match, 'matched_count': best[1] if best else None,
            'matched_words_differing': best[2]['words_differing'] if best else None,
            'markers': mk, 'unfit_today': ('mojibake' if mojibake_rs(head) else 'control' if control_rs(head) else None),
            'new_name': r['new_name'],
            'new_is_match': (match is not None and r['new_name'] is not None and crude_key(match) == crude_key(r['new_name'])),
            'V': r['voting'], 'T': r['top'], 'H': r['head'], 'clusters': r['clusters'],
            'head_newest': r['head_newest'], 'top_newest': r['top_newest'], 'rival_newest': r['rival_newest'],
            'head_current_lifts_after': (datetime.date.fromisoformat(r['head_newest']) + datetime.timedelta(days=1095)).isoformat()
                if r['head_newest'] and r['rule'] == 'R1' else None,
            'recent_released': r['recent_released'], 'release_withheld': r['release_withheld'],
            'spellings': r['spellings'], **chain, 'read': grade[0], 'read_note': grade[1],
        })

# keep-class probe: the plan's uniform keep samples and the canaries (keeps are not listed)
keep_probe = {}
for k in ('keep:S1', 'keep:bar', 'keep:D4', 'keep:E1'):
    hit = []
    for r in b['samples'].get(k, []):
        head = r['old_name'] or ''
        if any(dropped_accent(head, s) for s, n in r['spellings'] if s != head) or markers(head):
            hit.append(r['org_id'])
    keep_probe[k] = {'sampled': len(b['samples'].get(k, [])), 'hits': hit, 'class_orgs': b['by_keep'][k.split(':')[1]]['orgs']}
canary_hits = [r['org_id'] for r in b['canaries'] if (any(dropped_accent(r['old_name'] or '', s) for s, n in r['spellings'] if s != r['old_name']) or markers(r['old_name'] or ''))]

by_vr = collections.Counter(x['verdict_reason'] for x in rows)
by_shape = collections.Counter(x['shape'] + ':' + x['test'] for x in rows)
by_vr_shape = collections.Counter((x['verdict_reason'], x['shape']) for x in rows)
def yes(s): return s.startswith('yes') or s.startswith('ships')
summary = {
    'plan': {'job': 2653, 'epoch': b['epoch'], 'rev': b['rev'], 'as_of_date': b['as_of_date'],
             'held': len(b['held']), 'planned': len(b['planned'])},
    'hits': len(rows),
    'by_verdict_reason': dict(by_vr.most_common()),
    'by_shape': dict(by_shape),
    'by_verdict_reason_shape': {'%s %s' % k: v for k, v in sorted(by_vr_shape.items())},
    'unfit_today_on_hits': dict(collections.Counter(str(x['unfit_today']) for x in rows)),
    'new_is_match': dict(collections.Counter(str(x['new_is_match']) for x in rows if x['matched_spelling'])),
    'a_R0d': dict(collections.Counter(x['a_R0d'].split(' (')[0].split(';')[0] for x in rows)),
    'b_unfit': dict(collections.Counter(x['b_unfit'].split(':')[0] if not x['b_unfit'].startswith('ships') else 'ships today already' for x in rows)),
    'b2_admitted_release': dict(collections.Counter(x['b2_admitted_release'].split(',')[0].split(':')[0] for x in rows)),
    'read': dict(collections.Counter(x['read'] for x in rows)),
    'keep_probe': keep_probe, 'canary_hits': canary_hits,
    'loose_only_partial_drops': loose_only,
}
sample = None
if len(rows) > 60:
    key = lambda x: hashlib.sha256((SALT + str(x['org_id'])).encode()).hexdigest()
    sample = [x['org_id'] for x in sorted(rows, key=key)[:30]]
summary['salted_30'] = sample
summary['salted_30_note'] = ('not drawn: %d hits <= 60; the full listing is the read' % len(rows)) if sample is None else 'salt 573-u1-, sha256 ascending'
summary['lower_bound'] = ('keep entries (S1 %d, bar %d, D4 %d orgs) are not listed in the plan; its uniform keep samples read 0/30 '
    'for S1, bar and D4 and the 59 canaries 0; each entry lists only its top 5 spellings. So %d is a lower bound.'
    % (b['by_keep']['S1']['orgs'], b['by_keep']['bar']['orgs'], b['by_keep']['D4']['orgs'], len(rows)))
summary['near_misses'] = {
    'greek_word_omitted_excluded': [11221362, 22751177],
    'legit_A_tilde_or_circumflex_not_markers': [5770, 11109, 22783706],
    'question_mark_replacement': 0,
}
summary['recommendation'] = (
    'Unit 2: decide (a) R0d, narrow. Trigger: the head is the char-level dropped-accent form of a spelling S of another '
    'cluster (S with every non-ASCII char deleted, whitespace collapsed, equals the head case-folded; a word that keeps an '
    'ASCII letter lost one; no whole word of 2+ non-ASCII letters deleted). Elect S\'s cluster only when it is the unique '
    'top and passes R1\'s bars (T*20 >= V*11, T >= 3, T >= 2H); serve its display spelling; ask only the bind guards, never '
    'the window, residue, serve, head_current, top_stale or initialism holds; not in exclude_rules. No marker arm '
    '(unfit(head, head) would reopen 552). Ships 15/15 now incl. 22435268; (b) ships 0; an admitted release ships 14, not '
    'the exhibit; waiting ships 14 unread 2027-01-21..2027-02-18 and never the exhibit. Bar 0 wrong: this read 15 R on the '
    'dropped-accent arm (16 R / 1 N / 0 W over all 17).')
json.dump({'summary': summary, 'hits': rows}, open(OUT, 'w'), ensure_ascii=False, indent=1)
print(json.dumps(summary, ensure_ascii=False, indent=1))
