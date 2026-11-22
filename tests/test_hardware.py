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
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "T", "width": 900, "dividers": 2, "doors": 3}),
                   plugin(approval, "cabinet", {"name": "R", "units": "in", "width": 18, "height": 30, "depth": 12,
                                                "thickness": 0.75, "back_thickness": 0.25, "doors": 1, "door_hinge": "right"}),
                   MEASURE, faces("R Door"), faces("R Side R")])
    measure = results[2]
    assert [cylinders(measure, f"T Door {n}") for n in (1, 2, 3)] == [2, 2, 2]
    assert (cylinders(measure, "T Side L"), cylinders(measure, "T Side R")) == (4, 0)
    assert cylinders(measure, "R Side L") == 0 and len(holes(results[4], 2.5)) == 4
    # 18 in wide: the right edge at 9 in less the reveal, the cup 22.5 mm in from it
    assert [h[0] for h in holes(results[3], 17.5)] == [round(9 * 25.4 - 2 - 22.5, 1)] * 2, holes(results[3], 17.5)


def test_slides_drill_the_sides_and_the_boxes(approval):
    # Two drawers: each box 600 - 2 x 18 - 2 x 12.7 wide, its sides drilled for the slide (500 mm,
    # the longest that fits the 532 mm box; 3 mm, 8 deep from outside, 37 and 37 + 13 x 32 = 453
    # mm behind the box's front, in the middle of the box side's
    # height), and the carcass sides beside them (5 mm, as far behind their front edge). A
    # sink base's false front has no slide. A thinner slide in Parameters makes every box wider.
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"drawers": 2}), MEASURE,
                   faces("Cabinet Drawer 1 Left"), faces("Cabinet Side L"),
                   plugin(approval, "cabinet", {"name": "S", "kind": "sink", "drawers": 2}), MEASURE,
                   drive("Cabinet Drawer 1 slide", 200), faces("Cabinet Drawer 1 Left"), faces("Cabinet Side L"),
                   change("HW_slide_t", "10 mm"), MEASURE,
                   plugin(approval, "cabinet", {"name": "N", "drawers": 1, "thickness": 9}), faces("N Side L")],
                  "--save", str(ROOT / "dist" / "slides.sc"))
    box = boxes(results[1])
    assert box["Cabinet Drawer 1 Left"][0] == -300 + 18 + 12.7 and box["Cabinet Drawer 1 Right"][3] == 300 - 18 - 12.7
    z = round((box["Cabinet Drawer 1 Left"][2] + box["Cabinet Drawer 1 Left"][5]) / 2, 1)
    left = holes(results[2], 1.5)
    assert left == [[round(-300 + 18 + 12.7 + 4, 1), -282 + 37 + d, z] for d in (0, 416)], (left, z)
    side = holes(results[3], 2.5)
    assert len(side) == 4 and [[-287, -243 + d, z] for d in (0, 416)] == [h for h in side if h[2] == z], side
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
    assert hardware["Hinge, cup"] == 4 and hardware["Drawer slide, full extension, 500 mm, pair"] == 1, hardware


def models(measure, model="HW "):
    """The hardware models' boxes, from a measure through the CLI (the plugin's own leave them out)."""
    return {name: box for name, box in boxes(measure, models=True).items() if name.startswith(model)}


def middle(box):
    return [round((box[i] + box[i + 3]) / 2, 1) for i in range(3)]


def test_models_show_and_move_with_the_doors_and_drawers(approval):
    # One door and one drawer: a hinge model (cup and arm) in each cup, in the door's component,
    # and its mounting plate on the side, in the cabinet's; a slide's drawer
    # member on each box side in the drawer's component, its cabinet member on each carcass side.
    # The definitions are made once, under the floor, in a component at the root.
    results = cli([plugin(approval, "cabinet", {"doors": 1, "drawers": 1}), MEASURE, faces("Cabinet Door"),
                   drive("Cabinet Door hinge", 90), drive("Cabinet Drawer 1 slide", 200), MEASURE,
                   change("Cabinet_width", "700 mm"), drive("Cabinet Door hinge", 0), drive("Cabinet Drawer 1 slide", 0), MEASURE,
                   plugin(approval, "cabinet", {"name": "B", "doors": 1}),
                   {"command": "component.list", "params": {"compact": True}}],
                  "--save", str(ROOT / "dist" / "models.sc"))
    shut, opened, wider = models(results[1]), models(results[5]), models(results[9])
    hinges = sorted(n for n in shut if n.startswith("HW hinge (Cabinet:1/Cabinet Door:1/"))
    drawer = sorted(n for n in shut if n.startswith("HW slide drawer 500 mm (Cabinet:1/Cabinet Drawer 1:1/"))
    carcass = sorted(n for n in shut if n.startswith("HW slide cabinet 500 mm (Cabinet:1/HW"))
    plates = sorted(n for n in shut if n.startswith("HW hinge plate (Cabinet:1/HW"))
    assert (len(hinges), len(plates), len(drawer), len(carcass)) == (2, 2, 2, 2), sorted(shut)
    # the slides as long as the 500 mm slide that fits the box; the plates on Side L's inner
    # face (x = -282), 8.5 thick and 40 deep, centred on the plate holes 37 behind its front edge
    assert all(abs(shut[n][4] - shut[n][1] - 500) < 0.01 for n in drawer + carcass)
    assert all(shut[n][0] == -282 and shut[n][3] == -273.5 and shut[n][4] - shut[n][1] == 40
               and middle(shut[n])[1] == -243 for n in plates), [shut[n] for n in plates]
    # each hinge's cup is in a cup hole: its middle on the hole's axis, half the depth in
    cups = holes(results[2], 17.5)
    for name, cup in zip(hinges, sorted(cups, key=lambda c: c[2])):
        x, _, z = middle(shut[name])
        assert abs(x - cup[0]) < 0.2 and abs(z - cup[2]) < 0.2, (name, shut[name], cup)
    for name in [*hinges, *plates, *carcass, *drawer]:  # under the floor: the definitions only
        assert shut[name][2] > 0, (name, shut[name])
    definitions = {"HW hinge", "HW hinge plate", "HW slide cabinet 500 mm", "HW slide middle 500 mm",
                   "HW slide drawer 500 mm"}
    assert {n for n in shut if "(" not in n} == definitions
    assert all(shut[n][5] < 0 for n in definitions)
    # open: the hinges swing with the door, the drawer members slide 200 out, the cabinet members stay
    assert all(opened[n] != shut[n] for n in hinges)
    # (the middle members, on a full extension slide, are in the drawer's component too)
    middles = sorted(n for n in shut if n.startswith("HW slide middle 500 mm (Cabinet:1/Cabinet Drawer 1:1/"))
    assert len(middles) == 2
    assert all(abs(opened[n][1] - (shut[n][1] - 200)) < 0.01 for n in drawer + middles)
    assert all(opened[n] == shut[n] for n in carcass + plates)
    # wider in Parameters: the models follow, as the holes do
    assert abs(abs(middle(wider[carcass[-1]])[0] - middle(shut[carcass[-1]])[0]) - 50) < 0.2
    # a second cabinet copies the same definitions
    tree = {c["name"]: c for c in results[11]["components"]}
    assert [c for c in tree if c.startswith("Woodwork hardware")] == ["Woodwork hardware"]
    assert len(tree["HW hinge"]["occurrences"]) == 1 + 2 + 2


def test_models_off_is_the_same_cabinet_without_them(approval):
    # The boards, their holes and the cut list are the same with the models as without; the
    # cut list counts no model, and its hardware is counted once.
    cabinet = {"doors": 2, "drawers": 1, "handles": "pull"}
    def made(mode):
        results = designed(approval, {"hardware_models": mode}, [plugin(approval, "cabinet", cabinet), MEASURE, plugin(approval, "cutlist")])
        measure = {b["name"]: (b["bbox"], round(b["volume_mm3"]), b["face_types"]) for b in results[1]["bodies"]}
        return measure, results[2]["result"]
    (on, on_list), (off, off_list) = made("on"), made("off")
    assert {n: v for n, v in on.items() if not model(n)} == off, set(on) ^ set(off)
    assert any(n.startswith("HW hinge (") for n in on) and not any(n.startswith("HW") for n in off)
    assert on_list["csv"] == off_list["csv"] and on_list["hardware"] == off_list["hardware"]
    assert {h["item"]: h["qty"] for h in on_list["hardware"]}["Hinge, cup"] == 4


def test_models_can_cut_the_cups(approval):
    # `cut` (for SolidCraft 0.47): the hinge models cut the cups instead of cylinders. The
    # doors come out the same, the cut list counts the cups.
    def door(mode):
        results = designed(approval, {"hardware_models": mode}, [plugin(approval, "cabinet", {"doors": 1}), MEASURE,
                       faces("Cabinet Door"), plugin(approval, "cutlist"),
                       {"command": "document.inspect", "params": {"bodies": False, "timeline": "summary"}}])
        features = [f["name"] for f in results[4]["timeline"]]
        hardware = {h["item"]: h["qty"] for h in results[3]["result"]["hardware"]}
        return volume(results[1], "Cabinet Door"), holes(results[2], 17.5), hardware, features
    on, cut = door("on"), door("cut")
    assert abs(on[0] - cut[0]) <= 1 and on[1] == cut[1] and on[2] == cut[2], (on[:3], cut[:3])
    assert "Cabinet Door hinge cup 1" in cut[3]


def test_edit_makes_the_models_again_and_keeps_the_definitions(approval):
    results = cli([plugin(approval, "cabinet", {"doors": 1}), select("Cabinet Side L"),
                   plugin(approval, "edit", {"doors": 2}), MEASURE,
                   select("Cabinet Side L"), plugin(approval, "edit", {"hardware_models": "off"}), MEASURE,
                   {"command": "component.list", "params": {"compact": True}}])
    assert len([n for n in models(results[3], "HW hinge (") if "Cabinet Door" in n]) == 4, sorted(models(results[3]))
    assert not models(results[6], "HW hinge (")
    tree = {c["name"]: c for c in results[7]["components"]}
    assert len(tree["HW hinge"]["occurrences"]) == 1 and "Woodwork hardware" in tree


def test_slide_type_and_hinge_angle_set_the_joints_limits(approval):
    # A 165 degree hinge opens the door that far; a partial extension 500 mm slide pulls the
    # drawer out 375 mm and stops there, with no middle member; the cut list names the slide.
    results = designed(approval, {"hinge_angle": "165", "slide_type": "partial"}, [plugin(approval, "cabinet", {"doors": 1, "drawers": 1}),
                   {"command": "joint.list", "params": {}}, MEASURE,
                   drive("Cabinet Drawer 1 slide", 600), drive("Cabinet Door hinge", 200), MEASURE,
                   plugin(approval, "cutlist")])
    limits = {j["name"]: j["values"][0]["limits"] for j in results[1]["joints"]}
    assert limits == {"Cabinet Door hinge": [0, 165], "Cabinet Drawer 1 slide": [0, 375]}, limits
    assert results[3]["values"][0]["value"] == 375 and results[4]["values"][0]["value"] == 165
    shut, out = boxes(results[2]), boxes(results[5])
    assert abs(out["Cabinet Drawer 1 Front"][1] - (shut["Cabinet Drawer 1 Front"][1] - 375)) < 0.01
    assert not models(results[2], "HW slide middle")
    hardware = {h["item"]: h["qty"] for h in results[6]["result"]["hardware"]}
    assert hardware["Drawer slide, partial extension, 500 mm, pair"] == 1, hardware
