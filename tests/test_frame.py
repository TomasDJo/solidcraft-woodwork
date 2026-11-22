"""Frame: a leg-and-rail base, its joints, the cut list's timber and Edit."""
import json
import math
import subprocess

from build import CLI
from helpers import *

DOWEL = math.pi * 4 ** 2 * 21  # an 8 x 40 mm dowel's hole, 21 mm into a part (1 mm to spare)
BOLT = math.pi * 5 ** 2 * 70  # a 10 mm bolt's hole through a 70 mm leg


def frame(approval, **params):
    return plugin(approval, "frame", {"name": "Frame", "place": "origin"} | params)


def outline(box, prefix="Frame "):
    corners = [b for name, b in box.items() if name.startswith(prefix)]
    return [round(f(c[i] for c in corners), 3) for f, i in ((min, 0), (min, 1), (min, 2), (max, 3), (max, 4), (max, 5))]


def refused(steps):
    script = ROOT / "dist" / "refused.json"
    script.write_text(json.dumps(flat(steps)), encoding="utf-8")
    done = subprocess.run([CLI, "run", str(script), "--quiet"], capture_output=True, text=True)
    return done.stderr if done.returncode else None


def test_a_frame_with_dowels_and_its_cut_list(approval):
    # 1200 x 600 x 870 on the origin: legs 70, aprons 100 x 22 set 10 in, stretchers at the ends
    results = cli([frame(approval), MEASURE, plugin(approval, "cutlist")])
    box = boxes(results[1])
    assert outline(box) == [-600, -300, 0, 600, 300, 870], outline(box)
    assert box["Frame Leg front left"] == [-600, -300, 0, -530, -230, 870], box["Frame Leg front left"]
    # between the legs, top flush, outer face 10 mm in from the legs' outer faces
    assert box["Frame Apron front"] == [-530, -290, 770, 530, -268, 870], box["Frame Apron front"]
    assert box["Frame Apron L"] == [-590, -230, 770, -568, 230, 870], box["Frame Apron L"]
    assert box["Frame Stretcher R"] == [568, -230, 150, 590, 230, 250], box["Frame Stretcher R"]
    assert "Frame Stretcher front" not in box and len([n for n in box if n.startswith("Frame ")]) == 10, sorted(box)
    # two dowel holes at each end of the apron; six in a corner leg (two aprons and a stretcher)
    assert abs(volume(results[1], "Frame Apron front") - (1060 * 22 * 100 - 4 * DOWEL)) < 100
    assert abs(volume(results[1], "Frame Leg front left") - (70 * 70 * 870 - 6 * DOWEL)) < 100
    material = {b["name"]: b["material"] for b in results[1]["bodies"]}
    assert material["Frame Apron front"] == "Oak", material
    # the cut list: solid timber bought to length, not on sheets; the dowels counted
    listed = results[2]["result"]
    timber = {t["part"]: (t["length"], t["width"], t["thickness"]) for t in listed["timber"]}
    assert len(timber) == 10 and timber["Frame Leg back right"] == (870, 70, 70), timber
    assert timber["Frame Apron back"] == (1060, 100, 22), timber
    assert listed["layouts"] == [], listed["layouts"]
    assert {"item": "Dowel, 8 x 40 mm", "qty": 24} in listed["hardware"], listed["hardware"]
    assert not any(h["item"] == "Adjustable leg" for h in listed["hardware"]), listed["hardware"]


def test_bolts_six_legs_and_inches(approval):
    results = cli([frame(approval, joint="bolts"), MEASURE, plugin(approval, "cutlist"),
                   frame(approval, name="Long", width=2000, legs="6", stretchers="all"), MEASURE,
                   frame(approval, name="Inch", units="in"), MEASURE])
    # a bolt through the corner leg for each of its two aprons and its stretcher
    assert abs(volume(results[1], "Frame Leg front left") - (70 * 70 * 870 - 3 * BOLT)) < 100
    assert cylinders(results[1], "Frame Apron front") >= 4  # two bolts' ends and two barrel nuts
    hardware = results[2]["result"]["hardware"]
    assert {"item": "Bed bolt, 10 mm", "qty": 12} in hardware and {"item": "Barrel nut, 10 mm", "qty": 12} in hardware, hardware
    # six legs: a middle pair, the long rails split at it, a middle apron and stretcher across
    box = boxes(results[4])
    assert box["Long Leg middle back"] == [-35, 230, 0, 35, 300, 870], box["Long Leg middle back"]
    assert box["Long Apron front L"] == [-930, -290, 770, -35, -268, 870], box["Long Apron front L"]
    assert box["Long Apron middle"] == [-11, -230, 770, 11, 230, 870], box["Long Apron middle"]
    assert box["Long Stretcher back R"] == [35, 268, 150, 930, 290, 250], box["Long Stretcher back R"]
    assert len([n for n in box if n.startswith("Long ")]) == 20, sorted(box)
    assert cylinders(results[4], "Long Leg middle front") > 0
    # inches: 48 x 24 x 34 in
    assert outline(boxes(results[6]), "Inch ") == [-609.6, -304.8, 0, 609.6, 304.8, 863.6], outline(boxes(results[6]), "Inch ")


def test_edit_keeps_a_parameter_change_and_place_beside_a_cabinet(approval):
    change = {"command": "parameters.change", "params": {"name": "Frame_width", "expression": "1400 mm"}}
    results = cli([plugin(approval, "cabinet", {"name": "A"}), select("A Side R"),
                   frame(approval, place="right"), MEASURE,
                   change, select("Frame Apron front"),
                   plugin(approval, "edit", {"stretchers": "all", "frame_legs": "6"}), MEASURE,
                   {"command": "joint.list", "params": {}}])
    assert results[2]["result"]["linked"], results[2]
    # against A's right side (x = 300), backs flush (A is 560 deep, the frame 600)
    assert outline(boxes(results[3])) == [300, -320, 0, 1500, 280, 870], outline(boxes(results[3]))
    assert results[6]["result"]["edited"], results[6]
    box = boxes(results[7])
    # still 1400 wide from the Parameters, grown away from A, with six legs and stretchers all round
    assert outline(box) == [300, -320, 0, 1700, 280, 870], outline(box)
    assert "Frame Leg middle back" in box and "Frame Stretcher front R" in box, sorted(box)
    assert any(j["name"] == "Frame link" for j in results[8]["joints"]), results[8]


def test_what_a_frame_refuses(approval):
    start = [frame(approval), select("Frame Leg front left")]
    for steps in (start + [plugin(approval, "edit", {"joints": "dowels"})],
                  start + [plugin(approval, "panel", {"kind": "end"})],
                  start + [plugin(approval, "edit", {"shelves": 2})],
                  [frame(approval, leg=10)]):
        assert refused(steps), steps
    assert "is a frame" in refused(start + [plugin(approval, "panel", {"kind": "end"})])


def test_a_worktop_over_a_frame(approval):
    # the default frame: legs' outer faces x -600 to 600, y -300 to 300, top 870
    results = cli([frame(approval), pick("Frame Apron front"),
                   plugin(approval, "worktop", {"left_overhang": 30, "right_overhang": 30}), MEASURE,
                   plugin(approval, "cutlist"),
                   # with only the worktop picked: made again over the frame it remembers
                   pick("Worktop"), plugin(approval, "worktop"), MEASURE])
    assert results[2]["result"]["cabinets"] == ["Frame"], results[2]
    # on the aprons' top, the back flush with the back legs, 40 past the front, 30 at each end
    assert boxes(results[3])["Worktop"] == [-630, -340, 870, 630, 300, 908], boxes(results[3])["Worktop"]
    listed = results[4]["result"]
    assert [w["part"] for w in listed["worktops"]] == ["Worktop"], listed["worktops"]
    assert results[6]["result"]["cabinets"] == ["Frame"], results[6]
    assert boxes(results[7])["Worktop"] == [-630, -340, 870, 630, 300, 908], boxes(results[7])["Worktop"]
