"""fresh568 labels, written from the notices (view.py: pre-guard replay lot, no guard, no served lot) BEFORE any guard
outcome per block was looked at. Unit-7 criteria: title/description agreeing with the lot's title, value/winner agreeing
with the CN lot; a contract counter, CAN-only renumbering or a mis-chained CAN is wrong_other; one award block for
several lots printing one of them is multi_lot_contract.
Per Tender: a default (label, evidence) and per-block overrides keyed "<can notice id>/<result key>"."""
LABELS = {
    8365595: {'default': None, 'blocks': {
        '18617609/RES-1': ('wrong_other', "CAN-only renumbering: the CAN's only block prints LOT_NUMBER 1 but its title is "
                           "'... región východ' = CN LOT-3's title, and its estimate and award 98,562.00 EUR = LOT-3's value "
                           "exactly; LOT-1 is 'región západ', 583,219.00 EUR")}},
    5864590: {'default': None, 'blocks': {
        '19369697/RES-1': ('single_right', "'Bois de construction' = CN LOT-2 title (printed 2)"),
        '19369697/RES-2': ('single_right', "'Isolation et cloisons intérieures' = CN LOT-3 title (printed 3)"),
        '19369697/RES-3': ('wrong_other', "CAN mis-numbering: prints 3 (LOT-3 'Isolation et cloisons intérieures', already "
                           "awarded by RES-2) but its title 'Faux plafonds et plafonds décoratifs' = CN LOT-8's title "
                           "exactly; contract 2017-022 follows RES-2's 2017-021")}},
    8291946: {'default': ('multi_lot_contract', "one CAN block for several lots: the title lists 'обособени позиции № ...' "
                          "(several lots of the 60-lot CN) and the block prints only the first"), 'blocks': {}},
    4888356: {'default': ('single_right', "CONTRACT_TITLE (heading + scope) = the CN lot description of the printed lot "
                          "(both CNs 17199139 / 17972261); lot 15 unawarded"), 'blocks': {}},
    5362479: {'default': ('single_right', "CONTRACT_TITLE 'Lot k <subject>: ...' = the CN lot title/description of the "
                          "printed lot k (lots 3, 8, 10, 14 not in this CAN; RES-3/4 'déclaré sans suite' are still lot 4/5's "
                          "blocks)"), 'blocks': {}},
    4569314: {'default': ('single_right', "CONTRACT_TITLE = the CN lot description of the printed lot (CN prints 01..10; "
                          "12586585 is the relaunch of lots 1, 4, 9 whose CN 12472102 is in the chain)"), 'blocks': {}},
    5351872: {'default': ('single_right', "CONTRACT_TITLE = the CN lot description of the printed lot; 18349832 (file "
                          "15CG012) is the negotiated procedure for the open procedure's unsuccessful lots, referencing CN "
                          "2014/S 249-441281 (the anchor), so it is chained right; 18425315 awards the relaunched lots 1 and "
                          "15 of CN 18238684"), 'blocks': {}},
    5457789: {'default': ('single_right', "CONTRACT_TITLE 'Pakiet nr k ...' names the printed pakiet and the CAN's "
                          "INITIAL_ESTIMATED value = the CN lot k value to the cent; CONTRACT_NUMBER 'Pakiet nr k'"), 'blocks': {
        '18461129/RES-11': ('single_right', "title 'Pakiet nr 47 Różne leki tabl.' = CN LOT-47 description exactly "
                            "(estimate 4,145 vs lot 8,220: partial)"),
        '18461129/RES-18': ('single_right', "'Microdacyn / lub równoważne' = CN LOT-67 description; estimate 2,664.00 = "
                            "LOT-67 value"),
        '18461129/RES-8': ('single_right', "printed 37 and CONTRACT_NUMBER 'Pakiet nr 37'; INITIAL_ESTIMATED 9,975.00 PLN = "
                           "CN LOT-37's value to the cent (LOT-34: 2,082.96); award 9,733.40; winner Sanofi-Aventis (an "
                           "insulin maker; LOT-37 = 'Preparat insuliny'). Only the title 'Pakiet nr 34 „Cloxacillin/"
                           "Syntarpen' (= LOT-34's description) dissents: a copied title. Judged right on number + "
                           "contract number + value + winner; the title conflict is recorded")}},
}
