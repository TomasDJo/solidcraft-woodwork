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
    # quarter. A return 100 longer pushes U forward by 50: the corner cabinet stands on its own
    # middle and is held to A sideways only, so it grows 50 to the front and 50 to the back.
    # A wider A pushes the corner cabinet along, and U with it.
    results = cli([*run(approval, [("cabinet", {"name": "A"}), ("corner", {"name": "K", "place": "right"}),
                                   ("cabinet", {"name": "U", "place": "right"})]),
                   MEASURE, change(K_return=1000), MEASURE, change(A_width=800), MEASURE])
    at = lambda measure: boxes(measure)["U Top"][:2]
    start, longer, wider = at(results[5]), at(results[7]), at(results[9])
    assert [longer[0] - start[0], longer[1] - start[1]] == [0, -50], (start, longer)
    assert [wider[0] - longer[0], wider[1] - longer[1]] == [100, 0], (longer, wider)
    # and U still stands against the end of the return, which is the corner cabinet's right side
    box = boxes(results[9])
    assert box["U Side L"][4] == box["K Side R"][1], (box["U Side L"], box["K Side R"])


def test_edit_and_joinery_keep_the_links(approval):
    # Edit builds a cabinet again, which takes its joints away: the one that holds it to its
    # neighbour, and the ones that hold others to it. Joinery moves a cabinet to drill it. After
    # either, in the middle of a run and at its start, the run still follows a change of width.
    alone = cli([plugin(approval, "cabinet", {"name": "B"}), select("B Top"), plugin(approval, "joinery", {"type": "dowel"}), MEASURE])
    holes = cylinders(alone[3], "B Top")  # as many as a cabinet drilled on its own gets
    for edited in "BA":
        results = cli([*run(approval), select(f"{edited} Top"), plugin(approval, "edit", {"shelves": 2}),
                       select("B Top"), plugin(approval, "joinery", {"type": "dowel"}),
                       select("A Top"), plugin(approval, "joinery", {"type": "dowel"}),
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
