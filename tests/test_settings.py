"""Woodwork settings: defaults kept in the design, that Cabinet and Corner start from."""
from helpers import *


def without(step, *keys):
    """A plugin.run step with `keys` left out, for the dialog hook to fill in."""
    for key in keys:
        step["params"]["params"].pop(key)
    return step


def test_settings(approval):
    kept = ("material", "thickness", "plinth", "plinth_type")
    results = cli([plugin(approval, "settings", {"material": "MDF", "thickness": 25, "plinth": 100,
                                                 "plinth_type": "legs"}),
                   plugin(approval, "settings", {"units": "in", "thickness": 1}),  # only this changes
                   without(plugin(approval, "cabinet", {"name": "A"}), *kept),
                   without(plugin(approval, "cabinet", {"name": "B", "units": "in", "place": "origin"}), *kept),
                   plugin(approval, "corner", {"name": "C", "material": "Oak", "thickness": 18, "plinth": 0}),
                   MEASURE])
    stored = results[1].get("result", results[1])["settings"]
    assert stored == {"material": "MDF", "thickness": 25.4, "plinth": 100, "plinth_type": "legs"}, stored
    box = boxes(results[5])
    material = {b["name"]: b["material"] for b in results[5]["bodies"]}
    # A from the settings: MDF, 25.4 mm sides, on 100 mm legs.
    assert material["A Side L"] == "MDF", material
    assert round(box["A Side L"][3] - box["A Side L"][0], 3) == 25.4 and box["A Side L"][2] == 100, box["A Side L"]
    assert "A Leg 1" in box, box
    # B's dialog in inches gets them in inches: the same boards.
    assert round(box["B Side L"][3] - box["B Side L"][0], 3) == 25.4 and box["B Side L"][2] == 100, box["B Side L"]
    # What a run gives wins: C is oak, 18 mm, on the floor.
    assert material["C Side L"] == "Oak", material
    assert box["C Side L"][2] == 0 and "C Leg 1" not in box, box
