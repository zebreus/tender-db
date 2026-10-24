"""win.py KIND I [OUTFILE] -> the window statement for window i.
KIND m = minted general (salt 576-u5-, W=3000), n = minted NL AddInfo/TenderReceipt (salt 576-u5-, W=20000),
     o = minted, roles other than ReviewOrg/ReviewInfo (salt 576-u5-, W=20000),
     a = aliased (salt 586-u3-, W=WA).
Window start: s = int(sha256(SALT + str(i))[:16], 16) mod L over the concatenated eForms tender-id segments
A = [1, 1210000] and B = [8530000, 8869999] (576 U4's mapping, located by tender GET probes in 576-u4/fetch-notes.md);
a = 1 + s if s < |A| else 8530000 + (s - |A|)."""
import hashlib, sys, json
A_LO, A_HI = 1, 1210000
B_LO, B_HI = 8530000, 8869999
LA, LB = A_HI - A_LO + 1, B_HI - B_LO + 1
L = LA + LB
SALT = {'m': '576-u5-', 'n': '576-u5-', 'o': '576-u5-', 'a': '586-u3-'}
W = {'m': 3000, 'n': 20000, 'o': 20000, 'a': 3000}
TP_ROLES = ("'Lot-ReviewOrg', 'Lot-ReviewInfo', 'Lot-AddInfo', 'Lot-TenderReceipt', 'Lot-DocProvider', 'Lot-Mediator', "
            "'Lot-TenderEval', 'Lot-FiscalLegis', 'Lot-EnvironLegis', 'Lot-EmployLegis'")

def start(kind, i):
    h = hashlib.sha256((SALT[kind] + str(i)).encode()).hexdigest()
    s = int(h[:16], 16) % L
    a = A_LO + s if s < LA else B_LO + (s - LA)
    return h, s, a

MINTED = ("SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, "
          "p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, "
          "o.provisional AS org_provisional, c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq "
          "CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id "
          "AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 "
          "WHERE t.id BETWEEN {a} AND {b} AND p.mention_section_id LIKE 'TPO-%'{extra} "
          "GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40")

ALIASED = ("SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, "
           "p.mention_section_id AS section_id, min(l.lot_key) AS lot_min, count(*) AS n_rows, i.value AS tpo, "
           "tn.value AS tp_name, pn.value AS parent_name FROM tenders t "
           "CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq "
           "CROSS JOIN lots l ON l.id = p.lot_id "
           "CROSS JOIN notice_ids i ON i.notice_id = p.mention_notice_id AND i.section_id = l.lot_key "
           "AND i.field_id = 'OPT-301-' || p.role AND +i.value LIKE 'TPO-%' "
           "CROSS JOIN notice_sections s ON s.notice_id = i.notice_id AND s.section_id = i.value "
           "CROSS JOIN notice_texts tn ON tn.notice_id = i.notice_id AND tn.section_id = i.value AND tn.field_id = 'BT-500-Organization-TouchPoint' AND tn.ordinal = 0 "
           "CROSS JOIN notice_texts pn ON pn.notice_id = i.notice_id AND pn.section_id = p.mention_section_id AND pn.field_id = 'BT-500-Organization-Company' AND pn.ordinal = 0 "
           "WHERE t.id BETWEEN {a} AND {b} AND +p.mention_section_id LIKE 'ORG-%' AND +p.role IN (" + TP_ROLES + ") "
           "AND +s.parent_section_id = p.mention_section_id AND +s.kind = 'Touchpoint' AND lower(tn.value) <> lower(pn.value) "
           "GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id, i.value LIMIT {lim}")

def stmt(kind, i, w=None, lim=60):
    h, s, a = start(kind, i)
    b = a + (w or W[kind])
    if kind == 'a':
        sql = ALIASED.format(a=a, b=b, lim=lim)
    else:
        extra = {'n': " AND +p.role IN ('Lot-AddInfo', 'Lot-TenderReceipt') AND +o.country = 'NL'",
                 'o': " AND +p.role NOT IN ('Lot-ReviewOrg', 'Lot-ReviewInfo')"}.get(kind, '')
        sql = MINTED.format(a=a, b=b, extra=extra)
    return {'i': i, 'kind': kind, 'salt': SALT[kind], 'sha256_hex16': h[:16], 's': s, 'a': a, 'b': b, 'sql': sql}

if __name__ == '__main__':
    kind, i = sys.argv[1], int(sys.argv[2])
    w = int(sys.argv[4]) if len(sys.argv) > 4 else None
    d = stmt(kind, i, w)
    if kind == 'a' and len(sys.argv) > 5:          # a fixed single-tender window (the SARGA exhibit): a = b = argv[5]
        t = int(sys.argv[5]); d['a'] = d['b'] = t
        d['sql'] = ALIASED.format(a=t, b=t, lim=60); d['salt'] = None; d['fixed'] = True
    if len(sys.argv) > 3:
        open(sys.argv[3], 'w').write(d['sql'] + '\n')
    print(json.dumps({k: v for k, v in d.items() if k != 'sql'}))
