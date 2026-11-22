"""Joinery chosen when a cabinet is built (`src/joint/`): holes, rows that follow a resize, Edit."""
from helpers import *


def holes(faces, radius):
    """The centres of the round faces of this radius in `model.faces`' reply, rounded, sorted."""
    found = [f["centroid"] for f in faces["faces"] if abs((f.get("cylinder") or {}).get("radius", 0) - radius) < 0.01]
    return sorted([round(c[0], 1), round(c[1], 1), round(c[2], 1)] for c in found)


def hardware(result):
    return {h["item"]: h["qty"] for h in result["result"]["hardware"]}


def test_dowels_and_confirmat_follow_a_resize_and_edit(approval):
    # D: 600 x 720 x 560 with dowels (the default): 460 mm between the first and last, at most
    # 150 apart: 5 a joint, 115 apart. C beside it with confirmat screws.
    results = cli([plugin(approval, "cabinet", {"name": "D", "shelves": 0, "place": "origin"}),
                   select("D Side R"),
                   plugin(approval, "cabinet", {"name": "C", "shelves": 0, "joints": "confirmat", "place": "right"}),
                   MEASURE, faces("D Bottom"), faces("C Side R"), plugin(approval, "cutlist"),
                   {"command": "parameters.change", "params": {"name": "D_depth", "expression": "900 mm"}},
                   MEASURE, plugin(approval, "cutlist"),
                   select("D Top"), plugin(approval, "edit", {"joints": "none"}), MEASURE, plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "joints.sc"))
    measure = results[3]
    for part in ("Side L", "Side R", "Bottom", "Top"):
        assert cylinders(measure, f"D {part}") == 10, (part, cylinders(measure, f"D {part}"))
        assert cylinders(measure, f"C {part}") == 10, (part, cylinders(measure, f"C {part}"))
    # the dowels in the bottom's ends: 8 mm, half way through its thickness, 24 mm deep
    dowels = holes(results[4], 4)
    assert len(dowels) == 10, dowels
    ys = sorted({d[1] for d in dowels})
    assert ys == [-230, -115, 0, 115, 230], ys  # 50 mm in from each edge
    assert {d[2] for d in dowels} == {9}, dowels
    assert sorted({d[0] for d in dowels}) == [-270, 270], dowels  # 12 mm in from x = -282
    # confirmat: 7 mm through C's right side, at the bottom's and top's middles
    clear = holes(results[5], 3.5)
    assert len(clear) == 10 and {c[2] for c in clear} == {9, 711}, clear
    assert hardware(results[6]) == {"Dowel, 8 x 35 mm": 20, "Confirmat screw, 7 x 50 mm": 20}, hardware(results[6])
    # 900 deep: 7 a joint
    assert cylinders(results[8], "D Side L") == 14 and cylinders(results[8], "D Top") == 14
    assert hardware(results[9])["Dowel, 8 x 35 mm"] == 28, hardware(results[9])
    # Edit to none: no holes, no dowels to buy; C keeps its screws
    assert cylinders(results[12], "D Side L") == 0 and cylinders(results[12], "C Side L") == 10
    assert hardware(results[13]) == {"Confirmat screw, 7 x 50 mm": 20}, hardware(results[13])
    assert not [f["name"] for f in results[14]["timeline"] if f["warning"] or f["error"]]


def test_a_sink_base_and_an_old_cabinet(approval):
    # A sink base: one dowel in the middle of each rail. An old cabinet (no method remembered)
    # drilled by the Joinery entry keeps its holes through an Edit, and gets none of its own.
    results = cli([plugin(approval, "cabinet", {"name": "S", "kind": "sink", "doors": 2, "place": "origin"}),
                   plugin(approval, "cabinet", {"name": "O", "shelves": 0, "joints": "none", "place": "origin"}),
                   select("O Top"), plugin(approval, "joinery", {"type": "dowel"}),
                   select("O Top"), plugin(approval, "edit", {"shelves": 1}), MEASURE, plugin(approval, "cutlist")])
    measure = results[6]
    assert cylinders(measure, "S Rail front") == 2 and cylinders(measure, "S Rail back") == 2
    assert cylinders(measure, "S Bottom") == 10
    assert cylinders(measure, "O Bottom") == 6 and cylinders(measure, "O Side L") == 6
    assert hardware(results[7]) == {"Hinge, cup": 4, "Dowel, 8 x 35 mm": 10 + 4 + 12}, hardware(results[7])


def test_dividers_and_the_rail(approval):
    # W: 1200 wide, two dividers doweled into the bottom and the top (the inside's method), the
    # sides with confirmat. M: a drawer over a door, the rail screwed to the sides.
    results = cli([plugin(approval, "cabinet", {"name": "W", "width": 1200, "dividers": 2, "shelves": 1,
                                                "joints": "confirmat", "joints_inner": "dowels", "place": "origin"}),
                   plugin(approval, "cabinet", {"name": "M", "doors": 1, "drawers": 1, "joints": "none",
                                                "joints_inner": "confirmat", "place": "origin"}),
                   MEASURE, faces("W Divider 1"), plugin(approval, "cutlist")])
    measure = results[2]
    # shelves 542 deep (back groove): 442 between the first and last, 4 a joint; 2 ends a divider
    assert cylinders(measure, "W Divider 1") == 8, cylinders(measure, "W Divider 1")
    assert cylinders(measure, "W Bottom") == 10 + 8 and cylinders(measure, "W Top") == 10 + 8
    ends = holes(results[3], 4)
    assert len(ends) == 8 and {e[2] for e in ends} == {18 + 12, 702 - 12}, ends
    assert cylinders(measure, "M Rail") == 8 and cylinders(measure, "M Bottom") == 0
    assert hardware(results[4]) == {"Hinge, cup": 2, "Drawer slide, pair": 1, "Dowel, 8 x 35 mm": 16,
                                    "Confirmat screw, 7 x 50 mm": 20 + 8}, hardware(results[4])
