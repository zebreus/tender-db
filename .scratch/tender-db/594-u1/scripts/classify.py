"""classify.py -- the 594 U1 name classes for org 2660's mention names (pure; read by the summaries).
tarc     : TARC Andalucía spellings (a 'recurs…' + 'contract…' phrase, or the token TARC)
review   : another review body (a court: Tribunal Superior de Justicia, Juzgado/Sala de lo Contencioso, TACRC)
agency   : a Junta-sector body that is usually its own legal person (agencia, instituto andaluz, SAS/SAE, empresa pública,
           ente público, APES, consorcio, Consejo Audiovisual, Consejo de Transparencia …)
third    : a company, foundation or association (legal form or such a word): not a Junta body at all
unit     : a Junta-run establishment named alone (residencia, centro, museo, hospital …)
junta    : the Junta itself or a department of it (Consejería, Viceconsejería, Secretaría General, Dirección General,
           Delegación, D.T., Servicio de …, Junta de Andalucía, Presidencia …)
other    : anything else (read by hand)
blank    : empty name"""
import re, unicodedata

def fold(s):
    s = unicodedata.normalize('NFD', s or '')
    s = ''.join(c for c in s if not unicodedata.combining(c))
    return re.sub(r'\s+', ' ', s.lower()).strip()

AGENCY = [r'\bagencia\b', r'\binstituto andaluz\b', r'servicio andaluz de salud', r'\bsas\b',
          r'servicio andaluz de empleo', r'\bempresa publica\b', r'\bente publico\b', r'\bapes\b',
          r'agencia publica empresarial', r'consejo audiovisual', r'consejo de transparencia', r'camara de cuentas',
          r'parlamento de andalucia', r'defensor del pueblo', r'\bifapa\b', r'\bpuertos de andalucia\b',
          r'\bconsorcio\b', r'\bpatronato\b']
THIRD = [r'\bs\.?\s?l\.?$', r'\bs\.?\s?a\.?$', r'\bsl\b', r'\bute\b', r'\bfundacion\b', r'\basociacion\b', r'\basoc\b',
         r'\bcompania\b', r'\bcasa hogar\b', r'\bsociedad\b', r'\bcooperativa\b', r'\bslu\b']
JUNTA = [r'consejer', r'junta de andaluc', r'secretar', r'direccion general', r'\bd\.?\s?g\.?\s', r'delegaci', r'\bdeleg\b',
         r'^d\.?\s?t\.?\s', r'\bservicio\b', r'viceconsejer', r'presidencia', r'gabinete', r'\bjefatura\b',
         r'gerencia provincial', r'\bmesa de contratacion\b', r'\borgano de contratacion\b', r'comisionado',
         r'\bseccion\b', r'subdireccion', r'\bdepartamento\b', r'\bunidad\b', r'\boficina\b', r'\bcentro directivo\b',
         r'administracion de la junta', r'comunidad autonoma de andalucia', r'gobierno de andalucia',
         r'\bdelegado\b', r'\bconsejero\b', r'\bdelegada\b', r'\bconsejera\b', r'\bcoordinacion\b']
UNIT = [r'\bresidencia\b', r'\bcentro residencial\b', r'\bmuseo\b', r'\bhospital\b', r'\bcentro\b', r'\bconservatorio\b',
        r'\bbiblioteca\b', r'\barchivo\b', r'\buniversidad\b']
REVIEW = [r'tribunal superior de justicia', r'contencioso', r'\bjuzgado', r'\btsja?\b', r'\btacrc\b',
          r'tribunal administrativo central', r'audiencia nacional', r'tribunal supremo']

def klass(name):
    f = fold(name)
    if not f:
        return 'blank'
    if re.match(r'(acuerdo marco|contrato de|suministro|servicio de punto)\b', f) and not re.search(r'consejer|junta', f):
        return 'other'
    if (re.search(r'recurs', f) and re.search(r'contract|contrat', f)) or re.search(r'\btarc', f) \
            or re.search(r'tribunal administrativo de recursos', f):
        return 'tarc'
    if any(re.search(p, f) for p in REVIEW):
        return 'review'
    if any(re.search(p, f) for p in AGENCY):
        return 'agency'
    if any(re.search(p, f) for p in THIRD):
        return 'third'
    if any(re.search(p, f) for p in JUNTA):
        return 'junta'
    if any(re.search(p, f) for p in UNIT):
        return 'unit'
    return 'other'
