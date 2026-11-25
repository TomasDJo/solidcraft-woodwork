"""A cabinet placed against another is held to it: it follows when either changes size."""
from helpers import *


def change(**to):
    return {"command": "parameters.change", "params": {"changes": {name: f"{mm} mm" for name, mm in to.items()}}}


RUN = [("cabinet", {"name": "A"}), ("cabinet", {"name": "B", "place": "right"}),
       ("cabinet", {"name": "C", "place": "right", "doors": 1})]


def run(approval, cabinets=RUN):
    """The cabinets one after the other, each placed against the one before it."""
    steps = []
    for i, (entry, params) in enumerate(cabinets):
        if i:
            steps.append(select(f"{cabinets[i - 1][1]['name']} Top"))
        steps.append(plugin(approval, entry, params))
    return steps


def test_a_run_follows_a_change_of_width(approval):
    # Three cabinets in a row, 600 wide. The middle one made 900 wide keeps its left side
    # against the first and pushes the third along. The first made 800 wide grows both ways,
    # as it stands on its own middle, and the two others move over by half of that.
    results = cli([*run(approval), MEASURE, change(B_width=900), MEASURE, change(A_width=800),
                   drive("C Door hinge", 90), MEASURE])
    left = lambda measure: [boxes(measure)[f"{name} Side L"][0] for name in "ABC"]
    assert left(results[5]) == [-300, 300, 900], left(results[5])
    assert left(results[7]) == [-300, 300, 1200], left(results[7])
    assert left(results[10]) == [-400, 400, 1300], left(results[10])
    # the third cabinet is held, not grounded, and its door still opens against it
    door = boxes(results[10])["C Door"]
    assert door[1] < -800 and door[3] - door[0] == 18, door


def test_left_and_above_follow_too(approval):
    # B to the left of A and D on top of it. A made wider and higher: B keeps standing against
    # its left side, D on its top.
    results = cli([plugin(approval, "cabinet", {"name": "A"}), select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "place": "left"}), select("A Top"),
                   plugin(approval, "cabinet", {"name": "D", "place": "above"}),
                   MEASURE, change(A_width=800, A_height=900), MEASURE])
    before, after = boxes(results[5]), boxes(results[7])
    assert before["B Side R"][3] == -300 and after["B Side R"][3] == -400, (before["B Side R"], after["B Side R"])
    assert before["D Side L"][2] == 720 and after["D Side L"][2] == 900, (before["D Side L"], after["D Side L"])


def test_a_cabinet_on_a_return_follows_the_corner_cabinet(approval):
    # A, then a corner cabinet right of it, then U against the end of its return, turned a
    # quarter. A return 100 longer pushes U forward by all of it: the corner cabinet is held to
    # A by its corner at the wall, so its back stays on the wall and it grows to the front.
    # (It used to be held sideways only, and grew half to the back, into the wall.)
    # A wider A pushes the corner cabinet along, and U with it.
    results = cli([*run(approval, [("cabinet", {"name": "A"}), ("corner", {"name": "K", "place": "right"}),
                                   ("cabinet", {"name": "U", "place": "right"})]),
                   MEASURE, change(K_return=1000), MEASURE, change(A_width=800), MEASURE])
    at = lambda measure: boxes(measure)["U Top"][:2]
    start, longer, wider = at(results[5]), at(results[7]), at(results[9])
    assert [longer[0] - start[0], longer[1] - start[1]] == [0, -100], (start, longer)
    assert boxes(results[7])["K Back"][4] == boxes(results[5])["K Back"][4] == 280, boxes(results[7])["K Back"]
    assert [wider[0] - longer[0], wider[1] - longer[1]] == [100, 0], (longer, wider)
    # and U still stands against the end of the return, which is the corner cabinet's right side
    box = boxes(results[9])
    assert box["U Side L"][4] == box["K Side R"][1], (box["U Side L"], box["K Side R"])


def test_backs_and_walls_stay_flush(approval):
    # Held both ways. A run of two with a corner cabinet at its end and U on the return: a
    # deeper A, or a shallower B, leaves the backs on one line; a wider corner cabinet takes U
    # along sideways, its back still on the side wall; a deeper U stays on that wall too.
    results = cli([*run(approval, [("cabinet", {"name": "A"}), ("cabinet", {"name": "B", "place": "right"}),
                                   ("corner", {"name": "K", "place": "right"}), ("cabinet", {"name": "U", "place": "right"})]),
                   MEASURE, change(A_depth=700, B_depth=400), MEASURE, change(K_width=1100), MEASURE,
                   change(U_depth=450), MEASURE])
    start, deeper, wider, last = (boxes(results[i]) for i in (7, 9, 11, 13))
    back = lambda box, name: box[f"{name} Side L"][4]  # the far end of a side: the back wall
    assert [back(start, name) for name in "AB"] == [280, 280]
    # A stands on its own middle and is the first of the run: its back moves, and the run with it
    assert [back(deeper, name) for name in "AB"] == [350, 350], [back(deeper, name) for name in "AB"]
    assert deeper["K Back"][4] == 350 and deeper["B Side L"][1] == -50, (deeper["K Back"], deeper["B Side L"])
    # U's back is on the side wall, which is the corner cabinet's far end
    wall = lambda box: (box["K Back return"][3], box["U Side L"][3])
    assert wall(start)[0] == wall(start)[1] and wall(wider)[0] == wall(wider)[1] == wall(start)[0] + 200, (wall(start), wall(wider))
    assert wall(last) == wall(wider) and last["U Side L"][0] == wall(wider)[1] - 450, (wall(last), last["U Side L"])


def test_edit_and_joinery_keep_the_links(approval):
    # Edit builds a cabinet again, which takes its joints away: the one that holds it to its
    # neighbour, and the ones that hold others to it. Joinery moves a cabinet to drill it. After
    # either, in the middle of a run and at its start, the run still follows a change of width.
    alone = designed(approval, NONE, [plugin(approval, "cabinet", {"name": "B"}), select("B Top"), plugin(approval, "edit", {"joints": "dowels"}), MEASURE])
    holes = cylinders(alone[3], "B Top")  # as many as a cabinet drilled on its own gets
    for edited in "BA":
        results = designed(approval, NONE, [*run(approval), select(f"{edited} Top"), plugin(approval, "edit", {"shelves": 2}),
                       select("B Top"), plugin(approval, "edit", {"joints": "dowels"}),
                       select("A Top"), plugin(approval, "edit", {"joints": "dowels"}),
                       MEASURE, change(A_width=800, B_width=900), MEASURE,
                       {"command": "component.list", "params": {"compact": True}}])
        before, after = boxes(results[11]), boxes(results[13])
        left = lambda box: [box[f"{name} Side L"][0] for name in "ABC"]
        assert left(before) == [-300, 300, 900], (edited, left(before))
        assert left(after) == [-400, 400, 1300], (edited, left(after))
        assert f"{edited} Shelf 2" in after and cylinders(results[13], "B Top") == cylinders(results[13], "A Top") == holes > 0, (edited, holes)
        # only the first of the run is grounded; the others are held to it
        grounded = {c["name"]: c["occurrences"][0]["grounded"] for c in results[14]["components"] if c["name"] in "ABC"}
        assert grounded == {"A": True, "B": False, "C": False}, (edited, grounded)


def test_only_the_picked_cabinet_is_measured(approval):
    # Placing, a panel, Material and Edit measure only the picked cabinet's boards (a kitchen's
    # whole measure takes over a second): with another cabinet in the design, everything lands
    # where it did when every board was measured.
    away = [{"command": "occurrence.ground", "params": {"occurrence": "Z:1", "grounded": False}},
            {"command": "occurrence.move", "params": {"occurrence": "Z:1", "translate": [3000, 0, 0]}}]
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "Z"}), *away, plugin(approval, "cabinet", {"name": "A"}),
        select("A Side R"), plugin(approval, "panel", {"kind": "end"}),
        select("A Top"), plugin(approval, "material", {"material": "Oak", "scope": "cabinet"}),
        select("A Top"), plugin(approval, "cabinet", {"name": "B", "place": "right"}),
        select("B Top"), plugin(approval, "edit", {"shelves": 2}), MEASURE])
    box, measure = boxes(results[-1]), results[-1]
    # B stands beyond A's end panel, not linked to A (the link holds the two sides' seats)
    placed = results[9]["result"]
    assert placed["placed"] == "right of A" and placed["linked"] is False, placed
    assert box["A End panel R"][0] == 300 and box["B Side L"][0] == box["A End panel R"][3], (box["A End panel R"], box["B Side L"])
    assert box["Z Side L"][0] == 2700 and "B Shelf 2" in box, box["Z Side L"]
    material = {b["name"]: b.get("material") for b in measure["bodies"]}
    assert material["A Top"] == material["A End panel R"] == "Oak" and material["Z Top"] != "Oak", material
