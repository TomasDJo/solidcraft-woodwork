//! The cabinet the user has selected: any of its panels, or a face or edge of one.

use serde_json::Value;

pub struct Cabinet {
    pub name: String,
    /// World box around the two sides, mm. Doors and an overlaid back stand outside it.
    pub min: [f64; 3],
    pub max: [f64; 3],
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
    for side in ["Side L", "Side R"] {
        let wanted = format!("{name} {side}");
        let bodies = measure["bodies"].as_array().into_iter().flatten();
        let body = (bodies.into_iter().find(|b| b["name"] == wanted.as_str()))
            .ok_or(format!("`{wanted}` is missing"))?;
        for i in 0..3 {
            min[i] = min[i].min(body["bbox"]["min"][i].as_f64().ok_or("body without bbox")?);
            max[i] = max[i].max(body["bbox"]["max"][i].as_f64().ok_or("body without bbox")?);
        }
    }
    Ok(Cabinet {
        name: name.to_string(),
        min,
        max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_the_cabinet_of_a_selected_part() {
        let body = |name: &str, x: f64| json!({"name": name, "bbox": {"min": [x, 0, 0], "max": [x + 18.0, 560, 720]}});
        let measure = json!({"bodies": [
            body("Base Side L", 0.0),
            body("Base Side R", 582.0),
            body("Base Door", 0.0),
            body("Base 2 Side L", 600.0),
            body("Base 2 Side R", 1182.0),
            body("Base 2 Door", 600.0),
            body("Worktop", 0.0),
        ]});
        // a face of "Base 2 Door": the longer name, not "Base"
        let design = json!({"selection": [{"type": "face", "body": "Base 2 Door", "index": 3}]});
        let found = cabinet(&design, &measure).unwrap();
        assert_eq!(found.name, "Base 2");
        assert_eq!((found.min[0], found.max[0]), (600.0, 1200.0));
        assert_eq!(
            parts(&measure, "Base"),
            ["Base Side L", "Base Side R", "Base Door"]
        );
        assert_eq!(picked(&design), ["Base 2 Door"]);

        let design = json!({"selection": [{"type": "body", "name": "Worktop"}]});
        assert!(cabinet(&design, &measure).is_err());
        assert!(cabinet(&json!({"selection": []}), &measure).is_err());
    }
}
