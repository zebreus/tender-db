"""568 u8-oos group o2: the hand labels (written BEFORE the unit-7 rule was run; o2_run_rule.py reads them).

Per Tender: a default class for every matched block of the Tender, and per-block overrides keyed
"<can notice id>/<result key>". Each entry carries the reason (title/description/value/winner evidence).
"""
LABELS = {
    4930114: {'default': ('single_right', "CAN titles 'Pakiet k' equal the CN lot titles 'Pakiet k' of the printed LN (CONTRACT_NUMBER is a counter, LN follows the pakiet: lot 16 unawarded)")},
    4474165: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title of the printed lot (sanitary / electrical / building)')},
    5255589: {'default': ('single_right', "CONTRACT_TITLE 'Temat Ł.k' = CN lot title 'Temat Ł.k' of the printed lot (7, 8 unawarded)")},
    5287797: {'default': ('single_right', "CONTRACT_TITLE = the CN lot title without its '(Paket n)' suffix")},
    4950596: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title (paraphrase)')},
    4863888: {'default': ('single_right', 'CONTRACT_TITLE names the garrison (Toruń / Inowrocław) of the CN lot title')},
    4456278: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot description (calcaires / dioritiques / sables)')},
    5223189: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title of the printed lot (lot 2 unawarded)')},
    4603462: {'default': ('single_right', 'one CAN per lot; CONTRACT_TITLE = the CN lot description (unité territoriale) of the printed lot')},
    4744920: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title (Natura 2000 site) and the initial estimate = the CN lot value')},
    8360393: {'default': ('single_right', 'CONTRACT_TITLE = CN lot 1 title; estimate 4,300,000 CZK = lot 1 value')},
    5486834: {'default': ('single_right', 'CONTRACT_TITLE names the town of the CN lot description (Florina 2, Kastoria 3, Grevena 4, Ptolemaida 5)')},
    5170600: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title (the CN prints 2, 03, 4, 5, 6)')},
    5189170: {'default': ('single_right', "TITLE_CONTRACT 'Los k: <section>' = the CN lot title of the printed lot")},
    5416534: {'default': ('wrong_other', "contract counter: LN = CONTRACT_NUMBER = 1..105 while the titles name the real pakiet; the CN numbers its 111 lots sequentially (Pakiet 16 poz 1-6 = LOT-16..21, Pakiet 17 = LOT-22, ...). Pakiet 15 unawarded, so from RES-15 ('Pakiet 16 zad.1' on LOT-15 'PAKIET 15') every block sits on an earlier pakiet's lot"),
              'blocks': {f'18634900/RES-{k}': ('single_right', f"'Pakiet {k}' on LOT-{k} 'PAKIET {k}': the counter equals the lot until the first unawarded pakiet (15)") for k in range(1, 15)}},
    5260572: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title of the printed lot (regions / audits)')},
    4663756: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title, and the initial estimate = the CN lot value (62,729,196 / 14,136,320 / 22,616,680 HUF)')},
    4470191: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title (toner brands) of the printed lot')},
    5307717: {'default': ('single_right', "1-lot Tender: the CN's only lot ('Tenders for each individual municipality') is the whole procedure; the estimate 8,400,000 NOK = the Tender value")},
    4539729: {'default': ('single_right', "CONTRACT_TITLE 'Relacija ...' = the CN lot title (school bus route) of the printed lot")},
    4613136: {'default': ('single_right', 'RES-1 title and estimate 549,334.32 = CN lot 1; RES-2 title = CN lot 2 description')},
    5537089: {'default': ('single_right', "'Consommables pour labo' = CN lot 1 'Consommables labo'; value 127,497.38 EUR = the Tender value")},
    4986092: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot description (garment) of the printed lot (20, 21, 27 unawarded)')},
    4761481: {'default': ('single_right', 'CONTRACT_TITLE = the CN lot title of the printed lot')},
    8357839: {'default': ('single_right', "the CONTRACT_TITLE's school line = the CN lot description of the printed lot (lots 5 and 10 share 'Lycée de Trioncelle Baie-Mahault'; RES-4 prints 5, RES-7 prints 10)")},
}
