"""Materials, and the cut list in an inch design."""
from helpers import *


def test_materials(approval):
    # Materials: oak doors on a melamine cabinet, next to a plywood one.
    results = cli([plugin(approval, "cabinet", {"name": "A", "doors": 2}),
                   plugin(approval, "cabinet", {"name": "A", "material": "none"}),  # taken: becomes A 2
                   pick("A Top"),
                   plugin(approval, "material", {"material": "White melamine"}),
                   pick("A Door L", "A Door R"),
                   plugin(approval, "material", {"material": "Oak", "scope": "parts"}),
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")])
    material = {b["name"]: b["material"] for b in results[6]["bodies"]}
    assert material["A Side L"] == material["A Back"] == "White melamine", material
    assert material["A Door L"] == material["A Door R"] == "Oak", material
    assert material["A 2 Side L"] == "Default", material  # untouched: another cabinet
    sheets = results[7].get("result", results[7])["sheets"]
    assert {s["material"] for s in sheets} == {"White melamine", "Oak", "Default"}, sheets


def test_inches(approval):
    # Inches: a 24 x 30 x 22 in cabinet in a design set to inches.
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"units": "in", "width": 24, "height": 30, "depth": 22,
                                                "thickness": 0.75, "back_thickness": 0.25}),
                   plugin(approval, "cutlist")])
    cutlist = results[2].get("result", results[2])
    assert cutlist["units"] == "in", cutlist
    assert '2,30,22,3/4,"Birch plywood","Cabinet Side L, Cabinet Side R"' in cutlist["csv"], cutlist["csv"]

