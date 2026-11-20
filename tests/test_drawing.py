"""Drawing: a sheet per cabinet in the drawing Woodwork, with views, sizes, boards and its title."""
from pathlib import Path

from helpers import *


def step(command, **params):
    return {"command": command, "params": params}


def read(saved, *extra):
    """The drawing Woodwork of a saved design: its id, drawing.inspect's reply, and the replies
    to `extra` steps (given the sheets, they answer with steps of their own)."""
    listed = cli([step("drawing.list")], "--in", str(saved))[0]["drawings"]
    assert [d["name"] for d in listed] == ["Woodwork"], listed
    opened = step("drawing.open", drawing=listed[0]["id"])
    drawing = cli([opened], "--in", str(saved))[0]
    steps = [s for make in extra for s in make(drawing)]
    return drawing, cli([opened, *steps], "--in", str(saved))[1:]


def test_drawing_of_one_cabinet(approval):
    saved = ROOT / "dist" / "drawing.sc"
    cabinet = plugin(approval, "cabinet", {"doors": 2, "back": "overlay", "plinth": 100, "handles": "pull"})
    results = cli([cabinet, select("Cabinet Top"), plugin(approval, "drawing", {"scope": "selected"})],
                  "--save", str(saved))
    boards = [b.removeprefix("Cabinet ") for b in results[0]["result"]["bodies"]]
    assert results[2]["result"] == {"drawing": "Woodwork", "sheets": ["Cabinet"]}, results[2]

    def views(drawing):
        sheet = drawing["sheets"][0]
        ids = [v["id"] for v in sheet["views"]]
        dims = [d for v in sheet["views"] for d in v["dimensions"]]
        return ([step("drawing.view.inspect", view=v) for v in ids] +
                [step("drawing.dimension.inspect", dimension=d) for d in dims] +
                [step("drawing.note.inspect", note=n) for n in sheet["notes"]])

    def export(drawing):
        return [step("drawing.export", sheet="Cabinet", format="png", path=str(ROOT / "dist" / "drawing.png"), dpi=100)]

    drawing, replies = read(saved, views, export)
    [sheet] = drawing["sheets"]
    assert sheet["name"] == "Cabinet" and sheet["size_mm"] == [420.0, 297.0], sheet["name"]
    cells = {c["name"]: c.get("text") for c in sheet["title_block_cells"]}
    assert cells["title"] == "Cabinet", cells
    # Front, right and iso views of the cabinet, at one scale, its doors closed.
    kinds = sorted((v["kind"], v.get("side") or "") for v in sheet["views"])
    assert kinds == [("base", ""), ("base", ""), ("projected", "right")], kinds
    inspected = replies[:3]
    assert len({str(v["direction"]) for v in inspected}) == 3, [v["direction"] for v in inspected]
    assert all(v["settings"]["scale"]["value"] == "1:10" for v in inspected), [v["settings"]["scale"] for v in inspected]
    assert all(v["settings"]["pose"] != "designed" for v in inspected), [v["settings"]["pose"] for v in inspected]
    # Width, height and depth, the overlaid back included.
    values = sorted(round(d["value"], 3) for d in replies[3:6])
    assert values == [560, 600, 720], values
    assert all(d["status"] == "ok" for d in replies[3:6])
    # The board list: a note per column, every board in it.
    notes = {n["text"].splitlines()[0]: n["text"] for n in replies[6:-1]}
    assert sorted(notes) == ["L x W x T", "Material", "Part", "Qty"], notes
    parts = " ".join(notes["Part"].splitlines())
    assert all(board in parts for board in boards), (boards, parts)
    assert "720 x 554 x 18" in notes["L x W x T"], notes  # the sides, in front of the back
    assert replies[-1]["views"] and all(v["status"] == "drawn" for v in replies[-1]["views"])
    shots = Path(r"C:\Workspace\Privat\woodwork-night\shots")
    if shots.is_dir():
        shutil.copy(ROOT / "dist" / "drawing.png", shots / "shop-drawings-cabinet.png")


def test_drawing_of_every_cabinet_twice(approval):
    saved = ROOT / "dist" / "drawings.sc"
    every = plugin(approval, "drawing", {"scope": "all"})
    results = cli([plugin(approval, "cabinet", {}), select("Cabinet Side R"),
                   plugin(approval, "cabinet", {"place": "auto", "width": 400, "drawers": 2, "shelves": 0}),
                   every, every], "--save", str(saved))
    assert results[3]["result"]["sheets"] == ["Cabinet", "Cabinet 2"], results[3]
    # Drawn again: each cabinet's sheet is replaced, not added.
    drawing, _ = read(saved)
    assert sorted(s["name"] for s in drawing["sheets"]) == ["Cabinet", "Cabinet 2"], drawing["sheets"]
    # The second cabinet's sizes, in its own sheet.
    second = next(s for s in drawing["sheets"] if s["name"] == "Cabinet 2")
    dims = [d for v in second["views"] for d in v["dimensions"]]
    _, replies = read(saved, lambda _: [step("drawing.dimension.inspect", dimension=d) for d in dims])
    assert sorted(round(d["value"]) for d in replies) == [400, 560, 720], replies


def test_drawing_in_mm_in_an_inch_design(approval):
    saved = ROOT / "dist" / "drawing-in.sc"
    cabinet = plugin(approval, "cabinet", {"units": "in", "width": 24, "height": 30, "depth": 22})
    draw = plugin(approval, "drawing", {"scope": "all"})
    cli([step("document.units", units="in"), cabinet, draw], "--save", str(saved))
    # Set to inches by hand, then drawn again: back in mm.
    listed = cli([step("drawing.list")], "--in", str(saved))[0]["drawings"]
    cli([step("drawing.open", drawing=listed[0]["id"]), step("drawing.set", units="in"), step("drawing.close"), draw],
        "--in", str(saved), "--save", str(saved))

    def dims(drawing):
        sheet = drawing["sheets"][0]
        return [step("drawing.dimension.inspect", dimension=d) for v in sheet["views"] for d in v["dimensions"]]

    drawing, replies = read(saved, dims)
    assert drawing["units"] == {"fixed": {"unit": "mm"}}, drawing["units"]
    texts = sorted(d["text"] for d in replies)
    assert texts == ["559", "610", "762"], texts  # 22, 24 and 30 in, as whole mm


def test_drawing_of_appliance_cabinets(approval):
    # A tall cabinet and a sink base right of it: a sheet for each cabinet.
    saved = ROOT / "dist" / "drawing-kinds.sc"
    results = cli([plugin(approval, "cabinet", {"name": "O", "height": 2100, "doors": 1}),
                   select("O Side R"), plugin(approval, "cabinet", {"name": "S", "kind": "sink", "doors": 2, "place": "auto"}),
                   plugin(approval, "drawing", {"scope": "all"})], "--save", str(saved))
    assert results[3]["result"]["sheets"] == ["O", "S"], results[3]
    drawing, _ = read(saved)
    sheets = {s["name"]: s for s in drawing["sheets"]}
    dims = [d for v in sheets["S"]["views"] for d in v["dimensions"]]
    notes = sheets["S"]["notes"] + sheets["O"]["notes"]
    _, replies = read(saved, lambda _: [step("drawing.dimension.inspect", dimension=d) for d in dims] +
                      [step("drawing.note.inspect", note=n) for n in notes])
    # 600 x 720, 560 deep, measured on the sides
    assert sorted(round(d["value"]) for d in replies[:3]) == [560, 600, 720], replies[:3]
    assert all(d["status"] == "ok" for d in replies[:3])
    parts = " ".join(n["text"] for n in replies[3:])
    assert all(b in parts for b in ("Rail front", "Rail back", "Shelf 1")), parts
