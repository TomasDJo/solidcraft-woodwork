"""Build the demo kitchen with the plugin: examples/kitchen.sc and examples/kitchen-cutlist.csv.

    python build.py              # first: builds the plugin into dist/
    python examples/kitchen.py [out.sc]

An L-shaped kitchen: a tall unit, three base units and three wall units along the back wall,
a corner cabinet at the right end of both rows, and one more base and wall unit on the return.
It uses every function of the plugin:

- Cabinet, with shelves, dividers, doors, drawers, shelf pins, handle holes and a plinth
- Corner cabinet, at the right end of the base units and of the wall units
- Place: each floor unit goes to the right of the one before it, by selecting that one, and is
  held to it: widen `Base_1_width` and the units after it move along
- Match selected: only the first base and wall unit carry sizes. The others take height, depth,
  thickness and plinth from the unit they are placed against, so `Base_1_height` sets the whole run
- Material: white melamine carcasses, then oak on the selected doors and drawer fronts
- Joinery: confirmat screws in the floor units, dowels in the wall units
- Edit cabinet: the second wall unit gets a third shelf after it is made, hung and drilled
- Labels: the tall unit has each board's name written on it
- Cut list: written out as CSV, with grain, banded edges, the sheets, banding and hardware to buy

Everything sits in a Kitchen component; every cabinet is a component inside it, and every door
and drawer a component inside its cabinet, on a joint. A few are left open.
"""
import sys
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE.parent))
import build

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else HERE / "kitchen.sc").resolve()
WHITE = {"material": "White melamine"}
FLOOR_UNIT = {**WHITE, "height": 870, "depth": 560, "plinth": 150}
BASE = {**FLOOR_UNIT, "handles": "pull", "place": "auto"}  # auto: by the selected board, a Side R
WALL = {**WHITE, "height": 700, "depth": 320, "shelves": 2, "shelf_pins": True, "handles": "knob"}
MATCH = {"match": "selected"}  # sizes from the selected cabinet, and linked to it
WALL_Z = 2100 - 700  # wall units line up with the top of the tall unit

# The floor run, left to right. The tall unit stands on the origin; each of the others is
# placed against the one before it.
FLOOR = [
    ("Tall", {**WHITE, "width": 600, "height": 2100, "depth": 560, "plinth": 150, "shelves": 4, "doors": 1,
              "handles": "pull", "labels": True}),
    ("Base 1", {**BASE, "width": 800, "dividers": 1, "doors": 2}),
    ("Base 2", {**BASE, **MATCH, "width": 600, "drawers": 3}),
    ("Base 3", {**BASE, **MATCH, "width": 900, "dividers": 2, "doors": 3}),
]
# Wall units hang over the base units: name, left edge, parameters.
HANGING = [
    ("Wall 1", 300, {**WALL, "width": 800, "doors": 2}),
    ("Wall 2", 1100, {**WALL, "width": 600, "doors": 1}),
    ("Wall 3", 1700, {**WALL, "width": 900, "dividers": 2, "doors": 3}),
]
BACK = 280  # the wall: the tall unit is 560 deep and centred on the origin
# The corner cabinet goes right of the last base unit; its return runs forward along the right
# wall, and one more base unit continues it, turned to face into the room.
CORNER = {**FLOOR_UNIT, **MATCH, "width": 1000, "return": 1000, "doors": "hinged left",
          "handles": "pull", "place": "right"}
RIGHT = 300 + sum(params["width"] for _, params in FLOOR[1:]) + CORNER["width"]  # the right wall
RETURN = {**FLOOR_UNIT, **MATCH, "width": 600, "doors": 1, "handles": "pull"}
WALL_CORNER = {**WHITE, **MATCH, "width": 1000, "return": 1000, "height": 700, "depth": 320,
               "shape": "diagonal", "shelves": 2, "shelf_pins": True, "doors": "hinged right", "handles": "knob",
               "handle_position": "middle",
               "place": "right"}
WALL_RETURN = {**WALL, **MATCH, "width": 600, "doors": 1}
OPEN = {"Tall Door hinge": 40, "Base 1 Door L hinge": 100, "Base 1 Door R hinge": 75,
        "Base 2 Drawer 2 slide": 350, "Wall 3 Door 1 hinge": 95, "Wall 3 Door 3 hinge": 60,
        "Corner Door hinge": 60, "Corner Door return fold": 60, "Base 4 Door hinge": 50, "Wall corner Door hinge": 60}


def step(command, **params):
    return {"command": command, "params": params}


def select(*bodies):
    return step("select.set", items=[{"type": "body", "name": body} for body in bodies])


approval = build.cli([step("plugin.inspect", path=str(build.PKG))])[0]["approval"]
kitchen = step("component.activate", component="Kitchen")

# 1. The cabinets.
steps = [step("component.create", name="Kitchen")]
for i, (name, params) in enumerate(FLOOR):
    if i:
        steps.append(select(f"{FLOOR[i - 1][0]} Side R"))
    steps.append(build.plugin(approval, "cabinet", {**params, "name": name}))
for name, left, params in HANGING:
    # Made on the origin, then moved up onto the wall. The plugin grounds a cabinet so that
    # its doors can open, so it is ungrounded for the move.
    move = [left + params["width"] / 2, BACK - params["depth"] / 2, WALL_Z]
    if name != HANGING[0][0]:
        steps.append(select(f"{HANGING[0][0]} Top"))
        params = {**params, **MATCH}
    steps += [build.plugin(approval, "cabinet", {**params, "name": name}),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=False),
              step("occurrence.move", occurrence=f"{name}:1", translate=move),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=True)]
steps += [select(f"{FLOOR[-1][0]} Top"), build.plugin(approval, "corner", {**CORNER, "name": "Corner"}),
          select(f"{HANGING[-1][0]} Top"), build.plugin(approval, "corner", {**WALL_CORNER, "name": "Wall corner"})]
# The units on the return are made on the origin and drilled there (joinery wants a cabinet
# square to the axes), then turned a quarter and put at the end of the corner cabinet's return.
end = BACK - CORNER["return"]
for name, params, kind, z, like in (("Base 4", RETURN, "confirmat", 0, "Corner"),
                                    ("Wall 4", WALL_RETURN, "dowel", WALL_Z, "Wall corner")):
    to = {"position": [RIGHT - params["depth"] / 2, end - params["width"] / 2, z], "axis": [0, 0, 1], "angle": "-90 deg"}
    steps += [select(f"{like} Top"), build.plugin(approval, "cabinet", {**params, "name": name}),
              select(f"{name} Top"), build.plugin(approval, "joinery", {"type": kind}),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=False),
              step("occurrence.move", occurrence=f"{name}:1", to=to),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=True)]
# An oak worktop over the floor units, 40 mm past the carcass fronts, in two pieces.
steps += [step("solid.box", length=f"{RIGHT - 300} mm", width="600 mm", height="30 mm",
               corner=[300, BACK - 600, 870], name="Worktop", body_name="Worktop"),
          step("solid.box", length="600 mm", width=f"{CORNER['return'] - 600 + RETURN['width']} mm", height="30 mm",
               corner=[RIGHT - 600, end - RETURN["width"], 870], name="Worktop return", body_name="Worktop return"),
          step("material.assign", bodies=["Worktop", "Worktop return"], material="Oak")]
results = build.cli(steps, "--save", str(OUT))
made = [r["result"] for r in results if isinstance(r, dict) and "bodies" in r.get("result", {})]

# 2. Oak fronts, joinery, and a few doors and a drawer left open.
steps = [kitchen]
fronts = [b for cabinet in made for b in cabinet["bodies"] if b.endswith(" Front") or " Door" in b]
steps += [select(*fronts), build.plugin(approval, "material", {"material": "Oak", "scope": "parts"})]
for cabinet in made:
    if "orner" in cabinet["name"] or cabinet["name"].endswith(" 4"):  # no joinery for a corner cabinet; the turned units have theirs
        continue
    kind = "dowel" if cabinet["name"].startswith("Wall") else "confirmat"
    steps += [select(f"{cabinet['name']} Top"), build.plugin(approval, "joinery", {"type": kind})]
# One more shelf in a wall unit that is already hung, has an oak door and is drilled for dowels.
steps += [select("Wall 2 Top"), build.plugin(approval, "edit", {"shelves": 3})]
steps += [step("joint.drive", joint=joint, value=value) for joint, value in OPEN.items()]
steps += [step("select.clear"), step("component.activate", component="root"),
          build.plugin(approval, "cutlist")]
cutlist = build.cli(steps, "--in", str(OUT), "--save", str(OUT))[-1]["result"]

OUT.with_name(OUT.stem + "-cutlist.csv").write_text(cutlist["csv"], encoding="utf-8", newline="\n")
for sheet in cutlist["sheets"]:
    print(f"{sheet['sheets_2440x1220']:.0f} sheets of {sheet['thickness']:g} mm {sheet['material']} ({sheet['area_m2']} m2)")
for item in cutlist["hardware"]:
    print(item["qty"], "x", item["item"])
print(cutlist["boards"], "boards ->", OUT)
