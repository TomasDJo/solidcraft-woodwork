"""Mobile base: one frame on casters under a group of cabinets (#101)."""
import subprocess

from helpers import *


def test_a_base_under_a_run_lifts_it_and_is_made_again(approval):
    # A and B, B placed right of A and held to it: a base under both makes a frame round their
    # 1200 x 560 footprint with a cross board where they meet and a caster in each corner, and
    # stands them on it, 100 + 70 mm up (B goes with A). The cut list counts the casters and
    # lists the frame's boards. Run again by its name with a taller frame, nothing picked: the
    # same cabinets, now 200 mm up.
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", "width": 600}), select("A Side R"),
        plugin(approval, "cabinet", {"name": "B", "width": 600, "place": "right"}),
        pick("A Side L", "B Side L"),
        plugin(approval, "base", {"name": "Bench"}),
        MEASURE,
        plugin(approval, "cutlist", {}),
        pick(),
        plugin(approval, "base", {"name": "Bench", "frame_height": 100}),
        MEASURE])
    summary, box = results[4]["result"], boxes(results[5])
    assert summary["cabinets"] == ["A", "B"] and summary["casters"] == 4, summary
    assert box["Bench Frame front"] == [-300, -280, 100, 900, -262, 170], box["Bench Frame front"]
    assert box["Bench Frame cross 1"][0] == 291, box["Bench Frame cross 1"]
    assert box["A Side L"][2] == 170 and box["B Side L"][2] == 170, (box["A Side L"], box["B Side L"])
    hardware = {h["item"]: h["qty"] for h in results[6]["result"]["hardware"]}
    assert hardware.get("Caster, locking") == 4, hardware
    parts = [n for p in results[6]["result"]["parts"] for n in p["parts"]]
    assert "Bench Frame front" in parts and "Bench Block 1" in parts, parts
    again = boxes(results[9])
    assert results[8]["result"]["raised"] == {"A": 30, "B": 30}, results[8]["result"]
    assert again["A Side L"][2] == 200 and again["B Side L"][2] == 200, (again["A Side L"], again["B Side L"])
    assert again["Bench Frame front"][5] == 200 and len([n for n in again if n.startswith("Bench Frame front")]) == 1


def refused(script):
    """The error of a script whose last step fails, from solidcraft-cli's JSON report."""
    import json
    import tempfile
    from pathlib import Path

    from build import CLI, env, flat
    with tempfile.TemporaryDirectory(dir=ROOT / "dist") as folder:
        path, out = Path(folder) / "script.json", Path(folder) / "results.json"
        path.write_text(json.dumps(flat(script)), encoding="utf-8")
        done = subprocess.run([CLI, "run", str(path), "--results", "--quiet", "--json-out", str(out)],
                              cwd=ROOT, env=env(), capture_output=True)
        assert done.returncode != 0, "the script was not refused"
        return json.loads(out.read_text(encoding="utf-8"))["error"]["error"]


def test_a_cabinet_that_cannot_go_up_is_refused(approval):
    # B is held to A; a base under B alone cannot lift it (A stays on the floor): the run is
    # refused with a message naming B, so nothing is made.
    error = refused([plugin(approval, "settings", NONE), plugin(approval, "cabinet", {"name": "A", "width": 600}),
                     select("A Side R"), plugin(approval, "cabinet", {"name": "B", "width": 600, "place": "right"}),
                     pick("B Side L"), plugin(approval, "base", {"name": "Bench"})])
    assert "`B` did not go up onto the base" in error, error


def test_a_narrow_run_and_cabinets_at_different_heights(approval):
    # Two 250 mm cabinets side by side, narrower together (500) than deep (560): the frame's
    # long way is its depth, and the joint at x = 0 still gets a cross board, along it.
    # Not linked, B stands 100 mm up (an earlier base, say): each goes to the frame's top.
    up = [{"command": "occurrence.ground", "params": {"occurrence": "B:1", "grounded": False}},
          {"command": "occurrence.move", "params": {"occurrence": "B:1", "translate": [125, 0, 100]}},
          {"command": "occurrence.ground", "params": {"occurrence": "B:1", "grounded": True}}]
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", "width": 250, "doors": 1}),
        plugin(approval, "cabinet", {"name": "B", "width": 250, "doors": 1, "place": "origin"}), *up,
        {"command": "occurrence.ground", "params": {"occurrence": "A:1", "grounded": False}},
        {"command": "occurrence.move", "params": {"occurrence": "A:1", "translate": [-125, 0, 0]}},
        {"command": "occurrence.ground", "params": {"occurrence": "A:1", "grounded": True}},
        pick("A Side L", "B Side L"),
        plugin(approval, "base", {"name": "Cart"}),
        MEASURE])
    summary, box = results[9]["result"], boxes(results[10])
    assert summary["raised"] == {"A": 170, "B": 70}, summary
    assert box["A Side L"][2] == 170 and box["B Side L"][2] == 170, (box["A Side L"], box["B Side L"])
    cross = box["Cart Frame cross 1"]
    assert cross[0] == -9 and cross[3] == 9 and cross[4] - cross[1] == 524, cross  # along the depth, at x = 0
