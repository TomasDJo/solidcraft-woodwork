"""Draw Woodwork's pictures for the SolidCraft Workshop: docs/workshop-card.png, the card's (no text:
the card prints the name), and the plugin page's gallery, docs/workshop/*.png. Each is a small
design built through the CLI, posed open and set on docs/banner.py's dark background; the
gallery's newer ones each show one thing, named in a caption on the picture.

    python build.py          # first: builds the plugin into dist/
    python docs/workshop_card.py            all of them
    python docs/workshop_card.py corner     only those named

Needs Pillow, and PyMuPDF for the cut list's page.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "tests"))
from banner import backdrop, build, font, select, step

DIST = HERE.parent / "dist"
SIZE = (1200, 900)

PLY = {"material": "Birch plywood", "height": 720, "depth": 560, "plinth": 120, "plinth_type": "legs",
       "handles": "pull"}
BASE = {"material": "White melamine", "height": 720, "depth": 560, "plinth": 100, "handles": "pull"}


def cam(target, distance, yaw, pitch):
    return json.dumps({"target": target, "yaw": yaw, "pitch": pitch, "distance": distance, "fov": 0.5})


def card(run):
    return [run("settings", {"joints": "finger", "drawer_joints": "finger"}),
            run("cabinet", {**PLY, "name": "Doors", "width": 800, "doors": 2, "shelves": 2, "shelf_pins": True}),
            select("Doors Side R"),
            run("cabinet", {**PLY, "name": "Drawers", "match": "selected", "place": "right", "width": 500,
                            "drawers": 3}),
            select("Doors Top", "Drawers Top"),
            run("worktop", {"dog_holes": "zone", "zone_x": 80, "zone_y": 120, "zone_length": 1100,
                            "zone_width": 300})]


def see_through(run):
    return [run("settings", {"see_through": "on", "fastener_models": "on", "joints": "confirmat"}),
            run("cabinet", {**BASE, "name": "Unit", "width": 600, "doors": 1, "drawers": 1, "shelves": 1})]


def corner(run):
    return [run("corner", {**PLY, "name": "Corner", "corner": "left", "doors": "hinged left"}),
            select("Corner Side R"),
            run("cabinet", {**PLY, "name": "Drawers", "match": "selected", "place": "right", "width": 600,
                            "drawers": 3}),
            select("Corner Top", "Drawers Top"), run("worktop", {})]


def mobile_base(run):
    return [run("cabinet", {**PLY, "name": "Doors", "plinth": 0, "width": 600, "doors": 1}),
            select("Doors Side R"),
            run("cabinet", {**PLY, "name": "Drawers", "plinth": 0, "place": "right", "width": 500, "drawers": 3}),
            select("Doors Side L", "Drawers Side L"), run("base", {"name": "Base"}),
            select("Doors Top", "Drawers Top"), run("worktop", {"dog_holes": "zone", "zone_length": 900})]


def fronts(run):
    return [run("cabinet", {**PLY, "name": "Shaker", "width": 600, "doors": 1, "drawers": 1,
                            "front_style": "shaker"}),
            select("Shaker Side R"),
            run("cabinet", {**PLY, "name": "Glass", "place": "right", "width": 500, "doors": 1,
                            "door_hinge": "right", "shelves": 2, "front_style": "glass"})]


def runners(run):
    return [run("settings", {"slide_type": "strips", "hardware_models": "off"}),
            run("cabinet", {**PLY, "name": "Drawers", "width": 600, "drawers": 3})]


def pilasters(run):
    return [run("settings", {"hardware_models": "off"}),
            run("cabinet", {**PLY, "name": "Unit", "width": 600, "doors": 1, "door_hinge": "right", "shelves": 2, "shelf_pins": True,
                            "shelf_support": "strips"})]


def handleless(run):
    return [run("cabinet", {**PLY, "name": "Unit", "width": 600, "drawers": 1, "doors": 1, "drawers_at": "top",
                            "handles": "groove", "door_handles": "groove"})]


def integrated(run):
    return [run("settings", {"drawer_front": "integrated", "drawer_joints": "finger", "hardware_models": "off"}),
            run("cabinet", {**PLY, "name": "Drawers", "width": 600, "drawers": 3, "handles": "tab"})]


def cut_list(run):
    return [run("settings", {"prices": "Birch plywood 18 = 80; Birch plywood 6 = 30; Banding = 0.5; Hinge = 3; "
                                       "Birch plywood 12 = 55; Drawer slide = 12; Adjustable leg = 2; Pull = 4; Dowel = 0.05"}),
            run("cabinet", {**PLY, "name": "Doors", "width": 800, "doors": 2, "shelves": 2}),
            select("Doors Side R"),
            run("cabinet", {**PLY, "name": "Drawers", "place": "right", "width": 500, "drawers": 3})]


# out, steps (or a design to render as it is), fronts made oak, joints posed open, camera and any
# more snapshot options, and the caption
SHOTS = [
    (HERE / "workshop-card.png", card, True,
     {"Doors Door L hinge": 105, "Doors Door R hinge": 70, "Drawers Drawer 3 slide": 460},
     cam([0, -250, 450], 4300, -0.55, 0.42), [], None),
    (HERE / "workshop" / "kitchen.png", HERE.parent / "examples" / "kitchen.sc", False, {},
     cam([1650, -500, 900], 9000, 0.35, 0.3), [], None),
    (HERE / "workshop" / "see-through.png", see_through, False,
     {"Unit Door hinge": 100, "Unit Drawer 1 slide": 300}, cam([0, -200, 420], 2600, -0.6, 0.3), [], None),
    (HERE / "workshop" / "corner.png", corner, True,
     {"Corner Door return hinge": 100, "Corner Door fold": 0, "Drawers Drawer 1 slide": 300},
     cam([300, -300, 450], 3900, -0.95, 0.55), [], None),
    (HERE / "workshop" / "mobile-base.png", mobile_base, True, {"Drawers Drawer 1 slide": 300},
     cam([0, -250, 450], 3600, -0.6, 0.3), [], "Mobile base on casters"),
    (HERE / "workshop" / "fronts.png", fronts, True, {"Glass Door hinge": 60},
     cam([0, -250, 450], 3600, -0.5, 0.3), [], "Shaker and glass fronts"),
    (HERE / "workshop" / "runners.png", runners, True, {"Drawers Drawer 3 slide": 350},
     cam([0, -300, 450], 2600, -0.9, 0.4), ["--body", "Drawers Side R:opacity=0.2"],
     "Drawers on wooden runners, false back"),
    (HERE / "workshop" / "pilasters.png", pilasters, True, {"Unit Door hinge": 100},
     cam([0, -250, 450], 3000, 0.45, 0.25), [], "Pilaster strips"),
    (HERE / "workshop" / "handleless.png", handleless, True, {"Unit Drawer 1 slide": 200},
     cam([0, -250, 450], 2600, -0.6, 0.35), [], "Handle-less: finger groove"),
    (HERE / "workshop" / "integrated.png", integrated, False, {"Drawers Drawer 3 slide": 400},
     cam([0, -300, 450], 2600, -0.9, 0.45), [], "Integrated, finger-jointed drawers"),
    # the cut list's PDF, its first page, not a render
    (HERE / "workshop" / "cut-list.png", cut_list, False, {}, None, [], "Cut list with cost estimate"),
]


def scene(steps, oak, joints, path):
    """Builds the design from `steps` into `path`, its fronts oak and its joints driven open."""
    approval = build.cli([step("plugin.inspect", path=str(build.PKG))])[0]["approval"]
    results = build.cli(steps(lambda entry, params: build.plugin(approval, entry, params)), "--save", str(path))
    bodies = [b for r in results if isinstance(r, dict) for b in r.get("result", {}).get("bodies", [])]
    fronts = [b for b in bodies if b.endswith(" Front") or " Door" in b]
    posed = [select(*fronts), build.plugin(approval, "material", {"material": "Oak", "scope": "parts"})] if oak else []
    posed += [step("joint.drive", joint=joint, value=value) for joint, value in joints.items()]
    build.cli(posed + [step("select.clear")], "--in", str(path), "--save", str(path))


def unlabelled(design, path):
    """A copy of `design` in `path` with the board labels (sketches `<board> label`) hidden."""
    names = re.findall(r'"([^"]+ label)"', json.dumps(build.cli([step("document.inspect")], "--in", str(design))))
    build.cli([step("browser.visibility", visible=False, items=sorted(set(names)))], "--in", str(design),
              "--save", str(path))


def render(design, camera, options, out, caption):
    """The design from `camera` on the dark background, the model centred with a margin."""
    shot = DIST / "workshop-shot.png"
    subprocess.run([build.CLI, "snapshot", "--in", str(design), "--out", str(shot), "--camera", camera,
                    "--no-grid", "--no-axes", "--transparent", "--width", "2400", "--height", "1800",
                    "--quality", "fine", *options], check=True, stdout=subprocess.DEVNULL)
    model = Image.open(shot).convert("RGBA")
    finish(model.crop(model.getbbox()), out, caption)


def page(steps, out, caption):
    """The cut list's PDF of the design from `steps`, its first two pages (the parts, then the
    banding, hardware and cost), side by side as the picture's model."""
    import fitz
    from helpers import own_profile
    folder = DIST / "workshop-cutlist"
    with own_profile(folder) as install:
        install["params"]["approve"]["folders"].append({"name": "cutlists", "path": str(folder), "read": False,
                                                        "write": True})
        approval = build.cli([step("plugin.inspect", path=str(build.PKG))])[0]["approval"]
        run = lambda entry, params: build.plugin(approval, entry, params)
        build.cli([install, *steps(run), run("cutlist", {"file": "Workbench"})])
    sheets = [p.get_pixmap(dpi=110) for p in list(fitz.open(folder / "Workbench.pdf"))[:2]]
    model = Image.new("RGBA", (sheets[0].width * 2 + 60, sheets[0].height))
    for i, sheet in enumerate(sheets):
        model.paste(Image.frombytes("RGB", (sheet.width, sheet.height), sheet.samples), (i * (sheet.width + 60), 0))
    finish(model, out, caption)


def finish(model, out, caption):
    """`model` on the dark background, centred with a margin, under `caption` when there is one."""
    w, h = SIZE
    image = backdrop(SIZE, (0.08, 0.06, 0.92, 0.96)).convert("RGBA")
    top = 150 if caption else 0
    model.thumbnail((int(w * 0.86), int((h - top) * 0.82)), Image.LANCZOS)
    image.alpha_composite(model, ((w - model.width) // 2, top + (h - top - model.height) // 2))
    if caption:  # as banner.py's name, smaller
        draw = ImageDraw.Draw(image)
        draw.text((60, 40), caption, font=font(46, bold=True), fill="#f6f1ea")
        draw.rectangle((62, 112, 122, 116), fill="#d99a4e")
    out.parent.mkdir(exist_ok=True)
    image.convert("RGB").quantize(256).save(out, optimize=True)
    print(out, out.stat().st_size // 1024, "KB")


if __name__ == "__main__":
    for out, steps, oak, joints, camera, options, caption in SHOTS:
        if sys.argv[1:] and out.stem not in sys.argv[1:]:
            continue
        if camera is None:
            page(steps, out, caption)
            continue
        design = DIST / f"workshop-{out.stem}.sc"
        if callable(steps):
            scene(steps, oak, joints, design)
        else:
            unlabelled(steps, design)
        render(design, camera, options, out, caption)
