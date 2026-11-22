"""Joinery: screw and dowel holes in an existing cabinet."""
from helpers import *


def test_in_a_component_and_after_a_resize(approval):
    # Joinery: confirmat holes at the four corners of a cabinet inside a Kitchen component.
    # 560 deep: 3 screws per joint. They spread out with a resize to 900 deep.
    results = cli([{"command": "component.create", "params": {"name": "Kitchen"}},
                   plugin(approval, "cabinet", {"joints": "none", "name": "J", "shelves": 0}),
                   {"command": "select.set", "params": {"items": [{"type": "body", "name": "J Top"}]}},
                   plugin(approval, "joinery", {"type": "confirmat"}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "component.list", "params": {"compact": True}},
                   {"command": "parameters.change", "params": {"name": "J_depth", "expression": "900 mm"}},
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "joinery.sc"))
    # the cut list counts them: 3 screws in each of the 4 joints
    assert results[8]["result"]["hardware"] == [{"item": "Confirmat screw, 7 x 50 mm", "qty": 12}], results[8]["result"]["hardware"]
    assert cylinders(results[4], "J Side L") == 6 and cylinders(results[4], "J Bottom") == 6
    assert results[5]["active"] == 1, results[5]["active"]  # back in Kitchen
    assert cylinders(results[7], "J Side R") == 6 and cylinders(results[7], "J Top") == 6


def test_on_cabinets_that_have_been_moved(approval):
    # Joinery on cabinets that have been moved: placed to the right, and inside a moved room.
    # Every hole must remove material.
    # (B is as wide as A: drilled on the origin, its numbers are those of A's faces.)
    results = cli([plugin(approval, "cabinet", {"joints": "none", "name": "A", "height": 2100}),
                   select("A Top"),
                   plugin(approval, "cabinet", {"joints": "none", "name": "B", "shelves": 0, "place": "right"}),
                   select("B Top"),
                   plugin(approval, "joinery", {"type": "dowel"}),
                   {"command": "component.create", "params": {"name": "Room", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "Room:1", "translate": [2000, 500, 0]}},
                   plugin(approval, "cabinet", {"joints": "none", "name": "C", "shelf_pins": True}),
                   select("C Top"),
                   plugin(approval, "joinery", {"type": "confirmat"}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    assert not [f["name"] for f in results[11]["timeline"] if f["warning"]]
    for cabinet in "BC":
        for part in ("Side L", "Side R", "Bottom", "Top"):
            pins = 36 if cabinet == "C" and part.startswith("Side") else 0
            assert cylinders(results[10], f"{cabinet} {part}") == 6 + pins, (cabinet, part)
