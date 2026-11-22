"""Hinges and drawer slides: their holes, every size from the shared HW_ parameters."""
from helpers import *


def holes(faces, radius):
    """The middles of a body's round faces of that radius, from `model.faces`: x, y, z rounded."""
    found = [f for f in faces["faces"] if abs((f.get("cylinder") or {}).get("radius", 0) - radius) < 0.01]
    return sorted([round(v, 1) for v in f["centroid"]] for f in found)


def change(name, expression):
    return {"command": "parameters.change", "params": {"name": name, "expression": expression}}


def test_hinges_cut_cups_and_plate_holes(approval):
    # Two doors on a 600 x 720 x 560 cabinet: a 35 mm cup 22.5 mm in from each door's hinge edge,
    # 13 deep from its back face (y = -282), about 100 mm from its ends, on the pins' grid (z = 98 and 610, from the bottom panel's top
    # face at 18: 18 + 16 + 32k); and a plate's
    # two 5 mm holes, 10 deep, in each side's inner face, 37 behind its front edge (the pins'
    # line), 16 above and below each cup.
    looked = [MEASURE, faces("Cabinet Door L"), faces("Cabinet Door R"), faces("Cabinet Side L"), faces("Cabinet Side R")]
    results = cli([plugin(approval, "cabinet", {"doors": 2}), *looked,
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": False}},
                   change("HW_cup_d", "40 mm"), change("Cabinet_width", "800 mm"), change("Cabinet_height", "900 mm"), *looked,
                   drive("Cabinet Door L hinge", 90), faces("Cabinet Door L"), faces("Cabinet Side L"),
                   plugin(approval, "cabinet", {"name": "B", "doors": 1})],
                  "--save", str(ROOT / "dist" / "hinges.sc"))
    assert holes(results[2], 17.5) == [[-275.5, -288.5, 98], [-275.5, -288.5, 610]], holes(results[2], 17.5)
    assert holes(results[3], 17.5) == [[275.5, -288.5, 98], [275.5, -288.5, 610]], holes(results[3], 17.5)
    assert holes(results[4], 2.5) == [[-287, -243, z] for z in (82, 114, 594, 626)], holes(results[4], 2.5)
    assert holes(results[5], 2.5) == [[287, -243, z] for z in (82, 114, 594, 626)], holes(results[5], 2.5)
    # the shared sizes are parameters, made once (the second cabinet makes none)
    params = {p["name"]: p for p in results[6]["params"]}
    assert params["HW_cup_d"]["expression"] == "35 mm" and params["HW_slide_t"]["expression"] == "12.7 mm"
    # A bigger cup in Parameters cuts every door again; a resize moves the holes with the edges.
    assert holes(results[11], 20) == [[-375.5, -288.5, 98], [-375.5, -288.5, 802]], holes(results[11], 20)
    assert holes(results[12], 20) == [[375.5, -288.5, 98], [375.5, -288.5, 802]], holes(results[12], 20)
    assert holes(results[14], 2.5) == [[387, -243, z] for z in (82, 114, 786, 818)], holes(results[14], 2.5)
    # Open, the door takes its cups along; the side's holes stay.
    opened = holes(results[16], 20)
    assert len(opened) == 2 and opened != holes(results[11], 20), opened
    assert holes(results[17], 2.5) == holes(results[13], 2.5)
    assert results[18]["result"]["name"] == "B"


def test_plate_holes_go_into_the_pin_holes_or_clear_of_them(approval):
    # 800 x 900 on a 100 plinth with shelf pins: pin rows 118 + 32k (k = 2 to 21) on the line 37
    # behind the front (y = -163). The cups snap to 198 and 806, so the plates' holes are at
    # 182 and 214 (the pins of k = 2 and 3: the same holes) and 790 (k = 21) and 822, above the
    # top row. None runs half into a pin hole; nor with the doors over a drawer, from the rail.
    cabinet = {"width": 800, "height": 900, "depth": 400, "shelves": 2, "plinth": 100, "shelf_pins": True, "doors": 2}
    for drawers in ({}, {"drawers": 1, "drawers_at": "bottom", "drawer_height": 200}):
        results = cli([plugin(approval, "cabinet", cabinet | drawers), faces("Cabinet Side L")])
        round_faces = holes(results[1], 2.5)
        pins = [h[1:] for h in round_faces if h[0] == -388]  # 12 deep from the inner face at -382
        plates = [h[1:] for h in round_faces if h[0] == -387]  # 10 deep
        assert len(pins) >= 20 and len(pins) + len(plates) == len(round_faces), round_faces
        for y, z in plates:
            assert all(abs(y - py) >= 5 or abs(z - pz) >= 5 for py, pz in pins), (drawers, y, z)
        if not drawers:
            assert plates == [[-163, 822]] and [-163, 182] in pins and [-163, 790] in pins, plates
        else:
            assert len(plates) < 4, plates  # over the rail too, some of them are pin holes


def test_three_doors_and_a_right_hung_one(approval):
    # Three doors hinge left, right, left: plate holes in Side L only (the other hinge edges are
    # on dividers, which get none). A single door hinged right: the plate on Side R. Inches.
    results = cli([plugin(approval, "cabinet", {"joints": "none", "joints_inner": "none", "name": "T", "width": 900, "dividers": 2, "doors": 3}),
                   plugin(approval, "cabinet", {"joints": "none", "joints_inner": "none", "name": "R", "units": "in", "width": 18, "height": 30, "depth": 12,
                                                "thickness": 0.75, "back_thickness": 0.25, "doors": 1, "door_hinge": "right"}),
                   MEASURE, faces("R Door"), faces("R Side R")])
    measure = results[2]
    assert [cylinders(measure, f"T Door {n}") for n in (1, 2, 3)] == [2, 2, 2]
    assert (cylinders(measure, "T Side L"), cylinders(measure, "T Side R")) == (4, 0)
    assert cylinders(measure, "R Side L") == 0 and len(holes(results[4], 2.5)) == 4
    # 18 in wide: the right edge at 9 in less the reveal, the cup 22.5 mm in from it
    assert [h[0] for h in holes(results[3], 17.5)] == [round(9 * 25.4 - 2 - 22.5, 1)] * 2, holes(results[3], 17.5)


def test_slides_drill_the_sides_and_the_boxes(approval):
    # Two drawers: each box 600 - 2 x 18 - 2 x 12.7 wide, its sides drilled for the slide (3 mm,
    # 8 deep from outside, 37 and 229 mm behind the box's front, in the middle of the box side's
    # height), and the carcass sides beside them (5 mm, 37 and 229 behind their front edge). A
    # sink base's false front has no slide. A thinner slide in Parameters makes every box wider.
    results = cli([plugin(approval, "cabinet", {"joints": "none", "joints_inner": "none", "drawers": 2}), MEASURE,
                   faces("Cabinet Drawer 1 Left"), faces("Cabinet Side L"),
                   plugin(approval, "cabinet", {"joints": "none", "joints_inner": "none", "name": "S", "kind": "sink", "drawers": 2}), MEASURE,
                   drive("Cabinet Drawer 1 slide", 200), faces("Cabinet Drawer 1 Left"), faces("Cabinet Side L"),
                   change("HW_slide_t", "10 mm"), MEASURE,
                   plugin(approval, "cabinet", {"joints": "none", "joints_inner": "none", "name": "N", "drawers": 1, "thickness": 9}), faces("N Side L")],
                  "--save", str(ROOT / "dist" / "slides.sc"))
    box = boxes(results[1])
    assert box["Cabinet Drawer 1 Left"][0] == -300 + 18 + 12.7 and box["Cabinet Drawer 1 Right"][3] == 300 - 18 - 12.7
    z = round((box["Cabinet Drawer 1 Left"][2] + box["Cabinet Drawer 1 Left"][5]) / 2, 1)
    left = holes(results[2], 1.5)
    assert left == [[round(-300 + 18 + 12.7 + 4, 1), -282 + 37 + d, z] for d in (0, 192)], (left, z)
    side = holes(results[3], 2.5)
    assert len(side) == 4 and [[-287, -243 + d, z] for d in (0, 192)] == [h for h in side if h[2] == z], side
    assert cylinders(results[5], "S Side L") == 2 and cylinders(results[5], "S Drawer 1 Right") == 2
    # drawn out, the box takes its holes along; the carcass side's stay
    assert [h[1] for h in holes(results[7], 1.5)] == [h[1] - 200 for h in left]
    assert holes(results[8], 2.5) == side
    assert boxes(results[10])["Cabinet Drawer 1 Left"][0] == -300 + 18 + 10
    # in a 9 mm side the holes stop 3 mm short of its outside: 6 deep from the inner face at -291
    assert [h[0] for h in holes(results[12], 2.5)] == [-294] * 2, holes(results[12], 2.5)


def test_the_cut_list_counts_hinges(approval):
    results = cli([plugin(approval, "cabinet", {"doors": 2, "drawers": 1}), plugin(approval, "cutlist")])
    hardware = {h["item"]: h["qty"] for h in results[1]["result"]["hardware"]}
    assert hardware["Hinge, cup"] == 4 and hardware["Drawer slide, pair"] == 1, hardware
