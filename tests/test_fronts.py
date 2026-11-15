"""Drawers, handle holes, and the joints doors and drawers move on."""
from helpers import *


def test_drawers(approval):
    # Drawers: 3 in a 600 x 720 cabinet. The boxes clear the sides by 12.7 mm and follow a resize.
    results = cli([plugin(approval, "cabinet", {"drawers": 3}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "parameters.change", "params": {"name": "Cabinet_width", "expression": "800 mm"}},
                   {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "drawers.sc"))
    box = {b["name"]: [round(v, 4) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[1]["bodies"]}
    assert len(box) == 5 + 3 * 6, sorted(box)
    assert box["Cabinet Drawer 1 Left"][0] == -300 + 18 + 12.7, box["Cabinet Drawer 1 Left"]
    assert box["Cabinet Drawer 1 Right"][3] == 300 - 18 - 12.7, box["Cabinet Drawer 1 Right"]
    front = box["Cabinet Drawer 2 Front"]  # (720 - 4 gaps of 2) / 3 high
    assert round(front[5] - front[2], 3) == round(712 / 3, 3), front
    assert box["Cabinet Drawer 1 Bottom"][4] <= 280 - 12 - 6, box["Cabinet Drawer 1 Bottom"]  # clear of the back
    box = {b["name"]: [round(v, 4) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[3]["bodies"]}
    assert box["Cabinet Drawer 1 Right"][3] == 400 - 18 - 12.7, box["Cabinet Drawer 1 Right"]


def test_handles(approval):
    # Handles: a pull is two holes through each front, next to a door's two hinge cups.
    results = cli([plugin(approval, "cabinet", {"name": "H", "doors": 2, "plinth": 100, "handles": "pull"}),
                   plugin(approval, "cabinet", {"name": "K", "drawers": 2, "handles": "knob"}),
                   {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "handles.sc"))
    assert cylinders(results[2], "H Door L") == 4 and cylinders(results[2], "K Drawer 2 Front") == 1


def test_handle_position(approval):
    # The door stands from z = 2 to 718. A pull's two holes are 128 mm apart, its middle 164 mm
    # from the end; a knob is 100 mm from it.
    for at, pull in (("top", [490, 618]), ("middle", [296, 424]), ("bottom", [102, 230]), ("auto", [490, 618])):
        results = cli([plugin(approval, "cabinet", {"doors": 1, "handles": "pull", "handle_position": at}),
                       faces("Cabinet Door")])
        assert handle_heights(results[1]) == pull, (at, handle_heights(results[1]))
    # Both corner shapes, with a knob: on the L it is on the door that folds.
    for shape, door in (("L-shaped", "Cabinet Door return"), ("diagonal", "Cabinet Door")):
        for at, knob in (("top", [618]), ("middle", [360]), ("bottom", [102])):
            results = cli([plugin(approval, "corner", {"shape": shape, "doors": "hinged right", "handles": "knob",
                                                       "handle_position": at}),
                           faces(door)])
            assert handle_heights(results[1]) == knob, (shape, at, handle_heights(results[1]))


def test_joints(approval):
    # Joints: doors swing out on their hinges and a drawer slides out, in an inch design to
    # check the joint's point is not read in inches. The cabinet still moves as one piece.
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"name": "D", "doors": 2}),
                   plugin(approval, "cabinet", {"name": "E", "drawers": 2}),
                   drive("D Door L hinge", 90), drive("D Door R hinge", 200), drive("E Drawer 1 slide", 300),
                   {"command": "occurrence.ground", "params": {"occurrence": "D:1", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "D:1", "translate": [0, 0, 1000]}},
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "joints.sc"))
    # the cut list measures the doors as boards, however far they stand open
    # (716 x 297 x 18 mm, listed in inches in this inch design)
    assert '2,28 3/16,11 11/16,23/32,"Birch plywood","D Door L, D Door R"' in results[9]["result"]["csv"]
    box = {b["name"]: [round(v, 3) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[8]["bodies"]}
    # left door: turned 90 degrees about its front left edge (x -298, y -300), now along -Y and
    # still inside the cabinet's width, clear of a door on the next cabinet
    assert box["D Door L"] == [-298, -597, 1002, -280, -300, 1718], box["D Door L"]
    # right door: stopped by its limit at 110 degrees
    assert box["D Door R"][1] < -300 - 297 * 0.9 and box["D Door R"][3] > 298, box["D Door R"]
    assert box["E Drawer 1 Front"][1] == -300 - 300, box["E Drawer 1 Front"]
    assert box["E Drawer 2 Front"][1] == -300, box["E Drawer 2 Front"]
