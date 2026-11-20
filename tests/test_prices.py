"""Prices kept in the design (storage@0.1), and the cut list's cost estimate from them."""
import subprocess

from helpers import *

PRICES = {"currency": "EUR", "sheet_birch": 72, "banding_1": 0.6, "worktop": 150, "hardware_hinge": 2.5,
          "hardware_knob": 3}
STORED = {"currency": "EUR", "sheet": {"Birch plywood": 72}, "banding": {"1": 0.6}, "worktop": 150,
          "hardware": {"Hinge, 35 mm cup": 2.5, "Knob": 3}}


def test_prices_and_cost(approval):
    cabinet = plugin(approval, "cabinet", {"doors": 2, "handles": "knob", "shelf_pins": True})
    worktop = {"command": "solid.box", "params": {"length": "2000 mm", "width": "600 mm", "height": "38 mm",
               "corner": [1000, 0, 0], "name": "Worktop", "body_name": "Worktop"}}
    saved = ROOT / "dist" / "prices.sc"
    results = cli([cabinet, worktop, plugin(approval, "cutlist"), plugin(approval, "prices", PRICES),
                   plugin(approval, "cutlist")], "--save", str(saved))
    before, stored, listed = results[2]["result"], results[3]["result"], results[4]["result"]
    # No prices yet: no cost, a hint.
    assert "cost" not in before and "Prices" in before["hint"], before.get("hint")
    assert stored["prices"] == STORED, stored
    cost = listed["cost"]
    lines = {line["item"]: line for line in cost["lines"]}
    # Sheets at the 18 mm price, the 6 mm back at a third of it.
    sheets = {s["thickness"]: s["sheets"] for s in listed["sheets"]}
    assert lines["Sheet, Birch plywood 18 mm"]["price"] == 72 and lines["Sheet, Birch plywood 18 mm"]["qty"] == sheets[18]
    assert lines["Sheet, Birch plywood 6 mm"]["price"] == 24 and lines["Sheet, Birch plywood 6 mm"]["amount"] == 24 * sheets[6]
    metres = next(b["metres"] for b in listed["banding"] if b["thickness"] == 18)
    assert abs(lines["Edge banding 1 mm, Birch plywood 18 mm"]["amount"] - round(metres * 0.6, 2)) < 1e-9
    assert lines["Worktop, Worktop"]["qty"] == 2 and lines["Worktop, Worktop"]["amount"] == 300
    hardware = {h["item"]: h["qty"] for h in listed["hardware"]}
    assert lines["Hinge, 35 mm cup"]["amount"] == 2.5 * hardware["Hinge, 35 mm cup"] == 10
    assert lines["Knob"]["amount"] == 3 * hardware["Knob"] == 6
    # The shelf pins have no price: listed, out of the total.
    assert lines["Shelf pin, 5 mm"]["amount"] is None and cost["unpriced"] == 1 and "1 line" in cost["note"]
    priced = [line["amount"] for line in cost["lines"] if line["amount"] is not None]
    assert abs(cost["total"] - sum(priced)) < 1e-6 and cost["total"] > 300 + 72, cost
    assert "Cost estimate,Qty,Unit,Unit price (EUR),Amount (EUR)" in listed["csv"] and f"Total,,,,{cost['total']}" in listed["csv"]
    # The prices are saved in the design: opened again they are there, and an empty Prices run
    # keeps them while a filled field changes only its own price.
    results = cli([{"command": "storage.get", "params": {"plugin": "woodwork"}}, plugin(approval, "prices"),
                   plugin(approval, "prices", {"hardware_knob": 4}), plugin(approval, "cutlist")], "--in", str(saved))
    assert results[0]["document"] == STORED, results[0]
    assert results[1]["result"]["prices"] == STORED
    assert results[2]["result"]["prices"]["hardware"] == {"Hinge, 35 mm cup": 2.5, "Knob": 4}
    assert abs(results[3]["result"]["cost"]["total"] - (cost["total"] + 2)) < 1e-6
    # A negative price is refused (the unit tests check the message).
    try:
        cli([plugin(approval, "prices", {"worktop": -1})])
    except subprocess.CalledProcessError:
        return
    raise AssertionError("a negative price went through")
