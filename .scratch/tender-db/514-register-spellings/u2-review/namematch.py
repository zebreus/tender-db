"""Issue 514 U2.1: a mechanical name comparison, used only to route names for reading.

classify(mention_name, register_names) -> (class, matched_register_name)
  exact     normalised token sets equal, or the mention's tokens are a subset of a register name's
  close     a fuzzy match (typo / word order): SequenceMatcher >= 0.85 or Jaccard >= 0.6
  contains  the mention holds a register name plus other words (a branch, a service, a statutory
            long form -- or a different body whose name embeds the holder's: read it)
  unmatched none of the above
It is NOT the review rule: a reviewer reads every non-exact name (hand notes in build_packet.py).
"""
import difflib
import re

STOP = {"limited", "ltd", "the", "plc", "cic", "cio", "llp", "and", "of", "for", "t", "a", "ta", "co",
        "company", "uk", "inc", "cyf", "cyfyngedig"}


def tokens(s):
    s = (s or "").lower().replace("&", " and ").replace("’", "").replace("'", "")
    stripped = re.sub(r"\([^)]*\)", " ", s)
    if re.search(r"[a-z0-9]", stripped):
        s = stripped
    return [t for t in re.findall(r"[a-z0-9]+", s) if t not in STOP]


def classify(name, register_names):
    A = set(tokens(name))
    best = ("unmatched", None)
    rank = {"exact": 0, "close": 1, "contains": 2, "unmatched": 3}
    for rn in register_names:
        if not rn:
            continue
        B = set(tokens(rn))
        if not A or not B:
            continue
        if A == B or A < B:
            c = "exact"
        elif (difflib.SequenceMatcher(None, " ".join(sorted(A)), " ".join(sorted(B))).ratio() >= 0.85
              or len(A & B) / len(A | B) >= 0.6):
            c = "close"
        elif B < A:
            c = "contains"
        else:
            continue
        if rank[c] < rank[best[0]]:
            best = (c, rn)
    return best
