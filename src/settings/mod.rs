//! Woodwork settings: defaults kept in the design (`storage@0.1`) that the Cabinet and Corner
//! dialogs start from, through the dialog hook (`dialog@0.1`), and the design-wide joinery and
//! hardware every new cabinet takes (`design`).

use crate::joint;
use serde_json::{json, Map, Value};

/// The fields Settings keeps that the Cabinet and Corner dialogs show: the hook's defaults.
const FIELDS: [&str; 6] = [
    "material",
    "thickness",
    "reveal",
    "handles",
    "plinth",
    "plinth_type",
];

/// The design-wide choices, no Cabinet or Corner field: every new cabinet takes the stored
/// one, or this default (the Cabinet dialog's before they moved here).
const CHOICES: [(&str, &str); 8] = [
    ("joints", "dowels"),
    ("joints_inner", "dowels"),
    ("biscuit", "20"),
    ("pocket_face", "outside"),
    ("hardware_models", "on"),
    ("hinge_angle", "110"),
    ("slide_type", "full"),
    // no cabinet's: our board materials' opacity (`material::see_through`)
    ("see_through", "off"),
];

/// The lengths, kept in mm and shown in the dialog's unit, with the bounds in mm that both
/// units of the Cabinet and Corner fields allow (thickness 0.125 in to 50 mm): a default
/// outside them makes the app drop the hook's whole reply. The reveal is mm in every unit.
/// The joinery's sizes are design-wide, with `Joinery`'s bounds.
fn bounds(key: &str) -> Option<(f64, f64)> {
    match key {
        "thickness" => Some((3.175, 50.0)),
        "plinth" => Some((0.0, 500.0)),
        _ if joint::SIZES.iter().any(|s| s.0 == key) => Some(joint::range(key)),
        _ => None,
    }
}

/// The numbers the Settings dialog shows when nothing is stored: the Cabinet dialog's defaults
/// in mm, and the joinery's. A number with a value is a drag field the app keeps within its
/// bounds.
fn numbers() -> impl Iterator<Item = (&'static str, f64)> {
    [("thickness", 18.0), ("reveal", 2.0), ("plinth", 0.0)]
        .into_iter()
        .chain(joint::SIZES)
}

/// The default of a design-wide choice or a number.
fn default(key: &str) -> Option<Value> {
    let choice = CHOICES.iter().find(|c| c.0 == key).map(|c| json!(c.1));
    choice.or_else(|| numbers().find(|n| n.0 == key).map(|n| json!(n.1)))
}

/// Every field Settings keeps.
fn kept() -> impl Iterator<Item = &'static str> {
    (FIELDS.into_iter())
        .chain(CHOICES.iter().map(|c| c.0))
        .chain(joint::SIZES.iter().map(|s| s.0))
        .chain(["camera"])
}

/// The `storage.patch` merge patch for the Settings dialog's values: only the fields that were
/// set (empty or `keep` leaves the stored value), lengths in mm. A value the same as the
/// stored one, or the default when none is, is left out: the hook fills omitted keys.
pub fn patch(params: &Value, stored: &Value) -> Result<Value, String> {
    let scale = if params["units"] == "in" { 25.4 } else { 1.0 };
    let mut set = Map::new();
    for key in kept() {
        let value = &params[key];
        if value.is_null() || value == "keep" {
            continue;
        }
        let value = match bounds(key) {
            Some((min, max)) => {
                let v = value.as_f64().ok_or(format!("`{key}` must be a number"))?;
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

/// Fills a new cabinet's or corner cabinet's `params` with the design-wide joinery and
/// hardware from `storage.get`'s reply, or their defaults, lengths in its `units`: they are no
/// field of its dialog, and so the cabinet remembers them for Edit. A corner cabinet has
/// nothing inside to join, no drawers, and no dado (it takes the default instead). A key
/// already given is kept: the dialogs have none, but the unit tests give them.
pub fn design(entry: &str, params: &mut Value, stored: &Value) {
    let settings = &stored["document"]["settings"];
    let scale = if params["units"] == "in" { 25.4 } else { 1.0 };
    let corner = entry == "corner";
    for (key, default) in CHOICES {
        let skip = key == "see_through" || corner && matches!(key, "joints_inner" | "slide_type");
        if params.get(key).is_some() || skip {
            continue;
        }
        let v = match settings[key].as_str() {
            Some("dado") if corner => default,
            Some(v) => v,
            None => default,
        };
        params[key] = json!(v);
    }
    for (key, default) in joint::SIZES {
        if params.get(key).is_some() {
            continue;
        }
        let (low, high) = joint::range(key);
        let mm = settings[key].as_f64().filter(|v| (low..=high).contains(v));
        params[key] = json!((mm.unwrap_or(default) / scale * 1e6).round() / 1e6);
    }
}

/// The dialog hook's reply for `entry` with the dialog's `values`, from `storage.get`'s
/// reply: each stored setting as the field's default, lengths in the dialog's unit. The
/// Settings dialog gets every number and design-wide choice, stored or the default, and no
/// hint.
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
        for (key, mm) in numbers() {
            settings.entry(key).or_insert(json!(mm));
        }
        for (key, choice) in CHOICES {
            settings.entry(key).or_insert(json!(choice));
        }
    } else {
        settings.retain(|key, _| FIELDS.contains(&key.as_str()));
    }
    let scale = if values["units"] == "in" { 25.4 } else { 1.0 };
    for (key, value) in &settings {
        let value = match (bounds(key), value.as_f64()) {
            (Some((min, max)), Some(mm)) if (min..=max).contains(&mm) => {
                json!((mm / scale * 1e6).round() / 1e6)
            }
            // stored before the bounds were checked
            (Some(_), _) => continue,
            _ => value.clone(),
        };
        let field = if own {
            json!({ "default": value })
        } else {
            json!({"default": value, "hint": "From Woodwork settings"})
        };
        reply.insert(key.clone(), field);
    }
    Value::Object(reply)
}

#[cfg(test)]
mod tests;
