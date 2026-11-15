//! The cabinet the user has selected: any of its panels, or a face or edge of one.

use serde_json::Value;

pub struct Cabinet {
    pub name: String,
    /// World box around the two sides, mm. Doors and an overlaid back stand outside it.
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// Side L, Side R and the back, seen from above: x0, y0, x1, y1. A cabinet without a
    /// back has an empty one.
    pub sides: [Plan; 2],
    pub back: Plan,
}

/// A box seen from above: x0, y0, x1, y1 in mm.
pub type Plan = [f64; 4];
type Dir = [f64; 2];

fn centre(b: Plan) -> Dir {
    [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0]
}

/// The direction a panel is thin in, pointing away from `from`.
fn away(panel: Plan, from: Dir) -> Dir {
    let (c, along_x) = (centre(panel), panel[2] - panel[0] > panel[3] - panel[1]);
    match along_x {
        true => [0.0, (c[1] - from[1]).signum()],
        false => [(c[0] - from[0]).signum(), 0.0],
    }
}

/// How far a box reaches in a direction.
fn reach(b: Plan, dir: Dir) -> f64 {
    let dot = |x: f64, y: f64| x * dir[0] + y * dir[1];
    dot(b[0], b[1]).max(dot(b[2], b[3]))
}

impl Cabinet {
    /// Where a new cabinet of `width` x `depth` goes against this one, and its turn in
    /// degrees: the middle of its footprint in mm, standing on this one's floor or top.
    ///
    /// Right is against Side R and left against Side L, whichever way those face, with the
    /// backs flush. So a cabinet continues a corner cabinet's return turned a quarter, and a
    /// run that is turned carries on turned, corner cabinets included. Above keeps this
    /// cabinet's turn, left sides and backs flush.
    // ponytail: quarter turns only, read from the bounding boxes of the two sides and the back.
    pub fn seat(&self, place: &str, width: f64, depth: f64) -> ([f64; 3], i32) {
        let [l, r] = self.sides;
        let whole = [self.min[0], self.min[1], self.max[0], self.max[1]];
        let (out_l, out_r) = (away(l, centre(r)), away(r, centre(l)));
        // The new cabinet's own X, what it stands against, and which way it goes from it.
        let (x, against, step) = match place {
            "right" => (out_r, r, 1.0),
            "left" => (out_l.map(|v| -v), l, -1.0),
            // a corner cabinet's sides stand square to each other: go by its back
            _ if out_l[0] * out_r[0] + out_l[1] * out_r[1] == 0.0 => {
                let y = away(self.back, centre(whole));
                ([y[1], -y[0]], whole, 0.0)
            }
            _ => (out_r, whole, 0.0),
        };
        let y = [-x[1], x[0]]; // towards the back
        let along = match step == 0.0 {
            true => -reach(against, x.map(|v| -v)) + width / 2.0,
            false => step * (reach(against, x.map(|v| v * step)) + width / 2.0),
        };
        let across = reach(against, y) - depth / 2.0;
        let z = if place == "above" {
            self.max[2]
        } else {
            self.min[2]
        };
        let at = [
            x[0] * along + y[0] * across,
            x[1] * along + y[1] * across,
            z,
        ];
        (at, x[1].atan2(x[0]).to_degrees().round() as i32)
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
    let plan = |body: &Value| -> Plan {
        let at = |end: &str, i: usize| body["bbox"][end][i].as_f64().unwrap_or(0.0);
        [at("min", 0), at("min", 1), at("max", 0), at("max", 1)]
    };
    let mut sides = [[0.0; 4]; 2];
    for (n, side) in ["Side L", "Side R"].into_iter().enumerate() {
        let wanted = format!("{name} {side}");
        let bodies = measure["bodies"].as_array().into_iter().flatten();
        let body = (bodies.into_iter().find(|b| b["name"] == wanted.as_str()))
            .ok_or(format!("`{wanted}` is missing"))?;
        for i in 0..3 {
            min[i] = min[i].min(body["bbox"]["min"][i].as_f64().ok_or("body without bbox")?);
            max[i] = max[i].max(body["bbox"]["max"][i].as_f64().ok_or("body without bbox")?);
        }
        sides[n] = plan(body);
    }
    Ok(Cabinet {
        name: name.to_string(),
        min,
        max,
        sides,
        back: {
            let wanted = format!("{name} Back");
            let mut bodies = measure["bodies"].as_array().into_iter().flatten();
            let back = bodies.find(|b| b["name"] == wanted.as_str());
            back.map(plan).unwrap_or_default()
        },
    })
}

#[cfg(test)]
mod tests;
