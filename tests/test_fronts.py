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
    # the lowest box clears the cabinet's bottom and the highest its top, by 10 mm
    assert box["Cabinet Drawer 1 Left"][2] == box["Cabinet Bottom"][5] + 10, box["Cabinet Drawer 1 Left"]
    assert box["Cabinet Drawer 3 Left"][5] == box["Cabinet Top"][2] - 10, box["Cabinet Drawer 3 Left"]
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


def test_hinges_follow_a_resize(approval):
    # A cabinet made small and then widened and deepened opens its doors exactly as one made
    # at that size does: the hinge is on the door's edge, not at a point from when it was made.
    change = lambda name, to: {"command": "parameters.change", "params": {"name": name, "expression": to}}
    cases = [("cabinet", {"doors": 2}, {"width": 600, "depth": 560}, {"width": 900, "depth": 400},
              {"Cabinet Door L hinge": 90, "Cabinet Door R hinge": 60}),
             ("corner", {"doors": "hinged right"}, {"width": 900, "return": 900}, {"width": 1100, "return": 1000},
              {"Cabinet Door hinge": 70, "Cabinet Door return fold": 80}),
             ("corner", {"doors": "hinged right", "corner": "right"}, {"width": 900, "return": 900}, {"width": 1100, "return": 1000},
              {"Cabinet Door return hinge": 70, "Cabinet Door fold": 80}),
             ("corner", {"shape": "diagonal", "doors": "hinged left"}, {"width": 900, "depth": 560}, {"width": 900, "depth": 400},
              {"Cabinet Door hinge": 60})]
    for entry, params, small, large, joints in cases:
        drives = [drive(joint, value) for joint, value in joints.items()]
        resized = cli([plugin(approval, entry, {**params, **small}),
                       *[change(f"Cabinet_{key}", f"{value} mm") for key, value in large.items()], *drives, MEASURE])[-1]
        made = cli([plugin(approval, entry, {**params, **large}), *drives, MEASURE])[-1]
        doors = lambda measure: {name: box for name, box in boxes(measure).items() if " Door" in name}
        assert doors(resized) == doors(made) and doors(made), (entry, params, doors(resized), doors(made))


def test_labels(approval):
    # Each flat board gets its name on it, as a sketch that survives a resize; the drawer
    # boxes' thin boards and a cabinet made without the field get none.
    inspect = {"command": "document.inspect", "params": {"bodies": False, "timeline": False}}
    change = {"command": "parameters.change", "params": {"changes": {"Cabinet_width": "900 mm", "Cabinet_depth": "400 mm"}}}
    for entry, params, wanted in (("cabinet", {"doors": 2, "plinth": 100}, 9), ("corner", {"doors": "hinged right"}, 9),
                                  ("corner", {"shape": "diagonal", "doors": "hinged left", "plinth": 100}, 9)):
        results = cli([plugin(approval, entry, {**params, "labels": True}), MEASURE, change, inspect, plugin(approval, "cutlist")])
        sketches = {s["name"]: s["status"] for s in results[3]["sketches"]}
        bodies = [b["name"] for b in results[1]["bodies"]]
        assert sorted(sketches) == sorted(f"{body} label" for body in bodies), (entry, sorted(sketches), bodies)
        assert set(sketches.values()) == {"solved"} and len(sketches) == wanted, (entry, sketches)
        assert results[4]["result"]["boards"] == len(bodies)  # the cut list counts boards, not labels
    results = cli([plugin(approval, "cabinet", {"doors": 2}), inspect])
    assert results[1]["sketches"] == [], results[1]["sketches"]
