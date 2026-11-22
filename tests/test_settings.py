"""Woodwork settings: defaults kept in the design, that Cabinet and Corner start from."""
import json
import subprocess

from build import CLI
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


def test_camera_leave_skips_the_fit(approval):
    # Camera `leave` in Woodwork settings: a cabinet run reports no view; back to `fit`, it does.
    results = cli([plugin(approval, "settings", {"camera": "leave"}),
                   plugin(approval, "cabinet", {"name": "A"}),
                   plugin(approval, "settings", {"camera": "fit"}),
                   plugin(approval, "cabinet", {"name": "B"})])
    assert results[0].get("result", results[0])["settings"] == {"camera": "leave"}, results[0]
    assert [results[i].get("view") for i in (1, 3)] == [None, {"kind": "fit"}], results


def test_joinery_and_hardware_are_design_wide(approval):
    # Stored: confirmat 500 apart, hinges to 165, no models. A new cabinet takes them and
    # remembers them; Edit keeps them after the settings changed.
    results = cli([setting(approval, {"joints": "confirmat", "joint_spacing": 500, "hinge_angle": "165",
                                      "hardware_models": "off"}),
                   plugin(approval, "cabinet", {"name": "A", "doors": 1, "shelves": 0}),
                   setting(approval, {"joints": "none"}),
                   select("A Side L"), plugin(approval, "edit", {"labels": "yes"}),
                   MEASURE, {"command": "component.metadata", "params": {"component": "A"}},
                   {"command": "joint.list", "params": {}}])
    stored = results[0].get("result", results[0])["settings"]
    assert stored == {"joints": "confirmat", "joint_spacing": 500, "hinge_angle": "165", "hardware_models": "off"}, stored
    assert results[2].get("result", results[2])["settings"]["joints"] == "none"
    kept = results[6]["metadata"]["io.github.tomasdjo.woodwork"]["params"]
    assert (kept["joints"], kept["joint_spacing"], kept["hinge_angle"], kept["hardware_models"]) == ("confirmat", 500, "165", "off"), kept
    # 460 mm between the first and last, at most 500 apart: 2 screws a joint
    assert cylinders(results[5], "A Bottom") == 4, cylinders(results[5], "A Bottom")
    assert not [b for b in results[5]["bodies"] if b["name"].startswith("HW hinge (")]
    limits = {j["name"]: j["values"][0]["limits"] for j in results[7]["joints"]}
    assert limits["A Door hinge"] == [0, 165], limits


def test_a_cabinet_run_takes_no_joinery(approval):
    # Settings only: Cabinet has conditions, so a key it does not declare is refused.
    step = plugin(approval, "cabinet", {"name": "A"})
    step["params"]["params"]["joints"] = "dowels"
    script = ROOT / "dist" / "settings-refused.json"
    script.write_text(json.dumps([step]), encoding="utf-8")
    done = subprocess.run([CLI, "run", str(script), "--quiet"], capture_output=True, text=True)
    # (the app names no field)
    assert done.returncode and "is not an input of this entry" in done.stderr, done.stderr
