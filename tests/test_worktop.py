"""Worktop: pieces over the selected cabinets, measured where they stand, joined in corners."""
import json
import math
import subprocess

from build import CLI
from helpers import *

LIST = {"command": "component.list", "params": {}}


def unground(name):
    return {"command": "occurrence.ground", "params": {"occurrence": f"{name}:1", "grounded": False}}


def move(name, x, y=0):
    return {"command": "occurrence.move", "params": {"occurrence": f"{name}:1", "translate": [x, y, 0]}}


def worktops(results):
    """The components named Worktop in a component.list reply."""
    return [c for c in results["components"] if c["name"] == "Worktop"]


def test_a_straight_run_with_a_gap_made_again(approval):
    # A on the origin (x -300 to 300, back at y = 280), B 600 further right, a dishwasher's gap
    results = cli([plugin(approval, "cabinet", {"name": "A"}), plugin(approval, "cabinet", {"name": "B"}),
                   unground("B"), move("B", 1200), pick("A Top", "B Shelf 1"),
                   plugin(approval, "worktop", {"left_overhang": 10, "right_overhang": 15}), MEASURE,
                   plugin(approval, "cutlist"),
                   # B moved on: Worktop run again with the same name replaces it
                   move("B", 200), pick("A Top", "B Top"), plugin(approval, "worktop"), MEASURE, LIST,
                   # with only the worktop picked: over the cabinets it remembers
                   pick("Worktop"), plugin(approval, "worktop"), LIST])
    made, measure = results[5]["result"], results[6]
    assert made["bodies"] == ["Worktop"] and made["cabinets"] == ["A", "B"], made
    # over the gap, the back on the wall, 40 past the carcass fronts, on the tops
    box = boxes(measure)
    assert box["Worktop"] == [-310, -320, 720, 1515, 280, 758], box["Worktop"]
    assert {b["name"]: b["material"] for b in measure["bodies"]}["Worktop"] == "Oak"
    # bought to length in the cut list
    listed = results[7]["result"]["worktops"]
    assert [(w["part"], w["length"], w["width"], w["thickness"]) for w in listed] == [("Worktop", 1825, 600, 38)], listed
    assert results[10]["result"]["replaced"] == ["Worktop"], results[10]
    assert boxes(results[11])["Worktop"] == [-300, -320, 720, 1700, 280, 758], boxes(results[11])["Worktop"]
    assert len(worktops(results[12])) == 1, worktops(results[12])
    assert results[14]["result"]["cabinets"] == ["A", "B"], results[14]
    assert len(worktops(results[15])) == 1, worktops(results[15])


def corner_kitchen(approval):
    """K in a left corner (walls at x = -450 and y = 450), C on its return, A right of it and
    B 600 further on, a gap. Then everything picked."""
    return [plugin(approval, "corner", {"name": "K", "corner": "left"}),
            select("K Side L"), plugin(approval, "cabinet", {"name": "C", "place": "auto"}),
            select("K Side R"), plugin(approval, "cabinet", {"name": "A", "place": "auto"}),
            plugin(approval, "cabinet", {"name": "B"}), unground("B"), move("B", 1950, 170),
            pick("K Top", "A Top", "B Top", "C Top")]


def test_an_l_over_a_corner_cabinet(approval):
    saved = ROOT / "dist" / "worktop-corner.sc"
    results = cli([*corner_kitchen(approval), plugin(approval, "worktop"), MEASURE,
                   pick("Worktop 2", "K Top"), plugin(approval, "worktop", {"joint": "mitre"}), MEASURE,
                   plugin(approval, "cutlist"), LIST], "--save", str(saved))
    made = results[9]["result"]
    assert made["bodies"] == ["Worktop 1", "Worktop 2"] and made["cabinets"] == ["K", "A", "B", "C"], made
    # butt: the back run is longer, over the gap and through to the left wall; the return's
    # piece stops against its front edge
    box = boxes(results[10])
    assert box["Worktop 1"] == [-450, -150, 720, 2250, 450, 758], box["Worktop 1"]
    assert box["Worktop 2"] == [-450, -1050, 720, 150, -150, 758], box["Worktop 2"]
    # mitre, run again from a piece and a cabinet picked, so with the dialog's values: both to
    # the walls, cut corner to corner
    box = boxes(results[13])
    assert box["Worktop 1"] == [-450, -150, 720, 2250, 450, 758], box["Worktop 1"]
    assert box["Worktop 2"] == [-450, -1050, 720, 150, 450, 758], box["Worktop 2"]
    half = 600 * 600 / 2 * 38
    assert volume(results[13], "Worktop 1") == 2700 * 600 * 38 - half, volume(results[13], "Worktop 1")
    assert volume(results[13], "Worktop 2") == 1500 * 600 * 38 - half, volume(results[13], "Worktop 2")
    # bought at its whole length, the mitre's point included
    listed = results[14]["result"]["worktops"]
    assert sorted((w["part"], w["length"], w["width"]) for w in listed) == [("Worktop 1", 2700, 600), ("Worktop 2", 1500, 600)], listed
    assert len(worktops(results[15])) == 1


def test_a_diagonal_corner_and_what_is_refused(approval):
    # the same square L as an L-shaped corner cabinet: K is 900 x 900 on the origin
    results = cli([plugin(approval, "corner", {"name": "K", "shape": "diagonal", "corner": "left"}),
                   pick("K Top"), plugin(approval, "worktop", {"front_overhang": 20}), MEASURE])
    box = boxes(results[3])
    assert box["Worktop 1"] == [-450, -450, 720, 130, 450, 758], box["Worktop 1"]
    assert box["Worktop 2"] == [130, -130, 720, 450, 450, 758], box["Worktop 2"]
    refused = []
    for steps in ([plugin(approval, "cabinet", {"name": "A"}), plugin(approval, "cabinet", {"name": "H", "height": 900}),
                   unground("H"), move("H", 600), pick("A Top", "H Top"), plugin(approval, "worktop")],
                  [plugin(approval, "cabinet", {"name": "A"}), plugin(approval, "cabinet", {"name": "F"}), unground("F"),
                   {"command": "occurrence.move", "params": {"occurrence": "F:1", "translate": [0, -2000, 0], "axis": [0, 0, 1], "angle": "180 deg"}},
                   pick("A Top", "F Top"), plugin(approval, "worktop")]):
        script = ROOT / "dist" / "refused.json"
        script.write_text(json.dumps(steps), encoding="utf-8")
        done = subprocess.run([CLI, "run", str(script), "--quiet"], capture_output=True, text=True)
        refused.append(done.stderr if done.returncode else "")
    assert "`H` is not as high as `A`" in refused[0], refused[0]
    assert "face opposite ways: make one worktop per run, with different names" in refused[1], refused[1]


def test_a_bench_top_with_dog_holes_and_a_rebated_cut_out(approval):
    zone = {"dog_holes": "zone", "zone_x": 100, "zone_y": 100, "zone_length": 800, "zone_width": 400}
    cutout = {"cutout": "rebated", "cutout_length": 300, "cutout_width": 200, "cutout_x": 1400, "cutout_y": 300}
    saved = ROOT / "dist" / "worktop-bench.sc"
    results = cli([plugin(approval, "cabinet", {"name": "A", "width": 1800, "height": 900, "doors": 2}), pick("A Top"),
                   plugin(approval, "worktop", {"name": "Bench top", "material": "Birch plywood", **zone, **cutout}),
                   MEASURE, plugin(approval, "cutlist"),
                   # run again from its piece with the dialog's defaults: kept as it was
                   pick("Bench top"), plugin(approval, "worktop"), MEASURE, LIST], "--save", str(saved))
    made, measure = results[2]["result"], results[3]
    # 9 by 4 holes in the zone, on the 96 mm grid: one cylinder and its pattern
    assert made["dog_holes"] == 36 and cylinders(measure, "Bench top") == 36, (made, cylinders(measure, "Bench top"))
    assert boxes(measure)["Bench top"] == [-900, -320, 900, 900, 280, 938], boxes(measure)["Bench top"]
    holes = 36 * math.pi * 10 ** 2 * 38
    opening = 300 * 200 * 38 + (320 * 220 - 300 * 200) * 10
    assert abs(volume(measure, "Bench top") - (1800 * 600 * 38 - holes - opening)) < 100, volume(measure, "Bench top")
    # not named Worktop: a board laid out on the sheets like any other
    listed = results[4]["result"]
    assert listed["worktops"] == [], listed["worktops"]
    assert any("Bench top" in p["part"] for layout in listed["layouts"] for p in layout["parts"]), listed["layouts"]
    again = results[6]["result"]
    assert again["name"] == "Bench top" and again["dog_holes"] == 36 and again["replaced"] == ["Bench top"], again
    assert cylinders(results[7], "Bench top") == 36 and volume(results[7], "Bench top") == volume(measure, "Bench top")
    names = [c["name"] for c in results[8]["components"]]
    assert names.count("Bench top") == 1 and "Worktop" not in names, names
