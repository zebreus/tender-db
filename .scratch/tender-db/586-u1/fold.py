"""Issue 586 unit 1 (a): the reference implementation of the touchpoint alias fold's two tiers (fold.md).

A touchpoint is compared ONLY with its own parent Organization section in the same notice
(`touchpoint_decision`'s `parent` map), name by name over every language either publishes; `store::org_name_norm`
is untouched and its equality stays the first alias test. This file is the oracle the Rust port
(crates/ingest/src/project.rs) is checked against: `tier(touchpoint_name, parent_name)` answers 'A', 'C0', 'C1',
'C2', 'C3', 'C3r' or None; vectors.json holds both keys and the tier of all 3,751 saved pairs.

    python3 -I fold.py --selftest | --key NAME [PARENT] | --tier TOUCHPOINT PARENT

Key tokens of a name (`key_tokens(name, side)`, both sides):
  1. html entities decoded: &#NNN; &#xHHH; &amp; &quot; &apos; &lt; &gt; &nbsp;
  2. lower-cased; each char through NFKD with every combining char dropped, ONLY for the blocks of
     crates/ingest/src/project/title_key_nfkd.rs (the port reuses `title_key_nfkd::NFKD_STRIPPED`; U+037E -> ';',
     fullwidth -> ASCII, 'º' -> 'o'); then EXTRA_FOLD (letters NFKD leaves whole); lower-cased again;
  3. brackets () [] {} cut out by the side rule below (first closer of the same kind; an unclosed opener runs
     to the end; a stray closer is a space);
  4. every char that is not alphanumeric (str.isalnum / char::is_alphanumeric) is a space; split on spaces;
  5. LEGAL_ANYWHERE phrases dropped wherever they stand (leftmost, longest first);
  6. LEGAL_TRAILING phrases dropped from the END, repeatedly, longest first, never down to nothing;
  7. ARTICLES and HONORIFICS tokens dropped wherever they stand;
  8. every run of two or more one-character tokens joined into one ('c u c' -> 'cuc');
  9. touchpoint side only: one TP_HEAD_LABELS phrase cut from the head ('punto contacto ...'), never to nothing.

Bracket side rule (step 3), the one asymmetry:
  - parent side: every bracket is cut (what the parent prints in brackets describes the parent: "(SARGA)",
    "(AVRA)", "(Agrupación de Base)");
  - touchpoint side: a bracket is cut only when its content (steps 1-2 and 4, joined by one space) is in
    TP_BRACKET_LABELS or TP_BRACKET_PLACES, is empty, is only legal-form/article tokens ("(Medio Propio)"), or
    equals (after step 8) the content of one of the parent's own brackets ("(ICA)" / "(ICA)", "(OAGER)" /
    "(O.A.G.E.R.)"). Anything else stays in the key: "(TARCAS)", "(Contratación y Compras)", "( SERVCIO
    P..CIUDADANA)", "(PROCASA)", "(J. Arends)".

With T = touchpoint key and P = parent key (both non-empty):
  A    T == P.
  C0   T and P equal once a leading organ synonym is canonicalised on both (ORGAN_SYNONYMS: 'junta gobierno
       local' -> 'junta gobierno', 'pleno municipal' -> 'pleno', 'alcaldia presidencia' -> 'alcaldia').
  E  = P minus ONE leading ORGAN_PHRASES phrase (the longest that leads P and leaves something), else P.
  C1   an organ was cut, E is 2+ tokens or one token of 4+ chars, and T == E.
  C2   with MUNI words at the head of E and T canonicalised to 'ayuntamiento': E[0] == 'ayuntamiento',
       len(E) >= 2 and T == E ("Ajuntament de Biar" under "Junta de Gobierno del Ayuntamiento de Biar").
  C3   E[0] == 'ayuntamiento' (canonicalised), len(E) >= 2 and T == E[1:] (the bare town under its town hall).
  C3r  T[0] == 'ayuntamiento' (canonicalised), P has no MUNI head and P == T[1:] ("COMUNE DI LAURINO" under
       "LAURINO").
Never: an organ cut from the TOUCHPOINT, a bare place under anything but a town hall, a prefix/suffix or fuzzy
match.
"""
import html
import re
import sys
import unicodedata

# --- step 2: the char fold ---------------------------------------------------------------------------------------

# The blocks title_key_nfkd.rs covers; a char outside them passes through (the port's binary search misses).
NFKD_BLOCKS = [
    (0x00A0, 0x024F), (0x0250, 0x02FF), (0x0300, 0x036F), (0x0370, 0x03FF), (0x0400, 0x052F),
    (0x1AB0, 0x1AFF), (0x1D00, 0x1DFF), (0x1E00, 0x1FFF), (0x2000, 0x24FF), (0x2C60, 0x2C7F),
    (0x3000, 0x303F), (0x3300, 0x33FF), (0xA720, 0xA7FF), (0xFB00, 0xFB06), (0xFE10, 0xFE6F), (0xFF00, 0xFFEF),
]

# Letters NFKD leaves whole, folded after it. ASCII-ise the Latin ones; the Greek final sigma meets the medial.
EXTRA_FOLD = {
    'ł': 'l', 'ø': 'o', 'đ': 'd', 'ð': 'd', 'ħ': 'h', 'ı': 'i', 'ŧ': 't', 'ĸ': 'k', 'ŋ': 'n',
    'ß': 'ss', 'æ': 'ae', 'œ': 'oe', 'þ': 'th', 'ς': 'σ',
}


def _in_blocks(c):
    o = ord(c)
    return any(lo <= o <= hi for lo, hi in NFKD_BLOCKS)


def _nfkd_stripped(c):
    if not _in_blocks(c):
        return c
    return ''.join(x for x in unicodedata.normalize('NFKD', c) if unicodedata.combining(x) == 0)


_ENTITY = re.compile(r'&(#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|amp|quot|apos|lt|gt|nbsp);')
_NAMED = {'amp': '&', 'quot': '"', 'apos': "'", 'lt': '<', 'gt': '>', 'nbsp': ' '}


def decode_entities(s):
    def one(m):
        g = m.group(1)
        if g[0] == '#':
            n = int(g[2:], 16) if g[1] in 'xX' else int(g[1:])
            return chr(n) if 0 < n <= 0x10FFFF and not 0xD800 <= n <= 0xDFFF else m.group(0)
        return _NAMED[g]
    return _ENTITY.sub(one, s)


def char_fold(s):
    s = decode_entities(s).lower()
    out = []
    for c in s:
        for x in _nfkd_stripped(c):
            out.append(EXTRA_FOLD.get(x, x))
    return ''.join(out).lower()


# --- step 3/4: brackets and punctuation ----------------------------------------------------------------------------

_CLOSE = {'(': ')', '[': ']', '{': '}'}


def split_brackets(s):
    """(outside text with each bracket replaced by a NUL marker, [bracket contents]) of a char-folded string."""
    out, contents, i = [], [], 0
    while i < len(s):
        c = s[i]
        if c in _CLOSE:
            j = s.find(_CLOSE[c], i + 1)
            if j < 0:
                j = len(s)
            contents.append(s[i + 1:j])
            out.append('\0')
            i = j + 1
        elif c in ')]}':
            out.append(' ')
            i += 1
        else:
            out.append(c)
            i += 1
    return ''.join(out), contents


def tokens_of(s):
    return ''.join(c if c.isalnum() else ' ' for c in s).split()


# --- the word lists (all written as folded tokens) --------------------------------------------------------------

# Spelled-out legal forms: removed ANYWHERE (they are multi-word and unambiguous).
LEGAL_ANYWHERE = [
    'sociedad anonima unipersonal', 'sociedad limitada unipersonal', 'sociedad anonima municipal',
    'sociedad anonima', 'sociedad limitada', 'sociedad de responsabilidad limitada', 'sociedad cooperativa',
    'societat anonima', 'societat limitada', 'sociedade anonima', 'sociedade unipessoal',
    'spolka z ograniczona odpowiedzialnoscia', 'spolka akcyjna', 'spolka komandytowa', 'spolka jawna',
    'akciova spolecnost', 'spolecnost s rucenim omezenym', 'spolocnost s rucenim obmedzenym',
    'societa per azioni', 'societa a responsabilita limitata', 'societa cooperativa', 'societa consortile',
    'aktiengesellschaft', 'gesellschaft mit beschrankter haftung', 'anstalt des offentlichen rechts',
    'anstalt offentlichen rechts', 'korperschaft des offentlichen rechts',
    'naamloze vennootschap', 'besloten vennootschap',
    'societe anonyme', 'societe par actions simplifiee', 'societe a responsabilite limitee',
    'public limited company', 'medio propio personificado', 'medio propio', 'mitja propi',
]
# Abbreviated legal forms: removed only at the END of the name, repeatedly ("..., S.A.U. M.P." -> "...").
LEGAL_TRAILING = [
    # ES / PT / IT / FR (S.A., S.L., S.A.U., S.L.U., S.A.M., S.M.E., M.P. medio propio, E.P.E., O.A., A.I.E.,
    # F.S.P., I.P., E.M., Lda.; S.p.A., S.r.l., S.c.a r.l.; SAS, SARL ...)
    's a u', 'sau', 's l u', 'slu', 's a', 'sa', 's l', 'sl', 's l l', 'sll', 's l p', 'slp', 's coop', 'soc coop',
    's a l', 'sal', 's a m', 'sam', 's m e', 'sme', 'm p', 'mp', 'e p e', 'epe', 'e p e l', 'epel', 'o a', 'oa',
    'o a l', 'oal', 'e m', 'em', 'a i e', 'aie', 'e i r l', 'eirl', 'lda', 'unipessoal lda', 'f s p', 'fsp',
    'i p', 'ip', 's p a', 'spa', 's r l', 'srl', 's c a r l', 'scarl', 's c r l', 'scrl', 'onlus', 's n c', 'snc',
    's a s', 'sas', 's a r l', 'sarl', 'eurl', 'sasu', 'spl', 'sem', 'scop', 'gie',
    # DE / AT / NL / BE
    'gmbh', 'ggmbh', 'mbh', 'gmbh co kg', 'co kg', 'kg', 'ag', 'e v', 'ev', 'a o r', 'aor', 'k d o r', 'kdor',
    'b v', 'bv', 'n v', 'nv', 'u a', 'ua', 'v o f', 'vof', 'bvba', 'sprl', 'cvba', 'vzw', 'asbl',
    # UK / IE
    'ltd', 'limited', 'plc', 'llp', 'llc', 'inc', 'cic', 'dac', 'clg',
    # PL / CZ / SK / HU / HR / SI / MK / BG / RO
    'sp z o o', 'sp z oo', 'spzoo', 'sp k', 'spk', 'sp j', 's k a', 's r o', 'sro', 'spol s r o', 'a s', 'as',
    'p o', 'o p s', 'kft', 'zrt', 'nyrt', 'd o o', 'doo', 'dooel', 'eood', 'ood', 'ead', 'еоод', 'оод', 'еад',
    # Nordic / Baltic / GR
    'asa', 'ab', 'publ', 'oy', 'oyj', 'aps', 'uab', 'sia',
    'α ε', 'αε', 'ε π ε', 'επε', 'ι κ ε', 'ικε', 'ο ε', 'οε',
]
# Left out on purpose (two letters that also abbreviate a Spanish/Italian department or region at the end of a
# name, and none fired on the population): sc / s c (Servicio de Contratación), uc (Unidad de Contratación),
# se (Secretaría de Estado), cv (Comunitat Valenciana), s p, z s, i s, dd, bt, eg, ou, ad.
# Articles, prepositions and conjunctions: removed ANYWHERE (es ca gl pt fr it de nl en pl).
ARTICLES = set('''
el la los las lo de del al y e
els les l d dels i
o a os as do da dos das
le du des et au aux
il gli di dello della dei degli delle dell ed
der die das dem den und
het van
the of and
w we
en con
'''.split())
# Honorific styles of Spanish public bodies: removed ANYWHERE ("Excmo. Ayuntamiento de ...").
HONORIFICS = set('excmo excma exmo exma excelentisimo excelentisima ilmo ilma iltmo iltma iltre ilustrisimo '
                 'ilustrisima ilustre'.split())

# Touchpoint-side bracket contents that are cut (folded, articles kept, joined by one space).
TP_BRACKET_LABELS = {
    'punto de contacto', 'otro punto de contacto', 'punto contacto', 'pc', 'tpo', 'contact point',
    'point de contact', 'punt de contacte', 'sede electronica', 'registro', 'registro electronico',
    'registro general', 'preguntas y respuestas', 'a traves de placsp', 'na', 'n a',
}
# ... and a bracket naming exactly a Spanish province or autonomous community ("Ayuntamiento de Lucena (Córdoba)").
# Folded, articles kept, one space between words: the bracket's whole content must be one of these.
TP_BRACKET_PLACES = {
    'alava', 'araba', 'albacete', 'alicante', 'alacant', 'almeria', 'asturias', 'avila', 'badajoz', 'barcelona',
    'burgos', 'caceres', 'cadiz', 'cantabria', 'castellon', 'castello', 'ciudad real', 'cordoba', 'a coruna',
    'la coruna', 'coruna', 'cuenca', 'girona', 'gerona', 'granada', 'guadalajara', 'gipuzkoa', 'guipuzcoa',
    'huelva', 'huesca', 'illes balears', 'islas baleares', 'baleares', 'jaen', 'leon', 'lleida', 'lerida',
    'la rioja', 'rioja', 'lugo', 'madrid', 'malaga', 'murcia', 'navarra', 'ourense', 'orense', 'palencia',
    'las palmas', 'pontevedra', 'salamanca', 'santa cruz de tenerife', 'tenerife', 'segovia', 'sevilla', 'soria',
    'tarragona', 'teruel', 'toledo', 'valencia', 'valladolid', 'bizkaia', 'vizcaya', 'zamora', 'zaragoza',
    'ceuta', 'melilla', 'andalucia', 'aragon', 'canarias', 'castilla la mancha', 'castilla y leon', 'cataluna',
    'catalunya', 'comunidad de madrid', 'comunitat valenciana', 'extremadura', 'galicia', 'pais vasco', 'euskadi',
    'region de murcia',
}

# A contact label leading the touchpoint's name ("Punto de contacto - Tribunal ..."), cut from the head of the
# touchpoint key only (key tokens: articles already gone).
TP_HEAD_LABELS = ['otro punto contacto', 'punto contacto']

# Tier C: organ phrases cut from the head of the PARENT key (key tokens: articles already gone). Governing and
# collegiate organs and their officers; NOT departments (Secretaría General, Concejalía, Área, Servicio,
# Distrito, Dirección Provincial, Delegación): a parent printed as a department keeps its department words.
ORGAN_PHRASES = [
    'presidencia consejo administracion', 'gerencia consejo administracion', 'alcaldia presidencia', 'junta gobierno local', 'junta govern local',
    'xunta goberno local', 'junta gobierno', 'junta govern', 'xunta goberno', 'pleno', 'ple', 'pleno municipal',
    'alcaldia', 'presidencia', 'vicepresidencia', 'gerencia', 'direccion gerencia', 'direccion gerente',
    'consejo administracion', 'consell administracio', 'consello administracion', 'consejo rector',
    'consell rector', 'consello reitor', 'rectorado', 'consejo gobierno', 'consell govern', 'consello goberno',
    'consejo gobierno insular', 'comision ejecutiva', 'comision gestora', 'consejero delegado',
    'consejera delegada', 'consejeria delegada', 'direccion general', 'direccion ejecutiva', 'direccion',
    'asamblea', 'asamblea concejales', 'asamblea representantes', 'junta general', 'junta vecinal',
    'consejo general', 'consejo comarcal', 'junta directiva', 'consejo directivo', 'junta plenaria',
    'junta rectora', 'comision permanente', 'asamblea vecinal', 'asamblea general', 'comite directivo',
    'administrador unico', 'organo administracion',
    # officers
    'rector', 'rectora', 'presidente', 'presidenta', 'vicepresidente', 'vicepresidenta', 'alcalde', 'alcaldesa',
    'alcalde presidente', 'gerente', 'director', 'directora', 'director gerente', 'directora gerente',
    'director general', 'directora general',
]
ORGAN_PHRASES.sort(key=lambda p: -len(p.split()))
# Tier C0: one organ under two spellings, at the head of BOTH keys (the parent's organ, printed by the touchpoint).
ORGAN_SYNONYMS = {
    'junta gobierno local': 'junta gobierno', 'junta govern local': 'junta govern',
    'xunta goberno local': 'xunta goberno', 'pleno municipal': 'pleno', 'alcaldia presidencia': 'alcaldia',
}
# Municipal words canonicalised to 'ayuntamiento' at the head of a tier-C key (one token each): the Spanish forms
# the issue names, their abbreviations, and the same word in the other languages the population prints.
MUNI = {'ayuntamiento', 'ajuntament', 'concello', 'ayto', 'aj',
        'comune', 'commune', 'mairie', 'ville', 'gemeente', 'gmina', 'miasto', 'stadt', 'gemeinde', 'municipio',
        'concelho', 'obec', 'mesto', 'kommune'}

_LEGAL_ANY = sorted((p.split() for p in LEGAL_ANYWHERE), key=len, reverse=True)
_LEGAL_END = sorted((p.split() for p in LEGAL_TRAILING), key=len, reverse=True)
_ORGANS = [p.split() for p in ORGAN_PHRASES]
_TP_HEAD = [p.split() for p in TP_HEAD_LABELS]
_SYN = sorted(((k.split(), v.split()) for k, v in ORGAN_SYNONYMS.items()), key=lambda kv: -len(kv[0]))


def _drop_anywhere(toks, phrases):
    out, i = [], 0
    while i < len(toks):
        for p in phrases:
            if toks[i:i + len(p)] == p:
                i += len(p)
                break
        else:
            out.append(toks[i])
            i += 1
    return out


def _drop_trailing(toks, phrases):
    changed = True
    while changed and toks:
        changed = False
        for p in phrases:
            if len(toks) > len(p) and toks[-len(p):] == p:   # never strip a name to nothing
                toks = toks[:-len(p)]
                changed = True
                break
    return toks


def _legal_or_article_only(toks):
    rest = _drop_trailing(_drop_anywhere(toks, _LEGAL_ANY), _LEGAL_END)
    rest = [t for t in rest if t not in ARTICLES]
    # _drop_trailing keeps a last phrase; treat a content that is ONE legal form as legal-only.
    return not rest or any(rest == p for p in _LEGAL_END)


def join_initials(toks):
    """Every run of two or more one-character tokens made one token: 'c u c' -> 'cuc' (C.U.C. = CUC)."""
    out, run = [], []
    for t in toks + ['']:
        if len(t) == 1:
            run.append(t)
            continue
        if len(run) >= 2:
            out.append(''.join(run))
        else:
            out.extend(run)
        run = []
        if t:
            out.append(t)
    return out


def fold_tokens(name, side, parent_brackets=()):
    """Tokens of a name after steps 1-5. side: 'parent' cuts every bracket; 'tp' cuts only the allowed ones."""
    outside, contents = split_brackets(char_fold(name))
    parts = outside.split('\0')
    pb = {' '.join(join_initials(tokens_of(b))) for b in parent_brackets}
    text = [parts[0]]
    for content, after in zip(contents, parts[1:]):
        ct = tokens_of(content)
        joined = ' '.join(ct)
        cut = (side == 'parent' or not ct or joined in TP_BRACKET_LABELS or joined in TP_BRACKET_PLACES
               or ' '.join(join_initials(ct)) in pb or _legal_or_article_only(ct))
        text.append(' ' if cut else ' ' + content + ' ')
        text.append(after)
    return tokens_of(''.join(text))


def key_tokens(name, side, parent_brackets=()):
    toks = fold_tokens(name, side, parent_brackets)
    toks = _drop_anywhere(toks, _LEGAL_ANY)
    toks = _drop_trailing(toks, _LEGAL_END)
    toks = join_initials([t for t in toks if t not in ARTICLES and t not in HONORIFICS])
    if side == 'tp':
        for h in _TP_HEAD:
            if len(toks) > len(h) and toks[:len(h)] == h:
                toks = toks[len(h):]
                break
    return toks


def parent_brackets(parent):
    return split_brackets(char_fold(parent))[1]


def key_a(name, side='parent', parent=None):
    pb = parent_brackets(parent) if parent is not None else ()
    return ' '.join(key_tokens(name, side, pb))


def _muni(toks):
    return ['ayuntamiento'] + toks[1:] if toks and toks[0] in MUNI else list(toks)


def organ_split(ptoks):
    """(the organ phrase cut, or None; the entity tokens) of a parent key: ONE organ phrase, the longest that
    leads the key and leaves something after it."""
    for p in _ORGANS:
        if len(ptoks) > len(p) and ptoks[:len(p)] == p:
            return ' '.join(p), ptoks[len(p):]
    return None, list(ptoks)


def _organ_syn(toks):
    for k, v in _SYN:
        if len(toks) > len(k) and toks[:len(k)] == k:
            return v + toks[len(k):]
    return list(toks)


def _entity_ok(e):
    """An entity left after the organ is cut: two tokens or more, or one of four characters or more."""
    return len(e) >= 2 or (len(e) == 1 and len(e[0]) >= 4)


def tier(tp, parent):
    """'A', 'C0', 'C1', 'C2', 'C3', 'C3r' or None: whether the touchpoint `tp` aliases its parent ORG `parent`."""
    pb = parent_brackets(parent)
    t = key_tokens(tp, 'tp', pb)
    p = key_tokens(parent, 'parent')
    if not t or not p:
        return None
    if t == p:
        return 'A'
    if _organ_syn(t) == _organ_syn(p):
        return 'C0'
    organ, e = organ_split(p)
    if organ and _entity_ok(e) and t == e:
        return 'C1'
    em, tm = _muni(e), _muni(t)
    if em[0] == 'ayuntamiento' and len(em) >= 2:
        if tm == em:
            return 'C2'
        if t == em[1:]:
            return 'C3'
    if tm[0] == 'ayuntamiento' and len(tm) >= 2 and p[0] not in MUNI and p == tm[1:]:
        return 'C3r'
    return None


def _selftest():
    cases = [
        # 586 unit 2's named tests
        ('Sociedad Aragonesa de Gestión Agroambiental S.L.U',
         'Sociedad Aragonesa de Gestión Agroambiental, S.L.U. (SARGA)', 'A'),                      # 540072 SARGA
        ('AGENCIA DE VIVIENDA Y REHABILITACION DE ANDALUCIA (PC)',
         'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (AVRA)', 'A'),                          # AVRA
        ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA  (Punto de contacto)',
         'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA', 'A'),
        ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA (otro punto de contacto)',
         'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA', 'A'),
        ('AGENCIA DE VIVIENDA DE REHABILITACION DE ANDALUCIA (punto de contacto)',
         'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA', 'A'),
        ('AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCIA', 'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCÍA', 'A'),
        ('DIRECCIÓN PROVINCIAL DE AVRA EN SEVILLA (punto de contacto)',
         'AGENCIA DE VIVIENDA Y REHABILITACIÓN DE ANDALUCIA', None),                                # a branch: mints
        ('Gmina Panki ul. Tysiąclecia 5\u037e 42-140 Panki', 'Gmina Panki ul. Tysiąclecia 5; 42-140 Panki', 'A'),
        ('Gmina Panki ul. Tysiąclecia 5\u037e 42-140 Panki', 'Gmina Panki', None),   # the live rows: parent + address
        ('TARCAS. AYUNTAMIENTO DE SEVILLA', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
        ('Ayuntamiento de Sevilla - TARCAS', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
        ('Ayuntamiento de Sevilla (TARCAS)', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
        ('Registro del Tribunal Administrativo de Recursos Contractuales (TARCAS)',
         'Junta de Gobierno del Ayuntamiento de Sevilla', None),
        ('Tribunal Administrativo de Recursos Contractuales de la Excma. Diputación de Cádiz',
         'Gerencia del Consejo de Administración de la Empresa Municipal de Aguas de Cádiz', None),
        ('Stadt Fürth - Zentrale Vergabestelle', 'Stadt Fürth', None),
        # touchpoint brackets: a department stays, a label / the parent's own acronym / a province goes
        ('Ayuntamiento de Alcobendas (Contratación y Compras)', 'Junta de Gobierno del Ayuntamiento de Alcobendas',
         None),
        ('AYUNTAMIENTO DE SEVILLA ( SERVCIO P..CIUDADANA)', 'Junta de Gobierno del Ayuntamiento de Sevilla', None),
        ('Ayuntamiento de Lucena (Córdoba)', 'Alcaldía del Ayuntamiento de Lucena', 'C1'),
        ('Instituto de las Industrias Culturales y las Artes de la Región de Murcia (ICA)',
         'Instituto de las Industrias Culturales y de las Artes de la Región de Murcia (ICA)', 'A'),
        ('Delegación del Gobierno en Asturias (registro)', 'Delegación del Gobierno en Asturias', 'A'),
        ('PUNTO DE CONTACTO TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA',
         'TRIBUNAL ADMINISTRATIVO DE RECURSOS CONTRACTUALES DE LA JUNTA DE ANDALUCIA', 'A'),
        # tier A details
        ("DTT S.c.a r.l. - Consorzio per l'attuazione", 'DTT S.c.a r.l. - Consorzio per l&#8217;attuazione', 'A'),
        ('ESPAÇO MUNICIPAL RENOVAÇÃO URBANA E GESTÃO DO PATRIMÓNIO, EM.SA',
         'Espaço Municipal - Renovação Urbana e Gestão do Patrimonio, E. M., S. A.', 'A'),
        ('CUC DEI COMUNI DI TORRE DEL GRECO E TRECASE', 'C.U.C. COMUNI DI TORRE DEL GRECO E TRECASE', 'A'),
        ('Ciepło-Jawor Sp. z o.o.', 'Ciepło-Jawor Spółka z ograniczoną odpowiedzialnością', 'A'),
        ('Fundación Española para la Ciencia y la Tecnología F.S.P.',
         'Fundación Española para la Ciencia y la Tecnología', 'A'),
        # tier C
        ('Junta de Gobierno Local del Ayuntamiento de Altea', 'Junta de Gobierno del Ayuntamiento de Altea', 'C0'),
        ('Alcaldía del Ayuntamiento de Segorbe', 'Pleno del Ayuntamiento de Segorbe', None),    # another organ
        ('Ayuntamiento de Ezcaray', 'Junta de Gobierno Local del Ayuntamiento de Ezcaray', 'C1'),
        ('GESPESA SAU', 'Gerencia de GESPESA, S.A.U.', 'C1'),
        ('CE', 'Presidencia de la CE', None),                                    # a one-token entity under 4 chars
        ('Ajuntament de Biar', 'Junta de Gobierno del Ayuntamiento de Biar', 'C2'),
        ('ayto carmona', 'Junta de Gobierno del Ayuntamiento de Carmona', 'C2'),
        ('Marugán', 'Pleno del Ayuntamiento de Marugán', 'C3'),
        ('COMUNE DI LAURINO', 'LAURINO', 'C3r'),
        ('Bilbao', 'Presidencia de la Autoridad Portuaria de Bilbao', None),  # a bare place under no town hall
        ('Ayuntamiento Benalmádena', 'Concejalía de Hacienda del Ayuntamiento de Benalmádena', None),  # dept parent
        ('Ayuntamiento de Sevilla', 'Tribunal Administrativo de Recursos Contractuales del Ayuntamiento de Sevilla',
         None),                                                                  # the parent is the review organ
    ]
    bad = 0
    for tp, parent, want in cases:
        got = tier(tp, parent)
        if got != want:
            bad += 1
            print('FAIL', repr(tp), '|', repr(parent), 'want', want, 'got', got, '|', key_a(tp, 'tp', parent), '|',
                  key_a(parent))
    print(f'selftest: {len(cases) - bad}/{len(cases)} pass')
    return bad == 0


if __name__ == '__main__':
    # python3 -I fold.py --selftest
    # python3 -I fold.py --key NAME [PARENT]      key of NAME (touchpoint side when PARENT is given) and of PARENT
    # python3 -I fold.py --tier TOUCHPOINT PARENT
    a = sys.argv[1:]
    if a[:1] == ['--selftest']:
        sys.exit(0 if _selftest() else 1)
    elif a[:1] == ['--key'] and len(a) in (2, 3):
        if len(a) == 2:
            print(key_a(a[1]))
        else:
            print(key_a(a[1], 'tp', a[2]))
            print(key_a(a[2]))
    elif a[:1] == ['--tier'] and len(a) == 3:
        print(tier(a[1], a[2]))
    else:
        print(__doc__)
