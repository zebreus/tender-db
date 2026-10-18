"""O1 extension labels: the 3 new G2 Tenders, written from the notices (../fresh568/view.py over the extension's bar
cache: the PRE-GUARD combined replay lot only, no guard computed, no served lot of an award block shown) BEFORE any
guard outcome, parity result, G2 report or served lot of these Tenders was looked at. Only the bar read's completeness
fields (tenders / read / incomplete / fetch_errors / gets) had been read. Views: scratchpad rb8/ext/view_<id>.txt.

Unit-7 criteria (u8-oos/o1_run_rule.py `classes`): single_right = one lot, correctly placed; multi_lot_contract = one
award for several lots; wrong_other = on a lot that is not its lot for another reason; unclear = cannot tell.
Evidence: the block's CONTRACT_TITLE against the CN lot title / description of the printed lot, the contract-number
sequence, values and winners. Per Tender: a default (label, evidence) and per-block overrides keyed
"<can notice id>/<result key>"."""
LABELS = {
    5048894: {'default': ('single_right', "CAN 18258276 (file CNWL/T034 = CN 18025780, which prints 1 Hardware, 2 Special "
                          "Seating, 3 Cushions; 570 maps its sections onto the anchor CN 17545429's keys: LOT-2 = no 1 "
                          "Hardware, LOT-1 = no 2 Special Seating, LOT-3 = no 3 Cushions). A multi-supplier framework: one "
                          "block per (lot, winner). Printed 1 'Wheelchairs, Modifications and Construction and Buggies' = "
                          "the Hardware lot ('provision of wheelchairs/complete wheelchair systems ...'); printed 2 "
                          "'Wheelchairs - Special Seating' = Special Seating; printed 3 'Cushions and Accessories' = "
                          "Cushions. Each block names one lot. The INITIAL_ESTIMATED 2,500,000.00 GBP on every block is "
                          "the whole framework's value (= the Tender value), not a lot's, so it is no multi-lot signal "
                          "(the 8275759 reading). The anchor CN is file CNWL/T025/CS (2013), the CAN's CN T034 (2014): a "
                          "re-tender welded into one Tender (inherited, issue 364's class); both CNs name the same three "
                          "lots, so the placement holds under either"), 'blocks': {}},
    4836560: {'default': ('single_right', "anchor CN 17101010 numbers 20 lots 'Lot NN: <trade>'; four relaunch CNs (lots "
                          "4,6,7,14 / 1,2,5,20 / 3,9,15,16,17 / 7,9) and five R2.0.8 CANs with no lot section, one per "
                          "relaunch file. Every block prints the real lot number and its CONTRACT_TITLE is 'Lot NN : "
                          "<trade>' = the anchor's lot NN description; the contract numbers run in lot order (13002 lot "
                          "3, 13003 lot 4, 13004 lot 6 ... 13017 lot 19; 13051-13054 lots 1, 2, 5, 20); one winner and "
                          "one value per block"), 'blocks': {
        '17518149/RES-1': ('single_right', "printed 4, 'Lot 04 : Etanchéité - zinguerie', contract 13003, MJP Bardage "
                           "(cladding): the anchor CN's lot 4, key LOT-4, the only key for lot 4. Recorded caveat, "
                           "inherited and not the block's: relaunch CN 17244355 ('lots 4,6,7 et 14') printed LOT_NUMBER "
                           "1..4 positionally, so its lot-14 section ('lot 14 : Equipements de cuisine') was carried "
                           "onto LOT-4 (identity remap; pre-570 the same section id lands there too) and LOT-4's carried "
                           "title/description now read lot 14's text. The block is on its lot by the procedure's "
                           "numbering; the lot's text is wrong")}},
    5226033: {'default': ('single_right', "anchor CN 17900783 numbers 13 brand lots; relaunch CN 18020916 prints the real "
                          "numbers 3, 4, 5, 9, 12, 13 (570 maps its sections onto the anchor keys). Every block's "
                          "CONTRACT_TITLE is exactly the CN lot description of the printed lot ('Véhicules de moins de "
                          "3,5 tonnes de marque <brand>' etc.), the 18033024 contract numbers run 2014-318..324 in lot "
                          "order. The INFRUCTUEUX blocks (18033024 RES-3/4/5/9/12/13, 18242004 RES-2..6) are each one "
                          "lot's no-award block, still that lot's (the 5362479 'déclaré sans suite' reading); their "
                          "'winner' is the word 'infructueux' parsed as an organization, a party-data defect outside "
                          "lot placement. The relaunch awards (18242004 RES-1 Land Rover, 18527820 RES-1 Volvo, "
                          "18527875 RES-1/2 Mercedes) name their relaunched lot"), 'blocks': {}},
}
