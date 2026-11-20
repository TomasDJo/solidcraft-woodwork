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
