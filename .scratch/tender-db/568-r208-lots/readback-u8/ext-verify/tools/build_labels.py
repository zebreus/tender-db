"""Verifier's labels for the 66 blocks of the 3 new Tenders. The label and its reason come from the notices (nview):
the block's printed LOT_NUMBER and CONTRACT_TITLE against the CN lot with that number (the CN the CAN references and
the anchor). served_lot / guard are attached afterwards from served.json (fresh read) for the output only."""
import json, os, sys
VD = '/tmp/claude-0/-home-user-tender-db/3050fd14-5ca6-5dd7-8b63-f827e13fd8ce/scratchpad/rb8/ext-verify'
ld = lambda p: json.load(open(p, encoding='utf-8'))
served = ld(f'{VD}/served.json'); pre = ld(f'{VD}/preguard.json')
def secs(n): return ld(f'{VD}/cache3/notices_{n}_content.json')['sections']
def f1(s, fid):
    for v in s['values']:
        if v['field_id'] == fid: return v.get('value')
def cn_lots(n):
    out = {}
    for s in secs(n):
        if s['kind'] == 'Lot':
            out[str(f1(s, 'TED-LOT_NUMBER'))] = (s['section_id'], f1(s, 'TED-LOT_TITLE') or f1(s, 'TED-LOT_DESCRIPTION'))
    return out
REF = {  # CAN -> (the CN it references, the anchor CN)
    18258276: (18025780, 17545429),
    17517331: (17288398, 17101010), 17517659: (17101010, 17101010), 17518079: (17247214, 17101010),
    17518149: (17244355, 17101010), 17657578: (17474868, 17101010),
    18033024: (17900783, 17900783), 18242004: (18020916, 17900783), 18527820: (18020916, 17900783),
    18527875: (18020916, 17900783)}
labels = []
for key in sorted(served, key=lambda k: (k.split('/')[0], int(k.split('/')[1]), int(k.split('/')[2][4:]))):
    tid, n, rk = key.split('/'); tid, n = int(tid), int(n)
    blk = next(s for s in secs(n) if s['kind'] == 'LotResult' and s['section_id'] == rk)
    p, title, cno = str(f1(blk, 'TED-LOT_NUMBER')), f1(blk, 'TED-CONTRACT_TITLE'), f1(blk, 'TED-CONTRACT_NUMBER')
    ref, anc = REF[n]
    rl, al = cn_lots(ref), cn_lots(anc)
    right = pre[key]['verifier_right_lot']
    why = f"printed {p}, contract {cno!r}, title {title!r}"
    if tid == 5048894:
        why += (f"; = CN {ref} (its REF 2014/S 181-318987, file CNWL/T034) lot {p} {rl[p][1]!r}; the anchor CN {anc} prints that "
                f"lot as {[k for k, v in al.items() if v[1] == rl[p][1]][0]} on section {right} -> Tender key {right}. "
                "T034 relaunches T025 after 17923471 'awarding procedure discontinued' and cites it as prior publication: chained by the buyer's own reference")
    elif tid == 4836560:
        why += f"; = anchor CN {anc} {al[p][0]} {al[p][1]!r} -> {right}"
        if ref != anc and p not in rl:
            why += f" (its REF CN {ref} prints lot {p} under another number)"
    else:
        why += f"; = anchor CN {anc} {al[p][0]} {al[p][1]!r} -> {right}"
        if cno and 'nfructu' in cno.lower():
            why += '; a no-award (infructueux) block of that lot'
    lab = 'single_right'
    if key == '4836560/17518149/RES-1':
        why += ("; CAVEAT: relaunch CN 17244355 (the CAN's REF, 'lots 4,6,7 et 14') printed LOT_NUMBER 1..4 positionally, so "
                "its section printed 4 = lot 14 carried 'lot 14 : Equipements de cuisine' onto key LOT-4, the anchor's lot 4. The "
                "block is on lot 4's only key (right by identity); the key's served text is lot 14's (inherited carry defect, "
                "570 error-risk class 'one printed number naming different lots', not caused by the block)")
    if key == '4836560/17518149/RES-3':
        why += ("; right pre-guard (tentative LOT-14); G5 refuses it falsely because LOT-4 carries lot 14's title (above) "
                "while LOT-14 has only a description")
    s = served[key]
    guard = None
    if s['served'] is None:
        guard = s['reason'].split('refused ')[1].split(' ')[0] if 'refused' in s['reason'] else s['reason']
    labels.append({'tender_id': tid, 'notice_id': n, 'result_key': rk, 'label': lab, 'served_lot': s['served'],
                   'preguard_lot': pre[key]['preguard_lot'], 'guard': guard, 'why': why})
json.dump(labels, open(sys.argv[1], 'w'), indent=1, ensure_ascii=False)
import collections
print(len(labels), collections.Counter(l['label'] for l in labels), collections.Counter((l['tender_id'], l['served_lot'] is None) for l in labels))
