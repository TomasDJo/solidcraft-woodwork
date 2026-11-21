"""The diagonal corner cabinet: one front and one door at 45 degrees."""
from helpers import *

CORNER = {"name": "C", "width": 1000, "return": 900, "plinth": 100, "doors": "hinged right"}


def test_diagonal_door_resize_and_mirror(approval):
    # Diagonal corner cabinet, in an inch design: boards cut at 45 degrees, one door and the
    # plinth standing at 45 degrees. Then wider, and the mirror image.
    corner = CORNER
    diagonal = {**corner, "shape": "diagonal", "handles": "knob", "shelf_pins": True}
    measure = {"command": "inspect.measure", "params": {}}
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "corner", diagonal), measure,
                   plugin(approval, "cutlist"),
                   drive("C Door hinge", 90), measure,
                   {"command": "parameters.change", "params": {"changes": {"C_width": "1100 mm"}}}, measure,
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "diagonal.sc"))
    box = boxes(results[2])
    assert len(box) == 9, sorted(box)  # 2 sides, 2 backs, bottom, top, shelf, plinth, door
    assert box["C Side L"] == [-500, -450, 0, 160, -432, 720], box["C Side L"]  # 660 deep at the return's end
    # 964 x 864 less a corner of 304 x 304, cut across
    for part in ("C Bottom", "C Top", "C Shelf 1"):
        assert volume(results[2], part) == (964 * 864 - 304 * 304 / 2) * 18, (part, volume(results[2], part))
    # from the return's end to the back leg's end, 2 mm in front of the carcass
    door = [round(v, 1) for v in box["C Door"]]
    assert door == [177.0, -448.6, 102, 498.6, -127.0, 718], door  # inside the footprint: x <= 500, y >= -450
    csv = results[3]["result"]["csv"]
    assert '1,24 1/4,17 3/16,23/32,"Birch plywood",yes,L2 W2,"C Door"' in csv, csv
    assert '1,18 15/16,3 15/16,23/32,"Birch plywood",yes,,"C Plinth"' in csv, csv
    hardware = {h["item"]: h["qty"] for h in results[3]["result"]["hardware"]}
    assert hardware == {"Hinge, 35 mm cup": 2, "Shelf pin, 5 mm": 8, "Knob": 1}, hardware
    door = [round(v, 1) for v in boxes(results[5])["C Door"]]
    assert door == [485.9, -461.3, 102, 807.5, -139.7, 718], door  # swung out square to the front
    assert volume(results[7], "C Bottom") == (1064 * 864 - 304 * 304 / 2) * 18, volume(results[7], "C Bottom")
    assert boxes(results[7])["C Side L"][3] == 210, boxes(results[7])["C Side L"]  # 760 deep now
    assert not [f["name"] for f in results[8]["timeline"] if f["warning"]]
    results = cli([plugin(approval, "corner", {**diagonal, "corner": "right", "doors": "hinged right"}), measure,
                   drive("C Door hinge", 90), measure])
    mirror = boxes(results[1])
    for part in ("C Door", "C Bottom", "C Plinth", "C Side R"):
        x0, y0, z0, x1, y1, z1 = box[part.replace("Side R", "Side L")]
        assert [round(v, 3) for v in mirror[part]] == [-x1, y0, z0, -x0, y1, z1], (part, mirror[part])
    door = [round(v, 1) for v in boxes(results[3])["C Door"]]
    assert door == [-511.3, -757.5, 102, -189.7, -435.9, 718], door  # hinged at the return's end


def test_inset_diagonal_door(approval):
    # Inset, in an inch design: the door's face on the front's line, between the two ends (which
    # stand at 45 degrees to it, so its edges keep the reveal from their inner corners, 12.7 mm
    # along it), from 120 to 700 between bottom and top. The shelf's cut edge a panel back.
    diagonal = {**CORNER, "shape": "diagonal", "fronts": "inset", "handles": "knob"}
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "corner", diagonal), MEASURE,
                   drive("C Door hinge", 90), MEASURE,
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "diagonal-inset.sc"))
    door = [round(v, 1) for v in boxes(results[2])["C Door"]]
    assert door == [157.7, -439.6, 120, 489.6, -107.7, 700], door
    assert cylinders(results[2], "C Door") == 3, cylinders(results[2], "C Door")  # 2 cups, a knob
    assert volume(results[2], "C Bottom") == (964 * 864 - 304 * 304 / 2) * 18, volume(results[2], "C Bottom")
    cut = 340 - 36 + 18 * 2 ** 0.5
    assert abs(volume(results[2], "C Shelf 1") - (964 * 864 - cut * cut / 2) * 18) < 2, volume(results[2], "C Shelf 1")
    # swung square to the front about its front edge, the back edge clear of the end panel
    door = [round(v, 1) for v in boxes(results[4])["C Door"]]
    assert door == [476.9, -452.3, 120, 808.8, -120.4, 700], door
    assert not [f["name"] for f in results[5]["timeline"] if f["warning"]]
