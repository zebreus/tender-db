#!/usr/bin/env python3
"""519 M7, offline: one row per target mention read by m7_read.py, with the published party facts (FTS OCDS release)
and the reviewer's reading (kind, action, target org, confidence). Writes m7.json.

Facts come from m7-raw/fts/<publication_id>.json (the published release package) and m7-raw/notice/<id>.json.
The reading (READ below) is the analyst's, from those facts plus the target orgs read in m7-raw/org_*.json and the
bounded name/identity seeks in m7-raw/*.json (sqlcache).
"""
import json, os, re
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, 'm7-raw')

CSU, Y63, NENC = 30929531, 31541949, 31556600
# (notice_id, section_id) -> (kind, action, target, confidence, note). '*' = every refused CSU-named 13T mention.
READ = {
    'csu': ('csu-own-party', 'rehome', CSU, 'high',
            "NECS names itself as the buyer party (role buyer, its own address and necsu.* contact) and lists NENC as a "
            "separate party; the contract is NENC's (PSR direct award, NENC place codes in the title). The party is "
            "the CSU acting as agent, as 0AR's own mentions already record it (procuringEntity on NENC/NHSE notices)."),
    (46991199, 'ORG-GB-NHS-13T'): ('nhse-named', 'rehome', Y63, 'medium',
            "Buyer named 'NHS England - North East & Yorkshire' under 13T; the text says NECS acts for NENC ICB "
            "(the contracting authority). NECS is hosted by NHS England, so the name is the host's region. Named party "
            "= NHS England NE&Y: Y63 org 31541949; its exact-name provisional twin is 31619786 (3 mentions). Not NENC's "
            "own name, so it cannot stay on 13T under the strict gate."),
    (46763776, 'ORG-GB-NHS-26a#2'): ('shared-party-id-supplier', 'rehome', 30915905, 'high',
            "Supplier NNUH published under the ICB's party id GB-NHS-26a (issue 484 shape); the same release also "
            "lists NNUH correctly as GB-NHS-RM1. Target: the RM1 identity org 30915905 (RM1 is not allowlisted)."),
    (46880633, 'ORG-GB-NHS-26A#2'): ('shared-party-id-supplier', 'rehome', 1280779, 'high',
            "Supplier James Paget published under the ICB's party id GB-NHS-26A; the same release lists it as "
            "GB-NHS-RGP. Target: RGP body survivor 1280779 (merged GBNHSRGP)."),
    (46899287, 'ORG-GB-NHS-15E'): ('other-icb-buyer-wrong-code', 'rehome', 31542353, 'high',
            "Buyer is NHS Black Country ICB (Dudley short-breaks contract; AGEM CSU contact at St John's House, "
            "Leicester) typed with BSol's sub-ICB code 15E. Target: QUA survivor 31542353."),
    (46751084, 'ORG-GB-NHS-15E#2'): ('shared-party-id-supplier', 'rehome', 31601027, 'high',
            "GP practice supplier given the ICB's party id; its own ODS code M85074 stands as 31601027."),
    (46751166, 'ORG-GB-NHS-15E#2'): ('shared-party-id-supplier', 'rehome', 31601028, 'high',
            "GP practice supplier given the ICB's party id; its own ODS code M85087 stands as 31601028."),
    (46751166, 'ORG-GB-NHS-15E#3'): ('shared-party-id-supplier', 'rehome', 31601029, 'high',
            "GP practice supplier given the ICB's party id; its own ODS code M85108 stands as 31601029."),
    (46907344, 'ORG-GB-NHS-15C'): ('other-icb-buyer-wrong-code', 'rehome', 30917049, 'high',
            "Buyer is NHS Devon ICB (Aperture House, Exeter; d-icb.contracting) typed with BNSSG's sub-ICB code 15C. "
            "Target: QJK survivor 30917049."),
    (46807952, 'ORG-GB-NHS-15N'): ('other-icb-buyer-wrong-code', 'rehome', 31536069, 'high',
            "Buyer is NHS BNSSG ICB (SCW CSU contact) typed with Devon's sub-ICB code 15N. Target: QUY survivor 31536069."),
    (46807955, 'ORG-GB-NHS-15N'): ('other-icb-buyer-wrong-code', 'rehome', 31536069, 'high',
            "As 46807952 (same day, sibling contract)."),
    (46955480, 'ORG-GB-NHS-93C'): ('csu-own-party', 'rehome', 31617815, 'high (party); target is the exact-name cluster',
            "NEL CSU names itself as buyer (Unex Tower contact) on an NCL CCG procurement (Haringey), with NEL ICB as "
            "review/mediation body. No CSU body exists (0DJ inactive, no GBNHS0DJ org); NEL CSU stands as 5 "
            "provisional clusters. Target: the exact-name cluster 31617815 (8 mentions); the largest is 6325843 (323)."),
    (46942387, 'ORG-GB-NHS-RTR54'): ('nenc-under-site-code', 'rehome (optional)', NENC, 'high',
            "Buyer is NENC ICB (NECS John Snow House, South Tees locality) typed with South Tees Hospitals' site code "
            "RTR54. Not a member (other-body name); a 317 rehome to QHM survivor 31556600 empties the site holder."),
    (46919481, 'ORG-GB-NHS-X7V4Q'): ('nenc-under-site-code', 'rehome (optional)', NENC, 'high',
            "Buyer is NENC ICB (NECS contact, ophthalmology award) typed with North Cumbria ICT's site code X7V4Q. "
            "Not a member; optional rehome to 31556600."),
    (46934518, 'ORG-GB-NHS-RNZ91'): ('site-own-name', 'none', None, 'high',
            "Supplier 'Salisbury Medical Practice' is the ODS name of RNZ91 itself; the holder is right and is no member."),
    (46825611, 'ORG-GB-NHS-11X'): ('other-party-supplier', 'out of scope (A2 held)', None, 'medium',
            "Supplier 'Rural Practice Network' on a Somerset ICB award, typed with the sub-ICB code 11X; 11X is "
            "anchor-held, so no target was sought."),
}


def party_facts(p):
    cp = p.get('contactPoint') or {}
    ad = p.get('address') or {}
    ident = p.get('identifier') or {}
    return {'id': p.get('id'), 'name': p.get('name'), 'roles': p.get('roles'),
            'identifier': f"{ident.get('scheme')}-{ident.get('id')}" if ident.get('id') else None,
            'additional_identifiers': [f"{a.get('scheme')}-{a.get('id')}" for a in (p.get('additionalIdentifiers') or [])],
            'contact_email': cp.get('email'), 'contact_name': cp.get('name'),
            'address': ', '.join(x for x in (ad.get('streetAddress'), ad.get('locality'), ad.get('postalCode')) if x)}


def main():
    ts = json.load(open(os.path.join(RAW, 'targets.json')))
    rows = []
    for t in ts:
        meta = json.load(open(os.path.join(RAW, 'notice', f"{t['notice_id']}.json")))
        pkg = json.load(open(os.path.join(RAW, 'fts', f"{meta['publication_id']}.json")))
        rels = pkg.get('releases') or []
        rel = rels[0] if rels else {}
        pid = t['section_id'][4:].split('#')[0]
        parties = [party_facts(p) for p in rel.get('parties', [])]
        mine = [p for p in parties if p['id'] == pid and p['name'] == t['name']] or \
               [p for p in parties if p['id'] == pid]
        others = [p for p in parties if p not in mine[:1]]
        tender = rel.get('tender') or {}
        if t['code'] == '13T' and t['name'] == 'NORTH OF ENGLAND COMMISSIONING SUPPORT':
            reading = READ['csu']
        elif t['why'] != 'refused':
            reading = ('nenc-as-buyer', 'fold (no rehome)', None, 'high',
                       'NENC named as the buyer party; published by NECS (necsu.* contact, NECS address).')
        else:
            reading = READ.get((t['notice_id'], t['section_id']))
        rows.append({
            **t, 'publication_id': meta['publication_id'], 'published_at': meta['published_at'],
            'releases_in_package': len(rels), 'release_tags': rel.get('tag'), 'release_buyer': rel.get('buyer'),
            'title': tender.get('title'), 'description_head': (tender.get('description') or '')[:400],
            'party': mine[0] if mine else None, 'other_parties': others,
            'nenc_listed_separately': any(re.search(r'north east (and|&) north cumbria', (p['name'] or '').lower())
                                          for p in others),
            'reading': None if reading is None else dict(zip(
                ('kind', 'action', 'target_org', 'confidence', 'note'), reading)),
        })
    csu = [r for r in rows if r['code'] == '13T' and r['reading']['kind'] == 'csu-own-party']
    summary = {
        'org_13T': 31600907,
        'note_31593407': 'is GBNHS03N (NHS South Yorkshire ICB), not 13T',
        'targets_read': len(rows),
        'notices_read': len({r['notice_id'] for r in rows}),
        'packages_read': len({r['publication_id'] for r in rows}),
        'csu_named': len(csu),
        'csu_named_role_buyer': sum(1 for r in csu if r['party'] and r['party']['roles'] == ['buyer']),
        'csu_named_release_buyer_is_13T': sum(1 for r in csu if (r['release_buyer'] or {}).get('id') == 'GB-NHS-13T'),
        'csu_named_nenc_listed_separately': sum(1 for r in csu if r['nenc_listed_separately']),
        'csu_named_nenc_role': dict(Counter('+'.join(p['roles']) for r in csu for p in r['other_parties']
                                            if re.search(r'north east (and|&) north cumbria', (p['name'] or '').lower()))),
        'csu_named_contact': dict(Counter(r['party']['contact_email'] for r in csu)),
        'csu_named_tags': dict(Counter('+'.join(r['release_tags']) for r in csu)),
        'csu_named_dates': [min(r['date'] for r in csu), max(r['date'] for r in csu)],
        'nenc_sample': sum(1 for r in rows if r['code'] == '13T' and r['why'] != 'refused'),
        'nenc_sample_contact': dict(Counter(r['party']['contact_email'] for r in rows
                                            if r['code'] == '13T' and r['why'] != 'refused')),
        'by_action': dict(Counter((r['code'], r['reading']['action'], r['reading']['target_org'])
                                  for r in rows if r['why'] == 'refused').items()),
    }
    summary['by_action'] = [{'code': k[0], 'action': k[1], 'target_org': k[2], 'mentions': v}
                            for k, v in summary['by_action'].items()]
    json.dump({'computed': '2026-10-12', 'summary': summary, 'rows': rows},
              open(os.path.join(HERE, 'm7.json'), 'w'), indent=1)
    print(json.dumps(summary, indent=1))


if __name__ == '__main__':
    main()
