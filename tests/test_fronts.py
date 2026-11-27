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
    # with its handle hole at the other edge. Edit turns it round. Of three doors the end ones
    # hinge on the sides whatever is asked (#157), the middle one on the side asked,
    # and a pair opens from the middle whatever is asked.
    def door(params, name="Cabinet Door", joint="Cabinet Door hinge", then=()):
        made = [plugin(approval, "cabinet", {"handles": "knob", **params}), *then]
        hole = [f["centroid"][0] for f in cli([*made, faces(name)])[-1]["faces"]
                if abs((f.get("cylinder") or {}).get("radius", 0) - 2.5) < 0.01][0]
        box = boxes(cli([*made, drive(joint, 90), MEASURE])[-1])[name]
        return box[0], box[3], round(hole, 3)
    assert door({"doors": 1}) == (-298, -280, 248), door({"doors": 1})
    assert door({"doors": 1, "door_hinge": "right"}) == (280, 298, -248), door({"doors": 1, "door_hinge": "right"})
    turned = door({"doors": 1}, then=[select("Cabinet Top"), plugin(approval, "edit", {"doors": 1, "door_hinge": "right"})])
    assert turned == (280, 298, -248), turned
    three = {"width": 900, "dividers": 2, "doors": 3, "door_hinge": "right"}
    opened = [door(three, f"Cabinet Door {n}", f"Cabinet Door {n} hinge") for n in (1, 2, 3)]
    # each opened door stands at its hinge edge: Side L, the second's right (asked), Side R
    assert (opened[0][0], opened[1][1], opened[2][1]) == (-448, 148.667, 448), opened
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


def test_wooden_runners(approval):
    # Slide type Wood: the bottom, 6 mm, is wider than the box and runs in a 6 mm deep groove in
    # each side with 1 mm of play; the drawer comes out two thirds of its box (532 mm deep).
    # No slide holes, models or hardware; the cut list notes the grooves. Beside a door with
    # inset fronts too.
    results = designed(approval, {"slide_type": "wood"}, [
        plugin(approval, "cabinet", {"drawers": 2}), {"command": "joint.list", "params": {}}, MEASURE,
        drive("Cabinet Drawer 1 slide", 600), MEASURE, plugin(approval, "cutlist"),
        plugin(approval, "cabinet", {"name": "B", "drawers": 1, "doors": 1, "fronts": "inset", "place": "origin"}),
        MEASURE])
    limits = {j["name"]: j["values"][0]["limits"] for j in results[1]["joints"]}
    assert limits["Cabinet Drawer 1 slide"] == [0, 355], limits
    shut, out = boxes(results[2]), boxes(results[4])
    bottom = shut["Cabinet Drawer 1 Bottom"]
    assert (round(bottom[0], 3), round(bottom[3], 3)) == (-300 + 13, 300 - 13), bottom
    assert shut["Cabinet Drawer 1 Left"][0] == -300 + 19 and shut["Cabinet Drawer 1 Left"][2] == bottom[5]
    assert abs(out["Cabinet Drawer 1 Front"][1] - (shut["Cabinet Drawer 1 Front"][1] - 355)) < 0.01
    # each side loses two grooves of 6 x 8 mm, front to back
    side = volume(results[2], "Cabinet Side L")
    assert side < 720 * 560 * 18 - 2 * 6 * 8 * 500, side
    assert not [n for n in boxes(results[2], models=True) if n.startswith("HW slide")], "slide models"
    cutlist = results[5]["result"]
    assert not [h for h in cutlist["hardware"] if "slide" in h["item"].lower()], cutlist["hardware"]
    notes = [p.get("note", "") for p in cutlist["parts"] if "Cabinet Side L" in p["parts"]]
    assert notes and "grooved for the drawer runners" in notes[0], cutlist["parts"]
    b = boxes(results[7])
    assert b["B Drawer 1 Bottom"][3] - b["B Drawer 1 Bottom"][0] > b["B Drawer 1 Right"][3] - b["B Drawer 1 Left"][0], b


def test_a_cabinet_sized_by_its_drawers_inside(approval):
    # Size by Drawer inside, 672 x 672 mm (16 x 16 Gridfinity bins): the cabinet comes out
    # 757.4 x 724 mm and keeps that as its ordinary size, which an edit keeps.
    results = cli([plugin(approval, "cabinet", {"drawers": 2, "sizing": "drawer", "drawer_width": 672, "drawer_depth": 672}),
                   MEASURE, select("Cabinet Top"), plugin(approval, "edit", {"drawers": 3}), MEASURE])
    size = results[0]["result"]["size"]
    assert (size["width"], size["depth"]) == (757.4, 724), size
    for measure in (results[1], results[4]):
        box = boxes(measure)
        inside = box["Cabinet Drawer 1 Bottom"]
        assert (round(inside[3] - inside[0], 3), round(inside[4] - inside[1], 3)) == (672, 672), inside
    # Apply settings to an integrated front (#163): a board shallower outside, the same inside
    results = cli([plugin(approval, "cabinet", {"drawers": 2, "sizing": "drawer", "drawer_width": 672, "drawer_depth": 672}),
                   plugin(approval, "settings", {"drawer_front": "integrated"}),
                   plugin(approval, "rebuild", {"with": "settings"}), MEASURE,
                   plugin(approval, "settings", {"drawer_front": "separate"})])
    box = boxes(results[3])
    inside, side = box["Cabinet Drawer 1 Bottom"], box["Cabinet Side L"]
    assert round(inside[4] - inside[1], 3) == 672, inside
    assert round(side[4] - side[1], 3) == 712, side


def test_a_cabinet_sized_by_its_drawers_inside_keeps_it(approval):
    # Sized by its drawers' inside, then side-mount slides changed to undermount in Woodwork
    # settings and Rebuild cabinets with them: the boxes keep their inside width, and the
    # cabinet's outside is worked out again around them (undermount needs 42 mm, not 2 x 24.7).
    inside = lambda measure: round(boxes(measure)["Cabinet Drawer 1 Right"][0] - boxes(measure)["Cabinet Drawer 1 Left"][3], 3)
    results = cli([plugin(approval, "cabinet", {"drawers": 2, "sizing": "drawer", "drawer_width": 500, "drawer_depth": 400}),
                   MEASURE, plugin(approval, "settings", {"slide_type": "under_full"}),
                   plugin(approval, "rebuild", {"with": "settings"}), MEASURE,
                   select("Cabinet Top"), plugin(approval, "edit", {}),
                   select("Cabinet Top"), plugin(approval, "edit", {"drawers": 2, "sizing": "drawer", "drawer_width": 550}), MEASURE,
                   plugin(approval, "settings", {"slide_type": "full"})])
    before, after = results[0]["result"]["size"]["width"], boxes(results[4])["Cabinet Side R"][3] - boxes(results[4])["Cabinet Side L"][0]
    assert inside(results[1]) == inside(results[4]) == 500, (inside(results[1]), inside(results[4]))
    assert (before, round(after, 3)) == (500 + 2 * (12 + 12.7 + 18), 500 + 42 + 36), (before, after)
    # Edit shows it by its inside: left as shown nothing changes; a new inside width is built
    assert results[6]["result"]["edited"] is False, results[6]
    assert inside(results[9]) == 550, inside(results[9])


def test_drawer_box_finger_joints(approval):
    # Drawer box joints Finger (#131): the box's front and back as wide as the box, the four
    # corners interlocking (fingers 12 mm wide on 12 mm boards), the cut list noting them.
    results = designed(approval, {"drawer_joints": "finger"},
                       [plugin(approval, "cabinet", {"drawers": 1}), MEASURE, plugin(approval, "cutlist")])
    box = boxes(results[1])
    left, front = box["Cabinet Drawer 1 Left"], box["Cabinet Drawer 1 Box front"]
    assert round(front[0], 3) == round(left[0], 3), (front, left)
    full = (front[3] - front[0]) * (front[4] - front[1]) * (front[5] - front[2])
    assert volume(results[1], "Cabinet Drawer 1 Box front") < full - 1000, full
    notes = [p.get("note", "") for p in results[2]["result"]["parts"]]
    assert any("finger jointed" in n for n in notes), results[2]["result"]["parts"]


def test_grip_edges(approval):
    # Handle-less fronts: a grip edge takes a 45-degree bevel, 12 mm along both faces of an 18 mm
    # door, from the back of its top edge (a floor unit's, auto) or bottom edge (a wall unit's);
    # a finger groove a 20 mm half round from the front face, 25 mm under a drawer front's top.
    # Each runs the front's whole width (596), and no handle holes are drilled.
    results = cli([plugin(approval, "cabinet", {"name": "G", "doors": 1, "handles": "grip"}),
                   plugin(approval, "cabinet", {"name": "N", "doors": 1}),
                   plugin(approval, "cabinet", {"name": "W", "doors": 1, "handles": "grip", "depth": 320}),
                   plugin(approval, "cabinet", {"name": "R", "drawers": 2, "handles": "groove"}),
                   plugin(approval, "cabinet", {"name": "M", "drawers": 2}),
                   plugin(approval, "corner", {"name": "C", "doors": "hinged right", "handles": "grip"}),
                   plugin(approval, "corner", {"name": "D", "doors": "hinged right"}),
                   MEASURE, faces("G Door"), faces("W Door"), faces("R Drawer 2 Front"), faces("C Door return"),
                   {"command": "parameters.change", "params": {"name": "G_width", "expression": "800 mm"}},
                   {"command": "parameters.change", "params": {"name": "N_width", "expression": "800 mm"}},
                   MEASURE],
                  "--save", str(ROOT / "dist" / "grip-edges.sc"))
    m = results[7]
    bevel, groove = 12 * 12 / 2 * 596, 3.14159265 * 10 ** 2 / 2 * 596
    assert abs(volume(m, "N Door") - volume(m, "G Door") - bevel) < 0.002 * bevel, (volume(m, "N Door"), volume(m, "G Door"))
    assert abs(volume(m, "N Door") - volume(m, "W Door") - bevel) < 0.002 * bevel, volume(m, "W Door")
    for n in (1, 2):
        cut = volume(m, f"M Drawer {n} Front") - volume(m, f"R Drawer {n} Front")
        assert abs(cut - groove) < 0.01 * groove, (n, cut)
    assert boxes(m)["G Door"] == boxes(m)["N Door"] and cylinders(m, "G Door") == cylinders(m, "N Door")
    assert cylinders(m, "R Drawer 1 Front") == 1 and cylinders(m, "M Drawer 1 Front") == 0

    def slanted(faces):
        """The bevel's face: its middle, x y z, and its normal."""
        face = next(f for f in faces["faces"] if abs(abs((f["plane_normal"] or [0, 0, 0])[2]) - 0.7071) < 0.001)
        return [round(v, 3) + 0 for v in face["centroid"]], [round(v, 4) + 0 for v in face["plane_normal"]]

    # The door's back face is at y = -282, its top edge at 718: the bevel's middle 6 mm in from both.
    assert slanted(results[8]) == ([0, -288, 712], [0, 0.7071, 0.7071]), slanted(results[8])
    # The wall unit's door is from 2 up and its back face at y = -162: the bottom edge.
    assert slanted(results[9]) == ([0, -168, 8], [0, 0.7071, -0.7071]), slanted(results[9])
    # The groove's round face, 25 mm under the top drawer front's top (718), on its front face.
    top = boxes(m)["R Drawer 2 Front"]
    round_face = next(f for f in results[10]["faces"] if abs((f.get("cylinder") or {}).get("radius", 0) - 10) < 0.01)
    assert abs(round_face["cylinder"]["origin"][2] - (top[5] - 25)) < 0.01 and abs(round_face["cylinder"]["origin"][1] - top[1]) < 0.01, round_face
    # The L corner's folding door, on the return: its bevel faces the corner, at its top.
    assert volume(m, "D Door return") - volume(m, "C Door return") > 0.99 * 72 * (boxes(m)["C Door return"][4] - boxes(m)["C Door return"][1])
    # The bevel follows a resize: the door 796 wide.
    m = results[14]
    assert abs(volume(m, "N Door") - volume(m, "G Door") - 72 * 796) < 0.002 * 72 * 796, volume(m, "G Door")
    centre, normal = slanted(results[11])
    assert normal[1] == 0 and normal[2] == 0.7071 and centre[2] == boxes(m)["C Door return"][5] - 6, (centre, normal)


def test_integrated_drawer_fronts(approval):
    # Drawer front Integrated (#163): no front of its own, the box's front board where the
    # separate front was (drilled for the knob), one board fewer a drawer in the cut list.
    steps = [plugin(approval, "cabinet", {"drawers": 2, "handles": "knob"}), MEASURE, plugin(approval, "cutlist")]
    separate = designed(approval, {"drawer_front": "separate"}, steps)
    results = designed(approval, {"drawer_front": "integrated"}, steps)
    box = boxes(results[1])
    assert "Cabinet Drawer 1 Front" not in box, sorted(box)
    for n in (1, 2):
        own, front = box[f"Cabinet Drawer {n} Box front"], boxes(separate[1])[f"Cabinet Drawer {n} Front"]
        assert [round(v, 3) for v in own] == [round(v, 3) for v in front], (own, front)
    # the bottom starts at the front's back face (y1 of the front, y0 of the bottom)
    assert box["Cabinet Drawer 1 Bottom"][1] == box["Cabinet Drawer 1 Box front"][4], box
    holes = cylinders(results[1], "Cabinet Drawer 1 Box front")
    assert holes == cylinders(separate[1], "Cabinet Drawer 1 Front") > 0, holes
    count = lambda r: sum(p["qty"] for p in r[2]["result"]["parts"])
    assert count(results) == count(separate) - 2, (count(results), count(separate))


def test_grooved_fronts(approval):
    # Vertical square grooves, 50 mm apart about and 3 deep, 6 wide, in a 596 x 716 door: 12
    # strips, so 11 grooves up its whole height. Round horizontal ones in two 357 mm drawer
    # fronts: 7 strips, 6 half rounds of 3 mm radius across each 596 mm front. Then the door
    # 200 mm wider: 16 strips. Each against the same cabinet without grooves.
    results = cli([plugin(approval, "cabinet", {"name": "V", "doors": 1, "front_style": "vertical"}),
                   plugin(approval, "cabinet", {"name": "N", "doors": 1}),
                   plugin(approval, "cabinet", {"name": "H", "drawers": 2, "front_style": "horizontal",
                                                "groove_shape": "round"}),
                   plugin(approval, "cabinet", {"name": "M", "drawers": 2}),
                   plugin(approval, "corner", {"name": "C", "doors": "hinged left", "front_style": "vertical"}),
                   plugin(approval, "corner", {"name": "D", "doors": "hinged left"}),
                   MEASURE, faces("V Door"),
                   {"command": "parameters.change", "params": {"name": "V_width", "expression": "800 mm"}},
                   {"command": "parameters.change", "params": {"name": "N_width", "expression": "800 mm"}},
                   MEASURE],
                  "--save", str(ROOT / "dist" / "grooved-fronts.sc"))
    m = results[6]
    square, round_ = 6 * 3 * 716, 3.14159265 * 3 ** 2 / 2 * 596
    cut = volume(m, "N Door") - volume(m, "V Door")
    assert abs(cut - 11 * square) < 0.002 * 11 * square, cut
    for n in (1, 2):
        cut = volume(m, f"M Drawer {n} Front") - volume(m, f"H Drawer {n} Front")
        assert abs(cut - 6 * round_) < 0.01 * 6 * round_, (n, cut)
    assert boxes(m)["V Door"] == boxes(m)["N Door"]
    # The grooves' bottoms are 3 mm behind the door's front face (y = -300), evenly across it.
    bottoms = sorted(round(f["centroid"][0], 3) for f in results[7]["faces"]
                     if f["plane_normal"] == [0.0, -1.0, 0.0] and abs(f["centroid"][1] + 297) < 0.01)
    assert len(bottoms) == 11 and all(abs(b - (-298 + 596 / 12 * (k + 1))) < 0.01 for k, b in enumerate(bottoms)), bottoms
    # Both of the L corner's doors are grooved: whole 716 mm grooves, against the flat ones.
    for door in ("Door", "Door return"):
        grooves = (volume(m, f"D {door}") - volume(m, f"C {door}")) / square
        assert grooves >= 1 and abs(grooves - round(grooves)) < 0.002, (door, grooves)
    cut = volume(results[10], "N Door") - volume(results[10], "V Door")
    assert abs(cut - 15 * square) < 0.002 * 15 * square, cut


def test_bottom_tab(approval):
    # Handle holes Bottom tab (#181): under each drawer front's lower edge a tab of the bottom's
    # stock (12 mm here) sits in a notch, flush with that edge, 120 mm wide and 20 mm out past
    # the face, rounded at the front; the front is scooped above it. Separate and integrated.
    steps = [plugin(approval, "cabinet", {"drawers": 2, "handles": "tab"}), MEASURE]
    for front in ("separate", "integrated"):
        results = designed(approval, {"drawer_front": front, "drawer_bottom": 12}, steps)
        box = boxes(results[1])
        name = "Cabinet Drawer 1 " + ("Front" if front == "separate" else "Box front")
        tab, face = box["Cabinet Drawer 1 Tab"], box[name]
        assert round(face[1] - tab[1], 3) == 20 and tab[4] == face[4], (tab, face)
        assert tab[2] == face[2] and round(tab[5] - tab[2], 3) == 12, (tab, face)
        assert round(tab[3] - tab[0], 3) == 120, tab
        assert cylinders(results[1], name) > 0, name
        assert cylinders(results[1], "Cabinet Drawer 1 Tab") >= 2, front
    # a narrow inset cabinet with integrated, finger-jointed fronts: the box sides run through
    # the front from 8 mm above its lower edge, so the 12 mm tab must stay between them
    narrow = [plugin(approval, "cabinet", {"width": 162, "drawers": 1, "handles": "tab"}), MEASURE]
    results = designed(approval, {"fronts": "inset", "drawer_front": "integrated", "drawer_joints": "finger",
                                  "drawer_bottom": 12}, narrow)
    box = boxes(results[1])
    tab, left, right = (box[f"Cabinet Drawer 1 {p}"] for p in ("Tab", "Left", "Right"))
    assert left[3] <= tab[0] + 0.001 and tab[3] <= right[0] + 0.001, (tab, left, right)
