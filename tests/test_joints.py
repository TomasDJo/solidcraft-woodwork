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
    # 250 apart: 3 a joint, 230 apart. C beside it with confirmat screws.
    results = designed(approval, {}, [plugin(approval, "cabinet", {"name": "D", "shelves": 0, "place": "origin"}),
                   select("D Side R"), setting(approval, {"joints": "confirmat"}),
                   plugin(approval, "cabinet", {"name": "C", "shelves": 0, "place": "right"}),
                   MEASURE, faces("D Bottom"), faces("C Side R"), plugin(approval, "cutlist"),
                   {"command": "parameters.change", "params": {"name": "D_depth", "expression": "900 mm"}},
                   MEASURE, plugin(approval, "cutlist"),
                   select("D Top"), plugin(approval, "edit", {"joints": "none"}), MEASURE, plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "joints.sc"))
    measure = results[3]
    for part in ("Side L", "Side R", "Bottom", "Top"):
        assert cylinders(measure, f"D {part}") == 6, (part, cylinders(measure, f"D {part}"))
        assert cylinders(measure, f"C {part}") == 6, (part, cylinders(measure, f"C {part}"))
    # the dowels in the bottom's ends: 8 mm, half way through its thickness, 24 mm deep
    dowels = holes(results[4], 4)
    assert len(dowels) == 6, dowels
    ys = sorted({d[1] for d in dowels})
    assert ys == [-230, 0, 230], ys  # 50 mm in from each edge
    assert {d[2] for d in dowels} == {9}, dowels
    assert sorted({d[0] for d in dowels}) == [-270, 270], dowels  # 12 mm in from x = -282
    # confirmat: 7 mm through C's right side, at the bottom's and top's middles
    clear = holes(results[5], 3.5)
    assert len(clear) == 6 and {c[2] for c in clear} == {9, 711}, clear
    assert hardware(results[6]) == {"Dowel, 8 x 35 mm": 12, "Confirmat screw, 7 x 50 mm": 12}, hardware(results[6])
    # 900 deep: 5 a joint
    assert cylinders(results[8], "D Side L") == 10 and cylinders(results[8], "D Top") == 10
    assert hardware(results[9])["Dowel, 8 x 35 mm"] == 20, hardware(results[9])
    # Edit to none: no holes, no dowels to buy; C keeps its screws
    assert cylinders(results[12], "D Side L") == 0 and cylinders(results[12], "C Side L") == 6
    assert hardware(results[13]) == {"Confirmat screw, 7 x 50 mm": 12}, hardware(results[13])
    assert not [f["name"] for f in results[14]["timeline"] if f["warning"] or f["error"]]


def test_a_sink_base_and_joinery_by_edit(approval):
    # A sink base: one dowel in the middle of each rail. A cabinet made without joinery gets it
    # by Edit, and keeps it through the next Edit (its shelf unjoined: none inside).
    results = designed(approval, {}, [plugin(approval, "cabinet", {"name": "S", "kind": "sink", "doors": 2, "place": "origin"}),
                   setting(approval, NONE), plugin(approval, "cabinet", {"name": "O", "shelves": 0, "place": "origin"}),
                   select("O Top"), plugin(approval, "edit", {"joints": "dowels"}),
                   select("O Top"), plugin(approval, "edit", {"shelves": 1}), MEASURE, plugin(approval, "cutlist")])
    measure = results[6]
    assert cylinders(measure, "S Rail front") == 2 and cylinders(measure, "S Rail back") == 2
    assert cylinders(measure, "S Bottom") == 6
    assert cylinders(measure, "O Bottom") == 6 and cylinders(measure, "O Side L") == 6
    assert hardware(results[7]) == {"Hinge, cup": 4, "Dowel, 8 x 35 mm": 6 + 4 + 12}, hardware(results[7])


def test_dividers_and_the_rail(approval):
    # W: 1200 wide, two dividers doweled into the bottom and the top (the inside's method), the
    # sides with confirmat. M: a drawer over a door, the rail screwed to the sides.
    results = designed(approval, {"joints": "confirmat", "joints_inner": "dowels"},
                       [plugin(approval, "cabinet", {"name": "W", "width": 1200, "dividers": 2, "shelves": 0, "place": "origin"}),
                        setting(approval, {"joints": "none", "joints_inner": "confirmat"}),
                        plugin(approval, "cabinet", {"name": "M", "doors": 1, "drawers": 1, "shelves": 0, "place": "origin"}),
                   MEASURE, faces("W Divider 1"), plugin(approval, "cutlist")])
    measure = results[2]
    # shelves 542 deep (back groove): 442 between the first and last, 3 a joint; 2 ends a divider
    assert cylinders(measure, "W Divider 1") == 6, cylinders(measure, "W Divider 1")
    assert cylinders(measure, "W Bottom") == 6 + 6 and cylinders(measure, "W Top") == 6 + 6
    ends = holes(results[3], 4)
    assert len(ends) == 6 and {e[2] for e in ends} == {18 + 12, 702 - 12}, ends
    assert cylinders(measure, "M Rail") == 6 and cylinders(measure, "M Bottom") == 0
    assert hardware(results[4]) == {"Hinge, cup": 2, "Drawer slide, full extension, 500 mm, pair": 1, "Dowel, 8 x 35 mm": 12,
                                    "Confirmat screw, 7 x 50 mm": 12 + 6}, hardware(results[4])


def test_corner_cabinets(approval):
    # C: a left L, 900 x 900, 560 deep, 720 high, doweled. Upright joints 720 long: 4 dowels;
    # the ends' 542: 3; along the backs 864: 5.
    results = designed(approval, {}, [plugin(approval, "corner", {"name": "C", "shelves": 0, "place": "origin"}),
                   plugin(approval, "corner", {"name": "S", "shelves": 0, "boards": "split", "place": "origin"}),
                   setting(approval, {"joints": "confirmat"}),
                   plugin(approval, "corner", {"name": "D", "shelves": 1, "shape": "diagonal", "corner": "right",
                                               "wall_mount": "rail", "place": "origin"}),
                   MEASURE, plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}},
                   select("C Top"), plugin(approval, "edit", {"joints": "none"}), MEASURE])
    measure = results[3]
    counts = {part: cylinders(measure, f"C {part}") for part in ("Side R", "Back", "Back return", "Side L", "Bottom", "Top")}
    assert counts == {"Side R": 10, "Back": 18, "Back return": 18, "Side L": 10, "Bottom": 16, "Top": 16}, counts
    # split: the return's piece takes the return's end and the front of the back return, and
    # keeps its own two dowels into the back leg's board
    assert cylinders(measure, "S Bottom return") == 3 + 2 + 2, cylinders(measure, "S Bottom return")
    # a right diagonal on a hanging rail: the backs stand on the bottom, screwed up into
    assert cylinders(measure, "D Back") > 0 and cylinders(measure, "D Bottom") > 0
    hardware_ = hardware(results[4])
    assert hardware_["Dowel, 8 x 35 mm"] == 44 + 44 and hardware_["Confirmat screw, 7 x 50 mm"] > 0, hardware_
    assert not [f["name"] for f in results[5]["timeline"] if f["warning"] or f["error"]]
    assert cylinders(results[8], "C Back") == 0


def test_screws(approval):
    # 4.5 mm through the side, a 3 mm pilot into the end: 3 a joint at 560 deep
    results = designed(approval, {"joints": "screws"}, [plugin(approval, "cabinet", {"name": "P", "shelves": 0, "place": "origin"}),
                   MEASURE, faces("P Side L"), faces("P Bottom"), plugin(approval, "cutlist")])
    assert cylinders(results[1], "P Side L") == 6 and cylinders(results[1], "P Top") == 6
    clear = holes(results[2], 2.25)
    assert len(clear) == 6 and {c[0] for c in clear} == {-291}, clear
    assert len(holes(results[3], 1.5)) == 6
    assert hardware(results[4]) == {"Screw, 4 x 50 mm": 12}, hardware(results[4])


def test_dado(approval):
    # The bottom and top 6 mm longer at each end, in grooves in the sides; a divider into the
    # bottom and top. The cut list gives the longer boards and no hardware.
    results = designed(approval, {"joints": "dado", "joints_inner": "dado"},
                       [plugin(approval, "cabinet", {"name": "G", "shelves": 0, "dividers": 1, "place": "origin"}),
                   MEASURE, plugin(approval, "cutlist")])
    box = boxes(results[1])
    assert box["G Bottom"][0] == -288 and box["G Bottom"][3] == 288, box["G Bottom"]
    assert box["G Divider 1"][2] == 12 and box["G Divider 1"][5] == 708, box["G Divider 1"]
    # the side lost the two grooves, 6 x 18 x 560 each, and the back's 9 x 6 x 690
    side = volume(results[1], "G Side L")
    assert abs(side - (18 * 560 * 720 - 2 * 6 * 18 * 560 - 9 * 6 * 690)) < 100, side
    assert cylinders(results[1], "G Side L") == 0
    assert "576" in results[2]["result"]["csv"], results[2]["result"]["csv"][:400]
    assert hardware(results[2]) == {}, hardware(results[2])


def test_biscuits(approval):
    # #20 slots, 62 x 4 mm, 13 mm into each board: 3 a joint, 6 in each side; N without any.
    # A corner cabinet with Dominos.
    results = designed(approval, {"joints": "biscuit"}, [plugin(approval, "cabinet", {"name": "B", "shelves": 0, "place": "origin"}),
                   setting(approval, {"joints": "none"}),
                   plugin(approval, "cabinet", {"name": "N", "shelves": 0, "place": "origin"}),
                   setting(approval, {"joints": "biscuit", "biscuit": "domino"}),
                   plugin(approval, "corner", {"name": "C", "shelves": 0, "place": "origin"}),
                   MEASURE, plugin(approval, "cutlist")])
    slot = 62 * 4 * 13
    assert abs(volume(results[3], "N Side L") - volume(results[3], "B Side L") - 6 * slot) < 50
    assert abs(volume(results[3], "N Bottom") - volume(results[3], "B Bottom") - 6 * slot) < 50
    listed = hardware(results[4])
    assert listed["Biscuit, #20"] == 12 and listed["Domino, 5 x 30 mm"] == 44, listed


def test_cam_locks_and_pocket_screws(approval):
    # K: cam locks, 3 a joint: the bolt's thread in the side, its shank and the 15 mm cam in the
    # bottom, the cam from the bottom's top face. P: pocket screws from under the bottom, at 15
    # degrees, out of the middle of its end into the side.
    results = designed(approval, {"joints": "cam"}, [plugin(approval, "cabinet", {"name": "K", "shelves": 0, "place": "origin"}),
                   setting(approval, {"joints": "pocket"}),
                   plugin(approval, "cabinet", {"name": "P", "shelves": 0, "place": "origin"}),
                   MEASURE, faces("K Bottom"), faces("P Side L"), plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    measure = results[2]
    assert cylinders(measure, "K Side L") == 6 and cylinders(measure, "K Bottom") >= 12
    cams = holes(results[3], 7.5)
    # 24 mm in from the side (35 mm bolt less 11 mm of thread), half way down from the top face
    assert len(cams) == 6 and all(abs(abs(c[0]) - 258) < 1 for c in cams), cams
    assert all(9 < c[2] < 18 for c in cams), cams
    assert cylinders(measure, "P Bottom") == 12 and cylinders(measure, "P Side L") == 6
    tips = holes(results[4], 1.75)
    # into the side, rising (falling at the top) from the boards' middles at 9 and 711
    assert len(tips) == 6 and all(-300 < t[0] < -282 and (9 < t[2] < 14 or 706 < t[2] < 711) for t in tips), tips
    listed = hardware(results[5])
    assert listed == {"Cam lock and bolt, 15 mm": 12, "Pocket screw, 32 mm": 12}, listed
    assert not [f["name"] for f in results[6]["timeline"] if f["warning"] or f["error"]]


def test_without_pins_the_shelves_are_fixed_and_joined(approval):
    # F: no shelf pins, so two compartments of two fixed shelves, doweled (the inside's method)
    # to the sides and the divider: 3 a joint, the shelves right of the divider 2, between the
    # others' holes. G: a fixed shelf in a dado, 6 mm longer at each end.
    results = designed(approval, {"joints": "none"},
                       [plugin(approval, "cabinet", {"name": "F", "width": 1200, "dividers": 1, "shelves": 2, "place": "origin"}),
                        setting(approval, {"joints": "dado", "joints_inner": "dado"}),
                        plugin(approval, "cabinet", {"name": "G", "shelves": 1, "place": "origin"}),
                   MEASURE, faces("F Divider 1"), plugin(approval, "cutlist"),
                   select("F Top"), plugin(approval, "edit", {"shelf_pins": "yes"}), MEASURE])
    measure = results[2]
    assert cylinders(measure, "F Side L") == 6 and cylinders(measure, "F Side R") == 6, cylinders(measure, "F Side L")
    assert cylinders(measure, "F Divider 1") == 6 + 6 + 4, cylinders(measure, "F Divider 1")
    assert cylinders(measure, "F Shelf 2.1") == 5 and cylinders(measure, "F Shelf 1.2") == 6
    # in the divider, the two sides' holes at a shelf do not meet: different depths along it
    ys = sorted({h[1] for h in holes(results[3], 4) if 100 < h[2] < 700})
    assert len(ys) == 5, ys
    assert hardware(results[4]) == {"Dowel, 8 x 35 mm": 6 + 12 + 10}, hardware(results[4])
    box = boxes(measure)
    assert box["G Shelf 1"][0] == -288 and box["G Shelf 1"][3] == 288, box["G Shelf 1"]
    # Edit to shelf pins: adjustable, no joints, pin rows
    assert cylinders(results[7], "F Shelf 1.1") == 0 and cylinders(results[7], "F Side L") > 6, (cylinders(results[7], "F Shelf 1.1"), cylinders(results[7], "F Side L"))
