"""Connector bolts: M6 bolts through the touching sides of picked cabinets, without a base (#260)."""
from helpers import *


def test_connector_bolts_without_a_base(approval):
    # A and B side by side on the floor: four 9 mm holes through A Side R and B Side L, each with
    # a 20 mm counterbore 6 mm deep on the side's outer face (A Side R's at x 282, B Side L's at
    # 318, the sides meeting at 300). Run again, nothing is cut twice. The cut list counts four
    # bolts and eight washers.
    bare = {"width": 600, "shelves": 0, "doors": 0}
    results = designed(approval, NONE, [
        plugin(approval, "cabinet", {"name": "A", **bare}), select("A Side R"),
        plugin(approval, "cabinet", {"name": "B", "place": "right", **bare}),
        pick("A Side L", "B Side L"), plugin(approval, "bolts", {}),
        pick("B Side R", "A Side R"), plugin(approval, "bolts", {}), MEASURE,
        plugin(approval, "cutlist", {}), faces("A Side R"), faces("B Side L")])
    assert results[4]["result"]["bolts"] == 4, results[4]["result"]
    measure = results[7]
    for touching, free in (("A Side R", "A Side L"), ("B Side L", "B Side R")):
        assert cylinders(measure, touching) - cylinders(measure, free) == 8, (touching, cylinders(measure, touching))
    hardware = {h["item"]: h["qty"] for h in results[8]["result"]["hardware"]}
    assert hardware.get("Connector bolt, M6") == 4 and hardware.get("Washer, M6") == 8, hardware
    for result, outer in ((results[9], 282), (results[10], 318)):
        bores = [f for f in result["faces"] if abs((f.get("cylinder") or {}).get("radius", 0) - 10) < 0.01]
        # the counterbore's wall: half way into it, 3 mm in from the outer face
        assert len(bores) == 4 and all(abs(abs(f["centroid"][0] - outer) - 3) < 0.5 for f in bores), bores


def test_cabinets_that_do_not_touch_are_refused(approval):
    bare = {"width": 600, "shelves": 0, "doors": 0}
    error = refused([plugin(approval, "cabinet", {"name": "A", **bare}),
                     plugin(approval, "cabinet", {"name": "B", **bare}),
                     pick("A Side L", "B Side L"), plugin(approval, "bolts", {})])
    assert "touching" in error, error
