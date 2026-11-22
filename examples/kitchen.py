"""Build the demo kitchen with the plugin: examples/kitchen.sc and examples/kitchen-cutlist.csv.

    python build.py              # first: builds the plugin into dist/
    python examples/kitchen.py [out.sc]

An L-shaped kitchen: a tall unit, three base units and three wall units along the back wall,
a corner cabinet at the right end of both rows, and one more base and wall unit on the return.
It uses every function of the plugin:

- Cabinet, with shelves, dividers, doors, drawers, shelf pins, handle holes and a plinth; an
  inset door on the tall unit, a graduated drawer unit (a lower top drawer), a drawer over
  a door on the return, and Base 3 is a sink base (rails instead of a top)
- Corner cabinet, at the right end of the base units and of the wall units; the wall one hangs on French cleats along both walls
- Place: each floor unit goes to the right of the one before it, by selecting that one, and is
  held to it: widen `Base_1_width` and the units after it move along
- Match selected: only the first base and wall unit carry sizes. The others take height, depth,
  thickness and plinth from the unit they are placed against, so `Base_1_height` sets the whole run
- Material: white melamine carcasses, then oak on the selected doors and drawer fronts
- Joinery chosen in the Cabinet dialog: confirmat screws in most floor units (biscuits in the tall
  one, cam locks in the drawer unit, pocket screws in the sink base), dowels in the wall units (a
  dado in the first) and inside; the older Joinery function drills the wall unit on the return,
  made with none
- Edit cabinet: the wall unit on the return gets a third shelf after it is made, hung, turned and drilled
- Labels: the tall unit has each board's name written on it
- Panel: an end panel on the tall unit's free side and one at the open end of the return run
- Worktop: oak over the base units and the corner cabinet, butt-jointed in the corner, with a
  sink cut-out over the sink base
- Drawing: a shop drawing of every cabinet, a sheet each in the design's drawing Woodwork
- Cut list: written out as CSV (boards with grain and banded edges, and the banding to buy); the sheets from the cutting layouts and the hardware are printed

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
FLOOR_UNIT = {**WHITE, "height": 870, "depth": 560, "plinth": 150, "joints": "confirmat"}
BASE = {**FLOOR_UNIT, "handles": "pull", "place": "auto"}  # auto: by the selected board, a Side R
WALL = {**WHITE, "height": 700, "depth": 320, "shelves": 2, "shelf_pins": True, "handles": "knob",
        "joints": "dowels", "place": "origin"}  # made on the origin and moved, though a Top is selected to match
MATCH = {"match": "selected"}  # sizes from the selected cabinet, and linked to it
WALL_Z = 2100 - 700  # wall units line up with the top of the tall unit

# The floor run, left to right. The tall unit stands on the origin; each of the others is
# placed against the one before it.
FLOOR = [
    ("Tall", {**WHITE, "width": 600, "height": 2100, "depth": 560, "plinth": 150, "shelves": 4, "doors": 1,
              "handles": "pull", "labels": True, "fronts": "inset", "joints": "biscuit"}),
    ("Base 1", {**BASE, "width": 800, "dividers": 1, "doors": 2}),
    ("Base 2", {**BASE, **MATCH, "width": 600, "drawers": 3, "drawer_height": 140, "joints": "cam"}),
    ("Base 3", {**BASE, **MATCH, "kind": "sink", "width": 900, "doors": 2, "joints": "pocket"}),  # under the sink
]
# Wall units hang over the base units: name, left edge, parameters.
HANGING = [
    ("Wall 1", 300, {**WALL, "width": 800, "doors": 2, "joints": "dado"}),
    ("Wall 2", 1100, {**WALL, "width": 600, "doors": 1}),
    ("Wall 3", 1700, {**WALL, "width": 900, "dividers": 2, "doors": 3}),
]
BACK = 280  # the wall: the tall unit is 560 deep and centred on the origin
# The corner cabinet goes right of the last base unit; its return runs forward along the right
# wall, and one more base unit continues it, turned to face into the room.
CORNER = {**FLOOR_UNIT, **MATCH, "width": 1000, "return": 1000, "doors": "hinged left",
          "handles": "pull", "place": "right"}
RIGHT = 300 + sum(params["width"] for _, params in FLOOR[1:]) + CORNER["width"]  # the right wall
RETURN = {**FLOOR_UNIT, **MATCH, "width": 600, "drawers": 1, "doors": 1, "handles": "pull"}
WALL_CORNER = {**WHITE, **MATCH, "width": 1000, "return": 1000, "height": 700, "depth": 320,
               "shape": "diagonal", "shelves": 2, "shelf_pins": True, "doors": "hinged right", "handles": "knob",
               "handle_position": "middle",
               "wall_mount": "cleat", "place": "right"}
WALL_RETURN = {**WALL, **MATCH, "width": 600, "doors": 1, "joints": "none"}  # drilled by Joinery
OPEN = {"Tall Door hinge": 40, "Base 3 Door L hinge": 70, "Base 1 Door L hinge": 100, "Base 1 Door R hinge": 75,
        "Base 2 Drawer 2 slide": 350, "Wall 3 Door 1 hinge": 95, "Wall 3 Door 3 hinge": 60,
        "Corner Door hinge": 60, "Corner Door return fold": 60, "Base 4 Door hinge": 50, "Base 4 Drawer 1 slide": 250,
        "Wall corner Door hinge": 60}


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
    if name == "Tall":  # the run's visible end
        steps += [select("Tall Side L"), build.plugin(approval, "panel", {"kind": "end"})]
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
steps += [select(f"{FLOOR[-1][0]} Side R"), build.plugin(approval, "corner", {**CORNER, "name": "Corner"}),
          select(f"{HANGING[-1][0]} Top"), build.plugin(approval, "corner", {**WALL_CORNER, "name": "Wall corner"})]
# The units on the return are made on the origin (Wall 4 drilled there by the older Joinery
# function, which wants a cabinet square to the axes), then turned a quarter and put at the end
# of the corner cabinet's return.
end = BACK - CORNER["return"]
for name, params, z, like in (("Base 4", RETURN, 0, "Corner"), ("Wall 4", WALL_RETURN, WALL_Z, "Wall corner")):
    to = {"position": [RIGHT - params["depth"] / 2, end - params["width"] / 2, z], "axis": [0, 0, 1], "angle": "-90 deg"}
    drill = [select(f"{name} Top"), build.plugin(approval, "joinery", {"type": "dowel"})] if name == "Wall 4" else []
    steps += [select(f"{like} Top"), build.plugin(approval, "cabinet", {**params, "name": name, "place": "origin"}),
              *drill,
              *([select("Base 4 Side R"), build.plugin(approval, "panel", {"kind": "end"})] if name == "Base 4" else []),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=False),
              step("occurrence.move", occurrence=f"{name}:1", to=to),
              step("occurrence.ground", occurrence=f"{name}:1", grounded=True)]
results = build.cli(steps, "--save", str(OUT))
made = [r["result"] for r in results if isinstance(r, dict) and "bodies" in r.get("result", {})
        and "cabinet" not in r["result"]]  # the cabinets, not the panels

# 2. Oak fronts, and a few doors and a drawer left open.
steps = [kitchen]
fronts = [b for cabinet in made for b in cabinet["bodies"] if b.endswith(" Front") or " Door" in b]
steps += [select(*fronts), build.plugin(approval, "material", {"material": "Oak", "scope": "parts"})]
# An oak worktop over the floor units but the tall one: the back run is piece 1, through to the
# right wall; the return's piece butts against its front edge and covers the end panel. The
# sink's opening is over Base 3, between its rails (100 mm each, the front one 40 mm behind the
# worktop's front edge), measured from piece 1's left end (Base 1's left side).
SINK = {"cutout": "plain", "cutout_length": 760, "cutout_width": 300, "cutout_y": 311,
        "cutout_x": FLOOR[1][1]["width"] + FLOOR[2][1]["width"] + FLOOR[3][1]["width"] / 2}
steps += [select("Base 1 Top", "Base 2 Top", "Base 3 Rail front", "Corner Top", "Base 4 Top"),
          build.plugin(approval, "worktop", {"right_overhang": 18, **SINK})]
# One more shelf in a wall unit that is already hung, has an oak door and is drilled for dowels.
# The last cabinet made: an Edit rebuilds the history after it, which for Wall 2 ran past the
# 30 s limit on CI.
steps += [select("Wall 4 Top"), build.plugin(approval, "edit", {"shelves": 3})]
steps += [step("joint.drive", joint=joint, value=value) for joint, value in OPEN.items()]
steps += [step("select.clear"), step("component.activate", component="root"),
          # A run draws as many sheets as fit in its 30 s; the next carries on.
          *[build.plugin(approval, "drawing", {"scope": "all"})] * 4,
          build.plugin(approval, "cutlist")]
results = build.cli(steps, "--in", str(OUT), "--save", str(OUT))
assert results[-2]["result"]["remaining"] == [], results[-2]
cutlist = results[-1]["result"]

OUT.with_name(OUT.stem + "-cutlist.csv").write_text(cutlist["csv"], encoding="utf-8", newline="\n")
for sheet in cutlist["sheets"]:
    print(f"{sheet['sheets']} sheets of {sheet['thickness']:g} mm {sheet['material']} ({sheet['area_m2']} m2, {sheet['yield_pct']} % yield)")
for part in cutlist["worktops"]:
    print(f"worktop, bought to length: {part['part']} ({part['length']:g} x {part['width']:g} x {part['thickness']:g} mm)")
for part in cutlist["oversize"]:
    print(f"not laid out: {part['part']} ({part['length']:g} x {part['width']:g} mm), {part['reason']}")
for item in cutlist["hardware"]:
    print(item["qty"], "x", item["item"])
print(cutlist["boards"], "boards ->", OUT)
