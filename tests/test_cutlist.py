"""Saving the cut list as a file, in a folder approved at install."""
import json
import os
import shutil

from helpers import *


def test_a_board_turned_in_its_sketch(approval):
    # issue 39: a 150 x 800 rectangle drawn at 15 degrees and extruded 18 mm, listed as its bounding box
    board = [{"command": "sketch.create", "params": {"plane": "XY"}},
             {"command": "sketch.rectangle.three_point",
              "params": {"p0": [0, 0], "p1": [772.741, 207.055], "p2": [733.918, 351.944]}},
             {"command": "sketch.finish", "params": {}},
             {"command": "solid.extrude", "params": {"distance": "18 mm"}}]
    parts = cli(board + [plugin(approval, "cutlist")])[-1]["result"]["parts"]
    assert [(p["length"], p["width"], p["thickness"]) for p in parts] == [(800.0, 150.0, 18.0)], parts


def test_saved_as_a_file(approval):
    # A profile of its own, so installing with an approved folder leaves the user's SolidCraft alone.
    profile, folder = ROOT / "dist" / "profile", ROOT / "dist" / "cutlists"
    for path in (profile, folder):
        shutil.rmtree(path, ignore_errors=True)
        path.mkdir(parents=True)
    keys = ("LOCALAPPDATA", "APPDATA", "XDG_DATA_HOME", "XDG_CONFIG_HOME")
    before = {key: os.environ.get(key) for key in keys}
    os.environ.update({key: str(profile) for key in keys})
    try:
        version = json.loads((PKG / "plugin.json").read_text(encoding="utf-8"))["version"]
        grant = {"name": "cutlists", "path": str(folder), "read": False, "write": True}
        install = {"command": "plugin.install", "params": {
            "path": str(ROOT / "dist" / f"woodwork-{version}.zip"), "replace": True,
            "approve": {"command_groups": [], "folders": [grant]}}}
        cabinet = plugin(approval, "cabinet", {})
        results = cli([install, cabinet, plugin(approval, "cutlist", {"file": "My kitchen", "format": "csv"}), plugin(approval, "cutlist"),
                       plugin(approval, "cutlist", {"file": "My kitchen"})])
        saved, shown = results[2]["result"], results[3]["result"]
        assert saved["saved"] == "My kitchen.csv" and "saved" not in shown, (saved.get("saved"), shown.get("saved"))
        assert results[4]["result"]["saved"] == "My kitchen.pdf"
        assert sorted(p.name for p in folder.iterdir()) == ["My kitchen.csv", "My kitchen.pdf"]
        pdf = (folder / "My kitchen.pdf").read_bytes()
        assert pdf.startswith(b"%PDF-") and pdf.endswith(b"%%EOF\n") and b"Cabinet Side L" in pdf
        assert (folder / "My kitchen.csv").read_text(encoding="utf-8") == saved["csv"] == shown["csv"]
    finally:
        for key, value in before.items():
            os.environ.pop(key, None) if value is None else os.environ.__setitem__(key, value)
