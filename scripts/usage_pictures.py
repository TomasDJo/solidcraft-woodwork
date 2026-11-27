"""Draws the usage guide's pictures, docs/images/<name>.png, from small designs built with the
plugin in dist/woodwork (build it first: python build.py, or python -c "import build; build.build()").

    python scripts/usage_pictures.py            all of them
    python scripts/usage_pictures.py plinths    only those named

One list, SHOTS: name, caption, the steps, and the camera (and any snapshot options). Adding,
changing or removing a picture is one entry there and its line in docs/usage.md
(`![caption](images/<name>.png)`, which tests/test_docs.py checks). Rerun after a change that
alters how things look. solidcraft-cli renders one picture at a time. It never installs the plugin:
every step is plugin.run with the package's path, as the tests do.
"""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
from build import CLI, PKG, cli, flat, plugin  # noqa: E402

OUT = ROOT / "docs" / "images"
SIZE = (800, 560)
_approval = []


def run(entry, params=None):
    """A plugin.run step (with Woodwork settings around it for a moved field, as in the tests)."""
    if not _approval:
        _approval.append(cli([{"command": "plugin.inspect", "params": {"path": str(PKG)}}])[0]["approval"])
    return plugin(_approval[0], entry, params)


def pick(*names):
    return {"command": "select.set", "params": {"items": [{"type": "body", "name": n} for n in names]}}


def drive(joint, value):
    return {"command": "joint.drive", "params": {"joint": joint, "value": value}}


def at(name, x, y=0):
    """Moves a cabinet made on the origin along the floor."""
    return [{"command": "occurrence.ground", "params": {"occurrence": f"{name}:1", "grounded": False}},
            {"command": "occurrence.move", "params": {"occurrence": f"{name}:1", "translate": [x, y, 0]}}]


def row(entry, variants, step=750):
    """One cabinet per variant, side by side: A, B, C..."""
    steps = []
    for i, params in enumerate(variants):
        name = "ABCDEF"[i]
        steps += [lambda name=name, params=params: run(entry, {"name": name, **params}), *at(name, step * i)]
    return steps


def cam(target, distance, yaw=-0.65, pitch=0.22):
    return ["--camera", json.dumps({"target": target, "yaw": yaw, "pitch": pitch, "distance": distance})]


ONE = cam([0, 0, 380], 1700)
TWO = cam([375, 0, 380], 2500, yaw=-0.45)
FOUR = cam([1125, 0, 420], 3300, yaw=-0.3, pitch=0.15)
CORNER = cam([0, -100, 330], 2100)
BASE = {"plinth": 100, "handles": "knob"}

# name, caption, steps (a step may be a function, called when the picture is drawn), snapshot options
SHOTS = [
    ("cabinet", "A cabinet with two doors, a shelf and knobs, on a plinth.",
     [lambda: run("cabinet", {"doors": 2, **BASE})], ONE),
    ("drawers", "Three drawers: the top one 140 mm, the others share the rest.",
     [lambda: run("cabinet", {"drawers": 3, "drawer_height": 140, "plinth": 100, "handles": "pull"})], ONE),
    ("drawers-doors", "A drawer over two doors, on a rail.",
     [lambda: run("cabinet", {"doors": 2, "drawers": 1, "plinth": 100, "handles": "pull"})], ONE),
    ("fronts", "Fronts: overlay (left) cover the carcass edges, inset (right) sit inside it.",
     row("cabinet", [{"doors": 2, **BASE}, {"doors": 2, "fronts": "inset", **BASE}]), TWO),
    ("grip-edges", "Grip edges on the doors (left) and finger grooves on the drawers (right): no handles.",
     [*row("cabinet", [{"doors": 2, "plinth": 100, "handles": "grip"},
                       {"drawers": 3, "plinth": 100, "handles": "groove"}]), drive("A Door R hinge", 100)],
     cam([375, -100, 420], 2000, yaw=-0.5, pitch=0.4)),
    ("grooved-fronts", "Grooved fronts: vertical square grooves on the doors (left), round horizontal ones on the drawers (right).",
     row("cabinet", [{"doors": 2, **BASE, "front_style": "vertical"},
                     {"drawers": 3, "plinth": 100, "handles": "grip", "front_style": "horizontal", "groove_shape": "round",
                      "groove_spacing": 40}]), TWO),
    ("shaker-fronts", "Shaker fronts: a frame of stiles and rails round a thinner panel, on doors and drawers.",
     row("cabinet", [{"doors": 2, **BASE, "front_style": "shaker"},
                     {"drawers": 2, **BASE, "front_style": "shaker"}]), TWO),
    ("glass-fronts", "Glass doors: the shaker frame round a glass pane, the shelf behind it showing.",
     [lambda: run("cabinet", {"doors": 2, **BASE, "front_style": "glass"})], ONE),
    ("open", "Doors and drawers open on their joints, with the hinge and slide models.",
     [lambda: run("cabinet", {"doors": 1, "drawers": 1, **BASE}),
      drive("Cabinet Door hinge", 100), drive("Cabinet Drawer 1 slide", 350)], cam([0, -200, 380], 2400, yaw=-0.9)),
    ("hinge-model", "A hinge model: the cup and its rim in the door, the boss and arm, and the cross-shaped plate on the side.",
     [lambda: run("cabinet", {"doors": 1}), drive("Cabinet Door hinge", 100)],
     cam([-265, -270, 610], 700, yaw=-0.9, pitch=0.3)),
    ("integrated-fronts", "Integrated drawer fronts with finger joints: the box's own front is the front.",
     [lambda: run("settings", {"drawer_front": "integrated", "drawer_joints": "finger"}),
      lambda: run("cabinet", {"drawers": 3, "plinth": 100, "handles": "pull"}), drive("Cabinet Drawer 2 slide", 350)],
     cam([0, -250, 420], 1600, yaw=-0.7, pitch=0.35)),
    ("divider-hinges", "Three doors: the end ones hinge on the sides, the middle one on a divider with half overlay hinges.",
     [lambda: run("cabinet", {"width": 900, "height": 720, "depth": 320, "dividers": 2, "doors": 3, "door_hinge": "right", "shelves": 1,
                              "handles": "pull"}),
      drive("Cabinet Door 1 hinge", 30), drive("Cabinet Door 2 hinge", 95)], cam([0, -200, 360], 1450, yaw=0.6, pitch=0.3)),
    ("double-dividers", "Double dividers: the middle door hangs on its own board with standard hinges.",
     [lambda: run("settings", {"double_dividers": "yes"}),
      lambda: run("cabinet", {"width": 900, "height": 720, "depth": 320, "dividers": 2, "doors": 3, "door_hinge": "right",
                              "shelves": 1, "handles": "pull"}),
      drive("Cabinet Door 1 hinge", 30), drive("Cabinet Door 2 hinge", 95)], cam([0, -200, 360], 1450, yaw=0.6, pitch=0.3)),
    ("bottom-tab", "Bottom tab: a rounded tab under each drawer front's lower edge to pull by.",
     [lambda: run("settings", {"drawer_bottom": 12}),
      lambda: run("cabinet", {"drawers": 3, "plinth": 100, "handles": "tab"}), drive("Cabinet Drawer 2 slide", 250)],
     cam([0, -300, 150], 900, yaw=-0.3, pitch=0.15)),
    ("sink", "A sink base: two rails instead of a top, no shelves.",
     [lambda: run("cabinet", {"kind": "sink", "doors": 2, **BASE})], cam([0, 0, 380], 2100, pitch=0.6)),
    ("plinths", "Plinth types: board, notched sides, legs and casters.",
     row("cabinet", [{"doors": 1, "plinth": 100, "plinth_type": t} for t in ("board", "notch", "legs", "casters")]),
     FOUR),
    ("wall-mount", "A wall cabinet on a French cleat, cut through: the back stands in front of the cleat.",
     [lambda: run("cabinet", {"depth": 320, "wall_mount": "cleat", "doors": 1})],
     [*cam([0, 0, 360], 1400, yaw=-1.2), "--section", "YZ"]),
    ("joinery", "Dowels at build, with fastener models on and see-through boards.",
     [lambda: run("settings", {"see_through": "on", "fastener_models": "on"}), lambda: run("cabinet", {})],
     ONE),
    ("wood-runners", "Wooden runners: each drawer's bottom runs in a groove in the sides.",
     [lambda: run("settings", {"slide_type": "wood", "drawer_bottom": 9}), lambda: run("cabinet", {"drawers": 3})],
     [*cam([-100, 0, 420], 1000, yaw=-0.3, pitch=0.1), "--section", "y=150,flip"]),
    ("strip-runners", "Strip runners: each drawer's bottom rests on a wooden strip on each side, no groove.",
     [lambda: run("settings", {"slide_type": "strips", "drawer_bottom": 9}), lambda: run("cabinet", {"drawers": 3})],
     [*cam([-100, 0, 420], 1000, yaw=-0.3, pitch=0.1), "--section", "y=150,flip"]),
    ("corner", "An L-shaped corner cabinet with a pair of folding doors.",
     [lambda: run("corner", {"doors": "hinged left", **BASE})], CORNER),
    ("corner-diagonal", "A diagonal corner cabinet: one door across the corner.",
     [lambda: run("corner", {"shape": "diagonal", "doors": "hinged left", **BASE})], CORNER),
    ("corner-split", "Split boards: the L-shaped top, bottom and shelves in two dowelled pieces.",
     [lambda: run("corner", {"boards": "split", "plinth": 100})], cam([0, -100, 330], 2100, pitch=0.7)),
    ("run", "A run: each cabinet placed against the side of the one picked.",
     [lambda: run("cabinet", {"name": "A", "doors": 2, **BASE}), pick("A Side R"),
      lambda: run("cabinet", {"name": "B", "place": "auto", "drawers": 3, "plinth": 100, "handles": "pull"}),
      pick("B Side R"), lambda: run("cabinet", {"name": "C", "place": "auto", "doors": 1, **BASE})],
     cam([600, 0, 380], 3000, yaw=-0.4)),
    ("line", "Along a line: two cabinets against a wall's face, from its left and right ends.",
     [{"command": "solid.box", "params": {"name": "Wall", "body_name": "Wall", "corner": [-1000, 300, 0],
                                          "length": 2000, "width": 100, "height": 1200}},
      lambda: run("cabinet", {"name": "A", "doors": 2, **BASE, "place": "line", "offset": 150,
                              "line": [{"type": "face", "body": "Wall", "index": 3}]}),
      lambda: run("cabinet", {"name": "B", "drawers": 3, "plinth": 100, "handles": "pull", "place": "line",
                              "start_at": "right", "offset": 150,
                              "line": [{"type": "face", "body": "Wall", "index": 3}]})],
     cam([0, 0, 450], 3200, yaw=-0.4)),
    ("line-fill", "Width from line: one cabinet fills the wall between two side walls.",
     [*[{"command": "solid.box", "params": {"name": n, "body_name": n, "corner": c, "length": l, "width": w, "height": 1200}}
        for n, c, l, w in (("Wall", [-800, 300, 0], 1600, 100), ("Wall L", [-900, -400, 0], 100, 800),
                           ("Wall R", [800, -400, 0], 100, 800))],
      lambda: run("cabinet", {"name": "A", "doors": 2, "dividers": 1, **BASE, "place": "line", "width_from_line": True,
                              "line": [{"type": "face", "body": "Wall", "index": 3}]})],
     cam([0, 0, 450], 3000, yaw=-0.35)),
    ("match-run", "A matched run: three cabinets sized by their shared group, Run_A_height set to 850 mm.",
     [lambda: run("cabinet", {"name": "A", "doors": 2, **BASE}), pick("A Side R"),
      lambda: run("cabinet", {"name": "B", "place": "auto", "match": "selected", "drawers": 3, "handles": "pull"}),
      pick("B Side R"),
      lambda: run("cabinet", {"name": "C", "place": "auto", "match": "selected", "doors": 1, "handles": "knob"}),
      {"command": "parameters.change", "params": {"name": "Run_A_height", "expression": "850 mm"}}],
     cam([600, 0, 420], 3000, yaw=-0.4)),
    ("cost", "The cut list's PDF with a cost estimate from the prices in Woodwork settings.",
     [lambda: run("cabinet", {"name": "A", "doors": 2, **BASE}), pick("A Side R"),
      lambda: run("cabinet", {"name": "B", "place": "auto", "drawers": 3, "plinth": 100, "handles": "pull"}),
      lambda: run("settings", {"prices": "Birch plywood 18 = 80; Birch plywood 6 = 35; Banding = 0.5; "
                                         "Hinge = 3.2; Drawer slide = 12; Shelf pin = 0.1; Knob = 2.5"})],
     "pdf"),
    ("base", "A mobile base: one frame on locking casters under a run of two cabinets.",
     [lambda: run("cabinet", {"name": "A", "doors": 2, "handles": "knob", "height": 760}), pick("A Side R"),
      lambda: run("cabinet", {"name": "B", "place": "auto", "drawers": 3, "handles": "pull", "height": 760}),
      pick("A Side L", "B Side L"), lambda: run("base", {"name": "Bench"})],
     cam([300, 0, 500], 3000, yaw=-0.4, pitch=0.12)),
    ("base-bolts", "Connector bolts: four 9 mm holes through the touching sides of two cabinets on a base, seen through the boards.",
     [lambda: run("cabinet", {"name": "A", "width": 600, "shelves": 0, **BASE}), pick("A Side R"),
      lambda: run("cabinet", {"name": "B", "place": "auto", "width": 600, "shelves": 0, **BASE}),
      pick("A Side L", "B Side L"), lambda: run("base", {"name": "Bench"}),
      lambda: run("settings", {"see_through": "on"})],
     cam([300, 0, 550], 1400, yaw=-0.9, pitch=0.15)),
    ("base-screws", "Frame screws: two holes down through the cabinet's bottom into each frame board under it, seen through the boards.",
     [lambda: run("cabinet", {"name": "A", "width": 600, "shelves": 0, "doors": 0, **BASE, "plinth": 0}), pick("A Side L"),
      lambda: run("base", {"name": "Bench"}), lambda: run("settings", {"see_through": "on"})],
     cam([0, 0, 200], 1500, yaw=-0.5, pitch=0.9)),
    ("tool-bolts", "Bolt holes for the tool's feet: four 8 mm holes through the tool's shelf, seen through the opening.",
     [lambda: run("cabinet", {"name": "S", "width": 900, "depth": 650, "shelves": 0, "doors": 2, **BASE}), pick("S Top"),
      lambda: run("worktop", {"tool": "benchtop", "tool_width": 640, "tool_depth": 540, "tool_height": 300,
                              "tool_x": 450, "tool_y": 345, "bolt_x": 400, "bolt_y": 300})],
     cam([0, 0, 450], 1600, yaw=-0.3, pitch=1.45)),
    ("side-wing", "A side wing: the worktop carried on 600 mm past the run's right end, on two legs.",
     [lambda: run("cabinet", {"name": "S", "width": 900, "shelves": 0, "doors": 2, **BASE}), pick("S Top"),
      lambda: run("worktop", {"wing": "right", "wing_length": 600})],
     cam([300, 0, 450], 2300)),
    ("panel", "Panel: an end panel on the right, a filler with its return on the left.",
     [lambda: run("cabinet", {"name": "A", "doors": 2, **BASE}), pick("A Side R"), lambda: run("panel", {"kind": "end"}),
      pick("A Side L"), lambda: run("panel", {"kind": "filler", "width": 50})], ONE),
    ("material", "Material: white melamine doors on a birch plywood cabinet.",
     [lambda: run("cabinet", {"name": "A", "doors": 2, **BASE}), pick("A Door L", "A Door R"),
      lambda: run("material", {"material": "White melamine", "scope": "parts"}),
      {"command": "select.clear", "params": {}}], ONE),
    ("worktop", "Worktop over a corner, a sink base and a cabinet, with a sink cut-out.",
     [lambda: run("corner", {"name": "K", "corner": "left", "doors": "hinged left", **BASE}), pick("K Side L"),
      lambda: run("cabinet", {"name": "C", "place": "auto", "kind": "sink", "doors": 2, **BASE}), pick("K Side R"),
      lambda: run("cabinet", {"name": "A", "place": "auto", "doors": 2, **BASE}), pick("K Top", "A Top", "C Side R"),
      lambda: run("worktop", {"cutout": "plain", "cutout_x": 300, "cutout_y": 300})], cam([300, -300, 450], 2700)),
    ("bench", "A bench top with a zone of dog holes and a rebated cut-out.",
     [lambda: run("cabinet", {"name": "A", "width": 1800, "height": 900, "doors": 2}), pick("A Top"),
      lambda: run("worktop", {"name": "Bench top", "material": "Birch plywood", "dog_holes": "zone", "zone_x": 100,
                              "zone_y": 100, "zone_length": 800, "zone_width": 400, "cutout": "rebated",
                              "cutout_length": 300, "cutout_width": 200, "cutout_x": 1400, "cutout_y": 300})],
     cam([0, 0, 450], 3200, pitch=0.5)),
]


def draw_pdf(name, steps):
    """The cut list saved as a PDF (in a profile of its own, with its folder approved), its
    hardware and cost tables drawn with PyMuPDF."""
    import fitz
    sys.path.insert(0, str(ROOT / "tests"))
    from helpers import own_profile
    folder = ROOT / "dist" / "cost-picture"
    with own_profile(folder) as install:
        install["params"]["approve"]["folders"].append(
            {"name": "cutlists", "path": str(folder), "read": False, "write": True})
        steps = [step() if callable(step) else step for step in steps]
        cli([install, *flat(steps), run("cutlist", {"file": "Cost", "diagrams": "no"})])
        page = next(p for p in fitz.open(folder / "Cost.pdf") if p.search_for("Hardware"))
        top = page.search_for("Hardware")[0].y0 - 12
        bottom = max(b[3] for b in page.get_text("blocks")) + 12
        clip = fitz.Rect(0, top, page.rect.width, bottom)
        shot = page.get_pixmap(clip=clip, matrix=fitz.Matrix(SIZE[0] / page.rect.width, SIZE[0] / page.rect.width))
        png = folder / "shot.png"
        shot.save(png)
    from PIL import Image
    Image.open(png).convert("RGB").quantize(128).save(OUT / f"{name}.png", optimize=True)
    print(f"{name}.png  {(OUT / f'{name}.png').stat().st_size // 1024} KB")


def draw(name, steps, options):
    if options == "pdf":
        return draw_pdf(name, steps)
    script = flat([step() if callable(step) else step for step in steps])
    with tempfile.TemporaryDirectory(dir=ROOT / "dist") as folder:
        path, png = Path(folder) / "script.json", Path(folder) / "shot.png"
        path.write_text(json.dumps(script), encoding="utf-8")
        subprocess.run([CLI, "snapshot", str(path), "--out", str(png), "--width", str(SIZE[0]),
                        "--height", str(SIZE[1]), "--no-axes", *options], cwd=ROOT, check=True)
        # A small PNG: 128 colours are plenty for flat-shaded boards.
        from PIL import Image
        Image.open(png).convert("RGB").quantize(128).save(OUT / f"{name}.png", optimize=True)
    print(f"{name}.png  {(OUT / f'{name}.png').stat().st_size // 1024} KB")


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    wanted = sys.argv[1:]
    for name, _, steps, options in SHOTS:
        if not wanted or name in wanted:
            draw(name, steps, options)
