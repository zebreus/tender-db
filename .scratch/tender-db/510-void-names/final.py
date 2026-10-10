import re
from pred import fold, strip_q
VOID_STEMS = ["infructu","infructeu","sans suite","non attribu","declarado desiert","declarada desiert","queda desiert","nessuna aggiudicazione"]
VOID_WHOLE = ["desierto","desierta","deserto","deserta","desiertos","lote desierto","lotto deserto","gara deserta","non aggiudicato","non aggiudicata","lotto non aggiudicato","not awarded","lot not awarded","no award","no award made","contract not awarded","no contract awarded","niet gegund","uniewazniony","uniewazniono","postepowanie uniewaznione","brak ofert","aucune offre","aucune offre recue","pas d attributaire","pas d offre","sans offre","abandon","aufgehoben","nicht vergeben"]
PH_STEMS = ["would prejudice","not applicable","see section","perfil del contratante","perfil de contratante","voir autres informations","voir renseignements","ver informaci"]
PH_WHOLE = ["various"]
def classify(name):
    full = fold(name)
    if not full: return None, None
    pad = " " + full
    for p in VOID_STEMS:
        if " "+p in pad: return "void", "~"+p
    for p in PH_STEMS:
        if " "+p in pad: return "placeholder", "~"+p
    st = fold(strip_q(name))
    if st in VOID_WHOLE: return "void", "="+st
    if st in PH_WHOLE: return "placeholder", "="+st
    return None, None

AWARD_WORDS = {"attribue","attribuee","attribues","attribuees","attribuer"}
AWARD_PHRASES = (" avec la societe "," avec l entreprise "," avec les societes ")
def award_clause(f):
    w = f.split(" ")
    for i, x in enumerate(w):
        if x in AWARD_WORDS and not (i > 0 and w[i-1] in ("non","pas","sans")):
            return True
    pad = " " + f + " "
    return any(p in pad for p in AWARD_PHRASES)
def classify_x(name):
    k, e = classify(name)
    if k == "void" and e.startswith("~") and award_clause(fold(name)):
        return None, None
    return k, e
