"""Appliance cabinets: a sink base (rails instead of a top)."""
from helpers import *


def test_a_sink_base(approval):
    # 600 x 720 x 560, a grooved back (12 mm from the rear, 6 thick: the shelves' depth ends at
    # y = 280 - 18 = 262). No top: a front rail flush at the front, a back rail in front of the
    # back, 100 deep, their top faces at 720. Two doors that turn; joinery into the rails.
    results = cli([plugin(approval, "cabinet", {"name": "S", "kind": "sink", "doors": 2, "shelves": 1}), MEASURE,
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
    assert cylinders(results[5], "S Bottom") == 6 and cylinders(results[5], "S Side L") == 3 + 2
    # Edit keeps it a sink base, and drills the rails again
    box = boxes(results[8])
    assert "S Top" not in box and "S Door" in box and "S Rail back" in box, sorted(box)
    assert cylinders(results[8], "S Rail back") == 2
    assert results[9]["metadata"]["io.github.tomasdjo.woodwork"]["params"]["kind"] == "sink"
    # the cut list counts the rails' screws too: 2 x 3 into the bottom, 2 x 2 into the rails
    hardware = {h["item"]: h["qty"] for h in results[10]["result"]["hardware"]}
    assert hardware["Confirmat screw, 7 x 50 mm"] == 6 + 4, hardware


def test_a_worktop_over_a_run_with_a_sink_base(approval):
    # A standard cabinet and a sink base right of it; one worktop over both; the 900 wide sink
    # base ends at x = 1200.
    results = cli([plugin(approval, "cabinet", {"name": "A"}), select("A Side R"),
                   plugin(approval, "cabinet", {"name": "S", "kind": "sink", "width": 900, "doors": 2, "place": "auto"}),
                   pick("A Top", "S Rail front"), plugin(approval, "worktop"),
                   MEASURE])
    box = boxes(results[5])
    assert box["S Side L"][0] == 300 and box["S Side R"][3] == 1200, (box["S Side L"], box["S Side R"])
    assert box["Worktop"] == [-300, -320, 720, 1200, 280, 758], box["Worktop"]
