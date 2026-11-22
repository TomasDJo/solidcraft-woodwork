//! Changing a cabinet after it is made. Its dialog values are remembered on its component, so
//! Edit can build it again with some of them changed, in the same place.

use crate::kit;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// Our key in a component's metadata.
pub const KEY: &str = "io.github.tomasdjo.woodwork";

/// The sizes that are design parameters: on an edit they are read from the design, where the
/// user may have changed them, not from what was remembered.
/// `wall_rail` is here so that `Kit` knows the cabinet has it (see `cabinet::build`).
const SIZES: [&str; 23] = [
    "width",
    "height",
    "depth",
    "thickness",
    "back_thickness",
    "plinth",
    "plinth_setback",
    "return",
    "reveal",
    "drawer_height",
    "cleat_height",
    "cleat_thickness",
    "wall_rail",
    "wall_rail_return",
    // a frame's
    "leg",
    "apron_height",
    "apron_thickness",
    "inset",
    "stretcher_height",
    // joinery's (`joint`)
    "joint_spacing",
    "joint_edge",
    "dowel_d",
    "dowel_length",
];

/// The Edit fields only a box cabinet takes, and only a corner cabinet: `merge` refuses them
/// for the other kind and the dialog hook hides them. Shrink them as the two kinds catch up.
const BOX_ONLY: [&str; 9] = [
    "kind",
    "joints",
    "dividers",
    "doors",
    "drawers",
    "drawer_height",
    "drawers_at",
    "door_hinge",
    "back",
];
const CORNER_ONLY: [&str; 2] = ["corner_doors", "shape"];
/// The Edit fields both kinds of cabinet take, and those only a frame takes.
const CABINETS: [&str; 11] = [
    "shelves",
    "handles",
    "handle_position",
    "plinth",
    "plinth_type",
    "plinth_setback",
    "reveal",
    "fronts",
    "wall_mount",
    "shelf_pins",
    "labels",
];
const FRAME_ONLY: [&str; 3] = ["stretchers", "frame_legs", "joint"];

/// The dialog hook's reply for Edit: the fields the selected cabinet does not take, hidden.
/// A corner cabinet has a `<Prefix>_return` parameter (as Joinery tells it), a frame its
/// `<Name> Leg front left` body and only its own fields. Nothing picked, or
/// not a cabinet: everything shown. Hidden only from view, so a run is the same.
/// `design` is `document.inspect`'s reply with body names.
pub fn hidden(values: &Value, design: &Value) -> Value {
    let picked = crate::select::picked(values, design);
    let Some(cabinet) = picked.first().and_then(|b| crate::select::owner(design, b)) else {
        return json!({});
    };
    let id = format!("{}_return", kit::prefix(cabinet));
    let all = design["params"].as_array().into_iter().flatten();
    let corner = all.into_iter().any(|p| p["name"] == id.as_str());
    let leg = format!("{cabinet}{}", crate::frame::MARK);
    let bodies = design["bodies"].as_array().into_iter().flatten();
    let frame = bodies
        .into_iter()
        .any(|b| b.as_str().or(b["name"].as_str()) == Some(&leg));
    let hide: Vec<&str> = match (frame, corner) {
        (true, _) => [&CABINETS[..], &BOX_ONLY, &CORNER_ONLY].concat(),
        (_, true) => [&BOX_ONLY[..], &FRAME_ONLY].concat(),
        _ => [&CORNER_ONLY[..], &FRAME_ONLY].concat(),
    };
    let reply: Map<String, Value> = (hide.iter())
        .map(|key| (key.to_string(), json!({"visible": false})))
        .collect();
    Value::Object(reply)
}

/// What a cabinet's component remembers, as `component.metadata.set` takes it: the entry that
/// made it and the values it was made with. Where it was placed and what it was matched to
/// are not part of that.
pub fn remember(entry: &str, params: &Value, name: &str) -> Value {
    let mut params = params.as_object().cloned().unwrap_or_default();
    for key in ["place", "match", "selection", "_match", "_existing"] {
        params.remove(key);
    }
    params.insert("name".into(), json!(name));
    json!({ KEY: {"entry": entry, "version": env!("CARGO_PKG_VERSION"), "params": params} })
}

/// The entry and the values to build a cabinet again with: what it `remembered`, its sizes as
/// they are in the design now, and the fields of the Edit dialog that were filled in.
/// `design` is `document.inspect`'s reply. `_existing` in the result tells `Kit` which
/// parameters the design already has, in mm.
pub fn merge(
    remembered: &Value,
    changes: &Value,
    design: &Value,
) -> Result<(String, Value), String> {
    let (Some(entry), Some(params)) = (
        remembered["entry"].as_str(),
        remembered["params"].as_object(),
    ) else {
        return Err("it does not remember how it was made: only a cabinet made by Woodwork 0.1.7 or later can be edited".into());
    };
    let mut params = params.clone();
    let scale = if params.get("units") == Some(&json!("in")) {
        25.4
    } else {
        1.0
    };
    let prefix = kit::prefix(params["name"].as_str().unwrap_or_default());
    let mut existing = Map::new();
    for key in SIZES {
        let id = format!("{prefix}_{key}");
        let all = design["params"].as_array().into_iter().flatten();
        let found = all.into_iter().find(|p| p["name"] == id.as_str());
        if let Some(mm) = found.and_then(|p| p["value"].as_f64()) {
            // the reveal is a mm field whatever the cabinet's unit
            let scale = if key == "reveal" { 1.0 } else { scale };
            params.insert(key.into(), json!((mm / scale * 1e6).round() / 1e6));
            existing.insert(key.into(), json!(mm));
        }
    }
    // The fastener counts only tell `Kit` that the cabinet has them (`joint::COUNTS`).
    for key in crate::joint::COUNTS {
        let id = format!("{prefix}_{key}");
        let all = design["params"].as_array().into_iter().flatten();
        let found = all.into_iter().find(|p| p["name"] == id.as_str());
        if let Some(v) = found.and_then(|p| p["value"].as_f64()) {
            existing.insert(key.into(), json!(v));
        }
    }
    let mixed = |p: &Map<String, Value>| {
        let count = |key: &str| p.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        count("drawers") > 0.0 && count("doors") > 0.0
    };
    let was_mixed = mixed(&params);
    for (key, value) in changes.as_object().into_iter().flatten() {
        // `selection` is the dialog's field: which cabinet, not a change to it
        if value.is_null() || value == "keep" || key == "selection" {
            continue;
        }
        // the entries a field is for, and how a message calls them
        let only = |entries: &[&str], kind: &str| match entries.contains(&entry) {
            true => Ok(()),
            false => Err(format!("`{key}` is only for a {kind}")),
        };
        let cabinets = ["cabinet", "corner"];
        let (to, value) = match key.as_str() {
            "shelf_pins" | "labels" => {
                only(&cabinets, "cabinet")?;
                (key.as_str(), json!(value == "yes"))
            }
            k if CABINETS.contains(&k) => {
                only(&cabinets, "cabinet")?;
                (k, value.clone())
            }
            k if BOX_ONLY.contains(&k) => {
                only(&["cabinet"], "box cabinet")?;
                (k, value.clone())
            }
            "shape" | "corner_doors" => {
                only(&["corner"], "corner cabinet")?;
                (key.strip_prefix("corner_").unwrap_or(key), value.clone())
            }
            k if FRAME_ONLY.contains(&k) => {
                only(&["frame"], "frame")?;
                (if k == "frame_legs" { "legs" } else { k }, value.clone())
            }
            _ => return Err(format!("`{key}` cannot be edited")),
        };
        params.insert(to.into(), value);
    }
    // Beside doors a drawer height of 0 was kept as 150 mm: without the doors, back to 0 (all
    // alike) unless the edit gives one.
    if was_mixed && !mixed(&params) && changes["drawer_height"].is_null() {
        params.insert("drawer_height".into(), json!(0));
    }
    params.insert("_existing".into(), Value::Object(existing));
    Ok((entry.into(), Value::Object(params)))
}

/// Where a cabinet's component stands, from `component.list`'s reply: the component it is in,
/// its placement there as `occurrence.move` takes it in `to_matrix`, and whether it is grounded.
pub fn placement(tree: &Value, name: &str) -> Result<(Value, Value, bool), String> {
    let components = tree["components"].as_array().into_iter().flatten();
    let occurrence = components
        .filter(|c| c["name"] == name)
        .find_map(|c| c["occurrences"].get(0))
        .ok_or(format!("`{name}` has no component"))?;
    // The list gives the placement with its translation in the last row; the move wants it
    // in the last column.
    let at = |row: usize, column: usize| occurrence["transform"][column][row].clone();
    let rows: Vec<Vec<Value>> = (0..3).map(|r| (0..4).map(|c| at(r, c)).collect()).collect();
    let grounded = occurrence["grounded"].as_bool().unwrap_or(true);
    Ok((occurrence["parent"].clone(), json!(rows), grounded))
}

/// The links a cabinet is part of, from `component.list`'s and `joint.list`'s replies: each
/// joint called `<cabinet> link` with `name`'s occurrence on one side, and whether it is the
/// cabinet's own, the one that holds it to the cabinet it stands against (its second side).
pub fn links(tree: &Value, joints: &Value, name: &str) -> Vec<(String, bool)> {
    let components = tree["components"].as_array().into_iter().flatten();
    let id = components
        .filter(|c| c["name"] == name)
        .find_map(|c| c["occurrences"].get(0))
        .map(|o| &o["id"]);
    let all = joints["joints"].as_array().into_iter().flatten();
    all.filter(|j| Some(&j["a"]) == id || Some(&j["b"]) == id)
        .filter_map(|j| Some((j["name"].as_str()?, Some(&j["b"]) == id)))
        .filter(|(joint, _)| joint.ends_with(" link"))
        .map(|(joint, own)| (joint.to_string(), own))
        .collect()
}

/// The cabinets that are held to `name` by a link.
pub fn hanging(tree: &Value, joints: &Value, name: &str) -> Vec<String> {
    let others = links(tree, joints, name).into_iter().filter(|l| !l.1);
    others
        .filter_map(|l| l.0.strip_suffix(" link").map(String::from))
        .collect()
}

/// Which of `bodies` have which material, from `inspect.measure`'s reply: so boards that got
/// another material than the rest keep it.
pub fn materials(measure: &Value, bodies: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut by_material: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for body in measure["bodies"].as_array().into_iter().flatten() {
        let (Some(name), Some(material)) = (body["name"].as_str(), body["material"].as_str())
        else {
            continue;
        };
        if material != "Default" && bodies.iter().any(|b| b == name) {
            by_material
                .entry(material.into())
                .or_default()
                .push(name.into());
        }
    }
    by_material
}

#[cfg(test)]
mod tests;
