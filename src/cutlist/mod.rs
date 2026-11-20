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

/// The size of a body from `model.mesh`'s reply: along its own component's axes, in X, Y, Z
/// order, unless its smallest enclosing box is clearly smaller (a board turned inside its own
/// component); then that box as length, width, thickness. The box's thickness is across the
/// flat faces with the most area, and its other two sides are the smallest rectangle around
/// the body seen along that, which always has a side on the outline's convex hull.
pub fn extents(mesh: &Value) -> [f64; 3] {
    let points: Vec<[f64; 3]> = (mesh["positions"].as_array().into_iter().flatten())
        .filter_map(|p| Some([p[0].as_f64()?, p[1].as_f64()?, p[2].as_f64()?]))
        .collect();
    let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|d| span(&points, d));
    let volume = |s: [f64; 3]| s[0] * s[1] * s[2];
    match oriented(mesh, &points) {
        Some(own) if volume(own) < 0.99 * volume(axes) => own,
        _ => axes,
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt();
    a.map(|c| c / n)
}

/// How far the points reach along direction `d`.
fn span(points: &[[f64; 3]], d: [f64; 3]) -> f64 {
    let along = points.iter().map(|&p| dot(p, d));
    along.clone().fold(f64::MIN, f64::max) - along.fold(f64::MAX, f64::min)
}

/// The smallest box around the mesh with its thickness across the faces with the most area:
/// length, width, thickness. None without triangles.
fn oriented(mesh: &Value, points: &[[f64; 3]]) -> Option<[f64; 3]> {
    // Area per face direction, n and -n as one (keyed in thousandths), summed as vectors.
    let mut area: BTreeMap<[i64; 3], [f64; 3]> = BTreeMap::new();
    for t in mesh["triangles"].as_array()? {
        let corner = |k: usize| points.get(t[k].as_u64()? as usize).copied();
        let (a, b, c) = (corner(0)?, corner(1)?, corner(2)?);
        let mut n = cross(
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let len = dot(n, n).sqrt();
        if len == 0.0 {
            continue;
        }
        let mut key = n.map(|c| (c / len * 1000.0).round() as i64);
        if key.iter().find(|&&k| k != 0).is_some_and(|&k| k < 0) {
            key = key.map(|k| -k);
            n = n.map(|c| -c);
        }
        let sum = area.entry(key).or_default();
        *sum = [sum[0] + n[0], sum[1] + n[1], sum[2] + n[2]];
    }
    let most = area
        .into_values()
        .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)));
    let n = unit(most?);
    // Two directions in the plane across n, and the points seen along n.
    let other = if n[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = unit(cross(n, other));
    let v = cross(n, u);
    let flat: Vec<[f64; 2]> = points.iter().map(|&p| [dot(p, u), dot(p, v)]).collect();
    let hull = hull(flat);
    let mut best: Option<[f64; 3]> = None;
    for (i, a) in hull.iter().enumerate() {
        let b = hull[(i + 1) % hull.len()];
        let e = [b[0] - a[0], b[1] - a[1]];
        let len = e[0].hypot(e[1]);
        let e = [e[0] / len, e[1] / len];
        let reach = |d: [f64; 2]| {
            let along = hull.iter().map(|p| p[0] * d[0] + p[1] * d[1]);
            along.clone().fold(f64::MIN, f64::max) - along.fold(f64::MAX, f64::min)
        };
        let (l, w) = (reach(e), reach([-e[1], e[0]]));
        if best.is_none_or(|s| l * w < s[0] * s[1]) {
            best = Some([l.max(w), l.min(w), 0.0]);
        }
    }
    best.map(|[l, w, _]| [l, w, span(points, n)])
}

/// The convex hull of points in a plane, by Andrew's monotone chain, without repeats.
fn hull(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let turn = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut hull: Vec<[f64; 2]> = Vec::new();
    for pass in [points.clone(), points.into_iter().rev().collect()] {
        let start = hull.len();
        for p in pass {
            while hull.len() >= start + 2
                && turn(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop(); // the last point starts the other chain
    }
    hull
}

/// The hardware to buy, counted from what the plugin made: feature names in `timeline`
/// (`document.inspect`'s reply) and bodies in `measure`.
pub fn hardware(measure: &Value, design: &Value) -> Value {
    let named = |list: &Value| -> Vec<String> {
        let items = list.as_array().into_iter().flatten();
        items
            .filter_map(|item| item["name"].as_str().map(String::from))
            .collect()
    };
    let (features, bodies) = (named(&design["timeline"]), named(&measure["bodies"]));
    let ending = |end: &str| features.iter().filter(|f| f.ends_with(end)).count();
    // Cabinets with a feature of this name, such as `<Cabinet> Side L pins`.
    let cabinets = |end: &'static str| features.iter().filter_map(move |f| f.strip_suffix(end));
    // ponytail: a screw or dowel is a round hole in the bottom or top panel, so a hole the
    // user drills there by hand is counted too.
    let holes = |cabinet: &str| -> u64 {
        let ends = [format!("{cabinet} Bottom"), format!("{cabinet} Top")];
        let all = measure["bodies"].as_array().into_iter().flatten();
        all.filter(|b| ends.iter().any(|end| b["name"] == end.as_str()))
            .filter_map(|b| b["face_types"]["cylinder"].as_u64())
            .sum()
    };
    let shelves = |cabinet: &str| {
        let start = format!("{cabinet} Shelf ");
        bodies.iter().filter(|b| b.starts_with(&start)).count() as u64
    };
    let pulls = ending(" handle 2");
    let drawers = |b: &&String| b.contains(" Drawer ") && b.ends_with(" Front");
    let counts = [
        (
            "Hinge, 35 mm cup",
            features
                .iter()
                .filter(|f| f.contains(" hinge cup "))
                .count() as u64,
        ),
        (
            "Drawer slide, pair",
            bodies.iter().filter(drawers).count() as u64,
        ),
        (
            "Shelf pin, 5 mm",
            // four under each shelf; eight under a corner cabinet's L-shaped one
            cabinets(" Side L pins")
                .map(|c| {
                    let corner = features.contains(&format!("{c} Back return pins"));
                    shelves(c) * if corner { 8 } else { 4 }
                })
                .sum(),
        ),
        (
            "Confirmat screw, 7 x 50 mm",
            cabinets(" joint Side L Bottom pilot").map(holes).sum(),
        ),
        (
            "Dowel, 8 x 35 mm",
            cabinets(" joint Side L Bottom dowel end").map(holes).sum(),
        ),
        ("Knob", (ending(" handle 1") - pulls) as u64),
        ("Pull, 128 mm", pulls as u64),
    ];
    let bought = counts.iter().filter(|(_, qty)| *qty > 0);
    bought
        .map(|(item, qty)| json!({"item": item, "qty": qty}))
        .collect()
}

/// The folder grant in `plugin.json` that a cut list is saved into.
pub const FOLDER: &str = "cutlists";

/// The file the cut list is to be saved as, from its `file` field (none when that is empty)
/// and its `format`, `pdf` (the default) or `csv`, which gives the ending. A name only, since
/// the folder is the one the user approved; an ending typed in the name gives way to the format.
pub fn file_name(entry: &str, params: &Value) -> Result<Option<String>, String> {
    let name = params["file"].as_str().unwrap_or("").trim();
    if entry != "cutlist" || name.is_empty() {
        return Ok(None);
    }
    let plain = |c: char| c.is_alphanumeric() || " -_.".contains(c);
    if !name.chars().all(plain) || name.starts_with('.') {
        return Err(
            "file: a file name of letters, digits, spaces, - _ and . only, such as kitchen".into(),
        );
    }
    let format = params["format"].as_str().unwrap_or("pdf");
    if !matches!(format, "pdf" | "csv") {
        return Err("format must be pdf or csv".into());
    }
    let lower = name.to_lowercase();
    let typed = lower.ends_with(".csv") || lower.ends_with(".pdf");
    let stem = if typed { &name[..name.len() - 4] } else { name };
    Ok(Some(format!("{stem}.{format}")))
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
mod tests;
