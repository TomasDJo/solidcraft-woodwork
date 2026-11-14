"""Build a demo kitchen with the plugin and save it as a design.

    python examples/kitchen.py [out.sc]

A straight run against a wall (the wall is the plane Y = 0, the room is at -Y): a tall unit,
three base units under a worktop, and three wall units. Everything sits in a Kitchen
component; each cabinet is a component inside it, made by the plugin on the origin and then
moved into place as one piece.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
import build

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else build.ROOT / "dist" / "kitchen.sc").resolve()
BASE = {"height": 870, "depth": 560, "plinth": 150}
WALL = {"height": 700, "depth": 320, "shelves": 2, "shelf_pins": True}
WALL_Z = 2100 - 700  # wall units line up with the top of the tall unit

# name, left edge, height above the floor, parameters
CABINETS = [
    ("Tall", 0, 0, {"width": 600, "height": 2100, "depth": 560, "plinth": 150, "shelves": 4, "doors": 1}),
    ("Base 1", 600, 0, {**BASE, "width": 800, "dividers": 1, "doors": 2}),
    ("Base 2", 1400, 0, {**BASE, "width": 600, "drawers": 3}),
    ("Base 3", 2000, 0, {**BASE, "width": 900, "dividers": 2, "doors": 3}),
    ("Wall 1", 600, WALL_Z, {**WALL, "width": 800, "doors": 2}),
    ("Wall 2", 1400, WALL_Z, {**WALL, "width": 600, "doors": 1}),
    ("Wall 3", 2000, WALL_Z, {**WALL, "width": 900, "dividers": 2, "doors": 3}),
]

approval = build.cli([{"command": "plugin.inspect", "params": {"path": str(build.PKG)}}])[0]["approval"]
kitchen = {"command": "component.activate", "params": {"component": "Kitchen"}}
build.cli([{"command": "component.create", "params": {"name": "Kitchen"}}], "--save", str(OUT))
for name, x, z, params in CABINETS:
    made = build.cli([kitchen, build.plugin(approval, "cabinet", {**params, "name": name})],
                     "--in", str(OUT), "--save", str(OUT))[1]
    occurrence = made.get("result", made)["occurrence"]
    move = [x + params["width"] / 2, -params["depth"] / 2, z]
    # The plugin grounds a cabinet so its doors can open: unground it to move it.
    build.cli([kitchen,
               {"command": "occurrence.ground", "params": {"occurrence": occurrence, "grounded": False}},
               {"command": "occurrence.move", "params": {"occurrence": occurrence, "translate": move}},
               {"command": "occurrence.ground", "params": {"occurrence": occurrence, "grounded": True}}],
              "--in", str(OUT), "--save", str(OUT))

# Worktop over the base units: 20 mm past the door fronts, 30 mm thick.
steps = [kitchen,
         {"command": "solid.box", "params": {
             "length": "2300 mm", "width": "600 mm", "height": "30 mm", "corner": [600, -600, 870],
             "name": "Worktop", "body_name": "Worktop"}},
         {"command": "component.activate", "params": {"component": "root"}},
         build.plugin(approval, "cutlist")]
cutlist = build.cli(steps, "--in", str(OUT), "--save", str(OUT))[3]
cutlist = cutlist.get("result", cutlist)
print(cutlist["csv"])
print(cutlist["boards"], "boards,", cutlist["area_m2"], "m2 ->", OUT)
