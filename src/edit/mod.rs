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
const SIZES: [&str; 24] = [
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
    "finger_width",
];

/// The Edit fields only a box cabinet takes, and only a corner cabinet: the dialog hook hides
/// them for the other kind and a run leaves them out. Shrink them as the two kinds catch up.
/// Edit has only what is per cabinet; what is design-wide (joinery, hardware, thickness,
/// reveal, the plinth's height and set-back) is Woodwork settings' and Apply settings'.
const BOX_ONLY: [&str; 8] = [
    "kind",
    "dividers",
    "doors",
    "drawers",
    "drawer_height",
    "drawers_at",
    "door_hinge",
    "back",
];
const CORNER_ONLY: [&str; 3] = ["corner_doors", "shape", "return"];
/// The Edit fields both kinds of cabinet take.
const CABINETS: [&str; 12] = [
    "width",
    "height",
    "depth",
    "shelves",
    "shelf_pins",
    "handles",
    "handle_position",
    "plinth",
    "plinth_type",
    "fronts",
    "wall_mount",
    "labels",
];
/// Edit's sizes, read from the cabinet's parameters (`<Prefix>_<key>`, in mm) by the hook.
pub const SHOWN_SIZES: [&str; 5] = ["width", "height", "depth", "return", "drawer_height"];

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
    ];
    let numbers = ["shelves", "dividers", "doors", "drawers", "drawer_height"];
    for key in choices.iter().chain(&numbers) {
        let field = match *key {
            "doors" if corner => "corner_doors",
            k => k,
        };
        if let Some(v) = params.get(*key).filter(|v| v.is_string() || v.is_number()) {
            shown.insert(field.into(), v.clone());
        }
    }
    // Cabinet's Shelf pin holes is a tick box, Labels yes or no (as in Woodwork settings)
    shown.insert("shelf_pins".into(), json!(params["shelf_pins"] == true));
    let labels = params["labels"] == true;
    shown.insert("labels".into(), json!(if labels { "yes" } else { "no" }));
    let plinth = params["plinth"].as_f64().unwrap_or(0.0) > 0.0;
    shown.insert("plinth".into(), json!(if plinth { "yes" } else { "no" }));
    shown
}

/// The name of the file in our data that keeps what Edit shows for the cabinet `name`.
pub fn blob(name: &str) -> String {
    format!("edit/{}.json", kit::prefix(name))
}

/// What Edit's file for the cabinet `name` keeps (`blob`): its `shown` values and its boards'
/// first words (Side, Top, Drawer...) from their names `bodies`, so the hook knows a board of
/// "Cabinet 2" (made before, no file of its own) is not one of "Cabinet"'s.
pub fn copy<'a>(
    entry: &str,
    params: &Value,
    name: &str,
    bodies: impl Iterator<Item = &'a str>,
) -> Value {
    let mut parts: Vec<&str> = Vec::new();
    for b in bodies {
        let rest = b.strip_prefix(name).and_then(|r| r.strip_prefix(' '));
        if let Some(w) = rest.and_then(|r| r.split(' ').next()) {
            if !parts.contains(&w) {
                parts.push(w);
            }
        }
    }
    let shown = shown(entry, params);
    json!({"entry": entry, "units": params["units"], "shown": shown, "parts": parts})
}

/// Whether a cabinet made by `entry` (`cabinet` or `corner`) takes the Edit field `key`.
/// A Woodwork setting that is not a field of the Cabinet dialog (joinery, hardware, the cut
/// list's): refused, not left out without a word, as Edit changes only what is per cabinet.
pub fn design_wide(entry: &str, params: &Value) -> Result<(), String> {
    let stored = |k: &str| {
        crate::settings::SECTIONS
            .iter()
            .any(|(_, keys)| keys.contains(&k))
    };
    let given = |v: &Value| !v.is_null() && v != "keep";
    let wide: Vec<&String> = (params.as_object().into_iter().flatten())
        .filter(|(k, v)| given(v) && !takes(entry, k) && stored(k))
        .map(|(k, _)| k)
        .collect();
    if wide.is_empty() {
        return Ok(());
    }
    // Woodwork settings' fields the Cabinet dialog has too (material, plinth, ...) come with
    // every Edit from the dialog, so only those it lacks are refused.
    let manifest: Value =
        serde_json::from_str(include_str!("../../plugin.json")).unwrap_or_default();
    let entries = manifest["runtime"]["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let cabinet = entries.iter().find(|e| e["name"] == "cabinet");
    let dialog =
        |k: &str| cabinet.is_some_and(|c| c["input_schema"]["properties"].get(k).is_some());
    match wide.into_iter().find(|k| !dialog(k)) {
        Some(k) => Err(format!(
            "`{k}` is a Woodwork setting, for the whole design: set it in Woodwork settings, then Apply settings to this cabinet"
        )),
        None => Ok(()),
    }
}

pub fn takes(entry: &str, key: &str) -> bool {
    CABINETS.contains(&key)
        || match entry {
            "corner" => CORNER_ONLY.contains(&key),
            _ => BOX_ONLY.contains(&key),
        }
}

/// The Edit dialog's patch for one cabinet made by `entry`, as it is: each field it takes starts
/// at its value (`shown`, the sizes from the design's parameters as `sizes`, in the cabinet's
/// units), the other kind's fields hidden.
pub fn fields(entry: &str, shown: &Map<String, Value>, sizes: &Map<String, Value>) -> Value {
    let mut reply = Map::new();
    for key in CABINETS.iter().chain(&BOX_ONLY).chain(&CORNER_ONLY) {
        let mut value = sizes.get(*key).or(shown.get(*key)).cloned();
        // a file from before Shelf pin holes was a tick box
        if *key == "shelf_pins" {
            value = value.map(|v| json!(v == true || v == "yes"));
        }
        let patch = match (takes(entry, key), value.or_else(|| assumed(key))) {
            (false, _) => json!({"visible": false}),
            (true, Some(v)) => json!({ "default": v }),
            (true, None) => continue,
        };
        reply.insert(key.to_string(), patch);
    }
    Value::Object(reply)
}

/// What a builder takes for an Edit field a cabinet does not remember (the Edit and Cabinet
/// dialogs' defaults): every field the hook shows needs a value, as a run requires them all.
fn assumed(key: &str) -> Option<Value> {
    Some(match key {
        "kind" => json!("standard"),
        "shape" => json!("L-shaped"),
        "door_hinge" | "drawers_at" => json!(if key == "door_hinge" { "left" } else { "top" }),
        "corner_doors" | "handles" | "wall_mount" => json!("none"),
        "doors" | "drawers" | "dividers" | "drawer_height" => json!(0),
        "shelves" => json!(1),
        "fronts" => json!("overlay"),
        "handle_position" => json!("auto"),
        "back" => json!("groove"),
        "plinth_type" => json!("board"),
        _ => return None,
    })
}

/// Edit's `changes` without the fields at the dialog's own default: for a cabinet whose values
/// the dialog could not show (made before Edit's file), those are taken as unchanged.
// ponytail: a change to exactly the default (Doors 0, say) is lost on that first Edit only
pub fn unshown(changes: &Value) -> Value {
    let default = |key: &str| {
        Some(match key {
            "width" => json!(600),
            "height" => json!(720),
            "depth" => json!(560),
            "return" => json!(900),
            "plinth" => json!("yes"),
            "shelf_pins" => json!(false),
            "labels" => json!("no"),
            _ => return assumed(key),
        })
    };
    let mut changes = changes.clone();
    if let Some(all) = changes.as_object_mut() {
        all.retain(|k, v| !default(k).is_some_and(|d| same(&d, v)));
    }
    changes
}

/// The changes an Edit run makes to a cabinet made by `entry`: the fields it takes. The dialog
/// sends the other kind's fields too, hidden, at their schema defaults.
pub fn taken(entry: &str, changes: &Value) -> Value {
    let mut changes = changes.clone();
    if let Some(all) = changes.as_object_mut() {
        all.retain(|k, _| k == "selection" || takes(entry, k));
    }
    changes
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

/// The values in which merged `params` differ from `base` (`merge` with no changes): an Edit
/// that changes none is not built again.
pub fn changed(params: &Value, base: &Value) -> Vec<String> {
    let (Some(p), Some(b)) = (params.as_object(), base.as_object()) else {
        return vec!["params".into()];
    };
    let mut keys: Vec<&String> = p
        .keys()
        .chain(b.keys())
        .filter(|k| *k != "_existing")
        .collect();
    keys.sort();
    keys.dedup();
    // a value left out is what the builder takes for it (the hook shows that)
    let differs = |k: &&String| {
        let at = |m: &Map<String, Value>| m.get(k.as_str()).cloned().or_else(|| assumed(k));
        !same(&at(p).unwrap_or_default(), &at(b).unwrap_or_default())
    };
    keys.into_iter().filter(differs).cloned().collect()
}

/// What a cabinet's component remembers, as `component.metadata.set` takes it: the entry that
/// made it and the values it was made with. Where it was placed and what it was matched to
/// are not part of that.
pub fn remember(entry: &str, params: &Value, name: &str) -> Value {
    let mut params = params.as_object().cloned().unwrap_or_default();
    for key in ["place", "match", "selection", "_match", "_existing", "mode"] {
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
    let was_height = params.get("drawer_height").cloned().unwrap_or(json!(0));
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
                (key.as_str(), json!(value == "yes" || value == true))
            }
            k if CABINETS.contains(&k) => {
                only(&cabinets, "cabinet")?;
                (k, value.clone())
            }
            k if BOX_ONLY.contains(&k) => {
                only(&["cabinet"], "box cabinet")?;
                (k, value.clone())
            }
            "return" => {
                only(&["corner"], "corner cabinet")?;
                (key.as_str(), value.clone())
            }
            "shape" | "corner_doors" => {
                only(&["corner"], "corner cabinet")?;
                if value == "box" {
                    return Err("a corner cabinet stays a corner cabinet".into());
                }
                (key.strip_prefix("corner_").unwrap_or(key), value.clone())
            }
            _ => return Err(format!("`{key}` cannot be edited: what is not per cabinet is Woodwork settings', for new cabinets and Apply settings")),
        };
        params.insert(to.into(), value);
    }
    // Beside doors a drawer height of 0 was kept as 150 mm: without the doors, back to 0 (all
    // alike) unless the edit gives another one than the dialog showed.
    let kept = changes["drawer_height"].is_null() || same(&changes["drawer_height"], &was_height);
    if was_mixed && !mixed(&params) && kept {
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
