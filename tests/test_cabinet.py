"""The box cabinet: carcass, shelf pins, dividers, and following its parameters."""
from helpers import *


def test_carcass_pins_and_doors(approval):
    cabinet = {"width": 800, "height": 900, "depth": 400, "shelves": 2, "plinth": 100,
               "shelf_pins": True, "doors": 2}
    results = designed(approval, NONE, [plugin(approval, "cabinet", cabinet),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "cabinet.sc"))
    measure, cutlist = results[1], results[2]
    # the plugin's own result may be wrapped by plugin.run
    cutlist = cutlist.get("result", cutlist)
    bodies = {b["name"]: b for b in boards(measure)}
    # 2 sides, bottom, top, 2 shelves, plinth, back, 2 doors
    assert len(bodies) == 10, sorted(bodies)
    side = bodies["Cabinet Side L"]
    # clear height 764: pin rows 2..21 of the 32 mm grid, front and rear
    # and of the hinge plate's 4 screw holes on the pins' line, the 1 that is not a pin hole
    # (822, over the top row; 182, 214 and 790 are pin holes)
    assert side["face_types"]["cylinder"] == 40 + 1, side["face_types"]
    # one pattern copies both sides' pins
    assert bodies["Cabinet Side R"]["face_types"]["cylinder"] == 40 + 1, bodies["Cabinet Side R"]["face_types"]
    assert bodies["Cabinet Door L"]["face_types"]["cylinder"] == 2
    # centred on the origin, standing on Z = 0
    assert [round(v, 6) for v in side["bbox"]["min"]] == [-400, -200, 0], side["bbox"]
    assert [round(v, 6) for v in side["bbox"]["max"]] == [-382, 200, 900], side["bbox"]
    # the groove took material out of the side: 9 deep, 6 wide
    assert side["volume_mm3"] < 18 * 400 * 900 - 9 * 6 * 700, side["volume_mm3"]
    assert cutlist["boards"] == 10, cutlist
    assert cutlist["hardware"] == [{"item": "Hinge, cup", "qty": 4},
                                   {"item": "Shelf pin, 5 mm", "qty": 8}], cutlist["hardware"]



def test_dividers_and_a_resize(approval):
    # Dividers: 3 compartments of 2 shelves, pin holes through the dividers.
    wide = {"width": 1200, "dividers": 2, "shelves": 2, "shelf_pins": True}
    results = designed(approval, NONE, [plugin(approval, "cabinet", wide), {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "dividers.sc"))
    bodies = {b["name"]: b for b in results[1]["bodies"]}
    assert len(bodies) == 4 + 2 + 6 + 1, sorted(bodies)
    shelf = bodies["Cabinet Shelf 2.1"]["bbox"]
    assert round(shelf["max"][0] - shelf["min"][0], 6) == 376, shelf
    # clear height 684: pin rows 2..19, front and rear
    assert bodies["Cabinet Divider 1"]["face_types"]["cylinder"] == 36
    assert bodies["Cabinet Divider 2"]["face_types"]["cylinder"] == 36
    assert bodies["Cabinet Side L"]["face_types"]["cylinder"] == 36

    # Resize through the parameters: the whole cabinet follows.
    resize = {"changes": {"Cabinet_width": "1500 mm", "Cabinet_height": "1000 mm"}}
    results = cli([{"command": "parameters.change", "params": resize},
                   {"command": "inspect.measure", "params": {}}],
                  "--in", str(ROOT / "dist" / "dividers.sc"))
    bodies = {b["name"]: b for b in results[1]["bodies"]}
    shelf = bodies["Cabinet Shelf 2.1"]["bbox"]
    assert round(shelf["max"][0] - shelf["min"][0], 6) == 476, shelf
    side = bodies["Cabinet Side R"]
    assert [round(v, 6) for v in side["bbox"]["max"]] == [750, 280, 1000], side["bbox"]
    # clear height 964: pin rows 2..28
    assert side["face_types"]["cylinder"] == 54, side["face_types"]


def test_the_copy_of_a_component_one_works_in_stays_active(approval):
    # A room that stands in the design twice, the second copy active: the cabinet is made in
    # the room, and afterwards that copy is still the active one, not the first.
    tree = {"command": "component.list", "params": {"compact": True}}
    results = cli([{"command": "component.create", "params": {"name": "Room", "grounded": False}},
                   {"command": "occurrence.copy", "params": {"component": "Room", "translate": [5000, 0, 0]}},
                   tree])
    copy = results[1]["occurrence"]
    assert copy != results[0]["occurrence"], results[:2]
    activate = {"command": "component.activate", "params": {"component": "Room", "occurrence": copy, "path": [copy]}}
    results = cli([{"command": "component.create", "params": {"name": "Room", "grounded": False}},
                   {"command": "occurrence.copy", "params": {"component": "Room", "translate": [5000, 0, 0]}},
                   activate, tree, plugin(approval, "cabinet", {"name": "A"}), tree])
    assert results[3]["active_path"] == [copy], results[3]["active_path"]
    assert results[5]["active_path"] == [copy], results[5]["active_path"]
    assert results[5]["active"] == results[3]["active"]


def test_plinth_setback(approval):
    # A on a board 80 back, B notched 80 deep beside it, C in inches notched 3 in deep and D on
    # a board 3 in back; then an Edit that brings A's board to 30.
    results = cli([plugin(approval, "cabinet", {"name": "A", "plinth": 100, "plinth_setback": 80}),
                   select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "plinth": 100, "plinth_type": "notch",
                                                "plinth_setback": 80, "place": "right"}),
                   select("B Side R"),
                   plugin(approval, "cabinet", {"name": "C", "units": "in", "plinth": 4, "plinth_type": "notch",
                                                "plinth_setback": 3, "place": "right"}),
                   select("C Side R"),
                   plugin(approval, "cabinet", {"name": "D", "units": "in", "plinth": 4, "plinth_setback": 3,
                                                "place": "right"}),
                   MEASURE,
                   select("A Side L"),
                   plugin(approval, "edit", {"plinth_setback": 30}),
                   MEASURE])
    box, measure = boxes(results[7]), results[7]
    front = box["A Side L"][1]
    assert box["A Plinth"][1] == front + 80, (box["A Plinth"], front)
    assert volume(measure, "A Side L") - volume(measure, "B Side L") == 18 * 80 * 100
    inch = 25.4
    notch = volume(measure, "D Side L") - volume(measure, "C Side L")
    assert abs(notch - 0.75 * 3 * 4 * inch ** 3) < 2, notch
    assert abs(box["D Plinth"][1] - box["D Side L"][1] - 3 * inch) < 1e-3, box["D Plinth"]
    box = boxes(results[10])
    assert box["A Plinth"][1] == box["A Side L"][1] + 30, (box["A Plinth"], box["A Side L"])


def test_casters(approval):
    # A on casters (1000 wide: six) with an end panel on its left, B beside it on a board; the
    # cut list.
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "A", "width": 1000, "plinth": 100, "plinth_type": "casters"}),
                   select("A Side L"), plugin(approval, "panel", {"kind": "end"}),
                   select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "width": 1000, "plinth": 100, "place": "right"}),
                   MEASURE,
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "casters.sc"))
    box, measure = boxes(results[5]), results[5]
    casters = sorted(n for n in box if n.startswith("A Caster "))
    assert casters == [f"A Caster {n}" for n in range(1, 7)] and "A Plinth" not in box, sorted(box)
    # floor to the bottom panel: an 80 mm plate under it, clear of the side, its 75 mm wheel under it
    x0 = box["A Side L"][0]
    assert box["A Caster 1"] == [x0 + 28, -270, 0, x0 + 108, -190, 100], box["A Caster 1"]
    assert box["A Caster 3"] == [x0 + 892, -270, 0, x0 + 972, -190, 100], box["A Caster 3"]
    assert box["A Bottom"][2] == 100 and box["A Side L"][2] == 100, box["A Side L"]
    # four 4 mm pilot holes a caster, 8 mm deep in the 18 mm bottom
    assert cylinders(measure, "A Bottom") == 24, cylinders(measure, "A Bottom")
    drop = volume(measure, "B Bottom") - volume(measure, "A Bottom")
    assert abs(drop - 24 * 3.14159 * 2 * 2 * 8) < 60, drop
    # it rolls: the end panel stops at the carcass's bottom
    assert box["A End panel L"][2] == 100 and box["A End panel L"][5] == 720, box["A End panel L"]
    # the cut list: casters are hardware, not boards
    cutlist = results[6]["result"]
    assert {"item": "Caster, locking", "qty": 6} in cutlist["hardware"], cutlist["hardware"]
    assert "Caster" not in str(cutlist["parts"]), cutlist["parts"]


def test_plinth_types(approval):
    # A on a set-back board, B with notched sides beside it, C on legs (1000 wide: six) beside
    # that; the cut list, a drawing of B, and an Edit that puts A on legs.
    results = cli([plugin(approval, "cabinet", {"name": "A", "plinth": 100}),
                   select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "plinth": 100, "plinth_type": "notch", "place": "right"}),
                   select("B Side R"),
                   plugin(approval, "cabinet", {"name": "C", "width": 1000, "plinth": 100, "plinth_type": "legs", "place": "right"}),
                   select("C Side R"),
                   plugin(approval, "cabinet", {"name": "D", "place": "right"}),
                   MEASURE,
                   plugin(approval, "cutlist"),
                   select("B Side L"),
                   plugin(approval, "drawing", {"scope": "selected"}),
                   select("A Side L"),
                   plugin(approval, "edit", {"plinth_type": "legs"}),
                   MEASURE],
                  "--save", str(ROOT / "dist" / "plinth.sc"))
    box, measure = boxes(results[7]), results[7]
    volume_of = lambda name: volume(measure, name)
    # B: no board, the same sides less a 50 x 100 toe kick each, the bottom on top of it
    assert "B Plinth" not in box and "A Plinth" in box, sorted(box)
    assert box["B Side L"][1:3] == box["A Side L"][1:3] and box["B Side L"][5] == 720, box["B Side L"]
    assert volume_of("A Side L") - volume_of("B Side L") == 18 * 50 * 100, (volume_of("A Side L"), volume_of("B Side L"))
    assert box["B Bottom"][2] == 100, box["B Bottom"]
    # C: no board, six 40 mm legs 100 high, the carcass on them; D beside it on the floor
    legs = sorted(n for n in box if n.startswith("C Leg "))
    assert legs == [f"C Leg {n}" for n in range(1, 7)] and "C Plinth" not in box, sorted(box)
    x0 = box["C Side L"][0]
    assert box["C Leg 1"] == [x0 + 30, -250, 0, x0 + 70, -210, 100], box["C Leg 1"]
    assert box["C Side L"][2] == 100 and box["C Side L"][5] == 720 and box["C Bottom"][2] == 100, box["C Side L"]
    assert box["D Side L"][2] == 0, box["D Side L"]
    # the cut list: a notched side's size is its bounding box, noted; legs are hardware
    cutlist = results[8]["result"]
    row = next(p for p in cutlist["parts"] if "B Side L" in p["parts"])
    assert row["note"] and (row["length"], row["width"]) == (720, 560) and "C Leg 1" not in str(cutlist["parts"]), row
    assert "(notched" in cutlist["csv"], cutlist["csv"]
    assert {"item": "Adjustable leg", "qty": 6} in cutlist["hardware"], cutlist["hardware"]
    assert not any("Leg" in p["part"] for layout in cutlist["layouts"] for p in layout["parts"])
    # the drawing of B is made: its dimensions are on corners a notched side has
    assert results[10]["result"]["sheets"] == ["B"], results[10]
    # Edit: A on four legs now, its board gone, where it stood
    box = boxes(results[13])
    assert sorted(n for n in box if n.startswith("A Leg ")) == [f"A Leg {n}" for n in range(1, 5)], sorted(box)
    assert "A Plinth" not in box and box["A Side L"][2] == 100 and box["A Side L"][0] == boxes(results[7])["A Side L"][0], box["A Side L"]


def test_wall_mounts(approval):
    # A on a French cleat, B on a hanging rail without a back, C on a steel rail, D none; the cut
    # list, Joinery and a drawing of A, and E matched to D on a cleat.
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "A", "wall_mount": "cleat"}),
                   select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "wall_mount": "rail", "back": "none", "place": "right"}),
                   select("B Side R"),
                   plugin(approval, "cabinet", {"name": "C", "wall_mount": "steel", "back": "overlay", "place": "right"}),
                   select("C Side R"),
                   plugin(approval, "cabinet", {"name": "D", "place": "right"}),
                   MEASURE,
                   plugin(approval, "cutlist"),
                   select("A Side L"),
                   plugin(approval, "edit", {"joints": "confirmat"}),
                   select("A Side L"),
                   plugin(approval, "drawing", {"scope": "selected"}),
                   MEASURE,
                   select("D Side R"),
                   plugin(approval, "cabinet", {"name": "E", "wall_mount": "cleat", "match": "selected", "place": "right"}),
                   MEASURE],
                  "--save", str(ROOT / "dist" / "wall_mount.sc"))
    box, measure = boxes(results[7]), results[7]
    x0, y1 = box["A Side L"][0], box["A Side L"][4]
    # A: the cleat between the sides under the top, its rear face on theirs, bevelled under
    # its front; the wall cleat nested under it, bevelled on top at the wall
    assert box["A Cleat"] == [x0 + 18, y1 - 18, 612, x0 + 582, y1, 702], box["A Cleat"]
    assert box["A Wall cleat"] == [x0 + 18, y1 - 18, 540, x0 + 582, y1, 630], box["A Wall cleat"]
    bevelled = 564 * (18 * 90 - 18 * 18 / 2)
    for cleat in ("A Cleat", "A Wall cleat"):
        assert abs(volume(measure, cleat) - bevelled) < 1, volume(measure, cleat)
    # the cabinet's cleat is lower at the wall, the wall's higher in front: their centres of mass
    mass = {b["name"]: b["center_of_mass"] for b in measure["bodies"]}
    # (each loses a corner: the cabinet's its front bottom, the wall's its rear top)
    assert mass["A Cleat"][1] > y1 - 9 and mass["A Cleat"][2] > 657, mass["A Cleat"]
    assert mass["A Wall cleat"][1] < y1 - 9 and mass["A Wall cleat"][2] < 585, mass["A Wall cleat"]
    # the back in front of the cleat, the shelf in front of the back
    assert box["A Back"][4] == y1 - 18 and box["A Shelf 1"][4] == y1 - 24, (box["A Back"], box["A Shelf 1"])
    # B: a 90 mm hanging rail as thick as the panels, the shelf in front of it
    yb = box["B Side L"][4]
    assert box["B Hanging rail"][1:3] == [yb - 18, 612] and box["B Hanging rail"][4:] == [yb, 702], box["B Hanging rail"]
    assert box["B Shelf 1"][4] == yb - 18, box["B Shelf 1"]
    # C: nothing modelled; D as ever, its back 12 mm from the rear
    assert not [n for n in box if n.startswith(("C ", "D ")) and ("leat" in n or "rail" in n)], sorted(box)
    assert box["D Back"][4] == box["D Side L"][4] - 12, box["D Back"]
    # the cut list: the cleats and the rail are boards; C's steel rail and hangers are bought
    cutlist = results[8]["result"]
    sizes = {name: (p["length"], p["width"], p["thickness"]) for p in cutlist["parts"] for name in p["parts"]}
    assert sizes["A Cleat"] == sizes["A Wall cleat"] == (564, 90, 18), sizes["A Cleat"]
    assert sizes["B Hanging rail"] == (564, 90, 18), sizes["B Hanging rail"]
    assert {"item": "Steel wall rail, 600 mm", "qty": 1} in cutlist["hardware"], cutlist["hardware"]
    assert {"item": "Cabinet hanger", "qty": 2} in cutlist["hardware"], cutlist["hardware"]
    # Joinery drills A's corners and leaves the cleats as they were
    after = results[13]
    assert cylinders(after, "A Bottom") > 0 and cylinders(after, "A Cleat") == 0, after
    assert abs(volume(after, "A Cleat") - bevelled) < 1
    # the drawing of A is made
    assert results[12]["result"]["sheets"] == ["A"], results[12]
    # E, matched to D, hangs on its cleat
    box = boxes(results[16])
    assert box["E Cleat"][5] == box["E Side L"][5] - 18, box["E Cleat"]


def test_wall_mount_edit(approval):
    # A on a steel rail, then a cleat, then none; a parameter change moves the cleats.
    results = cli([plugin(approval, "cabinet", {"name": "A", "wall_mount": "steel"}),
                   select("A Side L"),
                   plugin(approval, "edit", {"wall_mount": "cleat"}),
                   MEASURE,
                   plugin(approval, "cutlist"),
                   {"command": "parameters.change", "params": {"name": "A_cleat_height", "expression": "100 mm"}},
                   MEASURE,
                   select("A Side L"),
                   plugin(approval, "edit", {"shelves": 2}),
                   MEASURE,
                   select("A Side L"),
                   plugin(approval, "edit", {"wall_mount": "none"}),
                   MEASURE,
                   plugin(approval, "cutlist")])
    box = boxes(results[3])
    y1 = box["A Side L"][4]
    assert box["A Cleat"][4:] == [y1, 702] and box["A Back"][4] == y1 - 18, (box["A Cleat"], box["A Back"])
    # steel's parameter went to 0: no rail bought for a cabinet on a cleat
    hardware = str(results[4]["result"]["hardware"])
    assert "Steel" not in hardware and "hanger" not in hardware, hardware
    # the cleats follow their parameter, also with the bevel
    box = boxes(results[6])
    assert box["A Cleat"][2] == 602 and box["A Wall cleat"][2] == 702 - 200 + 18, (box["A Cleat"], box["A Wall cleat"])
    # Keep keeps it, at the changed height
    box = boxes(results[9])
    assert box["A Cleat"][2] == 602 and "A Shelf 2" in box, box["A Cleat"]
    # none: the cleats gone, the back 12 mm from the rear again
    box = boxes(results[12])
    assert "A Cleat" not in box and "A Wall cleat" not in box, sorted(box)
    assert box["A Back"][4] == y1 - 12, box["A Back"]
    assert "Steel" not in str(results[13]["result"]["hardware"])


def test_fields_that_cannot_combine_are_hidden_and_not_used(approval):
    # Drawers take no dividers: with drawers the dialog hides Dividers, and the app leaves a
    # hidden field out of the run, so A has drawers and no divider. B is matched to A: its
    # height, depth, thickness and plinth come from A and are hidden, and so is Sizing (by the
    # drawers' inside), which a matched cabinet cannot take: it is sized from its width.
    results = cli([plugin(approval, "cabinet", {"name": "A", "drawers": 2, "dividers": 1, "height": 800}),
                   select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "drawers": 1, "sizing": "drawer", "height": 500,
                                                "match": "selected", "place": "right"}),
                   MEASURE])
    box = boxes(results[3])
    assert "A Drawer 2 Front" in box and not [n for n in box if "Divider" in n], sorted(box)
    assert box["B Side L"][5] == 800 and round(box["B Side R"][3] - box["B Side L"][0]) == 600, box["B Side L"]


def test_shelves_on_pilaster_strips(approval):
    # #264: Shelf support Pilaster strips: no pin holes; a 16 x 5 mm dado at the front and back
    # of each side over the shelves' height, a strip in each flush with the side's inner face,
    # and four clips under each shelf. The cut list buys the strips by length and the clips.
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"shelves": 2, "shelf_pins": True, "shelf_support": "strips"}),
                                        MEASURE, plugin(approval, "cutlist"),
                                        plugin(approval, "cabinet", {"name": "Plain", "shelves": 2, "place": "origin"}), MEASURE])
    measure = results[1]
    assert cylinders(measure, "Cabinet Side L") == 0
    every = boxes(measure, models=True)
    strips = {n: b for n, b in every.items() if n.startswith("Cabinet Side L pilaster ")}
    assert len(strips) == 2, sorted(every)
    for box in strips.values():
        assert (round(box[0], 3), round(box[3], 3), round(box[4] - box[1], 3)) == (-287, -282, 16), box
    clips = [n for n in every if " clip" in n]
    assert len(clips) == 8, clips
    # the side lost the two dados
    length = round(next(iter(strips.values()))[5] - next(iter(strips.values()))[2])
    lost = volume(results[4], "Plain Side L") - volume(measure, "Cabinet Side L")
    assert lost == 2 * 16 * 5 * length, (lost, length)
    hardware = {h["item"]: h["qty"] for h in results[2]["result"]["hardware"]}
    assert hardware["Shelf clip"] == 8 and hardware[f"Shelf pilaster strip, {length} mm"] == 4, hardware
    assert "Shelf pin, 5 mm" not in hardware, hardware
