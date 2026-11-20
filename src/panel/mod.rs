//! A finishing board on one end of a box cabinet: an end panel, or a filler with its return.
//! It is a body inside the cabinet's component, every size and position an
//! expression over the cabinet's parameters and anchored on its side (`±<Prefix>_width/2`), so
//! it follows a resize. The cabinet remembers it (`panels` beside the remembered values), and
//! Edit cabinet makes it again.

use crate::kit::{self, check, mm, num};
use crate::Commands;
use serde_json::{json, Value};

/// The part each kind makes, by `kind`, and how a message calls it.
const KINDS: [(&str, &str, &str); 2] = [
    ("end", "End panel", "an end panel"),
    ("filler", "Filler", "a filler"),
];

fn letter(side: &str) -> &'static str {
    if side == "left" {
        "L"
    } else {
        "R"
    }
}

/// The side a panel goes on: the `side` field, or on Auto by the picked board.
pub fn side(p: &Value, board: &str) -> Result<&'static str, String> {
    match (p["side"].as_str().unwrap_or("auto"), board) {
        ("left", _) | ("auto", "Side L") => Ok("left"),
        ("right", _) | ("auto", "Side R") => Ok("right"),
        ("auto", _) => {
            Err("pick the cabinet's Side L or Side R for the panel, or choose a side".into())
        }
        _ => Err("side must be auto, left or right".into()),
    }
}

/// The body a remembered panel `p` makes first: Edit makes it again only while that is there.
pub fn body(cabinet: &str, p: &Value) -> String {
    let kind = KINDS
        .into_iter()
        .find(|k| p["kind"] == k.0)
        .unwrap_or(KINDS[0]);
    let side = p["side"].as_str().unwrap_or_default();
    format!("{cabinet} {} {}", kind.1, letter(side))
}

/// The panel on that side of `cabinet`, if it has one: its first body and how a message calls
/// it. `parts` are the cabinet's bodies, `side` is left or right.
fn on(parts: &[String], cabinet: &str, side: &str) -> Option<(String, &'static str)> {
    (KINDS.into_iter())
        .map(|(_, part, called)| (format!("{cabinet} {part} {}", letter(side)), called))
        .find(|(body, _)| parts.contains(body))
}

/// Refuses to place a cabinet against a side of `cabinet` that carries a panel.
pub fn blocks(parts: &[String], cabinet: &str, place: &str) -> Result<(), String> {
    match on(parts, cabinet, place) {
        Some((body, called)) => Err(format!("`{cabinet}` has {called} on its {place}: place the new cabinet on its other side, or delete the folder `{body}` in the history first")),
        None => Ok(()),
    }
}

/// The commands that make the panel `p` asks for (its side settled, see [`side`]) on
/// `cabinet`, to run with the cabinet's component active; what the cabinet remembers of it;
/// and its bodies. `remembered` is what the cabinet's component remembers (our metadata key),
/// `parts` its bodies, `design` `document.inspect`'s reply.
pub fn build(
    p: &Value,
    cabinet: &str,
    remembered: &Value,
    parts: &[String],
    design: &Value,
) -> Result<(Commands, Value, Vec<String>), String> {
    let prefix = kit::prefix(cabinet);
    let value = |key: &str| {
        let id = format!("{prefix}_{key}");
        let all = design["params"].as_array().into_iter().flatten();
        let found = all.into_iter().find(|p| p["name"] == id.as_str());
        found.and_then(|p| p["value"].as_f64())
    };
    let Some(entry) = remembered["entry"].as_str() else {
        return Err(format!("`{cabinet}` does not remember how it was made: panels need a cabinet made by Woodwork 0.1.7 or later"));
    };
    check(
        entry == "cabinet",
        &format!("`{cabinet}` is a corner cabinet: panels are only made for box cabinets so far"),
    )?;
    let kind = p["kind"].as_str().unwrap_or("end");
    let (_, part, _) =
        (KINDS.into_iter().find(|k| k.0 == kind)).ok_or("kind must be end or filler")?;
    let side = p["side"].as_str().unwrap_or_default();
    check(
        matches!(side, "left" | "right"),
        "side must be left or right",
    )?;
    let part = format!("{part} {}", letter(side));
    let body = body(cabinet, p);
    // One panel a side: an end panel and a filler would stand in each other.
    if let Some((_, called)) = on(parts, cabinet, side) {
        return Err(format!("`{cabinet}` already has {called} on its {side}"));
    }
    let (unit, scale) = match p["units"].as_str().unwrap_or("mm") {
        "mm" => ("mm", 1.0),
        "in" => ("in", 25.4),
        _ => return Err("units must be mm or in".into()),
    };
    let width = num(p, "width", 50.0 / scale)?;
    let thickness = num(p, "thickness", 0.0)?;
    check(
        thickness == 0.0 || (3.0..=50.0).contains(&(thickness * scale)),
        "thickness must be 0 (the cabinet's panel thickness) or 3 to 50 mm",
    )?;

    let (ew, eh, ed) = (
        format!("{prefix}_width"),
        format!("{prefix}_height"),
        format!("{prefix}_depth"),
    );
    let (et, t) = match thickness == 0.0 {
        true => (format!("{prefix}_thickness"), value("thickness")),
        false => (format!("{thickness} {unit}"), Some(thickness * scale)),
    };
    let t = t.ok_or(format!("`{cabinet}` has no parameters"))?;
    let made = &remembered["params"];
    // A matched cabinet has its plinth from the cabinet it was matched to, not in its values.
    let plinth = made["plinth"].as_f64().or(value("plinth")).unwrap_or(0.0) > 0.0;
    let epl = match plinth {
        true => format!("{prefix}_plinth"),
        false => mm(0.0),
    };
    // Overlay doors and drawer fronts stand a reveal and a panel in front of the carcass: the
    // panel comes forward to their faces. Inset ones are flush with the carcass.
    let count = |key: &str| made[key].as_f64().unwrap_or(0.0);
    let overlay = (count("doors") > 0.0 || count("drawers") > 0.0) && made["fronts"] != "inset";
    let er = format!("{prefix}_reveal");
    let yf = match overlay {
        true => format!("(-{er} - {prefix}_thickness)"),
        false => mm(0.0),
    };
    // Each board: its name, x from the side outwards and its size, y and depth, z and height;
    // y from the carcass front, z from the floor.
    let mut boards = Vec::new();
    match kind {
        "end" => boards.push((
            part.clone(),
            [mm(0.0), et.clone()],
            [yf.clone(), format!("{ed} - {yf}")],
            [mm(0.0), eh.clone()],
        )),
        _ => {
            check(
                (10.0..=1000.0).contains(&(width * scale)),
                "the filler's width must be 10 to 1000 mm",
            )?;
            check(
                width * scale > t,
                "the filler must be wider than its thickness",
            )?;
            let fw = format!("{width} {unit}");
            // As high as the fronts, or the carcass above the plinth.
            let z = match overlay {
                true => [format!("{epl} + {er}"), format!("{eh} - {epl} - 2*{er}")],
                false => [epl.clone(), format!("{eh} - {epl}")],
            };
            boards.push((
                part.clone(),
                [mm(0.0), fw.clone()],
                [yf.clone(), et.clone()],
                z.clone(),
            ));
            // Behind its outer edge, back to the wall.
            boards.push((
                format!("{part} return"),
                [format!("{fw} - {et}"), et.clone()],
                [format!("{yf} + {et}"), format!("{ed} - {yf} - {et}")],
                z,
            ));
        }
    }
    let mut commands: Commands = Vec::new();
    let mut bodies = Vec::new();
    for (part, [x, sx], [y, sy], [z, sz]) in boards {
        let x = match side {
            "right" => format!("{ew}/2 + {x}"),
            _ => format!("-{ew}/2 - ({x}) - ({sx})"),
        };
        let name = format!("{cabinet} {part}");
        commands.push((
            "solid.box",
            json!({
                "length": sx, "width": sy, "height": sz,
                "corner": [x, format!("{y} - {ed}/2"), z],
                "name": name, "body_name": name,
            }),
        ));
        bodies.push(name);
    }
    commands.push(("timeline.group", json!({"name": body, "features": bodies})));
    let record = json!({
        "kind": kind, "side": side, "units": unit, "width": width, "thickness": thickness,
    });
    Ok((commands, record, bodies))
}

#[cfg(test)]
mod tests;
