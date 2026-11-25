//! Woodwork settings: defaults kept in the design (`storage@0.1`) that the Cabinet and Corner
//! dialogs start from, through the dialog hook (`dialog@0.1`), and what is the same for the whole
//! design, which a new cabinet, the cut list and a worktop take (`design`).

use crate::{joint, kit};
use serde_json::{json, Map, Value};

/// The fields Settings keeps that the Cabinet and Corner dialogs show: the hook's defaults.
const FIELDS: [&str; 4] = ["material", "thickness", "reveal", "handles"];

/// What a setting is: a choice and its default, a length (kept in mm, shown and given in the
/// dialog's unit) or a number in mm in every unit, with its default and bounds in mm. A
/// length's bounds are what both units of its field allow (0.125 in to 50 mm for a board): a
/// default outside them makes the app drop the hook's whole reply.
#[derive(Clone, Copy)]
enum Def {
    Choice(&'static str),
    Length(f64, f64, f64),
    Mm(f64, f64, f64),
}

/// Who takes a setting: only the Cabinet and Corner dialogs' hook, new box cabinets, new box and
/// corner cabinets, the cut list, a worktop, or nothing built (See-through boards).
#[derive(Clone, Copy, PartialEq)]
enum To {
    Hook,
    Box,
    Cabinets,
    Cutlist,
    Worktop,
    Nothing,
}

/// The settings with a default: key, the entry's field it fills (`""`: the same), what it is
/// and who takes it. The defaults are the dialogs' from before the fields moved here.
fn table() -> Vec<(&'static str, &'static str, Def, To)> {
    use Def::*;
    use To::*;
    let mut all = vec![
        ("thickness", "", Length(18.0, 3.175, 50.0), Hook),
        ("reveal", "", Mm(2.0, 0.5, 6.0), Hook),
        // the Cabinet dialog's defaults, so Settings starts at them (not Keep) when none is stored
        ("material", "", Choice("Birch plywood"), Hook),
        ("handles", "", Choice("none"), Hook),
        ("joints", "", Choice("dowels"), Cabinets),
        ("joints_inner", "", Choice("dowels"), Box),
        ("biscuit", "", Choice("20"), Cabinets),
        ("pocket_face", "", Choice("outside"), Cabinets),
        ("hardware_models", "", Choice("on"), Cabinets),
        ("hinge_angle", "", Choice("110"), Cabinets),
        ("slide_type", "", Choice("full"), Box),
        ("fastener_models", "", Choice("off"), Cabinets),
        ("back", "", Choice("groove"), Box),
        ("back_thickness", "", Length(6.0, 3.175, 50.0), Box),
        ("drawer_thickness", "", Length(12.0, 10.0, 25.0), Box),
        ("drawer_bottom", "", Length(6.0, 3.175, 19.0), Box),
        ("fronts", "", Choice("overlay"), Cabinets),
        ("handle_position", "", Choice("auto"), Cabinets),
        // `plinth` is Cabinet's Yes or No (`design`)
        ("plinth", "", Length(100.0, 0.0, 500.0), Nothing),
        ("plinth_type", "", Choice("board"), Cabinets),
        ("plinth_setback", "", Length(50.0, 0.0, 200.0), Cabinets),
        ("labels", "", Choice("no"), Cabinets),
        ("cleat_height", "", Length(90.0, 3.175, 500.0), Cabinets),
        ("cleat_thickness", "", Length(18.0, 3.175, 50.0), Cabinets),
        ("banding", "", Choice("1"), Cutlist),
        ("cut_sizes", "size", Choice("finished"), Cutlist),
        ("sheet", "", Choice("2440x1220"), Cutlist),
        ("kerf", "", Mm(4.0, 0.0, 10.0), Cutlist),
        ("trim", "", Mm(10.0, 0.0, 50.0), Cutlist),
        ("worktop_material", "material", Choice("Oak"), Worktop),
        (
            "worktop_thickness",
            "thickness",
            Length(38.0, 3.175, 200.0),
            Worktop,
        ),
        // no cabinet's: our board materials' opacity (`material::see_through`)
        ("see_through", "", Choice("off"), Nothing),
        ("camera", "", Choice("fit"), Nothing),
        // the unit Settings' sizes are given in, set in General and used by the other sections
        ("units", "", Choice("mm"), Nothing),
    ];
    for (key, mm) in joint::SIZES {
        let (low, high) = joint::range(key);
        all.push((key, "", Length(mm, low, high), Cabinets));
    }
    all
}

fn def(key: &str) -> Option<Def> {
    table().into_iter().find(|s| s.0 == key).map(|s| s.2)
}

/// A number's bounds in mm, and whether it is shown in the dialog's unit.
fn bounds(key: &str) -> Option<(f64, f64, bool)> {
    match def(key)? {
        Def::Length(_, low, high) => Some((low, high, true)),
        Def::Mm(_, low, high) => Some((low, high, false)),
        Def::Choice(_) => None,
    }
}

/// The default of a setting that has one.
fn default(key: &str) -> Option<Value> {
    Some(match def(key)? {
        Def::Choice(c) => json!(c),
        Def::Length(mm, ..) | Def::Mm(mm, ..) => json!(mm),
    })
}

/// Every field Settings keeps.
fn kept() -> Vec<&'static str> {
    table().into_iter().map(|s| s.0).collect()
}

/// The `storage.patch` merge patch for the Settings dialog's values: only the fields that were
/// set (empty or `keep` leaves the stored value), lengths in mm. A value the same as the
/// stored one, or the default when none is, is left out: the hook fills omitted keys.
pub fn patch(params: &Value, stored: &Value) -> Result<Value, String> {
    // a dialog without Units (a section other than General) gives its sizes in the stored one
    let units = params["units"].as_str();
    let units = units.or(stored["document"]["settings"]["units"].as_str());
    let unit = if units == Some("in") { 25.4 } else { 1.0 };
    let mut set = Map::new();
    for key in kept() {
        let value = &params[key];
        if value.is_null() || value == "keep" {
            continue;
        }
        let value = match bounds(key) {
            Some((min, max, scaled)) => {
                let v = value.as_f64().ok_or(format!("`{key}` must be a number"))?;
                let scale = if scaled { unit } else { 1.0 };
                let mm = (v * scale * 1e6).round() / 1e6;
                if !(min..=max).contains(&mm) {
                    return Err(format!("`{key}` must be {min} to {max} mm"));
                }
                json!(mm)
            }
            None => value.clone(),
        };
        let was = &stored["document"]["settings"][key];
        let was = if was.is_null() {
            default(key)
        } else {
            Some(was.clone())
        };
        let same = match (&was, value.as_f64()) {
            // 1e-3 mm: an inch value shown with 6 decimals comes back off by a little
            (Some(was), Some(mm)) => was.as_f64().is_some_and(|was| (was - mm).abs() < 1e-3),
            (Some(was), None) => *was == value,
            _ => false,
        };
        if !same {
            set.insert(key.into(), value);
        }
    }
    Ok(json!({ "settings": set }))
}

/// Whether a run ends by framing the result (`view.fit`): unless Camera is stored as `leave`.
pub fn fits(stored: &Value) -> bool {
    stored["document"]["settings"]["camera"] != "leave"
}

/// A stored setting within its bounds, else its default, in mm.
fn mm(settings: &Value, key: &str) -> f64 {
    let Some((low, high, _)) = bounds(key) else {
        return 0.0;
    };
    let stored = settings[key].as_f64().filter(|v| (low..=high).contains(v));
    stored
        .or(default(key).and_then(|d| d.as_f64()))
        .unwrap_or(0.0)
}

fn rounded(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

/// Fills the `params` of a new box or corner cabinet (`entry` cabinet or corner), the cut list
/// or a worktop with what Woodwork settings keep for the whole design, from `storage.get`'s
/// reply, or the defaults, lengths in its `units`: they are no field of its dialog, and so a
/// cabinet remembers them for Edit. A corner cabinet has nothing inside to join, no drawers, no
/// back choice and no dado (it takes the default instead). Cabinet's Plinth Yes takes the
/// height from here, No none. A key already given is kept: the dialogs have none, but the
/// unit tests and a worktop's remembered values give them.
pub fn design(entry: &str, params: &mut Value, stored: &Value) {
    let settings = &stored["document"]["settings"];
    let unit = if params["units"] == "in" { 25.4 } else { 1.0 };
    let takes = |to: To| match entry {
        "cabinet" => matches!(to, To::Box | To::Cabinets),
        "corner" => to == To::Cabinets,
        "cutlist" => to == To::Cutlist,
        "worktop" => to == To::Worktop,
        _ => false,
    };
    if matches!(entry, "cabinet" | "corner") {
        let plinth = match &params["plinth"] {
            Value::String(no) if no == "no" => Some(0.0),
            Value::Number(_) => None,
            _ => Some(rounded(mm(settings, "plinth") / unit)),
        };
        if let Some(plinth) = plinth {
            params["plinth"] = json!(plinth);
        }
    }
    for (key, field, def, to) in table() {
        let field = if field.is_empty() { key } else { field };
        if !takes(to) || params.get(field).is_some() {
            continue;
        }
        params[field] = match def {
            Def::Choice(default) => {
                let v = match settings[key].as_str() {
                    Some("dado") if entry == "corner" => default,
                    Some(v) => v,
                    None => default,
                };
                match key {
                    "labels" => json!(v == "yes"),
                    _ => json!(v),
                }
            }
            Def::Length(..) => json!(rounded(mm(settings, key) / unit)),
            Def::Mm(..) => json!(mm(settings, key)),
        };
    }
}

/// Edit's Plinth (`keep`, `yes`, `no`) as the plinth height `edit::merge` takes: No is 0; Yes
/// keeps a plinth the cabinet has (its parameter, from `document.inspect`'s `design`) and gives
/// one without the height and type of Woodwork settings; Keep leaves it.
pub fn edit_plinth(changes: &Value, remembered: &Value, design: &Value, stored: &Value) -> Value {
    let mut changes = changes.clone();
    let ask = changes["plinth"].as_str().map(String::from);
    if let Some(all) = changes.as_object_mut() {
        all.remove("plinth");
    }
    match ask.as_deref() {
        Some("no") => changes["plinth"] = json!(0),
        Some("yes") => {
            let p = &remembered["params"];
            let id = format!(
                "{}_plinth",
                kit::prefix(p["name"].as_str().unwrap_or_default())
            );
            let all = design["params"].as_array().into_iter().flatten();
            let found = all.into_iter().find(|q| q["name"] == id.as_str());
            if found.and_then(|q| q["value"].as_f64()).unwrap_or(0.0) <= 0.0 {
                let settings = &stored["document"]["settings"];
                let unit = if p["units"] == "in" { 25.4 } else { 1.0 };
                changes["plinth"] = json!(rounded(mm(settings, "plinth") / unit));
                if changes["plinth_type"].as_str().is_none_or(|t| t == "keep") {
                    let kind = settings["plinth_type"].as_str().unwrap_or("board");
                    changes["plinth_type"] = json!(kind);
                }
            }
        }
        _ => {}
    }
    changes
}

/// A built cabinet's `params` (`edit::merge`'s, sizes from the design's parameters) with what
/// Woodwork settings hold now, for Settings' Apply to selected; `None` when it already has them.
/// The design-wide settings count stored or default, and the dialogs' defaults (material,
/// thickness, reveal, handles) only when stored. Only what the cabinet's `entry` takes (a
/// corner has no drawers, no inside joinery, no back choice), and the plinth's only when it has
/// one: a cabinet without one keeps none. With the params, the fields that change.
pub fn apply(entry: &str, params: &Value, stored: &Value) -> Option<(Value, Vec<String>)> {
    let settings = &stored["document"]["settings"];
    let mut wanted = json!({"units": params["units"]});
    for key in ["material", "handles"] {
        if settings[key].is_string() {
            wanted[key] = settings[key].clone();
        }
    }
    let has_plinth = params["plinth"].as_f64().unwrap_or(0.0) > 0.0;
    for (key, field, def, to) in table() {
        let field = if field.is_empty() { key } else { field };
        let plinth = matches!(key, "plinth" | "plinth_type" | "plinth_setback");
        if to == To::Hook && settings[key].is_number() {
            wanted[field] = match def {
                Def::Length(..) => json!(rounded(mm(settings, key) / unit(params))),
                _ => json!(mm(settings, key)),
            };
        } else if plinth && has_plinth {
            wanted[field] = match def {
                Def::Choice(d) => json!(settings[key].as_str().unwrap_or(d)),
                _ => json!(rounded(mm(settings, key) / unit(params))),
            };
        }
    }
    // without a plinth its type and set-back stay as they are
    if !has_plinth {
        for key in ["plinth_type", "plinth_setback"] {
            if let Some(v) = params.get(key) {
                wanted[key] = v.clone();
            }
        }
    }
    // the rest as a new cabinet gets them (Plinth given, so `design` leaves it)
    wanted["plinth"] = wanted
        .get("plinth")
        .cloned()
        .unwrap_or(params["plinth"].clone());
    design(entry, &mut wanted, stored);
    let mut out = params.clone();
    let mut changed = Vec::new();
    for (key, value) in wanted.as_object().into_iter().flatten() {
        let same = match (&out[key.as_str()], value) {
            (Value::Number(a), Value::Number(b)) => {
                (a.as_f64().unwrap_or(0.0) - b.as_f64().unwrap_or(0.0)).abs() < 1e-6
            }
            (a, b) => a == b,
        };
        if !same {
            out[key.as_str()] = value.clone();
            changed.push(key.clone());
        }
    }
    (!changed.is_empty()).then_some((out, changed))
}

fn unit(params: &Value) -> f64 {
    if params["units"] == "in" {
        25.4
    } else {
        1.0
    }
}

/// The Cabinet hook's reply for Match selected, from the dialog's `values` and
/// `document.inspect`'s reply with body names: Yes and shown when the picked board (the field,
/// else the design's selection) is a box or corner cabinet's, hidden otherwise.
pub fn matching(values: &Value, design: &Value) -> Value {
    let picked = crate::select::picked(values, design);
    let bodies = design["bodies"].as_array().into_iter().flatten();
    let names: Vec<&str> = bodies
        .filter_map(|b| b.as_str().or(b["name"].as_str()))
        .collect();
    let cabinet = (picked.first())
        .and_then(|b| crate::select::owner(design, b))
        .filter(|c| names.contains(&format!("{c} Side L").as_str()));
    match cabinet {
        Some(_) => json!({"default": "selected", "visible": true}),
        None => json!({"visible": false}),
    }
}

/// The sizes Match selected takes from the picked cabinet, as the Cabinet hook's patch for those
/// fields (greyed while it is on, `x-solidcraft-enabled-when`): its height, depth, thickness
/// (its parameters, in the dialog's unit) and plinth (Yes when it has one), with a hint naming
/// it. Nothing when no cabinet is picked. `design` as for `matching`.
pub fn matched_sizes(values: &Value, design: &Value) -> Map<String, Value> {
    let mut reply = Map::new();
    let picked = crate::select::picked(values, design);
    let Some(cabinet) = picked.first().and_then(|b| crate::select::owner(design, b)) else {
        return reply;
    };
    let prefix = kit::prefix(cabinet);
    let param = |key: &str| {
        let all = design["params"].as_array().into_iter().flatten();
        let id = format!("{prefix}_{key}");
        all.into_iter()
            .find(|p| p["name"] == id.as_str())
            .and_then(|p| p["value"].as_f64())
    };
    let unit = if values["units"] == "in" { 25.4 } else { 1.0 };
    let hint = format!("Match selected takes it from {cabinet}");
    for key in ["height", "depth", "thickness"] {
        if let Some(mm) = param(key) {
            reply.insert(
                key.into(),
                json!({"default": rounded(mm / unit), "hint": hint}),
            );
        }
    }
    if let Some(mm) = param("plinth") {
        let plinth = if mm > 0.0 { "yes" } else { "no" };
        reply.insert("plinth".into(), json!({"default": plinth, "hint": hint}));
    }
    reply
}

/// The dialog hook's reply for `entry` with the dialog's `values`, from `storage.get`'s
/// reply: each stored setting as the field's default, lengths in the dialog's unit. The
/// Settings dialog gets every setting with a default, stored or the default, and no hint.
pub fn fields(entry: &str, values: &Value, stored: &Value) -> Value {
    let mut reply = Map::new();
    let own = entry == "settings";
    if !own && !matches!(entry, "cabinet" | "corner") {
        return Value::Object(reply);
    }
    let mut settings = stored["document"]["settings"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    if own {
        settings.retain(|key, _| default(key).is_some());
        for (key, ..) in table() {
            if let Some(d) = default(key) {
                settings.entry(key).or_insert(d);
            }
        }
    } else {
        settings.retain(|key, _| FIELDS.contains(&key.as_str()));
    }
    // Settings' sizes are in Units, which only General (and All) show; the other sections use
    // the stored one, said in each size's hint
    let shown = values["section"].as_str().unwrap_or("all");
    let with_units = matches!(shown, "general" | "all") || !own;
    let stored_units = stored["document"]["settings"]["units"]
        .as_str()
        .unwrap_or("mm");
    let units = match with_units {
        true => values["units"].as_str().unwrap_or(stored_units),
        false => stored_units,
    };
    let unit = if units == "in" { 25.4 } else { 1.0 };
    for (key, value) in &settings {
        let value = match (bounds(key), value.as_f64()) {
            (Some((min, max, scaled)), Some(mm)) if (min..=max).contains(&mm) => {
                json!(rounded(mm / if scaled { unit } else { 1.0 }))
            }
            // stored before the bounds were checked
            (Some(_), _) => continue,
            _ => value.clone(),
        };
        let field = if own && !with_units && bounds(key).is_some_and(|b| b.2) {
            let name = if units == "in" { "inches" } else { "mm" };
            json!({ "default": value, "hint": format!("In {name}, as set in General") })
        } else if own {
            json!({ "default": value })
        } else {
            json!({"default": value, "hint": "From Woodwork settings"})
        };
        reply.insert(key.clone(), field);
    }
    // Settings' Section shows one group of fields: the others hidden by the hook, which (unlike
    // a schema condition) still sends them, so a script's values are all used and the dialog's
    // hidden ones are the stored values, which `patch` leaves out.
    if own {
        reply.insert(
            "section".into(),
            json!({"hint": "Pick a group to see its settings"}),
        );
        // a divider row under Section, greyed and never stored (filled for a script)
        reply.insert("divider".into(), json!({ "default": DIVIDER }));
        for (section, keys) in SECTIONS {
            if shown == "all" || shown == section {
                continue;
            }
            for key in keys {
                let field = reply.entry(key.to_string()).or_insert(json!({}));
                field["visible"] = json!(false);
            }
        }
    }
    Value::Object(reply)
}

/// The divider row under Settings' Section: a greyed field, its value never stored.
pub const DIVIDER: &str = "────────────────────";

/// Settings' sections (its field `section`, the same in the manifest) and their fields. Apply
/// to shows in every section.
pub const SECTIONS: [(&str, &[&str]); 7] = [
    (
        "general",
        &[
            "units",
            "material",
            "thickness",
            "back",
            "back_thickness",
            "labels",
        ],
    ),
    (
        "fronts",
        &[
            "fronts",
            "reveal",
            "handles",
            "handle_position",
            "hinge_angle",
        ],
    ),
    (
        "drawers",
        &["drawer_thickness", "drawer_bottom", "slide_type"],
    ),
    (
        "plinth",
        &[
            "plinth",
            "plinth_type",
            "plinth_setback",
            "cleat_height",
            "cleat_thickness",
        ],
    ),
    (
        "joinery",
        &[
            "joints",
            "joints_inner",
            "joint_spacing",
            "joint_edge",
            "dowel_d",
            "dowel_length",
            "dado_depth",
            "biscuit",
            "cam_d",
            "cam_bolt",
            "pocket_face",
        ],
    ),
    (
        "cutlist",
        &[
            "banding",
            "cut_sizes",
            "sheet",
            "kerf",
            "trim",
            "worktop_material",
            "worktop_thickness",
        ],
    ),
    (
        "display",
        &[
            "hardware_models",
            "fastener_models",
            "see_through",
            "camera",
        ],
    ),
];

#[cfg(test)]
mod tests;
