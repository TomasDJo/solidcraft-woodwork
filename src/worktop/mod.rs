//! A worktop over a straight run of the selected cabinets: one board in a component of its
//! own, whose length, depth and height are expressions over the cabinets' parameters.
//!
//! The component's origin is the point of the run that holds still when the cabinets change
//! size (see `anchor`), on its floor; the board is built in its coordinates (X along the run,
//! front at -Y, Z up), then `lib.rs` moves and turns the component there (`at`, `turn` in the
//! summary).

use crate::kit::{check, Kit};
use crate::select::{self, owner};
use crate::Commands;
use serde_json::{json, Value};

/// The end of the comment on a worktop's length parameter, `<Name>: length along the run`:
/// the cut list knows a worktop by it, whatever its name.
pub const ALONG: &str = "length along the run";
const TOLERANCE: f64 = 0.5; // mm

/// A cabinet of the run, in the run's own axis: name, where it starts and ends along the run,
/// how far back it reaches, its floor and top, and which size parameter lies along the run.
struct Part {
    name: String,
    start: f64,
    end: f64,
    back: f64,
    front: f64,
    floor: f64,
    top: f64,
    size: &'static str,
    corner: bool,
}

/// The run: its cabinets left to right, the way along it, and the way to the back.
struct Run {
    parts: Vec<Part>,
    along: [f64; 2],
    back: [f64; 2],
}

fn refuse(why: String) -> String {
    format!("the selected cabinets are not one straight run: {why}")
}

/// The cabinets of the picked bodies, as one straight run, or why they are not.
fn find(picked: &[String], measure: &Value) -> Result<Run, String> {
    let mut names: Vec<&str> = vec![];
    for body in picked {
        let name = owner(measure, body).ok_or(format!("`{body}` is not part of a cabinet"))?;
        if !names.contains(&name) {
            names.push(name);
        }
    }
    check(
        !names.is_empty(),
        "select the cabinets for the worktop first",
    )?;
    let cabinets =
        (names.iter().map(|name| select::named(measure, name))).collect::<Result<Vec<_>, _>>()?;
    let out = |c: &select::Cabinet, side: &str| {
        let (dir, size) = c.toward(side);
        ([dir[0], dir[1]], size)
    };
    let is_corner = |c: &select::Cabinet| {
        let (l, r) = (out(c, "left").0, out(c, "right").0);
        l[0] * r[0] + l[1] * r[1] == 0.0
    };
    // The way along the run: out of a box cabinet's Side R; a corner cabinet alone runs
    // along its back wall leg, out of whichever side ends that leg.
    let along = match cabinets.iter().find(|c| !is_corner(c)) {
        Some(c) => out(c, "right").0,
        None => match out(&cabinets[0], "right") {
            (dir, "width") => dir,
            _ => out(&cabinets[0], "left").0.map(|v| -v),
        },
    };
    let back = [-along[1], along[0]];
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let mut parts = vec![];
    for c in &cabinets {
        // The side that faces along the run names its size: a corner cabinet's leg.
        let size = ["right", "left"]
            .into_iter()
            .map(|side| out(c, side))
            .find(|(dir, _)| dot(*dir, along).abs() == 1.0)
            .map(|(_, size)| size)
            .ok_or(refuse(format!("`{}` faces another way", c.name)))?;
        check(
            is_corner(c) || out(c, "right").0 == along,
            &refuse(format!("`{}` faces another way", c.name)),
        )?;
        // Its footprint: the sides and backs, which a door or a plinth never passes.
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        let bodies = measure["bodies"].as_array().into_iter().flatten();
        for body in bodies.filter(|b| {
            ["Side L", "Side R", "Back", "Back return"]
                .iter()
                .any(|part| b["name"] == format!("{} {part}", c.name).as_str())
        }) {
            for i in 0..2 {
                lo[i] = lo[i].min(body["bbox"]["min"][i].as_f64().unwrap_or(f64::NAN));
                hi[i] = hi[i].max(body["bbox"]["max"][i].as_f64().unwrap_or(f64::NAN));
            }
        }
        let ends = |dir: [f64; 2]| {
            let (a, b) = (dot(lo, dir), dot(hi, dir));
            (a.min(b), a.max(b))
        };
        let (start, end) = ends(along);
        parts.push(Part {
            name: c.name.clone(),
            start,
            end,
            back: ends(back).1,
            front: ends(back).0,
            floor: c.min[2],
            top: c.max[2],
            size,
            corner: is_corner(c),
        });
    }
    parts.sort_by(|a, b| a.start.total_cmp(&b.start));
    let first = &parts[0];
    for (i, part) in parts.iter().enumerate() {
        let near = |a: f64, b: f64| (a - b).abs() <= TOLERANCE;
        let (one, other) = (&first.name, &part.name);
        check(
            near(part.floor, first.floor),
            &refuse(format!(
                "`{other}` does not stand on the same floor as `{one}`"
            )),
        )?;
        check(
            near(part.top, first.top),
            &refuse(format!("`{other}` is not as high as `{one}`")),
        )?;
        check(
            near(part.back, first.back),
            &refuse(format!("the backs of `{one}` and `{other}` are not flush")),
        )?;
        check(
            !part.corner || i == 0 || i == parts.len() - 1,
            &refuse(format!("the corner cabinet `{other}` is not at an end")),
        )?;
    }
    for pair in parts.windows(2) {
        let (one, other) = (&pair[0].name, &pair[1].name);
        check(
            pair[1].start > pair[0].end - TOLERANCE,
            &refuse(format!("`{one}` and `{other}` overlap")),
        )?;
        check(
            pair[1].start < pair[0].end + TOLERANCE,
            &refuse(format!("there is a gap between `{one}` and `{other}`")),
        )?;
    }
    Ok(Run { parts, along, back })
}

/// The worktop's commands, built in its own component, and its summary, with `at` (where
/// the component's origin goes in the world, mm) and `turn` (degrees about Z) for `lib.rs`.
/// `picked` are the selected bodies, `measure` is `inspect.measure`'s reply and `design`
/// `document.inspect`'s (for the parameters' values, in mm).
pub fn build(
    p: &Value,
    picked: &[String],
    measure: &Value,
    design: &Value,
) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
    let name = kit.name.clone();
    // Body names are the data model: a cabinet must not take the worktop for one of its parts.
    if let Some(cabinet) = owner(measure, &name) {
        return Err(format!(
            "name: `{name}` would read as a part of the cabinet `{cabinet}`; choose another"
        ));
    }
    let size = |key: &str, default: f64| -> Result<f64, String> {
        let v = kit.size(p, key, default)?;
        check(v >= 0.0, &format!("`{key}` must not be negative"))?;
        Ok(v)
    };
    let thickness = size("thickness", 38.0)?;
    check(thickness > 0.0, "thickness must be more than 0")?;
    let (front, left, right) = (
        size("front_overhang", 40.0)?,
        size("left_overhang", 0.0)?,
        size("right_overhang", 0.0)?,
    );
    let run = find(picked, measure)?;
    let value = |id: &str| -> Result<f64, String> {
        let all = design["params"].as_array().into_iter().flatten();
        let found = all.into_iter().find(|p| p["name"] == id);
        found
            .and_then(|p| p["value"].as_f64())
            .ok_or(format!("the design has no parameter `{id}`"))
    };
    let prefix = |part: &Part| crate::kit::prefix(&part.name);

    // Along the run: the cabinets' sizes and the two end overhangs.
    let mut terms = vec![];
    let mut length = left + right;
    for part in &run.parts {
        let id = format!("{}_{}", prefix(part), part.size);
        length += value(&id)?;
        terms.push(id);
    }
    for overhang in [left, right].into_iter().filter(|v| *v > 0.0) {
        terms.push(kit.length(overhang));
    }
    // Across: the deepest cabinet and the front overhang.
    // ponytail: the deepest by its value now; a cabinet made deeper later than that one is not followed.
    let mut deepest = (String::new(), f64::NEG_INFINITY);
    for part in &run.parts {
        let id = format!("{}_depth", prefix(part));
        let v = value(&id)?;
        if v > deepest.1 {
            deepest = (id, v);
        }
    }
    let depth = deepest.1 + front;
    // Where it is held: the cabinet of the run that holds still when sizes change, and the
    // point of it that does: a grounded cabinet's middle (it grows to both sides), or the
    // side by which one is held to a cabinet outside the run (`_held`, from lib.rs: per
    // cabinet `grounded` and its `link`). Without either, the run's back left corner.
    let names: Vec<&str> = run.parts.iter().map(|part| part.name.as_str()).collect();
    let (i, fixed) = (run.parts.iter().enumerate())
        .find_map(|(i, part)| {
            let held = &p["_held"][&part.name];
            if held["grounded"] == true {
                return Some((i, "middle"));
            }
            let to = held["link"]["to"].as_str()?;
            let fixed = match held["link"]["place"].as_str() {
                _ if names.contains(&to) => return None,
                Some("right") => "left",
                Some("left") => "right",
                _ => "middle",
            };
            Some((i, fixed))
        })
        .unwrap_or((0, "left"));
    let anchor = &run.parts[i];
    let along = format!("{}_{}", prefix(anchor), anchor.size);
    let across = match (anchor.corner, anchor.size) {
        (false, _) => format!("{}_depth", prefix(anchor)),
        (true, "width") => format!("{}_return", prefix(anchor)),
        (true, _) => format!("{}_width", prefix(anchor)),
    };
    let mut lefts = match fixed {
        "middle" => vec![format!("{along} / 2")],
        "right" => vec![along.clone()],
        _ => vec![],
    };
    lefts.extend(
        run.parts[..i]
            .iter()
            .map(|part| format!("{}_{}", prefix(part), part.size)),
    );
    if left > 0.0 {
        lefts.push(kit.length(left));
    }
    // The board's left end and back from the anchor.
    let x0 = match lefts.is_empty() {
        true => kit.length(0.0),
        false => format!("-({})", lefts.join(" + ")),
    };
    let back = (fixed == "middle").then(|| format!("{across} / 2"));
    let height = format!("{}_height", prefix(anchor));
    value(&height)?;

    let add = |key: &str, expression: String, comment: &str| {
        let id = format!("{}_{key}", kit.prefix);
        kit.push((
            "parameters.add",
            json!({"name": id, "expression": expression, "comment": format!("{name}: {comment}")}),
        ));
        id
    };
    let pw = add("width", terms.join(" + "), ALONG);
    let mut deep = deepest.0.clone();
    if front > 0.0 {
        deep = format!("{deep} + {}", kit.length(front));
    }
    let pd = add("depth", deep, "depth, front overhang included");
    let y0 = match &back {
        Some(back) => format!("{back} - {pd}"),
        None => format!("-{pd}"),
    };
    let pt = kit.parameter("thickness", thickness, "thickness");
    kit.push((
        "solid.box",
        json!({
            "length": pw, "width": pd, "height": pt,
            "corner": [x0, y0, height],
            "name": name, "body_name": name,
        }),
    ));

    let (commands, _) = kit.finish(vec![], [0.0; 3]);
    // The component's origin: the anchor's point, on its floor.
    let a = anchor;
    let (x, y) = match fixed {
        "middle" => ((a.start + a.end) / 2.0, (a.back + a.front) / 2.0),
        "right" => (a.end, a.back),
        _ => (a.start, a.back),
    };
    let at: Vec<f64> = (0..2)
        .map(|i| run.along[i] * x + run.back[i] * y)
        .chain([a.floor])
        .collect();
    let turn = run.along[1].atan2(run.along[0]).to_degrees().round() as i32;
    let summary = json!({
        "name": name,
        "size": {"length": length, "depth": depth, "thickness": thickness},
        "bodies": [name],
        "cabinets": run.parts.iter().map(|part| &part.name).collect::<Vec<_>>(),
        "parameters": format!("{pw}, {pd}, {pt}"),
        "at": at,
        "turn": turn,
    });
    Ok((commands, summary))
}

#[cfg(test)]
mod tests;
