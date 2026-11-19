"""Saving the cut list as a file, in a folder approved at install."""
import json
import os
import shutil

from helpers import *


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
