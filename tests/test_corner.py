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
    assert '3,964,864,18,"Birch plywood",yes,L1,"C Bottom, C Top, C Shelf 1"' in results[7]["result"]["csv"], results[7]["result"]["csv"]
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
    # Hinged on the return instead, with a pull on the folding door and shelf pins: the return door
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
    # 2 hinge cups each; 2 screw holes on the door that folds only. 15 pin holes in each of 2 rows
    assert cylinders(results[1], "C Door return") == 2 and cylinders(results[1], "C Door") == 4
    # the hinge plates' screw holes in Side L land in its pin holes, on the same grid
    for part in ("Side L", "Side R", "Back", "Back return"):
        assert cylinders(results[1], f"C {part}") == 30, (part, cylinders(results[1], f"C {part}"))
    swung = boxes(results[4])
    assert swung["C Door return"][3] > 300, swung["C Door return"]
    assert swung["C Door"] == [-6.603, -513.588, 102, 362.664, -290, 718], swung["C Door"]
    hardware = {h["item"]: h["qty"] for h in results[5]["result"]["hardware"]}
    assert hardware == {"Hinge, cup": 4, "Shelf pin, 5 mm": 8, "Pull, 128 mm": 1}, hardware
    assert not [f["name"] for f in results[6]["timeline"] if f["warning"]]


def test_inset_doors(approval):
    # Inset in an inch design: the bi-fold pair a panel back inside the opening, the reveal in
    # from the end panel, bottom and top. Folded and swung a quarter, the pair stays inside the
    # cabinet's width, so it clears the neighbour, which stands where it would by an overlay corner.
    inset = {**CORNER, "fronts": "inset", "shelf_pins": True}
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "corner", inset), MEASURE,
                   select("C Top"), plugin(approval, "cabinet", {"name": "N", "place": "right"}),
                   drive("C Door return fold", 90), MEASURE,
                   drive("C Door hinge", 90), MEASURE,
                   plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "corner-inset.sc"))
    box = boxes(results[2])
    assert box["C Door"] == [42, -110, 120, 480, -92, 700], box["C Door"]
    assert box["C Door return"] == [42, -430, 120, 60, -112, 700], box["C Door return"]
    # the shelf's notch a panel back behind the doors; the bottom's at the carcass front
    assert volume(results[2], "C Shelf 1") == (964 * 864 - 440 * 340) * 18, volume(results[2], "C Shelf 1")
    assert volume(results[2], "C Bottom") == (964 * 864 - 422 * 322) * 18, volume(results[2], "C Bottom")
    box = boxes(results[6])
    assert box["N Side L"][0] == 500 and box["N Side L"][4] == 450, box["N Side L"]
    assert box["C Door return"] == [62, -128, 120, 380, -110, 700], box["C Door return"]  # on the first's face
    box = boxes(results[8])
    assert box["C Door"] == [462, -548, 120, 480, -110, 700], box["C Door"]  # clear of the end panel
    assert box["C Door return"] == [480, -528, 120, 498, -210, 700], box["C Door return"]
    hardware = {h["item"]: h["qty"] for h in results[9]["result"]["hardware"]}
    assert hardware == {"Hinge, cup": 4, "Shelf pin, 5 mm": 8}, hardware
    assert not [f["name"] for f in results[10]["timeline"] if f["warning"]]
    # Edit makes an overlay corner's doors inset
    results = cli([plugin(approval, "corner", CORNER), select("C Top"),
                   plugin(approval, "edit", {"fronts": "inset"}), MEASURE])
    assert boxes(results[3])["C Door"] == [42, -110, 120, 480, -92, 700], boxes(results[3])["C Door"]


def test_plinth_types(approval):
    # Notched ends (no plinth boards), and on legs: six under the L, five under the diagonal.
    corner = {**CORNER, "doors": "none"}
    results = cli([plugin(approval, "corner", corner), MEASURE,
                   plugin(approval, "corner", {**corner, "name": "N", "plinth_type": "notch"}), MEASURE,
                   plugin(approval, "corner", {**corner, "name": "L", "plinth_type": "legs"}),
                   plugin(approval, "corner", {**corner, "name": "D", "shape": "diagonal", "plinth_type": "legs"}),
                   MEASURE, plugin(approval, "cutlist")])
    plain, notched, legs = results[1], results[3], results[6]
    box = boxes(notched)
    assert not [n for n in box if n.startswith("N Plinth")], sorted(box)
    # the toe kick, 50 x 100, out of the front bottom corner of each end
    for end in ("Side L", "Side R"):
        assert volume(plain, f"C {end}") - volume(notched, f"N {end}") == 18 * 50 * 100, end
        assert box[f"N {end}"] == boxes(plain)[f"C {end}"], end
    box = boxes(legs)
    assert sorted(n for n in box if n.startswith("L Leg ")) == [f"L Leg {n}" for n in range(1, 7)], sorted(box)
    assert len([n for n in box if n.startswith("D Leg ")]) == 5, sorted(box)
    assert box["L Leg 1"] == [430, -80, 0, 470, -40, 100], box["L Leg 1"]
    assert box["L Leg 6"] == [-10, -80, 0, 30, -40, 100], box["L Leg 6"]  # the L's inner corner
    for part in ("L Side L", "L Back", "L Back return"):
        assert box[part][2] == 100 and box[part][5] == 720, (part, box[part])
    assert not [n for n in box if n.startswith(("L Plinth", "D Plinth"))], sorted(box)
    assert {"item": "Adjustable leg", "qty": 11} in results[7]["result"]["hardware"], results[7]["result"]["hardware"]


def test_split_boards(approval):
    # A big corner, 1400 x 1400: its L-shaped boards (1364 x 1364) are too big for a 2440 x 1220
    # sheet. Split, each is two rectangles with two dowels across the joint, and they nest.
    split = {"name": "C", "width": 1400, "return": 1400, "depth": 560, "boards": "split"}
    results = cli([plugin(approval, "corner", {**split, "shelf_pins": True}), MEASURE,
                   plugin(approval, "corner", {**split, "name": "W", "boards": "one piece"}), MEASURE,
                   plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "corner-split.sc"))
    box = boxes(results[1])
    # the back leg's board over the whole width; the return's in front of it, to the joint
    assert box["C Bottom"] == [-682, 140, 0, 682, 682, 18], box["C Bottom"]
    assert box["C Bottom return"] == [-682, -682, 0, -140, 140, 18], box["C Bottom return"]
    assert box["C Shelf 1 return"][:2] == [-682, -682] and box["C Top return"][4] == 140
    # a dowel hole 30 mm into each board
    dowels = 2 * 3.14159265 * 16 * 30
    for part, size in (("Bottom", 1364 * 542), ("Bottom return", 542 * 822)):
        assert cylinders(results[1], f"C {part}") == 2, part
        assert abs(volume(results[1], f"C {part}") - (size * 18 - dowels)) < 2, (part, volume(results[1], f"C {part}"))
    # one piece, the same corner is an L with its corner cut out
    assert "W Bottom return" not in boxes(results[3]) and boxes(results[3])["W Bottom"][1] == -682
    shown = results[4]["result"]
    csv = shown["csv"]
    assert '3,1364,542,18,"Birch plywood",yes,L1,"C Bottom, C Top, C Shelf 1"' in csv, csv
    assert '3,822,542,18,"Birch plywood",yes,L1,"C Bottom return, C Top return, C Shelf 1 return"' in csv, csv
    # the split boards are on the sheets; the L is not (nor are the backs, 1364 wide with the grain up)
    oversize = {o["part"] for o in shown["oversize"]}
    assert {"W Bottom", "W Top", "W Shelf 1"} <= oversize and not [p for p in oversize if p.startswith("C ") and "Back" not in p], oversize
    on_sheets = {p["part"] for layout in shown["layouts"] for p in layout["parts"]}
    assert {"C Bottom", "C Bottom return", "C Shelf 1", "C Shelf 1 return"} <= on_sheets, on_sheets
    assert {"item": "Dowel, 8 x 60 mm", "qty": 6} in shown["hardware"], shown["hardware"]
    assert {"item": "Shelf pin, 5 mm", "qty": 8} in shown["hardware"], shown["hardware"]  # one split shelf
    assert not [f["name"] for f in results[5]["timeline"] if f["warning"]]
    # Edit keeps it split, with the return pieces' own material; a right corner is the mirror image
    results = cli([plugin(approval, "corner", split), select("C Bottom return"),
                   plugin(approval, "material", {"material": "Oak", "scope": "parts"}),
                   {"command": "parameters.change", "params": {"name": "C_width", "expression": "1500 mm"}},
                   select("C Top"), plugin(approval, "edit", {"shelves": 2}), MEASURE,
                   plugin(approval, "corner", {**split, "name": "R", "corner": "right"}), MEASURE])
    box = boxes(results[6])
    assert "C Shelf 2 return" in box, sorted(box)
    assert box["C Bottom return"] == [-732, -682, 0, -190, 140, 18], box["C Bottom return"]
    assert cylinders(results[6], "C Bottom return") == 2
    material = {b["name"]: b["material"] for b in results[6]["bodies"]}
    assert material["C Bottom return"] == "Oak" and material["C Bottom"] == "Birch plywood", material
    box = boxes(results[8])
    assert box["R Bottom return"] == [140, -682, 0, 682, 140, 18], box["R Bottom return"]
    assert cylinders(results[8], "R Top return") == 2


def test_wall_mount(approval):
    # A left corner wall cabinet on French cleats along both walls with shelf pins, its mirror
    # image, a diagonal one on hanging rails; then the left one edited to steel rails.
    wall = {"name": "W", "width": 1000, "return": 900, "depth": 330, "shelf_pins": True,
            "wall_mount": "cleat"}
    results = cli([plugin(approval, "corner", wall), MEASURE,
                   plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}},
                   select("W Top"), plugin(approval, "edit", {"wall_mount": "steel"}), MEASURE,
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "corner_wall_mount.sc"))
    box, measure = boxes(results[1]), results[1]
    # the cleats under the top, their rear faces on the walls; the wall cleats nested below
    assert box["W Cleat"] == [-500, 432, 612, 482, 450, 702], box["W Cleat"]
    assert box["W Wall cleat"] == [-500, 432, 540, 482, 450, 630], box["W Wall cleat"]
    assert box["W Cleat return"] == [-500, -432, 612, -482, 432, 702], box["W Cleat return"]
    assert box["W Wall cleat return"] == [-500, -432, 540, -482, 432, 630], box["W Wall cleat return"]
    for cleat, length in (("W Cleat", 982), ("W Wall cleat", 982), ("W Cleat return", 864)):
        assert abs(volume(measure, cleat) - length * (18 * 90 - 18 * 18 / 2)) < 1, (cleat, volume(measure, cleat))
    # bevelled on the inside: the cabinet's cleat lower towards the room, the wall's higher there
    mass = {b["name"]: b["center_of_mass"] for b in measure["bodies"]}
    assert mass["W Cleat return"][0] < -491 and mass["W Cleat return"][2] > 657, mass["W Cleat return"]
    assert mass["W Wall cleat return"][0] > -491 and mass["W Wall cleat return"][2] < 585, mass["W Wall cleat return"]
    # the backs a cleat off the walls, between bottom and top; the top over the cleats
    assert box["W Back"] == [-464, 414, 18, 482, 432, 702], box["W Back"]
    assert box["W Back return"] == [-482, -432, 18, -464, 432, 702], box["W Back return"]
    assert box["W Top"][:2] == [-500, -432] and box["W Top"][4] == 450, box["W Top"]
    assert box["W Shelf 1"][0] == -464 and box["W Shelf 1"][4] == 414, box["W Shelf 1"]
    # the pin rows in the backs, not through them
    assert cylinders(measure, "W Back") > 0 and cylinders(measure, "W Back return") > 0
    assert not [f["name"] for f in results[3]["timeline"] if f["warning"]]
    sizes = {name: (p["length"], p["width"], p["thickness"]) for p in results[2]["result"]["parts"] for name in p["parts"]}
    assert sizes["W Cleat return"] == (864, 90, 18), sizes["W Cleat return"]
    # edited to steel: no boards, the backs at the walls, a rail on each wall
    box = boxes(results[6])
    assert not [n for n in box if "leat" in n], sorted(box)
    assert box["W Back"][4] == 450, box["W Back"]
    hardware = results[7]["result"]["hardware"]
    for item in ({"item": "Steel wall rail, 1000 mm", "qty": 1}, {"item": "Steel wall rail, 900 mm", "qty": 1},
                 {"item": "Cabinet hanger", "qty": 4}):
        assert item in hardware, hardware
    # the mirror image, and a diagonal on hanging rails
    results = cli([plugin(approval, "corner", {**wall, "corner": "right"}), MEASURE,
                   plugin(approval, "corner", {"name": "D", "shape": "diagonal", "depth": 330, "wall_mount": "rail"}),
                   MEASURE])
    mirror = boxes(results[1])
    assert mirror["W Cleat return"] == [482, -432, 612, 500, 432, 702], mirror["W Cleat return"]
    mass = {b["name"]: b["center_of_mass"] for b in results[1]["bodies"]}
    assert mass["W Cleat return"][0] > 491, mass["W Cleat return"]
    box = boxes(results[3])
    assert box["D Hanging rail"][1:3] == [432, 612] and box["D Hanging rail return"][0:3] == [-450, -432, 612], (box["D Hanging rail"], box["D Hanging rail return"])


def test_casters(approval):
    # On casters: six under the L at the legs' places but clear of the ends and backs, their
    # plates screwed into the bottom; split boards take the two at the return's end in the
    # return's board; five under the diagonal.
    corner = {**CORNER, "doors": "none", "plinth_type": "casters"}
    results = cli([plugin(approval, "corner", {**corner, "name": "S"}),
                   plugin(approval, "corner", {**corner, "name": "P", "boards": "split", "corner": "right"}),
                   plugin(approval, "corner", {**corner, "name": "D", "shape": "diagonal"}),
                   MEASURE, plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "corner_casters.sc"))
    measure = results[3]
    box = boxes(measure)
    assert sorted(n for n in box if n.startswith("S Caster ")) == [f"S Caster {n}" for n in range(1, 7)], sorted(box)
    assert len([n for n in box if n.startswith("D Caster ")]) == 5, sorted(box)
    assert box["S Caster 1"] == [392, -100, 0, 472, -20, 100], box["S Caster 1"]
    assert box["S Caster 3"] == [-472, 342, 0, -392, 422, 100], box["S Caster 3"]  # the wall corner
    assert box["S Caster 6"] == [-30, -100, 0, 50, -20, 100], box["S Caster 6"]  # the L's inner corner
    for part in ("S Side L", "S Back", "S Back return"):
        assert box[part][2] == 100, (part, box[part])
    assert not [n for n in box if "Plinth" in n or " Leg " in n], sorted(box)
    # four pilot holes a caster, in the board above it
    assert cylinders(measure, "S Bottom") == 24, cylinders(measure, "S Bottom")
    # (and the two dowels through both)
    assert cylinders(measure, "P Bottom") == 16 + 2 and cylinders(measure, "P Bottom return") == 8 + 2
    assert {"item": "Caster, locking", "qty": 17} in results[4]["result"]["hardware"], results[4]["result"]["hardware"]
    assert not [f["name"] for f in results[5]["timeline"] if f["warning"]]


def test_hinge_holes_on_the_shared_hardware(approval):
    # Without pins: the hinge plates' screw holes in the end the pair hangs on, 37 mm behind the
    # front, about each cup's height; the cups sized by the shared HW_ parameters, which a
    # change in Parameters follows. The diagonal door gets the cups only.
    results = cli([plugin(approval, "corner", {**CORNER, "name": "C"}), MEASURE,
                   faces("C Side R"), faces("C Door"),
                   plugin(approval, "corner", {**CORNER, "name": "D", "shape": "diagonal"}), MEASURE,
                   {"command": "parameters.change", "params": {"name": "HW_cup_d", "expression": "40 mm"}},
                   faces("C Door"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    measure = results[1]
    assert cylinders(measure, "C Side R") == 4 and cylinders(measure, "C Side L") == 0
    holes = [f for f in results[2]["faces"] if f.get("cylinder")]
    # 37 mm behind the back leg's front (y = -110 at 1000 x 900, 560 deep), on the inner face
    assert {round(f["centroid"][1]) for f in holes} == {-73}, holes
    heights = sorted(round(f["centroid"][2]) for f in holes)
    # 32 mm apart about each cup, on the pins' grid over the bottom panel (z = 118)
    assert heights[1] - heights[0] == 32 and (heights[0] - 118) % 32 == 0, heights
    cups = [f for f in results[3]["faces"] if (f.get("cylinder") or {}).get("radius")]
    assert {round(f["cylinder"]["radius"], 1) for f in cups} >= {17.5}, cups
    assert cylinders(results[5], "D Door") == 2 and cylinders(results[5], "D Side R") == 0
    cups = [f for f in results[7]["faces"] if (f.get("cylinder") or {}).get("radius")]
    assert {round(f["cylinder"]["radius"], 1) for f in cups} >= {20.0}, cups
    assert not [f["name"] for f in results[8]["timeline"] if f["warning"]]
