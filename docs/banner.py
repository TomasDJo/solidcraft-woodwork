"""Draw docs/banner.png, the first picture in the README: a small run of cabinets and the name.

    python build.py          # first: builds the plugin into dist/
    python docs/banner.py

Needs Pillow. The cabinets are built and rendered through the CLI, cut out of the render's
light background, and set on a dark one beside the text.
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
# Both doors of the first cabinet stand open, so its divider, shelves and pin holes show.
OPEN = {"Doors Door L hinge": 115, "Doors Door R hinge": 88, "Drawers Drawer 2 slide": 260,
        "Corner Door return hinge": 70, "Corner Door fold": 75}


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


def banner(model):
    w, h = SIZE
    image = Image.new("RGB", SIZE, "#14110e")
    # A warm glow behind the cabinets.
    glow = Image.new("L", SIZE, 0)
    ImageDraw.Draw(glow).ellipse((w * 0.42, h * 0.05, w * 1.05, h * 0.98), fill=150)
    image.paste(Image.new("RGB", SIZE, "#4a3520"), mask=glow.filter(ImageFilter.GaussianBlur(120)))

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
    subprocess.run(["solidcraft-cli", "snapshot", "--in", str(SCENE), "--out", str(SHOT), "--camera", CAMERA,
                    "--no-grid", "--no-axes", "--width", "2400", "--height", "2000", "--quality", "fine"],
                   check=True, stdout=subprocess.DEVNULL)
    banner(cut_out(Image.open(SHOT).convert("RGB")))
    print(OUT)
