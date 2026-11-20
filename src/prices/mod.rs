//! Prices kept in the design (`storage@0.1`), and the cut list's cost estimate from them.
//!
//! The stored document is small and plain: `{"currency": "EUR", "sheet": {"Birch plywood": 70},
//! "banding": {"1": 0.6}, "worktop": 120, "hardware": {"Knob": 3.5}}`. A sheet's price is for an
//! 18 mm sheet of the size the cut list lays out on; banding and worktops are per metre.

use serde_json::{json, Value};

/// The Prices dialog's sheet fields and the material each prices: the four of `material`.
const SHEETS: [(&str, &str); 4] = [
    ("sheet_birch", "Birch plywood"),
    ("sheet_mdf", "MDF"),
    ("sheet_melamine", "White melamine"),
    ("sheet_oak", "Oak"),
];

/// The banding fields and the cut list's `banding` choice each prices.
const BANDING: [(&str, &str); 3] = [
    ("banding_0_4", "0.4"),
    ("banding_1", "1"),
    ("banding_2", "2"),
];

/// The hardware fields and the item each prices, named as `cutlist::hardware` names it.
const HARDWARE: [(&str, &str); 7] = [
    ("hardware_hinge", "Hinge, 35 mm cup"),
    ("hardware_slide", "Drawer slide, pair"),
    ("hardware_shelf_pin", "Shelf pin, 5 mm"),
    ("hardware_confirmat", "Confirmat screw, 7 x 50 mm"),
    ("hardware_dowel", "Dowel, 8 x 35 mm"),
    ("hardware_knob", "Knob"),
    ("hardware_pull", "Pull, 128 mm"),
];

/// The cut list's note when the design has no prices.
pub const HINT: &str = "No prices in this design: set them with Prices for a cost estimate.";

/// The RFC 7396 merge patch for `storage.patch` from the Prices dialog: only the filled fields,
/// so an empty field keeps the stored price. A price is a number of 0 or more; `currency` is
/// text of at most 8 characters.
pub fn patch(params: &Value) -> Result<Value, String> {
    let price = |key: &str| match &params[key] {
        Value::Null => Ok(None),
        v if v.as_f64().is_some_and(|n| n >= 0.0) => Ok(Some(v.clone())),
        _ => Err(format!(
            "{key} must be a price of 0 or more, or empty to keep the stored one"
        )),
    };
    let mut patch = json!({});
    for (group, fields) in [
        ("sheet", &SHEETS[..]),
        ("banding", &BANDING[..]),
        ("hardware", &HARDWARE[..]),
    ] {
        for (key, name) in fields {
            if let Some(v) = price(key)? {
                patch[group][*name] = v;
            }
        }
    }
    if let Some(v) = price("worktop")? {
        patch["worktop"] = v;
    }
    match &params["currency"] {
        Value::Null => {}
        Value::String(s) if s.trim().is_empty() => {}
        Value::String(s) if s.trim().chars().count() <= 8 => patch["currency"] = json!(s.trim()),
        _ => return Err("currency must be text of at most 8 characters, such as EUR".into()),
    }
    Ok(patch)
}

/// Adds the cost of what the cut list `list` (with its `hardware`) buys at `prices`, the stored
/// document, as `cost`: a line per sheet stock, banding, worktop and hardware item, with its
/// quantity, unit price and amount, and the total. A line without a price (a material that is not
/// ours, a field never filled) has `price` and `amount` null and stays out of the total. Also
/// appends the cost to the CSV. Without any prices, only a `hint` that Prices sets them.
pub fn estimate(list: &mut Value, prices: &Value) {
    if !prices.as_object().is_some_and(|p| !p.is_empty()) {
        list["hint"] = json!(HINT);
        return;
    }
    let money = |v: f64| (v * 100.0).round() / 100.0;
    let units = list["units"].as_str().unwrap_or("mm");
    let shown = |v: &Value| {
        v.as_f64()
            .map_or_else(|| v.as_str().unwrap_or("").into(), |n| n.to_string())
    };
    let named = |m: &Value| match m.as_str().unwrap_or("") {
        "" => "No material".to_string(),
        m => m.to_string(),
    };
    let none = vec![];
    let all = |key: &str| list[key].as_array().unwrap_or(&none);
    let mut lines = Vec::new();
    let mut line = |item: String, qty: f64, unit: &str, price: Option<f64>| {
        let amount = price.map(|p| money(qty * p));
        lines.push(
            json!({"item": item, "qty": qty, "unit": unit, "price": price, "amount": amount}),
        );
    };
    // In an inch design the list shows inches as text, and adds the mm beside them.
    let mm = |v: &Value, key: &str| {
        v.get(format!("{key}_mm").as_str())
            .unwrap_or(&v[key])
            .as_f64()
    };
    for s in all("sheets") {
        let qty = s["sheets"].as_f64().unwrap_or(0.0);
        if qty == 0.0 {
            continue;
        }
        let at18 = prices["sheet"][s["material"].as_str().unwrap_or("")].as_f64();
        // ponytail: one price per material, for an 18 mm sheet, scaled by thickness; no price
        // per sheet size or per thickness.
        let price = at18
            .zip(mm(s, "thickness"))
            .map(|(p, t)| money(p * t / 18.0));
        let item = format!(
            "Sheet, {} {} {units}",
            named(&s["material"]),
            shown(&s["thickness"])
        );
        line(item, qty, "sheet", price);
    }
    let tape = list["banding_mm"].as_f64().unwrap_or(0.0).to_string(); // 0.4, 1 or 2
    for b in all("banding") {
        let item = format!(
            "Edge banding {tape} mm, {} {} {units}",
            named(&b["material"]),
            shown(&b["thickness"])
        );
        let metres = b["metres"].as_f64().unwrap_or(0.0);
        line(item, metres, "m", prices["banding"][tape.as_str()].as_f64());
    }
    for w in all("worktops") {
        let metres = mm(w, "length").unwrap_or(0.0) / 1000.0;
        let item = format!("Worktop, {}", w["part"].as_str().unwrap_or(""));
        line(item, metres, "m", prices["worktop"].as_f64());
    }
    for h in all("hardware") {
        let item = h["item"].as_str().unwrap_or("");
        let qty = h["qty"].as_f64().unwrap_or(0.0);
        // drawer slides are bought, and priced, in pairs
        let unit = if item.ends_with(", pair") {
            "pair"
        } else {
            "piece"
        };
        line(item.into(), qty, unit, prices["hardware"][item].as_f64());
    }
    let total = money(lines.iter().filter_map(|l| l["amount"].as_f64()).sum());
    let unpriced = lines.iter().filter(|l| l["price"].is_null()).count();
    let currency = prices["currency"].as_str().unwrap_or("").to_string();

    let quoted = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let cur = if currency.is_empty() {
        String::new()
    } else {
        format!(" ({currency})")
    };
    let mut csv = format!("\nCost estimate,Qty,Unit,Unit price{cur},Amount{cur}\n");
    for l in &lines {
        let (price, amount) = match (l["price"].as_f64(), l["amount"].as_f64()) {
            (Some(p), Some(a)) => (p.to_string(), a.to_string()),
            _ => (String::new(), "no price".to_string()),
        };
        let item = quoted(l["item"].as_str().unwrap_or(""));
        let (qty, unit) = (shown(&l["qty"]), l["unit"].as_str().unwrap_or(""));
        csv += &format!("{item},{qty},{unit},{price},{amount}\n");
    }
    csv += &format!("Total,,,,{total}\n");
    if unpriced > 0 {
        csv += &format!("{}\n", quoted(&unpriced_note(unpriced)));
    }
    if let Some(text) = list["csv"].as_str() {
        list["csv"] = json!(format!("{text}{csv}"));
    }
    list["cost"] =
        json!({"currency": currency, "lines": lines, "total": total, "unpriced": unpriced});
    if unpriced > 0 {
        list["cost"]["note"] = json!(unpriced_note(unpriced));
    }
}

/// What the reply, the CSV and the PDF say about lines without a price.
pub fn unpriced_note(n: usize) -> String {
    let (lines, have, are) = match n {
        1 => ("line", "has", "is"),
        _ => ("lines", "have", "are"),
    };
    format!("{n} {lines} {have} no price and {are} left out of the total: set prices with Prices.")
}

#[cfg(test)]
mod tests;
