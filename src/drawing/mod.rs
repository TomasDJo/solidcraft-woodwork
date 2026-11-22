//! Shop drawings: a sheet per cabinet in the design's drawing `Woodwork`, with its front, right
//! and iso views, its width, height and depth, and its boards. A3 landscape, mm.

use crate::select;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The drawing every sheet goes into.
pub const DRAWING: &str = "Woodwork";

/// Commands with their arguments, and the name the id in each reply is kept under ("" for
/// none). An argument `"$name"` is the id kept under that name earlier (`fill`).
pub type Steps = Vec<(&'static str, Value, &'static str)>;

/// Commands a sheet takes: the sheet, three views, three dimensions, four notes, and the old
/// sheet's deletion. And those a run takes whatever it draws: reading the design (4), the
/// drawing (2), setting an old drawing's units, and `END`.
const PER_SHEET: usize = 12;
const PER_RUN: usize = 7 + END;
/// The commands after the last sheet: the fingerprints, a new drawing's empty first sheet, close.
const END: usize = 3;
/// Seconds into a run after which no new sheet is started, of the app's 30: reading the sample
/// kitchen took about 1 s here, drawing a sheet about 0.15 s, and CI is about three times
/// slower.
const STOP_S: u64 = 20;
/// Where the drawing notes each sheet's fingerprint, by sheet id (`drawing.properties.set`).
pub const KEY: &str = crate::edit::KEY;

/// Refuses a run in which not even one sheet fits in the app's 256 commands, after `turned`
/// boards measured one command each.
pub fn budget(turned: usize) -> Result<(), String> {
    let needed = PER_RUN + PER_SHEET + turned;
    match needed <= 256 {
        true => Ok(()),
        false => Err(format!(
            "{turned} open doors and drawers need about {needed} commands, more than the 256 a run may use: close them, or draw with Selected cabinet"
        )),
    }
}

/// Whether a run may start another sheet of `next` commands, `elapsed` seconds in (0 without
/// the clock) with `used` commands run: before `STOP_S`, and with room left for it and `END`.
pub fn fits(elapsed: u64, used: usize, next: usize) -> bool {
    elapsed < STOP_S && used + next + END <= 256
}

/// A sheet's fingerprint, FNV-1a over its steps (the component and occurrence ids in them): a
/// sheet with the same one would come out the same.
pub fn fingerprint(steps: &Steps) -> String {
    let text = serde_json::to_string(steps).unwrap_or_default();
    let hash = (text.bytes()).fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// The title block `block` (a definition from `drawing.open`'s reply) without its Material and
/// Mass cells, Finish across their row. A cell that reads either weighs the cabinet each time
/// the drawing is opened: about 0.6 s a sheet on the sample kitchen, 2.7 ms for all without.
/// The board list names each board's material.
pub fn unweighed(block: &Value) -> Value {
    let mut block = block.clone();
    if let Some(cells) = block["cells"].as_array_mut() {
        cells.retain(|c| c["name"] != "material" && c["name"] != "mass");
        for cell in cells.iter_mut().filter(|c| c["name"] == "finish") {
            cell["rect_mm"] = json!([-180.0, 24.0, 0.0, 34.0]);
        }
    }
    block
}

/// Whether the drawing (`drawing.open`'s reply) has a sheet `name` noted with `fingerprint`.
pub fn up_to_date(drawing: &Value, name: &str, fingerprint: &str) -> bool {
    let sheets = drawing["sheets"].as_array().into_iter().flatten();
    sheets.into_iter().any(|s| {
        let noted = s["id"].as_str().map(|id| &drawing["properties"][KEY][id]);
        s["name"] == name && noted.is_some_and(|f| f == fingerprint)
    })
}

/// Every cabinet in the design, by its `<name> Side L` body. `measure` is `inspect.measure`'s
/// reply.
pub fn cabinets(measure: &Value) -> Vec<String> {
    let bodies = measure["bodies"].as_array().into_iter().flatten();
    let names = bodies.filter_map(|b| b["name"].as_str()?.strip_suffix(" Side L"));
    names.map(String::from).collect()
}

/// The cabinet's component id and the occurrences it is placed through, from the top, from
/// `component.list`'s compact reply.
// ponytail: the first copy of a component that stands in the design more than once.
fn path(tree: &Value, name: &str) -> Result<(u64, Vec<u64>), String> {
    let components = tree["components"].as_array().into_iter().flatten();
    let component = |id: u64| components.clone().find(|c| c["id"] == id);
    let found =
        (components.clone().find(|c| c["name"] == name)).ok_or(format!("no component `{name}`"))?;
    let id = found["id"].as_u64().unwrap_or(0);
    let (mut path, mut at) = (Vec::new(), found);
    while let Some(occurrence) = at["occurrences"].get(0) {
        path.insert(0, occurrence["id"].as_u64().unwrap_or(0));
        at = component(occurrence["parent"].as_u64().unwrap_or(0)).ok_or("broken tree")?;
    }
    Ok((id, path))
}

/// The joints that open the cabinet's doors and drawers, from `joint.list`'s reply: closed in
/// the views.
fn fronts(joints: &Value, measure: &Value, name: &str) -> Vec<Value> {
    let all = joints["joints"].as_array().into_iter().flatten();
    let names = all.filter_map(|j| j["name"].as_str());
    let moving = |j: &&str| {
        [" hinge", " fold", " slide"]
            .iter()
            .any(|end| j.ends_with(end))
    };
    let ours = names
        .filter(moving)
        .filter(|j| select::owner(measure, j) == Some(name));
    ours.map(|j| json!({"joint": j, "values": [0]})).collect()
}

/// A body's world box, min and max.
type Box3 = [[f64; 3]; 2];

fn bbox(measure: &Value, name: &str) -> Option<Box3> {
    let bodies = measure["bodies"].as_array()?;
    let body = bodies.iter().find(|b| b["name"] == name)?;
    let at = |end: &str, i: usize| body["bbox"][end][i].as_f64();
    let corner = |end: &str| Some([at(end, 0)?, at(end, 1)?, at(end, 2)?]);
    Some([corner("min")?, corner("max")?])
}

/// How far a box reaches along a direction in the floor plan (a unit axis).
fn reach(b: &Box3, d: [f64; 2]) -> f64 {
    (0..2).map(|i| (b[0][i] * d[i]).max(b[1][i] * d[i])).sum()
}

/// The cabinet's width axis (Side L to Side R) and the way its front faces, as unit axes in the
/// floor plan; none for a corner cabinet, whose two sides stand square to each other.
fn facing(l: &Box3, r: &Box3) -> Option<([f64; 2], [f64; 2])> {
    let thin_x = |b: &Box3| b[1][0] - b[0][0] < b[1][1] - b[0][1];
    if thin_x(l) != thin_x(r) {
        return None;
    }
    let mid = |b: &Box3, i: usize| (b[0][i] + b[1][i]) / 2.0;
    let (dx, dy) = (mid(r, 0) - mid(l, 0), mid(r, 1) - mid(l, 1));
    let u = match dx.abs() > dy.abs() {
        true => [dx.signum(), 0.0],
        false => [0.0, dy.signum()],
    };
    Some((u, [u[1], -u[0]]))
}

/// The base view and the iso view that look at a cabinet facing `f` from its front and from
/// its front right.
fn views(f: [f64; 2]) -> (&'static str, &'static str) {
    match f.map(|v| v.round() as i32) {
        [-1, 0] => ("left", "iso_front_left"),
        [1, 0] => ("right", "iso_back_right"),
        [0, 1] => ("back", "iso_back_left"),
        _ => ("front", "iso"),
    }
}

/// The view an L-shaped or diagonal corner cabinet is drawn from, and its iso view: from the
/// side and the diagonal of its open corner, so that its fronts face the viewer. That corner
/// is where the end panel standing thin in X (the end of one leg) meets the line of the one
/// standing thin in Y (the other's).
fn corner_views(l: &Box3, r: &Box3) -> ([f64; 2], &'static str) {
    let thin_x = |b: &Box3| b[1][0] - b[0][0] < b[1][1] - b[0][1];
    let (ex, ey) = if thin_x(l) { (l, r) } else { (r, l) };
    let mid = |b: &Box3, i: usize| (b[0][i] + b[1][i]) / 2.0;
    let right = mid(ex, 0) > mid(ey, 0);
    let front = mid(ey, 1) < mid(ex, 1);
    let iso = match (front, right) {
        (true, true) => "iso",
        (true, false) => "iso_front_left",
        (false, true) => "iso_back_right",
        (false, false) => "iso_back_left",
    };
    ([0.0, if front { -1.0 } else { 1.0 }], iso)
}

/// The scales a sheet is drawn at, largest first, as 1:n: ISO 5455's and the 1:4, 1:8, 1:15
/// and 1:25 of shop drawings.
const SCALES: [u32; 10] = [1, 2, 4, 5, 8, 10, 15, 20, 25, 50];

/// Where the views go on A3 landscape (420 x 297). Front and right views: left of the board
/// list and the title block, with room for the dimensions left of and under them. The iso
/// view: right of them, under the board list and over the title block.
const VIEWS: [f64; 2] = [40.0, 40.0];
const TOP: f64 = 282.0;
const RIGHT: f64 = 225.0;
const ISO_RIGHT: f64 = 405.0;
const GAP: f64 = 20.0; // between views: a projected view stands 20 mm past its parent

/// The largest scale, 1:n, at which a drawing `size` mm big fits into `room` mm.
fn fit(size: [f64; 2], room: [f64; 2]) -> u32 {
    let fits = |n: &&u32| (0..2).all(|i| size[i] / **n as f64 <= room[i]);
    *SCALES
        .iter()
        .find(fits)
        .unwrap_or(&SCALES[SCALES.len() - 1])
}

/// The size of a `w` x `h` x `d` box's iso view, at 1:1.
fn iso_size(w: f64, h: f64, d: f64) -> [f64; 2] {
    [
        (w + d) * std::f64::consts::FRAC_1_SQRT_2,
        (w + d) * 0.4082 + h * 0.8165,
    ]
}

/// Points of a dimension: a corner of a body placed through `path`.
fn corner(p: [f64; 3], path: &[u64]) -> Value {
    json!({"point": p, "path": path})
}

/// The board list: a note per column (part, size, material, quantity) at the top right, the
/// boards as the cut list groups them. `measure` holds the cabinet's bodies only (and
/// `notched`, the sides `cutlist::notched` found), `local` the
/// sizes of its turned boards (`cutlist::cutlist`). Each note's text and the middle of its column.
fn board_list(measure: &Value, local: &Local, name: &str) -> Result<Vec<(String, f64)>, String> {
    let notched = json!({"_notched": measure["notched"]});
    let list = crate::cutlist::cutlist(measure, local, false, &notched)?;
    let mut columns = [
        vec!["Qty".to_string()],
        vec!["Part".into()],
        vec!["L x W x T".into()],
    ];
    let mut material = vec!["Material".to_string()];
    let rows = list["parts"].as_array().cloned().unwrap_or_default();
    let shown = |v: &Value| crate::pdf::shown(v);
    for row in &rows {
        let names = row["parts"].as_array().into_iter().flatten();
        let parts: Vec<&str> = names
            .filter_map(|n| n.as_str())
            .map(|n| n.strip_prefix(name).unwrap_or(n).trim_start())
            .collect();
        let mut text = parts.join(", ");
        if row["note"].is_string() {
            text += " (notched)";
        }
        let lines = crate::pdf::wrap(&text, SIZE, PART_ROOM);
        let size = format!(
            "{} x {} x {}",
            shown(&row["length"]),
            shown(&row["width"]),
            shown(&row["thickness"])
        );
        for (i, line) in lines.into_iter().enumerate() {
            let first = |text: String| if i == 0 { text } else { String::new() };
            columns[0].push(first(row["qty"].to_string()));
            columns[1].push(line);
            columns[2].push(first(size.clone()));
            material.push(first(row["material"].as_str().unwrap_or("").into()));
        }
    }
    // ponytail: what does not fit above the title block is left to the cut list.
    let room = ((TOP + 2.0 - LIST_BOTTOM) / PITCH) as usize;
    if material.len() > room {
        for column in columns.iter_mut().chain([&mut material]) {
            column.truncate(room - 1);
            column.push(String::new());
        }
        columns[1][room - 1] = "... the rest: see the cut list".into();
    }
    let mut x = LIST_LEFT;
    let mut notes = Vec::new();
    for column in columns.into_iter().chain([material]) {
        let width = (column.iter()).fold(0.0, |w: f64, l| w.max(crate::pdf::width(l, SIZE)));
        // without blank lines at the end, which would move the column's middle
        let text = column.join("\n").trim_end_matches('\n').to_string();
        notes.push((text, x + width / 2.0));
        x += width + 5.0;
    }
    Ok(notes)
}

const CAP: f64 = 3.0; // the board list's text height, mm
const SIZE: f64 = CAP / 0.7; // its font size, for its widths
const PITCH: f64 = CAP * 1.6; // from line to line
const LIST_LEFT: f64 = 235.0;
const LIST_BOTTOM: f64 = 65.0; // above the title block
const PART_ROOM: f64 = 75.0; // the part names' column

/// The sizes of turned boards, measured in their own component.
pub type Local = BTreeMap<String, [f64; 3]>;

/// The steps that draw one cabinet's sheet. `measure` is `inspect.measure`'s reply, `local` as
/// for `board_list`, `tree` `component.list`'s compact reply, `joints` `joint.list`'s. The
/// drawing's source id is `$source`.
pub fn sheet(
    measure: &Value,
    local: &Local,
    tree: &Value,
    joints: &Value,
    name: &str,
) -> Result<Steps, String> {
    let (component, path) = path(tree, name)?;
    let side =
        |s: &str| bbox(measure, &format!("{name} {s}")).ok_or(format!("`{name} {s}` is missing"));
    let (l, r) = (side("Side L")?, side("Side R")?);
    let back = bbox(measure, &format!("{name} Back"));
    let turn = facing(&l, &r);
    let (u, f, iso) = match turn {
        Some((u, f)) => (u, f, views(f).1),
        None => {
            let (f, iso) = corner_views(&l, &r);
            ([-f[1], f[0]], f, iso)
        }
    };
    let neg = |d: [f64; 2]| d.map(|v| -v);
    let parts = select::parts(measure, name);
    // The views take in its end panels, fillers and legs too.
    let panels = (parts.iter())
        .filter(|p| {
            p.get(name.len() + 1..).is_some_and(crate::panel::is_panel)
                || crate::fittings::is_leg(p)
        })
        .map(|p| bbox(measure, p));
    let all: Vec<Option<Box3>> = [Some(l), Some(r), back].into_iter().chain(panels).collect();
    let span = |d: [f64; 2]| {
        let boxes = all.iter().flatten();
        let most = |d: [f64; 2]| boxes.clone().map(|b| reach(b, d)).fold(f64::MIN, f64::max);
        most(d) + most(neg(d))
    };
    let z1 = l[1][2];
    // legs reach lower
    let z0 = all
        .iter()
        .flatten()
        .map(|b| b[0][2])
        .fold(l[0][2], f64::min);
    // ponytail: 20 mm in front of the carcass for the doors and drawer fronts, closed.
    let (w, h, d) = (span(u), z1 - z0, span(f) + 20.0);
    let bodies = measure["bodies"].as_array().into_iter().flatten();
    let own: Vec<&Value> = bodies
        .filter(|b| parts.iter().any(|p| b["name"] == p.as_str()))
        .collect();
    let own = json!({ "bodies": own, "notched": measure["notched"] });
    let notes = board_list(&own, local, name)?;
    let (view, room) = (views(f).0, [RIGHT - VIEWS[0] - GAP, TOP - VIEWS[1]]);
    // Front and right views side by side, in the middle of their room.
    let n = fit([w + d, h], room);
    let s = 1.0 / n as f64;
    let front = [
        VIEWS[0] + (room[0] - (w + d) * s) / 2.0 + w * s / 2.0,
        VIEWS[1] + room[1] / 2.0,
    ];
    // The iso view at its own scale, under the board list's longest column.
    // ponytail: a list that reaches the title block leaves it no room: it then gets 1:50.
    let rows = notes
        .iter()
        .map(|(t, _)| t.lines().count())
        .max()
        .unwrap_or(0);
    let (low, high) = (LIST_BOTTOM, TOP + 2.0 - rows as f64 * PITCH - GAP / 2.0);
    let iso_n = fit(iso_size(w, h, d), [ISO_RIGHT - LIST_LEFT, high - low]);
    let iso_at = [(LIST_LEFT + ISO_RIGHT) / 2.0, (low + high) / 2.0];
    let mut base = json!({"sheet": "$sheet", "scale": format!("1:{n}"),
                          "source": {"occurrences": [path]}});
    let closed = fronts(joints, measure, name);
    if !closed.is_empty() {
        base["pose"] = json!(closed);
    }
    let at = |base: &Value, view: &str, at: [f64; 2]| {
        let mut base = base.clone();
        base["view"] = json!(view);
        base["position_mm"] = json!(at);
        base
    };
    let mut iso_base = base.clone();
    iso_base["scale"] = json!(format!("1:{iso_n}"));
    let mut steps: Steps = vec![
        (
            "drawing.sheet.add",
            json!({"name": name, "documents": {"source": "$source", "component": component}}),
            "sheet",
        ),
        ("drawing.view.base", at(&base, view, front), "front"),
        (
            "drawing.view.projected",
            json!({"parent": "$front", "side": "right"}),
            "right",
        ),
        ("drawing.view.base", at(&iso_base, iso, iso_at), ""),
    ];
    if turn.is_some() {
        // Corners in the world, from the cabinet's own width and front: on Side L, Side R and
        // the back, so that each is a corner of one body. At the top: a notched side has no
        // front bottom corner.
        // A side with an end panel or a filler: with inset fronts its corners are the panel's
        // too, so the dimensions go to the panel's outer corners, the width over all. A filler
        // and its return together: each outer corner is one of the two boards'.
        let outer = |s: &str, side: Box3| {
            let (end, filler) = (
                format!("{name} End panel {s}"),
                format!("{name} Filler {s}"),
            );
            (parts.iter())
                .filter(|p| **p == end || **p == filler || p.starts_with(&format!("{filler} ")))
                .filter_map(|p| bbox(measure, p))
                .reduce(|a, b| {
                    [
                        [0, 1, 2].map(|i| a[0][i].min(b[0][i])),
                        [0, 1, 2].map(|i| a[1][i].max(b[1][i])),
                    ]
                })
                .unwrap_or(side)
        };
        let (l, r) = (outer("L", l), outer("R", r));
        let p = |along: f64, out: f64, z: f64| {
            [along * u[0] + out * f[0], along * u[1] + out * f[1], z]
        };
        let left = p(-reach(&l, neg(u)), reach(&l, f), l[1][2]);
        let left_rear = p(-reach(&l, neg(u)), -reach(&l, neg(f)), l[0][2]);
        let right = p(reach(&r, u), reach(&r, f), r[1][2]);
        let rear = match back {
            Some(b) if reach(&b, neg(f)) > reach(&r, neg(f)) + 0.01 => {
                p(reach(&b, u), -reach(&b, neg(f)), b[1][2])
            }
            _ => p(reach(&r, u), -reach(&r, neg(f)), r[1][2]),
        };
        let under = front[1] - h * s / 2.0 - 10.0;
        let dimension = |view: &str, a: [f64; 3], b: [f64; 3], dir: &str, line: [f64; 2]| {
            json!({"view": view, "decimals": 0, "line_through_mm": line, "kind": {"linear": {
                "a": corner(a, &path), "b": corner(b, &path), "direction": dir}}})
        };
        let beside = [front[0] + w * s / 2.0 + GAP + d * s / 2.0, under];
        steps.extend([
            (
                "drawing.dimension.add",
                dimension("$front", left, right, "horizontal", [front[0], under]),
                "",
            ),
            (
                "drawing.dimension.add",
                dimension(
                    "$front",
                    left_rear,
                    left,
                    "vertical",
                    [front[0] - w * s / 2.0 - 10.0, front[1]],
                ),
                "",
            ),
            (
                "drawing.dimension.add",
                dimension("$right", right, rear, "horizontal", beside),
                "",
            ),
        ]);
    }
    for (content, x) in notes {
        // A note's `at` is the middle of its text: the columns' first lines line up.
        let y = TOP + 2.0 - content.lines().count() as f64 * PITCH / 2.0;
        steps.push((
            "drawing.note.add",
            json!({"anchor": {"sheet": "$sheet"}, "at": [x, y], "content": content, "cap_mm": CAP}),
            "",
        ));
    }
    Ok(steps)
}

/// The sheets to delete at the end, from the drawing as it was opened (`drawing.inspect`'s
/// reply): the old sheets of the cabinets drawn again, and a new drawing's empty first sheet.
pub fn stale(drawing: &Value, names: &[String], created: bool) -> Vec<Value> {
    let sheets = drawing["sheets"].as_array().into_iter().flatten();
    let old = sheets.enumerate().filter(|(i, sheet)| {
        (created && *i == 0) || names.iter().any(|n| sheet["name"] == n.as_str())
    });
    old.map(|(_, sheet)| sheet["id"].clone()).collect()
}

/// `args` with every `"$name"` replaced by the id kept under that name.
pub fn fill(args: &Value, ids: &BTreeMap<String, Value>) -> Value {
    match args {
        Value::String(s) => match s.strip_prefix('$').and_then(|key| ids.get(key)) {
            Some(id) => id.clone(),
            None => args.clone(),
        },
        Value::Array(items) => items.iter().map(|v| fill(v, ids)).collect(),
        Value::Object(map) => {
            let filled = map.iter().map(|(k, v)| (k.clone(), fill(v, ids)));
            Value::Object(filled.collect())
        }
        _ => args.clone(),
    }
}

#[cfg(test)]
mod tests;
