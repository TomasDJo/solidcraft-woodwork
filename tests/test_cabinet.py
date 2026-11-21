"""The box cabinet: carcass, shelf pins, dividers, and following its parameters."""
from helpers import *


def test_carcass_pins_and_doors(approval):
    cabinet = {"width": 800, "height": 900, "depth": 400, "shelves": 2, "plinth": 100,
               "shelf_pins": True, "doors": 2}
    results = cli([plugin(approval, "cabinet", cabinet),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "cabinet.sc"))
    measure, cutlist = results[1], results[2]
    # the plugin's own result may be wrapped by plugin.run
    cutlist = cutlist.get("result", cutlist)
    bodies = {b["name"]: b for b in measure["bodies"]}
    # 2 sides, bottom, top, 2 shelves, plinth, back, 2 doors
    assert len(bodies) == 10, sorted(bodies)
    side = bodies["Cabinet Side L"]
    # clear height 764: pin rows 2..21 of the 32 mm grid, front and rear
    assert side["face_types"]["cylinder"] == 40, side["face_types"]
    assert bodies["Cabinet Door L"]["face_types"]["cylinder"] == 2
    # centred on the origin, standing on Z = 0
    assert [round(v, 6) for v in side["bbox"]["min"]] == [-400, -200, 0], side["bbox"]
    assert [round(v, 6) for v in side["bbox"]["max"]] == [-382, 200, 900], side["bbox"]
    # the groove took material out of the side: 9 deep, 6 wide
    assert side["volume_mm3"] < 18 * 400 * 900 - 9 * 6 * 700, side["volume_mm3"]
    assert cutlist["boards"] == 10, cutlist
    assert cutlist["hardware"] == [{"item": "Hinge, 35 mm cup", "qty": 4},
                                   {"item": "Shelf pin, 5 mm", "qty": 8}], cutlist["hardware"]



def test_dividers_and_a_resize(approval):
    # Dividers: 3 compartments of 2 shelves, pin holes through the dividers.
    wide = {"width": 1200, "dividers": 2, "shelves": 2, "shelf_pins": True}
    results = cli([plugin(approval, "cabinet", wide), {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "dividers.sc"))
    bodies = {b["name"]: b for b in results[1]["bodies"]}
    assert len(bodies) == 4 + 2 + 6 + 1, sorted(bodies)
    shelf = bodies["Cabinet Shelf 2.1"]["bbox"]
    assert round(shelf["max"][0] - shelf["min"][0], 6) == 376, shelf
    # clear height 684: pin rows 2..19, front and rear
    assert bodies["Cabinet Divider 1"]["face_types"]["cylinder"] == 36
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
