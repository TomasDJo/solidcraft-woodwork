"""Panel: an end panel or a filler on one end of a cabinet, following its sizes."""
import json
import subprocess
import tempfile
from pathlib import Path

from build import CLI
from helpers import *


def change(name, mm):
    return {"command": "parameters.change", "params": {"name": name, "expression": f"{mm} mm"}}


def refused(steps):
    """The reason a script fails, or None when it runs."""
    with tempfile.TemporaryDirectory(dir=ROOT / "dist") as folder:
        script = Path(folder) / "refused.json"
        script.write_text(json.dumps(steps), encoding="utf-8")
        done = subprocess.run([CLI, "run", str(script), "--quiet"], capture_output=True, text=True)
    return done.stderr if done.returncode else None


def test_panels_follow_the_cabinet_and_survive_edit(approval):
    # A: 600 x 720 x 560 on the origin (x -300 to 300, front at y = -280), two overlay doors,
    # a 100 mm plinth. Its door faces are 2 + 18 mm in front of the carcass.
    delete = {"command": "timeline.delete", "params": {"features": ["A Filler L", "A Filler L return"]}}
    results = cli([plugin(approval, "cabinet", {"name": "A", "doors": 2, "plinth": 100}),
                   select("A Side R"), plugin(approval, "panel", {"kind": "end"}),
                   select("A Side L"), plugin(approval, "panel", {"kind": "filler", "width": 50}),
                   MEASURE, change("A_width", 800), change("A_height", 900), MEASURE,
                   select("A Top"), plugin(approval, "edit", {"shelves": 3, "plinth": 0}), MEASURE,
                   delete, select("A Top"), plugin(approval, "edit", {"shelves": 2}), MEASURE])
    assert results[2]["result"]["side"] == "right", results[2]
    box = boxes(results[5])
    assert box["A End panel R"] == [300, -300, 0, 318, 280, 720], box["A End panel R"]
    # the filler in the plane of the door faces, as high as the doors; its return to the back
    assert box["A Filler L"] == [-350, -300, 102, -300, -282, 718], box["A Filler L"]
    assert box["A Filler L return"] == [-350, -282, 102, -332, 280, 718], box["A Filler L return"]
    material = {b["name"]: b["material"] for b in results[5]["bodies"]}
    assert material["A End panel R"] == "Birch plywood", material
    # wider and higher: each stays at its side
    box = boxes(results[8])
    assert box["A End panel R"] == [400, -300, 0, 418, 280, 900], box["A End panel R"]
    assert box["A Filler L"] == [-450, -300, 102, -400, -282, 898], box["A Filler L"]
    assert box["A Filler L return"] == [-450, -282, 102, -432, 280, 898], box["A Filler L return"]
    # Edit cabinet made them again, with their material; without the plinth the filler reaches down
    box = boxes(results[11])
    assert "A Shelf 3" in box, sorted(box)
    assert box["A End panel R"] == [400, -300, 0, 418, 280, 900], box["A End panel R"]
    assert box["A Filler L"] == [-450, -300, 2, -400, -282, 898], box["A Filler L"]
    material = {b["name"]: b["material"] for b in results[11]["bodies"]}
    assert material["A End panel R"] == "Birch plywood", material
    # its folder's features deleted, the filler stays gone through the next Edit
    box = boxes(results[15])
    assert "A Shelf 2" in box and "A End panel R" in box and "A Filler L" not in box, sorted(box)


def test_not_beside_a_cabinet_and_on_a_moved_one(approval):
    # B on the origin with an end panel on its right; inset fronts: flush with the carcass.
    start = [plugin(approval, "settings", NONE), plugin(approval, "cabinet", {"name": "B", "doors": 1, "fronts": "inset"}),
             select("B Side R"), plugin(approval, "panel", {"kind": "end"})]
    # placed on that side, the new cabinet stands beyond the panel, by Right or by the panel
    # picked with Auto; with a filler between two cabinets too
    results = cli([*start, select("B Side R"), plugin(approval, "cabinet", {"name": "C", "place": "right"}),
                   select("C Side R"), plugin(approval, "panel", {"kind": "filler", "width": 50}),
                   select("C Filler R"), plugin(approval, "cabinet", {"name": "D", "place": "auto"}), MEASURE])
    box = boxes(results[-1])
    assert box["C Side L"][0] == 318 and box["C Side R"][3] == 918, (box["C Side L"], box["C Side R"])
    assert box["D Side L"][0] == 968 and box["D Side L"][1] == -280, box["D Side L"]
    # one panel a side, of any kind
    reason = refused([*start, select("B Side R"), plugin(approval, "panel", {"kind": "filler"})])
    assert reason and "already has an end panel on its right" in reason, reason
    reason = refused([plugin(approval, "corner", {"name": "K"}), select("K Side L"), plugin(approval, "panel", {})])
    assert reason and "`K` is a corner cabinet" in reason, reason
    # not where another cabinet stands, on either cabinet of the pair
    beside = [*start, select("B Side L"), plugin(approval, "cabinet", {"name": "C", "place": "left"})]
    reason = refused([*beside, select("B Side L"), plugin(approval, "panel", {})])
    assert reason and "`B` has a cabinet against its left" in reason, reason
    reason = refused([*beside, select("C Side R"), plugin(approval, "panel", {})])
    assert reason and "`C` has a cabinet against its right" in reason, reason
    # B deleted: C's link went with it, its note stays, and its right end is free
    gone = cli([*beside, {"command": "component.delete", "params": {"component": "B"}},
                select("C Side R"), plugin(approval, "panel", {}), MEASURE])
    assert boxes(gone[-1])["C End panel R"][3] == -282, boxes(gone[-1])["C End panel R"]
    # C left of B (x -900 to -300), moved and held: an end panel on its left, then joinery,
    # which takes C to the origin and back
    results = cli([*beside, select("C Side L"), plugin(approval, "panel", {"thickness": 25}), MEASURE,
                   select("C Top"), plugin(approval, "edit", {"joints": "confirmat"}), MEASURE])
    box = boxes(results[8])
    assert box["B End panel R"] == [300, -280, 0, 318, 280, 720], box["B End panel R"]
    assert box["C End panel L"] == [-925, -280, 0, -900, 280, 720], box["C End panel L"]
    box = boxes(results[11])
    assert box["C End panel L"] == [-925, -280, 0, -900, 280, 720], box["C End panel L"]
    # T turned a quarter (as Place turns a cabinet past a corner's return) and moved: its end
    # panel still stands at its own Side R, which faces -Y now (x 300 to 318 turned to y -318
    # to -300, y -280 to 280 to x 720 to 1280).
    to = {"position": [1000, 0, 0], "axis": [0, 0, 1], "angle": "-90 deg"}
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "T"}),
                   {"command": "occurrence.ground", "params": {"occurrence": "T:1", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "T:1", "to": to}},
                   {"command": "occurrence.ground", "params": {"occurrence": "T:1", "grounded": True}},
                   select("T Side R"), plugin(approval, "panel", {}), MEASURE])
    box = boxes(results[-1])
    assert box["T End panel R"] == [720, -318, 0, 1280, -300, 720], box["T End panel R"]


def test_a_filler_in_inches_on_a_sink_base(approval):
    # An inch design; S a sink base with two overlay doors. A 2 in filler, 3/4 in thick, on its
    # right: 50.8 mm out from the side, from the door faces (y = -300), as high as the doors.
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"name": "S", "kind": "sink", "doors": 2}),
                   select("S Side R"),
                   plugin(approval, "panel", {"kind": "filler", "units": "in", "width": 2, "thickness": 0.75}),
                   MEASURE, plugin(approval, "cutlist")])
    box = boxes(results[4])
    assert box["S Filler R"] == [300, -300, 2, 350.8, -280.95, 718], box["S Filler R"]
    assert box["S Filler R return"] == [331.75, -280.95, 2, 350.8, 280, 718], box["S Filler R return"]
    # in the cut list with its grain up and its front edge banded; the return not
    csv = results[5]["result"]["csv"]
    filler = next(line for line in csv.splitlines() if line.endswith('"S Filler R"'))
    assert ",yes,L1," in filler, filler
    back = next(line for line in csv.splitlines() if line.endswith('"S Filler R return"'))
    assert ",yes,," in back, back
