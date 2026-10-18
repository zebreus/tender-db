# FROZEN: the unit-7 reader exactly as decided (git 53db2c7, decision-u7.md). g3_port.py imports it
# to reproduce the FINAL row of measurement-u7.json. The amended reader (unit 8) is lot_designators.py.
"""Issue 568 unit 7: the lot designators an award block's own title names (the G3/G4 reader).

A designator is what a lot / part / position phrase names: `обособени позиции 5, 6, 15, 16` names
5, 6, 15 and 16; `Pakiet 8 poz. 6` names ONE compound designator ('8', '6') whose head is 8.
Returned as a list of tuples of normalised numbers. This is the reference the Rust reader
(`lot_designators` beside `normalise_lot_number` in crates/ingest/src/project.rs) must match
case for case on SELFTEST below (`python3 -I lot_designators.py` checks it).

Grammar (case-insensitive after NFC; a keyword is a whole word, with no letter on either side):
  clause   := KEYWORD MARK? LIST [POSITION-KEYWORD MARK? NUMBER]   (compound only after ONE number)
            | NUMBER '.'? POSTFIX-KEYWORD                            (Hungarian `IV. rész`, `1. daļa`)
  MARK     := nr | no | n | num | nos | nrs | číslo | č  (then optional . ° º :) | № | # | n° | nº
  LIST     := NUMBER (SEP NUMBER)*
  SEP      := , ; / + & 、 | a conjunction word (i и oraz a et and und e y en és ir ja och og și si ou)
            | a tight dash (-–—) or a dash with spaces on BOTH sides | a range word (do to à bis až au tot t/m al)
  NUMBER   := digits ['.' digits] [one letter not followed by a letter]  -> leading zeros dropped, letter lower-cased
            | an UPPER-CASE Roman numeral not followed by a letter      -> its value (a Roman token only continues a Roman list)
  A range: exactly two numbers joined by a dash, or any two joined by a range word, with a < b <= a + 300, expands
  to a..b; a dash chain of three or more numbers is a list.
"""
import re
import unicodedata

LOT_WORDS = [
    # lot
    r'lot(?:s|ul|uri(?:le)?|e|es|to|ti)?', r'los(?:e|nr)?', r'teillos', r'fachlos',
    # part
    r'part(?:s|ie|ies|e|es)?', r'partida', r'část(?:i|í)?', r'cast(?:i)?', r'časť', r'casti', r'częś[cć]i?',
    r'cz[eę]s[cćś]i?', r'dal(?:is|ys|ies|ių)', r'delaftal(?:e|er)', r'delkontrakt(?:et)?', r'delområde',
    r'osa(?:t)?', r'τμήμα(?:τα|τος)?', r'sklop(?:a|i|ov)?', r'grup(?:a|e|y)', r'skupin(?:a|y)', r'perce(?:el|len)',
    # position / package / task
    r'pozycj[aeiy]', r'poz', r'pozice', r'позици[яи]', r'обособен[аи]\s+позици[яи]', r'pakiet(?:y|u|ów|ach|ami)?',
    r'zadani[ea]', r'zadań', r'zad', r'položk[ay]',
]
POSTFIX_WORDS = [r'rész(?:ek|e)?', r'daļa', r'dalis', r'osa', r'část', r'część']
POSITION_WORDS = r'(?:pozycj[aeiy]|poz|pozice|položk[ay]|item|position)'

LETTER_BEFORE = r'(?<![^\W\d_])'
LETTER_AFTER = r'(?![^\W\d_])'
MARK = r'(?:\s*(?:nr|no|n|num|nos|nrs|číslo|č)\s*[.°º:]?|\s*[№#]|\s*n[°º])?\s*[.:]?\s*'
ROMAN = r'(?:M{0,3}(?:CM|CD|D?C{0,3})(?:XC|XL|L?X{0,3})(?:IX|IV|V?I{0,3}))'
NUM = r'(?:\d+(?:\.\d+)?(?:[a-zA-Z]' + LETTER_AFTER + r')?|' + ROMAN + LETTER_AFTER + r')'
DASH = r'(?:[-–—]|\s+[-–—]\s+)'
LISTSEP = (r'(?:\s*[,;/+&、]\s*|\s+(?:i|и|oraz|a|et|and|und|e|y|en|és|ir|ja|och|og|și|si|ou)\s+|' + DASH + ')')
RANGEWORD = r'(?:\s+(?:do|to|à|bis|až|au|tot|t/m|al)\s+)'

KW_RE = re.compile(LETTER_BEFORE + r'(?:' + '|'.join(LOT_WORDS) + r')' + LETTER_AFTER + MARK, re.IGNORECASE)
POS_RE = re.compile(r'[\s,.]?' + LETTER_BEFORE + POSITION_WORDS + LETTER_AFTER + MARK, re.IGNORECASE)
POSTFIX_RE = re.compile(LETTER_BEFORE + r'(' + NUM + r')\.?\s+(?:' + '|'.join(POSTFIX_WORDS) + r')' + LETTER_AFTER,
                        re.IGNORECASE)
FIRST_RE = re.compile(r'(' + NUM + r')')
NEXT_RE = re.compile(r'(' + LISTSEP + r'|' + RANGEWORD + r')(' + NUM + r')')
ROMAN_RE = re.compile('^' + ROMAN + '$')


def _roman(s):
    vals = {'I': 1, 'V': 5, 'X': 10, 'L': 50, 'C': 100, 'D': 500, 'M': 1000}
    total = 0
    for i, ch in enumerate(s):
        v = vals[ch]
        total += -v if i + 1 < len(s) and vals[s[i + 1]] > v else v
    return total


def norm_number(tok):
    tok = tok.strip()
    if tok and tok.isupper() and ROMAN_RE.match(tok):
        return str(_roman(tok))
    m = re.match(r'^0*(\d+)((?:\.\d+)?)([a-zA-Z]?)$', tok)
    if m:
        return (m.group(1) or '0') + m.group(2) + m.group(3).lower()
    return None


def _is_roman_tok(tok):
    return not tok[:1].isdigit()


def parse_list(text, pos):
    m = FIRST_RE.match(text, pos)
    if not m:
        return [], pos
    items = [(m.group(1), None)]
    end = m.end()
    while True:
        m2 = NEXT_RE.match(text, end)
        if not m2:
            break
        sep, tok = m2.group(1), m2.group(2)
        if _is_roman_tok(tok) != _is_roman_tok(items[0][0]):
            break
        items.append((tok, sep))
        end = m2.end()
    dash_seps = sum(1 for _, s in items[1:] if s is not None and re.fullmatch(DASH, s))
    nums = []
    for i, (tok, sep) in enumerate(items):
        n = norm_number(tok)
        if n is None:
            continue
        if i > 0 and sep is not None and ((re.fullmatch(DASH, sep) and dash_seps == 1 and len(items) == 2)
                                          or re.fullmatch(RANGEWORD, sep)):
            a = nums[-1] if nums else None
            if a and a.isdigit() and n.isdigit() and int(a) < int(n) <= int(a) + 300:
                nums.extend(str(k) for k in range(int(a) + 1, int(n) + 1))
                continue
        nums.append(n)
    return nums, end


def designators(text):
    """The designators a text names, in order of appearance (duplicates kept)."""
    if not text:
        return []
    t = unicodedata.normalize('NFC', text)
    out = []
    pos = 0
    while True:
        m = KW_RE.search(t, pos)
        if not m:
            break
        nums, end = parse_list(t, m.end())
        if not nums:
            pos = m.end()
            continue
        m2 = POS_RE.match(t, end)
        if len(nums) == 1 and m2 and not re.match(POSITION_WORDS, t[m.start():], re.IGNORECASE):
            sub, end2 = parse_list(t, m2.end())
            if len(sub) == 1:
                out.append((nums[0], sub[0]))
                pos = end2
                continue
        out.extend((n,) for n in nums)
        pos = end
    for m in POSTFIX_RE.finditer(t):
        n = norm_number(m.group(1))
        if n:
            out.append((n,))
    return out


def raw_numbers(text):
    """Every integer token of the text (candidate G3a, refused)."""
    return [str(int(x)) for x in re.findall(r'(?<!\d)(\d{1,6})(?!\d)', text or '')]


# (text, expected designators) -- the Rust reader's unit tests copy these cases.
SELFTEST = [
    ('“Доставка …” - обособени позиции 5, 6, 15, 16', [('5',), ('6',), ('15',), ('16',)]),    # 8327340
    ('обособена позиция 3, 8, 17', [('3',), ('8',), ('17',)]),
    ('обособена позиция 24', [('24',)]),
    ('с 4 обособени позиции: Обособена позиция № 1 „Работни станции', [('1',)]),            # 5476961: 4 is a count
    ('lots 2 et 3', [('2',), ('3',)]),
    ('Pakiety nr 1-4', [('1',), ('2',), ('3',), ('4',)]),
    ('Lots 1 à 4', [('1',), ('2',), ('3',), ('4',)]),
    ('Lose 1, 4', [('1',), ('4',)]),
    ('partie 3, 8, 17', [('3',), ('8',), ('17',)]),
    ('Pakiet I, II, III, VI', [('1',), ('2',), ('3',), ('6',)]),                              # 5020611
    ('Pakiet IV', [('4',)]),
    ("Fourniture d'anti-infectieux: lots no1, 2, 5", [('1',), ('2',), ('5',)]),               # 4514627
    ('Lots n° 11132-11193-11194 et 11280.', [('11132',), ('11193',), ('11194',), ('11280',)]),  # a dash chain is a list
    ('Zadanie nr 10,12,13,14,16', [('10',), ('12',), ('13',), ('14',), ('16',)]),
    ('Lote 4: Bata TNT visitante y Lote 11: Gorro', [('4',), ('11',)]),
    ('Perceel 1 + Perceel 4 : Arbeidsongevallen', [('1',), ('4',)]),
    ('Pakiet 8 poz. 6', [('8', '6')]),                                                         # compound, head 8
    ('Pakiet 42 poz.3', [('42', '3')]),                                                        # 4945774
    ('Pakiet 1A', [('1a',)]),
    ('Lot 02.1 carrelage - groupes 1+2+3', [('2.1',)]),                                        # 5071935; groupe is no keyword
    ('Lot 06 désamiantage - multi attributaire - groupe 1', [('6',)]),
    ('Zadanie 2 – Komputer przenośny', [('2',)]),                                              # 8364864
    ('Személykocsi fővizsga IV. rész', [('4',)]),                                              # 5106209 (postfix)
    ('Część 2- 80 miejsc.', [('2',)]),                                                         # 4667265: 80 seats
    ('Część 1 50 - miejsc.', [('1',)]),
    ('Programa A1 — Residencia mixta, de 3 a 17 años', []),                                    # 5693088
    ('Wykonywanie usług w leśnictwie Neple w latach 2019-2020', []),                          # 6328307
    ('Werksinstand von Systemen, M113 Vergabepaket 2', []),                                    # 4846721
    ('Utrzymanie zimowe dróg na obszarze II', []),                                             # 8022925
    ('Opracowanie programu dla zadania »Budowa i przebudowa', []),                             # 5652687
    ('2 Solo-Niederflubusse ca. 15 m', []),                                                    # 4968281 (residual)
    ('3a Leki do uzytku zewnętrznego', []),                                                    # 5364153 (residual)
    ('Del 1 Färdtjänst.', []),                                                                 # bare del is no keyword
    ("Procedura aperta per l'affidamento del 31.7.2008", []),
    ('Lot n° 11093.', [('11093',)]),
]

if __name__ == '__main__':
    bad = [(t, designators(t), want) for t, want in SELFTEST if designators(t) != want]
    for t, got, want in bad:
        print('FAIL', repr(t), got, '!=', want)
    print(f'{len(SELFTEST) - len(bad)}/{len(SELFTEST)} ok')
    raise SystemExit(1 if bad else 0)
