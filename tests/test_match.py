"""Match selected: a new cabinet takes and follows the selected one's sizes."""
from helpers import *


def test_match_selected(approval):
    # Match selected: B takes A's height, depth, thickness and plinth, and follows A afterwards.
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
                   {"command": "parameters.change", "params": {"changes": {"A_height": "800 mm", "A_plinth": "150 mm"}}},
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
