"""568 u8-oos group o1, SKEPTIC: write the skeptic's labels (o1_skeptic_labels.json) BEFORE the rule is re-run.

Starts from the measurer's o1_labels.json and records the skeptic's re-read. No class changed: every
matched block was re-read from cache_o1/ (o1_view.py, no guard computed) and the raw notice content.
Where the re-read added evidence or corrected a measurer reason, `why` is amended (prefixed "re-read:");
a class change would be prefixed "SKEPTIC:" (none).
Usage: python3 -I o1_skeptic_labels.py
"""
import json, os

HERE = os.path.dirname(os.path.abspath(__file__))

# 8338322: the CN (17907201, III.2.3) prints, per task ("dla zadania nr k"), the minimum annual delivery value a
# bidder must show; each block's estimate / 2 (24-month contract) sits just above the threshold of ITS printed
# number. printed -> (threshold PLN/yr, estimate/2)
THR_8338322 = {'RES-1': ('20', 450, 489.5), 'RES-2': ('2', 250, 250), 'RES-3': ('18', 800, 825), 'RES-4': ('19', 2000, 2200),
               'RES-5': ('3', 1200, 1225), 'RES-6': ('4', 2800, 2812.5), 'RES-7': ('5', 600, 600), 'RES-8': ('6', 400, 437.6),
               'RES-9': ('7', 800, 857.6), 'RES-10': ('8', 1400, 1419.6), 'RES-11': ('9', 600, 600),
               'RES-12': ('22', 35000, 37241), 'RES-13': ('14', 300, 337.5)}

R5084145 = {'RES-1': ('8', 15.39), 'RES-2': ('7', 4.47), 'RES-3': ('1', 197.13), 'RES-4': ('2', 53.98), 'RES-5': ('3', 44.84),
            'RES-6': ('14', 8.0), 'RES-7': ('6', 47.14), 'RES-8': ('12', 763.48), 'RES-9': ('9', 57.33), 'RES-10': ('10', 263.02),
            'RES-11': ('11', 57.06)}

G3_PAKIET = {'RES-4': ('8', '4'), 'RES-5': ('6', '5'), 'RES-8': ('3', '8')}


def main():
    labels = json.load(open(os.path.join(HERE, 'o1_labels.json'), encoding='utf-8'))
    out = []
    for b in labels:
        b = dict(b)
        tid, rk = b['tender_id'], b['result_key']
        if tid == 8338322:
            p, thr, half = THR_8338322[rk]
            b['why'] = (f"re-read: value agreement, not only numbering. The CN's per-task experience threshold 'dla zadania nr {p}' "
                        f"= {thr:,} PLN/yr, and this block's estimate / 2 (24-month contract) = {half:,} PLN/yr, just above it. "
                        "All 13 blocks fit their printed task this way (18 vs 19, whose descriptions are identical, are told apart: "
                        "800 vs 2,000). Title 'opatrunki' is every lot's title; printed = CONTRACT_NO, non-sequential, not a counter.")
        elif tid == 5084145:
            p, r = R5084145[rk]
            b['why'] = (f"re-read: title 'Вариант {p}' = LOT-{p}'s CN title exactly; CONTRACT_NO is a counter 1..11, LOT_NUMBER prints the lot. "
                        f"Award/lot value = {r}x (the group's range is 4.5x..763x, not 197x as the measurer wrote; total 69.9 M vs 2.0 M BGN "
                        "estimated) and no permutation of lots fits the values, so it is a value anomaly, not a placement error "
                        "(unit 7 judged 5284511's 'Grupa k' up to 237x right the same way).")
        elif tid == 5179195:
            b['why'] = ("re-read: title = lot k's description; also the CAN estimates are exactly 1/4 of the CN lot values "
                        "(50/200, 15/60, 12/48, 6/24 k EUR; the CN's 480,000 for lot 3 is a typo for 48,000, since its total 332,000 = 200+60+48+24).")
        elif tid == 5251601 and rk == 'RES-19':
            b['why'] = ("re-read: title 'Pakiet nr 21 – Siatki do operacji przepuklin – medyczne' = LOT-21's CN title exactly and the award "
                        "2,700 fits LOT-21's 3,150. Its INITIAL_ESTIMATED 14,925 equals LOT-22's CN value (and RES-20's estimate): "
                        "a copied estimate in the CAN, not a placement error.")
        elif tid == 8286472 and rk in G3_PAKIET:
            n, z = G3_PAKIET[rk]
            b['why'] = (b['why'] + f". re-read: right (zadanie {z} = LOT-{z}'s title, estimate = LOT-{z}'s CN value); the count phrase "
                        f"'(pakiet {n} pozycji)' (a package of {n} items, also in LOT-{z}'s own CN title) reads as designator Pakiet {n}.")
        elif tid == 8286472 and b['cls'] == 'wrong_other':
            k = int(rk.split('-')[1])
            b['why'] = (f"re-read: contract counter. LOT_NUMBER {k} = CONTRACT_NO {k:02d}, but the title says 'zadanie {k + 1}', restates "
                        f"LOT-{k + 1}'s CN title and its estimate = LOT-{k + 1}'s CN value. Lot 9 (423,000) and lot 19 (366,642.54) are absent "
                        "from the CAN's value list, so from RES-9 on every block sits one lot low.")
        out.append(b)
    with open(os.path.join(HERE, 'o1_skeptic_labels.json'), 'w', encoding='utf-8') as fh:
        json.dump(out, fh, ensure_ascii=False, indent=0)
    print(len(out), 'labels written')


if __name__ == '__main__':
    main()
