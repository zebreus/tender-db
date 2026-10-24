"""names.py -- classify the census 2781 rows and the 18 name_prefix listings; writes raw/names.json."""
import json, os, sys, collections
sys.path.insert(0, os.path.dirname(__file__))
from classify import classify, fold, _phrase_hits
HERE = '/home/user/tender-db/.scratch/tender-db/587-u1'
LISTINGS = {'g01_dcp': 'direction de la commande publique', 'g02_scp': 'service de la commande publique',
            'g03_smp': 'service des march', 'g04_dda': 'direction des achats', 'g05_sda': 'service des achats',
            'g06_udc': 'unidad de contrataci', 'g07_sdc': 'servicio de contrataci', 'g08_mdc': 'mesa de contrataci',
            'g09_odc': 'órgano de contrataci', 'g10_odc_ascii': 'organo de contrataci', 'g11_dzp': 'dział zamówień',
            'g12_ai': 'afdeling inkoop', 'g13_afd': 'afd. inkoop', 'g14_zv': 'zentrale vergabestelle', 'g15_v': 'vergabestelle',
            'g16_pd': 'procurement department', 'g17_pud': 'purchasing department', 'g18_pt': 'procurement team',
            'g19_ug': 'ufficio gare'}
CLASSES = ['BARE_STRICT', 'BARE_EXT', 'DEPT', 'OTHER_ORGAN']

def census():
    d = json.load(open(os.path.join(HERE, 'census_2781.json')))
    b = json.loads(d['body'])
    out = []
    for r in b['rows']:
        cls, det = classify(r['name'], r['phrases'])
        out.append({**r, 'cls': cls, 'residual': det})
    return b, out

def listings():
    orgs = {}
    for g, pfx in LISTINGS.items():
        d = json.load(open(os.path.join(HERE, 'raw', 'get', f'{g}.json')))
        for x in d['items']:
            cls, det = classify(x['name'])
            o = orgs.setdefault(x['id'], {**x, 'listings': [], 'cls': cls, 'residual': det, 'folded': fold(x['name']),
                                          'phrases': _phrase_hits(fold(x['name']))})
            o['listings'].append(g)
    return orgs

def tally(rows, key):
    t = collections.defaultdict(lambda: collections.defaultdict(lambda: [0, 0]))
    for r in rows:
        k = key(r)
        t[k][r['cls']][0] += 1
        t[k][r['cls']][1] += r['mentions']
    return {k: {c: v for c, v in sorted(cv.items())} for k, cv in sorted(t.items())}

if __name__ == '__main__':
    b, crow = census()
    orgs = listings()
    lst = list(orgs.values())
    res = {
        'census_rows': {'n': len(crow), 'org_id_range': [crow[0]['org_id'], crow[-1]['org_id']],
                        'by_class': tally(crow, lambda r: 'all')['all'],
                        'by_phrase_first': tally(crow, lambda r: r['phrases'][0])},
        'listings': {'orgs': len(lst), 'by_class': tally(lst, lambda r: 'all')['all'],
                     'by_listing': tally([dict(r, lk=g) for r in lst for g in r['listings']], lambda r: r['lk']),
                     'by_class_identified': tally(lst, lambda r: 'identified' if r['identifier'] else 'no_identifier')},
    }
    json.dump({'summary': res, 'census_rows': crow, 'listing_orgs': lst}, open(os.path.join(HERE, 'raw', 'names.json'), 'w'),
              ensure_ascii=False, indent=1)
    print(json.dumps(res, indent=1, ensure_ascii=False))
