"""Issue 568 unit 7: the multi-lot / contradicted-number guards (G3-G6) replayed on the cached parsed values.

Replays the deployed 568 rule (`../cross_notice_port.py`: steps 1-9 + G1/G2) on every award notice of
every Tender in three sets, then the unit-7 guards, and joins the judged classes:
  s150 -- the salted 150 of the 156,123 corrected (readback/u7_sample.json; skeptic-final per block:
          skeptic_u*/u*_skeptic_final*.json);
  m77  -- unit 1's 77 measured Tenders (measurement.json + extra strata; readback/measured77.json);
  f30  -- the read-back's fresh 30 (readback/fresh30.json; readback_result.json: 8327340's 7 multi-lot
          blocks wrong, every other block right).
Input: u7-cache.jsonl.gz (every tender's versions and every chain notice's /v1/notices/{id}/content).

Usage: python3 -I g3_port.py [out.json]   (default: measurement-u8.json next to this file)

Unit 8 (decision-u7.md "Amendment (unit 8, 2026-10-18)"): the decided rule (FINAL) is reproduced unchanged with
the frozen unit-7 reader (lot_designators_u7.py; features() / apply_final(), which the OOS drivers
../u8-oos/o1_skeptic_run.py and o2_skeptic_run.py import), and every old candidate row is kept byte for byte.
Beside it, FINAL_AMENDED (features_u8() / apply_amended()): the amended reader (lot_designators.py), titles
read per language, G3 on distinct heads, G4 always block-level and notice-level only on 2+ contradicting blocks
with title-confirmed blocks kept, and the clarified text sides (block titles decoded as served; the lot side the
lot's `title` / `description` facts as the fold carries them at the award version; the Tender side its `title`
facts). measurement-u7.json is not rewritten; the default output is measurement-u8.json.

Unit 8 round 2 (owner decision after the port + verify stages): FINAL_AMENDED now includes R1 and R2, which are
apply_amended()'s defaults:
  R1  G4 agreement compares HEADS: a block does not contradict when some language's Dn has p as a head, or shares a
      head with the designators of the matched lot's own title / description;
  R2  only a NON-COMPOUND designator title-confirms: inside a fired notice a block keeps only if, for every language
      with a non-empty Dn, every designator is non-compound and the heads are exactly {p}.
The round-1 rule stays measured as FINAL_AMENDED_R0 (agree='designator', confirm='heads'; tag 8r0), with R0+R1 and
R0+R2 alone as attribution rows (tags 8r0h, 8r0s). title_key moved to lot_designators.py unchanged (TITLE_KEY_SELFTEST).
"""
import collections, gzip, json, math, os, re, sys, unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
sys.path.insert(0, HERE)
import cross_notice_port as P  # noqa: E402
import lot_designators_u7 as L7  # noqa: E402  (frozen: the decided FINAL)
import lot_designators as L  # noqa: E402  (amended: FINAL_AMENDED)

nl = P.normalise_lot_number
TITLE_FIELDS = ('TED-CONTRACT_TITLE', 'TED-TITLE', 'TED-TITLE_CONTRACT')
LOT_TEXT_FIELDS = ('TED-LOT_TITLE', 'TED-TITLE', 'TED-LOT_DESCRIPTION', 'TED-SHORT_DESCR', 'TED-SHORT_CONTRACT_DESCRIPTION')
LOT_TITLE_FIELDS = ('TED-LOT_TITLE', 'TED-TITLE')
PROC_TITLE_FIELDS = ('TED-TITLE_CONTRACT', 'TED-TITLE', 'TED-CONTRACT_TITLE')

# Unit 8: the fold's text table (crates/ingest/src/project.rs `TEXTS`, the `title` / `description` rows).
FOLD_TEXTS = {
    'BT-21': 'title', 'BT-24': 'description',
    'TED-TITLE': 'title', 'TED-TITLE_CONTRACT': 'title', 'TED-CONTRACT_TITLE': 'title',
    'TED-TITLE_QUALIFICATION_SYSTEM': 'title', 'TED-TITLE_DESIGN_CONTACT_NOTICE': 'title',
    'TED-TITLE_RESULT_DESIGN_CONTEST': 'title', 'TED-TITLE_NOTICE_BUYER_PROFILE': 'title',
    'TED-LOT_TITLE': 'title', 'TED-LOT_DESCRIPTION': 'description', 'TED-SHORT_DESCR': 'description',
    'TED-SHORT_CONTRACT_DESCRIPTION': 'description', 'TED-SHORT_DESCRIPTION_CONTRACT': 'description',
    'TED-NEW_VALUE.TEXT': 'description',
    'TXT-TI': 'title', 'TXT-TX': 'description', 'TXT-AB': 'description',
    'SDK01-ProcurementProject-Name': 'title', 'SDK01-ProcurementProjectLot-ProcurementProject-Name': 'title',
    'SDK01-ProcurementProject-Description': 'description',
    'SDK01-ProcurementProjectLot-ProcurementProject-Description': 'description',
}
OJ_HEADING_FIELD = 'TED-TI_DOC'


# ---------------------------------------------------------------- served text (unit 8)
def decode_publisher_escapes(text):
    """crates/ingest/src/project/escapes.rs `decode_publisher_escapes`, rule for rule (issue 538): what a served
    `title` / `description` holds. Unchanged when the text has no decodable reference."""
    if not text or '&' not in text:
        return text
    chars = [(c, False) for c in text]
    decoded = False
    for _ in range(3):  # MAX_PASSES
        nxt = _escape_pass(chars)
        if nxt is None:
            break
        chars, decoded = nxt, True
    if not decoded:
        return text
    out, i = [], 0
    while i < len(chars):
        c, dec = chars[i]
        if dec and c == '<':
            ahead = ''.join(ch.lower() if ch.isascii() else ch for ch, _ in chars[i + 1:i + 6])
            tail = next((t for t in ('br>', 'br/>', 'br />') if ahead.startswith(t)), None)
            if tail:
                out.append('\n')
                i += 1 + len(tail)
                continue
        out.append('\u2022' if dec and '\ue000' <= c <= '\uf8ff' else c)
        i += 1
    return ''.join(out)


def _escape_pass(chars):
    out, changed, i = [], False, 0
    while i < len(chars):
        if chars[i][0] == '&':
            ref = _escape_reference(chars[i + 1:i + 18])
            if ref:
                out.append((ref[0], True))
                i += 1 + ref[1]
                changed = True
                continue
        out.append(chars[i])
        i += 1
    return out if changed else None


def _escape_reference(rest):
    semi = next((k for k, (c, _) in enumerate(rest[:17]) if c == ';'), None)  # MAX_BODY 16, then the `;`
    if semi is None:
        return None
    body = ''.join(c for c, _ in rest[:semi])
    if body.startswith('#'):
        num = body[1:]
        digits, radix = (num[1:], 16) if num[:1] in ('x', 'X') else (num, 10)
        allowed = '0123456789abcdefABCDEF' if radix == 16 else '0123456789'
        if not digits or any(ch not in allowed for ch in digits):
            return None
        n = int(digits, radix)
        if n > 0xFFFFFFFF or n > 0x10FFFF or 0xD800 <= n <= 0xDFFF:
            return None
        c = chr(n)
        if not (c in '\t\n\r' or not (unicodedata.category(c) == 'Cc' or c in '\ufffe\uffff')):
            return None
    else:
        c = {'amp': '&', 'lt': '<', 'gt': '>', 'quot': '"', 'apos': "'"}.get(
            ''.join(ch.lower() if ch.isascii() else ch for ch in body))
        if c is None:
            return None
    return c, semi + 1


def notice_text_facts(secs):
    """`state_of`'s text facts of one notice: {scope: {field: {(lang, served value)}}}, scope None = the Tender.
    Every FOLD_TEXTS value, scoped to its nearest enclosing lot section; a `title` under a result section is
    filed nowhere (issue 343); the OJ heading is the Tender title only when the notice mapped none."""
    out = collections.defaultdict(lambda: collections.defaultdict(set))
    for s in secs.values():
        lot = P.enclosing(secs, s['section_id'], P.LOT_KINDS)
        in_result = P.enclosing(secs, s['section_id'], P.RESULT_KINDS) is not None
        for v in s.get('values', []):
            field = FOLD_TEXTS.get(v['field_id'])
            if field is None or v.get('type') != 'text' or (field == 'title' and in_result):
                continue
            out[lot][field].add((v.get('lang') or '', decode_publisher_escapes(v.get('value') or '')))
    if not out[None].get('title'):
        ref = next((str(v.get('value')).strip() for sid in sorted(secs) for v in sorted(
            secs[sid].get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
            if v['field_id'] == 'TED-NO_DOC_OJS' and v.get('value') is not None), None)
        for sid in sorted(secs):
            if P.enclosing(secs, sid, P.LOT_KINDS) is not None:
                continue
            hit = next(((v.get('lang') or '', v.get('value') or '') for v in sorted(
                secs[sid].get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
                if v['field_id'] == OJ_HEADING_FIELD and v.get('type') == 'text'
                and (v.get('value') or '').strip() != ref), None)
            if hit:
                out[None]['title'] = {(hit[0], decode_publisher_escapes(hit[1]))}
                break
    return out


# ---------------------------------------------------------------- inputs
def load_bundle():
    tenders, notices, sets = {}, {}, collections.defaultdict(list)
    with gzip.open(os.path.join(HERE, 'u7-cache.jsonl.gz'), 'rt', encoding='utf-8') as fh:
        for line in fh:
            r = json.loads(line)
            if r['kind'] == 'tender':
                tenders[r['id']] = r['data']
                sets[r['set']].append(r['id'])
            else:
                notices[r['id']] = r['data']
    return tenders, notices, sets


def load_labels():
    lab = {}
    files = ['skeptic_u1/u1_skeptic_final.short.json', 'skeptic_u2/u2_skeptic_final.json',
             'skeptic_u3/u3_skeptic_final.min.json', 'skeptic_u4/u4_skeptic_final.min.json',
             'skeptic_u5/u5_skeptic_final.min.json', 'skeptic_u6/u6_skeptic_final.short.json']
    for f in files:
        for b in json.load(open(os.path.join(HERE, f), encoding='utf-8'))['blocks']:
            lab[('s150', b['tender_id'], b['can_notice_id'], b['result_key'])] = b['cls']
    m = json.load(open(os.path.join(ROOT, 'measurement.json'), encoding='utf-8'))
    for t in m['tenders']:
        for b in t['blocks']:
            lab[('m77', t['tender_id'], t['can_notice_id'], b['result_key'])] = b['cls']
    for b in m['extra_strata_judge_read']['blocks']:
        lab[('m77', b['tender_id'], b['notice_id'], b['result_key'])] = b.get('cls_with_G1') or b['cls_without_guards']
    return lab


F30_MULTI = {(17821481, 'RES-1'), (17821481, 'RES-2'), (17821481, 'RES-5'), (17821481, 'RES-6'),
             (17821481, 'RES-7'), (17830774, 'RES-1'), (17830774, 'RES-2')}
ONE_LOT_PROCEDURE = {4514627}  # the Tender's only lot is the whole 182-lot procedure (skeptic note)


def label(lab, b):
    if b['set'] == 'f30':
        return 'multi' if (b['can_notice_id'], b['result_key']) in F30_MULTI else 'right'
    c = lab.get((b['set'], b['tender_id'], b['can_notice_id'], b['result_key']))
    if c is None:
        return 'unjudged'
    if c in ('single_right', 'unique_right', 'unique_unverifiable'):
        return 'right'
    if c == 'multi_lot_contract':
        return 'multi_one_lot_tender' if b['tender_id'] in ONE_LOT_PROCEDURE else 'multi'
    return c


# ---------------------------------------------------------------- replay
def texts_of(section, fields):
    return [v.get('value') or '' for v in sorted(section.get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
            if v.get('type') == 'text' and v['field_id'] in fields]


def titles_lang(section):
    """Unit 8: the block's own title values with their language ('' when the value has none)."""
    return [(v.get('lang') or '', v.get('value') or '') for v in
            sorted(section.get('values', []), key=lambda v: (v['field_id'], v.get('ordinal', 0)))
            if v.get('type') == 'text' and v['field_id'] in TITLE_FIELDS]


def first_amount(section, fields):
    for f in fields:
        for v in sorted(section.get('values', []), key=lambda v: v.get('ordinal', 0)):
            if v['field_id'] == f and v.get('type') == 'amount':
                return (v.get('cents'), v.get('currency'))
    return None


def replay(tid, tender, notices):
    chain = [v['caused_by_notice_id'] for v in sorted(tender['versions'], key=lambda v: v['seq'])]
    carried, lotmeta, renumbered, tender_titles = {}, {}, False, []
    lotfacts, tender_facts = {}, {}  # unit 8: the fold's lot / Tender text facts, superseded per field
    blocks = []
    for n in chain:
        secs = P.sections_of(notices[n])
        refs = P.award_refs(secs)
        own, ren = P.carry(carried, secs)
        renumbered |= ren
        for s in own:
            lotmeta[s['section_id']] = {'title': texts_of(s, LOT_TITLE_FIELDS), 'texts': texts_of(s, LOT_TEXT_FIELDS),
                                        'value': first_amount(s, ('TED-VALUE_COST', 'TED-VAL_ESTIMATED_TOTAL', 'TED-VAL_OBJECT'))}
        tf = notice_text_facts(secs)
        for field, vals in tf[None].items():
            if vals:
                tender_facts[field] = set(vals)
        for s in own:
            cur = lotfacts.setdefault(s['section_id'], {})
            for field, vals in tf[s['section_id']].items():
                if vals:
                    cur[field] = set(vals)
        proc = secs.get('PROCEDURE')
        if proc is not None:
            t = texts_of(proc, PROC_TITLE_FIELDS)
            if t:  # per-field supersede: the latest notice that publishes a Tender title replaces it
                tender_titles = t
        if not refs:
            continue
        matched = P.match_notice(carried, renumbered, own, refs)
        inherited = {k: {'kind': kd, 'printed': num, **lotmeta.get(k, {'title': [], 'texts': [], 'value': None}),
                         'facts': {f: sorted(v) for f, v in lotfacts.get(k, {}).items()}}
                     for k, (kd, num) in carried.items()}
        tender_title_facts = sorted(t for _, t in tender_facts.get('title', set()))
        for res in sorted(refs, key=P.res_order):
            s = secs[res]
            lot, why = matched[res]
            field = next((f for f in ('TED-LOT_NO', 'TED-LOT_NUMBER') if any(x == f for x, _ in refs[res])), None)
            blocks.append({
                'tender_id': tid, 'can_notice_id': n, 'result_key': res,
                'printed': P.award_block_lot_number(refs[res]), 'lot_568': lot, 'why_568': why,
                'number_values': [v for f, v in refs[res] if f == field],
                'titles': texts_of(s, TITLE_FIELDS),
                'award': first_amount(s, ('TED-VAL_TOTAL', 'TED-VALUE_COST')),
                'inherited': inherited, 'tender_titles': list(tender_titles),
                # unit 8: (lang, value) per title value on the RES section, raw and decoded as served
                'titles_lang_raw': titles_lang(s),
                'titles_lang': [(lg, decode_publisher_escapes(t)) for lg, t in titles_lang(s)],
                'tender_title_facts': tender_title_facts,
            })
    return blocks


# ---------------------------------------------------------------- guards
# G5's comparable title. Unit 8 round 2 moved it, byte for byte, to lot_designators.py (beside the reader, with
# TITLE_KEY_SELFTEST pinning "Pakiet 2" -> "pakiet 2": the leading designator is dropped only when a separator
# follows); FINAL and FINAL_AMENDED both use it, so the decided FINAL is unchanged (parity_with_measurement_u7).
title_key = L.title_key


def features(b, reader=None):
    R = reader or L7  # the decided FINAL reads with the frozen unit-7 reader
    lots = {k: v for k, v in b['inherited'].items() if v['kind'] == 'Lot' and v['printed'] and v['printed'].strip()}
    N = {nl(v['printed']) for v in lots.values()} - {None}
    p = nl(b['printed']) if b['printed'] else None
    D = set()
    for t in b['titles']:
        D |= set(R.designators(t))
    Df = {d for d in D if d[0] in N}
    lot = b['inherited'].get(b['lot_568']) if b['lot_568'] else None
    Dlot = set()
    for t in (lot or {}).get('texts', []):
        Dlot |= set(R.designators(t))
    raw = set()
    for t in b['titles']:
        raw |= set(R.raw_numbers(t))
    ratio = None
    if lot and lot.get('value') and b['award'] and lot['value'][0] and b['award'][0] is not None \
            and lot['value'][1] == b['award'][1]:
        ratio = b['award'][0] / lot['value'][0]
    keys = {title_key(t) for t in b['titles']} - {''}
    mine = {title_key(t) for t in (lot or {}).get('title', [])} - {''}
    tender_keys = {title_key(t) for t in b['tender_titles']} - {''}
    others = [{title_key(t) for t in v.get('title', [])} - {''} for k, v in lots.items() if k != b['lot_568']]
    # one language agreeing with the matched lot's own title is agreement (8303947 RES-2: its Maltese
    # variant copies lot 1's, its other 22 languages are lot 2's)
    g5 = not (keys & mine) and any(k not in tender_keys and sum(1 for o in others if k in o) == 1 for k in keys)
    return {
        'N': sorted(N), 'p': p, 'one_lot': len(N) < 2,
        'D': sorted(D), 'Df': sorted(Df),
        'contradicts': bool(Df) and p not in {d[0] for d in Df} and not (Df & Dlot),
        'raw_in_N': sorted(raw & N), 'ratio': ratio,
        'several_numbers': len({nl(v) for v in b['number_values'] if v and v.strip()} - {None}) >= 2,
        'g5': g5,
    }


def apply_final(blocks, reader=None, tag=''):
    """Unit 7's rule over one award notice's blocks (all blocks of one CAN of one Tender). With `reader` and a
    `tag` (unit 8's attribution row) the results go to f<tag> / guards<tag> / lot_u7<tag>."""
    for b in blocks:
        b['f' + tag] = features(b, reader)
    # G4 reads only the blocks the 568 rule matched: a contradiction on a block that found no lot
    # (5218711's CAN-only 5/6/7 for sub-lots 3.B-3.D) says nothing about the numbers that did match.
    g4 = any(b['lot_568'] is not None and not b['f' + tag]['one_lot'] and b['f' + tag]['contradicts'] for b in blocks)
    for b in blocks:
        f = b['f' + tag]
        fired = []
        if b['lot_568'] is not None and not f['one_lot']:
            if g4:
                fired.append('G4')
            if f['several_numbers']:
                fired.append('G6')
            if len(f['Df']) >= 2:
                fired.append('G3')
            if f['g5']:
                fired.append('G5')
        b['guards' + tag] = fired
        b['lot_u7' + tag] = None if fired else b['lot_568']


# ---------------------------------------------------------------- the amended guards (unit 8)
def features_u8(b, sides='fold', agree='head', confirm='simple'):
    """The amended rule's evidence for one block. Titles are read per language (D): Dn[lang] is the union over
    that language's values of the designators whose head is in N. `sides='port'` reads the unit-7 port's text
    sides instead (raw block titles, the latest lot section's texts, the PROCEDURE title), for attribution.
    The defaults are FINAL_AMENDED as adopted in unit 8 round 2 (decision-u7.md "Amendment"): R1 `agree='head'`, a
    Dn designator agrees with the lot's text by its head (a compound counts by its head, as in C); R2
    `confirm='simple'`, only non-compound designators title-confirm. `agree='designator', confirm='heads'` is the
    round-1 rule, FINAL_AMENDED_R0 (whole designators agree; any designator with head p confirms)."""
    lots = {k: v for k, v in b['inherited'].items() if v['kind'] == 'Lot' and v['printed'] and v['printed'].strip()}
    N = {nl(v['printed']) for v in lots.values()} - {None}
    p = nl(b['printed']) if b['printed'] else None
    titles = b['titles_lang'] if sides == 'fold' else b['titles_lang_raw']
    Dn = collections.defaultdict(set)
    for lang, t in titles:
        Dn[lang] |= {d for d in L.designators(t) if d[0] in N}
    Dn = {lang: d for lang, d in Dn.items() if d}
    lot = b['inherited'].get(b['lot_568']) if b['lot_568'] else None
    lot = lot or {'title': [], 'texts': [], 'facts': {}}

    def lot_titles(v):
        return [t for _, t in v['facts'].get('title', [])] if sides == 'fold' else v.get('title', [])
    lot_texts = ([t for f in ('title', 'description') for _, t in lot['facts'].get(f, [])] if sides == 'fold'
                 else lot.get('texts', []))
    Dlot = set()
    for t in lot_texts:
        Dlot |= set(L.designators(t))
    if agree == 'head':
        lot_heads = L.heads(Dlot)
        agrees = any(p in L.heads(d) or (L.heads(d) & lot_heads) for d in Dn.values())
    else:
        agrees = any(p in L.heads(d) or (d & Dlot) for d in Dn.values())
    confirmed = bool(Dn) and all(L.heads(d) == {p} for d in Dn.values())
    if confirm == 'simple':
        confirmed = confirmed and all(len(x) == 1 for d in Dn.values() for x in d)
    keys = {title_key(t) for _, t in titles} - {''}
    mine = {title_key(t) for t in lot_titles(lot)} - {''}
    tender_keys = {title_key(t) for t in (b['tender_title_facts'] if sides == 'fold' else b['tender_titles'])} - {''}
    others = [{title_key(t) for t in lot_titles(v)} - {''} for k, v in lots.items() if k != b['lot_568']]
    g5 = not (keys & mine) and any(k not in tender_keys and sum(1 for o in others if k in o) == 1 for k in keys)
    return {
        'N': sorted(N), 'p': p, 'one_lot': len(N) < 2,
        'Dn': {lang: sorted(d) for lang, d in sorted(Dn.items())},
        # A (+R1): contradicts when some language names a lot in N and no language agrees (p a head, or a head
        # shared with the designators of the matched lot's own title / description; R0: a whole designator shared)
        'contradicts': bool(Dn) and not agrees,
        # A (+R2): title-confirmed when some language names a lot in N and every such language names exactly the
        # heads {p}, with no compound designator (R0: compounds with head p confirm too)
        'title_confirmed': confirmed,
        # C: two or more distinct heads in one language (a compound counts by its head)
        'g3': any(len(L.heads(d)) >= 2 for d in Dn.values()),
        'several_numbers': len({nl(v) for v in b['number_values'] if v and v.strip()} - {None}) >= 2,
        'g5': g5,
    }


def apply_amended(blocks, sides='fold', tag='8', agree='head', confirm='simple'):
    """The amended rule (decision-u7.md "Amendment (unit 8, 2026-10-18)") over one award notice's blocks:
    contradictions over every matched block first; a contradicting block is refused (G4, block level, always);
    with 2+ contradicting blocks every other matched block is refused too (G4n) unless title-confirmed; then
    G6, G3, G5 per block. None on a Tender whose inherited lots print < 2 distinct numbers. Results go to
    f<tag> / guards<tag> / lot_u<tag>. The defaults are FINAL_AMENDED with R1 + R2 (round 2); pass
    agree='designator', confirm='heads' for the round-1 rule FINAL_AMENDED_R0."""
    for b in blocks:
        b['f' + tag] = features_u8(b, sides, agree, confirm)
    active = [b for b in blocks if b['lot_568'] is not None and not b['f' + tag]['one_lot']]
    contradicting = sum(1 for b in active if b['f' + tag]['contradicts'])
    for b in blocks:
        f = b['f' + tag]
        f['notice_contradicting'] = contradicting
        fired = []
        if any(b is a for a in active):
            if f['contradicts']:
                fired.append('G4')
            elif contradicting >= 2 and not f['title_confirmed']:
                fired.append('G4n')
            if f['several_numbers']:
                fired.append('G6')
            if f['g3']:
                fired.append('G3')
            if f['g5']:
                fired.append('G5')
        b['guards' + tag] = fired
        b['lot_u' + tag] = None if fired else b['lot_568']


# ---------------------------------------------------------------- stats
def cp(k, n, a=0.05):
    if n == 0:
        return [None, None]
    def cdf(x, p):
        return sum(math.comb(n, i) * p ** i * (1 - p) ** (n - i) for i in range(0, x + 1))
    def bis(f):
        lo, hi = 0.0, 1.0
        for _ in range(60):
            m = (lo + hi) / 2
            lo, hi = (lo, m) if f(m) else (m, hi)
        return (lo + hi) / 2
    lo = 0.0 if k == 0 else bis(lambda p: 1 - cdf(k - 1, p) >= a / 2)
    hi = 1.0 if k == n else bis(lambda p: cdf(k, p) <= a / 2)
    return [round(lo, 4), round(hi, 4)]


def block_id(b):
    return f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}"


WRONG = ('multi', 'wrong_other')


def rule_stats(blocks, lot_key, guard_key, tenders):
    """Catches, false refusals, Tenders carrying a wrong block before / after, and the residual, for one rule
    over one pool of matched blocks; `tenders` the Tender ids the pool was drawn from."""
    def cnt(lbl):
        pool = [b for b in blocks if b['label'] == lbl]
        k = sum(1 for b in pool if b[guard_key])
        return {'k': k, 'n': len(pool)}
    right = [b for b in blocks if b['label'] == 'right']
    fr = [b for b in right if b[lot_key] is None]
    before = sorted({b['tender_id'] for b in blocks if b['label'] in WRONG})
    after = sorted({b['tender_id'] for b in blocks if b['label'] in WRONG and b[lot_key] is not None})
    n_t = len(tenders)
    return {
        'matched_blocks': len(blocks), 'multi_caught': cnt('multi'), 'other_wrong_caught': cnt('wrong_other'),
        'false_refusals': {'k': len(fr), 'n': len(right), 'ci95': cp(len(fr), len(right)),
                           'ids': [block_id(b) for b in fr]},
        'tenders_with_a_wrong_block_before': {'k': len(before), 'n': n_t, 'ci95': cp(len(before), n_t), 'ids': before},
        'tenders_with_a_wrong_block_after': {'k': len(after), 'n': n_t, 'ci95': cp(len(after), n_t), 'ids': after},
        'residual_wrong_blocks': [f"{block_id(b)} ({b['label']})" for b in blocks
                                  if b['label'] in WRONG and b[lot_key] is not None],
        'blocks_refused': sum(1 for b in blocks if b[guard_key]),
        'refused_by_guard': dict(sorted(collections.Counter(g for b in blocks for g in b[guard_key]).items())),
        'tenders_touched': sorted({b['tender_id'] for b in blocks if b[guard_key]}),
    }


def why_changed(b):
    """Why a block's outcome differs between FINAL (decision-u7) and FINAL_AMENDED, from the guards each fired."""
    f7, f8 = b['f'], b['f8']
    g7, g8 = set(b['guards']), set(b['guards8'])
    out = []
    if 'G4' in g7 and not {'G4', 'G4n'} & g8:
        if f7['contradicts'] and not f8['contradicts']:
            out.append(f"no longer contradicts: unit-7 Df {f7['Df']} -> amended Dn {f8['Dn']}")
        elif f8['notice_contradicting'] < 2:
            out.append(f"G4 notice: the notice has {f8['notice_contradicting']} contradicting block(s), fewer than 2, "
                       f"so only a contradicting block is refused (this one names {f8['Dn'] or 'no lot'})")
        elif f8['title_confirmed']:
            out.append(f"G4 notice fired ({f8['notice_contradicting']} contradicting blocks) but the block is "
                       f"title-confirmed: Dn heads = {{{f8['p']}}} in every language naming a lot ({f8['Dn']})")
    if 'G3' in g7 and 'G3' not in g8:
        out.append(f"G3: unit-7 Df {f7['Df']} -> amended Dn {f8['Dn']} (heads per language)")
    if 'G5' in g7 and 'G5' not in g8:
        out.append('G5: the folded / served text sides make the title agree with its lot, or not another lot\'s')
    if 'G6' in g7 and 'G6' not in g8:
        out.append('G6 lost (unexpected)')
    gained = (g8 - {'G4n'}) - g7 | ({'G4n'} & g8 if 'G4' not in g7 else set())
    for g in sorted(gained):
        detail = {'G4': f"contradicts: amended Dn {f8['Dn']} vs p {f8['p']} (unit-7 Df {f7['Df']})",
                  'G4n': f"G4 notice: {f8['notice_contradicting']} contradicting blocks, not title-confirmed ({f8['Dn']})",
                  'G3': f"G3: amended Dn {f8['Dn']}", 'G5': 'G5 on the folded / served text sides',
                  'G6': 'G6'}[g]
        out.append('gained ' + detail)
    return '; '.join(out) or f'guards {sorted(g7)} -> {sorted(g8)}'


def why_r0_changed(b):
    """Why a block's outcome differs between the round-1 rule (FINAL_AMENDED_R0, tag 8r0) and FINAL_AMENDED (R1+R2)."""
    f0, f8 = b['f8r0'], b['f8']
    g0, g8 = set(b['guards8r0']), set(b['guards8'])
    out = []
    if f0['contradicts'] and not f8['contradicts']:
        out.append(f"R1: Dn {f8['Dn']} shares a head with the matched lot's own text, so it no longer contradicts")
    if f0['notice_contradicting'] != f8['notice_contradicting']:
        out.append(f"R1: the notice's contradicting blocks {f0['notice_contradicting']} -> {f8['notice_contradicting']}")
    if f0['title_confirmed'] and not f8['title_confirmed']:
        out.append(f"R2: Dn {f8['Dn']} holds a compound, so it no longer title-confirms")
    out.append(f"guards {sorted(g0)} -> {sorted(g8)}")
    return '; '.join(out)


def amended_summary(matched, sets, uni, summary7, cp_, scale, pmf):
    """measurement-u8.json's `amended` section."""
    per_set = {}
    for name in ('s150', 'f30', 'm77'):
        pool = [b for b in matched if b['set'] == name]
        per_set[name] = {'FINAL': rule_stats(pool, 'lot_u7', 'guards', sets[name]),
                         'FINAL_AMENDED': rule_stats(pool, 'lot_u8', 'guards8', sets[name]),
                         'FINAL_AMENDED_R0': rule_stats(pool, 'lot_u8r0', 'guards8r0', sets[name]),
                         'R0_plus_R1_only': rule_stats(pool, 'lot_u8r0h', 'guards8r0h', sets[name]),
                         'R0_plus_R2_only': rule_stats(pool, 'lot_u8r0s', 'guards8r0s', sets[name]),
                         'U8a_reader_only': rule_stats(pool, 'lot_u7_r8', 'guards_r8', sets[name]),
                         'U8b_amended_on_port_text_sides': rule_stats(pool, 'lot_u8p', 'guards8p', sets[name])}
    uni_blocks = [b for b in matched if b['set'] in ('s150', 'f30')]
    uni_ids = [t for t, _ in uni]
    after8 = sorted({b['tender_id'] for b in uni_blocks if b['label'] in WRONG and b['lot_u8'] is not None})
    after8r0 = sorted({b['tender_id'] for b in uni_blocks if b['label'] in WRONG and b['lot_u8r0'] is not None})
    before = summary7['tenders_with_a_wrong_block_before']['ids']
    k8, n8 = len(after8), len(uni_ids)
    bar = {}
    for tag, p in (('point', k8 / n8), ('ci95_low', cp_(k8, n8)[0]), ('ci95_high', cp_(k8, n8)[1])):
        bar[tag] = {'p_tender': round(p, 4), 'P(0 of 30)': round(pmf(0, 30, p), 3),
                    'P(<=2 of 30)': round(sum(pmf(k, 30, p) for k in range(3)), 3)}
    changed = []
    for b in matched:
        if b['lot_u7'] != b['lot_u8']:
            changed.append({'block': block_id(b), 'label': b['label'], 'printed': b['printed'], 'lot_568': b['lot_568'],
                            'lot_u7': b['lot_u7'], 'lot_u8': b['lot_u8'], 'guards_u7': b['guards'],
                            'guards_u8': b['guards8'], 'why': why_changed(b),
                            'title': (b['titles'] or [''])[0][:160]})
    changed_r0 = [{'block': block_id(b), 'label': b['label'], 'printed': b['printed'], 'lot_568': b['lot_568'],
                   'lot_u8r0': b['lot_u8r0'], 'lot_u8': b['lot_u8'], 'guards_u8r0': b['guards8r0'],
                   'guards_u8': b['guards8'], 'why': why_r0_changed(b), 'title': (b['titles'] or [''])[0][:160]}
                  for b in matched if b['lot_u8r0'] != b['lot_u8'] or b['guards8r0'] != b['guards8']]
    s150 = [b for b in matched if b['set'] == 's150']
    touched8 = sorted({b['tender_id'] for b in s150 if b['guards8']})
    reverted8 = sorted(t for t in touched8 if all(b['lot_u8'] is None for b in s150 if b['tender_id'] == t))
    # parity: the old candidate rows against measurement-u7.json (the decided numbers must not move)
    parity = None
    p7 = os.path.join(HERE, 'measurement-u7.json')
    if os.path.exists(p7):
        m7 = json.load(open(p7, encoding='utf-8'))
        mine = {f"{x['set']}:{x['tender_id']}/{x['can_notice_id']}/{x['result_key']}": (x['lot_u7'], x['guards'])
                for x in m7['blocks']}
        diffs = [block_id(b) for b in matched if mine.get(block_id(b)) != (b['lot_u7'], b['guards'])]
        parity = {'blocks_compared': len(matched), 'blocks_in_measurement_u7': len(mine),
                  'FINAL_lot_or_guards_differ': diffs}
    return {
        'rule': 'FINAL_AMENDED with R1 + R2 (decision-u7.md "Amendment (unit 8, 2026-10-18)", round 2); '
                'FINAL_AMENDED_R0 = the round-1 rule',
        'per_set': per_set,
        'uniform_tenders_with_a_wrong_block': {
            'FINAL_before_any_guard': {'k': len(before), 'n': n8, 'ci95': cp_(len(before), n8)},
            'FINAL': {k: summary7['tenders_with_a_wrong_block_after'][k] for k in ('k', 'n', 'ci95', 'ids')},
            'FINAL_AMENDED': {'k': k8, 'n': n8, 'ci95': cp_(k8, n8), 'ids': after8},
            'FINAL_AMENDED_R0': {'k': len(after8r0), 'n': n8, 'ci95': cp_(len(after8r0), n8), 'ids': after8r0}},
        'fresh30_bar_ii_probabilities_at_amended_residual': bar,
        'changed_outcomes': changed,
        'changed_vs_FINAL_AMENDED_R0': changed_r0,
        's150_tenders_touched': {'k': len(touched8), 'n': 150, 'ci95': cp_(len(touched8), 150), 'ids': touched8},
        's150_tenders_fully_reverted': {'k': len(reverted8), 'n': 150, 'ci95': cp_(len(reverted8), 150),
                                        'ids': reverted8},
        's150_blocks_refused': sum(1 for b in s150 if b['guards8']),
        'scale_to_156123_corrected': {'tenders_touched': scale(len(touched8), 150),
                                      'tenders_fully_reverted': scale(len(reverted8), 150),
                                      'blocks_refused_point': round(156123 * sum(1 for b in s150 if b['guards8']) / 150),
                                      'right_blocks_refused_point': round(156123 * sum(
                                          1 for b in s150 if b['guards8'] and b['label'] == 'right') / 150)},
        'f30_8327340': {f"{b['can_notice_id']}/{b['result_key']}": [b['lot_568'], b['lot_u8'], b['guards8']]
                        for b in matched if b['tender_id'] == 8327340},
        'parity_with_measurement_u7': parity,
    }



def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, 'measurement-u8.json')
    tenders, notices, sets = load_bundle()
    lab = load_labels()
    allb = []
    for name in ('s150', 'm77', 'f30'):
        for tid in sets[name]:
            blocks = replay(tid, tenders[tid], notices)
            by_notice = collections.defaultdict(list)
            for b in blocks:
                b['set'] = name
                by_notice[b['can_notice_id']].append(b)
            for bl in by_notice.values():
                apply_final(bl)                          # FINAL as decided (frozen unit-7 reader)
                apply_final(bl, reader=L, tag='_r8')     # attribution: the decided rule on the amended reader
                apply_amended(bl)                        # FINAL_AMENDED (R1 + R2, round 2)
                apply_amended(bl, sides='port', tag='8p')  # attribution: FINAL_AMENDED on the unit-7 text sides
                apply_amended(bl, tag='8r0', agree='designator', confirm='heads')  # FINAL_AMENDED_R0 (round 1)
                apply_amended(bl, tag='8r0h', agree='head', confirm='heads')  # attribution: R0 + R1 alone
                apply_amended(bl, tag='8r0s', agree='designator', confirm='simple')  # attribution: R0 + R2 alone
            allb.extend(blocks)
    matched = [b for b in allb if b['lot_568'] is not None]
    for b in matched:
        b['label'] = label(lab, b)

    # candidate guards (block predicates; notice-level ones look at the notice's blocks)
    notice_of = collections.defaultdict(list)
    for b in matched:
        notice_of[(b['set'], b['tender_id'], b['can_notice_id'])].append(b)
    def nk(b):
        return (b['set'], b['tender_id'], b['can_notice_id'])
    def g3a(b): return not b['f']['one_lot'] and len(b['f']['raw_in_N']) >= 2
    def g3c(b): return not b['f']['one_lot'] and len(b['f']['Df']) >= 2
    def val(k): return lambda b: b['f']['ratio'] is not None and b['f']['ratio'] >= k
    def g4blk(b): return not b['f']['one_lot'] and b['f']['contradicts']
    def g4n(b): return any(g4blk(x) for x in notice_of[nk(b)])
    def g5(b): return not b['f']['one_lot'] and b['f']['g5']
    def g5n(b): return any(g5(x) for x in notice_of[nk(b)])
    def g6(b): return not b['f']['one_lot'] and b['f']['several_numbers']
    cands = {
        'G3a: title names >=2 distinct integers that inherited lots print': g3a,
        'G3c: title names >=2 distinct lot designators (lot/part/position phrase, any language) that inherited lots print': g3c,
    }
    for k in (1.5, 1.8, 2.0, 2.5, 3.0):
        cands[f'G3d: award >= {k}x the matched lot\'s own CN value (same currency)'] = val(k)
        cands[f'G3b: G3a or award >= {k}x lot value'] = (lambda k: lambda b: g3a(b) or val(k)(b))(k)
        cands[f'G3c or award >= {k}x lot value'] = (lambda k: lambda b: g3c(b) or val(k)(b))(k)
    cands['G4 (block): a title designator an inherited lot prints, not the printed number nor the matched lot\'s own'] = g4blk
    cands['G4 (notice): any block of the notice contradicts -> none matched'] = g4n
    cands['G5 (block): title is exactly one OTHER inherited lot\'s title (not the matched lot\'s, not the Tender\'s)'] = g5
    cands['G5 (notice)'] = g5n
    cands['G6 (block): the block prints 2+ distinct lot numbers'] = g6
    cands['FINAL = G3c | G4 notice | G5 block | G6'] = lambda b: bool(b['guards'])
    # ---- unit 8: the amended guards (decision-u7.md "Amendment (unit 8, 2026-10-18)"), appended after the old rows
    cands['U8a: the decided FINAL on the amended reader (attribution: the reader change alone)'] = \
        lambda b: bool(b['guards_r8'])
    cands['U8 G3: in one language the heads of Dn hold >= 2 distinct values (amended reader)'] = \
        lambda b: not b['f8']['one_lot'] and b['f8']['g3']
    cands['U8 G4 block (R1): some language names a lot in N, no language names p as a head or shares a head with '
          'l\'s own title/description designators'] = lambda b: not b['f8']['one_lot'] and b['f8']['contradicts']
    cands['U8 G4 notice (R2): contradicting blocks always; with 2+ contradicting blocks in the notice every other '
          'matched block too, except title-confirmed (non-compound, heads = {p})'] = \
        lambda b: bool({'G4', 'G4n'} & set(b['guards8']))
    cands['U8 G5 block (rule unchanged; text sides as folded)'] = lambda b: not b['f8']['one_lot'] and b['f8']['g5']
    cands['U8 G6 (unchanged)'] = lambda b: not b['f8']['one_lot'] and b['f8']['several_numbers']
    cands['FINAL_AMENDED (R1+R2) = U8 G3 | U8 G4 block | U8 G4 notice (2+, title-confirmed kept) | G5 block | G6'] = \
        lambda b: bool(b['guards8'])
    cands['U8b: FINAL_AMENDED on the unit-7 port\'s text sides (attribution: the text-field spec alone)'] = \
        lambda b: bool(b['guards8p'])
    cands['FINAL_AMENDED_R0 (round 1, superseded): G4 agreement by whole designator, any head-p designator confirms'] = \
        lambda b: bool(b['guards8r0'])
    cands['R0 + R1 alone (attribution): G4 agreement by a shared head'] = lambda b: bool(b['guards8r0h'])
    cands['R0 + R2 alone (attribution): only a non-compound designator title-confirms'] = lambda b: bool(b['guards8r0s'])
    labels = ['multi', 'wrong_other', 'multi_one_lot_tender', 'right']
    table = {}
    for name, g in cands.items():
        row = {}
        for labl in labels:
            for s in ('s150', 'm77', 'f30', 'all'):
                pool = [b for b in matched if b['label'] == labl and (s == 'all' or b['set'] == s)]
                if not pool:
                    continue
                k = sum(1 for b in pool if g(b))
                row[f'{labl}/{s}'] = {'k': k, 'n': len(pool), 'ci95': cp(k, len(pool))}
        hits = collections.defaultdict(list)
        for b in matched:
            if g(b) and b['label'] != 'right' or (g(b) and b['label'] == 'right'):
                hits[b['label']].append(f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}")
        row['hits'] = {k: v for k, v in hits.items()}
        table[name] = row

    # tender level over the uniform draws (s150 + f30)
    def tender_wrong(tid, setname, use_final):
        bl = [b for b in matched if b['tender_id'] == tid and b['set'] == setname]
        return any(b['label'] in ('multi', 'wrong_other') and (not use_final or b['lot_u7'] is not None) for b in bl)
    uni = [(t, s) for s in ('s150', 'f30') for t in sets[s]]
    before = [t for t, s in uni if tender_wrong(t, s, False)]
    after = [t for t, s in uni if tender_wrong(t, s, True)]
    refused_right = [b for b in matched if b['label'] == 'right' and b['lot_u7'] is None]
    s150_touched = sorted({b['tender_id'] for b in matched if b['set'] == 's150' and b['guards']})
    s150_all_refused = sorted(t for t in s150_touched
                              if all(b['lot_u7'] is None for b in matched if b['tender_id'] == t and b['set'] == 's150'))
    residual = [b for b in matched if b['label'] in ('multi', 'wrong_other') and b['lot_u7'] is not None]

    per_block = [{k: b[k] for k in ('set', 'tender_id', 'can_notice_id', 'result_key', 'printed', 'lot_568', 'lot_u7',
                                    'guards', 'label')}
                 | {'Df': b['f']['Df'], 'contradicts': b['f']['contradicts'], 'g5': b['f']['g5'],
                    'several_numbers': b['f']['several_numbers'],
                    'ratio': None if b['f']['ratio'] is None else round(b['f']['ratio'], 4),
                    'title': (b['titles'] or [''])[0][:160],
                    'lot_u8': b['lot_u8'], 'guards8': b['guards8'], 'Dn8': b['f8']['Dn'],
                    'contradicts8': b['f8']['contradicts'], 'title_confirmed8': b['f8']['title_confirmed'],
                    'notice_contradicting8': b['f8']['notice_contradicting'],
                    'lot_u8r0': b['lot_u8r0'], 'guards8r0': b['guards8r0']}
                 for b in matched]
    summary = {
        'matched_blocks': collections.Counter(f"{b['set']}/{b['label']}" for b in matched),
        'replay_matches_served': 'see extract check in decision-u7.md (s150 1173/1173 incl. nulls, m77 692/692, f30 212/212)',
        'uniform_tenders': len(uni),
        'tenders_with_a_wrong_block_before': {'k': len(before), 'n': len(uni), 'ci95': cp(len(before), len(uni)), 'ids': before},
        'tenders_with_a_wrong_block_after': {'k': len(after), 'n': len(uni), 'ci95': cp(len(after), len(uni)), 'ids': after},
        'right_blocks_refused': [f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']}" for b in refused_right],
        's150_tenders_touched': {'k': len(s150_touched), 'n': 150, 'ci95': cp(len(s150_touched), 150), 'ids': s150_touched},
        's150_tenders_fully_reverted': {'k': len(s150_all_refused), 'n': 150, 'ci95': cp(len(s150_all_refused), 150),
                                        'ids': s150_all_refused},
        's150_blocks_refused': sum(1 for b in matched if b['set'] == 's150' and b['guards']),
        'residual_wrong_blocks': [f"{b['set']}:{b['tender_id']}/{b['can_notice_id']}/{b['result_key']} ({b['label']})"
                                  for b in residual],
        'm77_blocks_refused': sum(1 for b in matched if b['set'] == 'm77' and b['guards']),
        'f30_8327340': {f"{b['can_notice_id']}/{b['result_key']}": [b['lot_568'], b['lot_u7'], b['guards']]
                        for b in matched if b['tender_id'] == 8327340},
    }
    pop = 156123
    def scale(k, n):
        lo, hi = cp(k, n)
        return {'point': round(pop * k / n), 'ci95': [round(pop * lo), round(pop * hi)]}
    def pmf(k, n, p):
        return math.comb(n, k) * p ** k * (1 - p) ** (n - k)
    res_k, res_n = len(after), len(uni)
    bar = {}
    for tag, p in (('point', res_k / res_n), ('ci95_low', cp(res_k, res_n)[0]), ('ci95_high', cp(res_k, res_n)[1])):
        bar[tag] = {'p_tender': round(p, 4), 'P(0 of 30)': round(pmf(0, 30, p), 3),
                    'P(<=2 of 30)': round(sum(pmf(k, 30, p) for k in range(3)), 3)}
    meta = {
        'issue': 568, 'unit': 7,
        'what': 'Refusal guards for the 568 cross-notice match: candidate catches and false refusals over the salted 150 '
                '(skeptic-final), the 77 measured (unit 1) and the read-back fresh 30 (8327340 included); the decided '
                'rule (FINAL) replayed on the cached parsed values (u7-cache.jsonl.gz) by g3_port.py.',
        'labels': {'right': 'single_right / unique_right / unique_unverifiable',
                   'multi': 'one award block for several lots, printing one of them (multi_lot_contract)',
                   'wrong_other': 'on a lot that is not its lot for another reason',
                   'multi_one_lot_tender': '4514627: a 17-lot contract on the Tender whose only lot is the whole '
                                           '182-lot procedure; same scope as the Tender, not counted wrong'},
        'decided_rule': 'G3 | G4(notice) | G5 | G6, none on a Tender whose inherited lots print < 2 distinct numbers; '
                        'see decision-u7.md',
        'scale_to_156123_corrected': {
            'tenders_touched': scale(len(s150_touched), 150),
            'tenders_fully_reverted': scale(len(s150_all_refused), 150),
            'blocks_refused_point': round(pop * summary['s150_blocks_refused'] / 150),
        },
        'fresh30_bar_probabilities_at_measured_residual': bar,
        'unit1_frame_vs_corrected_draws': {
            'm77_cans_with_a_wrong_or_multi_block': '0 of 70 CANs (656 matched blocks)',
            'uniform_draws': f'{len(before)} of {len(uni)} Tenders',
            'P(0 of 70 | rate of the uniform draws)': round((1 - len(before) / len(uni)) ** 70, 4)},
    }
    amended = amended_summary(matched, sets, uni, summary, cp, scale, pmf)
    meta['unit'] = '7 (old rows, FINAL as decided) + 8 (FINAL_AMENDED)'
    meta['what'] += (' Unit 8: the amended rule (FINAL_AMENDED) beside it, on the same replay; the old rows are '
                     'byte-identical to measurement-u7.json (check: `amended.parity_with_measurement_u7`).')
    meta['amended_rule'] = ('G3 heads per language | G4 block (per language; R1: agreement by p as a head or a head '
                            'shared with l\'s own text) | G4 notice only on 2+ contradicting blocks, title-confirmed '
                            'blocks kept (R2: non-compound designators only, heads = {p}) | G5 block | G6; none on a '
                            'Tender whose inherited lots print < 2 distinct numbers; amended reader lot_designators.py '
                            '(incl. the quantity-word rule); text sides as folded / served. FINAL_AMENDED_R0 (lot_u8r0 / '
                            'guards8r0) is the round-1 rule without R1 / R2. See decision-u7.md "Amendment (unit 8, '
                            '2026-10-18)".')
    with open(out_path, 'w', encoding='utf-8') as fh:
        fh.write('{\n"meta": ' + json.dumps(meta, ensure_ascii=False, indent=1))
        fh.write(',\n"summary": ' + json.dumps(summary, ensure_ascii=False, indent=1, default=list))
        fh.write(',\n"amended": ' + json.dumps(amended, ensure_ascii=False, indent=1, default=list))
        fh.write(',\n"candidates": ' + json.dumps(table, ensure_ascii=False, indent=1, default=list))
        fh.write(',\n"blocks": [\n' + ',\n'.join(json.dumps(b, ensure_ascii=False, default=list) for b in per_block) + '\n]\n}\n')
    for name, row in table.items():
        print(f"{name[:100]:100s} multi {row['multi/all']['k']}/{row['multi/all']['n']}  wrong {row['wrong_other/all']['k']}/{row['wrong_other/all']['n']}"
              f"  FR s150 {row['right/s150']['k']} m77 {row['right/m77']['k']} f30 {row['right/f30']['k']} all {row['right/all']['k']}/{row['right/all']['n']} {row['right/all']['ci95']}")
    print(json.dumps({k: v for k, v in summary.items() if k != 'f30_8327340'}, default=list)[:3000])
    print(summary['f30_8327340'])
    print(json.dumps({k: amended[k] for k in ('per_set', 'uniform_tenders_with_a_wrong_block', 'parity_with_measurement_u7')},
                     default=list, ensure_ascii=False, indent=1))
    for c in amended['changed_outcomes']:
        print('CHANGED', c['block'], c['label'], c['lot_u7'], '->', c['lot_u8'], '|', c['why'])
    for c in amended['changed_vs_FINAL_AMENDED_R0']:
        print('VS_R0', c['block'], c['label'], c['lot_u8r0'], '->', c['lot_u8'], '|', c['why'])


if __name__ == '__main__':
    main()
