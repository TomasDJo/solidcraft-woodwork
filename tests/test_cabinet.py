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
