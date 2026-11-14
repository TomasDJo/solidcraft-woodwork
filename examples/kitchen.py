"""Build the demo kitchen with the plugin: examples/kitchen.sc and examples/kitchen-cutlist.csv.

    python build.py              # first: builds the plugin into dist/
    python examples/kitchen.py [out.sc]

A straight run against a wall: a tall unit, three base units under a worktop, three wall
units. It uses every function of the plugin:

- Cabinet, with shelves, dividers, doors, drawers, shelf pins and a plinth
- Place: each base unit goes to the right of the one before it, by selecting that one
- Material: white melamine carcasses, then oak on the selected doors and drawer fronts
- Joinery: confirmat screws in the floor units, dowels in the wall units
- Cut list: written out as CSV, with the sheets to buy

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
BASE = {**WHITE, "height": 870, "depth": 560, "plinth": 150, "place": "right"}
WALL = {**WHITE, "height": 700, "depth": 320, "shelves": 2, "shelf_pins": True}
WALL_Z = 2100 - 700  # wall units line up with the top of the tall unit

# The floor run, left to right. The tall unit stands on the origin; each of the others is
# placed against the one before it.
FLOOR = [
    ("Tall", {**WHITE, "width": 600, "height": 2100, "depth": 560, "plinth": 150, "shelves": 4, "doors": 1}),
    ("Base 1", {**BASE, "width": 800, "dividers": 1, "doors": 2}),
    ("Base 2", {**BASE, "width": 600, "drawers": 3}),
    ("Base 3", {**BASE, "width": 900, "dividers": 2, "doors": 3}),
]
# Wall units hang over the base units: name, left edge, parameters.
HANGING = [
    ("Wall 1", 300, {**WALL, "width": 800, "doors": 2}),
    ("Wall 2", 1100, {**WALL, "width": 600, "doors": 1}),
    ("Wall 3", 1700, {**WALL, "width": 900, "dividers": 2, "doors": 3}),
]
BACK = 280  # the wall: the tall unit is 560 deep and centred on the origin
OPEN = {"Tall Door hinge": 40, "Base 1 Door L hinge": 100, "Base 1 Door R hinge": 75,
        "Base 2 Drawer 2 slide": 350, "Wall 3 Door 1 hinge": 95, "Wall 3 Door 3 hinge": 60}


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
        steps.append(select(f"{FLOOR[i - 1][0]} Top"))
    steps.append(build.plugin(approval, "cabinet", {**params, "name": name}))
for name, left, params in HANGING:
    # Made on the origin, then moved up onto the wall. The plugin grounds a cabinet so that
    # its doors can open, so it is ungrounded for the move.
    move = [left + params["width"] / 2, BACK - params["depth"] / 2, WALL_Z]
    steps += [build.plugin(approval, "cabinet", {**params, "name": name}),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=False),
              step("occurrence.move", occurrence=f"{name}:1", translate=move),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=True)]
# An oak worktop over the base units, 40 mm past the carcass fronts.
steps += [step("solid.box", length="2300 mm", width="600 mm", height="30 mm",
               corner=[300, BACK - 600, 870], name="Worktop", body_name="Worktop"),
          step("material.assign", bodies=["Worktop"], material="Oak")]
results = build.cli(steps, "--save", str(OUT))
made = [r["result"] for r in results if isinstance(r, dict) and "bodies" in r.get("result", {})]

# 2. Oak fronts, joinery, and a few doors and a drawer left open.
steps = [kitchen]
fronts = [b for cabinet in made for b in cabinet["bodies"] if b.endswith(" Front") or " Door" in b]
steps += [select(*fronts), build.plugin(approval, "material", {"material": "Oak", "scope": "parts"})]
for cabinet in made:
    kind = "dowel" if cabinet["name"].startswith("Wall") else "confirmat"
    steps += [select(f"{cabinet['name']} Top"), build.plugin(approval, "joinery", {"type": kind})]
steps += [step("joint.drive", joint=joint, value=value) for joint, value in OPEN.items()]
steps += [step("select.clear"), step("component.activate", component="root"),
          build.plugin(approval, "cutlist")]
cutlist = build.cli(steps, "--in", str(OUT), "--save", str(OUT))[-1]["result"]

OUT.with_name(OUT.stem + "-cutlist.csv").write_text(cutlist["csv"], encoding="utf-8", newline="\n")
for sheet in cutlist["sheets"]:
    print(f"{sheet['sheets_2440x1220']:.0f} sheets of {sheet['thickness']:g} mm {sheet['material']} ({sheet['area_m2']} m2)")
print(cutlist["boards"], "boards ->", OUT)
