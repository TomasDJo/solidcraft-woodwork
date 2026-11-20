use super::*;

#[test]
fn the_patch_has_only_the_filled_fields() {
    let params = json!({
        "sheet_birch": 70, "sheet_oak": 0, "banding_1": 0.6, "worktop": 120.5,
        "hardware_knob": 3.5, "currency": " EUR ", "sheet_mdf": null,
    });
    let expected = json!({
        "sheet": {"Birch plywood": 70, "Oak": 0}, "banding": {"1": 0.6}, "worktop": 120.5,
        "hardware": {"Knob": 3.5}, "currency": "EUR",
    });
    assert_eq!(patch(&params).unwrap(), expected);
    // Nothing filled: an empty patch, nothing written.
    assert_eq!(patch(&json!({"currency": ""})).unwrap(), json!({}));
}

#[test]
fn a_bad_price_is_refused() {
    for bad in [
        json!({"sheet_mdf": -1}),
        json!({"hardware_hinge": "4"}),
        json!({"currency": "EURO-DOLLAR"}),
        json!({"currency": 5}),
    ] {
        assert!(patch(&bad).is_err(), "{bad}");
    }
    assert!(patch(&json!({"worktop": -0.01}))
        .unwrap_err()
        .contains("worktop"));
}

#[test]
fn the_hardware_prices_are_the_items_the_cut_list_counts() {
    let named =
        |names: &[&str]| -> Vec<Value> { names.iter().map(|n| json!({"name": n})).collect() };
    let design = json!({"timeline": named(&[
        "A Door hinge cup 1", "A Door handle 1", "A Door handle 2", "B Door handle 1",
        "A Side L pins", "A joint Side L Bottom pilot", "B joint Side L Bottom dowel end",
    ])});
    let mut bodies = named(&["A Shelf 1", "A Drawer 1 Front"]);
    bodies.push(json!({"name": "A Bottom", "face_types": {"cylinder": 2}}));
    bodies.push(json!({"name": "B Bottom", "face_types": {"cylinder": 2}}));
    let counted = crate::cutlist::hardware(&json!({"bodies": bodies}), &design);
    let items: Vec<&str> = counted
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["item"].as_str().unwrap())
        .collect();
    let priced: Vec<&str> = HARDWARE.iter().map(|(_, item)| *item).collect();
    assert_eq!(items, priced);
}

/// A cut list with two sheet stocks (one not ours), banding, a worktop and hardware.
fn list() -> Value {
    json!({
        "units": "mm", "banding_mm": 1.0, "csv": "Qty\n",
        "sheets": [
            {"material": "Birch plywood", "thickness": 18.0, "sheets": 2},
            {"material": "Birch plywood", "thickness": 6.0, "sheets": 1},
            {"material": "Pine", "thickness": 18.0, "sheets": 1},
            {"material": "Oak", "thickness": 18.0, "sheets": 0},
        ],
        "banding": [{"material": "Birch plywood", "thickness": 18.0, "metres": 7.5}],
        "worktops": [{"part": "Worktop", "length": 2400.0}],
        "hardware": [{"item": "Hinge, 35 mm cup", "qty": 4}, {"item": "Knob", "qty": 2}],
    })
}

fn prices() -> Value {
    json!({"currency": "EUR", "sheet": {"Birch plywood": 72, "Oak": 90}, "banding": {"1": 0.6, "2": 1.1},
           "worktop": 150, "hardware": {"Hinge, 35 mm cup": 2.5}})
}

#[test]
fn the_cost_of_each_line_and_the_total() {
    let mut listed = list();
    estimate(&mut listed, &prices());
    let cost = &listed["cost"];
    let line = |item: &str| {
        let lines = cost["lines"].as_array().unwrap();
        let found = lines.iter().find(|l| l["item"] == item);
        found
            .unwrap_or_else(|| panic!("{item} in {lines:?}"))
            .clone()
    };
    // Sheets: the 18 mm price, and a third of it for 6 mm.
    let birch = line("Sheet, Birch plywood 18 mm");
    assert_eq!(
        (
            birch["qty"].as_f64(),
            birch["price"].as_f64(),
            birch["amount"].as_f64()
        ),
        (Some(2.0), Some(72.0), Some(144.0))
    );
    assert_eq!(line("Sheet, Birch plywood 6 mm")["amount"], 24.0);
    // Banding by the metre at the chosen tape's price, the worktop by its length.
    let banding = line("Edge banding 1 mm, Birch plywood 18 mm");
    assert_eq!(
        (banding["unit"].as_str(), banding["amount"].as_f64()),
        (Some("m"), Some(4.5))
    );
    let worktop = line("Worktop, Worktop");
    assert_eq!(
        (worktop["qty"].as_f64(), worktop["amount"].as_f64()),
        (Some(2.4), Some(360.0))
    );
    assert_eq!(line("Hinge, 35 mm cup")["amount"], 10.0);
    // Not ours, and no price for knobs: listed, out of the total. No sheets of oak: no line.
    for unpriced in ["Sheet, Pine 18 mm", "Knob"] {
        let l = line(unpriced);
        assert!(l["price"].is_null() && l["amount"].is_null(), "{l}");
    }
    assert_eq!(cost["lines"].as_array().unwrap().len(), 7);
    assert_eq!(cost["total"], 144.0 + 24.0 + 4.5 + 360.0 + 10.0);
    assert_eq!(
        (cost["unpriced"].as_u64(), cost["currency"].as_str()),
        (Some(2), Some("EUR"))
    );
    assert!(cost["note"]
        .as_str()
        .unwrap()
        .starts_with("2 lines have no price"));
    assert!(listed["hint"].is_null());
    let csv = listed["csv"].as_str().unwrap();
    assert!(
        csv.starts_with("Qty\n\nCost estimate,Qty,Unit,Unit price (EUR),Amount (EUR)\n"),
        "{csv}"
    );
    assert!(
        csv.contains("\"Sheet, Birch plywood 18 mm\",2,sheet,72,144\n"),
        "{csv}"
    );
    assert!(
        csv.contains("\"Knob\",2,piece,,no price\n") && csv.contains("Total,,,,542.5\n"),
        "{csv}"
    );
}

#[test]
fn an_inch_design_is_priced_by_its_mm() {
    let mut listed = json!({
        "units": "in", "banding_mm": 2.0,
        "sheets": [{"material": "Oak", "thickness": "23/32", "thickness_mm": 18.0, "sheets": 1}],
        "banding": [{"material": "Oak", "thickness": "23/32", "metres": 10.0, "feet": 32.808}],
        "worktops": [{"part": "Worktop", "length": "78 3/4", "length_mm": 2000.0}],
    });
    estimate(&mut listed, &prices());
    let lines = listed["cost"]["lines"].as_array().unwrap();
    let amounts: Vec<f64> = lines
        .iter()
        .map(|l| l["amount"].as_f64().unwrap())
        .collect();
    assert_eq!(amounts, [90.0, 11.0, 300.0]);
    assert_eq!(lines[0]["item"], "Sheet, Oak 23/32 in");
}

#[test]
fn without_prices_only_a_hint() {
    for stored in [Value::Null, json!({})] {
        let mut listed = list();
        estimate(&mut listed, &stored);
        assert!(listed["cost"].is_null());
        assert_eq!(listed["hint"], HINT);
        assert_eq!(listed["csv"], "Qty\n");
    }
}

#[test]
fn drawer_slides_are_priced_by_the_pair() {
    let mut listed =
        json!({"hardware": [{"item": "Drawer slide, pair", "qty": 3}, {"item": "Knob", "qty": 2}]});
    estimate(
        &mut listed,
        &json!({"hardware": {"Drawer slide, pair": 12}}),
    );
    let lines = &listed["cost"]["lines"];
    assert_eq!(
        (lines[0]["unit"].as_str(), lines[0]["amount"].as_f64()),
        (Some("pair"), Some(36.0))
    );
    assert_eq!(lines[1]["unit"], "piece");
}
