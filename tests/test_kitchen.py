"""The sample kitchen: a whole design, where things go wrong that a cabinet or two do not show."""
import subprocess
import sys

from helpers import *


def test_the_kitchen_builds_and_survives_a_rebuild(approval):
    # examples/kitchen.py uses every function, Edit on a drilled, turned wall unit among them. Edit
    # deletes the cabinet, which builds the history again; so does deleting one here. Holes
    # drilled with solid.hole did not survive that in a design this size: joints cut cylinders.
    out = ROOT / "dist" / "kitchen-test.sc"
    subprocess.run([sys.executable, str(ROOT / "examples" / "kitchen.py"), str(out)], check=True, stdout=subprocess.DEVNULL)
    results = cli([{"command": "component.delete", "params": {"component": "Base 2"}}, MEASURE], "--in", str(out))
    box = boxes(results[1])
    assert "Base 2 Top" not in box and len([n for n in box if n.startswith("Wall 4 Shelf")]) == 3, sorted(box)
    # the dowel holes are still in the wall units' boards: 2 at each end of a top (a row: a cut
    # and a pattern), 2 more for each of Wall 3's two dividers, as in Wall 4's (edited, turned)
    assert cylinders(results[1], "Wall 2 Top") == 4 and cylinders(results[1], "Wall 3 Top") == 8
    assert cylinders(results[1], "Wall 4 Top") == 4
