"""Panel: an end panel or a filler on one end of a cabinet, following its sizes."""
import json
import subprocess

from build import CLI
from helpers import *


def change(**to):
    return {"command": "parameters.change", "params": {"changes": {name: f"{mm} mm" for name, mm in to.items()}}}


def refused(steps):
    """The reason a script fails, or None when it runs."""
    script = ROOT / "dist" / "refused.json"
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
                   MEASURE, change(A_width=800, A_height=900), MEASURE,
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
    box = boxes(results[7])
    assert box["A End panel R"] == [400, -300, 0, 418, 280, 900], box["A End panel R"]
    assert box["A Filler L"] == [-450, -300, 102, -400, -282, 898], box["A Filler L"]
    assert box["A Filler L return"] == [-450, -282, 102, -432, 280, 898], box["A Filler L return"]
    # Edit cabinet made them again, with their material; without the plinth the filler reaches down
    box = boxes(results[10])
    assert "A Shelf 3" in box, sorted(box)
    assert box["A End panel R"] == [400, -300, 0, 418, 280, 900], box["A End panel R"]
    assert box["A Filler L"] == [-450, -300, 2, -400, -282, 898], box["A Filler L"]
    material = {b["name"]: b["material"] for b in results[10]["bodies"]}
    assert material["A End panel R"] == "Birch plywood", material
    # its folder's features deleted, the filler stays gone through the next Edit
    box = boxes(results[14])
    assert "A Shelf 2" in box and "A End panel R" in box and "A Filler L" not in box, sorted(box)


def test_place_beside_a_panel_and_joinery_and_worktop(approval):
    # B on the origin with an end panel on its right; inset fronts: flush with the carcass.
    start = [plugin(approval, "cabinet", {"name": "B", "doors": 1, "fronts": "inset"}),
             select("B Side R"), plugin(approval, "panel", {"kind": "end"})]
    reason = refused([*start, select("B Side R"), plugin(approval, "cabinet", {"name": "C", "place": "right"})])
    assert reason and "`B` has an end panel on its right: place the new cabinet on its other side" in reason, reason
    assert "already has an end panel on its right" in refused([*start, select("B Side R"), plugin(approval, "panel", {})])
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
    # C left of B (x -900 to -300), moved and held: an end panel on its left, then joinery
    # (which takes C to the origin and back) and a worktop over the two.
    results = cli([*start, select("B Side L"), plugin(approval, "cabinet", {"name": "C", "place": "left"}),
                   select("C Side L"), plugin(approval, "panel", {"thickness": 25}), MEASURE,
                   select("C Top"), plugin(approval, "joinery"),
                   pick("C Top", "B Top"), plugin(approval, "worktop"), MEASURE])
    box = boxes(results[7])
    assert box["B End panel R"] == [300, -280, 0, 318, 280, 720], box["B End panel R"]
    assert box["C End panel L"] == [-925, -280, 0, -900, 280, 720], box["C End panel L"]
    box = boxes(results[12])
    assert box["C End panel L"] == [-925, -280, 0, -900, 280, 720], box["C End panel L"]
    # the worktop covers the carcasses, not the panels
    assert box["Worktop"] == [-900, -320, 720, 300, 280, 758], box["Worktop"]
