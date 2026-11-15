//! The cabinet the user has selected: any of its panels, or a face or edge of one.

use serde_json::Value;

pub struct Cabinet {
    pub name: String,
    /// World box around the two sides, mm. Doors and an overlaid back stand outside it.
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// Side L and Side R: whether the panel runs along X, and where it starts in Y.
    pub sides: [(bool, f64); 2],
}

impl Cabinet {
    /// The quarter turn, in degrees, a box cabinet needs to continue from this one on
    /// `place`: along a corner cabinet's return, or along a run that is already turned.
    pub fn turn(&self, place: &str) -> Option<i32> {
        let [l, r] = self.sides;
        match place {
            "right" if r.0 && (!l.0 || r.1 < l.1) => Some(-90),
            "left" if l.0 && (!r.0 || l.1 < r.1) => Some(90),
            "above" if l.0 && r.0 => Some(if r.1 < l.1 { -90 } else { 90 }),
            _ => None,
        }
    }
}

fn names(measure: &Value) -> impl Iterator<Item = &str> {
    measure["bodies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| b["name"].as_str())
}

/// The cabinet a body belongs to. Every cabinet has a body called "<name> Side L"; of the
/// names that fit, the longest wins, so "Base 2 Door" is in "Base 2", not in "Base".
fn owner<'a>(measure: &'a Value, body: &str) -> Option<&'a str> {
    names(measure)
        .filter_map(|name| name.strip_suffix(" Side L"))
        .filter(|cabinet| body.starts_with(&format!("{cabinet} ")))
        .max_by_key(|cabinet| cabinet.len())
}

/// The selected bodies, by name: for a face or an edge, the body it is on.
/// `design` is `document.inspect`'s reply.
pub fn picked(design: &Value) -> Vec<String> {
    let items = design["selection"].as_array().into_iter().flatten();
    let mut bodies: Vec<String> = items
        .filter_map(|item| item["name"].as_str().or(item["body"].as_str()))
        .map(String::from)
        .collect();
    bodies.dedup();
    bodies
}

/// Every body of a cabinet. `measure` is `inspect.measure`'s reply.
pub fn parts(measure: &Value, cabinet: &str) -> Vec<String> {
    names(measure)
        .filter(|body| owner(measure, body) == Some(cabinet))
        .map(String::from)
        .collect()
}

/// The cabinet of the first selected body.
pub fn cabinet(design: &Value, measure: &Value) -> Result<Cabinet, String> {
    let picked = picked(design);
    let picked = picked.first().ok_or("select a part of a cabinet first")?;
    let name = owner(measure, picked).ok_or(format!("`{picked}` is not part of a cabinet"))?;

    // ponytail: world bounding box, so this is only right for a cabinet that still stands
    // square to the axes. Read the component's placement if turned cabinets matter.
    let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    let mut sides = [(false, 0.0); 2];
    for (n, side) in ["Side L", "Side R"].into_iter().enumerate() {
        let wanted = format!("{name} {side}");
        let bodies = measure["bodies"].as_array().into_iter().flatten();
        let body = (bodies.into_iter().find(|b| b["name"] == wanted.as_str()))
            .ok_or(format!("`{wanted}` is missing"))?;
        for i in 0..3 {
            min[i] = min[i].min(body["bbox"]["min"][i].as_f64().ok_or("body without bbox")?);
            max[i] = max[i].max(body["bbox"]["max"][i].as_f64().ok_or("body without bbox")?);
        }
        let at = |end: &str, i: usize| body["bbox"][end][i].as_f64().unwrap_or(0.0);
        let along_x = at("max", 0) - at("min", 0) > at("max", 1) - at("min", 1);
        sides[n] = (along_x, at("min", 1));
    }
    Ok(Cabinet {
        name: name.to_string(),
        min,
        max,
        sides,
    })
}

#[cfg(test)]
mod tests;
