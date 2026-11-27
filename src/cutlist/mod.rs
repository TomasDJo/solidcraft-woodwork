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
    // A leg or a caster is not a board, so never measured. A worktop is bought at its box, also
    // when it is mitred or has a cut-out, and it stands square in a component on the origin.
    bodies
        .filter(loose)
        .filter_map(|body| body["name"].as_str().map(String::from))
        .filter(|name| !crate::fittings::is_foot(name) && !worktop(name))
        .collect()
}

/// Whether a board is a worktop, bought to length: its name has `Worktop` in it, unless it is
/// one of a worktop's tool parts (`Worktop Tool shelf`), an ordinary board.
pub fn worktop(name: &str) -> bool {
    name.contains("Worktop") && !name.contains(" Tool ")
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
    // ponytail: a design drilled by the Joinery function of 0.2.0 and before: a screw or dowel
    // is a round hole in a board it drilled into (the bottom,
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
        let shelf = |b: &&String| b.starts_with(&start) && !b.ends_with(" return");
        bodies.iter().filter(shelf).count() as u64 // a split shelf is one shelf
    };
    let pulls = ending(" handle 2");
    // A drawer's slides by their length and type, in its left box side's first screw hole's
    // name (`<Drawer n Left> slide 1 (450 mm, full extension)`); drawers made before that are
    // only pairs.
    let mut slides: BTreeMap<&str, u64> = BTreeMap::new();
    for f in &features {
        let length = f
            .split_once(" Left slide 1 (")
            .map(|(_, l)| l.trim_end_matches(')'));
        if let Some(length) = length {
            *slides.entry(length).or_default() += 1;
        }
    }
    // a drawer on wooden runners (`<Name> Side L runner <n>`) has no slides
    let runners = |b: &String| {
        let runner = b.replacen(" Drawer ", " Side L runner ", 1);
        features.contains(&runner.trim_end_matches(" Front").to_string())
    };
    let drawers = |b: &&String| b.contains(" Drawer ") && b.ends_with(" Front") && !runners(b);
    let counts = [
        (
            "Hinge, cup",
            features
                .iter()
                .filter(|f| f.contains(" hinge cup ") && !f.ends_with(" (half overlay)"))
                .count() as u64,
        ),
        (
            // a door hinged on a divider covers only half of it (`cabinet::doors`)
            "Hinge, cup, half overlay",
            features
                .iter()
                .filter(|f| f.contains(" hinge cup ") && f.ends_with(" (half overlay)"))
                .count() as u64,
        ),
        (
            "Drawer slide, pair",
            (bodies.iter().filter(drawers).count() as u64).saturating_sub(slides.values().sum()),
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
        (
            "Dowel, 8 x 60 mm", // a split corner board's joints
            features
                .iter()
                .filter(|f| f.contains(" return dowel "))
                .count() as u64,
        ),
        (
            "Adjustable leg",
            bodies.iter().filter(|b| crate::fittings::is_leg(b)).count() as u64,
        ),
        (
            "Connector bolt, M6",
            features
                .iter()
                .filter(|f| f.contains(" connector bolt "))
                .count() as u64,
        ),
        (
            "Caster, locking",
            bodies
                .iter()
                .filter(|b| crate::fittings::is_caster(b))
                .count() as u64,
        ),
        ("Knob", (ending(" handle 1") - pulls) as u64),
        ("Pull, 128 mm", pulls as u64),
    ];
    let bought = counts.iter().filter(|(_, qty)| *qty > 0);
    let mut list: Vec<Value> = bought
        .map(|(item, qty)| json!({"item": item, "qty": qty}))
        .collect();
    // after the hinges
    let hinge = |h: &&Value| {
        h["item"]
            .as_str()
            .is_some_and(|i| i.starts_with("Hinge, cup"))
    };
    let at = list.iter().take_while(hinge).count();
    for (slide, qty) in slides.into_iter().rev() {
        let (length, kind) = slide.split_once(", ").unwrap_or((slide, ""));
        let item = format!("Drawer slide, {kind}, {length}, pair").replace(", ,", ",");
        list.insert(at, json!({"item": item, "qty": qty}));
    }
    // The joinery chosen when a cabinet was built counts its fasteners in its parameters
    // (`joint::hardware`); the old Joinery function's line of the same item takes them in.
    let cabinets = (bodies.iter()).filter_map(|b| b.strip_suffix(" Side L"));
    for cabinet in cabinets {
        for (item, qty) in crate::joint::hardware(&crate::kit::prefix(cabinet), &design["params"]) {
            match list.iter_mut().find(|line| line["item"] == item.as_str()) {
                Some(line) => line["qty"] = json!(line["qty"].as_u64().unwrap_or(0) + qty),
                None => list.push(json!({"item": item, "qty": qty})),
            }
        }
    }
    // A cabinet on a steel rail has `<Prefix>_wall_rail`, its width (0 once an edit took the
    // rail away), and a corner cabinet `_wall_rail_return` too: a rail bought to that length
    // and two hangers each. Only for a cabinet that is
    // still there, as a deleted cabinet's parameters stay.
    let standing: Vec<String> = (bodies.iter())
        .filter_map(|b| b.strip_suffix(" Side L"))
        .map(crate::kit::prefix)
        .collect();
    let mut rails: BTreeMap<i64, u64> = BTreeMap::new();
    for p in design["params"].as_array().into_iter().flatten() {
        let (Some(name), Some(mm)) = (p["name"].as_str(), p["value"].as_f64()) else {
            continue;
        };
        // a corner cabinet has one on each wall
        let cabinet = (name.strip_suffix("_wall_rail")).or(name.strip_suffix("_wall_rail_return"));
        if mm > 0.0 && cabinet.is_some_and(|c| standing.iter().any(|s| s == c)) {
            *rails.entry(mm.round() as i64).or_default() += 1;
        }
    }
    for (length, qty) in &rails {
        list.push(json!({"item": format!("Steel wall rail, {length} mm"), "qty": qty}));
    }
    let hangers: u64 = rails.values().sum::<u64>() * 2;
    if hangers > 0 {
        list.push(json!({"item": "Cabinet hanger", "qty": hangers}));
    }
    Value::from(list)
}

/// The sides cut with a toe-kick notch, from the features that cut them (`<Side> notch`) in
/// `timeline` (`document.inspect`'s reply): the cut list gives such a board's size over the
/// notch and says so.
pub fn notched(design: &Value) -> Vec<String> {
    let features = design["timeline"].as_array().into_iter().flatten();
    let sides = features.filter_map(|f| f["name"].as_str()?.strip_suffix(" notch"));
    sides
        .filter(|s| s.ends_with(" Side L") || s.ends_with(" Side R"))
        .map(String::from)
        .collect()
}

/// What a notched side's line in the cut list says.
pub const NOTCHED: &str = "notched for the toe kick; size over the notch";

/// The sides grooved for wooden drawer runners (`<Side> runner <n>`), as `notched`.
pub fn grooved(design: &Value) -> Vec<String> {
    let features = design["timeline"].as_array().into_iter().flatten();
    let sides = features.filter_map(|f| Some(f["name"].as_str()?.split_once(" runner ")?.0));
    let mut sides: Vec<String> = sides.map(String::from).collect();
    sides.dedup();
    sides
}

/// A corner cabinet's one-piece L-shaped boards, from the box cut that makes each one's open
/// corner (`<Board> notch`; a side's toe kick is named so too, and left out), as `notched`.
pub fn l_boards(design: &Value) -> Vec<String> {
    let features = design["timeline"].as_array().into_iter().flatten();
    let boards = features.filter_map(|f| f["name"].as_str()?.strip_suffix(" notch"));
    boards
        .filter(|s| !s.ends_with(" Side L") && !s.ends_with(" Side R"))
        .map(String::from)
        .collect()
}

/// An L-shaped board's open corner from `model.mesh` (in its own component, square to it):
/// its size along X, Y and Z (0 across the thickness), at the corner of its box that no point
/// reaches: along each edge of the box through that corner the points stop short of it by the
/// corner's size. None when every corner is filled.
pub fn open_corner(mesh: &Value) -> Option<[f64; 3]> {
    let points: Vec<[f64; 3]> = (mesh["positions"].as_array().into_iter().flatten())
        .filter_map(|p| Some([p[0].as_f64()?, p[1].as_f64()?, p[2].as_f64()?]))
        .collect();
    let lo = |i: usize| points.iter().map(|p| p[i]).fold(f64::MAX, f64::min);
    let hi = |i: usize| points.iter().map(|p| p[i]).fold(f64::MIN, f64::max);
    let mut axes = [0, 1, 2];
    axes.sort_by(|&i, &j| (hi(i) - lo(i)).total_cmp(&(hi(j) - lo(j))));
    let [_, a, b] = axes;
    // How far the points on the box's edge where `across` is `at` stay from `end` along `along`.
    let short = |along: usize, end: f64, across: usize, at: f64| {
        let on = points.iter().filter(|p| (p[across] - at).abs() < 0.01);
        on.map(|p| (p[along] - end).abs()).fold(f64::MAX, f64::min)
    };
    for ea in [lo(a), hi(a)] {
        for eb in [lo(b), hi(b)] {
            let (sa, sb) = (short(a, ea, b, eb), short(b, eb, a, ea));
            if sa > 1.0 && sb > 1.0 && sa < f64::MAX && sb < f64::MAX {
                let mut size = [0.0; 3];
                (size[a], size[b]) = (sa, sb);
                return Some(size);
            }
        }
    }
    None
}

/// The boards with finger joints (`<Board> finger <other board>`), as `notched`.
pub fn fingered(design: &Value) -> Vec<String> {
    let features = design["timeline"].as_array().into_iter().flatten();
    let boards = features.filter_map(|f| Some(f["name"].as_str()?.split_once(" finger ")?.0));
    let mut boards: Vec<String> = boards.map(String::from).collect();
    boards.sort();
    boards.dedup();
    boards
}

/// What a finger-jointed board's line in the cut list says.
pub const FINGERED: &str = "finger jointed; size over the fingers";

/// What a grooved side's line in the cut list says.
pub const GROOVED: &str = "grooved for the drawer runners";

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
/// none, all four, the front edge, which runs along the grain axis, or an end panel's front
/// and top edges. The grain axis is up for an upright board, the cabinet's width for one lying
/// flat, and none (the longest side) for a plinth, a drawer box's side, front or back, or a
/// part we do not know.
fn role(part: &str, width: usize) -> (Edges, Option<usize>) {
    // ponytail: by name only, no per-part override (that needs storage@0.1 or a body property).
    let drawer = part.starts_with("Drawer ");
    match part {
        "Side L" | "Side R" => (Edges::Front, Some(2)),
        "Top" | "Bottom" | "Rail front" | "Rail" => (Edges::Front, Some(width)),
        // a split corner board's return piece: its front edge runs front to back
        "Top return" | "Bottom return" => (Edges::Front, Some(1 - width)),
        _ if part.starts_with("Shelf ") && part.ends_with(" return") => {
            (Edges::Front, Some(1 - width))
        }
        "Rail back" => (Edges::None, Some(width)), // hidden under the worktop
        _ if part.starts_with("Shelf ") => (Edges::Front, Some(width)),
        _ if part.starts_with("Divider ") => (Edges::Front, Some(2)),
        _ if part.starts_with("Door")
            || drawer && part.ends_with(" Front")
            || part == "False front" =>
        {
            (Edges::All, Some(2))
        }
        _ if drawer && part.ends_with(" Bottom") => (Edges::None, Some(width)),
        "Back" | "Back return" => (Edges::None, Some(2)),
        // panels: an end panel shows its front and top edges, a filler its front, its return
        // is hidden behind it
        _ if part.starts_with("End panel ") => (Edges::FrontTop, Some(2)),
        _ if part.starts_with("Filler ") && part.ends_with(" return") => (Edges::None, Some(2)),
        _ if part.starts_with("Filler ") => (Edges::Front, Some(2)),
        // plinths, a drawer box's sides, front and back (grain along the box), and what we
        // do not know
        _ => (Edges::None, None),
    }
}

enum Edges {
    None,
    Front,
    FrontTop, // an upright board's front (along the length) and top edges
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
/// the PDF); and `_notched`, the boards `notched` found. Legs are hardware, not boards.
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
    let bodies: Vec<&Value> = (measure["bodies"].as_array())
        .ok_or("inspect.measure returned no bodies")?
        .iter()
        .filter(|b| !crate::fittings::is_foot(b["name"].as_str().unwrap_or("")))
        .collect();
    // A board's note: notched for the toe kick, grooved for runners, or both.
    let noted = |name: &str| {
        let has = |key: &str| (params[key].as_array().into_iter().flatten()).any(|n| n == name);
        let notes = [
            (has("_notched"), NOTCHED),
            (has("_grooved"), GROOVED),
            (has("_fingered"), FINGERED),
        ];
        let notes: Vec<&str> = notes.iter().filter(|n| n.0).map(|n| n.1).collect();
        notes.join("; ")
    };
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
    // Along its Back first: a left corner's Side L is the return's end, across the back leg.
    let width_axis = |cabinet: &str, name: &str| -> Result<usize, String> {
        let find = |part: &str| {
            let part = format!("{cabinet} {part}");
            let found = bodies.iter().find(|b| b["name"] == part.as_str());
            found.filter(|_| !local.contains_key(name) && !local.contains_key(&part))
        };
        Ok(match (find("Back"), find("Side L")) {
            (Some(back), _) => {
                let s = size(back)?;
                usize::from(s[1] > s[0])
            }
            (None, Some(side)) => {
                let s = size(side)?;
                usize::from(s[1] < s[0])
            }
            _ => 0,
        })
    };
    // Key: thickness, length, width in tenths of a mm, material, the banded edges along the
    // length and along the width, and whether it is notched.
    type Key = (i64, i64, i64, String, u64, u64, String);
    let mut rows: BTreeMap<Key, Vec<String>> = BTreeMap::new();
    let mut corners: BTreeMap<String, [f64; 2]> = BTreeMap::new(); // L boards' open corners
    for &body in &bodies {
        let name = body["name"].as_str().unwrap_or("?").to_string();
        let tenths = size(body)?.map(|mm| (mm * 10.0).round() as i64);
        let material = body["material"].as_str().unwrap_or("").to_string();
        // A drawer box's front with no `Drawer <n> Front` is the front (integrated, #163).
        let front = format!("{} Front", name.strip_suffix(" Box front").unwrap_or(&name));
        let part = match name.ends_with(" Box front") && !bodies.iter().any(|b| b["name"] == *front)
        {
            true => &front,
            false => &name,
        };
        let (edges, grain) = match crate::select::owner(measure, &name) {
            Some(cabinet) => role(&part[cabinet.len() + 1..], width_axis(cabinet, &name)?),
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
            Edges::FrontTop => (1, 1),
            Edges::Front if grain == Some(w) => (0, 1),
            Edges::Front => (1, 0),
        };
        let corner = |i: usize| params["_corners"][name.as_str()][i].as_f64();
        if let (Some(cl), Some(cw)) = (corner(l), corner(w)) {
            corners.insert(name.clone(), [cl, cw]);
        }
        let key = (
            tenths[t],
            tenths[l],
            tenths[w],
            material,
            le,
            we,
            noted(&name),
        );
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
    for ((t, l, w, material, le, we, notch), names) in &rows {
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
            // A worktop: bought to length.
            if worktop(name) {
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
            quoted(&match notch.is_empty() {
                false => format!("{} ({notch})", names.join(", ")),
                true => names.join(", "),
            })
        );
        let mut part = json!({
            "qty": qty, "length": l, "width": w, "thickness": t,
            "material": material, "grain": grain,
            "banding": {"length": le, "width": we}, "parts": names,
        });
        if !notch.is_empty() {
            part["note"] = json!(notch);
        }
        parts.push(part);
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
            "material": material, "thickness": shown(*t), "thickness_mm": mm(*t), "area_m2": round(*m2),
            "sheets": laid.len(), "yield_pct": if laid.is_empty() { 0.0 } else { percent(used, laid.len()) },
        }));
        for placed in &laid {
            let parts: Vec<Value> = (placed.iter())
                .map(|p| {
                    let (_, name, row) = &group[p.part];
                    let mut laid = json!({"part": name, "row": row, "x": tenth(p.x), "y": tenth(p.y), "w": tenth(p.w),
                           "h": tenth(p.h), "rotated": p.rotated});
                    // An L board's open corner, along x and y, at the part's far corner (the
                    // board turned or over to suit): drawn, not laid out on.
                    if let Some(&[cl, cw]) = corners.get(name) {
                        let (cx, cy) = if p.rotated { (cw, cl) } else { (cl, cw) };
                        laid["corner"] = json!([tenth(cx), tenth(cy)]);
                    }
                    laid
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
