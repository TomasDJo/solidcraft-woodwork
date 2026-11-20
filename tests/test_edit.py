"""Edit: the selected cabinet built again where it stands, with some of its values changed."""
import subprocess

from helpers import *


def move(occurrence, by):
    ground = lambda grounded: {"command": "occurrence.ground", "params": {"occurrence": occurrence, "grounded": grounded}}
    return [ground(False), {"command": "occurrence.move", "params": {"occurrence": occurrence, "translate": by}}, ground(True)]


def change(name, expression):
    return {"command": "parameters.change", "params": {"name": name, "expression": expression}}


def test_shelves_and_handles_on_a_cabinet_that_was_moved_and_resized(approval):
    results = cli([plugin(approval, "cabinet", {"name": "A", "doors": 2, "plinth": 100}),
                   pick("A Door L", "A Door R"),
                   plugin(approval, "material", {"material": "Oak", "scope": "parts"}),
                   select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "place": "right", "match": "selected"}),
                   select("A Top"),
                   plugin(approval, "joinery", {"type": "dowel"}),
                   MEASURE,
                   *move("A:1", [0, 500, 300]),
                   change("A_width", "800 mm"),
                   select("A Top"),
                   plugin(approval, "edit", {"shelves": 3, "handles": "pull", "shelf_pins": "yes"}),
                   MEASURE,
                   change("A_height", "900 mm"),
                   MEASURE,
                   {"command": "component.list", "params": {"compact": True}}])
    assert results[13]["result"]["edited"] and results[13]["result"]["joinery"] == "dowel", results[13]
    box = boxes(results[14])
    # three shelves now, and nothing left of the cabinet as it was
    assert sorted(n for n in box if n.startswith("A Shelf")) == ["A Shelf 1", "A Shelf 2", "A Shelf 3"], sorted(box)
    assert len([n for n in box if n.startswith("A ")]) == 11, sorted(box)
    # still 800 wide, where it was moved to: 500 back and 300 up
    assert box["A Side L"][:3] == [-400, 220, 300] and box["A Side R"][3] == 400, (box["A Side L"], box["A Side R"])
    # 2 hinge cups and now 2 screw holes for a pull in each door, which is still oak
    assert cylinders(results[14], "A Door L") == cylinders(results[14], "A Door R") == 4
    material = {b["name"]: b["material"] for b in results[14]["bodies"]}
    # its dowel holes were drilled again: as many in the new top as there were in the old
    assert cylinders(results[14], "A Top") == cylinders(results[7], "A Top") > 0, cylinders(results[14], "A Top")
    assert material["A Door L"] == "Oak" and material["A Shelf 3"] == "Birch plywood", material
    # the cabinet matched to it still follows its height, and being placed against it, is still
    # held to it: it went along 500 back and 300 up, and stands against the side that is now at 400
    assert boxes(results[16])["B Top"] == [418, 220, 300 + 882, 982, 780, 300 + 900], boxes(results[16])["B Top"]
    # and it is one component as before, grounded
    tree = results[17]["components"]
    assert [c["occurrences"][0]["grounded"] for c in tree if c["name"] == "A"] == [True], tree


def test_a_corner_cabinet_and_what_cannot_be_edited(approval):
    results = cli([plugin(approval, "corner", {"name": "C"}),
                   select("C Top"),
                   plugin(approval, "edit", {"corner_doors": "hinged left", "shape": "diagonal"}),
                   MEASURE])
    assert "C Door" in boxes(results[3]) and "C Door return" not in boxes(results[3]), sorted(boxes(results[3]))
    # a field of the other kind of cabinet, and a body that is no cabinet, are refused
    for steps in ([plugin(approval, "corner", {"name": "C"}), select("C Top"), plugin(approval, "edit", {"dividers": 1})],
                  [{"command": "solid.box", "params": {"length": "10 mm", "width": "10 mm", "height": "10 mm", "body_name": "Loose"}},
                   select("Loose"), plugin(approval, "edit", {"shelves": 2})]):
        try:
            cli(steps)
        except subprocess.CalledProcessError:
            continue
        raise AssertionError("an edit that should be refused went through")


def test_a_corner_that_followed_its_place_stays_that_corner(approval):
    # Placed right of a cabinet a corner cabinet becomes a right corner by itself. Edit builds
    # it again without knowing where it was placed: it must still be a right corner, there.
    results = cli([plugin(approval, "cabinet", {"name": "A"}), select("A Top"),
                   plugin(approval, "corner", {"name": "K", "place": "right"}), MEASURE,
                   select("K Top"), plugin(approval, "edit", {"shelves": 2}), MEASURE])
    before, after = boxes(results[3]), boxes(results[6])
    # the return's end is its right side, forward at the far end of the run
    assert before["K Side R"] == [640, -620, 0, 1200, -602, 720], before["K Side R"]
    # every board but the shelves, which are spaced anew, is where it was
    fixed = lambda box: {name: b for name, b in box.items() if "Shelf" not in name}
    assert "K Shelf 2" in after and fixed(after) == fixed(before), (fixed(after), fixed(before))


def test_drawers_over_doors_and_the_drawer_height_through_edit(approval):
    # A cabinet with one door gets a drawer over it; the drawer front height is then a
    # parameter that Edit takes from the design, and Edit can set it.
    results = cli([plugin(approval, "cabinet", {"doors": 1, "plinth": 100}), select("Cabinet Top"),
                   plugin(approval, "edit", {"drawers": 1}), MEASURE,
                   change("Cabinet_drawer_height", "200 mm"), select("Cabinet Top"),
                   plugin(approval, "edit", {"shelves": 2}), MEASURE,
                   select("Cabinet Top"), plugin(approval, "edit", {"drawer_height": 120}), MEASURE,
                   drive("Cabinet Drawer 1 slide", 200), drive("Cabinet Door hinge", 90), MEASURE])
    front = lambda measure: boxes(measure)["Cabinet Drawer 1 Front"][2:6:3]
    door = lambda measure: boxes(measure)["Cabinet Door"][2:6:3]
    assert front(results[3]) == [568, 718] and door(results[3]) == [102, 566], (front(results[3]), door(results[3]))
    assert "Cabinet Rail" in boxes(results[3]), sorted(boxes(results[3]))
    # the 200 set in the design survives an edit of something else
    assert front(results[7]) == [518, 718] and "Cabinet Shelf 2" in boxes(results[7]), front(results[7])
    assert front(results[10]) == [598, 718] and door(results[10]) == [102, 596], (front(results[10]), door(results[10]))
    assert boxes(results[13])["Cabinet Drawer 1 Front"][1] == -300 - 200, boxes(results[13])["Cabinet Drawer 1 Front"]
    assert boxes(results[13])["Cabinet Door"][1] < -500, boxes(results[13])["Cabinet Door"]
