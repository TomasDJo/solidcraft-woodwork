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


def test_cabinets_back_to_back_of_different_widths_get_boards_that_do_not_cross(approval):
    # #216: A 600 wide, B 800 wide turned round behind it, back to back, both from A's left side
    # (as Along a line places them). The frame's boards and blocks share no volume: the board
    # along the frame at A's right side stops at the board across where A and B meet.
    turned = [{"command": "occurrence.ground", "params": {"occurrence": "B:1", "grounded": False}},
              {"command": "occurrence.move", "params": {"occurrence": "B:1", "to_matrix": [
                  -1, 0, 0, 100, 0, -1, 0, 560, 0, 0, 1, 0]}},
              {"command": "occurrence.ground", "params": {"occurrence": "B:1", "grounded": True}}]
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", "width": 600}),
        plugin(approval, "cabinet", {"name": "B", "width": 800, "place": "origin"}), *turned,
        pick("A Side L", "B Side L"),
        plugin(approval, "base", {"name": "Cart"}),
        MEASURE])
    box = boxes(results[7])
    assert box["B Side L"][1] == 280 and box["B Side L"][4] == 840, box["B Side L"]
    frame = {n: b for n, b in box.items() if n.startswith(("Cart Frame", "Cart Block"))}
    names = sorted(frame)
    crossing = [(a, b) for i, a in enumerate(names) for b in names[i + 1:]
                if all(frame[a][k] < frame[b][k + 3] - 0.01 and frame[b][k] < frame[a][k + 3] - 0.01 for k in range(3))]
    assert crossing == [], crossing
    assert box["Cart Frame cross 2"][0] == 291 and box["Cart Frame cross 2"][4] == 271, box["Cart Frame cross 2"]


def test_a_timber_frame_is_bought_by_length(approval):
    # Frame of Timber, 48 x 98 by default: the frame stands 98 tall and 48 across, in softwood
    # timber, and the cabinet goes up by caster plus 98. The cut list buys it in running metres per
    # section, prices it per running metre (`Timber 48x98`), lays out no sheet for it, and lists
    # the caster blocks as pieces of their own size, out of the metres.
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", "width": 600}),
        pick("A Side L"),
        plugin(approval, "base", {"name": "Cart", "frame_material": "timber"}),
        MEASURE,
        setting(approval, {"prices": "Timber 48x98 = 3.5"}),
        plugin(approval, "cutlist", {})])
    box, listed = boxes(results[3]), results[4]["result"]
    assert box["Cart Frame front"] == [-300, -280, 100, 300, -232, 198], box["Cart Frame front"]
    assert box["A Side L"][2] == 198, box["A Side L"]
    timber = {t["part"]: (t["material"], t["thickness"], t["width"]) for t in listed["timber"]}
    assert timber["Cart Frame front"] == ("Softwood timber", 48, 98), timber
    sections = {m["section"]: m["metres"] for m in listed["timber_metres"]}
    assert list(sections) == ["48 x 98 mm"], sections
    blocks = [t for t in listed["timber"] if t.get("note") == "caster block"]
    assert len(blocks) == 4 and all(b["part"].startswith("Cart Block ") for b in blocks), blocks
    lines = {l["item"]: l for l in listed["cost"]["lines"]}
    assert lines["Timber 48x98"]["total"] == round(3.5 * sections["48 x 98 mm"], 2), lines
    assert all(s["material"] != "Softwood timber" for s in listed["sheets"]), listed["sheets"]


def test_a_long_base_has_casters_evenly_spaced(approval):
    # #228: three 1100 mm cabinets, a 3300 mm frame: casters in the corners and two pairs more,
    # evenly spaced (at most 1500 apart), each block moved off the cross board it would stand on,
    # so no two of the frame's boards and blocks share volume (stiffeners included).
    bare = {"width": 1100, "shelves": 0, "doors": 0}
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", **bare}), select("A Side R"),
        plugin(approval, "cabinet", {"name": "B", "place": "right", **bare}), select("B Side R"),
        plugin(approval, "cabinet", {"name": "C", "place": "right", **bare}),
        pick("A Side L", "B Side L", "C Side L"),
        plugin(approval, "base", {"name": "Long"}),
        MEASURE])
    assert results[6]["result"]["casters"] == 8, results[6]["result"]
    box = boxes(results[7])
    front = sorted(b[0] for n, b in box.items() if n.startswith("Long Block ") and b[1] < -250)
    assert front == [-532, 559, 1551, 2642], front
    # #229: a stiffener in each stretch between the ends and the cross boards (each about 1100 mm,
    # over the 600 mm spacing), half way
    stiffeners = sorted(b[0] + 9 for n, b in box.items() if n.startswith("Long Frame stiffener"))
    assert stiffeners == [4.5, 1100, 2195.5], stiffeners
    frame = {n: b for n, b in box.items() if n.startswith(("Long Frame", "Long Block"))}
    names = sorted(frame)
    crossing = [(a, b) for i, a in enumerate(names) for b in names[i + 1:]
                if all(frame[a][k] < frame[b][k + 3] - 0.01 and frame[b][k] < frame[a][k + 3] - 0.01 for k in range(3))]
    assert crossing == [], crossing


def test_close_cross_boards_keep_the_casters_clear(approval):
    # #228 review: two rows back to back, staggered (1100, 1100, 1100 in front; 1150, 1050, 1100
    # behind, turned round): cross boards 50 mm apart at x = 550 and 600. The middle block finds a
    # place clear of both, the casters stay within 1500 mm, and nothing overlaps.
    bare = {"shelves": 0, "doors": 0}
    steps = [plugin(approval, "cabinet", {"name": "A1", "width": 1100, **bare}), select("A1 Side R"),
             plugin(approval, "cabinet", {"name": "A2", "width": 1100, "place": "right", **bare}), select("A2 Side R"),
             plugin(approval, "cabinet", {"name": "A3", "width": 1100, "place": "right", **bare})]
    for name, width, x in [("B1", 1150, 25), ("B2", 1050, 1125), ("B3", 1100, 2200)]:
        steps += [plugin(approval, "cabinet", {"name": name, "width": width, "place": "origin", **bare}),
                  {"command": "occurrence.ground", "params": {"occurrence": f"{name}:1", "grounded": False}},
                  {"command": "occurrence.move", "params": {"occurrence": f"{name}:1", "to_matrix": [
                      -1, 0, 0, x, 0, -1, 0, 560, 0, 0, 1, 0]}},
                  {"command": "occurrence.ground", "params": {"occurrence": f"{name}:1", "grounded": True}}]
    steps += [pick(*[f"{c} Side L" for c in ["A1", "A2", "A3", "B1", "B2", "B3"]]),
              plugin(approval, "base", {"name": "Pair"}), MEASURE]
    results = designed(approval, NONE, steps)
    box = boxes(results[-1])
    starts = sorted({b[0] for n, b in box.items() if n.startswith("Pair Block ")})
    assert all(b - a <= 1500 for a, b in zip(starts, starts[1:])), starts
    frame = {n: b for n, b in box.items() if n.startswith(("Pair Frame", "Pair Block"))}
    names = sorted(frame)
    crossing = [(a, b) for i, a in enumerate(names) for b in names[i + 1:]
                if all(frame[a][k] < frame[b][k + 3] - 0.01 and frame[b][k] < frame[a][k + 3] - 0.01 for k in range(3))]
    assert crossing == [], crossing


def test_joints_that_leave_no_room_for_casters_are_refused(approval):
    # #228 review: thirteen 150 mm cabinets in a row under 150 mm timber: the cross boards at their
    # joints touch each other over 1560 mm, with no place for a caster block between the corners,
    # so the base is refused with a message, not made with casters too far apart.
    bare = {"width": 150, "shelves": 0, "doors": 0}
    steps = [plugin(approval, "settings", NONE), plugin(approval, "cabinet", {"name": "N1", **bare})]
    for i in range(2, 14):
        steps += [select(f"N{i - 1} Side R"), plugin(approval, "cabinet", {"name": f"N{i}", "place": "right", **bare})]
    steps += [pick(*[f"N{i} Side L" for i in range(1, 14)]),
              plugin(approval, "base", {"name": "Row", "frame_material": "timber", "timber_thickness": 150})]
    error = refused(steps)
    assert "no room for the caster blocks" in error, error


def test_a_stiffener_on_a_corner_block_is_refused(approval):
    # #229 review: a 600 x 600 cabinet, 150 mm timber, 300 mm Stiffener spacing: the stiffener
    # would stand on the corner blocks, so the base is refused with a word on the spacing.
    error = refused([plugin(approval, "settings", NONE),
                     plugin(approval, "cabinet", {"name": "A", "width": 600, "depth": 600, "shelves": 0, "doors": 0}),
                     pick("A Side L"),
                     plugin(approval, "base", {"name": "Cart", "frame_material": "timber", "timber_thickness": 150,
                                               "stiffener_spacing": 300})])
    assert "stiffener spacing" in error, error


def test_connector_bolts_join_the_cabinets_on_a_base(approval):
    # A and B side by side on a base: four 9 mm holes through A Side R and B Side L, counted once
    # as connector bolts. Run again with the cabinets picked the other way round, no hole is cut
    # twice.
    bare = {"width": 600, "shelves": 0, "doors": 0}
    steps = [plugin(approval, "cabinet", {"name": "A", **bare}), select("A Side R"),
             plugin(approval, "cabinet", {"name": "B", "place": "right", **bare}),
             pick("A Side L", "B Side L"), plugin(approval, "base", {"name": "Bench"}),
             pick("B Side L", "A Side L"), plugin(approval, "base", {"name": "Bench"}), MEASURE,
             plugin(approval, "cutlist", {})]
    results = designed(approval, NONE, steps)
    assert results[4]["result"]["bolts"] == 4, results[4]["result"]
    measure = results[7]
    for touching, free in (("A Side R", "A Side L"), ("B Side L", "B Side R")):
        assert cylinders(measure, touching) - cylinders(measure, free) == 4, (touching, cylinders(measure, touching))
    hardware = {h["item"]: h["qty"] for h in results[8]["result"]["hardware"]}
    assert hardware.get("Connector bolt, M6") == 4, hardware


def test_screws_fix_the_cabinets_to_the_frame(approval):
    # A on a base, Plinth No: its bottom rests on the frame's front and back boards, two 4.5 mm
    # holes down through it into each, counted as frame screws. Run again, none is cut twice;
    # run again with 40 mm frame boards, the old holes go and four new ones are cut on their
    # middle lines.
    steps = [plugin(approval, "cabinet", {"name": "A", "width": 600, "shelves": 0, "doors": 0}),
             pick("A Side L"), plugin(approval, "base", {"name": "Bench"}),
             pick("A Side L"), plugin(approval, "base", {"name": "Bench"}), MEASURE, plugin(approval, "cutlist", {}),
             pick("A Side L"), plugin(approval, "base", {"name": "Bench", "thickness": 40}), MEASURE,
             plugin(approval, "cutlist", {}), faces("A Bottom"),
             {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}]
    results = designed(approval, NONE, steps)
    assert results[2]["result"]["screws"] == 4, results[2]["result"]
    holes = cylinders(results[5], "A Bottom")
    assert holes >= 4, holes
    hardware = {h["item"]: h["qty"] for h in results[6]["result"]["hardware"]}
    assert hardware.get("Frame screw, 4 x 50 mm") == 4, hardware
    assert cylinders(results[9], "A Bottom") == holes, cylinders(results[9], "A Bottom")
    hardware = {h["item"]: h["qty"] for h in results[10]["result"]["hardware"]}
    assert hardware.get("Frame screw, 4 x 50 mm") == 4, hardware
    # the holes are on the 40 mm boards' middle lines (20 mm in from the front and the back,
    # not 9), and only the four new cuts are left in the timeline
    after = results[10:]
    front, back = boxes(results[9])["Bench Frame front"][1], boxes(results[9])["Bench Frame back"][4]
    # (a face's centroid is of its tessellation: to the millimetre)
    ys = sorted({round(f["centroid"][1]) for f in after[1]["faces"]
                 if abs((f.get("cylinder") or {}).get("radius", 0) - 2.25) < 0.01})
    assert ys == [round(front + 20), round(back - 20)], (ys, front, back)
    cuts = [f["name"] for f in after[2]["timeline"] if " frame screw " in f["name"]]
    assert len(cuts) == 4 and all(f"{round(front + 20, 1):g}" in c or f"{round(back - 20, 1):g}" in c for c in cuts), cuts


def test_cabinets_ride_on_the_base_through_edit_and_a_new_frame(approval):
    # #215: a cabinet Along a line, then a base under it: it is held to the base by a link
    # (`A link`). Edit (a wider cabinet) keeps it on the frame, where before it went back to
    # the line's height; so does Apply settings after the line got longer. Run again with a
    # taller frame, the cabinet goes up with it and is held again.
    line = [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]
    results = designed(approval, NONE, [
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Wall line"}},
        {"command": "sketch.line", "params": {"points": [[200, 500], [1400, 500]]}},
        {"command": "sketch.dimension", "params": {"entities": ["l1"], "value": "1200 mm"}},
        {"command": "sketch.finish", "params": {}},
        plugin(approval, "cabinet", {"name": "A", "depth": 400, "place": "line", "line": line}),
        pick("A Side L"), plugin(approval, "base", {"name": "Bench"}),
        {"command": "joint.list", "params": {}},
        select("A Side L"), plugin(approval, "edit", {"width": 700}), MEASURE,
        {"command": "parameters.change", "params": {"name": "d1", "expression": "1500 mm"}},
        plugin(approval, "rebuild", {}), MEASURE,
        pick(), plugin(approval, "base", {"name": "Bench", "frame_height": 100}), MEASURE,
        {"command": "joint.list", "params": {}}])
    assert "A link" in [j["name"] for j in results[7]["joints"]], results[7]
    edited = boxes(results[10])
    assert edited["A Side L"][2] == 170 and round(edited["A Side R"][3] - edited["A Side L"][0]) == 700, edited["A Side L"]
    applied = boxes(results[13])
    assert applied["A Side L"][2] == 170, applied["A Side L"]
    taller = boxes(results[16])
    assert taller["A Side L"][2] == 200 and taller["Bench Frame front"][5] == 200, (taller["A Side L"], taller["Bench Frame front"])
    assert "A link" in [j["name"] for j in results[17]["joints"]], results[17]


def test_a_base_under_cabinets_along_a_line_follows_the_line(approval):
    # #215 step 2: A along a line from its right end, a base under it. The line grows from
    # 1200 to 1500 mm (its right end moves 300 to the right); Apply settings puts A back on the
    # line and makes the base again under it: both 300 further right, A still on the frame and
    # held to it.
    line = [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]
    results = designed(approval, NONE, [
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Wall line"}},
        {"command": "sketch.line", "params": {"points": [[200, 500], [1400, 500]]}},
        {"command": "sketch.dimension", "params": {"entities": ["l1"], "value": "1200 mm"}},
        {"command": "sketch.finish", "params": {}},
        plugin(approval, "cabinet", {"name": "A", "depth": 400, "place": "line", "line": line, "start_at": "right"}),
        pick("A Side L"), plugin(approval, "base", {"name": "Bench"}), MEASURE,
        {"command": "parameters.change", "params": {"name": "d1", "expression": "1500 mm"}},
        plugin(approval, "rebuild", {}), MEASURE, {"command": "joint.list", "params": {}}])
    before, after = boxes(results[7]), boxes(results[10])
    assert after["A Side R"][3] - before["A Side R"][3] == 300, (before["A Side R"], after["A Side R"])
    assert after["Bench Frame front"][3] - before["Bench Frame front"][3] == 300, after["Bench Frame front"]
    assert after["A Side L"][2] == 170, after["A Side L"]
    assert "A link" in [j["name"] for j in results[11]["joints"]], results[11]
