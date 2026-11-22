"""Drawers, handle holes, and the joints doors and drawers move on."""
import subprocess

from helpers import *


def test_drawers(approval):
    # Drawers: 3 in a 600 x 720 cabinet. The boxes clear the sides by 12.7 mm and follow a resize.
    results = cli([plugin(approval, "cabinet", {"drawers": 3}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "parameters.change", "params": {"name": "Cabinet_width", "expression": "800 mm"}},
                   {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "drawers.sc"))
    box = {b["name"]: [round(v, 4) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in boards(results[1])}
    assert len(box) == 5 + 3 * 6, sorted(box)
    assert box["Cabinet Drawer 1 Left"][0] == -300 + 18 + 12.7, box["Cabinet Drawer 1 Left"]
    assert box["Cabinet Drawer 1 Right"][3] == 300 - 18 - 12.7, box["Cabinet Drawer 1 Right"]
    front = box["Cabinet Drawer 2 Front"]  # (720 - 4 gaps of 2) / 3 high
    assert round(front[5] - front[2], 3) == round(712 / 3, 3), front
    assert box["Cabinet Drawer 1 Bottom"][4] <= 280 - 12 - 6, box["Cabinet Drawer 1 Bottom"]  # clear of the back
    # the lowest box clears the cabinet's bottom and the highest its top, by 10 mm
    assert box["Cabinet Drawer 1 Left"][2] == box["Cabinet Bottom"][5] + 10, box["Cabinet Drawer 1 Left"]
    assert box["Cabinet Drawer 3 Left"][5] == box["Cabinet Top"][2] - 10, box["Cabinet Drawer 3 Left"]
    box = {b["name"]: [round(v, 4) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[3]["bodies"]}
    assert box["Cabinet Drawer 1 Right"][3] == 400 - 18 - 12.7, box["Cabinet Drawer 1 Right"]


def test_drawers_with_doors_and_a_graduated_stack(approval):
    # A 150 mm drawer over two doors, in 870 on a 150 plinth: the doors from 152 to 716, the front
    # from 718 to 868, and a rail centred behind the reveal between them (717), where both lap it.
    # The shelf is below the rail. The drawer slides out and a door turns.
    made = plugin(approval, "cabinet", {"drawers": 1, "doors": 2, "height": 870, "plinth": 150,
                                        "shelf_pins": True, "handles": "knob"})
    results = cli([made, MEASURE, drive("Cabinet Drawer 1 slide", 300), drive("Cabinet Door L hinge", 90), MEASURE],
                  "--save", str(ROOT / "dist" / "drawer-over-doors.sc"))
    box = boxes(results[1])
    assert box["Cabinet Door L"][2:6:3] == [152, 716] and box["Cabinet Door R"][2:6:3] == [152, 716], box["Cabinet Door L"]
    assert box["Cabinet Drawer 1 Front"][2:6:3] == [718, 868], box["Cabinet Drawer 1 Front"]
    assert box["Cabinet Rail"] == [-282, -280, 708, 282, 262, 726], box["Cabinet Rail"]
    assert box["Cabinet Shelf 1"][5] < 708 and box["Cabinet Drawer 1 Bottom"][2] > 726, (box["Cabinet Shelf 1"], box["Cabinet Drawer 1 Bottom"])
    assert box["Cabinet Drawer 1 Left"][5] < box["Cabinet Top"][2], box["Cabinet Drawer 1 Left"]
    assert cylinders(results[1], "Cabinet Door L") == 3, cylinders(results[1], "Cabinet Door L")  # 2 cups, a knob
    moved = boxes(results[4])
    assert moved["Cabinet Drawer 1 Front"][1] == -300 - 300 and moved["Cabinet Rail"] == box["Cabinet Rail"], moved["Cabinet Drawer 1 Front"]
    assert moved["Cabinet Door L"] == [-298, -597, 152, -280, -300, 716], moved["Cabinet Door L"]
    # Doors over drawers: two 140 mm fronts from the plinth (152 to 292, 294 to 434), the rail
    # behind the reveal at 435, the door from 436 to 868 with the shelf over the rail.
    made = plugin(approval, "cabinet", {"drawers": 2, "doors": 1, "drawers_at": "bottom", "height": 870,
                                        "plinth": 150, "drawer_height": 140, "shelf_pins": True})
    results = cli([made, MEASURE, drive("Cabinet Drawer 2 slide", 300), MEASURE])
    box = boxes(results[1])
    assert [box[f"Cabinet Drawer {n} Front"][2:6:3] for n in (1, 2)] == [[152, 292], [294, 434]], box["Cabinet Drawer 2 Front"]
    assert box["Cabinet Door"][2:6:3] == [436, 868] and box["Cabinet Rail"][2:6:3] == [426, 444], (box["Cabinet Door"], box["Cabinet Rail"])
    assert box["Cabinet Shelf 1"][2] > 444 and box["Cabinet Drawer 2 Left"][5] < 426, (box["Cabinet Shelf 1"], box["Cabinet Drawer 2 Left"])
    assert boxes(results[3])["Cabinet Drawer 2 Front"][1] == -600, boxes(results[3])["Cabinet Drawer 2 Front"]
    # Inset: the door from 20 to 548 and the 150 front from 550 to 700 inside the opening, the
    # rail behind them, the drawer box clear of it.
    box = boxes(cli([plugin(approval, "cabinet", {"fronts": "inset", "drawers": 1, "doors": 1}), MEASURE])[1])
    front = box["Cabinet Drawer 1 Front"]
    assert box["Cabinet Door"][2:6:3] == [20, 548] and front[1] == -280 and front[2:6:3] == [550, 700], front
    assert box["Cabinet Rail"] == [-282, -262, 540, 282, 262, 558], box["Cabinet Rail"]
    assert box["Cabinet Drawer 1 Bottom"][2] > 558, box["Cabinet Drawer 1 Bottom"]
    # Graduated: the top front 140 mm, the two below share the rest of 720: 286 each; inset
    # the rest of the 684 opening: 268 each.
    for fronts, rest, top in (("overlay", 286, 718), ("inset", 268, 700)):
        box = boxes(cli([plugin(approval, "cabinet", {"drawers": 3, "drawer_height": 140, "fronts": fronts}), MEASURE])[1])
        heights = [round(box[f"Cabinet Drawer {n} Front"][5] - box[f"Cabinet Drawer {n} Front"][2], 3) for n in (1, 2, 3)]
        assert heights == [rest, rest, 140] and box["Cabinet Drawer 3 Front"][5] == top, (fronts, heights, box["Cabinet Drawer 3 Front"])
    assert box["Cabinet Drawer 3 Left"][5] - box["Cabinet Drawer 3 Left"][2] == 140 - 2 * 8, box["Cabinet Drawer 3 Left"]


def test_handles(approval):
    # Handles: a pull is two holes through each front, next to a door's two hinge cups.
    results = cli([plugin(approval, "cabinet", {"name": "H", "doors": 2, "plinth": 100, "handles": "pull"}),
                   plugin(approval, "cabinet", {"name": "K", "drawers": 2, "handles": "knob"}),
                   {"command": "inspect.measure", "params": {}}],
                  "--save", str(ROOT / "dist" / "handles.sc"))
    assert cylinders(results[2], "H Door L") == 4 and cylinders(results[2], "K Drawer 2 Front") == 1


def test_handle_position(approval):
    # The door stands from z = 2 to 718. A pull's two holes are 128 mm apart, its middle 164 mm
    # from the end; a knob is 100 mm from it.
    for at, pull in (("top", [490, 618]), ("middle", [296, 424]), ("bottom", [102, 230]), ("auto", [490, 618])):
        results = cli([plugin(approval, "cabinet", {"doors": 1, "handles": "pull", "handle_position": at}),
                       faces("Cabinet Door")])
        assert handle_heights(results[1]) == pull, (at, handle_heights(results[1]))
    # Both corner shapes, with a knob: on the L it is on the door that folds.
    for shape, door in (("L-shaped", "Cabinet Door return"), ("diagonal", "Cabinet Door")):
        for at, knob in (("top", [618]), ("middle", [360]), ("bottom", [102])):
            results = cli([plugin(approval, "corner", {"shape": shape, "doors": "hinged right", "handles": "knob",
                                                       "handle_position": at}),
                           faces(door)])
            assert handle_heights(results[1]) == knob, (shape, at, handle_heights(results[1]))


def test_joints(approval):
    # Joints: doors swing out on their hinges and a drawer slides out, in an inch design to
    # check the joint's point is not read in inches. The cabinet still moves as one piece.
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", {"name": "D", "doors": 2}),
                   plugin(approval, "cabinet", {"name": "E", "drawers": 2}),
                   drive("D Door L hinge", 90), drive("D Door R hinge", 200), drive("E Drawer 1 slide", 300),
                   {"command": "occurrence.ground", "params": {"occurrence": "D:1", "grounded": False}},
                   {"command": "occurrence.move", "params": {"occurrence": "D:1", "translate": [0, 0, 1000]}},
                   {"command": "inspect.measure", "params": {}},
                   plugin(approval, "cutlist")],
                  "--save", str(ROOT / "dist" / "joints.sc"))
    # the cut list measures the doors as boards, however far they stand open
    # (716 x 297 x 18 mm, listed in inches in this inch design)
    assert '2,28 3/16,11 11/16,23/32,"Birch plywood",yes,L2 W2,"D Door L, D Door R"' in results[9]["result"]["csv"], results[9]["result"]["csv"]
    box = {b["name"]: [round(v, 3) for v in b["bbox"]["min"] + b["bbox"]["max"]] for b in results[8]["bodies"]}
    # left door: turned 90 degrees about its front left edge (x -298, y -300), now along -Y and
    # still inside the cabinet's width, clear of a door on the next cabinet
    assert box["D Door L"] == [-298, -597, 1002, -280, -300, 1718], box["D Door L"]
    # right door: stopped by its limit at 110 degrees
    assert box["D Door R"][1] < -300 - 297 * 0.9 and box["D Door R"][3] > 298, box["D Door R"]
    assert box["E Drawer 1 Front"][1] == -300 - 300, box["E Drawer 1 Front"]
    assert box["E Drawer 2 Front"][1] == -300, box["E Drawer 2 Front"]


def test_the_first_door_hinges_on_the_side_that_is_asked(approval):
    # One door, 600 wide. Hinged left it swings out about x = -298, hinged right about x = 298,
    # with its handle hole at the other edge. Edit turns it round. Three doors alternate from
    # the first, and a pair opens from the middle whatever is asked.
    def door(params, name="Cabinet Door", joint="Cabinet Door hinge", then=()):
        made = [plugin(approval, "cabinet", {"handles": "knob", **params}), *then]
        hole = [f["centroid"][0] for f in cli([*made, faces(name)])[-1]["faces"]
                if abs((f.get("cylinder") or {}).get("radius", 0) - 2.5) < 0.01][0]
        box = boxes(cli([*made, drive(joint, 90), MEASURE])[-1])[name]
        return box[0], box[3], round(hole, 3)
    assert door({"doors": 1}) == (-298, -280, 248), door({"doors": 1})
    assert door({"doors": 1, "door_hinge": "right"}) == (280, 298, -248), door({"doors": 1, "door_hinge": "right"})
    turned = door({"doors": 1}, then=[select("Cabinet Top"), plugin(approval, "edit", {"door_hinge": "right"})])
    assert turned == (280, 298, -248), turned
    three = {"width": 900, "dividers": 2, "doors": 3, "door_hinge": "right"}
    opened = [door(three, f"Cabinet Door {n}", f"Cabinet Door {n} hinge") for n in (1, 2, 3)]
    # each opened door stands at its hinge edge: the right of the first, the left of the second
    assert [d[1] for d in opened[::2]] == [-150.667, 448] and opened[1][0] == -148.667, opened
    pair = {"doors": 2, "door_hinge": "right"}
    assert door(pair, "Cabinet Door L", "Cabinet Door L hinge")[0] == -298, "a pair opens from the middle"


def test_a_bifold_handle_is_at_the_free_end(approval):
    # The second door of a corner cabinet's pair hangs on the first at the fold. Its handle is
    # 50 mm from its other end, the free one, not beside the fold: in both corners, hinged at either end.
    for corner in ("left", "right"):
        for doors in ("hinged left", "hinged right"):
            params = {"corner": corner, "doors": doors, "handles": "knob"}
            box = boxes(cli([plugin(approval, "corner", params), MEASURE])[1])
            first, second = "Cabinet Door", "Cabinet Door return"
            for name, other, axis in ((first, second, 0), (second, first, 1)):
                holes = [f["centroid"] for f in cli([plugin(approval, "corner", params), faces(name)])[1]["faces"]
                         if abs((f.get("cylinder") or {}).get("radius", 0) - 2.5) < 0.01]
                if not holes:
                    continue  # the door that is hinged on the cabinet
                lo, hi = box[name][axis], box[name][axis + 3]
                middle = (box[other][axis] + box[other][axis + 3]) / 2
                free = lo if abs(lo - middle) > abs(hi - middle) else hi  # the end away from the other door
                assert round(abs(holes[0][axis] - free), 3) == 50, (params, name, holes[0], box[name])
                break
            else:
                raise AssertionError(("no handle", params))


def test_a_diagonal_door_is_made_closed_and_opens_outwards(approval):
    # The diagonal door stands at 45 degrees. Whichever end it is hinged at, in either corner,
    # it is made closed across the front, that is where its joint is at 0, and at 90 it has
    # swung out of the cabinet, away from its middle.
    for corner in ("left", "right"):
        closed = {}
        for doors in ("hinged left", "hinged right"):
            made = plugin(approval, "corner", {"shape": "diagonal", "corner": corner, "doors": doors})
            door = lambda *steps: boxes(cli([made, *steps, MEASURE])[-1])["Cabinet Door"]
            away = lambda box: ((box[0] + box[3]) ** 2 + (box[1] + box[4]) ** 2) ** 0.5 / 2
            closed[doors] = door()
            assert door(drive("Cabinet Door hinge", 0)) == closed[doors], (corner, doors, closed[doors])
            opened = door(drive("Cabinet Door hinge", 90))
            assert away(opened) > away(closed[doors]) + 200, (corner, doors, opened, closed[doors])
        assert closed["hinged left"] == closed["hinged right"], (corner, closed)


def test_a_drawer_handle_follows_its_position(approval):
    # Top, middle and bottom move a drawer front's handle holes too: 40 mm from the front's
    # edge, or its middle, which auto also is. Two drawers in 720: the lower front is 2 to 359.
    wanted = {"auto": 180.5, "middle": 180.5, "top": 359 - 40, "bottom": 2 + 40}
    for at, height in wanted.items():
        for handles, holes in (("knob", 1), ("pull", 2)):
            results = cli([plugin(approval, "cabinet", {"drawers": 2, "handles": handles, "handle_position": at}),
                           faces("Cabinet Drawer 1 Front")])
            assert handle_heights(results[1]) == [height] * holes, (at, handles, handle_heights(results[1]))


def test_hinges_follow_a_resize(approval):
    # A cabinet made small and then widened and deepened opens its doors exactly as one made
    # at that size does: the hinge is on the door's edge, not at a point from when it was made.
    change = lambda name, to: {"command": "parameters.change", "params": {"name": name, "expression": to}}
    cases = [("cabinet", {"doors": 2}, {"width": 600, "depth": 560}, {"width": 900, "depth": 400},
              {"Cabinet Door L hinge": 90, "Cabinet Door R hinge": 60}),
             ("corner", {"doors": "hinged right"}, {"width": 900, "return": 900}, {"width": 1100, "return": 1000},
              {"Cabinet Door hinge": 70, "Cabinet Door return fold": 80}),
             ("corner", {"doors": "hinged right", "corner": "right"}, {"width": 900, "return": 900}, {"width": 1100, "return": 1000},
              {"Cabinet Door return hinge": 70, "Cabinet Door fold": 80}),
             ("corner", {"shape": "diagonal", "doors": "hinged left"}, {"width": 900, "depth": 560}, {"width": 900, "depth": 400},
              {"Cabinet Door hinge": 60})]
    for entry, params, small, large, joints in cases:
        drives = [drive(joint, value) for joint, value in joints.items()]
        resized = cli([plugin(approval, entry, {**params, **small}),
                       *[change(f"Cabinet_{key}", f"{value} mm") for key, value in large.items()], *drives, MEASURE])[-1]
        made = cli([plugin(approval, entry, {**params, **large}), *drives, MEASURE])[-1]
        doors = lambda measure: {name: box for name, box in boxes(measure).items() if " Door" in name}
        assert doors(resized) == doors(made) and doors(made), (entry, params, doors(resized), doors(made))


def test_labels(approval):
    # Each flat board gets its name on it, as a sketch that survives a resize; the drawer
    # boxes' thin boards and a cabinet made without the field get none.
    inspect = {"command": "document.inspect", "params": {"bodies": False, "timeline": False}}
    change = {"command": "parameters.change", "params": {"changes": {"Cabinet_width": "900 mm", "Cabinet_depth": "400 mm"}}}
    for entry, params, wanted in (("cabinet", {"doors": 2, "plinth": 100}, 9), ("corner", {"doors": "hinged right"}, 9),
                                  ("corner", {"shape": "diagonal", "doors": "hinged left", "plinth": 100}, 9)):
        results = cli([plugin(approval, entry, {**params, "labels": True}), MEASURE, change, inspect, plugin(approval, "cutlist")])
        sketches = {s["name"]: s["status"] for s in results[3]["sketches"]}
        bodies = [b["name"] for b in boards(results[1])]
        assert sorted(sketches) == sorted(f"{body} label" for body in bodies), (entry, sorted(sketches), bodies)
        assert set(sketches.values()) == {"solved"} and len(sketches) == wanted, (entry, sketches)
        assert results[4]["result"]["boards"] == len(bodies)  # the cut list counts boards, not labels
    results = cli([plugin(approval, "cabinet", {"doors": 2}), inspect])
    assert results[1]["sketches"] == [], results[1]["sketches"]


def overlap(a, b):
    """Whether two boxes share any volume."""
    return all(a[i] < b[i + 3] - 1e-6 and b[i] < a[i + 3] - 1e-6 for i in range(3))


def test_inset_doors_sit_in_the_opening_and_open_clear_of_the_sides(approval):
    # Two inset doors with a 3 mm reveal in 600 x 720: the opening is 564 x 684 between the
    # panels, so each door is (564 - 3 x 3) / 2 = 277.5 wide and 684 - 6 = 678 high, its front
    # face on the carcass front (y = -280). Open at 90 degrees, each stands in front of the
    # carcass, clear of its side. The cut list measures the doors at that size.
    made = plugin(approval, "cabinet", {"fronts": "inset", "reveal": 3, "doors": 2, "handles": "knob"})
    results = cli([made, MEASURE, drive("Cabinet Door L hinge", 90), drive("Cabinet Door R hinge", 90), MEASURE,
                   plugin(approval, "cutlist")], "--save", str(ROOT / "dist" / "inset-doors.sc"))
    box = boxes(results[1])
    assert box["Cabinet Door L"] == [-279, -280, 21, -1.5, -262, 699], box["Cabinet Door L"]
    assert box["Cabinet Door R"] == [1.5, -280, 21, 279, -262, 699], box["Cabinet Door R"]
    assert box["Cabinet Shelf 1"][1] == -262 and box["Cabinet Side L"][1] == -280, box["Cabinet Shelf 1"]
    assert cylinders(results[1], "Cabinet Door L") == 3, cylinders(results[1], "Cabinet Door L")  # 2 cups, a knob
    opened = boxes(results[4])
    assert opened["Cabinet Door L"] == [-279, -557.5, 21, -261, -280, 699], opened["Cabinet Door L"]
    assert opened["Cabinet Door R"] == [261, -557.5, 21, 279, -280, 699], opened["Cabinet Door R"]
    for door, side in (("Cabinet Door L", "Cabinet Side L"), ("Cabinet Door R", "Cabinet Side R")):
        assert not overlap(opened[door], opened[side]), (opened[door], opened[side])
    assert '2,678,277.5,18,"Birch plywood",yes,L2 W2,"Cabinet Door L, Cabinet Door R"' in results[5]["result"]["csv"], results[5]["result"]["csv"]


def test_inset_drawers_and_doors_per_compartment(approval):
    # Two drawers in 870 on a 150 plinth: the opening is 684 high between the bottom (top face
    # at 168) and the top panel, so each front is (684 - 3 x 2) / 2 = 339, from 170 to 509 and
    # from 511 to 850. The box sits behind the front and slides out with it.
    made = plugin(approval, "cabinet", {"fronts": "inset", "drawers": 2, "height": 870, "plinth": 150,
                                        "handles": "knob"})
    results = cli([made, MEASURE, drive("Cabinet Drawer 1 slide", 300), MEASURE],
                  "--save", str(ROOT / "dist" / "inset-drawer.sc"))
    box = boxes(results[1])
    assert box["Cabinet Drawer 1 Front"] == [-280, -280, 170, 280, -262, 509], box["Cabinet Drawer 1 Front"]
    assert box["Cabinet Drawer 2 Front"][2:6:3] == [511, 850], box["Cabinet Drawer 2 Front"]
    assert box["Cabinet Drawer 1 Left"][1] == -262 and box["Cabinet Drawer 1 Left"][2] == 178, box["Cabinet Drawer 1 Left"]
    assert boxes(results[3])["Cabinet Drawer 1 Front"][1] == -580, boxes(results[3])["Cabinet Drawer 1 Front"]
    # Three compartments in 900: each door fills its own, 276 - 4 wide, the dividers behind them.
    box = boxes(cli([plugin(approval, "cabinet", {"fronts": "inset", "width": 900, "dividers": 2, "doors": 3}), MEASURE])[1])
    assert [box[f"Cabinet Door {n}"][0:4:3] for n in (1, 2, 3)] == [[-430, -158], [-136, 136], [158, 430]], box
    assert box["Cabinet Divider 1"][0:2] == [-156, -262], box["Cabinet Divider 1"]


def test_overlay_fronts_with_a_reveal(approval):
    # Overlay doors with a 3 mm reveal: 3 mm in from the outside and between them, 3 mm proud.
    box = boxes(cli([plugin(approval, "cabinet", {"reveal": 3, "doors": 2}), MEASURE])[1])
    assert box["Cabinet Door L"] == [-297, -301, 3, -1.5, -283, 717], box["Cabinet Door L"]


def test_the_lowest_drawer_front_height(approval):
    # 0 is Auto, else 100 mm or more: 102 mm with 18 mm boards, the room for a box. The lowest
    # builds, one below is refused (the message, naming the minimum, is a unit test's).
    results = cli([plugin(approval, "cabinet", {"drawers": 3, "drawer_height": 102}), MEASURE])
    heights = [round(b[5] - b[2], 3) for name, b in boxes(results[1]).items() if name.endswith("Front")]
    assert 102 in heights, heights
    try:
        cli([plugin(approval, "cabinet", {"drawers": 3, "drawer_height": 101})])
    except subprocess.CalledProcessError:
        return
    raise AssertionError("a drawer front under the minimum went through")


def test_drawer_box_and_bottom_thickness(approval):
    # Woodwork settings' drawer box boards and bottoms: 15 and 9 mm instead of 12 and 6. The
    # box keeps its outside; its front, back and bottom fit between the thicker sides.
    results = cli([plugin(approval, "cabinet", {"drawers": 1, "drawer_thickness": 15, "drawer_bottom": 9}), MEASURE])
    box = boxes(results[1])
    size = lambda name, axis: round(box[f"Cabinet Drawer 1 {name}"][axis + 3] - box[f"Cabinet Drawer 1 {name}"][axis], 3)
    assert (size("Left", 0), size("Right", 0), size("Box back", 1), size("Bottom", 2)) == (15, 15, 15, 9), box
    assert box["Cabinet Drawer 1 Left"][0] == -300 + 18 + 12.7, box["Cabinet Drawer 1 Left"]
    assert size("Bottom", 0) == 600 - 2 * 18 - 2 * 12.7 - 2 * 15, box["Cabinet Drawer 1 Bottom"]
