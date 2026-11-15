"""Shared by the end-to-end tests: steps for a CLI script, and readers for its results."""
from build import ROOT, cli, plugin  # noqa: F401 (the tests use them through `import *`)

MEASURE = {"command": "inspect.measure", "params": {}}


def select(name):
    return {"command": "select.set", "params": {"items": [{"type": "body", "name": name}]}}


def pick(*names):
    return {"command": "select.set", "params": {"items": [{"type": "body", "name": n} for n in names]}}


def drive(joint, value):
    return {"command": "joint.drive", "params": {"joint": joint, "value": value}}


def boxes(measure):
    """Each body's box: x0, y0, z0, x1, y1, z1 in mm."""
    return {b["name"]: [round(v, 3) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in measure["bodies"]}


def cylinders(measure, name):
    """How many round faces a body has: its holes."""
    return next(b for b in measure["bodies"] if b["name"] == name)["face_types"].get("cylinder", 0)


def volume(measure, name):
    return round(next(b for b in measure["bodies"] if b["name"] == name)["volume_mm3"])
