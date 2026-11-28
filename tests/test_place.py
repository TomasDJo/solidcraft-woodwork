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
                   plugin(approval, "corner", {"name": "U", "place": "right"}), select("S Top"),
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


def test_auto_goes_by_the_selected_board(approval):
    # Auto: left of a Side L, right of a Side R, above a Top, and at the origin with nothing
    # or another board selected. The board comes from the dialog's field here (SolidCraft 0.44 sends what was
    # picked in it as selection references: a body, or a face or an edge of one), which goes
    # before the selection: Edit, Joinery and Material read it the same way.
    auto = {"width": 400, "place": "auto"}
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "A", "width": 600}),
                   select("A Side R"),
                   plugin(approval, "cabinet", {**auto, "name": "L", "selection": [{"type": "face", "body": "A Side L", "index": 0}]}),
                   select("A Side R"),  # a run lets the selection go
                   plugin(approval, "cabinet", {**auto, "name": "R"}),  # the selection: A Side R
                   plugin(approval, "corner", {"place": "auto", "name": "T", "selection": [{"type": "body", "name": "A Top"}]}),
                   pick(),
                   plugin(approval, "cabinet", {**auto, "name": "O"}),
                   # joinery is design-wide: set in Woodwork settings, then applied to L alone
                   setting(approval, {"joints": "confirmat"}),
                   plugin(approval, "rebuild", {"with": "settings", "selection": ["L Shelf 1"]}),
                   setting(approval, {"joints": "none"}),
                   plugin(approval, "material", {"material": "Oak", "scope": "parts", "selection": ["R Top", "R Bottom"]}),
                   plugin(approval, "edit", {"shelves": 3, "selection": ["R Side L"]}),
                   MEASURE])
    box = boxes(results[11])
    assert box["L Side R"][3] == -300 and box["R Side L"][0] == 300, (box["L Side R"], box["R Side L"])
    assert box["T Side L"][0] == -300 and box["T Side L"][2] == 720, box["T Side L"]  # above, left sides flush
    assert box["O Side L"][0] == -200, box["O Side L"]  # nothing selected: at the origin
    assert cylinders(results[11], "L Side L") > 0 and cylinders(results[11], "R Side L") == 0
    material = {b["name"]: b["material"] for b in results[11]["bodies"]}
    assert material["R Bottom"] == "Oak" and material["R Side L"] != "Oak", material
    assert "R Shelf 3" in box and "L Shelf 3" not in box, sorted(box)
    # any other board does not say which side: at the origin, as with nothing selected
    results = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "A"}),
                   plugin(approval, "cabinet", {**auto, "name": "B", "selection": ["A Shelf 1"]}),
                   MEASURE])
    assert boxes(results[2])["B Side L"][0] == -200, boxes(results[2])["B Side L"]


def test_framed_on_the_whole_model(approval):
    # A run that adds or rebuilds a cabinet ends with Fit (0.45), which the window applies. Fit
    # frames the selection when there is one, so the board placed against is let go first.
    results = cli([plugin(approval, "cabinet", {"name": "A"}), select("A Side R"),
                   plugin(approval, "cabinet", {"name": "B", "place": "right"}),
                   {"command": "selection.get", "params": {}}, select("B Top"),
                   plugin(approval, "edit", {"shelves": 2}), select("B Top"), plugin(approval, "material")])
    assert [results[i].get("view") for i in (0, 2, 5, 7)] == [{"kind": "fit"}] * 3 + [None], results
    assert results[3]["items"] == [], results[3]


def test_along_a_sketch_line_with_its_width(approval):
    # Along a line (#147): a floor line from (200, 500) to (1400, 500) with an unnamed length
    # dimension. The cabinet's back is on it, facing -Y (nothing else in the sketch), its
    # left side at the left end, and Width from line names the dimension and follows it:
    # 1500 mm makes the cabinet 1500 wide with no rebuild.
    line = [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]
    along = {"place": "line", "line": line, "width_from_line": True, "depth": 400}
    results = designed(approval, NONE, [
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Wall line"}},
        {"command": "sketch.line", "params": {"points": [[200, 500], [1400, 500]]}},
        {"command": "sketch.dimension", "params": {"entities": ["l1"], "value": "1200 mm"}},
        {"command": "sketch.finish", "params": {}},
        plugin(approval, "cabinet", {**along, "name": "A"}),
        MEASURE,
        {"command": "parameters.change", "params": {"name": "Wall_line_l1", "expression": "1500 mm"}},
        MEASURE,
        {"command": "parameters.list", "params": {"filter": "A_width"}}])
    summary, before, after = results[4]["result"], boxes(results[5]), boxes(results[7])
    assert "width follows Wall_line_l1" in summary["line"], summary
    assert before["A Side L"][0] == 200 and before["A Side R"][3] == 1400, (before["A Side L"], before["A Side R"])
    assert before["A Side L"][4] == 500 and before["A Side L"][1] == 100, before["A Side L"]  # back on the line
    # it grows away from its Start at end: the left side stays at the line's left end
    assert after["A Side L"][0] == 200 and after["A Side R"][3] == 1700, (after["A Side L"], after["A Side R"])
    width = [p for p in results[8]["parameters"] if p["name"] == "A_width"]
    assert width[0]["expression"] == "Wall_line_l1", width


def test_along_a_wall_face_and_a_flipped_edge(approval):
    # A wall 2000 long from (0, 500): its -Y face stands for its bottom edge, and the cabinet
    # faces away from it, its right side 100 from the right end. The +Y bottom edge faces +Y,
    # away from the wall; flipped, it faces -Y, its back at y = 600 (the test only checks the
    # side it takes, not that it overlaps the wall).
    wall = {"command": "solid.box", "params": {"name": "Wall", "body_name": "Wall", "corner": [0, 500, 0],
                                               "length": 2000, "width": 100, "height": 2400}}
    face = [{"type": "face", "body": "Wall", "index": 3}]
    results = designed(approval, NONE, [
        wall,
        {"command": "model.edges", "params": {"body": "Wall"}},
        plugin(approval, "cabinet", {"name": "B", "width": 600, "depth": 400, "place": "line", "line": face,
                                     "start_at": "right", "offset": 100}),
        MEASURE])
    edges = results[1]["edges"]
    back = next(e["index"] for e in edges if e["name"] == "F1:+y|F1:-z")
    box = boxes(results[3])
    assert box["B Side R"][3] == 1900 and box["B Side L"][0] == 1300, (box["B Side L"], box["B Side R"])
    assert box["B Side L"][4] == 500 and box["B Side L"][1] == 100, box["B Side L"]  # facing away from the wall
    # the back edge faces +Y (away from the wall); flipped it faces -Y, into the wall
    edge = [{"type": "edge", "body": "Wall", "index": back}]
    results = designed(approval, NONE, [
        wall,
        plugin(approval, "cabinet", {"name": "C", "width": 600, "depth": 400, "place": "line", "line": edge}),
        plugin(approval, "cabinet", {"name": "D", "width": 600, "depth": 400, "place": "line", "line": edge,
                                     "start_at": "middle", "faces": "flip"}),
        MEASURE])
    box, summary = boxes(results[3]), results[1]["result"]
    assert "Apply settings reads the line again" in summary["line"], summary
    # C faces +Y, turned half: its left side (Side L) at the line's +X end
    assert box["C Side L"][3] == 2000 and box["C Side L"][1] == 600 and box["C Side L"][4] == 1000, box["C Side L"]
    # D faces -Y, its back on y = 600, in the middle
    assert box["D Side L"][0] == 700 and box["D Side L"][4] == 600, box["D Side L"]


def test_height_from_a_line_and_a_deeper_cabinet_keep_to_the_line(approval):
    # Height from line (#147 step 2): a second sketch's 800 mm line, dimensioned, makes the
    # cabinet 800 tall, and 900 mm then 900; made deeper, its back stays on the floor line.
    along = {"place": "line", "line": [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}],
             "height_line": [{"type": "external_sketch_curve", "sketch": 2, "id": "l1"}], "depth": 400}
    results = designed(approval, NONE, [
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Floor"}},
        {"command": "sketch.line", "params": {"points": [[200, 500], [1400, 500]]}},
        {"command": "sketch.finish", "params": {}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Window"}},
        {"command": "sketch.line", "params": {"points": [[0, 2000], [800, 2000]]}},
        {"command": "sketch.dimension", "params": {"entities": ["l1"], "value": "800 mm"}},
        {"command": "sketch.finish", "params": {}},
        plugin(approval, "cabinet", {**along, "name": "A", "width": 600}),
        MEASURE,
        {"command": "parameters.change", "params": {"name": "Window_l1", "expression": "900 mm"}},
        {"command": "parameters.change", "params": {"name": "A_depth", "expression": "300 mm"}},
        MEASURE])
    summary, before, after = results[7]["result"], boxes(results[8]), boxes(results[11])
    assert "height follows Window_l1" in summary["line"], summary
    assert before["A Side L"][5] == 800 and after["A Side L"][5] == 900, (before["A Side L"], after["A Side L"])
    assert before["A Side L"][4] == 500 and after["A Side L"][4] == 500, after["A Side L"]  # back on the line
    assert after["A Side L"][1] == 200 and after["A Side L"][0] == 200, after["A Side L"]


def test_apply_settings_follows_a_wall_that_changed(approval):
    # A wall whose length is a parameter: the cabinet along its face, as wide as it less 100,
    # is read once; the wall made longer, Apply settings reads the face again and the cabinet
    # is 400 wider, its left side still 100 from the wall's end. An Edit keeps it there too.
    wall = [{"command": "parameters.add", "params": {"name": "WallLen", "expression": "2000 mm"}},
            {"command": "solid.box", "params": {"name": "Wall", "body_name": "Wall", "corner": [0, 500, 0],
                                                "length": "WallLen", "width": 100, "height": 2400}}]
    face = [{"type": "face", "body": "Wall", "index": 3}]
    results = designed(approval, NONE, [
        *wall,
        plugin(approval, "cabinet", {"name": "B", "depth": 400, "doors": 2, "place": "line", "line": face,
                                     "offset": 100, "width_from_line": True}),
        {"command": "parameters.change", "params": {"name": "WallLen", "expression": "2400 mm"}},
        MEASURE,
        plugin(approval, "rebuild", {"with": "settings"}),
        MEASURE,
        plugin(approval, "edit", {"shelves": 2, "selection": ["B Side L"]}),
        MEASURE])
    summary, once, applied, edited = results[2]["result"], boxes(results[4]), boxes(results[6]), boxes(results[8])
    assert "width read from the line" in summary["line"] and "Apply settings" in summary["line"], summary
    assert once["B Side L"][0] == 100 and once["B Side R"][3] == 2000, (once["B Side L"], once["B Side R"])
    assert results[5]["result"]["changed"]["B"] == ["line"], results[5]["result"]
    assert applied["B Side L"][0] == 100 and applied["B Side R"][3] == 2400, (applied["B Side L"], applied["B Side R"])
    assert applied["B Side L"][4] == 500, applied["B Side L"]
    assert edited["B Side R"][3] == 2400 and "B Shelf 2" in edited, edited["B Side R"]
    # the wall gone: Apply settings leaves the cabinet where it stands and says why
    results = designed(approval, NONE, [
        *wall,
        plugin(approval, "cabinet", {"name": "B", "depth": 400, "place": "line", "line": face, "offset": 100}),
        {"command": "timeline.suppress", "params": {"feature": "Wall"}},
        plugin(approval, "rebuild", {"with": "settings"}),
        MEASURE])
    assert "left where it stood" in results[4]["result"]["lines"]["B"], results[4]["result"]
    assert boxes(results[5])["B Side L"][0] == 100, boxes(results[5])["B Side L"]


def test_an_inch_cabinet_rebuilt_keeps_following_its_line(approval):
    # Inches, Width from line with a 2 in offset: Apply settings builds it again (the width
    # read again is 45.244094 in, a hair off 1149.2 mm), and the width stays the dimension's
    # expression, so 1500 mm afterwards still makes it 1500 - 50.8 wide, its left side put.
    line = [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]
    results = designed(approval, NONE, [
        {"command": "document.units", "params": {"units": "in"}},
        {"command": "sketch.create", "params": {"plane": "XY", "name": "Wall line"}},
        {"command": "sketch.line", "params": {"points": [[200, 500], [1400, 500]]}},
        {"command": "sketch.dimension", "params": {"entities": ["l1"], "value": "1200 mm"}},
        {"command": "sketch.finish", "params": {}},
        plugin(approval, "cabinet", {"name": "A", "units": "in", "depth": 16, "place": "line", "line": line,
                                     "offset": 2, "width_from_line": True}),
        plugin(approval, "rebuild", {"with": "settings"}),
        {"command": "parameters.change", "params": {"name": "Wall_line_l1", "expression": "1500 mm"}},
        MEASURE,
        {"command": "parameters.list", "params": {"filter": "A_width"}}])
    assert results[6]["result"]["applied"] == ["A"], results[6]["result"]
    box = boxes(results[8])
    assert box["A Side L"][0] == 250.8 and box["A Side R"][3] == 1700, (box["A Side L"], box["A Side R"])
    width = [p for p in results[9]["parameters"] if p["name"] == "A_width"]
    assert width[0]["expression"] == "Wall_line_l1 - 2 in", width


def test_a_selected_sketch_line_sets_place_along_it(approval):
    # #212: with a sketch line selected, a run that leaves Place and Line out (as the dialog
    # opens) gets them from the hook: Along a line, on the selected line. The cabinet stands as
    # one placed on that line by hand.
    sketch = [{"command": "sketch.create", "params": {"plane": "XY", "name": "Wall line"}},
              {"command": "sketch.line", "params": {"points": [[200, 500], [1400, 500]]}},
              {"command": "sketch.finish", "params": {}}]
    left_out = entry_step(approval, "cabinet", {"name": "A", "depth": 400})
    # and the fields Along a line shows, as plugin.run takes a given field Place Auto hides out
    for key in ("place", "line", "height_line", "width_from_line", "start_at", "offset", "faces"):
        left_out["params"]["params"].pop(key, None)
    by_hand = {"name": "A", "depth": 400, "place": "line",
               "line": [{"type": "external_sketch_curve", "sketch": 1, "id": "l1"}]}
    selected = {"command": "select.set", "params": {"items": [{"type": "sketch_curve", "sketch": 1, "id": "l1"}]}}
    auto = cli([*sketch, selected, left_out, MEASURE])
    hand = cli([*sketch, plugin(approval, "cabinet", by_hand), MEASURE])
    assert auto[4]["result"].get("line"), auto[4]["result"]
    assert boxes(auto[-1])["A Side L"] == boxes(hand[-1])["A Side L"], (boxes(auto[-1])["A Side L"], boxes(hand[-1])["A Side L"])
