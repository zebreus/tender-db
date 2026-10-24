"""win.py KIND I -> writes the window statement for window i (KIND g = general W=3000, n = NL AddInfo/TenderReceipt W=20000).
Salt: sha256('576-u4-' + str(i)); first 16 hex digits as an int mod L over the concatenated eForms id segments
A = [1, 1210000] and B = [8530000, 8869999] (located by tender GET probes, see fetch-notes.md)."""
import hashlib, sys, json
A_LO, A_HI = 1, 1210000
B_LO, B_HI = 8530000, 8869999
LA, LB = A_HI - A_LO + 1, B_HI - B_LO + 1
L = LA + LB
W = {'g': 3000, 'n': 20000}

def start(i):
    h = hashlib.sha256(('576-u4-' + str(i)).encode()).hexdigest()
    s = int(h[:16], 16) % L
    a = A_LO + s if s < LA else B_LO + (s - LA)
    return h, s, a

SEL = ("SELECT t.id AS tender_id, p.seq AS seq, p.role AS role, p.organization_id AS org_id, p.mention_notice_id AS notice_id, "
       "p.mention_section_id AS section_id, min(p.lot_id) AS lot_id_min, count(*) AS n_rows, o.country AS org_country, o.name AS org_name, "
       "c.code AS tp_bt514 FROM tenders t CROSS JOIN tender_version_parties p ON p.tender_id = t.id AND p.seq = t.current_seq "
       "CROSS JOIN organizations o ON o.id = p.organization_id LEFT JOIN notice_codes c ON c.notice_id = p.mention_notice_id "
       "AND c.section_id = p.mention_section_id AND c.field_id = 'BT-514-Organization-TouchPoint' AND c.ordinal = 0 "
       "WHERE t.id BETWEEN {a} AND {b} AND p.mention_section_id LIKE 'TPO-%'{extra} "
       "GROUP BY t.id, p.seq, p.role, p.organization_id, p.mention_notice_id, p.mention_section_id LIMIT 40")

def stmt(kind, i):
    h, s, a = start(i)
    b = a + W[kind]
    extra = " AND +p.role IN ('Lot-AddInfo', 'Lot-TenderReceipt') AND +o.country = 'NL'" if kind == 'n' else ''
    return {'i': i, 'kind': kind, 'sha256_hex16': h[:16], 's': s, 'a': a, 'b': b, 'sql': SEL.format(a=a, b=b, extra=extra)}

if __name__ == '__main__':
    kind, i = sys.argv[1], int(sys.argv[2])
    d = stmt(kind, i)
    if len(sys.argv) > 3:
        open(sys.argv[3], 'w').write(d['sql'] + '\n')
    print(json.dumps({k: v for k, v in d.items() if k != 'sql'}))
