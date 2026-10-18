"""Issue 568 unit 7, amended in unit 8: the lot designators an award block's own title names (the G3/G4 reader).

A designator is what a lot / part / position phrase names: `обособени позиции 5, 6, 15, 16` names
5, 6, 15 and 16; `Pakiet 8 poz. 6` names ONE compound designator ('8', '6') whose head is 8, and
`Pakiet 8 poz. 6 i 7` names ONE compound ('8', '6', '7'), head 8. Returned as a list of tuples of
normalised numbers, the head first. This is the reference the Rust reader (`lot_designators` beside
`normalise_lot_number` in crates/ingest/src/project.rs) must match case for case on SELFTEST below
(`python3 -I lot_designators.py` checks it). The unit-7 reader as decided is frozen in
lot_designators_u7.py; the changes are listed under "Unit 8" below.

Input (unit 8 review): the text as parsed, NOT normalised. The reader expects NFC (every one of the 348,446 cached
values is) and reads a decomposed title as its decomposed letters, as the Rust reader does: NFD "Część 2" -> [].
A digit is ASCII 0-9 only; any other decimal digit (fullwidth, Arabic-Indic, ...) is a letter, as in Rust:
"Lot ３" -> []. Both pinned in SELFTEST. The one remaining class where Python's `\\w` / `\\s` and Rust's
`char::is_alphanumeric` / `char::is_whitespace` disagree is recorded in decision-u7.md "Designators, as amended".

Grammar (a keyword is a whole word, with no letter on either side):
  clause   := KEYWORD MARK? LIST [COMPOUND-WORD MARK? LIST]          (compound only after ONE number)
            | NUMBER '.'? POSTFIX-KEYWORD                            (Hungarian `IV. rész`, `1. daļa`)
  MARK     := nr | no | n | num | nos | nrs | číslo | č  (then optional . ° º :) | № | # | n° | nº
  LIST     := NUMBER (SEP NUMBER)*
  SEP      := , ; / + & 、 | a conjunction word (i и oraz a et and und e y en és ir ja och og și si ou)
            | a TIGHT dash (-–—, no space on either side) | a range word (do to à bis až au tot t/m al)
  NUMBER   := ASCII digits ['.' digits] [one letter not followed by a letter]  -> leading zeros dropped, letter lower-cased
            | an UPPER-CASE Roman numeral not followed by a letter      -> its value (a Roman token only continues a Roman list)
  A range: exactly two numbers joined by a tight dash, or any two joined by a range word, with a < b <= a + 300,
  expands to a..b; a dash chain of three or more numbers is a list.
Case: keywords, marks, compound words and postfix words match case-insensitively, except the German `Los` family,
which matches only as `Los Lose Losnr` or `LOS LOSE LOSNR`. List separators, conjunctions, range words and Roman
numerals are case-sensitive (lower-case words, upper-case numerals), as in unit 7.

Unit 8 (decision-u7.md "Amendment (unit 8, 2026-10-18)", from the decision critic's probes):
  (a) only a TIGHT dash joins numbers; a dash with a space on either side ends the list
      ("Część 1 – 50 miejsc" -> [1], "Lot 1 - 5 ordinateurs" -> [1], "Część 2- 80 miejsc." -> [2]);
  (b) `los / lose / losnr` are lot keywords only capitalised or all upper-case: lower-case `los` is the Spanish
      article ("de los 2 vehículos" -> []);
  (c) a compound's second half (after `poz. / pozycja / pozice / položka / item / position`, and now `zad. / zadanie`)
      takes a list or a range and stays ONE designator under the first half's head ("Pakiet 8 poz. 6 i 7",
      "Pakiet 16 zad.1"); `pakiet N pozycji|pozycje|pozycja` with no number after the word is a count, not a
      designator ("(pakiet 8 pozycji)" -> []); a position word (`poz pozycja pozycje pozycji pozycjy pozice
      položka položky позиция позиции`) is a lot keyword only when no lot / part / pakiet / zadanie clause
      precedes it in the same text (`обособена позиция`, the Bulgarian lot keyword, is unaffected);
  (d) from the amendment's item E: a list's last number followed by a quantity or duration word (COUNT_WORDS:
      `szt osób lat dni ans mois years Stück ks броя` ...) is a count, not a lot: it is dropped
      ("Lot 2, 3 ans" -> [2], "Pakiet 5, 2 szt." -> [5], "grupy 20 osób" -> []). Measured free: over the
      8,792 distinct cached texts (in-sample + OOS) it moves only three notice-level descriptions, no block or
      lot title. Removing a number usually withholds a refusal, but it CAN add one: two blocks "Lot 2, 3 ans"
      printed 3 and "Lot 5, 4 ans" printed 4 turn from G3 refusals into two contradictions, which fire the notice
      level on a third, untitled block (the verifier's synthetic case; rules (a)-(c) can do the same). No such
      case occurs in the data.

Accepted recall cost of (a), pinned in SELFTEST: "Lots 1 - 2 - 3" -> [1] (a spaced dash after a lot word is almost
always an amount in the corpus, "dla części nr 1 – 2 500,00 zł"). `\\s` includes the no-break space: "Lot 1\\u00a0–\\u00a050
places" reads [1] exactly as with plain spaces; the Rust reader must treat U+00A0 the same.

`title_key` (G5's comparable title) lives here too, beside the reader, with its own TITLE_KEY_SELFTEST: the leading
designator is dropped only when a separator (`. : ) - – —`) follows it, so "Pakiet 2" keeps the key "pakiet 2".
"""
import re
import unicodedata

# Position words: lot keywords only when no other keyword clause precedes them in the same text (unit 8 (c)).
POSITION_KW = r'(?:pozycj[aeiy]|poz|pozice|позици[яи]|položk[ay])'
PAKIET_KW = r'pakiet(?:y|u|ów|ach|ami)?'
LOT_WORDS = [
    # lot (the German `Los` family is case-sensitive: lower-case `los` is the Spanish article, unit 8 (b))
    r'lot(?:s|ul|uri(?:le)?|e|es|to|ti)?', r'(?-i:Los(?:e|nr)?|LOS(?:E|NR)?)', r'teillos', r'fachlos',
    # part
    r'part(?:s|ie|ies|e|es)?', r'partida', r'část(?:i|í)?', r'cast(?:i)?', r'časť', r'casti', r'częś[cć]i?',
    r'cz[eę]s[cćś]i?', r'dal(?:is|ys|ies|ių)', r'delaftal(?:e|er)', r'delkontrakt(?:et)?', r'delområde',
    r'osa(?:t)?', r'τμήμα(?:τα|τος)?', r'sklop(?:a|i|ov)?', r'grup(?:a|e|y)', r'skupin(?:a|y)', r'perce(?:el|len)',
    # package / task
    r'обособен[аи]\s+позици[яи]', PAKIET_KW, r'zadani[ea]', r'zadań', r'zad',
    # position (see POSITION_KW)
    POSITION_KW,
]
POSTFIX_WORDS = [r'rész(?:ek|e)?', r'daļa', r'dalis', r'osa', r'část', r'część']
# The second half of a compound (unit 8 (c): `zad. / zadanie` added).
COMPOUND_WORDS = r'(?:pozycj[aeiy]|poz|pozice|položk[ay]|item|position|zad|zadani[ea])'

# A letter: a word character that is neither an ASCII digit nor `_` (unit 8 review: `[0-9]`, not `\d`, so a
# non-ASCII decimal digit is a letter here exactly as in the Rust reader).
LETTER_BEFORE = r'(?<![^\W0-9_])'
LETTER_AFTER = r'(?![^\W0-9_])'
MARK = r'(?:\s*(?:nr|no|n|num|nos|nrs|číslo|č)\s*[.°º:]?|\s*[№#]|\s*n[°º])?\s*[.:]?\s*'
ROMAN = r'(?:M{0,3}(?:CM|CD|D?C{0,3})(?:XC|XL|L?X{0,3})(?:IX|IV|V?I{0,3}))'
NUM = r'(?:[0-9]+(?:\.[0-9]+)?(?:[a-zA-Z]' + LETTER_AFTER + r')?|' + ROMAN + LETTER_AFTER + r')'
DASH = r'(?:[-–—])'  # unit 8 (a): tight only; a spaced dash ends the list
LISTSEP = (r'(?:\s*[,;/+&、]\s*|\s+(?:i|и|oraz|a|et|and|und|e|y|en|és|ir|ja|och|og|și|si|ou)\s+|' + DASH + ')')
RANGEWORD = r'(?:\s+(?:do|to|à|bis|až|au|tot|t/m|al)\s+)'
# Unit 8 (E): a list number directly followed by one of these words is a quantity or a duration, not a lot
# ("Lot 2, 3 ans", "Pakiet 5, 2 szt.", "grupy 20 osób"): it is dropped and ends the list.
COUNT_WORDS = [
    r'szt', r'sztuk[i]?', r'osób', r'osoby', r'osob', r'miejsc[a]?', r'lat[a]?', r'miesięcy', r'miesiące', r'dni', r'kpl',  # pl
    r'ans', r'années', r'mois', r'jours',                                                                        # fr
    r'years', r'months', r'days', r'units', r'pcs', r'persons', r'people',                                        # en
    r'años', r'anos', r'meses', r'días', r'unidades', r'anni', r'mesi', r'giorni',                                # es pt it
    r'jahre', r'monate', r'stück', r'stk', r'personen',                                                           # de
    r'let', r'roky', r'měsíců', r'ks', r'kusů',                                                                   # cs sk
    r'години', r'месеца', r'броя', r'бр',                                                                         # bg
]

KW_RE = re.compile(LETTER_BEFORE + r'(?P<kw>' + '|'.join(LOT_WORDS) + r')' + LETTER_AFTER + MARK, re.IGNORECASE)
POS_RE = re.compile(r'[\s,.]?' + LETTER_BEFORE + COMPOUND_WORDS + LETTER_AFTER + MARK, re.IGNORECASE)
COUNT_RE = re.compile(r'\s+pozycj[iea]' + LETTER_AFTER, re.IGNORECASE)
QUANTITY_RE = re.compile(r'\s+(?:' + '|'.join(COUNT_WORDS) + r')' + LETTER_AFTER, re.IGNORECASE)
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
    m = re.match(r'^0*([0-9]+)((?:\.[0-9]+)?)([a-zA-Z]?)$', tok)
    if m:
        return (m.group(1) or '0') + m.group(2) + m.group(3).lower()
    return None


def _is_roman_tok(tok):
    return not tok[:1].isdigit()


def parse_list(text, pos):
    m = FIRST_RE.match(text, pos)
    if not m:
        return [], pos
    items = [(m.group(1), None, m.end())]
    end = m.end()
    while True:
        m2 = NEXT_RE.match(text, end)
        if not m2:
            break
        sep, tok = m2.group(1), m2.group(2)
        if _is_roman_tok(tok) != _is_roman_tok(items[0][0]):
            break
        items.append((tok, sep, m2.end()))
        end = m2.end()
    # Unit 8 (E): the last number followed by a quantity word is a count, not a lot (only the last can be: the
    # list continues only through a separator).
    if QUANTITY_RE.match(text, end):
        items.pop()
        if not items:
            return [], pos
        end = items[-1][2]
    items = [(tok, sep) for tok, sep, _ in items]
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
    t = text  # as parsed, not normalised (unit 8 review: the Rust reader does not normalise either)
    out = []
    pos = 0
    clause_seen = False  # a lot / part / pakiet / zadanie clause has named something in this text
    while True:
        m = KW_RE.search(t, pos)
        if not m:
            break
        is_position = re.fullmatch(POSITION_KW, m.group('kw'), re.IGNORECASE) is not None
        if is_position and clause_seen:
            pos = m.end()
            continue
        nums, end = parse_list(t, m.end())
        if not nums:
            pos = m.end()
            continue
        if len(nums) == 1 and not is_position:
            m2 = POS_RE.match(t, end)
            if m2:
                sub, end2 = parse_list(t, m2.end())
                if sub:
                    out.append((nums[0], *sub))
                    clause_seen = True
                    pos = end2
                    continue
            if re.fullmatch(PAKIET_KW, m.group('kw'), re.IGNORECASE):
                m3 = COUNT_RE.match(t, end)
                if m3:  # `pakiet 8 pozycji`: a package of 8 items, not a designator
                    pos = m3.end()
                    continue
        out.extend((n,) for n in nums)
        clause_seen |= not is_position
        pos = end
    for m in POSTFIX_RE.finditer(t):
        n = norm_number(m.group(1))
        if n:
            out.append((n,))
    return out


def heads(ds):
    """The distinct heads of designators (a compound counts by its head)."""
    return {d[0] for d in ds}


def raw_numbers(text):
    """Every integer token of the text (candidate G3a, refused)."""
    return [str(int(x)) for x in re.findall(r'(?<!\d)(\d{1,6})(?!\d)', text or '')]


def title_key(s):
    """G5's comparable title: NFKD, accents and case folded, one leading designator dropped ('5. Snagging',
    'Lot 3: ...') ONLY when a separator `. : ) - – —` follows it ('Pakiet 2' -> 'pakiet 2', 'Lot 06 désamiantage' ->
    'lot 06 desamiantage'), runs of non-word characters collapsed. Moved here from g3_port.py in unit 8 round 2,
    byte for byte (g3_port.title_key is this function), so the decided FINAL is unchanged."""
    if not s:
        return ''
    s = unicodedata.normalize('NFKD', s.lower())
    s = ''.join(ch for ch in s if not unicodedata.combining(ch))
    s = re.sub(r'^\s*(?:(?:lot|lots|los|lote|lotto|pakiet|czesc|zadanie|part|partie|perceel|cast)\s*'
               r'(?:n[or°º]\.?\s*)?)?[0-9ivxlc]{1,4}\s*[.:)\-–—]+\s*', '', s)
    return re.sub(r'[^\w]+', ' ', s).strip()


# (text, expected key) -- the Rust `title_key` unit tests copy these cases (unit 8 round 2, owner decision).
TITLE_KEY_SELFTEST = [
    ('Pakiet 2', 'pakiet 2'),                          # no separator after the designator: nothing is dropped
    ('Lot 06 désamiantage', 'lot 06 desamiantage'),    # 5071935's block title: no separator, accents folded
    ('Lot 06 DESAMIANTAGE', 'lot 06 desamiantage'),    # its CN lot title: the same key
    ('5. Snagging', 'snagging'),                       # 4854639's lot title: a bare number + '.' is dropped
    ('Lot 1 : Dents', 'dents'),
    ('LOT 3 – Nettoyage', 'nettoyage'),
    ('Lot n° 4 - Voirie', 'voirie'),
    ('Pakiet nr 3. Leki', 'leki'),
    ('Part 2) Cleaning', 'cleaning'),
    ('', ''),
]


# (text, expected designators) -- the Rust reader's unit tests copy these cases.
SELFTEST = [
    # --- unit 7's 35 cases (all unchanged by unit 8)
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
    # --- unit 8: the decision critic's probes (u8-oos/critic-and-review.json findings 1, 3, 4), output after the fix
    ('Część 1 – 50 miejsc', [('1',)]),                         # (a) 4667265's own CN 12540916 lot title; unit 7 read 1..50
    ('Lot 1 - 5 ordinateurs portables', [('1',)]),             # (a) unit 7: 1..5
    ('Lot 12 – 15 sites', [('12',)]),                          # (a) unit 7: 12..15
    ('Lot 3 - 2 véhicules', [('3',)]),                         # (a) unit 7: [3, 2]
    ('Pakiet 8 poz. 6 i 7', [('8', '6', '7')]),                # (c) one compound, head 8; unit 7: [8, 6, 7]
    ('Pakiet 8 poz. 6-7', [('8', '6', '7')]),                  # (c) unit 7: [8, 6, 7]
    ('Pakiet 3 (poz. 1-12)', [('3',)]),                        # (c) `poz.` after a pakiet clause is no keyword; unit 7: [3, 1..12]
    ('Elementy urządzenia typu UPP-1 (pakiet 8 pozycji) w łącznej ilości 116 szt. - zadanie 4.', [('4',)]),
    #                                                            (c) 8286472: `pakiet 8 pozycji` is a count; unit 7: [8, 4]
    ('Pakiet 16 zad.1', [('16', '1')]),                        # (c) 5416534's CAN; unit 7: [16, 1]
    ('Suministro de los 2 vehículos', []),                     # (b) the Spanish article; unit 7: [2]
    ('Los 3: Reinigung', [('3',)]),                            # (b) the German keyword stays
    ('Lotes 1 a 5', [('1',), ('5',)]),                         # ACCEPTED: `a` stays a conjunction (Czech/Slovak "1 a 2")
    ('Lot 2, 3 ans', [('2',)]),                                # (E) a duration; unit 7: [2, 3]
    ('Pakiet 5, 2 szt.', [('5',)]),                            # (E) a quantity; unit 7: [5, 2]
    ('Szkolenie dla grupy 20 osób', []),                       # (E) a group size; unit 7: [20]
    ('Lot 2 (remplace le lot 5 infructueux)', [('2',), ('5',)]),  # ACCEPTED: prose naming another lot
    ('LOT C', [('100',)]),                                     # ACCEPTED: a one-letter Roman numeral
    ('Pakiet 8 poz. 6, Pakiet 8 poz. 7', [('8', '6'), ('8', '7')]),  # two compounds, ONE head: G3 counts heads
    # --- unit 8: the rules' own edges
    ('LOS 2', [('2',)]),
    ('Losnr. 4', [('4',)]),
    ('de los 4 ascensores del lote 2', [('2',)]),              # the critic's G3 shape: unit 7 read [4, 2]
    ('Pakiet 1 - pozycja 2', [('1',)]),                        # the position word after a pakiet clause; unit 7: [1, 2]
    ('Pozycja nr 3 – Paracetamol 500 mg', [('3',)]),           # a position word with no clause before it is a keyword
    ('Pakiet 8 pozycja 3', [('8', '3')]),                      # a position word followed by a number is a compound half
    ('Część 2 - 80 miejsc', [('2',)]),                         # unit 7 read 2..80
    ('Lots 3 – 4', [('3',)]),                                  # a spaced dash ends the list even between two lots
    ('Lots 3-4', [('3',), ('4',)]),
    ('LOTS 3 ET 2', [('3',)]),                                 # separators are case-sensitive (review finding 0, option A)
    ('Lot vi, VII', []),                                       # a lower-case Roman word is no number
    ('zadanie 5 do 20 dni', [('5',)]),                         # (E) a range word into a duration
    ('Lot 10 sols', [('10',)]),                                # 5071935: a lot number before a noun stays
    # --- unit 8 round 2 (owner decision after the port + verify stages)
    ('Lots 1 - 2 - 3', [('1',)]),                              # ACCEPTED recall cost of (a): spaced dashes end the list
    ('Lot 1 – 50 places', [('1',)]),                 # no-break spaces are spaces (verifier's build note)
    # --- unit 8 review: each half of the tight-dash rule (a) alone, with no quantity word to hide it, the count
    #     rule (c) only after a pakiet keyword, and the input preconditions (ASCII digits; read as parsed, no NFC)
    ('Lot 1- 5 ordinateurs', [('1',)]),                        # (a) a space after the dash
    ('Lot 1 -5 ordinateurs', [('1',)]),                        # (a) a space before the dash
    ('Lots 3- 4', [('3',)]),
    ('Lots 3 -4', [('3',)]),
    ('Pakiet nr 1- 12 000,00 PLN.', [('1',)]),                 # a cached TED-CONTENTS value: an amount
    ('Pakiet nr 2 -100,51;', [('2',)]),                        # a cached TED-CONTENTS value
    ('Zadanie 8 pozycji', [('8',)]),                           # (c) `N pozycji` is a count only after a pakiet keyword
    ('Lot \uff13', []),                                        # a fullwidth digit is no digit (ASCII 0-9 only)
    ('Cze\u0328s\u0301c\u0301 2', []),                          # decomposed (NFD) text is read as parsed
]

if __name__ == '__main__':
    bad = [(t, designators(t), want) for t, want in SELFTEST if designators(t) != want]
    for t, got, want in bad:
        print('FAIL', repr(t), got, '!=', want)
    bad_k = [(t, title_key(t), want) for t, want in TITLE_KEY_SELFTEST if title_key(t) != want]
    for t, got, want in bad_k:
        print('FAIL title_key', repr(t), repr(got), '!=', repr(want))
    print(f'{len(SELFTEST) - len(bad)}/{len(SELFTEST)} designators ok, '
          f'{len(TITLE_KEY_SELFTEST) - len(bad_k)}/{len(TITLE_KEY_SELFTEST)} title_key ok')
    raise SystemExit(1 if bad or bad_k else 0)
