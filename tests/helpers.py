"""Shared by the end-to-end tests: steps for a CLI script, and readers for its results."""
import contextlib
import json
import os
import shutil
import threading

from build import LOCAL, PKG, ROOT, cli, env, plugin  # noqa: F401 (the tests use them through `import *`)

MEASURE = {"command": "inspect.measure", "params": {}}
# The tests with a profile share dist/profile and the folders they empty: one at a time.
PROFILE = threading.Lock()


@contextlib.contextmanager
def own_profile(*folders):
    """A SolidCraft profile of its own in dist/, so installing leaves the user's alone. Empties
    `folders` too. Yields the install step for the built package, folder grants to add. Only this
    thread's solidcraft-cli runs see the profile: those through `cli`, or given `env=env()`."""
    with PROFILE:
        profile = ROOT / "dist" / "profile"
        for path in (profile, *folders):
            shutil.rmtree(path, ignore_errors=True)
            path.mkdir(parents=True)
        keys = ("LOCALAPPDATA", "APPDATA", "XDG_DATA_HOME", "XDG_CONFIG_HOME")
        LOCAL.env = os.environ | {key: str(profile) for key in keys}
        version = json.loads((PKG / "plugin.json").read_text(encoding="utf-8"))["version"]
        install = {"command": "plugin.install", "params": {
            "path": str(ROOT / "dist" / f"woodwork-{version}.zip"), "replace": True,
            "approve": {"command_groups": [], "folders": []}}}
        try:
            yield install
        finally:
            del LOCAL.env


def select(name):
    return {"command": "select.set", "params": {"items": [{"type": "body", "name": name}]}}


def pick(*names):
    return {"command": "select.set", "params": {"items": [{"type": "body", "name": n} for n in names]}}


def drive(joint, value):
    return {"command": "joint.drive", "params": {"joint": joint, "value": value}}


def boards(measure):
    """The bodies of a measure through the CLI but the hardware models (`HW hinge`, ...: their
    definitions and copies), which the plugin's own measures leave out."""
    return [b for b in measure["bodies"] if not b["name"].startswith("HW ")]


def boxes(measure, models=False):
    """Each body's box: x0, y0, z0, x1, y1, z1 in mm; the hardware models' too with `models`."""
    bodies = measure["bodies"] if models else boards(measure)
    return {b["name"]: [round(v, 3) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in bodies}


def cylinders(measure, name):
    """How many round faces a body has: its holes."""
    return next(b for b in measure["bodies"] if b["name"] == name)["face_types"].get("cylinder", 0)


def volume(measure, name):
    return round(next(b for b in measure["bodies"] if b["name"] == name)["volume_mm3"])


def faces(body):
    return {"command": "model.faces", "params": {"body": body}}


def handle_heights(faces):
    """The heights of the 5 mm screw holes in a door, from `model.faces`: lowest first."""
    holes = [f for f in faces["faces"] if abs((f.get("cylinder") or {}).get("radius", 0) - 2.5) < 0.01]
    return sorted(round(f["centroid"][2], 3) for f in holes)
