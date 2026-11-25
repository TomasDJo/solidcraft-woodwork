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
    "drawer_thickness",
    "drawer_bottom",
    "runner_depth",
    "runner_play",
    "cleat_height",
    "cleat_thickness",
    "wall_rail",
    "wall_rail_return",
    // joinery's (`joint`)
    "joint_spacing",
    "joint_edge",
    "dowel_d",
    "dowel_length",
    "dado_depth",
];

/// The Edit fields only a box cabinet takes, and only a corner cabinet: `merge` refuses them
/// for the other kind and the dialog hook hides them. Shrink them as the two kinds catch up.
const BOX_ONLY: [&str; 11] = [
    "kind",
    "joints_inner",
    "dividers",
    "doors",
    "drawers",
    "drawers_height",
    "drawers_at",
    "door_hinge",
    "back",
    "slide_length",
    "slide_type",
];
const CORNER_ONLY: [&str; 2] = ["corner_doors", "shape"];
/// The Edit fields both kinds of cabinet take.
const CABINETS: [&str; 14] = [
    "joints",
    "shelves",
    "handles",
    "handle_position",
    "plinth",
    "plinth_type",
    "plinth_setback",
    "reveal",
    "fronts",
    "wall_mount",
    "hardware_models",
    "hinge_angle",
    "shelf_pins",
    "labels",
];

/// The dialog hook's reply for Edit: the fields the selected cabinet does not take, hidden.
/// A corner cabinet has a `<Prefix>_return` parameter. Nothing picked, or
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
    let hide: &[&str] = if corner { &BOX_ONLY } else { &CORNER_ONLY };
    let reply: Map<String, Value> = (hide.iter())
        .map(|key| (key.to_string(), json!({"visible": false})))
        .collect();
    Value::Object(reply)
}

/// The Edit dialog's fields as the cabinet is now (`merge` with no changes: what it remembers,
/// its sizes from the design): the hook answers each as the field's `default` (#128), so the
/// conditional fields show what applies and a field left as shown changes nothing. A corner's
/// doors are `corner_doors`; the yes/no choices and the plinth are the dialog's words.
pub fn shown(entry: &str, params: &Value) -> Map<String, Value> {
    let mut shown = Map::new();
    let corner = entry == "corner";
    let choices = [
        "door_hinge",
        "drawers_at",
        "fronts",
        "handles",
        "handle_position",
        "plinth_type",
        "wall_mount",
        "back",
        "kind",
        "shape",
        "joints",
        "joints_inner",
        "hinge_angle",
        "slide_type",
        "slide_length",
        "hardware_models",
    ];
    let numbers = [
        "shelves",
        "dividers",
        "doors",
        "drawers",
        "drawer_height",
        "reveal",
        "plinth_setback",
    ];
    for key in choices.iter().chain(&numbers) {
        let field = match *key {
            "doors" if corner => "corner_doors",
            // Edit's key, so its field sorts right after Drawers (the dialog sorts optional
            // fields by key)
            "drawer_height" => "drawers_height",
            k => k,
        };
        if let Some(v) = params.get(*key).filter(|v| v.is_string() || v.is_number()) {
            shown.insert(field.into(), v.clone());
        }
    }
    for key in ["shelf_pins", "labels"] {
        let yes = params[key].as_bool().unwrap_or(false);
        shown.insert(key.into(), json!(if yes { "yes" } else { "no" }));
    }
    let plinth = params["plinth"].as_f64().unwrap_or(0.0) > 0.0;
    shown.insert("plinth".into(), json!(if plinth { "yes" } else { "no" }));
    shown.entry("slide_length").or_insert(json!("auto"));
    shown
}

/// The name of the file in our data that keeps what Edit shows for the cabinet `name`.
pub fn blob(name: &str) -> String {
    format!("edit/{}.json", kit::prefix(name))
}

/// The Edit values `make` kept for a cabinet, without the sizes that are design parameters (drawer
/// height, reveal, plinth set-back): they may have changed in Parameters since, and the hook
/// does not read the design's parameters (10M fuel), so they stay empty, which is unchanged.
pub fn without_sizes(shown: &Map<String, Value>) -> Map<String, Value> {
    let mut shown = shown.clone();
    for key in ["drawers_height", "reveal", "plinth_setback"] {
        shown.remove(key);
    }
    shown
}

/// Whether a cabinet made by `entry` (`cabinet` or `corner`) takes the Edit field `key`.
pub fn takes(entry: &str, key: &str) -> bool {
    CABINETS.contains(&key)
        || match entry {
            "corner" => CORNER_ONLY.contains(&key),
            _ => BOX_ONLY.contains(&key),
        }
}

/// The Edit fields a run changes: filled in, not Keep, not the selection.
pub fn asked(changes: &Value) -> Vec<String> {
    (changes.as_object().into_iter().flatten())
        .filter(|(k, v)| !v.is_null() && *v != "keep" && *k != "selection")
        .map(|(k, _)| k.clone())
        .collect()
}

/// The Edit dialog's patch for the picked cabinets (each its entry and `shown` values): a
/// field one of them does not take is hidden; one they all show the same gets that as its default;
/// one that differs stays empty (unchanged), with a hint.
pub fn fields(cabinets: &[(String, Map<String, Value>)]) -> Value {
    let mut reply = Map::new();
    for key in CABINETS.iter().chain(&BOX_ONLY).chain(&CORNER_ONLY) {
        // A field shows only when every picked cabinet takes it: a box and a corner together
        // show only the fields both take (#128: fewer fields).
        let takers: Vec<_> = cabinets.iter().filter(|(e, _)| takes(e, key)).collect();
        if takers.len() < cabinets.len() {
            reply.insert(key.to_string(), json!({"visible": false}));
            continue;
        }
        let mut values: Vec<&Value> = Vec::new();
        for (_, shown) in &takers {
            let v = shown.get(*key).unwrap_or(&Value::Null);
            if !values
                .iter()
                .any(|w| same(w, v) && w.is_null() == v.is_null())
            {
                values.push(v);
            }
        }
        match values.as_slice() {
            [v] if !v.is_null() => {
                reply.insert(key.to_string(), json!({"default": v}));
            }
            [_] => {}
            _ => {
                let hint = format!("differs: {} values", values.len());
                reply.insert(key.to_string(), json!({"hint": hint}));
            }
        }
    }
    Value::Object(reply)
}

/// Whether two values are the same, a number however it is written (`2` and `2.0`), and a
/// value left out the same as no, 0, none or auto (what a builder reads for it).
pub fn same(a: &Value, b: &Value) -> bool {
    let empty = |v: &Value| {
        v.is_null() || v == &json!(false) || v.as_f64() == Some(0.0) || v == "none" || v == "auto"
    };
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-6,
        _ => a == b || (empty(a) && empty(b)),
    }
}

/// Whether merged `params` build the same cabinet as `base` (`merge` with no changes): an Edit
/// that changes nothing is not built again.
pub fn unchanged(params: &Value, base: &Value) -> bool {
    let (Some(p), Some(b)) = (params.as_object(), base.as_object()) else {
        return false;
    };
    let keys = p.keys().chain(b.keys()).filter(|k| *k != "_existing");
    keys.into_iter().all(|k| {
        same(
            p.get(k).unwrap_or(&Value::Null),
            b.get(k).unwrap_or(&Value::Null),
        )
    })
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
                // Edit's `drawers_height` (named to sort after Drawers) is `drawer_height`
                let to = if k == "drawers_height" {
                    "drawer_height"
                } else {
                    k
                };
                (to, value.clone())
            }
            "shape" | "corner_doors" => {
                only(&["corner"], "corner cabinet")?;
                (key.strip_prefix("corner_").unwrap_or(key), value.clone())
            }
            _ => return Err(format!("`{key}` cannot be edited")),
        };
        params.insert(to.into(), value);
    }
    // Beside doors a drawer height of 0 was kept as 150 mm: without the doors, back to 0 (all
    // alike) unless the edit gives one.
    if was_mixed && !mixed(&params) && changes["drawers_height"].is_null() {
        params.insert("drawer_height".into(), json!(0));
    }
    // Holes drilled by the Joinery function of 0.2.0 and before (`joinery` beside the values):
    // the same kind of joint at build, unless the edit or the cabinet chose one.
    let method = |p: &Map<String, Value>| {
        p.get("joints").and_then(Value::as_str).unwrap_or("none") != "none"
    };
    let asked = changes["joints"].as_str().is_some_and(|m| m != "keep");
    if let Some(kind) = remembered["joinery"]
        .as_str()
        .filter(|_| !method(&params) && !asked)
    {
        let joints = if kind == "dowel" {
            "dowels"
        } else {
            "confirmat"
        };
        params.insert("joints".into(), json!(joints));
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
