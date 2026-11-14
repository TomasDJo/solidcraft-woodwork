"""Build, package and end-to-end test the plugin.

    python build.py            build dist/woodwork/ and dist/woodwork-<version>.zip, then test
    python build.py --install  also install the zip into SolidCraft

Needs: cargo (with the wasm32-unknown-unknown target), wasm-tools, solidcraft-cli.
"""
import json
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).parent
PKG = ROOT / "dist" / "woodwork"
os.environ["PATH"] += os.pathsep + str(Path.home() / ".cargo" / "bin")
CLI = shutil.which("solidcraft-cli") or r"C:\Program Files\SolidCraft\solidcraft-cli.exe"


def sh(*args):
    subprocess.run(args, cwd=ROOT, check=True)


def cli(script, *extra):
    """Run a command script through solidcraft-cli and return the results."""
    path = ROOT / "dist" / "script.json"
    path.write_text(json.dumps(script), encoding="utf-8")
    out = ROOT / "dist" / "results.json"
    sh(CLI, "run", str(path), "--results", "--quiet", "--json-out", str(out), *extra)
    return json.loads(out.read_text(encoding="utf-8"))


def plugin(approval, entry, params=None):
    return {"command": "plugin.run", "params": {
        "path": str(PKG), "approval": approval, "entry": entry, "params": params or {}}}


def build():
    sh("cargo", "test", "--quiet")
    sh("cargo", "build", "--release", "--target", "wasm32-unknown-unknown")
    PKG.mkdir(parents=True, exist_ok=True)
    sh("wasm-tools", "component", "new",
       "target/wasm32-unknown-unknown/release/woodwork.wasm", "-o", str(PKG / "woodwork.wasm"))
    shutil.copy(ROOT / "plugin.json", PKG)
    shutil.copy(ROOT / "LICENSE", PKG)
    version = json.loads((PKG / "plugin.json").read_text(encoding="utf-8"))["version"]
    archive = ROOT / "dist" / f"woodwork-{version}.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
        for name in ("plugin.json", "woodwork.wasm", "LICENSE"):
            z.write(PKG / name, f"woodwork/{name}")
    return archive


def test():
    approval = cli([{"command": "plugin.inspect", "params": {"path": str(PKG)}}])[0]["approval"]
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
    print(cutlist["csv"])

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

    # Place against the selection: B right of A, C on top of B. In an inch design, to check
    # that the move is not read in inches.
    def select(name):
        return {"command": "select.set", "params": {"items": [{"type": "body", "name": name}]}}
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"name": "A", "width": 600}),
                   select("A Shelf 1"),
                   plugin(approval, "cabinet", {"name": "B", "width": 800, "depth": 400, "place": "right"}),
                   select("B Side L"),
                   plugin(approval, "cabinet", {"name": "C", "width": 500, "depth": 300, "place": "above"}),
                   {"command": "inspect.measure", "params": {}}])
    box = {b["name"]: [round(v, 6) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[6]["bodies"]}
    assert box["A Side R"][3] == 300 and box["B Side L"][0] == 300, (box["A Side R"], box["B Side L"])
    assert box["B Side L"][4] == 280, box["B Side L"]  # backs flush at y = 280
    assert box["C Side L"][:3] == [300, -20, 720] and box["C Side L"][4] == 280, box["C Side L"]

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
    box = {b["name"]: [round(v, 4) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[3]["bodies"]}
    assert box["Cabinet Drawer 1 Right"][3] == 400 - 18 - 12.7, box["Cabinet Drawer 1 Right"]

    def cylinders(measure, name):
        return next(b for b in measure["bodies"] if b["name"] == name)["face_types"].get("cylinder", 0)
    # Handles: a pull is two holes through each front, next to a door's two hinge cups.
    results = cli([plugin(approval, "cabinet", {"name": "H", "doors": 2, "plinth": 100, "handles": "pull"}),
                   plugin(approval, "cabinet", {"name": "K", "drawers": 2, "handles": "knob"}),
                   {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "handles.sc"))
    assert cylinders(results[2], "H Door L") == 4 and cylinders(results[2], "K Drawer 2 Front") == 1

    # Joints: doors swing out on their hinges and a drawer slides out, in an inch design to
    # check the joint's point is not read in inches. The cabinet still moves as one piece.
    def drive(joint, value):
        return {"command": "joint.drive", "params": {"joint": joint, "value": value}}
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
    # left door: turned 90 degrees about its inner left edge (x -298, y -282), now along -Y
    assert box["D Door L"] == [-316, -579, 1002, -298, -282, 1718], box["D Door L"]
    # right door: stopped by its limit at 110 degrees
    assert box["D Door R"][1] < -282 - 297 * 0.9 and box["D Door R"][3] > 298, box["D Door R"]
    assert box["E Drawer 1 Front"][1] == -300 - 300, box["E Drawer 1 Front"]
    assert box["E Drawer 2 Front"][1] == -300, box["E Drawer 2 Front"]

    # Joinery: confirmat holes at the four corners of a cabinet inside a Kitchen component.
    # 560 deep: 3 screws per joint. They spread out with a resize to 900 deep.
    results = cli([{"command": "component.create", "params": {"name": "Kitchen"}},
                   plugin(approval, "cabinet", {"name": "J", "shelves": 0}),
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

    # Joinery on cabinets that have been moved: placed to the right, and inside a moved room.
    # Every hole must remove material.
    # (B is as wide as A: drilled on the origin, its numbers are those of A's faces.)
    results = cli([plugin(approval, "cabinet", {"name": "A", "height": 2100}),
                   select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "shelves": 0, "place": "right"}),
                   select("B Top"),
                   plugin(approval, "joinery", {"type": "dowel"}),
                   {"command": "component.create", "params": {"name": "Room", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "Room:1", "translate": [2000, 500, 0]}},
                   plugin(approval, "cabinet", {"name": "C", "shelf_pins": True}),
                   select("C Top"),
                   plugin(approval, "joinery", {"type": "confirmat"}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    assert not [f["name"] for f in results[11]["timeline"] if f["warning"]]
    for cabinet in "BC":
        for part in ("Side L", "Side R", "Bottom", "Top"):
            pins = 36 if cabinet == "C" and part.startswith("Side") else 0
            assert cylinders(results[10], f"{cabinet} {part}") == 6 + pins, (cabinet, part)

    # Corner cabinet: an L of 1000 x 900 with the return on the left, a box cabinet placed to
    # its right, both doors opened. Then the mirror image.
    corner = {"name": "C", "width": 1000, "return": 900, "plinth": 100, "doors": True}
    results = cli([plugin(approval, "corner", corner),
                   {"command": "inspect.measure", "params": {}},
                   select("C Top"),
                   plugin(approval, "cabinet", {"name": "N", "place": "right"}),
                   drive("C Door hinge", 60), drive("C Door return hinge", 60),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist"),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}],
                  "--save", str(ROOT / "dist" / "corner.sc"))
    def boxes(measure):
        return {b["name"]: [round(v, 3) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in measure["bodies"]}
    box = boxes(results[1])
    assert len(box) == 11, sorted(box)  # 2 sides, 2 backs, bottom, top, shelf, 2 plinth boards, 2 doors
    assert box["C Side L"] == [-500, -450, 0, 60, -432, 720], box["C Side L"]  # the end of the return
    assert box["C Side R"] == [482, -110, 0, 500, 450, 720], box["C Side R"]
    # the doors stop 2 mm short of each other in the inner corner
    assert box["C Door"] == [82, -130, 102, 498, -112, 718], box["C Door"]
    assert box["C Door return"] == [62, -448, 102, 80, -132, 718], box["C Door return"]
    # an L-shaped board: 964 x 864 less the open corner of 422 x 322
    bottom = next(b for b in results[1]["bodies"] if b["name"] == "C Bottom")
    assert round(bottom["volume_mm3"]) == (964 * 864 - 422 * 322) * 18, bottom["volume_mm3"]
    box = boxes(results[6])
    assert box["N Side L"][0] == 500 and box["N Side L"][4] == 450, box["N Side L"]  # beside it, backs flush
    assert box["C Door"][1] < -400 and box["C Door return"][3] > 300, (box["C Door"], box["C Door return"])  # both open outwards
    assert '3,964,864,18,"Birch plywood","C Bottom, C Top, C Shelf 1"' in results[7]["result"]["csv"]
    assert not [f["name"] for f in results[8]["timeline"] if f["warning"]]
    results = cli([plugin(approval, "corner", {**corner, "corner": "right"}),
                   drive("C Door hinge", 60), drive("C Door return hinge", 60),
                   {"command": "inspect.measure", "params": {}}])
    mirror = boxes(results[3])
    for part in ("C Door", "C Door return", "C Bottom", "C Plinth"):
        x0, y0, z0, x1, y1, z1 = box[part]
        assert mirror[part] == [-x1, y0, z0, -x0, y1, z1], (part, mirror[part], box[part])
    assert mirror["C Side R"] == [-60, -450, 0, 500, -432, 720], mirror["C Side R"]

    # Match selected: B takes A's height, depth, thickness and plinth, and follows A afterwards.
    # The corner cabinet too. In an inch design, with B's own sizes given in inches.
    a = {"name": "A", "height": 900, "depth": 400, "thickness": 19, "plinth": 100}
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", a),
                   select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "units": "in", "width": 20, "doors": 1,
                                                "match": "selected", "place": "right"}),
                   select("B Top"),
                   plugin(approval, "corner", {"name": "K", "corner": "right", "match": "selected", "place": "right"}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "parameters.change", "params": {"changes": {"A_height": "800 mm", "A_plinth": "150 mm"}}},
                   {"command": "inspect.measure", "params": {}},
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    box = boxes(results[6])
    assert box["B Side L"] == [300, -200, 0, 319, 200, 900], box["B Side L"]
    assert box["B Bottom"][2] == 100 and box["B Side R"][3] == 808, (box["B Bottom"], box["B Side R"])
    assert box["K Side L"][5] == 900 and box["K Bottom"][2] == 100, (box["K Side L"], box["K Bottom"])
    box = boxes(results[8])
    for cabinet in "ABK":
        assert box[f"{cabinet} Side L"][5] == 800 and box[f"{cabinet} Bottom"][2] == 150, cabinet
    assert not [f["name"] for f in results[9]["timeline"] if f["warning"]]

    # Materials: oak doors on a melamine cabinet, next to a plywood one.
    def pick(*names):
        return {"command": "select.set", "params": {"items": [{"type": "body", "name": n} for n in names]}}
    results = cli([plugin(approval, "cabinet", {"name": "A", "doors": 2}),
                   plugin(approval, "cabinet", {"name": "A", "material": "none"}),  # taken: becomes A 2
                   pick("A Top"),
                   plugin(approval, "material", {"material": "White melamine"}),
                   pick("A Door L", "A Door R"),
                   plugin(approval, "material", {"material": "Oak", "scope": "parts"}),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")])
    material = {b["name"]: b["material"] for b in results[6]["bodies"]}
    assert material["A Side L"] == material["A Back"] == "White melamine", material
    assert material["A Door L"] == material["A Door R"] == "Oak", material
    assert material["A 2 Side L"] == "Default", material  # untouched: another cabinet
    sheets = results[7].get("result", results[7])["sheets"]
    assert {s["material"] for s in sheets} == {"White melamine", "Oak", "Default"}, sheets

    # Inches: a 24 x 30 x 22 in cabinet in a design set to inches.
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"units": "in", "width": 24, "height": 30, "depth": 22,
                                                "thickness": 0.75, "back_thickness": 0.25}),
                   plugin(approval, "cutlist")])
    cutlist = results[2].get("result", results[2])
    assert cutlist["units"] == "in", cutlist
    assert '2,30,22,3/4,"Birch plywood","Cabinet Side L, Cabinet Side R"' in cutlist["csv"], cutlist["csv"]
    print(cutlist["csv"])
    print("PASS")


if __name__ == "__main__":
    archive = build()
    test()
    if "--install" in sys.argv:
        print(cli([{"command": "plugin.install", "params": {"path": str(archive), "replace": True}}]))
