//! Woodwork settings: defaults kept in the design (`storage@0.1`) that the Cabinet and Corner
//! dialogs start from, through the dialog hook (`dialog@0.1`).

use serde_json::{json, Map, Value};

/// The fields Settings keeps, by the names Cabinet and Corner give them.
const FIELDS: [&str; 8] = [
    "material",
    "thickness",
    "reveal",
    "handles",
    "plinth",
    "plinth_type",
    "joints",
    "joints_inner",
];

/// The lengths, kept in mm and shown in the dialog's unit, with the bounds in mm that both
/// units of the Cabinet and Corner fields allow (thickness 0.125 in to 50 mm): a default
/// outside them makes the app drop the hook's whole reply. The reveal is mm in every unit.
fn bounds(key: &str) -> Option<(f64, f64)> {
    match key {
        "thickness" => Some((3.175, 50.0)),
        "plinth" => Some((0.0, 500.0)),
        _ => None,
    }
}

/// The `storage.patch` merge patch for the Settings dialog's values: only the fields that were
/// set (empty or `keep` leaves the stored value), lengths in mm. A number the same as the
/// stored one, or the default when none is, is left out: the hook fills omitted numbers.
pub fn patch(params: &Value, stored: &Value) -> Result<Value, String> {
    let scale = if params["units"] == "in" { 25.4 } else { 1.0 };
    let mut set = Map::new();
    // Camera is a setting of its own, not a Cabinet field.
    for key in FIELDS.into_iter().chain(["camera"]) {
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
        if let Some(mm) = value.as_f64() {
            let was = stored["document"]["settings"][key].as_f64();
            let default = NUMBERS.iter().find(|(k, _)| *k == key).map(|(_, d)| *d);
            // 1e-3 mm: an inch value shown with 6 decimals comes back off by a little
            if was.or(default).is_some_and(|was| (was - mm).abs() < 1e-3) {
                continue;
            }
        }
        set.insert(key.into(), value);
    }
    Ok(json!({ "settings": set }))
}

/// Whether a run ends by framing the result (`view.fit`): unless Camera is stored as `leave`.
pub fn fits(stored: &Value) -> bool {
    stored["document"]["settings"]["camera"] != "leave"
}

/// The numbers the Settings dialog shows when nothing is stored: the Cabinet dialog's defaults
/// in mm. A number with a value is a drag field the app keeps within its bounds.
const NUMBERS: [(&str, f64); 3] = [("thickness", 18.0), ("reveal", 2.0), ("plinth", 0.0)];

/// The dialog hook's reply for `entry` with the dialog's `values`, from `storage.get`'s
/// reply: each stored setting as the field's default, lengths in the dialog's unit. The
/// Settings dialog gets every number, stored or the Cabinet default, and no hint.
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
        settings.retain(|key, _| NUMBERS.iter().any(|(k, _)| k == key));
        for (key, mm) in NUMBERS {
            settings.entry(key).or_insert(json!(mm));
        }
    }
    let scale = if values["units"] == "in" { 25.4 } else { 1.0 };
    for (key, value) in &settings {
        // both dialogs have every field Settings keeps, but Corner no joinery yet
        if !FIELDS.contains(&key.as_str()) || (entry == "corner" && key.starts_with("joints")) {
            continue;
        }
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
