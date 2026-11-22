"""Appliance cabinets: a sink base (rails instead of a top), with doors or with drawers."""
from helpers import *


def test_a_sink_base(approval):
    # 600 x 720 x 560, a grooved back (12 mm from the rear, 6 thick: the shelves' depth ends at
    # y = 280 - 18 = 262). No top: a front rail flush at the front, a back rail in front of the
    # back, 100 deep, their top faces at 720. Two doors that turn; joinery into the rails.
    results = cli([plugin(approval, "cabinet", {"joints": "none", "joints_inner": "none", "name": "S", "kind": "sink", "doors": 2, "shelves": 1}), MEASURE,
                   drive("S Door L hinge", 90), select("S Side L"), plugin(approval, "joinery", {"type": "confirmat"}),
                   MEASURE, select("S Side L"), plugin(approval, "edit", {"kind": "keep", "doors": 1}), MEASURE,
                   {"command": "component.metadata", "params": {"component": "S"}}, plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "sink.sc"))
    box = boxes(results[1])
    assert "S Top" not in box and "S Shelf 1" not in box, sorted(box)
    assert box["S Rail front"] == [-282, -280, 702, 282, -180, 720], box["S Rail front"]
    assert box["S Rail back"] == [-282, 162, 702, 282, 262, 720], box["S Rail back"]
    assert box["S Door L"][5] == 718, box["S Door L"]
    moved = boxes(results[5])
    assert moved["S Door L"][1] < -500, moved["S Door L"]  # turned out on its hinge
    # 3 screws a joint at the bottom, the first and last of them at the top, one in each rail
    assert cylinders(results[5], "S Rail front") == 2 and cylinders(results[5], "S Rail back") == 2
    # (and the hinge plate's 4 holes in the side)
    assert cylinders(results[5], "S Bottom") == 6 and cylinders(results[5], "S Side L") == 3 + 2 + 4
    # Edit keeps it a sink base, and drills the rails again
    box = boxes(results[8])
    assert "S Top" not in box and "S Door" in box and "S Rail back" in box, sorted(box)
    assert cylinders(results[8], "S Rail back") == 2
    assert results[9]["metadata"]["io.github.tomasdjo.woodwork"]["params"]["kind"] == "sink"
    # the cut list counts the rails' screws too: 2 x 3 into the bottom, 2 x 2 into the rails
    hardware = {h["item"]: h["qty"] for h in results[10]["result"]["hardware"]}
    assert hardware["Confirmat screw, 7 x 50 mm"] == 6 + 4, hardware


def test_a_sink_base_with_drawers(approval):
    # 600 x 720 x 560, three fronts of (720 - 4 x 2) / 3: two drawers and on top a false front,
    # fixed in front of the sink (480.667 to 718), with a knob but no box. The boxes are half the
    # shelves' depth (542 / 2 = 271) from behind the fronts (y = -282), clear of the trap. An end
    # panel comes forward to the fronts' faces. Edit to standard makes three full drawers under a
    # top, and back to a sink base the false front again.
    results = cli([plugin(approval, "cabinet", {"name": "S", "kind": "sink", "drawers": 3, "handles": "knob"}),
                   select("S Side R"), plugin(approval, "panel", {"kind": "end"}), MEASURE,
                   drive("S Drawer 1 slide", 200), MEASURE, plugin(approval, "cutlist"),
                   select("S Side L"), plugin(approval, "edit", {"kind": "standard"}), MEASURE,
                   select("S Side L"), plugin(approval, "edit", {"kind": "sink"}), MEASURE],
                  "--save", str(ROOT / "dist" / "sink-drawers.sc"))
    box = boxes(results[3])
    assert box["S False front"] == [-298, -300, 480.667, 298, -282, 718], box["S False front"]
    assert "S Drawer 3 Front" not in box and "S Drawer 2 Front" in box and "S Top" not in box, sorted(box)
    assert box["S Drawer 1 Left"][1:5:3] == [-282, -11], box["S Drawer 1 Left"]
    assert box["S End panel R"][1] == -300, box["S End panel R"]
    assert cylinders(results[3], "S False front") == 1
    moved = boxes(results[5])
    assert moved["S Drawer 1 Front"][1] == -500 and moved["S False front"] == box["S False front"], moved["S Drawer 1 Front"]
    hardware = {h["item"]: h["qty"] for h in results[6]["result"]["hardware"]}
    assert hardware["Drawer slide, pair"] == 2, hardware
    box = boxes(results[9])
    assert "S Top" in box and "S Drawer 3 Front" in box and "S False front" not in box, sorted(box)
    assert box["S Drawer 1 Left"][4] == 250 and box["S End panel R"][1] == -300, box["S Drawer 1 Left"]
    box = boxes(results[12])
    assert "S False front" in box and "S Top" not in box and box["S Drawer 1 Left"][4] == -11, sorted(box)
