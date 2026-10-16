"""Python port of crates/ingest/src/crosswalk.rs head_vote_key (and its helpers), char for char."""
import unicodedata

def rs_is_alphanumeric(c):
    cat = unicodedata.category(c)
    # Rust: Alphabetic property || Numeric (Nd|Nl|No). Python approximation: L*, Nl, Other_Alphabetic (Mn/Mc of
    # some scripts) is not reachable without the property table; flag those for a cross-check.
    return cat[0] == 'L' or cat in ('Nd', 'Nl', 'No') or c.isalpha()

def rs_is_whitespace(c):
    return c in '\t\n\x0b\x0c\r \x85\xa0                　'

def split_whitespace(s):
    out, cur = [], []
    for c in s:
        if rs_is_whitespace(c):
            if cur: out.append(''.join(cur)); cur = []
        else:
            cur.append(c)
    if cur: out.append(''.join(cur))
    return out

GREEK = {'ά': 'α', 'έ': 'ε', 'ή': 'η', 'ί': 'ι', 'ϊ': 'ι', 'ΐ': 'ι', 'ό': 'ο', 'ύ': 'υ', 'ϋ': 'υ', 'ΰ': 'υ', 'ώ': 'ω', 'ς': 'σ'}

def match_norm(name):
    out, gap = [], False
    for c in name:
        if rs_is_alphanumeric(c):
            if gap and out: out.append(' ')
            gap = False
            out.extend(GREEK.get(x, x) for x in c.lower())
        else:
            gap = True
    return ''.join(out)

def family_token(t):
    return {'gmbh': 'gmbh', 'mbh': 'gmbh', 'ag': 'ag', 'sarl': 'sarl', 'eurl': 'sarl', 'sas': 'sas', 'sasu': 'sas',
            'oy': 'aktiebolag', 'oyj': 'aktiebolag', 'ab': 'aktiebolag', 'abp': 'aktiebolag', 'uab': 'aktiebolag',
            'aps': 'aps', 'bv': 'bv', 'nv': 'nv', 'kft': 'kft', 'zrt': 'zrt', 'nyrt': 'zrt', 'spa': 'spa', 'srl': 'srl',
            'sia': 'sia', 'sro': 'sro', 'spzoo': 'spzoo'}.get(t)

FAMILY_SEQUENCES = [(['spol', 's', 'r', 'o'], 'sro'), (['ges', 'm', 'b', 'h'], 'gmbh'), (['sp', 'z', 'o', 'o'], 'spzoo'),
                    (['s', 'r', 'o'], 'sro')]

def n3_key(name):
    toks = [t for t in match_norm(name).split(' ') if t]
    out, i = [], 0
    while i < len(toks):
        for seq, fam in FAMILY_SEQUENCES:
            if toks[i:i + len(seq)] == seq:
                out.append('§' + fam); i += len(seq); break
        else:
            f = family_token(toks[i])
            out.append('§' + f if f else toks[i]); i += 1
    return ' '.join(out)

def altid_trim(name):
    low = name.encode('utf-8')
    def find(needle):
        nb = needle.encode()
        lb = bytes(b + 32 if 65 <= b <= 90 else b for b in low)
        p = lb.find(nb)
        return None if p < 0 else p
    cut = len(low)
    for m in [' t/a ', '(t/a ', ' trading as ', '(trading as ']:
        p = find(m)
        if p is not None: cut = min(cut, p)
    kept = low[:cut].decode('utf-8').rstrip(rs_ws_chars())
    if kept.endswith(')') and '(' in kept:
        op = kept.rfind('(')
        before = kept[:op].rstrip(rs_ws_chars()).rstrip('.,')
        lower = before.lower() if before.isascii() else ''.join(c.lower() if c.isascii() else c for c in before)
        if any(lower.endswith(f) for f in [' ltd', ' limited', ' plc', ' llp']):
            kept = before
    return kept

def rs_ws_chars():
    return '\t\n\x0b\x0c\r \x85\xa0                　'

def altid_name_key(name):
    out, initials = [], ''
    spelled = altid_trim(name).replace('&', ' and ')
    for t in [t for t in n3_key(spelled).split(' ') if t and t != 'the']:
        if t == 'and':
            if initials: out.append(initials); initials = ''
            continue
        if len(t) == 1 and t.isalpha():
            initials += t; continue
        if initials: out.append(initials); initials = ''
        out.append({'ltd': '§ltd', 'limited': '§ltd', 'plc': '§plc', 'llp': '§llp', 'lp': '§lp', 'cic': '§cic',
                    'co': 'company'}.get(t, t))
    if initials: out.append(initials)
    return ' '.join(out)

LOW = "abbbcdddddeeeeefghhhhhiikkkllllmmmnnnnoooopprrrrsssssttttuuuuuvvwwwwwxxyzzz"
HIGH = "aaaaaaaaaaaaeeeeeeeeiioooooooooooouuuuuuuyyyy"
TABLE = {}
for chars, base in [('àáâãäåāăąǎǟǡǻȁȃȧ', 'a'), ('æǣǽ', 'ae'), ('ƀ', 'b'), ('çćĉċčƈ', 'c'), ('ďđðƌ', 'd'),
                    ('èéêëēĕėęěȅȇȩ', 'e'), ('ƒ', 'f'), ('ĝğġģǥǧǵ', 'g'), ('ĥħȟ', 'h'), ('ìíîïĩīĭįıǐȉȋ', 'i'), ('ĳ', 'ij'),
                    ('ĵǰ', 'j'), ('ķĸǩ', 'k'), ('ĺļľŀł', 'l'), ('ñńņňŉŋǹ', 'n'), ('òóôõöøōŏőơǒǫǭǿȍȏȫȭȯȱ', 'o'),
                    ('œ', 'oe'), ('ŕŗřȑȓ', 'r'), ('śŝşšſș', 's'), ('ß', 'ss'), ('ţťŧț', 't'), ('þ', 'th'),
                    ('ùúûüũūŭůűųưǔǖǘǚǜȕȗ', 'u'), ('ŵ', 'w'), ('ýÿŷƴȳ', 'y'), ('źżžƶ', 'z')]:
    for ch in chars: TABLE[ch] = base

def fold_latin_letter(c):
    if c in TABLE: return TABLE[c]
    cp = ord(c)
    if 0x1E00 <= cp <= 0x1E95: return LOW[(cp - 0x1E00) // 2]
    extra = {0x1E96: 'h', 0x1E97: 't', 0x1E98: 'w', 0x1E99: 'y', 0x1E9A: 'a', 0x1E9B: 's'}
    if cp in extra: return extra[cp]
    if 0x1EA0 <= cp <= 0x1EF9: return HIGH[(cp - 0x1EA0) // 2]
    return None

def fold_diacritics(name):
    out = []
    for ch in name:
        for c in ch.lower():
            cp = ord(c)
            if 0x300 <= cp <= 0x36F or 0x1AB0 <= cp <= 0x1AFF or 0x1DC0 <= cp <= 0x1DFF or 0x20D0 <= cp <= 0x20FF or 0xFE20 <= cp <= 0xFE2F:
                continue
            b = fold_latin_letter(c)
            out.append(b if b is not None else c)
    return ''.join(out)

def head_vote_key(name):
    collapsed = ' '.join(split_whitespace(name))
    alt = altid_name_key(fold_diacritics(collapsed))
    kept = [t for t in alt.split(' ') if t and not t.startswith('§') and family_token(t) is None]
    return ' '.join(kept) if kept else alt
