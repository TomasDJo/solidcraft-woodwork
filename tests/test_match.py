"""Match selected: a new cabinet takes and follows the selected one's sizes."""
from helpers import *


def test_match_selected(approval):
    # Match selected: B takes A's height, depth, thickness and plinth, and the run's shared
    # parameters (`Run_A_height`, ...) size all three afterwards.
    # The corner cabinet too. In an inch design, with B's own sizes given in inches.
    a = {"name": "A", "height": 900, "depth": 400, "thickness": 19, "plinth": 100}
    results = cli([{"command": "document.units", "params": {"units": "in"}},
                   plugin(approval, "cabinet", a),
                   select("A Top"),
                   plugin(approval, "cabinet", {"name": "B", "units": "in", "width": 20, "doors": 1,
                                                "match": "selected", "place": "right"}),
                   select("B Top"),
                   plugin(approval, "corner", {"name": "K", "match": "selected", "place": "right"}),
                   {"command": "inspect.measure", "params": {}},
                   {"command": "parameters.change", "params": {"changes": {"Run_A_height": "800 mm", "Run_A_plinth": "150 mm"}}},
                   {"command": "inspect.measure", "params": {}},
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    box = boxes(results[6])
    assert box["B Side L"] == [300, -200, 0, 319, 200, 900], box["B Side L"]
    assert box["B Bottom"][2] == 100 and box["B Side R"][3] == 808, (box["B Bottom"], box["B Side R"])
    assert box["K Side L"][5] == 900 and box["K Bottom"][2] == 100, (box["K Side L"], box["K Bottom"])
    box = boxes(results[8])
    for cabinet in "ABK":
        assert box[f"{cabinet} Side L"][5] == 800 and box[f"{cabinet} Bottom"][2] == 150, cabinet
    assert not [f["name"] for f in results[9]["timeline"] if f["warning"]]


def test_match_follows_what_is_picked(approval):
    # The Cabinet hook gives Match selected Yes when a cabinet is picked and hides it otherwise.
    # A script that leaves `match` out gets that: B, with A's top selected, is matched. One that
    # says `selected` with nothing picked or selected is not refused: C is made unmatched.
    b = plugin(approval, "cabinet", {"name": "B", "place": "right"})
    del b["params"]["params"]["match"]
    results = cli([plugin(approval, "cabinet", {"name": "A", "height": 900}), select("A Top"), b,
                   plugin(approval, "cabinet", {"name": "C", "match": "selected", "place": "origin"}),
                   {"command": "document.inspect", "params": {"bodies": False, "timeline": False}}])
    params = {p["name"]: p.get("expression") for p in results[4]["params"]}
    assert params["B_height"] == params["A_height"] == "Run_A_height", params["B_height"]
    assert params["C_height"] != "Run_A_height", params["C_height"]


def test_a_run_shares_its_sizes_and_survives_a_deleted_cabinet(approval):
    # Shared groups (#137): B matched to A and C matched to B all point at A's run
    # (`Run_A_height`), not at each other. A deleted, with its parameters, breaks nothing:
    # the run's height still sizes B and C. Changing B's own parameter takes only B out.
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", "height": 900}), select("A Top"),
        plugin(approval, "cabinet", {"name": "B", "match": "selected", "place": "right"}), select("B Top"),
        plugin(approval, "cabinet", {"name": "C", "match": "selected", "place": "right"}),
        {"command": "document.inspect", "params": {"bodies": False, "timeline": False}},
        {"command": "component.delete", "params": {"component": "A"}},
        {"command": "parameters.delete", "params": {"name": "A_height"}},
        {"command": "parameters.change", "params": {"changes": {"Run_A_height": "800 mm", "B_height": "700 mm"}}},
        MEASURE])
    params = {p["name"]: p.get("expression") for p in results[5]["params"]}
    assert params["A_height"] == params["B_height"] == params["C_height"] == "Run_A_height", params
    assert params["Run_A_height"] == "900 mm", params["Run_A_height"]
    box = boxes(results[9])
    assert box["C Side L"][5] == 800 and box["B Side L"][5] == 700, (box["C Side L"], box["B Side L"])


def test_a_run_over_a_size_of_the_first_cabinet_stands_on_its_own(approval):
    # A's depth was an expression over its own height: the run takes the value (600 mm), so A's
    # height changed alone leaves B's depth, and A deleted with its parameters breaks nothing.
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", "height": 900, "depth": 600}),
        {"command": "parameters.change", "params": {"name": "A_depth", "expression": "A_height - 300 mm"}},
        select("A Top"),
        plugin(approval, "cabinet", {"name": "B", "match": "selected", "place": "right"}),
        {"command": "parameters.change", "params": {"name": "A_height", "expression": "700 mm"}},
        MEASURE,
        {"command": "component.delete", "params": {"component": "A"}},
        {"command": "parameters.delete", "params": {"name": "A_height"}},
        {"command": "parameters.delete", "params": {"name": "A_depth"}},
        {"command": "parameters.change", "params": {"name": "Run_A_depth", "expression": "500 mm"}},
        MEASURE,
        {"command": "document.inspect", "params": {"bodies": False, "timeline": True}}])
    before, after = boxes(results[5]), boxes(results[10])
    assert before["B Side L"][4] - before["B Side L"][1] == 600, before["B Side L"]
    assert after["B Side L"][4] - after["B Side L"][1] == 500, after["B Side L"]
    assert not [f["name"] for f in results[11]["timeline"] if f["error"]], results[11]["timeline"]
