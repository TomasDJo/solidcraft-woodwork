"""The L-shaped corner cabinet and its bi-fold doors."""
from helpers import *

CORNER = {"name": "C", "width": 1000, "return": 900, "plinth": 100, "doors": "hinged right"}


def test_l_shape_doors_and_mirror(approval):
    # Corner cabinet: an L of 1000 x 900 with the return on the left, a box cabinet placed to
    # its right, the bi-fold doors opened. Then the mirror image.
    corner = CORNER
    results = cli([plugin(approval, "corner", corner),
                   {"command": "inspect.measure", "params": {}},
                   select("C Top"),
                   plugin(approval, "cabinet", {"name": "N", "place": "right"}),
                   drive("C Door hinge", 60), drive("C Door return fold", 90),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "corner.sc"))
    box = boxes(results[1])
    assert len(box) == 11, sorted(box)  # 2 sides, 2 backs, bottom, top, shelf, 2 plinth boards, 2 doors
    assert box["C Side L"] == [-500, -450, 0, 60, -432, 720], box["C Side L"]  # the end of the return
    assert box["C Side R"] == [482, -110, 0, 500, 450, 720], box["C Side R"]
    # the first door runs into the inner corner; the second stops 2 mm short of its face
    assert box["C Door"] == [62, -130, 102, 498, -112, 718], box["C Door"]
    assert box["C Door return"] == [62, -448, 102, 80, -132, 718], box["C Door return"]
    # an L-shaped board: 964 x 864 less the open corner of 422 x 322
    bottom = next(b for b in results[1]["bodies"] if b["name"] == "C Bottom")
    assert round(bottom["volume_mm3"]) == (964 * 864 - 422 * 322) * 18, bottom["volume_mm3"]
    box = boxes(results[6])
    assert box["N Side L"][0] == 500 and box["N Side L"][4] == 450, box["N Side L"]  # beside it, backs flush
    # a bi-fold pair: the door swings out, and the return door hangs on it, folded onto its face
    assert box["C Door"][1] < -400, box["C Door"]
    assert box["C Door return"] == [290, -499.267, 102, 463.588, -216.603, 718], box["C Door return"]
    # the fold alone, with the first door shut: out of the cabinet, onto the first door's face
    for side, flat in (("left", [82, -148, 102, 398, -130, 718]), ("right", [-398, -148, 102, -82, -130, 718])):
        doors = "hinged right" if side == "left" else "hinged left"
        alone = cli([plugin(approval, "corner", {**corner, "corner": side, "doors": doors}),
                     drive("C Door return fold", 90), {"command": "inspect.measure", "params": {}}])
        assert boxes(alone[2])["C Door return"] == flat, (side, boxes(alone[2])["C Door return"])
    assert '3,964,864,18,"Birch plywood","C Bottom, C Top, C Shelf 1"' in results[7]["result"]["csv"]
    assert not [f["name"] for f in results[8]["timeline"] if f["warning"]]
    results = cli([plugin(approval, "corner", {**corner, "corner": "right", "doors": "hinged left"}),
                   drive("C Door hinge", 60), drive("C Door return fold", 90),
                   {"command": "inspect.measure", "params": {}}])
    mirror = boxes(results[3])
    for part in ("C Door", "C Door return", "C Bottom", "C Plinth"):
        x0, y0, z0, x1, y1, z1 = box[part]
        assert mirror[part] == [-x1, y0, z0, -x0, y1, z1], (part, mirror[part], box[part])
    assert mirror["C Side R"] == [-60, -450, 0, 500, -432, 720], mirror["C Side R"]


def test_hinged_on_the_return(approval):
    corner = CORNER
    # Hinged on the return instead, with a pull on that door and shelf pins: the return door
    # swings out and the back leg's door hangs on it, folded out in line.
    results = cli([plugin(approval, "corner", {**corner, "doors": "hinged left", "handles": "pull",
                                               "shelf_pins": True}),
                   {"command": "inspect.measure", "params": {}},
                   drive("C Door return hinge", 60), drive("C Door fold", 90),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    shut = boxes(results[1])
    assert shut["C Door"] == [82, -130, 102, 498, -112, 718], shut["C Door"]
    assert shut["C Door return"] == [62, -448, 102, 80, -112, 718], shut["C Door return"]
    # 2 hinge cups each; 2 screw holes on the return door only. 15 pin holes in each of 2 rows
    assert cylinders(results[1], "C Door return") == 4 and cylinders(results[1], "C Door") == 2
    for part in ("Side L", "Side R", "Back", "Back return"):
        assert cylinders(results[1], f"C {part}") == 30, (part, cylinders(results[1], f"C {part}"))
    swung = boxes(results[4])
    assert swung["C Door return"][3] > 300, swung["C Door return"]
    assert swung["C Door"] == [-6.603, -513.588, 102, 362.664, -290, 718], swung["C Door"]
    hardware = {h["item"]: h["qty"] for h in results[5]["result"]["hardware"]}
    assert hardware == {"Hinge, 35 mm cup": 4, "Shelf pin, 5 mm": 8, "Pull, 128 mm": 1}, hardware
    assert not [f["name"] for f in results[6]["timeline"] if f["warning"]]
