//! Cost estimate (#56): the cut list's sheets, edge banding and hardware priced from Woodwork
//! settings' Prices, a text of `name = price` entries (`White melamine 18 = 42.50; Hinge = 3.20;
//! Banding = 0.60`). A sheet's price is per sheet of that material and thickness in mm, the
//! banding's per metre, a hardware item's per piece: the entry whose name the item starts with
//! (the longest), so `Hinge` prices every hinge line. No currency: the numbers as given.

use serde_json::{json, Value};

/// A name as it is matched: lower case, single spaces, a number as a number (`18.0` is `18`).
fn key(name: &str) -> String {
    let words = name.split_whitespace().map(|w| match w.parse::<f64>() {
        Ok(n) => format!("{n}"),
        Err(_) => w.to_lowercase(),
    });
    words.collect::<Vec<_>>().join(" ")
}

/// The Prices text as (name, price) pairs, names as `key` makes them. Entries are separated by
/// `;` or new lines; an empty one is skipped.
pub fn prices(text: &str) -> Result<Vec<(String, f64)>, String> {
    let entries = text
        .split([';', '\n'])
        .map(str::trim)
        .filter(|e| !e.is_empty());
    entries
        .map(|entry| {
            let (name, price) = entry
                .rsplit_once('=')
                .ok_or(format!("price `{entry}`: write it as name = price"))?;
            let price = price.trim().replace(',', ".");
            let price: f64 = (price.parse().ok().filter(|p: &f64| *p >= 0.0))
                .ok_or(format!("price `{entry}`: {price} is not a price"))?;
            Ok((key(name), price))
        })
        .collect()
}

fn money(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The estimate for a cut list (`cutlist::cutlist`'s reply with its `hardware`): a line for
/// each sheet size, the banding and each hardware item, priced where Prices has it, the total of
/// those priced, and `missing`: an entry to add for each line without a price, ready to copy.
pub fn estimate(list: &Value, text: &str) -> Result<Value, String> {
    let prices = prices(text)?;
    let exact = |name: &str| prices.iter().find(|p| p.0 == key(name)).map(|p| p.1);
    let starting = |item: &str| {
        let item = key(item);
        (prices.iter())
            .filter(|p| item.starts_with(&p.0))
            .max_by_key(|p| p.0.len())
            .map(|p| p.1)
    };
    let (mut lines, mut missing, mut total) = (Vec::new(), Vec::new(), 0.0);
    let mut line = |item: String, qty: f64, unit: &str, price: Option<f64>, ask: String| {
        let mut row = json!({"item": item, "qty": qty, "unit": unit});
        match price {
            Some(p) => {
                row["price"] = json!(p);
                // the total adds the line totals as shown, so the PDF and CSV add up
                row["total"] = json!(money(p * qty));
                total += money(p * qty);
            }
            None if !missing.contains(&ask) => missing.push(ask),
            None => {}
        }
        lines.push(row);
    };
    for s in list["sheets"].as_array().into_iter().flatten() {
        let (material, t) = (s["material"].as_str().unwrap_or("?"), &s["thickness_mm"]);
        let name = format!("{material} {}", t.as_f64().unwrap_or(0.0));
        let qty = s["sheets"].as_f64().unwrap_or(0.0);
        line(
            name.clone(),
            qty,
            if qty == 1.0 { "sheet" } else { "sheets" },
            exact(&name),
            format!("{name} = ?"),
        );
    }
    let metres: f64 = (list["banding"].as_array().into_iter().flatten())
        .filter_map(|b| b["metres"].as_f64())
        .sum();
    if metres > 0.0 {
        let item = "Banding".to_string();
        line(item, metres, "m", exact("Banding"), "Banding = ?".into());
    }
    for h in list["hardware"].as_array().into_iter().flatten() {
        let item = h["item"].as_str().unwrap_or("?").to_string();
        let qty = h["qty"].as_f64().unwrap_or(0.0);
        // what to ask for: the item up to its first comma (`Hinge`, `Drawer slide`)
        let ask = format!("{} = ?", item.split(',').next().unwrap_or(&item));
        let price = starting(&item);
        line(item, qty, "pcs", price, ask);
    }
    // `given`: Prices has entries, so the PDF shows the table even when none matches
    let given = !prices.is_empty();
    Ok(json!({"lines": lines, "total": money(total), "missing": missing, "given": given}))
}

/// The estimate as a CSV section, after the cut list's.
pub fn csv(cost: &Value) -> String {
    let mut out = String::from("\nCost,Qty,Unit,Price,Total\n");
    for l in cost["lines"].as_array().into_iter().flatten() {
        let item = l["item"].as_str().unwrap_or("").replace('"', "\"\"");
        let number = |v: &Value| v.as_f64().map(|n| n.to_string()).unwrap_or_default();
        let (qty, unit) = (number(&l["qty"]), l["unit"].as_str().unwrap_or(""));
        out += &format!(
            "\"{item}\",{qty},{unit},{},{}\n",
            number(&l["price"]),
            number(&l["total"])
        );
    }
    out += &format!("Total,,,,{}\n", cost["total"]);
    for m in cost["missing"].as_array().into_iter().flatten() {
        let ask = m.as_str().unwrap_or("").replace('"', "\"\"");
        out += &format!("\"No price: {ask}\"\n");
    }
    out
}

#[cfg(test)]
mod tests;
