//! Cut list from `inspect.measure`, in mm or in inches as fractions.

use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Millimetres as inches to the nearest 1/32: `23 1/4`, `3/4`, `24`.
fn inches(mm: f64) -> String {
    let n = (mm / 25.4 * 32.0).round() as i64;
    let (whole, mut num, mut den) = (n / 32, n % 32, 32);
    while num != 0 && num % 2 == 0 {
        num /= 2;
        den /= 2;
    }
    match (whole, num) {
        (w, 0) => format!("{w}"),
        (0, n) => format!("{n}/{den}"),
        (w, n) => format!("{w} {n}/{den}"),
    }
}

const SHEET_M2: f64 = 2.44 * 1.22; // a full sheet, 2440 x 1220 mm (4 x 8 ft)
const WASTE: f64 = 0.15; // offcuts and saw kerf

/// The bodies that do not fill their world bounding box: a board that stands turned off the
/// axes, such as an open door, or a part that is not a plain board. A board square to the axes
/// fills its box but for holes and grooves, which take well under 3 %.
/// `measure` is `inspect.measure`'s reply. (The commands that list placements are not open to
/// a read-only entry, hence this test on the sizes.)
pub fn turned(measure: &Value) -> Vec<String> {
    let bodies = measure["bodies"].as_array().into_iter().flatten();
    let loose = |body: &&Value| {
        let side = |i: usize| {
            body["bbox"]["max"][i]
                .as_f64()
                .zip(body["bbox"]["min"][i].as_f64())
        };
        let sides = (0..3).filter_map(side).map(|(max, min)| max - min);
        (body["volume_mm3"].as_f64()).is_some_and(|volume| volume < 0.97 * sides.product::<f64>())
    };
    bodies
        .filter(loose)
        .filter_map(|body| body["name"].as_str().map(String::from))
        .collect()
}

/// The size of a body in its own component's axes, from `model.mesh`'s reply.
pub fn extents(mesh: &Value) -> [f64; 3] {
    let mut size = [0.0; 3];
    for (i, s) in size.iter_mut().enumerate() {
        let along = || {
            mesh["positions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |p| p[i].as_f64())
        };
        *s = along().fold(f64::MIN, f64::max) - along().fold(f64::MAX, f64::min);
    }
    size
}

/// `local` holds the sizes of the turned bodies, measured in their own component.
pub fn cutlist(
    measure: &Value,
    local: &BTreeMap<String, [f64; 3]>,
    imperial: bool,
) -> Result<Value, String> {
    let bodies = measure["bodies"]
        .as_array()
        .ok_or("inspect.measure returned no bodies")?;
    // Key: thickness, length, width in tenths of a mm, then material.
    let mut rows: BTreeMap<(i64, i64, i64, String), Vec<String>> = BTreeMap::new();
    for body in bodies {
        // A board square to the axes is as big as its world bounding box. A turned one is
        // not, and comes measured in its own component.
        let name = body["name"].as_str().unwrap_or("?").to_string();
        let mut size = [0i64; 3];
        for (i, s) in size.iter_mut().enumerate() {
            let max = body["bbox"]["max"][i].as_f64().ok_or("body without bbox")?;
            let min = body["bbox"]["min"][i].as_f64().ok_or("body without bbox")?;
            let mm = local.get(&name).map_or(max - min, |own| own[i]);
            *s = (mm * 10.0).round() as i64;
        }
        size.sort();
        let material = body["material"].as_str().unwrap_or("").to_string();
        rows.entry((size[0], size[2], size[1], material))
            .or_default()
            .push(name);
    }

    let mm = |tenths: i64| tenths as f64 / 10.0;
    // A size as the list shows it: a number of mm, or inches as text.
    let shown = |tenths: i64| {
        if imperial {
            json!(inches(mm(tenths)))
        } else {
            json!(mm(tenths))
        }
    };
    // For the CSV: `702`, not JSON's `702.0`.
    let text = |v: &Value| {
        v.as_f64()
            .map_or_else(|| v.as_str().unwrap().into(), |n| n.to_string())
    };
    let quoted = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let units = if imperial { "in" } else { "mm" };
    let mut parts = Vec::new();
    let mut csv =
        format!("Qty,Length ({units}),Width ({units}),Thickness ({units}),Material,Parts\n");
    let mut area_m2 = 0.0;
    let mut stock: BTreeMap<(String, i64), f64> = BTreeMap::new(); // area per material and thickness
    for ((t, l, w, material), names) in &rows {
        let qty = names.len();
        let row_m2 = mm(*l) * mm(*w) * qty as f64 / 1e6;
        area_m2 += row_m2;
        *stock.entry((material.clone(), *t)).or_insert(0.0) += row_m2;
        let (l, w, t) = (shown(*l), shown(*w), shown(*t));
        csv += &format!(
            "{qty},{},{},{},{},{}\n",
            text(&l),
            text(&w),
            text(&t),
            quoted(material),
            quoted(&names.join(", "))
        );
        parts.push(json!({
            "qty": qty, "length": l, "width": w, "thickness": t,
            "material": material, "parts": names,
        }));
    }
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    // ponytail: sheets to buy are estimated from area with 15 % waste, not from a cutting
    // layout, and a part longer than a sheet is not noticed. Nest the parts for a real count.
    let sheets: Vec<Value> = (stock.iter())
        .map(|((material, t), m2)| {
            json!({
                "material": material, "thickness": shown(*t), "area_m2": round(*m2),
                "sheets_2440x1220": (m2 * (1.0 + WASTE) / SHEET_M2).ceil(),
            })
        })
        .collect();
    let mut list = json!({
        "units": units,
        "sheets": sheets,
        "parts": parts,
        "boards": bodies.len(),
        "area_m2": round(area_m2),
        "csv": csv,
    });
    if imperial {
        list["area_ft2"] = json!(round(area_m2 * 10.7639));
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure() -> Value {
        let body = |name: &str, max: [f64; 3]| json!({"name": name, "material": "Default", "bbox": {"min": [0, 0, 0], "max": max}});
        json!({"bodies": [
            body("Side L", [19.05, 558.8, 762.0]),
            body("Side R", [19.05, 558.8, 762.0]),
            body("Back", [582.0, 6.0, 702.0]),
        ]})
    }

    #[test]
    fn groups_equal_boards() {
        let list = cutlist(&measure(), &BTreeMap::new(), false).unwrap();
        assert_eq!(list["boards"], 3);
        // 6 mm: 0.41 m2, 18 mm: 0.85 m2, one sheet each
        assert_eq!(list["sheets"][0]["thickness"], 6.0);
        assert_eq!(list["sheets"][1]["sheets_2440x1220"], 1.0);
        assert_eq!(list["parts"][0]["thickness"], 6.0); // thinnest first
        assert_eq!(list["parts"][1]["qty"], 2);
        assert_eq!(list["parts"][1]["length"], 762.0);
        let csv = list["csv"].as_str().unwrap();
        assert!(
            csv.contains("2,762,558.8,19.1,\"Default\",\"Side L, Side R\""),
            "{csv}"
        );
    }

    #[test]
    fn an_open_door_is_measured_in_its_own_component() {
        let body = |name: &str, max: [f64; 3], volume: f64| json!({"name": name, "volume_mm3": volume, "bbox": {"min": [0, 0, 0], "max": max}});
        let board = 397.0 * 18.0 * 716.0;
        let measured = json!({"bodies": [
            body("Closed", [397.0, 18.0, 716.0], board - 25000.0), // two hinge cups
            body("Ajar", [397.4, 31.9, 716.0], board),             // 2 degrees open
            body("Open", [296.0, 355.0, 716.0], board),
        ]});
        assert_eq!(turned(&measured), ["Ajar", "Open"]);

        let mesh = json!({"positions": [[0, 0, 0], [397, 18, 716], [397, 0, 0]]});
        let local = BTreeMap::from([("Side L".to_string(), extents(&mesh))]);
        let list = cutlist(&measure(), &local, false).unwrap();
        assert!(list["csv"]
            .as_str()
            .unwrap()
            .contains("1,716,397,18,\"Default\",\"Side L\""));
    }

    #[test]
    fn inches_as_fractions() {
        assert_eq!(inches(19.05), "3/4");
        assert_eq!(inches(609.6), "24");
        assert_eq!(inches(590.55), "23 1/4");
        let list = cutlist(&measure(), &BTreeMap::new(), true).unwrap();
        let csv = list["csv"].as_str().unwrap();
        assert!(
            csv.contains("2,30,22,3/4,\"Default\",\"Side L, Side R\""),
            "{csv}"
        );
        assert_eq!(list["parts"][1]["length"], "30");
    }
}
