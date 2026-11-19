"""Worktop: one board over a straight run of the selected cabinets, following their parameters."""
import json
import subprocess

from build import CLI
from helpers import *


def change(**to):
    return {"command": "parameters.change", "params": {"changes": {name: f"{mm} mm" for name, mm in to.items()}}}


# A, B and C side by side, 600 wide and 560 deep: A on the origin (x -300 to 300), the backs at
# y = 280. B and C are held to the one before them, so the run grows to the right.
RUN = [("A", {}), ("B", {"place": "right"}), ("C", {"place": "right", "doors": 1})]
TOP = {"left_overhang": 10, "right_overhang": 15}


def run(approval):
    steps = []
    for i, (name, params) in enumerate(RUN):
        if i:
            steps.append(select(f"{RUN[i - 1][0]} Top"))
        steps.append(plugin(approval, "cabinet", {**params, "name": name}))
    return steps


def test_a_worktop_over_a_run_follows_it(approval):
    results = cli([*run(approval), pick("A Top", "C Side R", "B Shelf 1"),
                   plugin(approval, "worktop", TOP), MEASURE, plugin(approval, "cutlist"),
                   change(B_width=800), MEASURE,
                   select("B Top"), plugin(approval, "edit", {"shelves": 3}), change(C_width=700), MEASURE])
    made, measure = results[6]["result"], results[7]
    assert made["name"] == "Worktop" and made["cabinets"] == ["A", "B", "C"], made
    # x from the run's left side to its right side plus the overhangs, the back flush, 40 mm
    # past the carcass fronts, 40 by default, on top of the cabinets
    box = boxes(measure)
    assert box["Worktop"] == [-310, -320, 720, 1515, 280, 758], box["Worktop"]
    assert volume(measure, "Worktop") == 1825 * 600 * 38, volume(measure, "Worktop")
    material = {b["name"]: b["material"] for b in measure["bodies"]}
    assert material["Worktop"] == "Oak", material
    # the cut list has it at its outer size
    row = next(r for r in results[8]["result"]["parts"] if "Worktop" in r["parts"])
    assert [row["length"], row["width"], row["thickness"]] == [1825, 600, 38], row
    # wider B: the worktop grows to the right with the run
    assert boxes(results[10])["Worktop"] == [-310, -320, 720, 1715, 280, 758], boxes(results[10])["Worktop"]
    # Edit cabinet on a cabinet under it: the worktop stays and still follows
    box = boxes(results[14])
    assert "B Shelf 3" in box and box["Worktop"] == [-310, -320, 720, 1815, 280, 758], box["Worktop"]


def test_it_holds_where_the_run_holds(approval):
    # A grounded on the origin grows to both sides, also in depth: the worktop stands on its
    # middle, so its back stays on A's back (and B's, which is held flush to it).
    results = cli([*run(approval)[:3], pick("A Top", "B Top"), plugin(approval, "worktop"),
                   change(A_depth=660), MEASURE])
    box = boxes(results[6])
    assert box["A Side L"][4] == 330 and box["Worktop"] == [-300, -370, 720, 900, 330, 758], box["Worktop"]
    # B placed left of A: a wider B grows to the left, and so does the worktop
    results = cli([plugin(approval, "cabinet", {"name": "A"}), select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "place": "left"}),
                   pick("A Top", "B Top"), plugin(approval, "worktop"), change(B_width=800), MEASURE])
    box = boxes(results[6])
    assert box["B Side L"][0] == -1100 and box["Worktop"] == [-1100, -320, 720, 300, 280, 758], box["Worktop"]


def test_a_turned_run_and_what_is_not_a_run(approval):
    # Along a corner cabinet's return: U is turned a quarter, facing -X; its left end, seen
    # from its front, is at the corner.
    results = cli([plugin(approval, "corner", {"name": "K", "corner": "right"}), select("K Top"),
                   plugin(approval, "cabinet", {"name": "U", "place": "right"}),
                   pick("U Top"), plugin(approval, "worktop", {"left_overhang": 400, "name": "Return"}),
                   MEASURE, plugin(approval, "cutlist")])
    box = boxes(results[5])
    # K: x -450 to 450, back at y = 450, its return ends at y = -450; U 600 wide beyond that
    assert box["Return"] == [-150, -1050, 720, 450, -50, 758], box["Return"]
    # the cut list knows it for a worktop by how it was made, not by its name
    worktops = results[6]["result"]["worktops"]
    assert [(w["part"], w["length"], w["width"]) for w in worktops] == [("Return", 1000, 600)], worktops
    refused = []
    for steps in ([plugin(approval, "cabinet", {"name": "A"}),
                   {"command": "occurrence.ground", "params": {"occurrence": "A:1", "grounded": False}},
                   plugin(approval, "cabinet", {"name": "B"}),
                   {"command": "occurrence.ground", "params": {"occurrence": "B:1", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "B:1", "translate": [610, 0, 0]}},
                   pick("A Top", "B Top"), plugin(approval, "worktop")],
                  [plugin(approval, "cabinet", {"name": "A"}), select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "place": "above"}),
                   pick("A Top", "B Top"), plugin(approval, "worktop")]):
        script = ROOT / "dist" / "refused.json"
        script.write_text(json.dumps(steps), encoding="utf-8")
        done = subprocess.run([CLI, "run", str(script), "--quiet"], capture_output=True, text=True)
        if done.returncode:  # the run fails, with the reason
            refused.append(done.stderr)
    assert len(refused) == 2, refused
    assert "not one straight run: there is a gap between `A` and `B`" in refused[0], refused[0]
    assert "not one straight run: `B` does not stand on the same floor as `A`" in refused[1], refused[1]
