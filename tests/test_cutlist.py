"""Saving the cut list as a file, in a folder approved at install."""
from helpers import *


def test_a_board_turned_in_its_sketch(approval):
    # issue 39: a 150 x 800 rectangle drawn at 15 degrees and extruded 18 mm, listed as its bounding box
    board = [{"command": "sketch.create", "params": {"plane": "XY"}},
             {"command": "sketch.rectangle.three_point",
              "params": {"p0": [0, 0], "p1": [772.741, 207.055], "p2": [733.918, 351.944]}},
             {"command": "sketch.finish", "params": {}},
             {"command": "solid.extrude", "params": {"distance": "18 mm"}}]
    parts = cli(board + [plugin(approval, "cutlist")])[-1]["result"]["parts"]
    assert [(p["length"], p["width"], p["thickness"]) for p in parts] == [(800.0, 150.0, 18.0)], parts


def test_saved_as_a_file(approval):
    # A profile of its own, so installing with an approved folder leaves the user's SolidCraft alone.
    folder = ROOT / "dist" / "cutlists"
    with own_profile(folder) as install:
        grant = {"name": "cutlists", "path": str(folder), "read": False, "write": True}
        install["params"]["approve"]["folders"].append(grant)
        cabinet = plugin(approval, "cabinet", {})
        # a worktop, bought to length: not laid out on sheets
        worktop = {"command": "solid.box", "params": {"length": "2000 mm", "width": "600 mm", "height": "38 mm",
                   "corner": [1000, 0, 0], "name": "Worktop", "body_name": "Worktop"}}
        results = cli([install, cabinet, worktop, plugin(approval, "cutlist", {"file": "My kitchen", "format": "csv"}), plugin(approval, "cutlist"),
                       plugin(approval, "cutlist", {"file": "My kitchen"}), plugin(approval, "cutlist", {"file": "Plain", "diagrams": "no"}),
                       plugin(approval, "cutlist", {"file": "Stickers", "format": "labels"})])
        saved, shown = results[3]["result"], results[4]["result"]
        assert saved["saved"] == "My kitchen.csv" and "saved" not in shown, (saved.get("saved"), shown.get("saved"))
        assert results[5]["result"]["saved"] == "My kitchen.pdf"
        assert sorted(p.name for p in folder.iterdir()) == ["My kitchen.csv", "My kitchen.pdf", "Plain.pdf", "Stickers.pdf"]
        pdf = (folder / "My kitchen.pdf").read_bytes()
        assert pdf.startswith(b"%PDF-") and pdf.endswith(b"%%EOF\n") and b"Cabinet Side L" in pdf
        assert (folder / "My kitchen.csv").read_text(encoding="utf-8") == saved["csv"] == shown["csv"]
        # The sheets to buy are the cutting layouts', and the PDF draws them on a page of their own.
        assert sum(s["sheets"] for s in shown["sheets"]) == len(shown["layouts"]) >= 2, shown["sheets"]
        assert shown["layout_units"] == "mm" and shown["oversize"] == [] and shown["sheet_mm"] == [2440, 1220]
        assert [(w["part"], w["length"], w["width"], w["thickness"]) for w in shown["worktops"]] == [("Worktop", 2000, 600, 38)], shown["worktops"]
        assert all(p["part"] != "Worktop" for layout in shown["layouts"] for p in layout["parts"])
        assert all(s["thickness"] != 38 for s in shown["sheets"]), shown["sheets"]
        side = next(p for layout in shown["layouts"] for p in layout["parts"] if p["part"] == "Cabinet Side L")
        assert {side["w"], side["h"]} == {720, 560} and side["x"] >= 10 and side["y"] >= 10, side
        plain = (folder / "Plain.pdf").read_bytes()
        pages = lambda pdf: pdf.count(b"/Type /Page ")
        assert b"(Worktops, bought to length)" in pdf and b"(Cutting diagrams)" in pdf and b"sheet 1 of 1" in pdf and b"Cutting diagrams" not in plain
        assert pages(pdf) > pages(plain), (pages(pdf), pages(plain))
        # Labels: one per board, in the order of the cutting diagrams, then the worktop on no sheet.
        labels = (folder / "Stickers.pdf").read_bytes()
        assert results[7]["result"]["saved"] == "Stickers.pdf" and labels.endswith(b"%%EOF\n")
        on_sheets = [p["part"] for layout in shown["layouts"] for p in layout["parts"]]
        assert labels.count(b", sheet ") == len(on_sheets) and b"(bought to length)" in labels
        assert labels.count(b"0.6 w ") == shown["boards"] == len(on_sheets) + 1, shown["boards"]
        assert pages(labels) == -(-shown["boards"] // 24)
        assert labels.index(f"({on_sheets[0]})".encode()) < labels.index(b"(Worktop)")


def test_banding_and_grain(approval):
    # Birch plywood (grained), lower than deep, with an open door; and a narrow cabinet turned a
    # quarter, whose shelf's grain still runs along the cabinet's width.
    birch = {"height": 500, "plinth": 100, "doors": 1, "shelves": 1}
    turn = {"position": [2000, 0, 0], "axis": [0, 0, 1], "angle": "90 deg"}
    results = cli([plugin(approval, "cabinet", birch), drive("Cabinet Door hinge", 70),
                   plugin(approval, "cabinet", {"width": 400, "shelves": 1, "name": "T"}),
                   {"command": "occurrence.ground", "params": {"occurrence": "T:1", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "T:1", "to": turn}},
                   plugin(approval, "cutlist"), plugin(approval, "cutlist", {"banding": "2", "size": "cut"})])
    finished, cut = results[-2]["result"], results[-1]["result"]

    def row(listed, part):
        return next(p for p in listed["parts"] if part in p["parts"])

    door, side, shelf = (row(finished, p) for p in ("Cabinet Door", "Cabinet Side L", "T Shelf 1"))
    assert door["banding"] == {"length": 2, "width": 2} and door["grain"], door
    assert door["length"] < door["width"], door  # grain up: the length is the door's height
    assert side["banding"] == {"length": 1, "width": 0} and (side["length"], side["width"]) == (500, 560), side
    assert shelf["banding"] == {"length": 1, "width": 0} and shelf["length"] == 364, shelf
    assert finished["size"] == "finished" and finished["banding_mm"] == 1
    birch = next(b for b in finished["banding"] if b["thickness"] == 18)
    # fronts of sides, top, bottom, shelf, the door all round: 4.68 m and 2.53 m (T), plus 10 %
    assert abs(birch["metres"] - 7.93) < 0.05, finished["banding"]
    assert "Banding,Parts" in finished["csv"] and "Edge banding 1 mm" in finished["csv"]
    cut_door = row(cut, "Cabinet Door")
    assert (cut_door["length"], cut_door["width"]) == (door["length"] - 4, door["width"] - 4), cut_door
    assert row(cut, "Cabinet Side L")["width"] == 558 and cut["size"] == "cut"


def dxf_entities(path):
    """A DXF's entities as (type, {group code: value}), and its layer table."""
    lines = path.read_text(encoding="ascii").splitlines()
    pairs = list(zip(lines[::2], lines[1::2]))
    assert pairs[:4] == [("0", "SECTION"), ("2", "HEADER"), ("9", "$ACADVER"), ("1", "AC1009")] and pairs[-1] == ("0", "EOF")
    table = [b[1] for a, b in zip(pairs, pairs[1:]) if a == ("0", "LAYER")]
    start = pairs.index(("2", "ENTITIES"))
    entities = []
    for code, value in pairs[start + 1:]:
        if code == "0":
            entities.append((value, {}))
        else:
            entities[-1][1].setdefault(code, value)
    return entities, table


def test_saved_as_dxf(approval):
    # A cabinet with shelves on pins, two doors with hinge cups and knobs, moved off the origin:
    # one DXF per sheet, with every hole on the part it is drilled in.
    folder = ROOT / "dist" / "cutlists"
    with own_profile(folder) as install:
        grant = {"name": "cutlists", "path": str(folder), "read": False, "write": True}
        install["params"]["approve"]["folders"].append(grant)
        cabinet = plugin(approval, "cabinet", {"shelves": 2, "shelf_pins": True, "doors": 2, "handles": "knob"})
        moved = [{"command": "occurrence.ground", "params": {"occurrence": "Cabinet:1", "grounded": False}},
                 {"command": "occurrence.move", "params": {"occurrence": "Cabinet:1", "translate": [1000, 500, 300]}}]
        results = cli([install, plugin(approval, "settings", NONE), cabinet, *moved, MEASURE, plugin(approval, "cutlist", {"file": "Box.pdf", "format": "dxf"})])
        measure, listed = results[-2], results[-1]["result"]
        names = listed["saved"]
        assert sorted(names) == sorted(p.name for p in folder.iterdir()) and len(names) == len(listed["layouts"]) >= 2, names
        assert names == ["Box-Birch plywood-6-1.dxf", "Box-Birch plywood-18-1.dxf", "Box-Birch plywood-18-2.dxf"], names
        circles, layers = [], set()
        for name, layout in zip(names, listed["layouts"]):
            entities, table = dxf_entities(folder / name)
            used = {e[1]["8"] for e in entities if e[0] in ("POLYLINE", "CIRCLE", "TEXT")}
            assert set(table) == used and {"SHEET", "OUTLINE", "TEXT"} <= used, (table, used)
            kinds = lambda layer, kind: sum(1 for e in entities if e[0] == kind and e[1]["8"] == layer)
            assert kinds("OUTLINE", "POLYLINE") == kinds("TEXT", "TEXT") == len(layout["parts"])
            assert kinds("SHEET", "POLYLINE") == 2  # the sheet and its trim
            for e in entities:
                if e[0] == "CIRCLE":
                    x, y, r = float(e[1]["10"]), float(e[1]["20"]), float(e[1]["40"])
                    part = next(p for p in layout["parts"] if p["x"] < x < p["x"] + p["w"] and p["y"] < y < p["y"] + p["h"])
                    circles.append((e[1]["8"], part, x, y, r))
            layers |= used
        # Every hole the model has is drawn, or counted as drilled into an edge.
        holes = sum(b["face_types"].get("cylinder", 0) for b in boards(measure))
        assert len(circles) + listed["dxf_edge_holes"] == holes and "dxf_holes_skipped" not in listed, (len(circles), holes)
        hardware = {h["item"]: h["qty"] for h in listed["hardware"]}
        cups = [c for c in circles if c[0] == "DRILL_35_D13"]
        assert len(cups) == hardware["Hinge, cup"] == 4, layers
        # a cup's middle is 22.5 mm in from the door's hinge edge
        for _, p, x, y, _ in cups:
            assert "Door" in p["part"] and abs(min(x - p["x"], p["x"] + p["w"] - x, y - p["y"], p["y"] + p["h"] - y) - 22.5) < 0.1, (p, x, y)
        # The pins, and the hinge plates' screw holes: on the pins' grid, so here all of them are
        # pin holes.
        sides = [c for c in circles if c[0].startswith("DRILL_5_D1")]
        assert {c[0] for c in sides} == {"DRILL_5_D12"} and all("Side" in c[1]["part"] for c in sides), sides
        assert len(sides) == cylinders(measure, "Cabinet Side L") + cylinders(measure, "Cabinet Side R"), len(sides)
        knobs = [c for c in circles if c[0] == "DRILL_5_THRU"]
        assert len(knobs) == hardware["Knob"] == 2 and all("Door" in c[1]["part"] for c in knobs)
        # The sheet drawing reads back into a sketch.
        back = cli([{"command": "sketch.insert_dxf", "params": {"path": str(folder / names[0])}}])[0]
        assert back["curves"] >= 4 * (len(listed["layouts"][0]["parts"]) + 2), back
