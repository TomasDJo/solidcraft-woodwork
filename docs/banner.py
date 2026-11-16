"""Draw the README's pictures on one dark background: docs/banner.png, a small run of cabinets
beside the name, and docs/kitchen.png, the sample kitchen in examples/kitchen.sc.

    python build.py          # first: builds the plugin into dist/
    python docs/banner.py

Needs Pillow. The cabinets are rendered through the CLI, cut out of the render's light
background, and set on the dark one.
"""
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE.parent))
import build

SCENE = HERE.parent / "dist" / "banner.sc"
SHOT = HERE.parent / "dist" / "banner-shot.png"
OUT = HERE / "banner.png"
SIZE = (1280, 640)
KITCHEN = HERE.parent / "examples" / "kitchen.sc"
KITCHEN_CAMERA = '{"target":[1800,-350,1000],"yaw":0.22,"pitch":0.33,"distance":5600,"fov":0.6}'
CAMERA = '{"target":[1100,-350,400],"yaw":0.3,"pitch":0.4,"distance":6800,"fov":0.5}'

UNIT = {"material": "White melamine", "height": 870, "depth": 560, "plinth": 120, "handles": "pull"}
NEXT = {**UNIT, "match": "selected", "place": "right"}
# Left to right; each is placed against the one before it, the last along the corner's return.
RUN = [
    ("cabinet", "Doors", {**UNIT, "width": 900, "dividers": 1, "shelves": 2, "shelf_pins": True, "doors": 2}),
    ("cabinet", "Drawers", {**NEXT, "width": 500, "drawers": 3}),
    ("corner", "Corner", {**NEXT, "corner": "right", "width": 900, "return": 900, "doors": "hinged right"}),
    ("cabinet", "End", {**NEXT, "width": 450, "doors": 1}),
]
# The first cabinet has its left door wide open and its right one ajar: shelves, divider and pin holes show.
OPEN = {"Doors Door L hinge": 115, "Doors Door R hinge": 15, "Drawers Drawer 2 slide": 260,
        "Corner Door return hinge": 40, "Corner Door fold": 75}


def step(command, **params):
    return {"command": command, "params": params}


def select(*bodies):
    return step("select.set", items=[{"type": "body", "name": body} for body in bodies])


def scene():
    approval = build.cli([step("plugin.inspect", path=str(build.PKG))])[0]["approval"]
    steps = []
    for i, (entry, name, params) in enumerate(RUN):
        if i:
            steps.append(select(f"{RUN[i - 1][1]} Top"))
        steps.append(build.plugin(approval, entry, {**params, "name": name}))
    results = build.cli(steps, "--save", str(SCENE))
    bodies = [b for r in results if isinstance(r, dict) for b in r.get("result", {}).get("bodies", [])]
    fronts = [b for b in bodies if b.endswith(" Front") or " Door" in b]
    steps = [select(*fronts), build.plugin(approval, "material", {"material": "Oak", "scope": "parts"})]
    steps += [step("joint.drive", joint=joint, value=value) for joint, value in OPEN.items()]
    build.cli(steps + [step("select.clear")], "--in", str(SCENE), "--save", str(SCENE))


def cut_out(shot):
    """The render without its background, which is one colour per row: the left edge's."""
    edge = shot.crop((0, 0, 1, shot.height)).resize(shot.size)
    r, g, b = ImageChops.difference(shot, edge).split()
    far = ImageChops.lighter(ImageChops.lighter(r, g), b)
    alpha = far.point(lambda v: 0 if v < 8 else 255).filter(ImageFilter.MinFilter(3))
    shot = shot.convert("RGBA")
    shot.putalpha(alpha)
    return shot.crop(shot.getbbox())


def font(size, bold=False):
    for name in ("segoeuib.ttf" if bold else "segoeui.ttf", "DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default(size)


def shot(design, camera, size):
    """The design rendered at `size` from `camera`, cut out of its background."""
    subprocess.run(["solidcraft-cli", "snapshot", "--in", str(design), "--out", str(SHOT), "--camera", camera,
                    "--no-grid", "--no-axes", "--width", str(size[0]), "--height", str(size[1]), "--quality", "fine"],
                   check=True, stdout=subprocess.DEVNULL)
    return cut_out(Image.open(SHOT).convert("RGB"))


def backdrop(size, glow_at):
    """The dark background, with a warm glow in the box `glow_at` (fractions of the size)."""
    w, h = size
    x0, y0, x1, y1 = glow_at
    image = Image.new("RGB", size, "#14110e")
    glow = Image.new("L", size, 0)
    ImageDraw.Draw(glow).ellipse((w * x0, h * y0, w * x1, h * y1), fill=150)
    image.paste(Image.new("RGB", size, "#4a3520"), mask=glow.filter(ImageFilter.GaussianBlur(w // 10)))
    return image


def kitchen():
    size = (1400, 900)
    model = shot(KITCHEN, KITCHEN_CAMERA, (2800, 1800))
    model.thumbnail((int(size[0] * 0.9), int(size[1] * 0.86)), Image.LANCZOS)
    image = backdrop(size, (0.1, 0.05, 0.9, 0.95))
    image.paste(model, ((size[0] - model.width) // 2, (size[1] - model.height) // 2), model)
    image.save(HERE / "kitchen.png")


def banner(model):
    w, h = SIZE
    image = backdrop(SIZE, (0.42, 0.05, 1.05, 0.98))

    box = (int(w * 0.5), int(h * 0.86))
    model.thumbnail(box, Image.LANCZOS)
    at = (w - model.width - 36, (h - model.height) // 2)
    image.paste(model, at, model)

    draw = ImageDraw.Draw(image)
    x = 76
    draw.text((x + 2, 176), "SOLIDCRAFT PLUGIN", font=font(22, bold=True), fill="#d99a4e", spacing=4)
    draw.text((x - 3, 206), "Woodwork", font=font(92, bold=True), fill="#f6f1ea")
    draw.rectangle((x + 2, 338, x + 74, 343), fill="#d99a4e")
    draw.text((x + 2, 366), "Parametric cabinets, joinery\nand cut lists", font=font(30), fill="#b9ada0", spacing=10)
    image.save(OUT)


if __name__ == "__main__":
    scene()
    banner(shot(SCENE, CAMERA, (2400, 2000)))
    kitchen()
    print(OUT, HERE / "kitchen.png")
