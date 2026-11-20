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
    // ponytail: a screw or dowel is a round hole in a board Joinery drills into (the bottom,
    // top, a sink base's rails), so a hole the user drills
    // there by hand is counted too.
    let holes = |cabinet: &str| -> u64 {
        let drilled = ["Bottom", "Top", "Rail front", "Rail back"];
        let ends = drilled.map(|board| format!("{cabinet} {board}"));
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
/// and its `format`, `pdf` (the default), `csv`, `labels` (a PDF too) or `dxf` (`save` writes one
/// file per sheet from it), which gives the ending. A name only, since
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
    let ending = match format {
        "pdf" | "labels" => "pdf",
        "csv" => "csv",
        "dxf" => "dxf",
        _ => return Err("format must be pdf, csv, labels or dxf".into()),
    };
    let lower = name.to_lowercase();
    let typed = [".csv", ".pdf", ".dxf"].iter().any(|e| lower.ends_with(e));
    let stem = if typed { &name[..name.len() - 4] } else { name };
    Ok(Some(format!("{stem}.{ending}")))
}

/// Which edges of a part are banded, and the axis its grain runs along, by its name in its
/// cabinet (`Side L`, `Shelf 2.1`): `width` is the cabinet's width axis. The banded edges are
/// none, all four, or the front edge, which runs along the grain axis. The grain axis is up for
/// an upright board, the cabinet's width for one lying flat, and none (the longest side) for a
/// plinth, a drawer box's side, front or back, or a part we do not know.
fn role(part: &str, width: usize) -> (Edges, Option<usize>) {
    // ponytail: by name only, no per-part override (that needs storage@0.1 or a body property).
    let drawer = part.starts_with("Drawer ");
    match part {
        "Side L" | "Side R" => (Edges::Front, Some(2)),
        "Top" | "Bottom" | "Rail front" => (Edges::Front, Some(width)),
        "Rail back" => (Edges::None, Some(width)), // hidden under the worktop
        _ if part.starts_with("Shelf ") => (Edges::Front, Some(width)),
        _ if part.starts_with("Divider ") => (Edges::Front, Some(2)),
        _ if part.starts_with("Door") || drawer && part.ends_with(" Front") => {
            (Edges::All, Some(2))
        }
        _ if drawer && part.ends_with(" Bottom") => (Edges::None, Some(width)),
        "Back" | "Back return" => (Edges::None, Some(2)),
        // plinths, a drawer box's sides, front and back (grain along the box), and what we
        // do not know
        _ => (Edges::None, None),
    }
}

enum Edges {
    None,
    Front,
    All,
}

/// The banded edges as the lists mark them: `L2 W2`, `L1`, `` for none. L counts the edges
/// along the length, W those along the width.
pub fn mark(length: u64, width: u64) -> String {
    let marks = [("L", length), ("W", width)];
    let banded = marks.iter().filter(|(_, n)| *n > 0);
    banded
        .map(|(edge, n)| format!("{edge}{n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

const BANDING_WASTE: f64 = 0.10; // ends trimmed off each edge

/// The sheet sizes the dialog offers, length and width in mm.
const SHEETS: [(&str, [f64; 2]); 4] = [
    ("2440x1220", [2440.0, 1220.0]),
    ("2500x1250", [2500.0, 1250.0]),
    ("2800x2070", [2800.0, 2070.0]),
    ("3050x1530", [3050.0, 1530.0]),
];

/// `local` holds the sizes of the turned bodies, measured in their own component. `params` is
/// the dialog: `banding` (the tape's thickness, `none`, `0.4`, `1` or `2` mm), `size`
/// (`finished`, or `cut`: less the tape on each banded edge), and for the cutting layouts
/// `sheet` (one of `SHEETS`), `kerf` and `trim` in mm, and `diagrams` (`yes` or `no`, read by
/// the PDF); and `_worktops`, from lib.rs: the names of the worktops made by Worktop.
pub fn cutlist(
    measure: &Value,
    local: &BTreeMap<String, [f64; 3]>,
    imperial: bool,
    params: &Value,
) -> Result<Value, String> {
    let band = match params["banding"].as_str().unwrap_or("1") {
        "none" => 0,
        "0.4" => 4,
        "1" => 10,
        "2" => 20,
        _ => return Err("banding must be none, 0.4, 1 or 2".into()),
    }; // in tenths of a mm
    let cut = match params["size"].as_str().unwrap_or("finished") {
        "finished" => false,
        "cut" => true,
        _ => return Err("size must be finished or cut".into()),
    };
    let sheet_name = params["sheet"].as_str().unwrap_or("2440x1220");
    let sheet = match SHEETS.iter().find(|(name, _)| *name == sheet_name) {
        Some((_, size)) => *size,
        None => return Err("sheet must be 2440x1220, 2500x1250, 2800x2070 or 3050x1530".into()),
    };
    let millimetres = |key: &str, default: f64, max: f64| match &params[key] {
        Value::Null => Ok(default),
        v => (v.as_f64().filter(|n| (0.0..=max).contains(n)))
            .ok_or(format!("{key} must be a number of mm from 0 to {max}")),
    };
    let kerf = millimetres("kerf", 4.0, 10.0)?;
    let trim = millimetres("trim", 10.0, 50.0)?;
    if !matches!(params["diagrams"].as_str().unwrap_or("yes"), "yes" | "no") {
        return Err("diagrams must be yes or no".into());
    }
    let bodies = measure["bodies"]
        .as_array()
        .ok_or("inspect.measure returned no bodies")?;
    if bodies.is_empty() {
        return Err("the design has no boards to list".into());
    }
    let size = |body: &Value| -> Result<[f64; 3], String> {
        let name = body["name"].as_str().unwrap_or("?");
        let mut size = [0.0; 3];
        for (i, s) in size.iter_mut().enumerate() {
            let max = body["bbox"]["max"][i].as_f64().ok_or("body without bbox")?;
            let min = body["bbox"]["min"][i].as_f64().ok_or("body without bbox")?;
            *s = local.get(name).map_or(max - min, |own| own[i]);
        }
        Ok(size)
    };
    // A cabinet's width axis: across its Side L. A turned board is measured in its own
    // component, whose X is the cabinet's width.
    let width_axis = |cabinet: &str, name: &str| -> Result<usize, String> {
        let side = format!("{cabinet} Side L");
        let found = bodies.iter().find(|b| b["name"] == side.as_str());
        Ok(match found {
            Some(body) if !local.contains_key(name) && !local.contains_key(&side) => {
                let s = size(body)?;
                usize::from(s[1] < s[0])
            }
            _ => 0,
        })
    };
    // Key: thickness, length, width in tenths of a mm, material, then the banded edges along
    // the length and along the width.
    type Key = (i64, i64, i64, String, u64, u64);
    let mut rows: BTreeMap<Key, Vec<String>> = BTreeMap::new();
    for body in bodies {
        let name = body["name"].as_str().unwrap_or("?").to_string();
        let tenths = size(body)?.map(|mm| (mm * 10.0).round() as i64);
        let material = body["material"].as_str().unwrap_or("").to_string();
        let (edges, grain) = match crate::select::owner(measure, &name) {
            Some(cabinet) => role(&name[cabinet.len() + 1..], width_axis(cabinet, &name)?),
            None => (Edges::None, None),
        };
        // Thinnest axis is the thickness; the length runs along the grain on a grained board,
        // else along the longest side.
        let mut axes = [0, 1, 2];
        axes.sort_by_key(|&i| tenths[i]);
        let (t, mut l, mut w) = (axes[0], axes[2], axes[1]);
        let grained = crate::material::GRAINED.contains(&material.as_str());
        if grained && grain == Some(w) {
            (l, w) = (w, l);
        }
        let (le, we) = match edges {
            Edges::None => (0, 0),
            _ if band == 0 => (0, 0),
            Edges::All => (2, 2),
            Edges::Front if grain == Some(w) => (0, 1),
            Edges::Front => (1, 0),
        };
        let key = (tenths[t], tenths[l], tenths[w], material, le, we);
        rows.entry(key).or_default().push(name);
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
    let sized = if cut { "Cut" } else { "Finished" };
    let mut parts = Vec::new();
    let mut csv = format!(
        "Qty,{sized} length ({units}),{sized} width ({units}),Thickness ({units}),Material,Grain,Banding,Parts\n"
    );
    let mut area_m2 = 0.0;
    let mut stock: BTreeMap<(String, i64), f64> = BTreeMap::new(); // area per material and thickness
    let mut tape: BTreeMap<(String, i64), f64> = BTreeMap::new(); // banding in mm, the same
    type Laid = (crate::nesting::Part, String, usize); // a part, its name and its row
    let mut pieces: BTreeMap<(String, i64), Vec<Laid>> = BTreeMap::new(); // to lay out, the same
    let mut worktops = Vec::new();
    for ((t, l, w, material, le, we), names) in &rows {
        let qty = names.len();
        let row_m2 = mm(*l) * mm(*w) * qty as f64 / 1e6;
        area_m2 += row_m2;
        let banded = (*le as f64 * mm(*l) + *we as f64 * mm(*w)) * qty as f64;
        if banded > 0.0 {
            *tape.entry((material.clone(), *t)).or_insert(0.0) += banded;
        }
        // Banding on the edges along the length takes from the width, and the other way round.
        let (cl, cw) = match cut {
            true => (l - *we as i64 * band, w - *le as i64 * band),
            false => (*l, *w),
        };
        let grain = crate::material::GRAINED.contains(&material.as_str());
        let (row, one_m2) = (parts.len(), mm(*l) * mm(*w) / 1e6);
        for name in names {
            // A worktop made by Worktop (`_worktops`, from lib.rs), or a board named so (a
            // hand-made one in an older design).
            let mut made = params["_worktops"].as_array().into_iter().flatten();
            if name.contains("Worktop") || made.any(|w| w == name.as_str()) {
                worktops.push(
                    json!({"part": name, "material": material, "thickness": shown(*t),
                                     "length": shown(cl), "width": shown(cw)}),
                );
                continue;
            }
            *stock.entry((material.clone(), *t)).or_insert(0.0) += one_m2;
            let part = crate::nesting::Part {
                length: mm(cl),
                width: mm(cw),
                grain,
            };
            let laid = pieces.entry((material.clone(), *t)).or_default();
            laid.push((part, name.clone(), row));
        }
        let (l, w, t) = (shown(cl), shown(cw), shown(*t));
        csv += &format!(
            "{qty},{},{},{},{},{},{},{}\n",
            text(&l),
            text(&w),
            text(&t),
            quoted(material),
            if grain { "yes" } else { "" },
            mark(*le, *we),
            quoted(&names.join(", "))
        );
        parts.push(json!({
            "qty": qty, "length": l, "width": w, "thickness": t,
            "material": material, "grain": grain,
            "banding": {"length": le, "width": we}, "parts": names,
        }));
    }
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    // The sheets to buy are those of the cutting layouts, in mm whatever the design's units.
    let tenth = |v: f64| (v * 10.0).round() / 10.0;
    let percent =
        |used: f64, sheets: usize| tenth(used / (sheets as f64 * sheet[0] * sheet[1]) * 100.0);
    let (mut sheets, mut layouts, mut oversize) = (Vec::new(), Vec::new(), Vec::new());
    for ((material, t), m2) in &stock {
        let group = &pieces[&(material.clone(), *t)];
        let parts: Vec<crate::nesting::Part> = (group.iter()).map(|(p, ..)| *p).collect();
        let (laid, big) = crate::nesting::nest(&parts, sheet, kerf, trim);
        let area =
            |placed: &[crate::nesting::Placed]| placed.iter().map(|p| p.w * p.h).sum::<f64>();
        let used: f64 = laid.iter().map(|placed| area(placed)).sum();
        sheets.push(json!({
            "material": material, "thickness": shown(*t), "area_m2": round(*m2),
            "sheets": laid.len(), "yield_pct": if laid.is_empty() { 0.0 } else { percent(used, laid.len()) },
        }));
        for placed in &laid {
            let parts: Vec<Value> = (placed.iter())
                .map(|p| {
                    let (_, name, row) = &group[p.part];
                    json!({"part": name, "row": row, "x": tenth(p.x), "y": tenth(p.y), "w": tenth(p.w),
                           "h": tenth(p.h), "rotated": p.rotated})
                })
                .collect();
            layouts.push(json!({
                "material": material, "thickness": mm(*t), "size": sheet,
                "yield_pct": percent(area(placed), 1), "parts": parts,
            }));
        }
        let room = [sheet[0] - 2.0 * trim, sheet[1] - 2.0 * trim];
        for i in big {
            let (p, name, _) = &group[i];
            let turned = p.grain && p.width <= room[0] && p.length <= room[1];
            let reason = match turned {
                true => "does not fit with its grain along the sheet",
                false => "bigger than the sheet less its trim",
            };
            oversize.push(
                json!({"part": name, "material": material, "thickness": mm(*t),
                                 "length": p.length, "width": p.width, "reason": reason}),
            );
        }
    }
    let feet = |m: f64| round(m / 0.3048);
    let banding: Vec<Value> = (tape.iter())
        .map(|((material, t), mm)| {
            let m = round(mm / 1000.0 * (1.0 + BANDING_WASTE));
            let mut total = json!({"material": material, "thickness": shown(*t), "metres": m});
            if imperial {
                total["feet"] = json!(feet(m));
            }
            total
        })
        .collect();
    if !banding.is_empty() {
        let length = if imperial { "Feet" } else { "Metres" };
        csv += &format!(
            "\nEdge banding {} mm,Thickness ({units}),{length} with 10 % waste\n",
            mm(band)
        );
        for total in &banding {
            let figure = if imperial {
                &total["feet"]
            } else {
                &total["metres"]
            };
            let material = total["material"].as_str().unwrap_or("");
            csv += &format!(
                "{},{},{}\n",
                quoted(material),
                text(&total["thickness"]),
                text(figure)
            );
        }
    }
    let mut list = json!({
        "units": units,
        "size": if cut { "cut" } else { "finished" },
        "banding_mm": mm(band),
        "sheets": sheets,
        "sheet_mm": sheet,
        "kerf_mm": kerf,
        "trim_mm": trim,
        "layout_units": "mm",
        "layouts": layouts,
        "oversize": oversize,
        "worktops": worktops,
        "banding": banding,
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
