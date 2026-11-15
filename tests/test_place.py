"""Place: a new cabinet against the selected one."""
from helpers import *


def test_beside_and_above(approval):
    # Place against the selection: B right of A, C on top of B. In an inch design, to check
    # that the move is not read in inches.
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"name": "A", "width": 600}),
                   select("A Shelf 1"),
                   plugin(approval, "cabinet", {"name": "B", "width": 800, "depth": 400, "place": "right"}),
                   select("B Side L"),
                   plugin(approval, "cabinet", {"name": "C", "width": 500, "depth": 300, "place": "above"}),
                   {"command": "inspect.measure", "params": {}}])
    box = {b["name"]: [round(v, 6) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[6]["bodies"]}
    assert box["A Side R"][3] == 300 and box["B Side L"][0] == 300, (box["A Side R"], box["B Side L"])
    assert box["B Side L"][4] == 280, box["B Side L"]  # backs flush at y = 280
    assert box["C Side L"][:3] == [300, -20, 720] and box["C Side L"][4] == 280, box["C Side L"]


def test_along_a_return_and_a_turned_run(approval):
    # Right of a right-hand corner cabinet is along its return: turned to face into the room.
    # One more continues the run, and one goes above it.
    box_cabinet = {"width": 400, "doors": 1, "handles": "knob", "place": "right"}
    results = cli([plugin(approval, "corner", {"name": "K", "corner": "right"}), select("K Top"),
                   plugin(approval, "cabinet", {**box_cabinet, "name": "R"}), select("R Top"),
                   plugin(approval, "cabinet", {**box_cabinet, "name": "S"}), select("S Top"),
                   plugin(approval, "corner", {"name": "U", "corner": "right", "place": "right"}), select("S Top"),
                   plugin(approval, "cabinet", {**box_cabinet, "name": "T", "place": "above", "depth": 300}),
                   {"command": "inspect.measure", "params": {}}])
    box = boxes(results[9])
    # a corner cabinet carries on too: its Side L wall to wall with S's Side R
    assert box["U Side L"] == [-110, -1268, 0, 450, -1250, 720], box["U Side L"]
    assert [box["K Side R"][i] for i in (3, 1)] == [450, -450]  # the right wall, the return's end
    assert box["R Side L"] == [-110, -468, 0, 450, -450, 720], box["R Side L"]  # back on the wall
    assert box["S Side R"] == [-110, -1250, 0, 450, -1232, 720], box["S Side R"]  # and on along it
    assert box["T Side L"] == [150, -868, 720, 450, -850, 1440], box["T Side L"]  # above S, left sides flush
    assert box["R Door"] == [-130, -848, 2, -112, -452, 718], box["R Door"]  # facing into the room
